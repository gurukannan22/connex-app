// ============================================================
// main.js — Connex frontend
// A single-window remote-desktop utility:
//   Home (This Computer | Connect to a Computer) → Remote view
// ============================================================

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles/main.css";

// ─────────────────────────────────────────────────────────────
// Inline SVG icons (no emoji — crisp at any size/theme)
// ─────────────────────────────────────────────────────────────
const I = {
  link:    `<path d="M9 15l6-6"/><path d="M11 6l1-1a4 4 0 0 1 6 6l-1 1"/><path d="M13 18l-1 1a4 4 0 0 1-6-6l1-1"/>`,
  monitor: `<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>`,
  pointer: `<path d="M5 3l15 7-6.5 2.2L11 19z"/>`,
  copy:    `<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/>`,
  refresh: `<path d="M20 11a8 8 0 1 0-1.5 5.5"/><path d="M20 4v5h-5"/>`,
  check:   `<path d="M4 12l5 5L20 6"/>`,
  lock:    `<rect x="4" y="10" width="16" height="10" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/>`,
  maximize:`<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>`,
  x:       `<path d="M6 6l12 12M18 6L6 18"/>`,
  info:    `<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>`,
  alert:   `<path d="M12 3l9 16H3z"/><path d="M12 10v4M12 17h.01"/>`,
  send:    `<path d="M21 4L3 11l6 2 2 6z"/><path d="M21 4l-10 9"/>`,
  arrowRight: `<path d="M5 12h14M13 6l6 6-6 6"/>`,
  share:   `<path d="M4 12v7a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-7"/><path d="M12 3v12M8 7l4-4 4 4"/>`,
  // Official Connex symbol (interlocking link), from the brand asset kit
  brand:   `<path d="M5 10.2H14.3C15.6 10.2 16.4 9.4 16.4 8.1V5.9C16.4 4.6 15.6 3.8 14.3 3.8H8.2"/><path d="M19 13.8H9.7C8.4 13.8 7.6 14.6 7.6 15.9V18.1C7.6 19.4 8.4 20.2 9.7 20.2H15.8"/>`,
};
function icon(name, cls = "icon") { return `<svg class="${cls}" viewBox="0 0 24 24">${I[name]}</svg>`; }

// ─────────────────────────────────────────────────────────────
// Mount
// ─────────────────────────────────────────────────────────────
document.getElementById("app").innerHTML = buildAppHTML();

const screens = {
  home:   document.getElementById("screen-home"),
  remote: document.getElementById("screen-remote"),
};
function showScreen(name) {
  Object.values(screens).forEach((s) => s.classList.remove("active"));
  screens[name].classList.add("active");
}

const $ = (id) => document.getElementById(id);

let state = { identity: null, connected: false, sharing: false };

// ─────────────────────────────────────────────────────────────
// Global status chip (top bar)
// ─────────────────────────────────────────────────────────────
function setGlobalStatus(label, kind = "") {
  const el = $("global-status");
  el.className = `top-status ${kind}`;
  el.innerHTML = `<span class="dot"></span>${label}`;
}

// ─────────────────────────────────────────────────────────────
// Device identity
// ─────────────────────────────────────────────────────────────
async function loadIdentity() {
  setIdentityLoading(true);
  try {
    state.identity = await invoke("get_device_identity");
    renderIdentity();
  } catch (err) {
    setIdentityLoading(false);
    showToast(`Couldn't load device identity: ${err}`, "error");
  }
}
function setIdentityLoading(loading) {
  document.querySelectorAll("[data-device-id],[data-passcode]")
    .forEach((el) => el.classList.toggle("loading", loading));
}
function renderIdentity() {
  if (!state.identity) return;
  setIdentityLoading(false);
  document.querySelectorAll("[data-device-id]").forEach((el) => { el.textContent = state.identity.device_id; });
  document.querySelectorAll("[data-passcode]").forEach((el) => { el.textContent = state.identity.passcode; });
}

