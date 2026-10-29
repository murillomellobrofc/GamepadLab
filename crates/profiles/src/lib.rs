//! Calibration profiles: one file per controller (VID:PID + serial), many
//! named profiles per file, quick switching, JSON import/export with
//! validation and schema migration.

mod migrate;
mod time;

pub use migrate::{migrate_profile_value, CURRENT_SCHEMA};
pub use time::now_rfc3339;

use gamepadlab_calibration::{CalibrationProfile, Invalid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use thiserror::Error;

pub const EXPORT_FORMAT: &str = "gamepadlab-profile";
pub const MAX_PROFILES: usize = 50;
pub const MAX_NAME: usize = 60;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("profile {0} not found")]
    NotFound(String),
    #[error("invalid profile: {0}")]
    Invalid(String),
    #[error("cannot delete the only profile")]
    LastProfile,
    #[error("too many profiles (max {MAX_PROFILES})")]
    TooMany,
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ProfileError {
    pub fn code(&self) -> &'static str {
        match self {
            ProfileError::NotFound(_) => "notFound",
            ProfileError::Invalid(_)
            | ProfileError::LastProfile
            | ProfileError::TooMany
            | ProfileError::Json(_) => "invalidInput",
            ProfileError::Io(_) => "io",
        }
    }
}

pub type Result<T> = std::result::Result<T, ProfileError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub calibration: CalibrationProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfiles {
    pub schema_version: u32,
    pub device_key: String,
    pub device_name: String,
    pub active_profile_id: Option<String>,
    pub profiles: Vec<Profile>,
}

impl DeviceProfiles {
    pub fn active(&self) -> Option<&Profile> {
        let id = self.active_profile_id.as_deref()?;
        self.profiles.iter().find(|p| p.id == id)
    }

    fn get_mut(&mut self, id: &str) -> Result<&mut Profile> {
        self.profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ProfileError::NotFound(id.into()))
    }

    fn unique_name(&self, base: &str) -> String {
        let base = clean_name(base);
        if !self.profiles.iter().any(|p| p.name == base) {
            return base;
        }
        (2..)
            .map(|n| format!("{base} ({n})"))
            .find(|c| !self.profiles.iter().any(|p| &p.name == c))
            .unwrap()
    }
}

/// Envelope written by "export" and accepted by "import".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEnvelope {
    pub format: String,
    pub schema_version: u32,
    pub exported_at: String,
    pub app_version: String,
    pub device_key: String,
    pub device_name: String,
    pub profile: Profile,
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("p{:x}{:04x}", nanos & 0xFFFF_FFFF_FFFF, n & 0xFFFF)
}

fn clean_name(name: &str) -> String {
    let n: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME)
        .collect();
    if n.is_empty() {
        "Perfil".into()
    } else {
        n
    }
}

