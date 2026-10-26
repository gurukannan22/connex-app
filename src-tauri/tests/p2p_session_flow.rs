// ============================================================
// tests/p2p_session_flow.rs — Integration test for P2P signaling flow
// ============================================================

use connex_lib::core::session::{Role, Session};
use connex_lib::core::signaling::{
    apply_answer, create_answer, create_offer, decode_payload, encode_payload_for_device,
    encode_payload_with_passcode, verify_answer_passcode,
};
use connex_lib::core::webrtc_peer::{create_host_channels, create_peer_connection};

#[tokio::test]
async fn test_full_p2p_signaling_handshake() {
    // 1. Host creates Session & Peer Connection
    let host_session = Session::new_host("592104".to_string());
    assert_eq!(host_session.role, Role::Host);

    let host_peer = create_peer_connection().await.expect("Host peer creation");
    let (frames_ch, input_ch, _) = create_host_channels(&host_peer)
        .await
        .expect("Host channels creation");

    assert_eq!(frames_ch.label(), "connex-frames");
    assert_eq!(input_ch.label(), "connex-input");

    // Host generates Offer & Encodes to Base64 payload
    let offer = create_offer(&host_peer).await.expect("Host create offer");
    let host_payload = encode_payload_for_device(
        &offer,
        vec!["mock-candidate-1".to_string()],
        "592104".to_string(),
    )
    .expect("Encode host payload");

    assert!(!host_payload.is_empty());

    // 2. Controller receives and decodes Host payload
    let decoded_host_signal = decode_payload(&host_payload).expect("Decode host payload");
    assert_eq!(decoded_host_signal.sdp, offer.sdp);
    assert_eq!(decoded_host_signal.candidates.len(), 1);

    // Controller creates Session & Peer Connection
    let ctrl_session = Session::new_controller("714839".to_string());
    assert_eq!(ctrl_session.role, Role::Controller);

    let ctrl_peer = create_peer_connection()
        .await
        .expect("Controller peer creation");
    let answer = create_answer(&ctrl_peer, &decoded_host_signal.sdp)
        .await
        .expect("Controller create answer");

    let passcode = "849201";
    let answer_payload = encode_payload_with_passcode(
        &answer,
        vec!["mock-ctrl-candidate".to_string()],
        passcode.to_string(),
        decoded_host_signal.device_id.clone(),
    )
    .expect("Encode controller answer payload");

    // 3. Host receives Answer and verifies Passcode
    let decoded_answer = decode_payload(&answer_payload).expect("Decode controller answer");
    assert!(decoded_answer.passcode.is_empty());
    assert!(verify_answer_passcode(&decoded_answer, passcode, "592104").unwrap());
    assert_eq!(decoded_answer.candidates.len(), 1);

    // Host applies remote answer description
    let apply_res = apply_answer(&host_peer, &decoded_answer.sdp).await;
    assert!(apply_res.is_ok(), "Host apply answer must succeed");

    // Cleanup peers
    let _ = host_peer.close().await;
    let _ = ctrl_peer.close().await;
}

#[test]
fn test_passcode_mismatch_detection() {
    let host_passcode = "849201";
    let fake_answer_payload = r#"{"sdp":"mock-sdp","candidates":[],"passcode":"000000"}"#;

    use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
    use base64::Engine;
    let encoded = B64.encode(fake_answer_payload.as_bytes());

    let decoded = decode_payload(&encoded).expect("Decode should succeed");
    assert_ne!(
        decoded.passcode, host_passcode,
        "Mismatched passcodes must not equal"
    );
}
