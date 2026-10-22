// ============================================================
// ipc/commands.rs  — Tauri commands exposed to the frontend
// ============================================================

use crate::core::input::{inject, InputEvent};
use crate::core::session::Session;
use crate::core::signaling::{create_answer, create_offer, decode_payload, encode_payload};
use crate::core::webrtc_peer::{create_host_channels, create_peer_connection};
use anyhow::Result;
use log::info;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

// ─────────────────────────────────────────────────────────────
// Shared state stored in Tauri's managed state
// ─────────────────────────────────────────────────────────────

pub struct ConnexState {
    pub session: Mutex<Option<Session>>,
    pub stop_capture: Arc<AtomicBool>,
}

impl ConnexState {
    pub fn new() -> Self {
        ConnexState {
            session: Mutex::new(None),
            stop_capture: Arc::new(AtomicBool::new(false)),
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────

/// HOST: Create a new session, return the offer payload for user to share
#[tauri::command]
pub async fn host_create_session(state: State<'_, ConnexState>) -> Result<String, String> {
    info!("[cmd] host_create_session");

    let session = Session::new_host();
    let code = session.code.clone();

    *state.session.lock().await = Some(session);

    // Create peer connection + data channels
    let peer = create_peer_connection().await.map_err(|e| e.to_string())?;
    let (_frames_ch, _input_ch, _control_ch) =
        create_host_channels(&peer).await.map_err(|e| e.to_string())?;

    // Collect ICE candidates (wait briefly for gathering)
    let candidates = Arc::new(Mutex::new(Vec::<String>::new()));
    let candidates_clone = Arc::clone(&candidates);

    peer.on_ice_candidate(Box::new(move |candidate| {
        let candidates_clone = Arc::clone(&candidates_clone);
        Box::pin(async move {
            if let Some(c) = candidate {
                if let Ok(json) = serde_json::to_string(&c.to_json().unwrap()) {
                    candidates_clone.lock().await.push(json);
                }
            }
        })
    }));

    // Generate offer
    let offer = create_offer(&peer).await.map_err(|e| e.to_string())?;

    // Wait for ICE gathering to settle
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let cands = candidates.lock().await.clone();
    let payload = encode_payload(&offer, cands).map_err(|e| e.to_string())?;

    // Return: code|payload (user shares the code, controller pastes the payload)
    Ok(format!("{}|{}", code, payload))
}

/// CONTROLLER: Enter the code+payload from the host, return answer payload
#[tauri::command]
pub async fn controller_join_session(
    host_payload: String,
    state: State<'_, ConnexState>,
) -> Result<String, String> {
    info!("[cmd] controller_join_session");

    // Parse "CODE|PAYLOAD"
    let parts: Vec<&str> = host_payload.splitn(2, '|').collect();
    if parts.len() != 2 {
        return Err("Invalid payload format. Expected CODE|PAYLOAD".into());
    }
    let code = parts[0].to_string();
    let encoded = parts[1];

    let signal = decode_payload(encoded).map_err(|e| e.to_string())?;
    let session = Session::new_controller(code);
    *state.session.lock().await = Some(session);

    let peer = create_peer_connection().await.map_err(|e| e.to_string())?;
    let answer = create_answer(&peer, &signal.sdp)
        .await
        .map_err(|e| e.to_string())?;

    // Collect ICE candidates
    let candidates = Arc::new(Mutex::new(Vec::<String>::new()));
    let candidates_clone = Arc::clone(&candidates);
    peer.on_ice_candidate(Box::new(move |candidate| {
        let candidates_clone = Arc::clone(&candidates_clone);
        Box::pin(async move {
            if let Some(c) = candidate {
                if let Ok(json) = serde_json::to_string(&c.to_json().unwrap()) {
                    candidates_clone.lock().await.push(json);
                }
            }
        })
    }));

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let cands = candidates.lock().await.clone();
    let answer_payload = encode_payload(&answer, cands).map_err(|e| e.to_string())?;

    Ok(answer_payload)
}

/// HOST: Apply the answer from the controller to complete WebRTC handshake
#[tauri::command]
pub async fn host_apply_answer(answer_payload: String) -> Result<(), String> {
    info!("[cmd] host_apply_answer");
    let signal = decode_payload(&answer_payload).map_err(|e| e.to_string())?;
    // In a full impl, we'd retrieve the stored peer from state
    // For now this is a placeholder until full state wiring
    info!("[cmd] Answer SDP received, len={}", signal.sdp.len());
    Ok(())
}

/// HOST: Start screen capture and stream frames as Tauri events (fallback for signaling phase)
#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, ConnexState>,
) -> Result<(), String> {
    info!("[cmd] start_capture");

    state.stop_capture.store(false, Ordering::Relaxed);
    let stop = Arc::clone(&state.stop_capture);

    let (width, height) = crate::core::capture::display_size().map_err(|e| e.to_string())?;

    let mut rx = crate::core::capture::start_capture(stop).map_err(|e| e.to_string())?;

    // Stream frames to frontend via Tauri event (used when peer isn't connected yet / testing)
    tokio::spawn(async move {
        while let Some(frame_b64) = rx.recv().await {
            if app
                .emit("connex://frame", FrameEvent { frame_b64, width, height })
                .is_err()
            {
                break;
            }
        }
    });

    Ok(())
}

/// HOST: Stop screen capture
#[tauri::command]
pub async fn stop_capture(state: State<'_, ConnexState>) -> Result<(), String> {
    state.stop_capture.store(true, Ordering::Relaxed);
    info!("[cmd] stop_capture");
    Ok(())
}

/// HOST: Inject an input event from the controller
#[tauri::command]
pub async fn inject_input(event_json: String) -> Result<(), String> {
    let event: InputEvent =
        serde_json::from_str(&event_json).map_err(|e| e.to_string())?;

    let (w, h) = crate::core::capture::display_size().map_err(|e| e.to_string())?;
    inject(event, w as u32, h as u32).map_err(|e| e.to_string())?;
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
