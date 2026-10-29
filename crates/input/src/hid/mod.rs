//! Raw HID backend (hidapi) for controllers whose full feature set the OS
//! gamepad APIs do not expose: DualShock 4, DualSense (gyro, accelerometer,
//! touchpad, battery, lightbar, adaptive triggers, true report rate) and,
//! on request, the Switch Pro Controller IMU.

pub mod common;
pub mod crc;
pub mod ds4;
pub mod ds5;
pub mod switch;

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::mapping::{self, hid_capabilities};
use crate::model::*;
use hidapi::{BusType, HidApi, HidDevice};
use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const SCAN_EVERY: Duration = Duration::from_millis(1500);
const READ_TIMEOUT_MS: i32 = 4;

/// Protocol driver for one device family.
pub trait Driver: Send {
    /// Feature report to read right after opening (id, length).
    fn feature_report(&self) -> Option<(u8, usize)> {
        None
    }
    fn on_feature_report(&mut self, _buf: &[u8]) {}
    /// Output reports to write right after opening.
    fn init_reports(&mut self) -> Vec<Vec<u8>> {
        Vec::new()
    }
    fn parse(&mut self, report: &[u8], timestamp_us: u64) -> Option<DeviceState>;
    /// Applies an output command and returns the report to write.
    fn output(&mut self, cmd: &OutputCommand) -> Result<Vec<u8>, InputError>;
}

enum Msg {
    State(ControllerId, DeviceState),
    Updated(ControllerInfo),
    Closed(ControllerId, Option<String>),
}

type Reply = Sender<Result<(), InputError>>;

