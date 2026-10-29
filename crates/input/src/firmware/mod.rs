//! Calibration written to the controller itself ("firmware calibration").
//!
//! * DualShock 4 / DualSense: the factory stick-calibration commands
//!   reverse-engineered by the DualShock Calibration GUI project (MIT,
//!   © the_al). Calibrations first live in the controller's working memory
//!   (lost on power-off); "save permanently" is a separate, explicit step
//!   that cycles the NVS lock.
//! * Switch Pro: the user stick-calibration block in SPI flash (the same
//!   area the console's "Calibrate Control Sticks" writes). Writes are
//!   restricted to that block, backed up first and verified by reading back.
//!
//! Every operation is refused when a safety check fails: Bluetooth on Sony
//! controllers, clones, outdated DualSense firmware, DualSense Edge (needs a
//! hardware mod), low or unknown battery.

pub mod switch_cal;

use self::switch_cal::SwitchStickCal;
use crate::backend::InputError;
use crate::model::{BatteryInfo, BatteryState};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const MIN_BATTERY: u8 = 30;

/// Raw access to a HID device (real hidapi device or a test fake).
pub trait FeatureIo {
    /// `data[0]` is the report id; padded to the declared length.
    fn send_feature(&self, data: &[u8]) -> Result<(), String>;
    /// Returns the report with its id in byte 0.
    fn get_feature(&self, id: u8) -> Result<Vec<u8>, String>;
    fn write(&self, data: &[u8]) -> Result<(), String>;
    fn read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, String>;
    fn sleep_ms(&self, ms: u64);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FirmwareModel {
    Ds4,
    Ds5,
    Ds5Edge,
    SwitchPro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NvStatus {
    /// Calibrations are temporary until saved (the safe default).
    Locked,
    /// Calibrations are written straight to flash.
    Unlocked,
    PendingReboot,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FirmwarePhase {
    Idle,
    Center,
    Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Blocker {
    Bluetooth,
    Clone,
    OutdatedFirmware,
    EdgeNeedsHardwareMod,
    LowBattery,
    BatteryUnknown,
    PendingReboot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareStatus {
    pub model: FirmwareModel,
    pub simulated: bool,
    pub genuine: Option<bool>,
    pub clone_reason: Option<String>,
    pub firmware: Option<String>,
    pub usb: bool,
    pub battery: Option<u8>,
    pub nv: NvStatus,
    pub phase: FirmwarePhase,
    pub center_samples: u32,
    pub unsaved_changes: bool,
    pub blockers: Vec<Blocker>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchCalibrationData {
    pub factory_left: SwitchStickCal,
    pub factory_right: SwitchStickCal,
    pub user_left: Option<SwitchStickCal>,
    pub user_right: Option<SwitchStickCal>,
    pub user_block: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "op")]
pub enum FirmwareOp {
    Status,
    CenterBegin,
    CenterSample,
    CenterEnd,
    RangeBegin,
    RangeEnd,
    /// Commits the temporary calibration to flash (Sony).
    SavePermanent {
        acknowledge_risk: bool,
    },
    /// Reboots the controller, discarding unsaved changes (Sony).
    Reboot,
    SwitchRead,
    SwitchWrite {
        left: SwitchStickCal,
        right: SwitchStickCal,
        acknowledge_risk: bool,
    },
    /// Erases the user calibration: the controller falls back to factory data.
    SwitchClear {
        acknowledge_risk: bool,
    },
}

impl FirmwareOp {
    fn name(&self) -> &'static str {
        match self {
            FirmwareOp::Status => "status",
            FirmwareOp::CenterBegin => "centerBegin",
            FirmwareOp::CenterSample => "centerSample",
            FirmwareOp::CenterEnd => "centerEnd",
            FirmwareOp::RangeBegin => "rangeBegin",
            FirmwareOp::RangeEnd => "rangeEnd",
            FirmwareOp::SavePermanent { .. } => "savePermanent",
            FirmwareOp::Reboot => "reboot",
            FirmwareOp::SwitchRead => "switchRead",
            FirmwareOp::SwitchWrite { .. } => "switchWrite",
            FirmwareOp::SwitchClear { .. } => "switchClear",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareReply {
    pub status: FirmwareStatus,
    pub switch: Option<SwitchCalibrationData>,
    /// Bytes of the user calibration block before a Switch write (backup).
    pub backup: Option<Vec<u8>>,
}

fn protocol(msg: impl Into<String>) -> InputError {
    InputError::FirmwareProtocol(msg.into())
}

fn blocked(msg: impl Into<String>) -> InputError {
    InputError::FirmwareBlocked(msg.into())
}

fn be_u32(b: &[u8], i: usize) -> Option<u32> {
    b.get(i..i + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn ascii(b: &[u8]) -> String {
    String::from_utf8_lossy(b)
        .trim_matches(char::from(0))
        .trim()
        .to_string()
}

fn current_year() -> i64 {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    // Civil-from-days (Howard Hinnant), year only.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    yoe + era * 400 + i64::from(mp >= 10)
}

const NEUTRAL_RUMBLE: [u8; 8] = [0x00, 0x01, 0x40, 0x40, 0x00, 0x01, 0x40, 0x40];

/// Sends a Switch subcommand and waits for its 0x21 reply (3 attempts).
pub fn switch_subcommand(
    io: &dyn FeatureIo,
    counter: &mut u8,
    id: u8,
    args: &[u8],
) -> Result<Vec<u8>, InputError> {
    for _ in 0..3 {
        let mut r = vec![0u8; 49];
        r[0] = 0x01;
        r[1] = *counter & 0x0F;
        *counter = counter.wrapping_add(1);
        r[2..10].copy_from_slice(&NEUTRAL_RUMBLE);
        r[10] = id;
        r[11..11 + args.len()].copy_from_slice(args);
        io.write(&r).map_err(InputError::Hid)?;
        let deadline = Instant::now() + Duration::from_millis(600);
        while Instant::now() < deadline {
            let mut buf = [0u8; 64];
            let n = io.read(&mut buf, 50).map_err(InputError::Hid)?;
            if n > 15 && buf[0] == 0x21 && buf[14] == id {
                return Ok(buf[..n].to_vec());
            }
        }
    }
    Err(protocol(format!(
        "Switch: no reply to subcommand 0x{id:02X}"
    )))
}

pub fn switch_spi_read(
    io: &dyn FeatureIo,
    counter: &mut u8,
    addr: u32,
    len: u8,
) -> Result<Vec<u8>, InputError> {
    let mut args = addr.to_le_bytes().to_vec();
    args.push(len);
    let r = switch_subcommand(io, counter, 0x10, &args)?;
    let end = 20 + len as usize;
    if r.len() < end || r[13] & 0x80 == 0 || r[15..19] != addr.to_le_bytes() || r[19] != len {
        return Err(protocol(format!(
            "Switch: bad SPI read reply at 0x{addr:04X}"
        )));
    }
    Ok(r[20..end].to_vec())
}

/// The only flash range GamepadLab ever writes: the user stick calibration.
pub fn switch_write_allowed(addr: u32, len: usize) -> bool {
    addr >= switch_cal::USER_STICKS_ADDR
        && addr as usize + len
            <= (switch_cal::USER_STICKS_ADDR + switch_cal::USER_STICKS_LEN as u32) as usize
        && len > 0
}

fn switch_spi_write(
    io: &dyn FeatureIo,
    counter: &mut u8,
    addr: u32,
    data: &[u8],
) -> Result<(), InputError> {
    if !switch_write_allowed(addr, data.len()) {
        return Err(blocked(format!("refusing to write flash at 0x{addr:04X}")));
    }
    let mut args = addr.to_le_bytes().to_vec();
    args.push(data.len() as u8);
    args.extend_from_slice(data);
    let r = switch_subcommand(io, counter, 0x11, &args)?;
    match r.get(15) {
        Some(0) => Ok(()),
        Some(1) => Err(protocol("Switch: flash is write-protected")),
        other => Err(protocol(format!("Switch: SPI write failed ({other:?})"))),
    }
}

#[derive(Debug, Clone)]
struct SonyInfo {
    genuine: bool,
    clone_reason: Option<String>,
    firmware: Option<String>,
    outdated: bool,
}

/// Per-device firmware session: cached checks and the calibration state
/// machine. Lives on the device thread; dropped when the device goes away.
pub struct FirmwareSession {
    model: FirmwareModel,
    usb: bool,
    info: Option<SonyInfo>,
    phase: FirmwarePhase,
    samples: u32,
    unsaved: bool,
    counter: u8,
}

impl FirmwareSession {
    pub fn new(model: FirmwareModel, usb: bool) -> Self {
        Self {
            model,
            usb,
            info: None,
            phase: FirmwarePhase::Idle,
            samples: 0,
            unsaved: false,
            counter: 0,
        }
    }

    fn is_sony(&self) -> bool {
        self.model != FirmwareModel::SwitchPro
    }

    fn probe(&mut self, io: &dyn FeatureIo) -> SonyInfo {
        if let Some(i) = &self.info {
            return i.clone();
        }
        let info = match self.model {
            FirmwareModel::Ds4 => probe_ds4(io),
            FirmwareModel::Ds5 | FirmwareModel::Ds5Edge => probe_ds5(io),
            FirmwareModel::SwitchPro => SonyInfo {
                genuine: true,
                clone_reason: None,
                firmware: None,
                outdated: false,
            },
        };
        log::info!("firmware probe {:?}: {:?}", self.model, info);
        self.info = Some(info.clone());
        info
    }

    fn nv_status(&self, io: &dyn FeatureIo) -> NvStatus {
        let result = match self.model {
            FirmwareModel::Ds5 | FirmwareModel::Ds5Edge => io
                .send_feature(&[0x80, 3, 3])
                .and_then(|_| io.get_feature(0x81))
                .map(|r| match be_u32(&r, 1) {
                    Some(0x0303_0201) => NvStatus::Locked,
                    Some(0x0303_0200) => NvStatus::Unlocked,
                    Some(0x1501_0100) => NvStatus::PendingReboot,
                    _ => NvStatus::Unknown,
                }),
            FirmwareModel::Ds4 => io
                .send_feature(&[0x08, 0xFF, 0, 12])
                .and_then(|_| io.get_feature(0x11))
                .map(|r| match r.get(1) {
                    Some(1) => NvStatus::Locked,
                    Some(0) => NvStatus::Unlocked,
                    _ => NvStatus::Unknown,
                }),
            FirmwareModel::SwitchPro => Ok(NvStatus::NotApplicable),
        };
        result.unwrap_or(NvStatus::Unknown)
    }

    fn nv_lock(&self, io: &dyn FeatureIo) -> Result<(), InputError> {
        match self.model {
            FirmwareModel::Ds4 => io.send_feature(&[0xA0, 10, 1, 0]),
            _ => io
                .send_feature(&[0x80, 3, 1])
                .and_then(|_| io.get_feature(0x81).map(|_| ())),
        }
        .map_err(|e| protocol(format!("NVS lock failed: {e}")))
    }

    fn nv_unlock(&self, io: &dyn FeatureIo) -> Result<(), InputError> {
        match self.model {
            FirmwareModel::Ds4 => io.send_feature(&[0xA0, 10, 2, 0x3E, 0x71, 0x7F, 0x89]),
            _ => io
                .send_feature(&[0x80, 3, 2, 101, 50, 64, 12])
                .and_then(|_| io.get_feature(0x81).map(|_| ())),
        }
        .map_err(|e| protocol(format!("NVS unlock failed: {e}")))
    }

    /// Makes sure calibrations go to working memory only.
    fn ensure_temporary(&self, io: &dyn FeatureIo) -> Result<(), InputError> {
        match self.nv_status(io) {
            NvStatus::Locked => Ok(()),
            NvStatus::Unlocked => {
                log::warn!(
                    "{:?}: NVS was unlocked, locking before calibrating",
                    self.model
                );
                self.nv_lock(io)?;
                if self.nv_status(io) == NvStatus::Locked {
                    Ok(())
                } else {
                    Err(blocked("could not switch the controller to temporary mode"))
                }
            }
            other => Err(blocked(format!(
                "controller memory state is {other:?}; reboot it and try again"
            ))),
        }
    }

    pub fn status(&mut self, io: &dyn FeatureIo, battery: Option<BatteryInfo>) -> FirmwareStatus {
        let info = self.probe(io);
        let nv = self.nv_status(io);
        let level = battery.and_then(|b| b.level);
        let mut blockers = Vec::new();
        if self.is_sony() && !self.usb {
            blockers.push(Blocker::Bluetooth);
        }
        if !info.genuine {
            blockers.push(Blocker::Clone);
        }
        if info.outdated {
            blockers.push(Blocker::OutdatedFirmware);
        }
        if self.model == FirmwareModel::Ds5Edge {
            blockers.push(Blocker::EdgeNeedsHardwareMod);
        }
        match (level, battery.map(|b| b.state)) {
            (_, Some(BatteryState::Full)) => {}
            (Some(l), _) if l < MIN_BATTERY => blockers.push(Blocker::LowBattery),
            (None, _) => blockers.push(Blocker::BatteryUnknown),
            _ => {}
        }
        if nv == NvStatus::PendingReboot {
            blockers.push(Blocker::PendingReboot);
        }
        FirmwareStatus {
            model: self.model,
            simulated: false,
            genuine: Some(info.genuine),
            clone_reason: info.clone_reason,
            firmware: info.firmware,
            usb: self.usb,
            battery: level,
            nv,
            phase: self.phase,
            center_samples: self.samples,
            unsaved_changes: self.unsaved,
            blockers,
        }
    }

    pub fn execute(
        &mut self,
        io: &dyn FeatureIo,
        op: &FirmwareOp,
        battery: Option<BatteryInfo>,
    ) -> Result<FirmwareReply, InputError> {
        log::info!("firmware {:?}: {}", self.model, op.name());
        let status = self.status(io, battery);
        let reply = |s: &mut Self, switch, backup| {
            Ok(FirmwareReply {
                status: s.status(io, battery),
                switch,
                backup,
            })
        };
        let sony_only = |s: &Self| {
            if s.is_sony() {
                Ok(())
            } else {
                Err(InputError::Unsupported(
                    "this operation on a Switch Pro".into(),
                ))
            }
        };
        let switch_only = |s: &Self| {
            if s.is_sony() {
                Err(InputError::Unsupported(
                    "this operation on a Sony controller".into(),
                ))
            } else {
                Ok(())
            }
        };
        let unblocked = || {
            if status.blockers.is_empty() {
                Ok(())
            } else {
                Err(blocked(format!("{:?}", status.blockers)))
            }
        };
        match op {
            FirmwareOp::Status => reply(self, None, None),
            FirmwareOp::CenterBegin | FirmwareOp::RangeBegin => {
                sony_only(self)?;
                unblocked()?;
                if self.phase != FirmwarePhase::Idle {
                    return Err(protocol("another calibration is still running"));
                }
                self.ensure_temporary(io)?;
                let center = matches!(op, FirmwareOp::CenterBegin);
                self.sony_cal(
                    io,
                    if center { [1, 1, 1] } else { [1, 1, 2] },
                    if center { 0x0101 } else { 0x0201 },
                )?;
                self.phase = if center {
                    FirmwarePhase::Center
                } else {
                    FirmwarePhase::Range
                };
                self.samples = 0;
                reply(self, None, None)
            }
            FirmwareOp::CenterSample => {
                sony_only(self)?;
                if self.phase != FirmwarePhase::Center {
                    return Err(protocol("center calibration is not running"));
                }
                self.sony_cal(io, [3, 1, 1], 0x0101)?;
                self.samples += 1;
                reply(self, None, None)
            }
            FirmwareOp::CenterEnd => {
                sony_only(self)?;
                if self.phase != FirmwarePhase::Center || self.samples == 0 {
                    return Err(protocol("take at least one center sample first"));
                }
                let result = self.sony_cal(io, [2, 1, 1], 0x0102);
                self.phase = FirmwarePhase::Idle;
                result?;
                self.unsaved = true;
                reply(self, None, None)
            }
            FirmwareOp::RangeEnd => {
                sony_only(self)?;
                if self.phase != FirmwarePhase::Range {
                    return Err(protocol("range calibration is not running"));
                }
                let result = self.sony_cal(io, [2, 1, 2], 0x0202);
                self.phase = FirmwarePhase::Idle;
                result?;
                self.unsaved = true;
                reply(self, None, None)
            }
            FirmwareOp::SavePermanent { acknowledge_risk } => {
                sony_only(self)?;
                unblocked()?;
                if !acknowledge_risk {
                    return Err(blocked("the risk was not acknowledged"));
                }
                if !self.unsaved || self.phase != FirmwarePhase::Idle {
                    return Err(blocked(
                        "calibrate (temporarily) and check the result first",
                    ));
                }
                self.nv_unlock(io)?;
                let mut locked = self.nv_lock(io).is_ok() && self.nv_status(io) == NvStatus::Locked;
                if !locked {
                    io.sleep_ms(300);
                    locked = self.nv_lock(io).is_ok() && self.nv_status(io) == NvStatus::Locked;
                }
                if !locked {
                    return Err(protocol(
                        "saved, but the memory stayed unlocked: reboot the controller",
                    ));
                }
                self.unsaved = false;
                reply(self, None, None)
            }
            FirmwareOp::Reboot => {
                sony_only(self)?;
                let cmd: &[u8] = if self.model == FirmwareModel::Ds4 {
                    &[0xA0, 4, 1, 0]
                } else {
                    &[0x80, 1, 1]
                };
                // The controller drops off the bus immediately; errors are expected.
                let _ = io.send_feature(cmd);
                self.unsaved = false;
                self.phase = FirmwarePhase::Idle;
                Ok(FirmwareReply {
                    status: FirmwareStatus {
                        unsaved_changes: false,
                        phase: FirmwarePhase::Idle,
                        ..status
                    },
                    switch: None,
                    backup: None,
                })
            }
            FirmwareOp::SwitchRead => {
                switch_only(self)?;
                let data = self.switch_read(io)?;
                reply(self, Some(data), None)
            }
            FirmwareOp::SwitchWrite {
                left,
                right,
                acknowledge_risk,
            } => {
                switch_only(self)?;
                unblocked()?;
                if !acknowledge_risk {
                    return Err(blocked("the risk was not acknowledged"));
                }
                let mut problems = left.problems();
                problems.extend(right.problems());
                if !problems.is_empty() {
                    return Err(blocked(problems.join("; ")));
                }
                let before = self.switch_read(io)?;
                let block = switch_cal::user_block(left, right);
                self.switch_write_verified(io, &block)?;
                let after = self.switch_read(io)?;
                reply(self, Some(after), Some(before.user_block))
            }
            FirmwareOp::SwitchClear { acknowledge_risk } => {
                switch_only(self)?;
                unblocked()?;
                if !acknowledge_risk {
                    return Err(blocked("the risk was not acknowledged"));
                }
                let before = self.switch_read(io)?;
                self.switch_write_verified(io, &[0xFF; 22])?;
                let after = self.switch_read(io)?;
                reply(self, Some(after), Some(before.user_block))
            }
        }
    }

    fn sony_cal(&self, io: &dyn FeatureIo, cmd: [u8; 3], expect: u16) -> Result<(), InputError> {
        let fail =
            |e: String| protocol(format!("{:?} calibration command {cmd:?}: {e}", self.model));
        if self.model == FirmwareModel::Ds4 {
            io.send_feature(&[0x90, cmd[0], cmd[1], cmd[2]])
                .map_err(fail)?;
            io.sleep_ms(200);
            let a = io.get_feature(0x91).map_err(fail)?;
            let b = io.get_feature(0x92).map_err(fail)?;
            let want_a = 0x9101_0000 | expect as u32;
            // 0x92 echoes the stage: 0x92xxxxFF while running, 0x92010101/0x92010201 at the end.
            let want_b = match expect {
                0x0101 => 0x9201_01FF,
                0x0201 => 0x9201_02FF,
                0x0102 => 0x9201_0101,
                _ => 0x9201_0201,
            };
            let (ga, gb) = (be_u32(&a, 0), be_u32(&b, 0));
            if ga != Some(want_a) || gb != Some(want_b) {
                return Err(fail(format!("unexpected reply {ga:08X?} {gb:08X?}")));
            }
        } else {
            io.send_feature(&[0x82, cmd[0], cmd[1], cmd[2]])
                .map_err(fail)?;
            let r = io.get_feature(0x83).map_err(fail)?;
            let want = 0x8301_0000 | expect as u32;
            let got = be_u32(&r, 0);
            if got != Some(want) {
                return Err(fail(format!(
                    "unexpected reply {got:08X?}, expected {want:08X}"
                )));
            }
        }
        Ok(())
    }

    fn switch_read(&mut self, io: &dyn FeatureIo) -> Result<SwitchCalibrationData, InputError> {
        let factory = switch_spi_read(
            io,
            &mut self.counter,
            switch_cal::FACTORY_STICKS_ADDR,
            switch_cal::FACTORY_STICKS_LEN,
        )?;
        let user = switch_spi_read(
            io,
            &mut self.counter,
            switch_cal::USER_STICKS_ADDR,
            switch_cal::USER_STICKS_LEN,
        )?;
        let (user_left, user_right) = switch_cal::parse_user_block(&user);
        Ok(SwitchCalibrationData {
            factory_left: SwitchStickCal::decode_left(&factory[0..9]),
            factory_right: SwitchStickCal::decode_right(&factory[9..18]),
            user_left,
            user_right,
            user_block: user,
        })
    }

    fn switch_write_verified(
        &mut self,
        io: &dyn FeatureIo,
        block: &[u8],
    ) -> Result<(), InputError> {
        switch_spi_write(io, &mut self.counter, switch_cal::USER_STICKS_ADDR, block)?;
        io.sleep_ms(50);
        let back = switch_spi_read(
            io,
            &mut self.counter,
            switch_cal::USER_STICKS_ADDR,
            switch_cal::USER_STICKS_LEN,
        )?;
        if back != block {
            return Err(protocol(format!(
                "Switch: verification failed, flash holds {:02X?}",
                back
            )));
        }
        Ok(())
    }
}

fn probe_ds5(io: &dyn FeatureIo) -> SonyInfo {
    let mut info = SonyInfo {
        genuine: true,
        clone_reason: None,
        firmware: None,
        outdated: false,
    };
    match io.get_feature(0x20) {
        Ok(r) if r.len() >= 64 && r[0] == 0x20 => {
            let date = ascii(&r[1..12]);
            let time = ascii(&r[12..20]);
            info.outdated = date.contains(" 2020") || date.contains(" 2021");
            let year: i64 = date
                .rsplit(' ')
                .next()
                .and_then(|y| y.parse().ok())
                .unwrap_or(0);
            if year > current_year() {
                info.genuine = false;
                info.clone_reason = Some("buildDate".into());
            }
            info.firmware = Some(format!("{date} {time}"));
        }
        _ => {
            info.genuine = false;
            info.clone_reason = Some("info".into());
            return info;
        }
    }
    // Every genuine DualSense answers the stick fine-tune query.
    let ok = io.send_feature(&[0x80, 12, 2]).is_ok()
        && {
            io.sleep_ms(100);
            matches!(io.get_feature(0x81), Ok(r) if r.len() >= 4 && r[0] == 0x81 && r[1] == 12 && (r[2] == 2 || r[2] == 4) && r[3] == 2)
        };
    if !ok && info.genuine {
        info.genuine = false;
        info.clone_reason = Some("finetune".into());
    }
    info
}

fn probe_ds4(io: &dyn FeatureIo) -> SonyInfo {
    match io.get_feature(0xA3) {
        Ok(r) if r.len() == 49 && r[0] == 0xA3 => {
            let firmware = Some(format!("{} {}", ascii(&r[1..0x10]), ascii(&r[0x10..0x20])));
            // Clones do not implement this report.
            let genuine = io.get_feature(0x81).is_ok();
            SonyInfo {
                genuine,
                clone_reason: (!genuine).then(|| "probe".into()),
                firmware,
                outdated: false,
            }
        }
        _ => SonyInfo {
            genuine: false,
            clone_reason: Some("info".into()),
            firmware: None,
            outdated: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};

    /// Scripted fake: records sends, answers get_feature from queues, and
    /// emulates the Switch SPI flash for subcommands 0x10/0x11.
    #[derive(Default)]
    struct Fake {
        sent: RefCell<Vec<Vec<u8>>>,
        features: RefCell<HashMap<u8, VecDeque<Vec<u8>>>>,
        sticky: RefCell<HashMap<u8, Vec<u8>>>,
        flash: RefCell<HashMap<u32, u8>>,
        inbox: RefCell<VecDeque<Vec<u8>>>,
        writes: RefCell<Vec<Vec<u8>>>,
        ignore_flash_writes: bool,
    }

    impl Fake {
        fn answer(&self, id: u8, data: Vec<u8>) {
            self.features
                .borrow_mut()
                .entry(id)
                .or_default()
                .push_back(data);
        }
        fn always(&self, id: u8, data: Vec<u8>) {
            self.sticky.borrow_mut().insert(id, data);
        }
        fn sent_cmds(&self) -> Vec<Vec<u8>> {
            self.sent.borrow().clone()
        }
    }

    impl FeatureIo for Fake {
        fn send_feature(&self, data: &[u8]) -> Result<(), String> {
            self.sent.borrow_mut().push(data.to_vec());
            Ok(())
        }
        fn get_feature(&self, id: u8) -> Result<Vec<u8>, String> {
            if let Some(v) = self
                .features
                .borrow_mut()
                .get_mut(&id)
                .and_then(VecDeque::pop_front)
            {
                return Ok(v);
            }
            self.sticky
                .borrow()
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("no report 0x{id:02X}"))
        }
        fn write(&self, data: &[u8]) -> Result<(), String> {
            self.writes.borrow_mut().push(data.to_vec());
            let sub = data[10];
            let addr = u32::from_le_bytes([data[11], data[12], data[13], data[14]]);
            let len = data[15] as usize;
            let mut reply = vec![0u8; 64];
            reply[0] = 0x21;
            reply[14] = sub;
            match sub {
                0x10 => {
                    reply[13] = 0x90;
                    reply[15..19].copy_from_slice(&addr.to_le_bytes());
                    reply[19] = len as u8;
                    let flash = self.flash.borrow();
                    for k in 0..len {
                        reply[20 + k] = *flash.get(&(addr + k as u32)).unwrap_or(&0xFF);
                    }
                }
                0x11 => {
                    reply[13] = 0x80;
                    if !self.ignore_flash_writes {
                        let mut flash = self.flash.borrow_mut();
                        for k in 0..len {
                            flash.insert(addr + k as u32, data[16 + k]);
                        }
                    }
                }
                _ => {}
            }
            // A streaming 0x30 report arrives before the reply.
            let mut noise = vec![0u8; 64];
            noise[0] = 0x30;
            self.inbox.borrow_mut().push_back(noise);
            self.inbox.borrow_mut().push_back(reply);
            Ok(())
        }
        fn read(&self, buf: &mut [u8], _t: i32) -> Result<usize, String> {
            match self.inbox.borrow_mut().pop_front() {
                Some(v) => {
                    buf[..v.len()].copy_from_slice(&v);
                    Ok(v.len())
                }
                None => Ok(0),
            }
        }
        fn sleep_ms(&self, _ms: u64) {}
    }

    fn full_battery() -> Option<BatteryInfo> {
        Some(BatteryInfo {
            level: Some(80),
            state: BatteryState::Charging,
        })
    }

    fn genuine_ds5() -> Fake {
        let f = Fake::default();
        let mut info = vec![0u8; 64];
        info[0] = 0x20;
        info[1..12].copy_from_slice(b"Sep 12 2024");
        info[12..20].copy_from_slice(b"10:00:00");
        f.always(0x20, info);
        // Fine-tune probe reply, then NV status "locked" for every query.
        f.answer(0x81, vec![0x81, 12, 2, 2, 0, 0]);
        f.always(0x81, vec![0x81, 0x03, 0x03, 0x02, 0x01]);
        f.always(0x83, vec![0x83, 0x01, 0x01, 0x01]);
        f
    }

    #[test]
    fn ds5_status_and_center_flow_uses_reference_commands() {
        let f = genuine_ds5();
        let mut s = FirmwareSession::new(FirmwareModel::Ds5, true);
        let st = s
            .execute(&f, &FirmwareOp::Status, full_battery())
            .unwrap()
            .status;
        assert_eq!(st.genuine, Some(true));
        assert_eq!(st.nv, NvStatus::Locked);
        assert!(st.blockers.is_empty(), "{:?}", st.blockers);
        assert_eq!(st.firmware.as_deref(), Some("Sep 12 2024 10:00:00"));

        s.execute(&f, &FirmwareOp::CenterBegin, full_battery())
            .unwrap();
        for _ in 0..4 {
            s.execute(&f, &FirmwareOp::CenterSample, full_battery())
                .unwrap();
        }
        f.answer(0x83, vec![0x83, 0x01, 0x01, 0x02]);
        let st = s
            .execute(&f, &FirmwareOp::CenterEnd, full_battery())
            .unwrap()
            .status;
        assert!(st.unsaved_changes);
        assert_eq!(st.phase, FirmwarePhase::Idle);
        let cal: Vec<Vec<u8>> = f.sent_cmds().into_iter().filter(|c| c[0] == 0x82).collect();
        assert_eq!(cal[0], vec![0x82, 1, 1, 1]);
        assert_eq!(cal[1], vec![0x82, 3, 1, 1]);
        assert_eq!(cal.last().unwrap(), &vec![0x82, 2, 1, 1]);
    }

    #[test]
    fn unexpected_reply_is_an_error_and_resets_phase() {
        let f = genuine_ds5();
        let mut s = FirmwareSession::new(FirmwareModel::Ds5, true);
        s.execute(&f, &FirmwareOp::CenterBegin, full_battery())
            .unwrap();
        s.execute(&f, &FirmwareOp::CenterSample, full_battery())
            .unwrap();
        f.answer(0x83, vec![0x83, 0x01, 0x01, 0x07]);
        let err = s
            .execute(&f, &FirmwareOp::CenterEnd, full_battery())
            .unwrap_err();
        assert_eq!(err.code(), "firmwareProtocol");
        let st = s
            .execute(&f, &FirmwareOp::Status, full_battery())
            .unwrap()
            .status;
        assert_eq!(st.phase, FirmwarePhase::Idle);
        assert!(!st.unsaved_changes);
    }

    #[test]
    fn save_permanent_requires_a_temporary_calibration_and_consent() {
        let f = genuine_ds5();
        let mut s = FirmwareSession::new(FirmwareModel::Ds5, true);
        let op = FirmwareOp::SavePermanent {
            acknowledge_risk: true,
        };
        assert_eq!(
            s.execute(&f, &op, full_battery()).unwrap_err().code(),
            "firmwareBlocked"
        );
        f.always(0x83, vec![0x83, 0x01, 0x02, 0x01]);
        s.execute(&f, &FirmwareOp::RangeBegin, full_battery())
            .unwrap();
        f.answer(0x83, vec![0x83, 0x01, 0x02, 0x02]);
        s.execute(&f, &FirmwareOp::RangeEnd, full_battery())
            .unwrap();
        let no_consent = FirmwareOp::SavePermanent {
            acknowledge_risk: false,
        };
        assert!(s.execute(&f, &no_consent, full_battery()).is_err());
        let st = s.execute(&f, &op, full_battery()).unwrap().status;
        assert!(!st.unsaved_changes);
        let sent = f.sent_cmds();
        let unlock = sent
            .iter()
            .position(|c| c == &vec![0x80, 3, 2, 101, 50, 64, 12])
            .unwrap();
        let lock = sent.iter().rposition(|c| c == &vec![0x80, 3, 1]).unwrap();
        assert!(unlock < lock, "unlock then lock commits to flash");
    }

    #[test]
    fn unlocked_controller_is_locked_before_calibrating() {
        let f = genuine_ds5();
        f.answer(0x81, vec![0x81, 0x03, 0x03, 0x02, 0x00]); // status: unlocked (permanent)
        f.answer(0x81, vec![0x81, 0x03, 0x03, 0x02, 0x00]);
        let mut s = FirmwareSession::new(FirmwareModel::Ds5, true);
        s.execute(&f, &FirmwareOp::CenterBegin, full_battery())
            .unwrap();
        assert!(f.sent_cmds().contains(&vec![0x80, 3, 1]));
    }

    #[test]
    fn safety_blockers() {
        let f = genuine_ds5();
        let mut bt = FirmwareSession::new(FirmwareModel::Ds5, false);
        let st = bt
            .execute(&f, &FirmwareOp::Status, full_battery())
            .unwrap()
            .status;
        assert!(st.blockers.contains(&Blocker::Bluetooth));
        assert!(bt
            .execute(&f, &FirmwareOp::CenterBegin, full_battery())
            .is_err());

        let low = Some(BatteryInfo {
            level: Some(15),
            state: BatteryState::Discharging,
        });
        let mut s = FirmwareSession::new(FirmwareModel::Ds5, true);
        let st = s.execute(&f, &FirmwareOp::Status, low).unwrap().status;
        assert!(st.blockers.contains(&Blocker::LowBattery));
        let st = s.execute(&f, &FirmwareOp::Status, None).unwrap().status;
        assert!(st.blockers.contains(&Blocker::BatteryUnknown));

        let mut edge = FirmwareSession::new(FirmwareModel::Ds5Edge, true);
        let st = edge
            .execute(&f, &FirmwareOp::Status, full_battery())
            .unwrap()
            .status;
        assert!(st.blockers.contains(&Blocker::EdgeNeedsHardwareMod));
    }

    #[test]
    fn outdated_and_clone_dualsense() {
        let f = Fake::default();
        let mut info = vec![0u8; 64];
        info[0] = 0x20;
        info[1..12].copy_from_slice(b"Jun 04 2021");
        f.always(0x20, info);
        f.always(0x81, vec![0x81, 0x03, 0x03, 0x02, 0x01]); // no fine-tune answer → clone
        let mut s = FirmwareSession::new(FirmwareModel::Ds5, true);
        let st = s
            .execute(&f, &FirmwareOp::Status, full_battery())
            .unwrap()
            .status;
        assert!(st.blockers.contains(&Blocker::OutdatedFirmware));
        assert!(st.blockers.contains(&Blocker::Clone));
        assert_eq!(st.clone_reason.as_deref(), Some("finetune"));
    }

    #[test]
    fn ds4_commands_and_clone_check() {
        let f = Fake::default();
        let mut a3 = vec![0u8; 49];
        a3[0] = 0xA3;
        a3[1..12].copy_from_slice(b"Sep 21 2018");
        f.always(0xA3, a3);
        f.always(0x81, vec![0x81, 0, 0]);
        f.always(0x11, vec![0x11, 1]);
        f.always(0x91, vec![0x91, 0x01, 0x01, 0x01]);
        f.always(0x92, vec![0x92, 0x01, 0x01, 0xFF]);
        let mut s = FirmwareSession::new(FirmwareModel::Ds4, true);
        let st = s
            .execute(&f, &FirmwareOp::CenterBegin, full_battery())
            .unwrap()
            .status;
        assert_eq!(st.phase, FirmwarePhase::Center);
        assert!(
            f.sent_cmds().contains(&vec![0x08, 0xFF, 0, 12]),
            "DS4 NV query"
        );
        assert!(f.sent_cmds().contains(&vec![0x90, 1, 1, 1]));

        let clone = Fake::default();
        clone.always(0xA3, vec![0xA3; 30]);
        let mut c = FirmwareSession::new(FirmwareModel::Ds4, true);
        let st = c
            .execute(&clone, &FirmwareOp::Status, full_battery())
            .unwrap()
            .status;
        assert_eq!(st.genuine, Some(false));
        assert!(c
            .execute(&clone, &FirmwareOp::CenterBegin, full_battery())
            .is_err());
    }

    fn switch_with_factory() -> Fake {
        let f = Fake::default();
        let left = SwitchStickCal::default().encode_left();
        let right = SwitchStickCal::default().encode_right();
        for (k, b) in left.iter().chain(right.iter()).enumerate() {
            f.flash
                .borrow_mut()
                .insert(switch_cal::FACTORY_STICKS_ADDR + k as u32, *b);
        }
        f
    }

    #[test]
    fn switch_write_is_backed_up_verified_and_restricted() {
        let f = switch_with_factory();
        let mut s = FirmwareSession::new(FirmwareModel::SwitchPro, false);
        let data = s
            .execute(&f, &FirmwareOp::SwitchRead, full_battery())
            .unwrap()
            .switch
            .unwrap();
        assert_eq!(data.factory_left, SwitchStickCal::default());
        assert_eq!(data.user_left, None, "erased block = no user calibration");

        let new = SwitchStickCal {
            center_x: 2010,
            center_y: 2070,
            x_max_above: 1400,
            x_min_below: 1420,
            y_max_above: 1410,
            y_min_below: 1390,
        };
        let op = FirmwareOp::SwitchWrite {
            left: new,
            right: new,
            acknowledge_risk: true,
        };
        let r = s.execute(&f, &op, full_battery()).unwrap();
        assert_eq!(r.backup, Some(vec![0xFF; 22]));
        assert_eq!(r.switch.unwrap().user_left, Some(new));
        // Only the user block was written.
        for w in f.writes.borrow().iter().filter(|w| w[10] == 0x11) {
            let addr = u32::from_le_bytes([w[11], w[12], w[13], w[14]]);
            assert!(switch_write_allowed(addr, w[15] as usize));
        }

        let cleared = s
            .execute(
                &f,
                &FirmwareOp::SwitchClear {
                    acknowledge_risk: true,
                },
                full_battery(),
            )
            .unwrap();
        assert_eq!(cleared.switch.unwrap().user_left, None);
    }

    #[test]
    fn switch_refuses_bad_values_and_detects_failed_writes() {
        let f = switch_with_factory();
        let mut s = FirmwareSession::new(FirmwareModel::SwitchPro, false);
        let bad = SwitchStickCal {
            y_max_above: 0x100,
            ..SwitchStickCal::default()
        };
        let op = FirmwareOp::SwitchWrite {
            left: bad,
            right: SwitchStickCal::default(),
            acknowledge_risk: true,
        };
        assert_eq!(
            s.execute(&f, &op, full_battery()).unwrap_err().code(),
            "firmwareBlocked"
        );

        let stuck = Fake {
            ignore_flash_writes: true,
            ..switch_with_factory()
        };
        let good = FirmwareOp::SwitchWrite {
            left: SwitchStickCal::default(),
            right: SwitchStickCal::default(),
            acknowledge_risk: true,
        };
        let err = s.execute(&stuck, &good, full_battery()).unwrap_err();
        assert!(err.to_string().contains("verification failed"), "{err}");
    }

    #[test]
    fn write_window_is_only_the_user_stick_block() {
        assert!(switch_write_allowed(0x8010, 22));
        assert!(!switch_write_allowed(0x8010, 23));
        assert!(
            !switch_write_allowed(0x603D, 9),
            "factory data is never written"
        );
        assert!(!switch_write_allowed(0x800F, 2));
        assert!(
            !switch_write_allowed(0x8026, 2),
            "IMU calibration untouched"
        );
    }

    #[test]
    fn op_json_shape() {
        let op: FirmwareOp =
            serde_json::from_str(r#"{"op":"savePermanent","acknowledgeRisk":true}"#).unwrap();
        assert_eq!(
            op,
            FirmwareOp::SavePermanent {
                acknowledge_risk: true
            }
        );
        let op: FirmwareOp = serde_json::from_str(r#"{"op":"centerSample"}"#).unwrap();
        assert_eq!(op, FirmwareOp::CenterSample);
        assert!(current_year() >= 2024);
    }
}
