// ============================================================
// ipc/commands.rs  — Tauri commands exposed to the frontend
// ============================================================

use crate::core::identity::DeviceIdentity;
use crate::core::input::{inject, InputEvent};
use crate::core::session::{Role, Session};
use crate::core::signaling::{
    apply_answer, create_answer, create_offer, decode_payload, encode_payload_for_device,
    encode_payload_with_passcode, verify_answer_passcode,
};
use crate::core::webrtc_peer::{create_host_channels, create_peer_connection};
use log::info;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;

const FRAME_CHUNK_BYTES: usize = 48 * 1024;
const MAX_FRAME_CHUNKS: usize = 128;
static NEXT_FRAME_ID: AtomicU64 = AtomicU64::new(1);

struct PendingFrame {
    chunks: Vec<Option<String>>,
    received: usize,
    created: Instant,
}

#[derive(Default)]
struct FrameAssembler {
    frames: HashMap<u64, PendingFrame>,
    newest_frame: u64,
}

impl FrameAssembler {
    fn push(&mut self, message: &str) -> Option<String> {
        let rest = message.strip_prefix("cx1:")?;
        let mut parts = rest.splitn(4, ':');
        let frame_id = parts.next()?.parse::<u64>().ok()?;
        let chunk_index = parts.next()?.parse::<usize>().ok()?;
        let chunk_count = parts.next()?.parse::<usize>().ok()?;
        let chunk = parts.next()?;

        if chunk_count == 0
            || chunk_count > MAX_FRAME_CHUNKS
            || chunk_index >= chunk_count
            || chunk.len() > FRAME_CHUNK_BYTES
        {
            return None;
        }

        self.newest_frame = self.newest_frame.max(frame_id);
        self.frames.retain(|id, frame| {
            frame.created.elapsed() < Duration::from_secs(2)
                && id.saturating_add(2) >= self.newest_frame
        });

        if !self.frames.contains_key(&frame_id) && self.frames.len() >= 3 {
            if let Some(oldest) = self.frames.keys().min().copied() {
                self.frames.remove(&oldest);
            }
        }

        let frame = self.frames.entry(frame_id).or_insert_with(|| PendingFrame {
            chunks: vec![None; chunk_count],
            received: 0,
            created: Instant::now(),
        });
        if frame.chunks.len() != chunk_count {
            self.frames.remove(&frame_id);
            return None;
        }

        if frame.chunks[chunk_index].is_none() {
            frame.chunks[chunk_index] = Some(chunk.to_string());
            frame.received += 1;
        }
        if frame.received != chunk_count {
            return None;
        }

        self.frames
            .remove(&frame_id)
            .map(|frame| frame.chunks.into_iter().flatten().collect())
    }
}

// ─────────────────────────────────────────────────────────────
// Shared state stored in Tauri's managed state
// ─────────────────────────────────────────────────────────────

pub struct ConnexState {
    pub session: Mutex<Option<Session>>,
    pub host_peer: Mutex<Option<Arc<RTCPeerConnection>>>,
    pub controller_peer: Mutex<Option<Arc<RTCPeerConnection>>>,
    pub frames_channel: Mutex<Option<Arc<RTCDataChannel>>>,
    pub host_input_channel: Mutex<Option<Arc<RTCDataChannel>>>,
    pub control_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    pub controller_input_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    pub capture_stop: Mutex<Option<Arc<AtomicBool>>>,
    pub display_index: Arc<AtomicUsize>,
    session_epoch: AtomicU64,
}

