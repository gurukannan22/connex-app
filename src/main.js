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
  hostCreateSession,
  hostApplyAnswer,
  controllerJoinSession,
  startCapture,
  stopCapture,
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
};

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
      <span class="top-status" id="global-status"><span class="dot"></span>Ready</span>
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

function copyText(text) {
  navigator.clipboard.writeText(text).catch(() => {});
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
  copyText(`Connex ID ${state.identity.device_id} · passcode ${state.identity.passcode}`);
  flashCopied(e.currentTarget);
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
  copyText(el.textContent.trim());
  showToast("Copied to clipboard.", "success", 1400);
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
  copyText(v);
  flashCopied(e.currentTarget);
});

$("btn-host-apply-answer")?.addEventListener("click", async () => {
  const answerPayload = $("host-answer-input").value.trim();
  if (!answerPayload) return showToast("Paste the other computer's reply first.", "error");

  const btn = $("btn-host-apply-answer");
  btn.disabled = true;
  setShareStep(3, "active");
  try {
    await hostApplyAnswer(answerPayload);
    setShareStep(2, "done");
    setShareStep(3, "done");
    $("share-status").className = "flow-status is-online";
    $("share-status").innerHTML = `<span class="dot"></span>Sharing active`;
    setGlobalStatus("Sharing active", "is-online");
    await startCapture();
    showToast("Connection accepted — your screen is being shared.", "success");
  } catch (err) {
    setShareStep(3, "");
    showToast(`${err}`, "error");
  } finally {
    btn.disabled = false;
  }
});

// ─────────────────────────────────────────────────────────────
// Controller Connection Flow
// ─────────────────────────────────────────────────────────────
function validateConnectForm() {
  const pass = $("ctrl-passcode")?.value.trim() ?? "";
  const pl = $("ctrl-payload-input")?.value.trim() ?? "";
  const btn = $("btn-controller-connect");
  if (btn) btn.disabled = !(pass.length === 6 && pl.length > 0);
}

$("btn-controller-connect")?.addEventListener("click", async () => {
  const deviceId = $("ctrl-device-id").value.trim();
  const passcode = $("ctrl-passcode").value.trim();
  const hostPayload = $("ctrl-payload-input").value.trim();

  if (passcode.length !== 6) return showToast("Enter the 6-digit passcode.", "error");
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
  copyText(v);
  flashCopied(e.currentTarget);
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
    showScreen("remote");
    setupCanvasInteractions($("remote-canvas"), $("custom-cursor"), () => state.connected);
  }
});

$("btn-disconnect")?.addEventListener("click", async () => {
  await stopCapture().catch(() => {});
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
