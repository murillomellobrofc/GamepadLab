//! Backend on top of gilrs: XInput/WGI on Windows, IOKit HID on macOS,
//! evdev on Linux. Covers every "ordinary" controller.

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::mapping;
use crate::model::*;
use gamepadlab_calibration::Vec2;
use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Replay, Ticks};
use gilrs::{Axis, Button as GButton, EventType, GamepadId, Gilrs, GilrsBuilder, PowerInfo};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

struct Pad {
    id: ControllerId,
    gid: GamepadId,
    /// Timestamp of the event group being accumulated.
    group_time: Option<SystemTime>,
    rumble: Option<(Effect, Effect)>,
    last_power: PowerInfo,
}

pub struct GilrsBackend {
    gilrs: Gilrs,
    pads: HashMap<GamepadId, Pad>,
    started: bool,
}

impl GilrsBackend {
    pub fn new() -> Result<Self, InputError> {
        let gilrs = GilrsBuilder::new()
            // No deadzone/jitter filtering: the app must see raw values.
            .with_default_filters(false)
            .build()
            .map_err(|e| InputError::BackendUnavailable(format!("gilrs: {e}")))?;
        Ok(Self {
            gilrs,
            pads: HashMap::new(),
            started: false,
        })
    }

    fn make_id(&self, gid: GamepadId) -> ControllerId {
        let gp = self.gilrs.gamepad(gid);
        let uuid: String = gp.uuid().iter().map(|b| format!("{b:02x}")).collect();
        ControllerId::new(format!(
            "gilrs:{:04x}:{:04x}:{}:{}",
            gp.vendor_id().unwrap_or(0),
            gp.product_id().unwrap_or(0),
            &uuid[..8.min(uuid.len())],
            usize::from(gid)
        ))
    }

    fn info(&self, gid: GamepadId, id: &ControllerId) -> ControllerInfo {
        let gp = self.gilrs.gamepad(gid);
        let vid = gp.vendor_id().unwrap_or(0);
        let pid = gp.product_id().unwrap_or(0);
        let os_name = gp.os_name().to_string();
        let kind = mapping::detect_kind(vid, pid, &os_name);
        let battery = power_to_battery(gp.power_info());
        let state = gp.state();
        ControllerInfo {
            id: id.clone(),
            name: mapping::friendly_name(vid, pid, gp.name()),
            vendor_id: vid,
            product_id: pid,
            serial: None,
            kind,
            connection: ConnectionType::Unknown,
            button_count: state.buttons().count() as u32,
            axis_count: state.axes().count() as u32,
            capabilities: Capabilities {
                rumble: gp.is_ff_supported(),
                battery: battery.is_some(),
                ..Capabilities::default()
            },
            battery,
            backend: "gilrs".into(),
            is_virtual: mapping::is_virtual(vid, pid, &os_name),
            is_demo: false,
            profile_key: ControllerInfo::make_profile_key(vid, pid, None),
        }
    }

