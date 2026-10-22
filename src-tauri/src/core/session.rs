// ============================================================
// core/session.rs  — Session & role management
// ============================================================

use serde::{Deserialize, Serialize};

/// The role this instance plays in a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Shares this machine's screen and accepts an incoming connection.
    Host,
    /// Connects to a host to view and control it.
    Controller,
}

/// State of a Connex session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The static 6-digit device id involved in this session
    pub device_id: String,
    pub role: Role,
}

impl Session {
    /// Create a new HOST session bound to this device's static id.
    pub fn new_host(device_id: String) -> Self {
        Session {
            device_id,
            role: Role::Host,
        }
    }

    /// Create a CONTROLLER session targeting the host's device id.
    pub fn new_controller(device_id: String) -> Self {
        Session {
            device_id,
            role: Role::Controller,
        }
    }
}
