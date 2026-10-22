// ============================================================
// core/webrtc_peer.rs  — WebRTC peer connection management
// ============================================================
// Handles peer-to-peer connection using webrtc-rs.
// Uses Google's public STUN servers for NAT traversal.
// Screen frames flow over a DataChannel (Host → Controller).
// Input events flow back over a second DataChannel (Controller → Host).
// ============================================================

use anyhow::{Context, Result};
use log::info;
use std::sync::Arc;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;

/// Google's free public STUN servers — no signup required
const STUN_SERVERS: &[&str] = &[
    "stun:stun.l.google.com:19302",
    "stun:stun1.l.google.com:19302",
    "stun:stun2.l.google.com:19302",
];

/// Channel labels
pub const CHANNEL_FRAMES: &str = "connex-frames";   // Host → Controller: screen frames
pub const CHANNEL_INPUT: &str = "connex-input";     // Controller → Host: input events
pub const CHANNEL_CONTROL: &str = "connex-control"; // Both: session control messages

/// Build and return a configured RTCPeerConnection
pub async fn create_peer_connection() -> Result<Arc<RTCPeerConnection>> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();

    let ice_servers: Vec<RTCIceServer> = STUN_SERVERS
        .iter()
        .map(|url| RTCIceServer {
            urls: vec![url.to_string()],
            ..Default::default()
        })
        .collect();

    let config = RTCConfiguration {
        ice_servers,
        ..Default::default()
    };

    let peer = Arc::new(api.new_peer_connection(config).await?);

    // Log connection state changes
    {
        let peer_clone = Arc::clone(&peer);
        peer.on_peer_connection_state_change(Box::new(
            move |state: RTCPeerConnectionState| {
                info!("[webrtc] Connection state: {:?}", state);
                let _ = &peer_clone;
                Box::pin(async {})
            },
        ));
    }

    Ok(peer)
}

/// Create the three data channels on the HOST side (offerer)
pub async fn create_host_channels(
    peer: &Arc<RTCPeerConnection>,
) -> Result<(
    Arc<RTCDataChannel>, // frames (send)
    Arc<RTCDataChannel>, // input  (receive)
    Arc<RTCDataChannel>, // control
)> {
    use webrtc::data_channel::data_channel_init::RTCDataChannelInit;

    let frames_ch = peer
        .create_data_channel(
            CHANNEL_FRAMES,
            Some(RTCDataChannelInit {
                ordered: Some(false),       // drop old frames — lower latency
                max_retransmits: Some(0),   // no retransmit for real-time video
                ..Default::default()
            }),
        )
        .await
        .context("Failed to create frames channel")?;

    let input_ch = peer
        .create_data_channel(CHANNEL_INPUT, None)
        .await
        .context("Failed to create input channel")?;

    let control_ch = peer
        .create_data_channel(CHANNEL_CONTROL, None)
        .await
        .context("Failed to create control channel")?;

    Ok((frames_ch, input_ch, control_ch))
}

/// Register a message handler on a data channel
pub fn on_message<F>(channel: &Arc<RTCDataChannel>, handler: F)
where
    F: Fn(DataChannelMessage) + Send + Sync + 'static,
{
    channel.on_message(Box::new(move |msg| {
        handler(msg);
        Box::pin(async {})
    }));
}
