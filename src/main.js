// ============================================================
// main.js — Connex application bootstrap & controller
// Clean orchestrator following SOLID principles
// ============================================================

import { listen } from "@tauri-apps/api/event";
import { icon } from "./components/icons.js";
import { showToast } from "./components/toast.js";
import { renderFrame, setupCanvasInteractions } from "./components/canvas.js";
import { getRecent, saveRecent } from "./services/storage.js";
import {
  getDeviceIdentity,
  rotatePasscode,
  setStealthMode,
  hostCreateSession,
  hostApplyAnswer,
  controllerJoinSession,
  startCapture,
  stopCapture,
  sendChatMessage,
  listDisplays,
} from "./services/tauri.js";
import { renderHomeScreen } from "./screens/homeScreen.js";
import { renderRemoteScreen } from "./screens/remoteScreen.js";
import "./styles/main.css";

// ─────────────────────────────────────────────────────────────
// App State
// ─────────────────────────────────────────────────────────────
const state = {
  identity: null,
  connected: false,
  sharing: false,
  connectionTimer: null,
  stealthEnabled: false,
};
const incomingFiles = new Map();

const $ = (id) => document.getElementById(id);

// ─────────────────────────────────────────────────────────────
// Bootstrap & Shell
// ─────────────────────────────────────────────────────────────
function buildAppHTML() {
  return `
  <div class="app-shell">
    <header class="topbar">
      <div class="brand">
        <img src="/connex-logo.png" class="brand-app-icon" alt="Connex" />
        <span class="brand-name">Connex</span>
      </div>
      <div class="topbar-tools">
        <button class="btn btn-ghost btn-sm btn-stealth" id="btn-stealth" type="button" aria-pressed="false"
          title="Exclude only the Connex window from supported screen captures. Connex stays visible and controllable locally.">
          ${icon("lock", "icon icon-sm")}<span id="stealth-label">Stealth off</span>
        </button>
        <span class="top-status" id="global-status" aria-live="polite"><span class="dot"></span>Ready</span>
      </div>
    </header>

    ${renderHomeScreen()}
    ${renderRemoteScreen()}

    <div id="toast-container"></div>
  </div>`;
}

$("app").innerHTML = buildAppHTML();

const screens = {
  home: $("screen-home"),
  remote: $("screen-remote"),
};

function showScreen(name) {
  Object.values(screens).forEach((s) => s.classList.remove("active"));
  screens[name]?.classList.add("active");
}

function setGlobalStatus(label, kind = "") {
  const el = $("global-status");
  if (!el) return;
  el.className = `top-status ${kind}`;
  el.innerHTML = `<span class="dot"></span>${label}`;
}

const stealthButton = $("btn-stealth");
const stealthSupported = /Windows|Macintosh|Mac OS X/i.test(navigator.userAgent)
  || /Win|Mac/i.test(navigator.platform || "");
if (stealthButton && !stealthSupported) {
  stealthButton.disabled = true;
  stealthButton.title = "Screen-capture exclusion is available on Windows and macOS.";
}

$("btn-stealth")?.addEventListener("click", async (event) => {
  const button = event.currentTarget;
  const next = !state.stealthEnabled;
  button.disabled = true;
  try {
    await setStealthMode(next);
    state.stealthEnabled = next;
    button.setAttribute("aria-pressed", String(next));
    button.classList.toggle("is-enabled", next);
    $("stealth-label").textContent = next ? "Stealth on" : "Stealth off";
    showToast(
      next
        ? "Stealth is on. Connex remains visible locally; supported screen captures may omit its window."
        : "Stealth is off. Connex can appear in screen captures.",
      "info",
      5000,
    );
  } catch (err) {
    showToast(`Couldn't change stealth mode: ${err}`, "error");
  } finally {
    button.disabled = false;
  }
});

// ─────────────────────────────────────────────────────────────
// Identity Management
// ─────────────────────────────────────────────────────────────
async function loadIdentity() {
  setIdentityLoading(true);
  try {
    state.identity = await getDeviceIdentity();
    renderIdentity();
  } catch (err) {
    setIdentityLoading(false);
    showToast(`Couldn't load device identity: ${err}`, "error");
  }
}

function setIdentityLoading(loading) {
  document.querySelectorAll("[data-device-id],[data-passcode]").forEach((el) => {
    el.classList.toggle("loading", loading);
  });
}