struct OpenDevice {
    id: ControllerId,
    cmd_tx: Sender<(OutputCommand, Reply)>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for OpenDevice {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub struct HidBackend {
    api: HidApi,
    open: HashMap<CString, OpenDevice>,
    denied: HashSet<CString>,
    extended: HashSet<(u16, u16)>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    last_scan: Option<Instant>,
}

impl HidBackend {
    pub fn new() -> Result<Self, InputError> {
        // We enumerate only the vendors we care about, on our own schedule.
        HidApi::disable_device_discovery();
        let api =
            HidApi::new().map_err(|e| InputError::BackendUnavailable(format!("hidapi: {e}")))?;
        // Never seize the device on macOS: the OS (and gilrs) keep reading it.
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false);
        let (tx, rx) = mpsc::channel();
        Ok(Self {
            api,
            open: HashMap::new(),
            denied: HashSet::new(),
            extended: HashSet::new(),
            tx,
            rx,
            last_scan: None,
        })
    }

    fn wanted(&self, vid: u16, pid: u16) -> bool {
        mapping::is_dualsense(vid, pid)
            || mapping::is_dualshock4(vid, pid)
            || (mapping::is_switch_pro(vid, pid) && self.extended.contains(&(vid, pid)))
    }

    fn scan(&mut self, clock: &Clock, out: &mut Vec<BackendEvent>) {
        let mut seen: HashSet<CString> = HashSet::new();
        for vid in [mapping::VID_SONY, mapping::VID_NINTENDO] {
            if let Err(e) = self.api.add_devices(vid, 0) {
                log::debug!("hid enumerate {vid:04x}: {e}");
            }
        }
        let candidates: Vec<_> = self
            .api
            .device_list()
            .filter(|d| self.wanted(d.vendor_id(), d.product_id()))
            .filter(|d| {
                let (page, usage) = (d.usage_page(), d.usage());
                page == 0 || (page == 0x01 && (usage == 0x04 || usage == 0x05))
            })
            .map(|d| {
                (
                    d.path().to_owned(),
                    d.vendor_id(),
                    d.product_id(),
                    d.serial_number().map(str::to_string),
                    d.product_string().unwrap_or_default().to_string(),
                    d.bus_type(),
                )
            })
            .collect();
        self.api.reset_devices().ok();

        for (path, vid, pid, serial, product, bus) in candidates {
            if !seen.insert(path.clone()) || self.open.contains_key(&path) {
                continue;
            }
            if self.denied.contains(&path) {
                continue;
            }
            match self.api.open_path(&path) {
                Ok(dev) => {
                    let info = make_info(vid, pid, serial.as_deref(), &product, bus, &path);
                    log::info!("hid: opened {} ({:?})", info.name, bus);
                    let handle = spawn_device(dev, info.clone(), *clock, self.tx.clone());
                    self.open.insert(path, handle);
                    out.push(BackendEvent::Connected(info));
                }
                Err(e) => {
                    let msg = e.to_string();
                    log::warn!("hid: cannot open {path:?}: {msg}");
                    self.denied.insert(path.clone());
                    let lower = msg.to_lowercase();
                    let error = if lower.contains("permission") || lower.contains("access") {
                        InputError::PermissionDenied(format!(
                            "{} ({vid:04x}:{pid:04x})",
                            mapping::friendly_name(vid, pid, &product)
                        ))
                    } else {
                        InputError::Hid(msg)
                    };
                    out.push(BackendEvent::Error(error));
                }
            }
        }
        // Forget denials for devices that went away so a replug retries.
        self.denied.retain(|p| seen.contains(p));
        // Devices no longer enumerated: their reader thread also notices.
        let gone: Vec<CString> = self
            .open
            .keys()
            .filter(|p| !seen.contains(*p))
            .cloned()
            .collect();
        for p in gone {
            if let Some(d) = self.open.remove(&p) {
                out.push(BackendEvent::Disconnected(d.id.clone()));
            }
        }
    }
}

fn make_info(
    vid: u16,
    pid: u16,
    serial: Option<&str>,
    product: &str,
    bus: BusType,
    path: &CString,
) -> ControllerInfo {
    let kind = mapping::detect_kind(vid, pid, product);
    let serial = serial
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let suffix = serial.clone().unwrap_or_else(|| {
        let h = path
            .as_bytes()
            .iter()
            .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(*b as u32));
        format!("p{h:08x}")
    });
    ControllerInfo {
        id: ControllerId::new(format!("hid:{vid:04x}:{pid:04x}:{}", suffix.to_lowercase())),
        name: mapping::friendly_name(vid, pid, product),
        vendor_id: vid,
        product_id: pid,
        serial: serial.clone(),
        kind,
        connection: match bus {
            BusType::Usb => ConnectionType::Usb,
            BusType::Bluetooth => ConnectionType::Bluetooth,
            _ => ConnectionType::Unknown,
        },
        button_count: match kind {
            ControllerKind::Ps5 if pid == mapping::PID_DUALSENSE_EDGE => 23,
            ControllerKind::Ps5 => 19,
            _ => 18,
        },
        axis_count: 6,
        capabilities: hid_capabilities(kind),
        battery: None,
        backend: "hid".into(),
        is_virtual: false,
        is_demo: false,
        profile_key: ControllerInfo::make_profile_key(vid, pid, serial.as_deref()),
    }
}

fn spawn_device(dev: HidDevice, info: ControllerInfo, clock: Clock, tx: Sender<Msg>) -> OpenDevice {
    let (cmd_tx, cmd_rx) = mpsc::channel::<(OutputCommand, Reply)>();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_t = stop.clone();
    let id = info.id.clone();
    let thread = std::thread::Builder::new()
        .name(format!("hid-{}", info.id))
        .spawn(move || device_loop(dev, info, clock, tx, cmd_rx, stop_t))
        .expect("spawn hid thread");
    OpenDevice {
        id,
        cmd_tx,
        stop,
        thread: Some(thread),
    }
}

fn make_driver(info: &ControllerInfo) -> Box<dyn Driver> {
    let bt = info.connection == ConnectionType::Bluetooth;
    match info.kind {
        ControllerKind::Ps5 => Box::new(ds5::DualSense::new(
            bt,
            info.product_id == mapping::PID_DUALSENSE_EDGE,
        )),
        ControllerKind::SwitchPro => Box::new(switch::SwitchPro::new(!bt)),
        _ => Box::new(ds4::DualShock4::new(bt)),
    }
}