// ─────────────────────────────────────────────────────────────
// Small helpers
// ─────────────────────────────────────────────────────────────
function flashCopied(btn, labelHTML) {
  const original = labelHTML ?? btn.innerHTML;
  btn.innerHTML = `${icon("check","icon icon-sm")} Copied`;
  btn.disabled = true;
  setTimeout(() => { btn.innerHTML = original; btn.disabled = false; }, 1100);
}
function copyText(text) { navigator.clipboard.writeText(text).catch(() => {}); }
function digitsOnly(el, max = 6) {
  el.addEventListener("input", () => {
    el.value = el.value.replace(/\D/g, "").slice(0, max);
    validateConnectForm();
  });
}

// ─────────────────────────────────────────────────────────────
// THIS COMPUTER — share flow
// ─────────────────────────────────────────────────────────────
$("btn-copy-identity").addEventListener("click", (e) => {
  if (!state.identity) { showToast("Device identity not loaded yet.", "warning"); return; }
  copyText(`Connex ID ${state.identity.device_id} · passcode ${state.identity.passcode}`);
  flashCopied(e.currentTarget);
});
$("btn-rotate-passcode").addEventListener("click", async () => {
  try {
    state.identity = await invoke("rotate_passcode");
    renderIdentity();
    showToast("New passcode generated.", "success");
  } catch (err) { showToast(`Couldn't update passcode: ${err}`, "error"); }
});
// click any ID / passcode value to copy it
document.addEventListener("click", (e) => {
  const el = e.target.closest("[data-device-id],[data-passcode]");
  if (!el || !state.identity || el.classList.contains("loading")) return;
  copyText(el.textContent.trim());
  showToast("Copied to clipboard.", "success", 1400);
});

$("btn-share-start").addEventListener("click", async () => {
  const btn = $("btn-share-start");
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Preparing…`;
  setShareStep(1, "active");
  try {
    const payload = await invoke("host_create_session");
    $("host-payload-box").value = payload;
    $("share-collapse").classList.add("open");
    setShareStep(1, "done"); setShareStep(2, "active");
    state.sharing = true;
    setGlobalStatus("Waiting to share", "is-waiting");
    $("share-status").className = "flow-status is-waiting";
    $("share-status").innerHTML = `<span class="dot"></span>Waiting for reply`;
    btn.innerHTML = `${icon("refresh","icon icon-sm")} Restart session`;
    btn.disabled = false;
    showToast("Share session ready — send your payload.", "success");
  } catch (err) {
    setShareStep(1, "");
    btn.innerHTML = `${icon("share","icon icon-sm")} Start sharing`;
    btn.disabled = false;
    showToast(`Couldn't start sharing: ${err}`, "error");
  }
});
$("btn-host-copy-payload").addEventListener("click", (e) => {
  const v = $("host-payload-box").value;
  if (!v) return showToast("Nothing to copy yet.", "warning");
  copyText(v); flashCopied(e.currentTarget);
});
$("btn-host-apply-answer").addEventListener("click", async () => {
  const answerPayload = $("host-answer-input").value.trim();
  if (!answerPayload) return showToast("Paste the other computer's reply first.", "error");
  const btn = $("btn-host-apply-answer");
  btn.disabled = true; setShareStep(3, "active");
  try {
    await invoke("host_apply_answer", { answerPayload });
    setShareStep(2, "done"); setShareStep(3, "done");
    $("share-status").className = "flow-status is-online";
    $("share-status").innerHTML = `<span class="dot"></span>Sharing active`;
    setGlobalStatus("Sharing active", "is-online");
    await invoke("start_capture");
    showToast("Connection accepted — your screen is being shared.", "success");
  } catch (err) {
    setShareStep(3, "");
    showToast(`${err}`, "error");
  } finally { btn.disabled = false; }
});
function setShareStep(n, cls) {
  const el = $(`share-step-${n}`);
  if (el) el.className = `stp ${cls}`;
}

// ─────────────────────────────────────────────────────────────
// CONNECT TO A COMPUTER — controller flow
// ─────────────────────────────────────────────────────────────
function validateConnectForm() {
  const pass = $("ctrl-passcode").value.trim();
  const pl   = $("ctrl-payload-input").value.trim();
  $("btn-controller-connect").disabled = !(pass.length === 6 && pl.length > 0);
}

