// ============================================================
// core/identity.rs  — Stable per-device identity
// ============================================================
// Every machine gets a STATIC 6-digit numeric ID plus a passcode.
//
//  - device_id : derived deterministically from stable machine
//                properties (hostname / user / OS). Same value on
//                every launch, so a device is always "123456".
//  - passcode  : a 6-digit code the controller must enter to connect.
//                Generated once and persisted; the host can rotate it.
//
// Both are persisted to <app_config_dir>/identity.json so they
// survive restarts. The device_id is seeded deterministically, so
// even a fresh install on the same machine lands on the same number.
// ============================================================

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    /// Static 6-digit device number, e.g. "480213"
    pub device_id: String,
    /// 6-digit passcode a controller must enter to connect
    pub passcode: String,
}

impl DeviceIdentity {
    fn path(config_dir: &Path) -> PathBuf {
        config_dir.join("identity.json")
    }

    /// Load the persisted identity, or create + persist a new one.
    pub fn load_or_create(config_dir: &Path) -> Result<Self> {
        let path = Self::path(config_dir);

        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(identity) = serde_json::from_slice::<DeviceIdentity>(&bytes) {
                return Ok(identity);
            }
        }

        let identity = DeviceIdentity {
            device_id: derive_device_id(),
            passcode: random_passcode(),
        };
        identity.persist(config_dir)?;
        Ok(identity)
    }

    /// Rotate just the passcode, keeping the (static) device id.
    pub fn rotate_passcode(config_dir: &Path) -> Result<Self> {
        let mut identity = Self::load_or_create(config_dir)?;
        identity.passcode = random_passcode();
        identity.persist(config_dir)?;
        Ok(identity)
    }

    fn persist(&self, config_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(config_dir)?;
        let json = serde_json::to_vec_pretty(self)?;
        std::fs::write(Self::path(config_dir), json)?;
        Ok(())
    }
}

/// Deterministic 6-digit device id from stable machine properties.
fn derive_device_id() -> String {
    // Collect whatever stable identifiers the OS exposes via env.
    // Falls back gracefully when a value is missing.
    let seed = format!(
        "{}|{}|{}|{}",
        env_any(&["HOSTNAME", "COMPUTERNAME"]),
        env_any(&["USER", "USERNAME"]),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let h = hasher.finish();

    // Map into the 6-digit range [100000, 999999] — never a leading zero.
    let n = 100_000 + (h % 900_000);
    n.to_string()
}

/// Random 6-digit passcode in [100000, 999999].
fn random_passcode() -> String {
    use rand::Rng;
    let n: u32 = rand::thread_rng().gen_range(100_000..1_000_000);
    n.to_string()
}

/// First non-empty env var among the given names.
fn env_any(names: &[&str]) -> String {
    for name in names {
        if let Ok(val) = std::env::var(name) {
            if !val.is_empty() {
                return val;
            }
        }
    }
    String::from("connex-device")
}