fn describe(errors: &[Invalid]) -> String {
    errors
        .iter()
        .map(|e| format!("{}: {}", e.field, e.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn validate(cal: &CalibrationProfile) -> Result<()> {
    let errors = cal.validate();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ProfileError::Invalid(describe(&errors)))
    }
}

/// File-backed store: `<dir>/<sanitised device key>.json`.
pub struct ProfileStore {
    dir: PathBuf,
    lock: Mutex<()>,
}

impl ProfileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, device_key: &str) -> PathBuf {
        let safe: String = device_key
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.json"))
    }

    fn fresh(device_key: &str, device_name: &str) -> DeviceProfiles {
        let now = now_rfc3339();
        let p = Profile {
            id: new_id(),
            name: "Padrão".into(),
            created_at: now.clone(),
            updated_at: now,
            calibration: CalibrationProfile::default(),
        };
        DeviceProfiles {
            schema_version: CURRENT_SCHEMA,
            device_key: device_key.into(),
            device_name: device_name.into(),
            active_profile_id: Some(p.id.clone()),
            profiles: vec![p],
        }
    }

    /// Loads (and migrates) a device's profiles, creating a default one.
    pub fn load(&self, device_key: &str, device_name: &str) -> Result<DeviceProfiles> {
        let _g = self.lock.lock().unwrap();
        self.load_unlocked(device_key, device_name)
    }

    fn load_unlocked(&self, device_key: &str, device_name: &str) -> Result<DeviceProfiles> {
        let path = self.path_for(device_key);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::fresh(device_key, device_name))
            }
            Err(e) => return Err(e.into()),
        };
        let raw: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                // Keep the broken file for inspection, start over.
                log::warn!("profiles for {device_key} unreadable ({e}); backing up and resetting");
                let _ = std::fs::rename(&path, path.with_extension("broken.json"));
                return Ok(Self::fresh(device_key, device_name));
            }
        };
        let mut doc = migrate::migrate_device_value(raw)?;
        if doc.profiles.is_empty() {
            doc = Self::fresh(device_key, device_name);
        }
        if doc.active().is_none() {
            doc.active_profile_id = doc.profiles.first().map(|p| p.id.clone());
        }
        if !device_name.is_empty() {
            doc.device_name = device_name.into();
        }
        Ok(doc)
    }

    pub fn save(&self, doc: &DeviceProfiles) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path_for(&doc.device_key);
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(doc)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Runs `f` on the loaded document and saves it atomically.
    pub fn modify<T>(
        &self,
        device_key: &str,
        f: impl FnOnce(&mut DeviceProfiles) -> Result<T>,
    ) -> Result<(DeviceProfiles, T)> {
        let _g = self.lock.lock().unwrap();
        let mut doc = self.load_unlocked(device_key, "")?;
        let out = f(&mut doc)?;
        doc.schema_version = CURRENT_SCHEMA;
        self.save(&doc)?;
        Ok((doc, out))
    }

    pub fn create(
        &self,
        device_key: &str,
        name: &str,
        cal: Option<CalibrationProfile>,
    ) -> Result<Profile> {
        let cal = cal.unwrap_or_default();
        validate(&cal)?;
        self.modify(device_key, |doc| {
            if doc.profiles.len() >= MAX_PROFILES {
                return Err(ProfileError::TooMany);
            }
            let now = now_rfc3339();
            let p = Profile {
                id: new_id(),
                name: doc.unique_name(name),
                created_at: now.clone(),
                updated_at: now,
                calibration: cal,
            };
            doc.profiles.push(p.clone());
            doc.active_profile_id = Some(p.id.clone());
            Ok(p)
        })
        .map(|(_, p)| p)
    }

    pub fn update(&self, device_key: &str, profile: Profile) -> Result<Profile> {
        validate(&profile.calibration)?;
        self.modify(device_key, |doc| {
            let p = doc.get_mut(&profile.id)?;
            p.calibration = profile.calibration;
            p.name = clean_name(&profile.name);
            p.updated_at = now_rfc3339();
            Ok(p.clone())
        })
        .map(|(_, p)| p)
    }

    pub fn rename(&self, device_key: &str, id: &str, name: &str) -> Result<()> {
        self.modify(device_key, |doc| {
            let unique = doc.unique_name(name);
            let p = doc.get_mut(id)?;
            if p.name != clean_name(name) {
                p.name = unique;
            }
            p.updated_at = now_rfc3339();
            Ok(())
        })
        .map(|_| ())
    }

    pub fn duplicate(&self, device_key: &str, id: &str) -> Result<Profile> {
        self.modify(device_key, |doc| {
            if doc.profiles.len() >= MAX_PROFILES {
                return Err(ProfileError::TooMany);
            }
            let src = doc
                .profiles
                .iter()
                .find(|p| p.id == id)
                .cloned()
                .ok_or_else(|| ProfileError::NotFound(id.into()))?;
            let now = now_rfc3339();
            let copy = Profile {
                id: new_id(),
                name: doc.unique_name(&format!("{} (cópia)", src.name)),
                created_at: now.clone(),
                updated_at: now,
                calibration: src.calibration,
            };
            doc.profiles.push(copy.clone());
            Ok(copy)
        })
        .map(|(_, p)| p)
    }

    pub fn delete(&self, device_key: &str, id: &str) -> Result<()> {
        self.modify(device_key, |doc| {
            if doc.profiles.len() <= 1 {
                return Err(ProfileError::LastProfile);
            }
            let before = doc.profiles.len();
            doc.profiles.retain(|p| p.id != id);
            if doc.profiles.len() == before {
                return Err(ProfileError::NotFound(id.into()));
            }
            if doc.active_profile_id.as_deref() == Some(id) {
                doc.active_profile_id = doc.profiles.first().map(|p| p.id.clone());
            }
            Ok(())
        })
        .map(|_| ())
    }

    pub fn set_active(&self, device_key: &str, id: &str) -> Result<Profile> {
        self.modify(device_key, |doc| {
            let p = doc
                .profiles
                .iter()
                .find(|p| p.id == id)
                .cloned()
                .ok_or_else(|| ProfileError::NotFound(id.into()))?;
            doc.active_profile_id = Some(p.id.clone());
            Ok(p)
        })
        .map(|(_, p)| p)
    }

    /// Back to factory settings (no correction at all).
    pub fn reset(&self, device_key: &str, id: &str) -> Result<Profile> {
        self.modify(device_key, |doc| {
            let p = doc.get_mut(id)?;
            p.calibration = CalibrationProfile::default();
            p.updated_at = now_rfc3339();
            Ok(p.clone())
        })
        .map(|(_, p)| p)
    }

    pub fn export(&self, device_key: &str, id: &str, app_version: &str) -> Result<String> {
        let doc = self.load(device_key, "")?;
        let profile = doc
            .profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| ProfileError::NotFound(id.into()))?;
        let env = ExportEnvelope {
            format: EXPORT_FORMAT.into(),
            schema_version: CURRENT_SCHEMA,
            exported_at: now_rfc3339(),
            app_version: app_version.into(),
            device_key: doc.device_key.clone(),
            device_name: doc.device_name.clone(),
            profile,
        };
        Ok(serde_json::to_string_pretty(&env)?)
    }

    /// Validates, migrates and adds an exported profile under a new id.
    pub fn import(&self, device_key: &str, json: &str) -> Result<Profile> {
        if json.len() > 512 * 1024 {
            return Err(ProfileError::Invalid("file too large".into()));
        }
        let raw: Value = serde_json::from_str(json)?;
        let mut imported = migrate::migrate_import_value(raw)?;
        validate(&imported.calibration)?;
        self.modify(device_key, |doc| {
            if doc.profiles.len() >= MAX_PROFILES {
                return Err(ProfileError::TooMany);
            }
            let now = now_rfc3339();
            imported.id = new_id();
            imported.name = doc.unique_name(&imported.name);
            imported.updated_at = now.clone();
            if imported.created_at.is_empty() {
                imported.created_at = now;
            }
            doc.profiles.push(imported.clone());
            Ok(imported)
        })
        .map(|(_, p)| p)
    }
}