$("btn-controller-connect").addEventListener("click", async () => {
  const deviceId    = $("ctrl-device-id").value.trim();
  const passcode    = $("ctrl-passcode").value.trim();
  const hostPayload = $("ctrl-payload-input").value.trim();
  if (passcode.length !== 6) return showToast("Enter the 6-digit passcode.", "error");
  if (!hostPayload)          return showToast("Paste the host's payload.", "error");

  const btn = $("btn-controller-connect");
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Connecting…`;
  try {
    const answerPayload = await invoke("controller_join_session", { hostPayload, passcode, deviceId });
    $("ctrl-answer-box").value = answerPayload;
    $("connect-collapse").classList.add("open");
    if (deviceId) saveRecent(deviceId);
    setGlobalStatus("Waiting for host", "is-waiting");
    btn.innerHTML = `${icon("check","icon icon-sm")} Reply ready`;
    showToast("Reply generated — send it back to the host.", "success");
  } catch (err) {
    showToast(`${err}`, "error");
    btn.innerHTML = `Connect ${icon("arrowRight","icon icon-sm")}`;
    validateConnectForm();
  }
});
$("btn-ctrl-copy-answer").addEventListener("click", (e) => {
  const v = $("ctrl-answer-box").value;
  if (!v) return;
  copyText(v); flashCopied(e.currentTarget);
});

// Enter-to-connect from the code fields
["ctrl-passcode", "ctrl-device-id"].forEach((id) =>
  $(id).addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !$("btn-controller-connect").disabled) $("btn-controller-connect").click();
  }));
$("ctrl-payload-input").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && !$("btn-controller-connect").disabled) {
    e.preventDefault(); $("btn-controller-connect").click();
  }
});

// ─────────────────────────────────────────────────────────────
// Recent connections (localStorage)
// ─────────────────────────────────────────────────────────────
const RECENT_KEY = "connex.recent";
function getRecent() {
  try { return JSON.parse(localStorage.getItem(RECENT_KEY)) || []; } catch { return []; }
}
function saveRecent(id) {
  if (!/^\d{6}$/.test(id)) return;
  let list = getRecent().filter((x) => x !== id);
  list.unshift(id);
  list = list.slice(0, 6);
  try { localStorage.setItem(RECENT_KEY, JSON.stringify(list)); } catch {}
  renderRecent();
}
function renderRecent() {
  const list = getRecent();
  const wrap = $("recent-list");
  const section = $("recent-wrap");
  if (section) section.classList.toggle("hidden", !list.length);  // hide entirely when empty
  if (!list.length) { wrap.innerHTML = ""; return; }
  wrap.innerHTML = list.map((id) =>
    `<button class="recent-chip" data-recent="${id}">${icon("monitor","icon icon-sm")}${id}</button>`).join("");
  wrap.querySelectorAll("[data-recent]").forEach((chip) =>
    chip.addEventListener("click", () => {
      $("ctrl-device-id").value = chip.dataset.recent;
      $("ctrl-passcode").focus();
      validateConnectForm();
    }));
}

// ─────────────────────────────────────────────────────────────
// Remote view
// ─────────────────────────────────────────────────────────────
listen("connex://frame", (event) => {
  const { frame_b64, width, height } = event.payload;
  renderFrame(frame_b64, width, height);
  if (!state.connected) {
    state.connected = true;
    showScreen("remote");
    setupRemoteInteractions();
  }
});

const canvas = () => $("remote-canvas");
function renderFrame(b64, w, h) {
  const c = canvas();
  c.width = w; c.height = h;
  const ctx = c.getContext("2d");
  const img = new Image();
  img.onload = () => ctx.drawImage(img, 0, 0);
  img.src = `data:image/jpeg;base64,${b64}`;
}
function setupRemoteInteractions() {
  const c = canvas();
  const cursor = $("custom-cursor");
  c.addEventListener("mousemove", (e) => {
    const r = c.getBoundingClientRect();
    cursor.style.left = `${e.clientX}px`; cursor.style.top = `${e.clientY}px`;
    invoke("inject_input", { eventJson: JSON.stringify({ type: "mouse_move", x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height }) }).catch(() => {});
  });
  ["mousedown", "mouseup"].forEach((n) => c.addEventListener(n, (e) => {
    e.preventDefault();
    const m = { 0: "left", 1: "middle", 2: "right" };
    invoke("inject_input", { eventJson: JSON.stringify({ type: "mouse_button", button: m[e.button] ?? "left", pressed: n === "mousedown" }) }).catch(() => {});
  }));
  c.addEventListener("wheel", (e) => {
    e.preventDefault();
    invoke("inject_input", { eventJson: JSON.stringify({ type: "mouse_scroll", delta_x: Math.round(e.deltaX / 10), delta_y: Math.round(e.deltaY / 10) }) }).catch(() => {});
  }, { passive: false });
  window.addEventListener("keydown", (e) => { if (!state.connected) return; e.preventDefault(); invoke("inject_input", { eventJson: JSON.stringify({ type: "key_press", key: e.key, pressed: true }) }).catch(() => {}); });
  window.addEventListener("keyup", (e) => { if (!state.connected) return; invoke("inject_input", { eventJson: JSON.stringify({ type: "key_press", key: e.key, pressed: false }) }).catch(() => {}); });
  c.addEventListener("contextmenu", (e) => e.preventDefault());
  c.addEventListener("mouseleave", () => { cursor.style.opacity = "0"; });
  c.addEventListener("mouseenter", () => { cursor.style.opacity = "1"; });
}
$("btn-disconnect").addEventListener("click", async () => {
  await invoke("stop_capture").catch(() => {});
  state.connected = false;
  setGlobalStatus("Ready");
  showScreen("home");
  showToast("Disconnected.", "info");
});
$("btn-fullscreen").addEventListener("click", () => {
  if (!document.fullscreenElement) $("screen-remote").requestFullscreen?.();
  else document.exitFullscreen?.();
});

// ─────────────────────────────────────────────────────────────
// Toasts
// ─────────────────────────────────────────────────────────────
function showToast(message, type = "info", duration = 3600) {
  const ic = { info: "info", success: "check", error: "alert", warning: "alert" }[type] ?? "info";
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.innerHTML = `${icon(ic)}<span>${message}</span>`;
  $("toast-container").appendChild(el);
  setTimeout(() => { el.style.animation = "toastIn 0.2s ease reverse"; setTimeout(() => el.remove(), 200); }, duration);
}

// ─────────────────────────────────────────────────────────────
// Markup
// ─────────────────────────────────────────────────────────────
function buildAppHTML() {
  return `
  <div class="app-shell">
    <header class="topbar">
      <div class="brand">
        <span class="brand-mark">${icon("brand")}</span>
        <span class="brand-name">Connex</span>
      </div>
      <span class="top-status" id="global-status"><span class="dot"></span>Ready</span>
    </header>

    <!-- ── Home ── -->
    <div id="screen-home" class="screen active">
      <div class="content">
        <div class="content-inner">
          <div class="page-head">
            <h1>Remote desktop</h1>
            <p>Share this computer, or connect to another.</p>
          </div>

          <div class="home-grid">

            <!-- This computer -->
            <section class="panel">
              <div class="panel-title"><span class="tile">${icon("monitor","icon icon-sm")}</span><h2>This computer</h2></div>
              <p class="panel-sub">Share your ID and passcode with someone you trust.</p>

              <div class="creds">
                <div class="cred">
                  <div class="label">Device ID</div>
                  <div class="value loading" data-device-id title="Click to copy"></div>
                </div>
                <div class="cred">
                  <div class="label">Passcode</div>
                  <div class="value loading" data-passcode title="Click to copy"></div>
                </div>
              </div>

              <div class="inline-actions">
                <button class="btn btn-ghost btn-sm" id="btn-copy-identity">${icon("copy","icon icon-sm")} Copy</button>
                <button class="btn btn-ghost btn-sm" id="btn-rotate-passcode">${icon("refresh","icon icon-sm")} New passcode</button>
              </div>

              <button class="btn btn-primary btn-lg btn-block" id="btn-share-start">${icon("share","icon icon-sm")} Start sharing</button>

              <div class="collapse" id="share-collapse">
                <div class="hr"></div>
                <div class="steps">
                  <div class="stp" id="share-step-1"><div class="stp-num">1</div><div class="stp-body"><div class="t">Send your payload</div><div class="d">Copy it and send to the other person.</div></div></div>
                  <div class="stp" id="share-step-2"><div class="stp-num">2</div><div class="stp-body"><div class="t">Wait for their reply</div><div class="d">They paste it and send a reply back.</div></div></div>
                  <div class="stp" id="share-step-3"><div class="stp-num">3</div><div class="stp-body"><div class="t">Accept the connection</div><div class="d">The passcode is checked before sharing.</div></div></div>
                </div>
                <div class="field">
                  <div class="label">Your payload</div>
                  <textarea class="input input-mono" id="host-payload-box" rows="3" readonly></textarea>
                  <button class="btn btn-ghost btn-sm" id="btn-host-copy-payload">${icon("copy","icon icon-sm")} Copy payload</button>
                </div>
                <div class="field">
                  <div class="label">Paste their reply</div>
                  <textarea class="input input-mono" id="host-answer-input" rows="2" placeholder="Paste the reply…"></textarea>
                  <button class="btn btn-primary btn-sm" id="btn-host-apply-answer">Accept &amp; connect</button>
                </div>
                <span class="flow-status" id="share-status"><span class="dot"></span>Idle</span>
              </div>
            </section>

            <!-- Connect to a computer -->
            <section class="panel">
              <div class="panel-title"><span class="tile">${icon("pointer","icon icon-sm")}</span><h2>Connect to a computer</h2></div>
              <p class="panel-sub">Enter their passcode and paste the payload they sent.</p>

              <div class="creds">
                <div class="field" style="flex:1">
                  <div class="label">Passcode</div>
                  <input class="input input-code" id="ctrl-passcode" inputmode="numeric" maxlength="6" placeholder="••••••" />
                </div>
                <div class="field" style="flex:1">
                  <div class="label">Device ID <span class="opt">(optional)</span></div>
                  <input class="input input-code" id="ctrl-device-id" inputmode="numeric" maxlength="6" placeholder="123456" />
                </div>
              </div>

              <div class="field">
                <div class="label">Payload</div>
                <textarea class="input input-mono" id="ctrl-payload-input" rows="3" placeholder="Paste the payload they sent…"></textarea>
              </div>

              <button class="btn btn-primary btn-lg btn-block" id="btn-controller-connect" disabled>Connect ${icon("arrowRight","icon icon-sm")}</button>

              <div class="collapse" id="connect-collapse">
                <div class="hr"></div>
                <div class="field">
                  <div class="label">Your reply — send this back</div>
                  <textarea class="input input-mono" id="ctrl-answer-box" rows="3" readonly></textarea>
                  <button class="btn btn-ghost btn-sm" id="btn-ctrl-copy-answer">${icon("copy","icon icon-sm")} Copy reply</button>
                </div>
                <span class="flow-status is-waiting"><span class="dot"></span>Waiting for them to accept…</span>
              </div>

              <div class="recent" id="recent-wrap">
                <div class="label">Recent</div>
                <div class="recent-list" id="recent-list"></div>
              </div>
            </section>

          </div>
        </div>
      </div>
    </div>

    <!-- ── Remote view ── -->
    <div id="screen-remote" class="screen">
      <div class="remote-shell">
        <div class="remote-toolbar">
          <span class="live"><span class="dot"></span>Connected</span>
          <button class="btn btn-sm" id="btn-fullscreen">${icon("maximize","icon icon-sm")} Fullscreen</button>
          <button class="btn btn-danger btn-sm" id="btn-disconnect">${icon("x","icon icon-sm")} Disconnect</button>
        </div>
        <div class="remote-canvas-wrap"><canvas id="remote-canvas"></canvas></div>
        <div class="custom-cursor" id="custom-cursor" style="opacity:0"></div>
      </div>
    </div>

    <div id="toast-container"></div>
  </div>`;
}

// ─────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────
digitsOnly($("ctrl-passcode"));
digitsOnly($("ctrl-device-id"));
$("ctrl-payload-input").addEventListener("input", validateConnectForm);
validateConnectForm();
renderRecent();
setGlobalStatus("Ready");
showScreen("home");
loadIdentity();
