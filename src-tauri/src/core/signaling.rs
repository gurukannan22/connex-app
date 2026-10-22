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
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
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

/// Decode a base64 payload string back into SignalPayload
pub fn decode_payload(encoded: &str) -> Result<SignalPayload> {
    let json_bytes = B64.decode(encoded.trim())?;
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
