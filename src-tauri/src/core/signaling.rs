// ============================================================
// core/signaling.rs  — Lightweight in-band signaling
// ============================================================
// Since we have NO backend server, signaling (SDP offer/answer
// and ICE candidate exchange) happens out-of-band:
//
//  1. HOST generates SDP offer → encodes as compact base64 string
//  2. HOST displays it as the "session code payload" (QR or copyable text)
//  3. CONTROLLER pastes it → sends back SDP answer
//  4. ICE candidates are trickled via the same text channel
//
// In practice the UI shows:
//  - Step 1: "Share this code with the controller: XYZABC"
//  - Step 2: "Enter the payload from the controller"
//
// This avoids needing ANY server while still being user-friendly.
// ============================================================

use anyhow::Result;
use base64::engine::general_purpose::{
    STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD as B64,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

/// Wrapper for exchanging signaling data as compact base64
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPayload {
    pub sdp: String,             // SDP offer or answer
    pub candidates: Vec<String>, // serialized ICE candidates
    /// Passcode the controller entered (present on the controller's answer
    /// so the host can verify it before accepting). Empty on the host offer.
    #[serde(default)]
    pub passcode: String,
}

/// Serialize an SDP to a compact base64 string for display to user
pub fn encode_payload(sdp: &RTCSessionDescription, candidates: Vec<String>) -> Result<String> {
    encode_payload_with_passcode(sdp, candidates, String::new())
}

/// Serialize an SDP + passcode to a compact base64 string
pub fn encode_payload_with_passcode(
    sdp: &RTCSessionDescription,
    candidates: Vec<String>,
    passcode: String,
) -> Result<String> {
    let payload = SignalPayload {
        sdp: sdp.sdp.clone(),
        candidates,
        passcode,
    };
    let json = serde_json::to_string(&payload)?;
    Ok(B64.encode(json.as_bytes()))
}

/// Decode a base64 payload string back into SignalPayload, resilient to whitespace/line breaks
pub fn decode_payload(encoded: &str) -> Result<SignalPayload> {
    let clean: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
    let json_bytes = B64
        .decode(&clean)
        .or_else(|_| URL_SAFE.decode(&clean))
        .or_else(|_| STANDARD_NO_PAD.decode(&clean))
        .or_else(|_| STANDARD.decode(&clean))?;
    let payload: SignalPayload = serde_json::from_slice(&json_bytes)?;
    Ok(payload)
}

/// Generate SDP offer (HOST calls this)
pub async fn create_offer(peer: &Arc<RTCPeerConnection>) -> Result<RTCSessionDescription> {
    let offer = peer.create_offer(None).await?;
    peer.set_local_description(offer.clone()).await?;
    Ok(offer)
}

/// Accept SDP offer and generate answer (CONTROLLER calls this)
pub async fn create_answer(
    peer: &Arc<RTCPeerConnection>,
    offer_sdp: &str,
) -> Result<RTCSessionDescription> {
    let offer = RTCSessionDescription::offer(offer_sdp.to_string())?;
    peer.set_remote_description(offer).await?;
    let answer = peer.create_answer(None).await?;
    peer.set_local_description(answer.clone()).await?;
    Ok(answer)
}

/// Apply remote answer (HOST calls after receiving controller's answer)
pub async fn apply_answer(
    peer: &Arc<RTCPeerConnection>,
    answer_sdp: &str,
) -> Result<()> {
    let answer = RTCSessionDescription::answer(answer_sdp.to_string())?;
    peer.set_remote_description(answer).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_and_decode_roundtrip() {
        let payload = SignalPayload {
            sdp: "v=0\r\no=- 123456 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n".to_string(),
            candidates: vec!["candidate:1 1 UDP 2130706431 192.168.1.5 5000 typ host".to_string()],
            passcode: "849201".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let encoded = B64.encode(json.as_bytes());

        let decoded = decode_payload(&encoded).expect("Decode should succeed");
        assert_eq!(decoded.sdp, payload.sdp);
        assert_eq!(decoded.candidates, payload.candidates);
        assert_eq!(decoded.passcode, "849201");
    }

    #[test]
    fn test_decode_with_whitespace_and_newlines() {
        let payload = SignalPayload {
            sdp: "v=0\r\ntest\r\n".to_string(),
            candidates: vec![],
            passcode: "123456".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let encoded = B64.encode(json.as_bytes());

        // Simulate multi-line email/chat copy-paste with spaces and line breaks
        let corrupted_spacing = format!("  \n  {} \r\n  ", encoded);
        let decoded = decode_payload(&corrupted_spacing).expect("Should clean whitespace and decode");
        assert_eq!(decoded.passcode, "123456");
        assert_eq!(decoded.sdp, "v=0\r\ntest\r\n");
    }

    #[test]
    fn test_decode_corrupted_payload_returns_err() {
        assert!(decode_payload("not-a-valid-base64!").is_err());
        assert!(decode_payload("").is_err());
        // Valid base64 but invalid JSON
        let non_json_b64 = B64.encode(b"random binary data here");
        assert!(decode_payload(&non_json_b64).is_err());
    }

    #[test]
    fn test_passcode_defaults_to_empty() {
        // Old payload format without passcode field
        let json_no_passcode = r#"{"sdp":"test-sdp","candidates":[]}"#;
        let encoded = B64.encode(json_no_passcode.as_bytes());
        let decoded = decode_payload(&encoded).expect("Should deserialize with default empty passcode");
        assert_eq!(decoded.passcode, "");
        assert_eq!(decoded.sdp, "test-sdp");
    }

    #[test]
    fn test_all_base64_variants_compatibility() {
        let payload = SignalPayload {
            sdp: "v=0\r\no=alice 1000 2000 IN IP4 10.0.0.1\r\n".to_string(),
            candidates: vec!["candidate:foo.bar?query=1&param=2".to_string()],
            passcode: "654321".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();

        // 1. STANDARD with padding
        let std_b64 = STANDARD.encode(json.as_bytes());
        let dec1 = decode_payload(&std_b64).expect("Standard B64 must decode");
        assert_eq!(dec1.passcode, "654321");

        // 2. STANDARD_NO_PAD
        let std_no_pad_b64 = STANDARD_NO_PAD.encode(json.as_bytes());
        let dec2 = decode_payload(&std_no_pad_b64).expect("Standard no pad must decode");
        assert_eq!(dec2.passcode, "654321");

        // 3. URL_SAFE with padding
        let url_b64 = URL_SAFE.encode(json.as_bytes());
        let dec3 = decode_payload(&url_b64).expect("URL-safe B64 must decode");
        assert_eq!(dec3.passcode, "654321");

        // 4. URL_SAFE_NO_PAD
        let url_no_pad_b64 = B64.encode(json.as_bytes());
        let dec4 = decode_payload(&url_no_pad_b64).expect("URL-safe no pad must decode");
        assert_eq!(dec4.passcode, "654321");
    }

    #[test]
    fn test_multiple_ice_candidates() {
        let candidates = vec![
            "candidate:1 1 UDP 2130706431 192.168.1.1 5000 typ host".to_string(),
            "candidate:2 1 UDP 1694498815 10.0.0.2 5001 typ srflx raddr 192.168.1.1 rport 5000".to_string(),
            "candidate:3 1 TCP 1518280447 192.168.1.1 9 typ host tcptype active".to_string(),
        ];
        let payload = SignalPayload {
            sdp: "v=0\r\ns=-\r\n".to_string(),
            candidates: candidates.clone(),
            passcode: "112233".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let encoded = B64.encode(json.as_bytes());
        let decoded = decode_payload(&encoded).unwrap();
        assert_eq!(decoded.candidates.len(), 3);
        assert_eq!(decoded.candidates, candidates);
    }
}