fn device_loop(
    dev: HidDevice,
    mut info: ControllerInfo,
    clock: Clock,
    tx: Sender<Msg>,
    cmd_rx: Receiver<(OutputCommand, Reply)>,
    stop: Arc<AtomicBool>,
) {
    let mut driver = make_driver(&info);
    if let Some((id, len)) = driver.feature_report() {
        let mut buf = vec![0u8; len];
        buf[0] = id;
        match dev.get_feature_report(&mut buf) {
            Ok(n) => driver.on_feature_report(&buf[..n]),
            Err(e) => log::warn!("{}: feature report 0x{id:02x} failed: {e}", info.name),
        }
    }
    for report in driver.init_reports() {
        if let Err(e) = dev.write(&report) {
            log::warn!("{}: init write failed: {e}", info.name);
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let mut buf = [0u8; 128];
    let mut last_battery: Option<BatteryInfo> = None;
    let mut reason: Option<String> = None;
    while !stop.load(Ordering::Relaxed) {
        match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(0) => {}
            Ok(n) => {
                let t = clock.now_us();
                if let Some(st) = driver.parse(&buf[..n], t) {
                    if st.battery != last_battery && st.battery.is_some() {
                        last_battery = st.battery;
                        info.battery = st.battery;
                        let _ = tx.send(Msg::Updated(info.clone()));
                    }
                    if tx.send(Msg::State(info.id.clone(), st)).is_err() {
                        return;
                    }
                }
            }
            Err(e) => {
                reason = Some(e.to_string());
                break;
            }
        }
        while let Ok((cmd, reply)) = cmd_rx.try_recv() {
            let result = driver.output(&cmd).and_then(|report| {
                dev.write(&report)
                    .map(|_| ())
                    .map_err(|e| InputError::Hid(e.to_string()))
            });
            let _ = reply.send(result);
        }
    }
    let _ = tx.send(Msg::Closed(info.id.clone(), reason));
}

impl GamepadBackend for HidBackend {
    fn name(&self) -> &'static str {
        "hid"
    }

    fn priority(&self) -> u8 {
        2
    }

    fn poll(&mut self, clock: &Clock, out: &mut Vec<BackendEvent>) {
        if self.last_scan.is_none_or(|t| t.elapsed() >= SCAN_EVERY) {
            self.last_scan = Some(Instant::now());
            self.scan(clock, out);
        }
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::State(id, st) => out.push(BackendEvent::State(id, st)),
                Msg::Updated(info) => out.push(BackendEvent::Updated(info)),
                Msg::Closed(id, reason) => {
                    if let Some(r) = reason {
                        log::info!("hid: {id} closed: {r}");
                    }
                    let path = self
                        .open
                        .iter()
                        .find(|(_, d)| d.id == id)
                        .map(|(p, _)| p.clone());
                    if let Some(p) = path {
                        self.open.remove(&p);
                        out.push(BackendEvent::Disconnected(id));
                    }
                }
            }
        }
    }

    fn send(&mut self, id: &ControllerId, cmd: &OutputCommand) -> Result<(), InputError> {
        let dev = self
            .open
            .values()
            .find(|d| &d.id == id)
            .ok_or_else(|| InputError::Disconnected(id.to_string()))?;
        let (tx, rx) = mpsc::channel();
        dev.cmd_tx
            .send((cmd.clone(), tx))
            .map_err(|_| InputError::Disconnected(id.to_string()))?;
        rx.recv_timeout(Duration::from_millis(250))
            .unwrap_or_else(|_| Err(InputError::Hid("device did not answer".into())))
    }

    fn enable_extended(&mut self, info: &ControllerInfo) -> Result<bool, InputError> {
        if mapping::is_switch_pro(info.vendor_id, info.product_id) {
            self.extended.insert((info.vendor_id, info.product_id));
            self.last_scan = None; // open it on the next poll
            return Ok(true);
        }
        Ok(false)
    }
}
