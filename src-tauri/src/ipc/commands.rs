// ============================================================
// ipc/commands.rs  — Tauri commands exposed to the frontend
// ============================================================

use crate::core::identity::DeviceIdentity;
use crate::core::input::{inject, InputEvent};
use crate::core::session::Session;
use crate::core::signaling::{
    apply_answer, create_answer, create_offer, decode_payload, encode_payload,
    encode_payload_with_passcode,
};
use crate::core::webrtc_peer::{create_host_channels, create_peer_connection};
use log::info;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::RTCPeerConnection;

// ─────────────────────────────────────────────────────────────
// Shared state stored in Tauri's managed state
// ─────────────────────────────────────────────────────────────

pub struct ConnexState {
    pub session: Mutex<Option<Session>>,
    pub host_peer: Mutex<Option<Arc<RTCPeerConnection>>>,
    pub controller_peer: Mutex<Option<Arc<RTCPeerConnection>>>,
    pub frames_channel: Mutex<Option<Arc<RTCDataChannel>>>,
    pub host_input_channel: Mutex<Option<Arc<RTCDataChannel>>>,
    pub controller_input_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    pub stop_capture: Arc<AtomicBool>,
}

impl ConnexState {
    pub fn new() -> Self {
        ConnexState {
            session: Mutex::new(None),
            host_peer: Mutex::new(None),
            controller_peer: Mutex::new(None),
            frames_channel: Mutex::new(None),
            host_input_channel: Mutex::new(None),
            controller_input_channel: Arc::new(Mutex::new(None)),
            stop_capture: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Resolve (and create if needed) the per-device identity.
fn load_identity(app: &AppHandle) -> Result<DeviceIdentity, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("No config dir: {e}"))?;
    DeviceIdentity::load_or_create(&dir).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────
// Identity commands
// ─────────────────────────────────────────────────────────────

/// Return this device's static 6-digit id + current passcode.
#[tauri::command]
pub async fn get_device_identity(app: AppHandle) -> Result<DeviceIdentity, String> {
    load_identity(&app)
}

/// Generate a fresh passcode (device id stays the same).
#[tauri::command]
pub async fn rotate_passcode(app: AppHandle) -> Result<DeviceIdentity, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("No config dir: {e}"))?;
    DeviceIdentity::rotate_passcode(&dir).map_err(|e| e.to_string())
}