    fn state(&self, gid: GamepadId, timestamp_us: u64) -> DeviceState {
        let gp = self.gilrs.gamepad(gid);
        let mut st = DeviceState::neutral(timestamp_us);
        let btn = |b: GButton| gp.button_data(b).map(|d| d.value()).unwrap_or(0.0);
        let axis = |a: Axis| gp.axis_data(a).map(|d| d.value()).unwrap_or(0.0);

        let digital = [
            (GButton::South, Button::South),
            (GButton::East, Button::East),
            (GButton::West, Button::West),
            (GButton::North, Button::North),
            (GButton::LeftTrigger, Button::LeftBumper),
            (GButton::RightTrigger, Button::RightBumper),
            (GButton::Select, Button::Select),
            (GButton::Start, Button::Start),
            (GButton::LeftThumb, Button::LeftStick),
            (GButton::RightThumb, Button::RightStick),
            (GButton::DPadUp, Button::DpadUp),
            (GButton::DPadDown, Button::DpadDown),
            (GButton::DPadLeft, Button::DpadLeft),
            (GButton::DPadRight, Button::DpadRight),
            (GButton::Mode, Button::Guide),
            (GButton::C, Button::Misc),
        ];
        for (g, b) in digital {
            let v = btn(g);
            st.set_button(b, ButtonState::analog(v, 0.5));
        }
        // Hats reported as axes when the dpad-to-button filter is off.
        let dx = axis(Axis::DPadX);
        let dy = axis(Axis::DPadY);
        if dx != 0.0 || dy != 0.0 {
            st.set_button(Button::DpadRight, ButtonState::digital(dx > 0.5));
            st.set_button(Button::DpadLeft, ButtonState::digital(dx < -0.5));
            st.set_button(Button::DpadUp, ButtonState::digital(dy > 0.5));
            st.set_button(Button::DpadDown, ButtonState::digital(dy < -0.5));
        }

        let trigger = |b: GButton, z: Axis| match gp.button_data(b) {
            Some(d) => d.value(),
            None => gp
                .axis_data(z)
                .map(|d| (d.value() + 1.0) * 0.5)
                .unwrap_or(0.0),
        };
        st.left_trigger = trigger(GButton::LeftTrigger2, Axis::LeftZ).clamp(0.0, 1.0);
        st.right_trigger = trigger(GButton::RightTrigger2, Axis::RightZ).clamp(0.0, 1.0);
        st.set_button(
            Button::LeftTrigger,
            ButtonState::analog(st.left_trigger, 0.12),
        );
        st.set_button(
            Button::RightTrigger,
            ButtonState::analog(st.right_trigger, 0.12),
        );
        st.left_stick = Vec2::new(axis(Axis::LeftStickX), axis(Axis::LeftStickY));
        st.right_stick = Vec2::new(axis(Axis::RightStickX), axis(Axis::RightStickY));

        let state = gp.state();
        let mut axes: Vec<RawAxisValue> = state
            .axes()
            .map(|(code, data)| RawAxisValue {
                code: code.into_u32(),
                label: gp
                    .axis_or_btn_name(code)
                    .map(|n| format!("{n:?}").replace("Axis(", "").replace(')', ""))
                    .unwrap_or_else(|| format!("{code}")),
                value: data.value(),
                raw: None,
            })
            .collect();
        axes.sort_by_key(|a| a.code);
        let mut buttons: Vec<RawButtonValue> = state
            .buttons()
            .map(|(code, data)| RawButtonValue {
                code: code.into_u32(),
                label: gp
                    .axis_or_btn_name(code)
                    .map(|n| format!("{n:?}").replace("Btn(", "").replace(')', ""))
                    .unwrap_or_else(|| format!("{code}")),
                pressed: data.is_pressed(),
                value: data.value(),
            })
            .collect();
        buttons.sort_by_key(|b| b.code);
        st.raw_axes = axes;
        st.raw_buttons = buttons;
        st.battery = power_to_battery(gp.power_info());
        st
    }

    fn ensure_rumble(&mut self, gid: GamepadId) -> Result<(), InputError> {
        let has = self.pads.get(&gid).is_some_and(|p| p.rumble.is_some());
        if has {
            return Ok(());
        }
        let make = |kind: BaseEffectType, gilrs: &mut Gilrs| {
            EffectBuilder::new()
                .add_effect(BaseEffect {
                    kind,
                    scheduling: Replay {
                        play_for: Ticks::from_ms(1000),
                        ..Default::default()
                    },
                    envelope: Default::default(),
                })
                .gamepads(&[gid])
                .gain(0.0)
                .finish(gilrs)
                .map_err(|e| InputError::Unsupported(format!("rumble ({e})")))
        };
        let weak = make(
            BaseEffectType::Weak {
                magnitude: u16::MAX,
            },
            &mut self.gilrs,
        )?;
        let strong = make(
            BaseEffectType::Strong {
                magnitude: u16::MAX,
            },
            &mut self.gilrs,
        )?;
        weak.play().map_err(|e| InputError::Other(e.to_string()))?;
        strong
            .play()
            .map_err(|e| InputError::Other(e.to_string()))?;
        if let Some(p) = self.pads.get_mut(&gid) {
            p.rumble = Some((weak, strong));
        }
        Ok(())
    }
}

fn power_to_battery(p: PowerInfo) -> Option<BatteryInfo> {
    match p {
        PowerInfo::Unknown => None,
        PowerInfo::Wired => Some(BatteryInfo {
            level: None,
            state: BatteryState::Wired,
        }),
        PowerInfo::Discharging(l) => Some(BatteryInfo {
            level: Some(l.min(100)),
            state: BatteryState::Discharging,
        }),
        PowerInfo::Charging(l) => Some(BatteryInfo {
            level: Some(l.min(100)),
            state: BatteryState::Charging,
        }),
        PowerInfo::Charged => Some(BatteryInfo {
            level: Some(100),
            state: BatteryState::Full,
        }),
    }
}

fn to_us(clock: &Clock, t: SystemTime) -> u64 {
    let now_us = clock.now_us();
    let age = SystemTime::now()
        .duration_since(t)
        .unwrap_or(Duration::ZERO)
        .as_micros() as u64;
    now_us.saturating_sub(age)
}