function renderIdentity() {
  if (!state.identity) return;
  setIdentityLoading(false);
  document.querySelectorAll("[data-device-id]").forEach((el) => {
    el.textContent = state.identity.device_id;
  });
  document.querySelectorAll("[data-passcode]").forEach((el) => {
    el.textContent = state.identity.passcode;
  });
}

// ─────────────────────────────────────────────────────────────
// UI Helpers
// ─────────────────────────────────────────────────────────────
function flashCopied(btn, labelHTML) {
  const original = labelHTML ?? btn.innerHTML;
  btn.innerHTML = `${icon("check", "icon icon-sm")} Copied`;
  btn.disabled = true;
  setTimeout(() => {
    btn.innerHTML = original;
    btn.disabled = false;
  }, 1100);
}

async function copyText(text) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through to the WebView-compatible copy method.
  }

  const field = document.createElement("textarea");
  field.value = text;
  field.setAttribute("readonly", "");
  field.style.position = "fixed";
  field.style.opacity = "0";
  document.body.appendChild(field);
  field.select();
  let copied = false;
  try {
    copied = document.execCommand("copy");
  } catch {
    copied = false;
  }
  field.remove();
  return copied;
}

async function copyWithFeedback(text, btn) {
  if (await copyText(text)) {
    if (btn) flashCopied(btn);
    else showToast("Copied to clipboard.", "success", 1400);
    return;
  }
  showToast("Couldn't copy automatically. Select and copy the text manually.", "error");
}

function digitsOnly(el, max = 6) {
  if (!el) return;
  el.addEventListener("input", () => {
    el.value = el.value.replace(/\D/g, "").slice(0, max);
    validateConnectForm();
  });
}

function setShareStep(n, cls) {
  const el = $(`share-step-${n}`);
  if (el) el.className = `stp ${cls}`;
}

// ─────────────────────────────────────────────────────────────
// Host Sharing Flow
// ─────────────────────────────────────────────────────────────
$("btn-copy-identity")?.addEventListener("click", (e) => {
  if (!state.identity) {
    showToast("Device identity not loaded yet.", "warning");
    return;
  }
  copyWithFeedback(`Connex ID ${state.identity.device_id} · passcode ${state.identity.passcode}`, e.currentTarget);
});

$("btn-rotate-passcode")?.addEventListener("click", async () => {
  try {
    state.identity = await rotatePasscode();
    renderIdentity();
    showToast("New passcode generated.", "success");
  } catch (err) {
    showToast(`Couldn't update passcode: ${err}`, "error");
  }
});

// Click any ID / passcode badge to copy
document.addEventListener("click", (e) => {
  const el = e.target.closest("[data-device-id],[data-passcode]");
  if (!el || !state.identity || el.classList.contains("loading")) return;
  copyWithFeedback(el.textContent.trim());
});

$("btn-share-start")?.addEventListener("click", async () => {
  const btn = $("btn-share-start");
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Preparing…`;
  setShareStep(1, "active");
  try {
    const payload = await hostCreateSession();
    $("host-payload-box").value = payload;
    $("share-collapse").classList.add("open");
    setShareStep(1, "done");
    setShareStep(2, "active");
    state.sharing = true;
    setGlobalStatus("Waiting to share", "is-waiting");
    $("share-status").className = "flow-status is-waiting";
    $("share-status").innerHTML = `<span class="dot"></span>Waiting for reply`;
    btn.innerHTML = `${icon("refresh", "icon icon-sm")} Restart session`;
    $("btn-share-stop").hidden = false;
    btn.disabled = false;
    showToast("Share session ready — send your payload.", "success");
  } catch (err) {
    setShareStep(1, "");
    btn.innerHTML = `${icon("share", "icon icon-sm")} Start sharing`;
    btn.disabled = false;
    showToast(`Couldn't start sharing: ${err}`, "error");
  }
});

$("btn-host-copy-payload")?.addEventListener("click", (e) => {
  const v = $("host-payload-box").value;
  if (!v) return showToast("Nothing to copy yet.", "warning");
  copyWithFeedback(v, e.currentTarget);
});