/// Apply stealth mode to exclude window from screen capture (Zoom, Google Meet, OBS, Teams)
#[tauri::command]
pub async fn apply_stealth(app: AppHandle) -> Result<(), String> {
    app.run_on_main_thread(move || {
        let _ = crate::core::stealth::apply_stealth();
    })
    .map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────
// Connection commands
// ─────────────────────────────────────────────────────────────

/// HOST: Create a session offer. Returns the base64 payload to share.
#[tauri::command]
pub async fn host_create_session(
    app: AppHandle,
    state: State<'_, ConnexState>,
) -> Result<String, String> {
    info!("[cmd] host_create_session");

    // Apply stealth protection on the main thread so Connex window is never captured in Zoom/Google Meet
    let _ = app.run_on_main_thread(|| {
        let _ = crate::core::stealth::apply_stealth();
    });

    let identity = load_identity(&app)?;
    *state.session.lock().await = Some(Session::new_host(identity.device_id.clone()));

    // Create peer connection + data channels
    let peer = create_peer_connection().await.map_err(|e| e.to_string())?;
    let (frames_ch, input_ch, _control_ch) =
        create_host_channels(&peer).await.map_err(|e| e.to_string())?;

    // Host listens for remote input from the controller on input_ch
    {
        input_ch.on_message(Box::new(move |msg: DataChannelMessage| {
            if let Ok(text) = std::str::from_utf8(&msg.data) {
                if let Ok(event) = serde_json::from_str::<InputEvent>(text) {
                    if let Ok((w, h)) = crate::core::capture::display_size() {
                        let _ = inject(event, w as u32, h as u32);
                    }
                }
            }
            Box::pin(async {})
        }));
    }

    // Collect ICE candidates while gathering settles
    let candidates = Arc::new(Mutex::new(Vec::<String>::new()));
    let candidates_clone = Arc::clone(&candidates);
    peer.on_ice_candidate(Box::new(move |candidate| {
        let candidates_clone = Arc::clone(&candidates_clone);
        Box::pin(async move {
            if let Some(c) = candidate {
                if let Ok(init) = c.to_json() {
                    if let Ok(json) = serde_json::to_string(&init) {
                        candidates_clone.lock().await.push(json);
                    }
                }
            }
        })
    }));

    // Generate offer
    let offer = create_offer(&peer).await.map_err(|e| e.to_string())?;

    // Wait briefly for initial local/host candidates to be gathered
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let cands = candidates.lock().await.clone();
    let payload = encode_payload(&offer, cands).map_err(|e| e.to_string())?;

    // Store peer and data channels in ConnexState
    *state.host_peer.lock().await = Some(peer);
    *state.frames_channel.lock().await = Some(frames_ch);
    *state.host_input_channel.lock().await = Some(input_ch);

    info!("[cmd] host_create_session offer generated (len={})", payload.len());
    Ok(payload)
}

/// CONTROLLER: Join using the host's payload + the passcode the user typed.
/// Returns the answer payload to hand back to the host.
#[tauri::command]
pub async fn controller_join_session(
    app: AppHandle,
    host_payload: String,
    passcode: String,
    device_id: String,
    state: State<'_, ConnexState>,
) -> Result<String, String> {
    info!("[cmd] controller_join_session");

    if passcode.trim().is_empty() {
        return Err("Enter the host's passcode.".into());
    }

    let signal = decode_payload(host_payload.trim()).map_err(|_| {
        "That host payload doesn't look valid. Copy the full payload from the host.".to_string()
    })?;

    *state.session.lock().await = Some(Session::new_controller(device_id));

    let peer = create_peer_connection().await.map_err(|e| e.to_string())?;

    // On controller side, listen for data channels initiated by host
    {
        let app_clone = app.clone();
        let ctrl_input_ch = Arc::clone(&state.controller_input_channel);

        peer.on_data_channel(Box::new(move |channel: Arc<RTCDataChannel>| {
            let label = channel.label().to_string();
            info!("[controller] Data channel opened: {}", label);

            if label == crate::core::webrtc_peer::CHANNEL_FRAMES {
                let app = app_clone.clone();
                channel.on_message(Box::new(move |msg: DataChannelMessage| {
                    if let Ok(frame_b64) = std::str::from_utf8(&msg.data) {
                        let _ = app.emit(
                            "connex://frame",
                            FrameEvent {
                                frame_b64: frame_b64.to_string(),
                                width: 0,
                                height: 0,
                            },
                        );
                    }
                    Box::pin(async {})
                }));
            } else if label == crate::core::webrtc_peer::CHANNEL_INPUT {
                let slot = Arc::clone(&ctrl_input_ch);
                let ch = Arc::clone(&channel);
                tokio::spawn(async move {
                    *slot.lock().await = Some(ch);
                });
            }

            Box::pin(async {})
        }));
    }

    let answer = create_answer(&peer, &signal.sdp)
        .await
        .map_err(|e| e.to_string())?;

    // Add host's ICE candidates to peer connection
    for c_str in &signal.candidates {
        if let Ok(init) = serde_json::from_str::<RTCIceCandidateInit>(c_str) {
            let _ = peer.add_ice_candidate(init).await;
        }
    }

    let candidates = Arc::new(Mutex::new(Vec::<String>::new()));
    let candidates_clone = Arc::clone(&candidates);
    peer.on_ice_candidate(Box::new(move |candidate| {
        let candidates_clone = Arc::clone(&candidates_clone);
        Box::pin(async move {
            if let Some(c) = candidate {
                if let Ok(init) = c.to_json() {
                    if let Ok(json) = serde_json::to_string(&init) {
                        candidates_clone.lock().await.push(json);
                    }
                }
            }
        })
    }));

    // Wait briefly for controller's candidates
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let cands = candidates.lock().await.clone();
    let answer_payload =
        encode_payload_with_passcode(&answer, cands, passcode.trim().to_string())
            .map_err(|e| e.to_string())?;

    *state.controller_peer.lock().await = Some(peer);

    info!("[cmd] controller_join_session answer generated");
    Ok(answer_payload)
}

/// HOST: Apply the controller's answer, verifying the passcode first.
#[tauri::command]
pub async fn host_apply_answer(
    app: AppHandle,
    answer_payload: String,
    state: State<'_, ConnexState>,
) -> Result<(), String> {
    info!("[cmd] host_apply_answer");

    let identity = load_identity(&app)?;
    let signal = decode_payload(answer_payload.trim())
        .map_err(|_| "That answer doesn't look valid. Paste the full answer.".to_string())?;

    if signal.passcode.trim() != identity.passcode {
        return Err("Wrong passcode — connection rejected.".into());
    }

    let guard = state.host_peer.lock().await;
    let peer = guard
        .as_ref()
        .ok_or("No active host session. Create a session first.")?;

    apply_answer(peer, &signal.sdp)
        .await
        .map_err(|e| e.to_string())?;

    // Add controller's ICE candidates to peer connection
    for c_str in &signal.candidates {
        if let Ok(init) = serde_json::from_str::<RTCIceCandidateInit>(c_str) {
            let _ = peer.add_ice_candidate(init).await;
        }
    }

    info!("[cmd] Answer accepted, passcode verified, ICE candidates added");
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Capture / input
// ─────────────────────────────────────────────────────────────

/// HOST: Start screen capture and stream frames as Tauri events and WebRTC data channel.
#[tauri::command]
pub async fn start_capture(app: AppHandle, state: State<'_, ConnexState>) -> Result<(), String> {
    info!("[cmd] start_capture");

    state.stop_capture.store(false, Ordering::Relaxed);
    let stop = Arc::clone(&state.stop_capture);

    let (width, height) = crate::core::capture::display_size().map_err(|e| e.to_string())?;
    let mut rx = crate::core::capture::start_capture(stop).map_err(|e| e.to_string())?;

    let frames_ch = state.frames_channel.lock().await.clone();

    tokio::spawn(async move {
        while let Some(frame_b64) = rx.recv().await {
            // Send to local frontend preview / controller
            let _ = app.emit(
                "connex://frame",
                FrameEvent {
                    frame_b64: frame_b64.clone(),
                    width,
                    height,
                },
            );

            // Send over WebRTC DataChannel to remote controller
            if let Some(ref ch) = frames_ch {
                let _ = ch.send_text(frame_b64).await;
            }
        }
    });

    Ok(())
}

/// Stop screen capture and reset session
#[tauri::command]
pub async fn stop_capture(state: State<'_, ConnexState>) -> Result<(), String> {
    state.stop_capture.store(true, Ordering::Relaxed);
    if let Some(peer) = state.host_peer.lock().await.take() {
        let _ = peer.close().await;
    }
    if let Some(peer) = state.controller_peer.lock().await.take() {
        let _ = peer.close().await;
    }
    *state.frames_channel.lock().await = None;
    *state.host_input_channel.lock().await = None;
    *state.controller_input_channel.lock().await = None;
    *state.session.lock().await = None;
    info!("[cmd] stop_capture & session reset");
    Ok(())
}

/// Inject an input event from controller (forward over WebRTC if connected, or execute locally)
#[tauri::command]
pub async fn inject_input(
    event_json: String,
    state: State<'_, ConnexState>,
) -> Result<(), String> {
    // If controller is connected via WebRTC input channel, send event to host
    let ch = state.controller_input_channel.lock().await.clone();
    if let Some(ref input_ch) = ch {
        let _ = input_ch.send_text(event_json.clone()).await;
    }

    // Also execute on local host (for host testing or direct execution)
    let event: InputEvent = serde_json::from_str(&event_json).map_err(|e| e.to_string())?;
    let (w, h) = crate::core::capture::display_size().map_err(|e| e.to_string())?;
    let _ = inject(event, w as u32, h as u32);
    Ok(())
}

/// Get current session info
#[tauri::command]
pub async fn get_session(state: State<'_, ConnexState>) -> Result<Option<Session>, String> {
    Ok(state.session.lock().await.clone())
}

// ─────────────────────────────────────────────────────────────
// Event types emitted to frontend
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FrameEvent {
    pub frame_b64: String,
    pub width: usize,
    pub height: usize,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        println!("Starting create_peer_connection...");
        let peer = create_peer_connection().await.unwrap();
        println!("Peer created. Creating channels...");
        let (_f, _i, _c) = create_host_channels(&peer).await.unwrap();
        println!("Channels created. Creating offer...");
        let offer = create_offer(&peer).await.unwrap();
        println!("Offer created! len = {}", offer.sdp.len());
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("Done!");
    }
}
