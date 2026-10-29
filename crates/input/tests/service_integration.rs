//! End-to-end tests of the input service with scripted (fake) backends, so
//! CI exercises hotplug, dedupe, normalisation, captures and output routing
//! without any physical controller.

use gamepadlab_calibration::Vec2;
use gamepadlab_input::scripted::{ScriptHandle, ScriptedBackend};
use gamepadlab_input::*;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

fn wait_until(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn next_event(rx: &Receiver<ServiceEvent>) -> ServiceEvent {
    rx.recv_timeout(Duration::from_secs(3))
        .expect("service event")
}

fn start_with(
    backends: Vec<(&'static str, u8)>,
) -> (InputService, Vec<ScriptHandle>, Receiver<ServiceEvent>) {
    let (tx, rx) = mpsc::channel();
    let mut boxed: Vec<Box<dyn GamepadBackend>> = Vec::new();
    let mut handles = Vec::new();
    for (name, prio) in backends {
        let (b, h) = ScriptedBackend::new(name, prio);
        boxed.push(Box::new(b));
        handles.push(h);
    }
    (
        InputService::start(ServiceConfig::default(), boxed, tx),
        handles,
        rx,
    )
}

fn state(t: u64, lx: f32, ly: f32, south: bool) -> DeviceState {
    let mut s = DeviceState::neutral(t);
    s.left_stick = Vec2::new(lx, ly);
    s.set_button(Button::South, ButtonState::digital(south));
    s.raw_axes = vec![RawAxisValue {
        code: 0,
        label: "LX".into(),
        value: lx,
        raw: None,
    }];
    s.raw_buttons = vec![RawButtonValue {
        code: 0,
        label: "A".into(),
        pressed: south,
        value: if south { 1.0 } else { 0.0 },
    }];
    s.report_based = true;
    s
}

#[test]
fn hotplug_states_and_polling_rate() {
    let (svc, h, rx) = start_with(vec![("scripted", 1)]);
    let pad = ScriptedBackend::controller("pad-1", 0x045e, 0x0b13, "scripted");
    h[0].connect(pad.clone());
    assert!(matches!(next_event(&rx), ServiceEvent::Connected(i) if i.id == pad.id));
    assert_eq!(svc.controllers().len(), 1);
    assert_eq!(svc.controllers()[0].kind, ControllerKind::Xbox);

    // 500 reports, 1 ms apart → 1000 Hz.
    for i in 0..500u64 {
        h[0].state(
            &pad.id,
            state(
                10_000 + i * 1000,
                (i as f32 / 500.0) * 2.0 - 1.0,
                0.0,
                i % 50 == 0,
            ),
        );
    }
    assert!(wait_until(|| svc
        .latest(&pad.id)
        .is_some_and(|s| s.seq == 500)));
    let frame = svc.take_frame();
    assert_eq!(frame.len(), 1);
    let entry = &frame[0];
    assert!(
        (entry.stats.hz - 1000.0).abs() < 1.0,
        "hz = {}",
        entry.stats.hz
    );
    assert_eq!(entry.stats.source, IntervalSource::Report);
    assert_eq!(entry.stats.recent_intervals_us.len(), 499);
    // Min/max seen and press counting.
    let lx = &entry.snapshot.raw_axes[0];
    assert!(lx.min <= -0.99 && lx.max > 0.99);
    assert_eq!(entry.snapshot.raw_buttons[0].press_count, 10);
    assert_eq!(entry.snapshot.button_presses[Button::South.index()], 10);

    svc.reset_minmax(&pad.id);
    h[0].state(&pad.id, state(600_000, 0.5, 0.0, false));
    assert!(wait_until(|| svc.latest(&pad.id).is_some_and(|s| s
        .raw_axes[0]
        .min
        == 0.5
        && s.raw_axes[0].max == 0.5)));

    h[0].disconnect(&pad.id);
    assert!(matches!(next_event(&rx), ServiceEvent::Disconnected(i) if i.id == pad.id));
    assert!(wait_until(|| svc.controllers().is_empty()));
    assert!(svc.take_frame().is_empty());
}

#[test]
fn duplicate_device_is_listed_once() {
    let (svc, h, rx) = start_with(vec![("gilrs-like", 1), ("hid-like", 2)]);
    let os_copy = ScriptedBackend::controller("os", 0x054c, 0x0ce6, "gilrs-like");
    let hid_copy = ScriptedBackend::controller("hid", 0x054c, 0x0ce6, "hid-like");
    h[0].connect(os_copy.clone());
    assert!(matches!(next_event(&rx), ServiceEvent::Connected(i) if i.id == os_copy.id));
    h[1].connect(hid_copy.clone());
    assert!(matches!(next_event(&rx), ServiceEvent::Disconnected(i) if i.id == os_copy.id));
    assert!(matches!(next_event(&rx), ServiceEvent::Connected(i) if i.id == hid_copy.id));
    assert!(wait_until(|| svc.controllers().len() == 1));

    // States from the hidden copy are ignored.
    h[0].state(&os_copy.id, state(1, 0.9, 0.0, false));
    h[1].state(&hid_copy.id, state(1, -0.4, 0.0, false));
    assert!(wait_until(|| svc.latest(&hid_copy.id).is_some()));
    assert!(svc.latest(&os_copy.id).is_none());

    h[1].disconnect(&hid_copy.id);
    assert!(matches!(next_event(&rx), ServiceEvent::Disconnected(i) if i.id == hid_copy.id));
    assert!(matches!(next_event(&rx), ServiceEvent::Connected(i) if i.id == os_copy.id));
}

#[test]
fn capture_records_every_report() {
    let (svc, h, rx) = start_with(vec![("scripted", 1)]);
    let pad = ScriptedBackend::controller("pad", 1, 2, "scripted");
    h[0].connect(pad.clone());
    next_event(&rx);
    svc.capture_start(&pad.id, CaptureTarget::LeftStick);
    std::thread::sleep(Duration::from_millis(20));
    for i in 0..300u64 {
        let a = i as f32 / 300.0 * std::f32::consts::TAU;
        h[0].state(&pad.id, state(5_000 + i * 4000, a.cos(), a.sin(), false));
    }
    assert!(wait_until(|| svc
        .latest(&pad.id)
        .is_some_and(|s| s.seq == 300)));
    let cap = svc.capture_stop().expect("capture");
    assert_eq!(cap.samples.len(), 300);
    assert_eq!(cap.samples[0].t, 0);
    assert!((cap.duration_ms - 1196.0).abs() < 0.01);
    assert!((cap.samples[75].y - 1.0).abs() < 0.01);
    assert!(svc.capture_stop().is_none(), "capture is consumed");
}

#[test]
fn outputs_are_routed_to_the_owning_backend() {
    let (svc, h, rx) = start_with(vec![("a", 1), ("b", 1)]);
    let p1 = ScriptedBackend::controller("p1", 1, 1, "a");
    let p2 = ScriptedBackend::controller("p2", 2, 2, "b");
    h[0].connect(p1.clone());
    h[1].connect(p2.clone());
    next_event(&rx);
    next_event(&rx);
    let cmd = OutputCommand::Rumble {
        weak: 0.25,
        strong: 0.75,
    };
    svc.send_output(&p2.id, cmd.clone()).unwrap();
    assert!(h[0].sent().is_empty());
    assert_eq!(h[1].sent(), vec![(p2.id.clone(), cmd.clone())]);
    let err = svc
        .send_output(&ControllerId::new("nope"), cmd)
        .unwrap_err();
    assert_eq!(err.code(), "disconnected");
}

#[test]
fn demo_controller_lifecycle() {
    let (svc, _h, rx) = start_with(vec![]);
    svc.set_demo(true);
    let ev = next_event(&rx);
    let ServiceEvent::Connected(info) = ev else {
        panic!("expected connect, got {ev:?}")
    };
    assert!(info.is_demo);
    assert!(info.capabilities.gyro && info.capabilities.adaptive_triggers);
    assert!(wait_until(|| svc
        .latest(&info.id)
        .is_some_and(|s| s.seq > 50)));
    std::thread::sleep(Duration::from_millis(300));
    let frame = svc.take_frame();
    let stats = &frame[0].stats;
    assert!(stats.hz > 150.0 && stats.hz < 350.0, "demo hz {}", stats.hz);
    assert!(frame[0].snapshot.motion.is_some());

    svc.demo_scenario(DemoScenario::Idle);
    svc.capture_start(&info.id, CaptureTarget::LeftTrigger);
    std::thread::sleep(Duration::from_millis(200));
    let cap = svc.capture_stop().unwrap();
    assert!(cap.samples.len() > 20);

    svc.send_output(
        &info.id,
        OutputCommand::Rumble {
            weak: 1.0,
            strong: 1.0,
        },
    )
    .unwrap();
    svc.set_demo(false);
    assert!(matches!(next_event(&rx), ServiceEvent::Disconnected(i) if i.is_demo));
}