impl ConnexState {
    pub fn new() -> Self {
        ConnexState {
            session: Mutex::new(None),
            host_peer: Mutex::new(None),
            controller_peer: Mutex::new(None),
            frames_channel: Mutex::new(None),
            host_input_channel: Mutex::new(None),
            control_channel: Arc::new(Mutex::new(None)),
            controller_input_channel: Arc::new(Mutex::new(None)),
            capture_stop: Mutex::new(None),
            display_index: Arc::new(AtomicUsize::new(0)),
            session_epoch: AtomicU64::new(0),
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

fn emit_connection_state(peer: &Arc<RTCPeerConnection>, app: AppHandle) {
    peer.on_peer_connection_state_change(Box::new(
        move |connection_state: RTCPeerConnectionState| {
            let _ = app.emit(
                "connex://connection-state",
                format!("{connection_state:?}").to_lowercase(),
            );
            Box::pin(async {})
        },
    ));
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

/// Toggle exclusion of the Connex window from supported screen-capture APIs.
#[tauri::command]
pub async fn set_stealth_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "windows")]
        let result = window.hwnd().map_err(|e| e.to_string()).and_then(|hwnd| {
            crate::core::stealth::set_stealth_for_window(hwnd, enabled).map_err(|e| e.to_string())
        });

        #[cfg(not(target_os = "windows"))]
        let result = crate::core::stealth::set_stealth(enabled).map_err(|e| e.to_string());

        let _ = result_tx.send(result);
    })
    .map_err(|e| e.to_string())?;

    result_rx.await.map_err(|e| e.to_string())?
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
    let generation = state.session_epoch.fetch_add(1, Ordering::Relaxed) + 1;

    let identity = load_identity(&app)?;
    // A restarted share must not leave an old peer or capture loop running.
    if let Some(stop) = state.capture_stop.lock().await.take() {
        stop.store(true, Ordering::Relaxed);
    }
    if let Some(peer) = state.host_peer.lock().await.take() {
        let _ = peer.close().await;
    }
    if let Some(peer) = state.controller_peer.lock().await.take() {
        let _ = peer.close().await;
    }
    *state.frames_channel.lock().await = None;
    *state.host_input_channel.lock().await = None;
    *state.controller_input_channel.lock().await = None;
    *state.control_channel.lock().await = None;
    *state.session.lock().await = Some(Session::new_host(identity.device_id.clone()));

    // Create peer connection + data channels
    let peer = create_peer_connection().await.map_err(|e| e.to_string())?;
    {
        let mut current_peer = state.host_peer.lock().await;
        if state.session_epoch.load(Ordering::Relaxed) != generation {
            let _ = peer.close().await;
            return Err("Connection setup was cancelled. Start sharing again when ready.".into());
        }
        *current_peer = Some(Arc::clone(&peer));
    }
    emit_connection_state(&peer, app.clone());
    let (frames_ch, input_ch, control_ch) = create_host_channels(&peer)
        .await
        .map_err(|e| e.to_string())?;

    let chat_app = app.clone();
    control_ch.on_message(Box::new(move |msg: DataChannelMessage| {
        if let Ok(text) = std::str::from_utf8(&msg.data) {
            if text.len() <= 4096 {
                let _ = chat_app.emit("connex://chat", text.to_string());
            }
        }
        Box::pin(async {})
    }));

    // Host listens for remote input from the controller on input_ch
    {
        let selected_display = Arc::clone(&state.display_index);
        input_ch.on_message(Box::new(move |msg: DataChannelMessage| {
            if let Ok(text) = std::str::from_utf8(&msg.data) {
                if let Ok(event) = serde_json::from_str::<InputEvent>(text) {
                    if let Ok((w, h)) = crate::core::capture::display_size_for(
                        selected_display.load(Ordering::Relaxed),
                    ) {
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

    // Wait for ICE gathering to finish before creating a copy/paste payload.
    // A fixed delay can race slower networks and omit candidates entirely.
    let mut gathering_complete = peer.gathering_complete_promise().await;

    // Generate offer
    let offer = create_offer(&peer).await.map_err(|e| e.to_string())?;
    tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        gathering_complete.recv(),
    )
    .await
    .map_err(|_| {
        "Timed out while preparing the connection. Check your network and try again.".to_string()
    })?;

    if state.session_epoch.load(Ordering::Relaxed) != generation {
        let _ = peer.close().await;
        return Err("Connection setup was cancelled. Start sharing again when ready.".into());
    }

    let cands = candidates.lock().await.clone();
    let payload =
        encode_payload_for_device(&offer, cands, identity.device_id).map_err(|e| e.to_string())?;

    // Store peer and data channels in ConnexState
    *state.frames_channel.lock().await = Some(frames_ch);
    *state.host_input_channel.lock().await = Some(input_ch);
    *state.control_channel.lock().await = Some(control_ch);

    info!(
        "[cmd] host_create_session offer generated (len={})",
        payload.len()
    );
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

    let passcode = passcode.trim();
    if passcode.len() != 6 || !passcode.chars().all(|digit| digit.is_ascii_digit()) {
        return Err("Enter the host's 6-digit passcode.".into());
    }
    let requested_device_id = device_id.trim();
    if !requested_device_id.is_empty()
        && (requested_device_id.len() != 6
            || !requested_device_id
                .chars()
                .all(|digit| digit.is_ascii_digit()))
    {
        return Err("Enter all 6 digits of the device ID, or leave it blank.".into());
    }

    let signal = decode_payload(host_payload.trim()).map_err(|_| {
        "That host payload doesn't look valid. Copy the full payload from the host.".to_string()
    })?;

    if !requested_device_id.is_empty() && signal.device_id != requested_device_id {
        return Err(
            "That payload belongs to a different device ID. Check the ID and try again.".into(),
        );
    }

    let generation = state.session_epoch.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(old_peer) = state.controller_peer.lock().await.take() {
        let _ = old_peer.close().await;
    }
    *state.controller_input_channel.lock().await = None;
    *state.control_channel.lock().await = None;
    let remote_device_id = if requested_device_id.is_empty() {
        signal.device_id.clone()
    } else {
        requested_device_id.to_string()
    };
    *state.session.lock().await = Some(Session::new_controller(remote_device_id));

    let peer = create_peer_connection().await.map_err(|e| e.to_string())?;
    {
        let mut current_peer = state.controller_peer.lock().await;
        if state.session_epoch.load(Ordering::Relaxed) != generation {
            let _ = peer.close().await;
            return Err("Connection setup was cancelled. Try connecting again.".into());
        }
        *current_peer = Some(Arc::clone(&peer));
    }
    emit_connection_state(&peer, app.clone());

    // On controller side, listen for data channels initiated by host
    {
        let app_clone = app.clone();
        let ctrl_input_ch = Arc::clone(&state.controller_input_channel);
        let state_control_channel = Arc::clone(&state.control_channel);

        peer.on_data_channel(Box::new(move |channel: Arc<RTCDataChannel>| {
            let label = channel.label().to_string();
            info!("[controller] Data channel opened: {}", label);

            if label == crate::core::webrtc_peer::CHANNEL_FRAMES {
                let app = app_clone.clone();
                let assembler = Arc::new(std::sync::Mutex::new(FrameAssembler::default()));
                channel.on_message(Box::new(move |msg: DataChannelMessage| {
                    if let Ok(frame_b64) = std::str::from_utf8(&msg.data) {
                        let complete_frame = if frame_b64.starts_with("cx1:") {
                            assembler
                                .lock()
                                .ok()
                                .and_then(|mut state| state.push(frame_b64))
                        } else {
                            Some(frame_b64.to_string())
                        };
                        if let Some(frame_b64) = complete_frame {
                            let _ = app.emit(
                                "connex://frame",
                                FrameEvent {
                                    frame_b64,
                                    width: 0,
                                    height: 0,
                                },
                            );
                        }
                    }
                    Box::pin(async {})
                }));
            } else if label == crate::core::webrtc_peer::CHANNEL_INPUT {
                let slot = Arc::clone(&ctrl_input_ch);
                let ch = Arc::clone(&channel);
                tokio::spawn(async move {
                    *slot.lock().await = Some(ch);
                });
            } else if label == crate::core::webrtc_peer::CHANNEL_CONTROL {
                let slot = Arc::clone(&state_control_channel);
                let app = app_clone.clone();
                let ch = Arc::clone(&channel);
                tokio::spawn(async move {
                    *slot.lock().await = Some(ch.clone());
                    ch.on_message(Box::new(move |msg: DataChannelMessage| {
                        if let Ok(text) = std::str::from_utf8(&msg.data) {
                            if text.len() <= 4096 {
                                let _ = app.emit("connex://chat", text.to_string());
                            }
                        }
                        Box::pin(async {})
                    }));
                });
            }

            Box::pin(async {})
        }));
    }

    let mut gathering_complete = peer.gathering_complete_promise().await;
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

    tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        gathering_complete.recv(),
    )
    .await
    .map_err(|_| {
        "Timed out while preparing the reply. Check your network and try again.".to_string()
    })?;

    if state.session_epoch.load(Ordering::Relaxed) != generation {
        let _ = peer.close().await;
        return Err("Connection setup was cancelled. Try connecting again.".into());
    }

    let cands = candidates.lock().await.clone();
    let answer_payload = encode_payload_with_passcode(
        &answer,
        cands,
        passcode.to_string(),
        signal.device_id.clone(),
    )
    .map_err(|e| e.to_string())?;

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

    if !verify_answer_passcode(&signal, &identity.passcode, &identity.device_id).map_err(|_| {
        "That reply could not be authenticated. Ask the partner to reconnect.".to_string()
    })? {
        return Err("Reply authentication failed — check the passcode and reconnect.".into());
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
pub async fn start_capture(
    quality: Option<u8>,
    display_index: Option<usize>,
    state: State<'_, ConnexState>,
) -> Result<(), String> {
    info!("[cmd] start_capture");

    let stop = Arc::new(AtomicBool::new(false));
    let display_index = display_index.unwrap_or(0);
    crate::core::capture::display_size_for(display_index).map_err(|e| e.to_string())?;
    state.display_index.store(display_index, Ordering::Relaxed);
    let mut rx = crate::core::capture::start_capture(
        Arc::clone(&stop),
        quality
            .unwrap_or(crate::core::capture::DEFAULT_JPEG_QUALITY)
            .clamp(30, 95),
        display_index,
    )
    .map_err(|e| e.to_string())?;
    *state.capture_stop.lock().await = Some(stop);

    let frames_ch = state.frames_channel.lock().await.clone();

    tokio::spawn(async move {
        while let Some(frame_b64) = rx.recv().await {
            // Only the controller renders incoming frames. Emitting the host's
            // own capture here sent the host into the viewer before the reply
            // handshake could be completed.
            if let Some(ref ch) = frames_ch {
                let frame_id = NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed);
                let chunks: Vec<&str> = frame_b64
                    .as_bytes()
                    .chunks(FRAME_CHUNK_BYTES)
                    .filter_map(|chunk| std::str::from_utf8(chunk).ok())
                    .collect();
                let chunk_count = chunks.len();
                for (index, chunk) in chunks.into_iter().enumerate() {
                    let message = format!("cx1:{frame_id}:{index}:{chunk_count}:{chunk}");
                    if ch.send_text(message).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn list_displays() -> Result<Vec<crate::core::capture::DisplayInfo>, String> {
    crate::core::capture::displays().map_err(|e| e.to_string())
}

/// Stop screen capture and reset session
#[tauri::command]
pub async fn stop_capture(state: State<'_, ConnexState>) -> Result<(), String> {
    state.session_epoch.fetch_add(1, Ordering::Relaxed);
    if let Some(stop) = state.capture_stop.lock().await.take() {
        stop.store(true, Ordering::Relaxed);
    }
    if let Some(peer) = state.host_peer.lock().await.take() {
        let _ = peer.close().await;
    }
    if let Some(peer) = state.controller_peer.lock().await.take() {
        let _ = peer.close().await;
    }
    *state.frames_channel.lock().await = None;
    *state.host_input_channel.lock().await = None;
    *state.control_channel.lock().await = None;
    *state.controller_input_channel.lock().await = None;
    *state.session.lock().await = None;
    info!("[cmd] stop_capture & session reset");
    Ok(())
}

/// Send a small in-session chat message over the encrypted WebRTC data channel.
#[tauri::command]
pub async fn send_chat_message(
    message: String,
    state: State<'_, ConnexState>,
) -> Result<(), String> {
    let message = message.trim();
    if message.is_empty() || message.len() > 4096 {
        return Err("Messages must be between 1 and 4,096 bytes.".into());
    }
    let channel = state
        .control_channel
        .lock()
        .await
        .clone()
        .ok_or("Chat is available after the peer connection opens.")?;
    channel
        .send_text(message.to_string())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Inject an input event from controller (forward over WebRTC if connected, or execute locally if host)
#[tauri::command]
pub async fn inject_input(event_json: String, state: State<'_, ConnexState>) -> Result<(), String> {
    // If controller is connected via WebRTC input channel, send event to host and do NOT execute locally
    let ch = state.controller_input_channel.lock().await.clone();
    if let Some(ref input_ch) = ch {
        let _ = input_ch.send_text(event_json).await;
        return Ok(());
    }

    // Only execute on local machine if this instance is the host or standalone test
    let is_controller = {
        let s = state.session.lock().await;
        matches!(s.as_ref().map(|sess| &sess.role), Some(Role::Controller))
    };
    if is_controller {
        // Controller without active data channel yet; drop to avoid hijacking local input
        return Ok(());
    }

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

    #[test]
    fn test_connex_state_initialization() {
        let state = ConnexState::new();
        assert!(state.capture_stop.try_lock().unwrap().is_none());
    }

    #[test]
    fn test_frame_event_serialization() {
        let frame_event = FrameEvent {
            frame_b64: "dGVzdC1mcmFtZQ==".to_string(),
            width: 1920,
            height: 1080,
        };
        let json = serde_json::to_string(&frame_event).expect("Serialize must succeed");
        assert!(json.contains("frame_b64"));
        assert!(json.contains("1920"));
        assert!(json.contains("1080"));
    }

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

    #[tokio::test]
    async fn test_connex_state_concurrent_locks() {
        let state = Arc::new(ConnexState::new());
        let mut handles = Vec::new();

        for i in 0..5 {
            let state_clone = Arc::clone(&state);
            handles.push(tokio::spawn(async move {
                let mut sess = state_clone.session.lock().await;
                *sess = Some(Session::new_host(format!("10000{}", i)));
                let stop = state_clone.capture_stop.lock().await;
                assert!(stop.is_none());
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let sess = state.session.lock().await;
        assert!(sess.is_some());
    }

    #[tokio::test]
    async fn test_connex_state_reset_simulation() {
        let state = ConnexState::new();

        // 1. Populate
        *state.session.lock().await = Some(Session::new_controller("555444".to_string()));
        let stop = Arc::new(AtomicBool::new(false));
        *state.capture_stop.lock().await = Some(Arc::clone(&stop));

        // 2. Reset (simulating stop_capture command)
        state
            .capture_stop
            .lock()
            .await
            .take()
            .unwrap()
            .store(true, Ordering::Relaxed);
        *state.session.lock().await = None;

        assert!(stop.load(Ordering::Relaxed));
        assert!(state.session.lock().await.is_none());
    }
}
