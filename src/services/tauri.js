// ============================================================
// services/tauri.js — Typed Tauri IPC client
// Decouples backend command invocations from UI view components
// ============================================================

import { invoke } from "@tauri-apps/api/core";

/**
 * Fetch this device's static 6-digit ID and passcode
 * @returns {Promise<{ device_id: string, passcode: string }>}
 */
export async function getDeviceIdentity() {
  return await invoke("get_device_identity");
}

/**
 * Rotate current passcode, keeping static device ID
 * @returns {Promise<{ device_id: string, passcode: string }>}
 */
export async function rotatePasscode() {
  return await invoke("rotate_passcode");
}

/**
 * Host initiates a WebRTC sharing session
 * @returns {Promise<string>} Base64 SDP offer payload
 */
export async function hostCreateSession() {
  return await invoke("host_create_session");
}

/**
 * Host applies the controller's answer payload
 * @param {string} answerPayload
 * @returns {Promise<void>}
 */
export async function hostApplyAnswer(answerPayload) {
  return await invoke("host_apply_answer", { answerPayload });
}

/**
 * Controller joins host using host payload and passcode
 * @param {string} hostPayload
 * @param {string} passcode
 * @param {string} [deviceId=""]
 * @returns {Promise<string>} Base64 SDP answer payload
 */
export async function controllerJoinSession(hostPayload, passcode, deviceId = "") {
  return await invoke("controller_join_session", { hostPayload, passcode, deviceId });
}

/**
 * Start screen capture streaming
 * @returns {Promise<void>}
 */
export async function startCapture() {
  return await invoke("start_capture");
}

/**
 * Stop screen capture and terminate active session
 * @returns {Promise<void>}
 */
export async function stopCapture() {
  return await invoke("stop_capture");
}

/**
 * Inject input event (forwarded over WebRTC or executed locally)
 * @param {object} event
 * @returns {Promise<void>}
 */
export async function injectInput(event) {
  return await invoke("inject_input", { eventJson: JSON.stringify(event) });
}
