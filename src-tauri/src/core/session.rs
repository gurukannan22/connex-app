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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_host_session() {
        let sess = Session::new_host("592104".to_string());
        assert_eq!(sess.device_id, "592104");
        assert_eq!(sess.role, Role::Host);
    }

    #[test]
    fn test_new_controller_session() {
        let sess = Session::new_controller("714839".to_string());
        assert_eq!(sess.device_id, "714839");
        assert_eq!(sess.role, Role::Controller);
    }

    #[test]
    fn test_session_serde_roundtrip() {
        let host = Session::new_host("111222".to_string());
        let json = serde_json::to_string(&host).expect("Serialization must succeed");
        assert!(json.contains(r#""role":"host""#));

        let deserialized: Session = serde_json::from_str(&json).expect("Deserialization must succeed");
        assert_eq!(deserialized.device_id, "111222");
        assert_eq!(deserialized.role, Role::Host);
    }

    #[test]
    fn test_controller_session_serde_roundtrip() {
        let ctrl = Session::new_controller("998877".to_string());
        let json = serde_json::to_string(&ctrl).expect("Serialization must succeed");
        assert!(json.contains(r#""role":"controller""#));

        let deserialized: Session = serde_json::from_str(&json).expect("Deserialization must succeed");
        assert_eq!(deserialized.device_id, "998877");
        assert_eq!(deserialized.role, Role::Controller);
    }

    #[test]
    fn test_session_clone_and_debug() {
        let host = Session::new_host("123456".to_string());
        let cloned = host.clone();
        assert_eq!(cloned.device_id, host.device_id);
        assert_eq!(cloned.role, host.role);

        let debug_str = format!("{:?}", host);
        assert!(debug_str.contains("123456"));
        assert!(debug_str.contains("Host"));
    }

    #[test]
    fn test_session_deserialize_invalid_role() {
        let invalid_json = r#"{"device_id":"123456","role":"unknown_role"}"#;
        let res = serde_json::from_str::<Session>(invalid_json);
        assert!(res.is_err(), "Invalid role must fail deserialization");
    }
}