$("btn-host-apply-answer")?.addEventListener("click", async () => {
  const answerPayload = $("host-answer-input").value.trim();
  if (!answerPayload) return showToast("Paste the other computer's reply first.", "error");

  const btn = $("btn-host-apply-answer");
  btn.disabled = true;
  setShareStep(3, "active");
  try {
    await hostApplyAnswer(answerPayload);
  } catch (err) {
    setShareStep(3, "");
    $("share-status").className = "flow-status is-error";
    $("share-status").innerHTML = `<span class="dot"></span>Reply rejected — check the passcode`;
    showToast(`${err}`, "error");
    btn.disabled = false;
    return;
  }

  try {
    await startCapture(Number($("stream-quality")?.value || 75), Number($("stream-display")?.value || 0));
    setShareStep(2, "done");
    setShareStep(3, "done");
    $("share-status").className = "flow-status is-online";
    $("share-status").innerHTML = `<span class="dot"></span>Sharing active`;
    setGlobalStatus("Sharing active", "is-online");
    showToast("Connection accepted — your screen is being shared.", "success");
  } catch (err) {
    setShareStep(3, "");
    $("share-status").className = "flow-status is-error";
    $("share-status").innerHTML = `<span class="dot"></span>Screen capture failed — start a new session`;
    await stopCapture().catch(() => {});
    state.sharing = false;
    $("btn-share-start").innerHTML = `${icon("share", "icon icon-sm")} Start sharing`;
    showToast(`${err}`, "error");
  } finally {
    btn.disabled = false;
  }
});

$("btn-share-stop")?.addEventListener("click", async () => {
  await stopCapture().catch(() => {});
  state.sharing = false;
  $("host-payload-box").value = "";
  $("host-answer-input").value = "";
  $("share-collapse").classList.remove("open");
  $("btn-share-start").innerHTML = `${icon("share", "icon icon-sm")} Start sharing`;
  $("btn-share-stop").hidden = true;
  $("share-status").className = "flow-status";
  $("share-status").innerHTML = `<span class="dot"></span>Idle`;
  [1, 2, 3].forEach((step) => setShareStep(step, ""));
  setGlobalStatus("Ready");
  showToast("Sharing session stopped.", "info");
});

// ─────────────────────────────────────────────────────────────
// Controller Connection Flow
// ─────────────────────────────────────────────────────────────
function validateConnectForm() {
  const pass = $("ctrl-passcode")?.value.trim() ?? "";
  const deviceId = $("ctrl-device-id")?.value.trim() ?? "";
  const pl = $("ctrl-payload-input")?.value.trim() ?? "";
  const btn = $("btn-controller-connect");
  if (btn) btn.disabled = !(pass.length === 6 && (!deviceId || deviceId.length === 6) && pl.length > 0);
}

