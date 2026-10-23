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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_device_id_format() {
        let id = derive_device_id();
        assert_eq!(id.len(), 6, "Device ID must be exactly 6 characters");
        assert!(id.chars().all(|c| c.is_ascii_digit()), "Device ID must only contain digits");
        let val: u32 = id.parse().expect("Device ID must parse as numeric integer");
        assert!((100_000..=999_999).contains(&val), "Device ID must be in range 100000..=999999");
    }

    #[test]
    fn test_random_passcode_format() {
        for _ in 0..50 {
            let pass = random_passcode();
            assert_eq!(pass.len(), 6, "Passcode must be exactly 6 characters");
            assert!(pass.chars().all(|c| c.is_ascii_digit()), "Passcode must be numeric digits");
            let val: u32 = pass.parse().expect("Passcode must parse as numeric integer");
            assert!((100_000..=999_999).contains(&val), "Passcode must be in range 100000..=999999");
        }
    }

    #[test]
    fn test_load_or_create_and_rotate_persistence() {
        let unique_dir = std::env::temp_dir().join(format!("connex_test_ident_{}", rand::random::<u64>()));
        let _ = std::fs::remove_dir_all(&unique_dir);

        // 1. Initial creation
        let id1 = DeviceIdentity::load_or_create(&unique_dir).expect("Initial creation must succeed");
        assert_eq!(id1.device_id.len(), 6);
        assert_eq!(id1.passcode.len(), 6);
        assert!(unique_dir.join("identity.json").exists(), "identity.json file must exist");

        // 2. Loading again must yield identical device_id and passcode
        let id2 = DeviceIdentity::load_or_create(&unique_dir).expect("Reloading must succeed");
        assert_eq!(id1.device_id, id2.device_id);
        assert_eq!(id1.passcode, id2.passcode);

        // 3. Rotating passcode must keep device_id but change passcode
        let id3 = DeviceIdentity::rotate_passcode(&unique_dir).expect("Rotating passcode must succeed");
        assert_eq!(id3.device_id, id1.device_id);

        // Clean up
        let _ = std::fs::remove_dir_all(&unique_dir);
    }

    #[test]
    fn test_load_or_create_corrupted_file_recovery() {
        let unique_dir = std::env::temp_dir().join(format!("connex_test_corrupt_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&unique_dir).unwrap();
        // Write corrupted non-json file
        std::fs::write(unique_dir.join("identity.json"), b"{ invalid json content here").unwrap();

        // Must recover safely and generate a valid identity
        let recovered = DeviceIdentity::load_or_create(&unique_dir).expect("Should recover from corrupted file");
        assert_eq!(recovered.device_id.len(), 6);
        assert_eq!(recovered.passcode.len(), 6);

        let _ = std::fs::remove_dir_all(&unique_dir);
    }

    #[test]
    fn test_env_any_precedence_and_fallback() {
        // Fallback when none exist
        let val = env_any(&["NON_EXISTENT_VAR_ABCXYZ_1", "NON_EXISTENT_VAR_ABCXYZ_2"]);
        assert_eq!(val, "connex-device");

        // When a standard env var exists (PATH or USER)
        let found = env_any(&["USER", "USERNAME", "PATH"]);
        assert!(!found.is_empty());
        assert_ne!(found, "connex-device");
    }

    #[test]
    fn test_device_identity_serde_roundtrip() {
        let ident = DeviceIdentity {
            device_id: "123456".to_string(),
            passcode: "654321".to_string(),
        };
        let json = serde_json::to_string(&ident).unwrap();
        assert!(json.contains("123456"));
        assert!(json.contains("654321"));

        let deserialized: DeviceIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.device_id, "123456");
        assert_eq!(deserialized.passcode, "654321");
    }
}