impl GamepadBackend for GilrsBackend {
    fn name(&self) -> &'static str {
        "gilrs"
    }

    fn poll(&mut self, clock: &Clock, out: &mut Vec<BackendEvent>) {
        if !self.started {
            // Controllers already plugged in at startup.
            self.started = true;
            let ids: Vec<GamepadId> = self.gilrs.gamepads().map(|(id, _)| id).collect();
            for gid in ids {
                let id = self.make_id(gid);
                let power = self.gilrs.gamepad(gid).power_info();
                out.push(BackendEvent::Connected(self.info(gid, &id)));
                self.pads.insert(
                    gid,
                    Pad {
                        id,
                        gid,
                        group_time: None,
                        rumble: None,
                        last_power: power,
                    },
                );
            }
        }

        let mut dirty: Vec<(GamepadId, SystemTime)> = Vec::new();
        while let Some(ev) = self.gilrs.next_event() {
            let gid = ev.id;
            match ev.event {
                EventType::Connected => {
                    let id = self.make_id(gid);
                    let power = self.gilrs.gamepad(gid).power_info();
                    out.push(BackendEvent::Connected(self.info(gid, &id)));
                    self.pads.insert(
                        gid,
                        Pad {
                            id,
                            gid,
                            group_time: None,
                            rumble: None,
                            last_power: power,
                        },
                    );
                }
                EventType::Disconnected => {
                    if let Some(p) = self.pads.remove(&gid) {
                        out.push(BackendEvent::Disconnected(p.id));
                    }
                }
                EventType::ForceFeedbackEffectCompleted | EventType::Dropped => {}
                _ => {
                    // A new timestamp closes the previous event group: emit it
                    // as one state so the interval reflects report timing.
                    let pending = self.pads.get(&gid).and_then(|p| p.group_time);
                    if let Some(prev) = pending {
                        if prev != ev.time {
                            let id = self.pads[&gid].id.clone();
                            out.push(BackendEvent::State(id, self.state(gid, to_us(clock, prev))));
                        }
                    }
                    if let Some(p) = self.pads.get_mut(&gid) {
                        p.group_time = Some(ev.time);
                    }
                    if !dirty.iter().any(|(g, _)| *g == gid) {
                        dirty.push((gid, ev.time));
                    }
                }
            }
        }
        for (gid, _) in dirty {
            if let Some(p) = self.pads.get_mut(&gid) {
                if let Some(t) = p.group_time.take() {
                    let id = p.id.clone();
                    out.push(BackendEvent::State(id, self.state(gid, to_us(clock, t))));
                }
            }
        }

        // Battery changes arrive without events; check cheaply.
        let gids: Vec<GamepadId> = self.pads.keys().copied().collect();
        for gid in gids {
            let power = self.gilrs.gamepad(gid).power_info();
            let changed = self.pads.get(&gid).is_some_and(|p| p.last_power != power);
            if changed {
                let id = self.pads[&gid].id.clone();
                if let Some(p) = self.pads.get_mut(&gid) {
                    p.last_power = power;
                }
                out.push(BackendEvent::Updated(self.info(gid, &id)));
            }
        }
    }

    fn send(&mut self, id: &ControllerId, cmd: &OutputCommand) -> Result<(), InputError> {
        let gid = self
            .pads
            .values()
            .find(|p| &p.id == id)
            .map(|p| p.gid)
            .ok_or_else(|| InputError::Disconnected(id.to_string()))?;
        match cmd {
            OutputCommand::Rumble { weak, strong } => {
                if !self.gilrs.gamepad(gid).is_ff_supported() {
                    return Err(InputError::Unsupported("rumble".into()));
                }
                self.ensure_rumble(gid)?;
                let pad = self.pads.get(&gid).expect("pad");
                let (w, s) = pad.rumble.as_ref().expect("effects");
                w.set_gain(weak.clamp(0.0, 1.0))
                    .and_then(|_| s.set_gain(strong.clamp(0.0, 1.0)))
                    .map_err(|e| InputError::Other(e.to_string()))
            }
            OutputCommand::TriggerRumble { .. } => {
                Err(InputError::Unsupported("impulse triggers".into()))
            }
            OutputCommand::TriggerEffects { .. } => {
                Err(InputError::Unsupported("adaptive triggers".into()))
            }
            OutputCommand::Lightbar { .. } | OutputCommand::PlayerLeds { .. } => {
                Err(InputError::Unsupported("lights".into()))
            }
        }
    }
}