$("btn-controller-connect")?.addEventListener("click", async () => {
  const deviceId = $("ctrl-device-id").value.trim();
  const passcode = $("ctrl-passcode").value.trim();
  const hostPayload = $("ctrl-payload-input").value.trim();

  if (passcode.length !== 6) return showToast("Enter the 6-digit passcode.", "error");
  if (deviceId && deviceId.length !== 6) return showToast("Enter all 6 digits of the device ID, or leave it blank.", "error");
  if (!hostPayload) return showToast("Paste the host's payload.", "error");

  const btn = $("btn-controller-connect");
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Connecting…`;
  try {
    const answerPayload = await controllerJoinSession(hostPayload, passcode, deviceId);
    $("ctrl-answer-box").value = answerPayload;
    $("connect-collapse").classList.add("open");
    if (deviceId) {
      saveRecent(deviceId);
      renderRecentChips();
    }
    setGlobalStatus("Waiting for host", "is-waiting");
    btn.innerHTML = `${icon("check", "icon icon-sm")} Reply ready`;
    clearTimeout(state.connectionTimer);
    state.connectionTimer = setTimeout(() => {
      if (!state.connected) {
        setGlobalStatus("No screen yet", "is-waiting");
        showToast("No screen has arrived yet. Check that the host accepted your reply, or cancel and try again.", "warning", 6000);
      }
    }, 45_000);
    showToast("Reply generated — send it back to the host.", "success");
  } catch (err) {
    showToast(`${err}`, "error");
    btn.innerHTML = `Connect ${icon("arrowRight", "icon icon-sm")}`;
    validateConnectForm();
  }
});

$("btn-ctrl-copy-answer")?.addEventListener("click", (e) => {
  const v = $("ctrl-answer-box").value;
  if (!v) return;
  copyWithFeedback(v, e.currentTarget);
});

$("btn-ctrl-cancel")?.addEventListener("click", async () => {
  await stopCapture().catch(() => {});
  clearTimeout(state.connectionTimer);
  state.connectionTimer = null;
  state.connected = false;
  $("ctrl-answer-box").value = "";
  $("connect-collapse").classList.remove("open");
  $("btn-controller-connect").innerHTML = `Connect ${icon("arrowRight", "icon icon-sm")}`;
  validateConnectForm();
  setGlobalStatus("Ready");
  showToast("Connection cancelled.", "info");
});

// Keyboard shortcuts for quick connection
["ctrl-passcode", "ctrl-device-id"].forEach((id) =>
  $(id)?.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !$("btn-controller-connect").disabled) {
      $("btn-controller-connect").click();
    }
  })
);

$("ctrl-payload-input")?.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && !$("btn-controller-connect").disabled) {
    e.preventDefault();
    $("btn-controller-connect").click();
  }
});

// ─────────────────────────────────────────────────────────────
// Recent Connections History
// ─────────────────────────────────────────────────────────────
function renderRecentChips() {
  const list = getRecent();
  const wrap = $("recent-list");
  const section = $("recent-wrap");
  if (section) section.classList.toggle("hidden", !list.length);
  if (!list.length || !wrap) {
    if (wrap) wrap.innerHTML = "";
    return;
  }
  wrap.innerHTML = list
    .map((id) => `<button class="recent-chip" data-recent="${id}">${icon("monitor", "icon icon-sm")}${id}</button>`)
    .join("");

  wrap.querySelectorAll("[data-recent]").forEach((chip) =>
    chip.addEventListener("click", () => {
      $("ctrl-device-id").value = chip.dataset.recent;
      $("ctrl-passcode").focus();
      validateConnectForm();
    })
  );
}

// ─────────────────────────────────────────────────────────────
// Remote Viewer & Frame Streaming
// ─────────────────────────────────────────────────────────────
listen("connex://frame", (event) => {
  const { frame_b64, width, height } = event.payload;
  renderFrame($("remote-canvas"), frame_b64, width, height);

  if (!state.connected) {
    state.connected = true;
    clearTimeout(state.connectionTimer);
    state.connectionTimer = null;
    setGlobalStatus("Connected", "is-online");
    showScreen("remote");
    setupCanvasInteractions($("remote-canvas"), $("custom-cursor"), () => state.connected);
  }
});

listen("connex://chat", (event) => handleSessionMessage(String(event.payload ?? "")));

function handleSessionMessage(message) {
  if (message.startsWith("CXFILEMETA|")) {
    const [, id, name, mime, sizeText] = message.split("|");
    const size = Number(sizeText);
    if (!/^[\w-]{1,64}$/.test(id || "") || !Number.isInteger(size) || size < 0 || size > 2 * 1024 * 1024) return;
    if (!incomingFiles.has(id) && incomingFiles.size >= 3) incomingFiles.delete(incomingFiles.keys().next().value);
    const transfer = incomingFiles.get(id) || { chunks: new Map() };
    try {
      transfer.name = decodeURIComponent(name || "file");
      transfer.mime = decodeURIComponent(mime || "application/octet-stream");
    } catch { return; }
    transfer.size = size;
    incomingFiles.set(id, transfer);
    finishIncomingFile(id, transfer);
    return;
  }
  if (message.startsWith("CXFILE|")) {
    const [, id, indexText, totalText, data] = message.split("|");
    const index = Number(indexText), total = Number(totalText);
    if (!/^[\w-]{1,64}$/.test(id || "") || !Number.isInteger(index) || !Number.isInteger(total) || total < 1 || total > 1200 || index < 0 || index >= total || data === undefined || data.length > 2600) return;
    if (!incomingFiles.has(id) && incomingFiles.size >= 3) incomingFiles.delete(incomingFiles.keys().next().value);
    const transfer = incomingFiles.get(id) || { chunks: new Map() };
    transfer.total = total;
    transfer.chunks.set(index, data);
    incomingFiles.set(id, transfer);
    finishIncomingFile(id, transfer);
    return;
  }
  appendChatMessage(message, false);
}

function finishIncomingFile(id, transfer) {
  if (!transfer.name || !transfer.total || transfer.chunks.size !== transfer.total || transfer.url) return;
  let binary = "";
  for (let index = 0; index < transfer.total; index++) {
    const chunk = transfer.chunks.get(index);
    if (chunk === undefined) return;
    binary += atob(chunk);
  }
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  if (bytes.byteLength !== transfer.size) return;
  transfer.url = URL.createObjectURL(new Blob([bytes], { type: transfer.mime || "application/octet-stream" }));
  const row = document.createElement("div");
  row.className = "chat-message file-received";
  const label = document.createElement("span");
  label.textContent = `${transfer.name} (${(transfer.size / 1024).toFixed(0)} KB) · `;
  const link = document.createElement("a");
  link.href = transfer.url;
  link.download = transfer.name;
  link.textContent = "Save file";
  row.append(label, link);
  document.querySelectorAll(".chat-messages").forEach((list) => list.appendChild(row.cloneNode(true)));
  incomingFiles.delete(id);
}
listen("connex://connection-state", (event) => {
  const connectionState = String(event.payload ?? "");
  if (connectionState === "connected") {
    setGlobalStatus("Connected", "is-online");
  } else if (connectionState === "disconnected") {
    setGlobalStatus("Reconnecting…", "is-waiting");
  } else if (connectionState === "failed") {
    setGlobalStatus("Connection lost", "is-error");
    showToast("The peer connection failed. Check both networks and reconnect using a fresh payload.", "error", 6500);
  }
});

function appendChatMessage(message, outgoing) {
  if (!message) return;
  const row = document.createElement("div");
  row.className = `chat-message${outgoing ? " is-outgoing" : ""}`;
  row.textContent = message;
  document.querySelectorAll(".chat-messages").forEach((list) => {
    list.appendChild(row.cloneNode(true));
    list.scrollTop = list.scrollHeight;
  });
}

document.querySelectorAll(".chat-form").forEach((form) => form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const input = form.querySelector(".chat-input");
  const message = input?.value.trim();
  if (!message) return;
  try {
    await sendChatMessage(message);
    appendChatMessage(message, true);
    input.value = "";
  } catch (err) {
    showToast(`Couldn't send message: ${err}`, "error");
  }
}));

