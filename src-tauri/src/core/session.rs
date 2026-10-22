// ============================================================
// core/session.rs  — Session code generation & role management
// ============================================================

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// The role this instance plays in a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Generates the code → shares their screen (invisible agent)
    Host,
    /// Enters the code → sees and controls the remote screen (visible UI)
    Controller,
}

/// State of a Connex session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub code: String,
    pub role: Role,
}

impl Session {
    /// Create a new HOST session: generate a 6-digit code
    pub fn new_host() -> Self {
        let code = generate_code();
        Session {
            id: Uuid::new_v4().to_string(),
            code,
            role: Role::Host,
        }
    }

    /// Create a CONTROLLER session: enter the code given by host
    pub fn new_controller(code: String) -> Self {
        Session {
            id: Uuid::new_v4().to_string(),
            code,
            role: Role::Controller,
        }
    }
}

/// Generate a random 6-digit alphanumeric code (e.g. "A3X9K2")
fn generate_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Global shared session state
pub type SharedSession = Arc<Mutex<Option<Session>>>;

pub fn new_shared_session() -> SharedSession {
    Arc::new(Mutex::new(None))
}