document.querySelectorAll(".chat-file-button").forEach((button) => button.addEventListener("click", () => {
  button.closest(".chat-panel, .host-chat-block")?.querySelector(".chat-file-input")?.click();
}));
document.querySelectorAll(".chat-file-input").forEach((input) => input.addEventListener("change", async (event) => {
  const fileInput = event.currentTarget;
  const file = fileInput.files?.[0];
  if (!file) return;
  if (file.size > 2 * 1024 * 1024) {
    showToast("Choose a file smaller than 2 MB.", "warning");
    fileInput.value = "";
    return;
  }
  const id = crypto.randomUUID();
  const bytes = new Uint8Array(await file.arrayBuffer());
  const parts = [];
  for (let offset = 0; offset < bytes.length; offset += 1800) {
    const slice = bytes.subarray(offset, Math.min(offset + 1800, bytes.length));
    let binary = "";
    for (const byte of slice) binary += String.fromCharCode(byte);
    parts.push(btoa(binary));
  }
  const total = Math.max(1, parts.length);
  try {
    await sendChatMessage(`CXFILEMETA|${id}|${encodeURIComponent(file.name)}|${encodeURIComponent(file.type || "application/octet-stream")}|${file.size}`);
    for (let index = 0; index < total; index++) {
      await sendChatMessage(`CXFILE|${id}|${index}|${total}|${parts[index] || ""}`);
    }
    appendChatMessage(`Sent file: ${file.name}`, true);
  } catch (err) {
    showToast(`File transfer stopped: ${err}`, "error");
  } finally {
    fileInput.value = "";
  }
}));

$("btn-disconnect")?.addEventListener("click", async () => {
  await stopCapture().catch(() => {});
  clearTimeout(state.connectionTimer);
  state.connectionTimer = null;
  state.connected = false;
  setGlobalStatus("Ready");
  showScreen("home");
  showToast("Disconnected.", "info");
});

$("btn-fullscreen")?.addEventListener("click", () => {
  if (!document.fullscreenElement) {
    $("screen-remote")?.requestFullscreen?.();
  } else {
    document.exitFullscreen?.();
  }
});

// ─────────────────────────────────────────────────────────────
// App Initialization
// ─────────────────────────────────────────────────────────────
digitsOnly($("ctrl-passcode"));
digitsOnly($("ctrl-device-id"));
$("ctrl-payload-input")?.addEventListener("input", validateConnectForm);
validateConnectForm();
renderRecentChips();
setGlobalStatus("Ready");
showScreen("home");
loadIdentity();
listDisplays().then((displays) => {
  const select = $("stream-display");
  if (!select || !displays?.length) return;
  select.innerHTML = displays.map((display) =>
    `<option value="${display.index}">Display ${display.index + 1} · ${display.width} × ${display.height}</option>`
  ).join("");
}).catch((err) => showToast(`Couldn't list displays: ${err}`, "warning", 4000));
