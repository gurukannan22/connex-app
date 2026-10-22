// ============================================================
// main.js  — Connex Frontend Entry Point
// Renders the full SPA: Home → Host/Controller → Remote View
// ============================================================

import { invoke }       from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import "./styles/main.css";

// ─────────────────────────────────────────────────────────────
// App mount
// ─────────────────────────────────────────────────────────────
document.getElementById("app").innerHTML = buildAppHTML();

const screens = {
  home:       document.getElementById("screen-home"),
  host:       document.getElementById("screen-host"),
  controller: document.getElementById("screen-controller"),
  remote:     document.getElementById("screen-remote"),
};

function showScreen(name) {
  Object.values(screens).forEach((s) => s.classList.remove("active"));
  screens[name].classList.add("active");
}

// ─────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────
let appState = {
  role: null,         // "host" | "controller"
  sessionCode: null,
  hostPayload: null,
  connected: false,
};

// ─────────────────────────────────────────────────────────────
// Home screen
// ─────────────────────────────────────────────────────────────
document.getElementById("btn-become-host").addEventListener("click", () => {
  appState.role = "host";
  initHostFlow();
});

document.getElementById("btn-become-controller").addEventListener("click", () => {
  appState.role = "controller";
  showScreen("controller");
});

// ─────────────────────────────────────────────────────────────
// HOST FLOW
// ─────────────────────────────────────────────────────────────
async function initHostFlow() {
  showScreen("host");

  setHostStep(1, "loading");
  showToast("Initializing secure session…", "info");

  try {
    // Apply stealth FIRST — hide host from screen capture & taskbar
    await invoke("apply_stealth");

    // Generate WebRTC offer + session code
    const result = await invoke("host_create_session");
    const [code, payload] = result.split("|", 2);

    appState.sessionCode = code;
    appState.hostPayload = payload;

    // Update UI
    document.getElementById("host-code").textContent = code;
    document.getElementById("host-payload-box").value = `${code}|${payload}`;
    setHostStep(1, "done");
    setHostStep(2, "active");

    showToast(`Session code: ${code}`, "success");
  } catch (err) {
    showToast(`Error: ${err}`, "error");
    setHostStep(1, "error");
  }
}

document.getElementById("btn-host-copy-payload").addEventListener("click", () => {
  const payload = document.getElementById("host-payload-box").value;
  navigator.clipboard.writeText(payload);
  showToast("Payload copied to clipboard!", "success");
});

document.getElementById("btn-host-apply-answer").addEventListener("click", async () => {
  const answerPayload = document.getElementById("host-answer-input").value.trim();
  if (!answerPayload) {
    showToast("Please paste the controller's answer payload", "error");
    return;
  }

  setHostStep(3, "loading");
  try {
    await invoke("host_apply_answer", { answerPayload });
    setHostStep(2, "done");
    setHostStep(3, "done");
    showToast("Connected! Starting screen capture…", "success");
    await invoke("start_capture");
    updateHostStatus("connected");
  } catch (err) {
    showToast(`Connection failed: ${err}`, "error");
    setHostStep(3, "error");
  }
});

function setHostStep(num, state) {
  const step = document.getElementById(`host-step-${num}`);
  if (!step) return;
  step.className = `step ${state === "active" ? "active" : state === "done" ? "done" : ""}`;
}

function updateHostStatus(status) {
  const badge = document.getElementById("host-status-badge");
  if (status === "connected") {
    badge.className = "badge badge-connected";
    badge.innerHTML = `<span class="badge-dot"></span> Connected`;
  } else {
    badge.className = "badge badge-waiting";
    badge.innerHTML = `<span class="badge-dot"></span> Waiting for controller…`;
  }
}

// Back buttons
document.getElementById("btn-host-back").addEventListener("click", () => {
  showScreen("home");
  invoke("stop_capture").catch(() => {});
});

// ─────────────────────────────────────────────────────────────
// CONTROLLER FLOW
// ─────────────────────────────────────────────────────────────
document.getElementById("btn-controller-connect").addEventListener("click", async () => {
  const input = document.getElementById("ctrl-payload-input").value.trim();
  if (!input) {
    showToast("Please paste the host's payload", "error");
    return;
  }

  const btn = document.getElementById("btn-controller-connect");
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Connecting…`;

  try {
    const answerPayload = await invoke("controller_join_session", { hostPayload: input });

    // Show answer payload for user to send back to host
    document.getElementById("ctrl-answer-box").value = answerPayload;
    document.getElementById("ctrl-answer-section").classList.remove("hidden");
    showToast("Answer generated! Send it to the host.", "success");

    btn.innerHTML = "✓ Answer Ready";
    btn.style.background = "rgba(34,211,160,0.15)";
    btn.style.color = "var(--success)";
  } catch (err) {
    showToast(`Failed: ${err}`, "error");
    btn.disabled = false;
    btn.innerHTML = "Connect";
  }
});

document.getElementById("btn-ctrl-copy-answer").addEventListener("click", () => {
  const answer = document.getElementById("ctrl-answer-box").value;
  navigator.clipboard.writeText(answer);
  showToast("Answer copied! Send it to the host.", "success");
});

document.getElementById("btn-ctrl-back").addEventListener("click", () => {
  showScreen("home");
});

// Listen for screen frames from Rust
listen("connex://frame", (event) => {
  const { frame_b64, width, height } = event.payload;
  renderFrame(frame_b64, width, height);
  if (!appState.connected) {
    appState.connected = true;
    showScreen("remote");
    setupRemoteInteractions();
  }
});

// ─────────────────────────────────────────────────────────────
// REMOTE VIEW — Canvas rendering + Input capture
// ─────────────────────────────────────────────────────────────
const canvas  = document.getElementById("remote-canvas");
const ctx     = canvas.getContext("2d");
const cursor  = document.getElementById("custom-cursor");

function renderFrame(b64, w, h) {
  canvas.width  = w;
  canvas.height = h;

  const img = new Image();
  img.onload = () => ctx.drawImage(img, 0, 0);
  img.src = `data:image/jpeg;base64,${b64}`;
}

function setupRemoteInteractions() {
  // Custom cursor tracking
  canvas.addEventListener("mousemove", (e) => {
    const rect  = canvas.getBoundingClientRect();
    const normX = (e.clientX - rect.left) / rect.width;
    const normY = (e.clientY - rect.top)  / rect.height;

    cursor.style.left = `${e.clientX}px`;
    cursor.style.top  = `${e.clientY}px`;

    invoke("inject_input", {
      eventJson: JSON.stringify({ type: "mouse_move", x: normX, y: normY }),
    }).catch(() => {});
  });

  // Mouse buttons
  ["mousedown", "mouseup"].forEach((evtName) => {
    canvas.addEventListener(evtName, (e) => {
      e.preventDefault();
      const btnMap = { 0: "left", 1: "middle", 2: "right" };
      invoke("inject_input", {
        eventJson: JSON.stringify({
          type: "mouse_button",
          button: btnMap[e.button] ?? "left",
          pressed: evtName === "mousedown",
        }),
      }).catch(() => {});
    });
  });

  // Mouse scroll
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    invoke("inject_input", {
      eventJson: JSON.stringify({
        type: "mouse_scroll",
        delta_x: Math.round(e.deltaX / 10),
        delta_y: Math.round(e.deltaY / 10),
      }),
    }).catch(() => {});
  }, { passive: false });

  // Keyboard
  window.addEventListener("keydown", (e) => {
    if (!appState.connected) return;
    e.preventDefault();
    invoke("inject_input", {
      eventJson: JSON.stringify({ type: "key_press", key: e.key, pressed: true }),
    }).catch(() => {});
  });

  window.addEventListener("keyup", (e) => {
    if (!appState.connected) return;
    invoke("inject_input", {
      eventJson: JSON.stringify({ type: "key_press", key: e.key, pressed: false }),
    }).catch(() => {});
  });

  // Context menu — suppress on canvas
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());

  // Show/hide cursor
  canvas.addEventListener("mouseleave", () => { cursor.style.opacity = "0"; });
  canvas.addEventListener("mouseenter", () => { cursor.style.opacity = "1"; });
}

// Remote toolbar buttons
document.getElementById("btn-disconnect").addEventListener("click", async () => {
  await invoke("stop_capture").catch(() => {});
  appState.connected = false;
  showScreen("home");
  showToast("Disconnected", "info");
});

document.getElementById("btn-fullscreen").addEventListener("click", () => {
  if (!document.fullscreenElement) {
    document.getElementById("screen-remote").requestFullscreen();
  } else {
    document.exitFullscreen();
  }
});

// ─────────────────────────────────────────────────────────────
// Toast system
// ─────────────────────────────────────────────────────────────
function showToast(message, type = "info", duration = 3500) {
  const icons = { info: "ℹ️", success: "✅", error: "❌", warning: "⚠️" };
  const container = document.getElementById("toast-container");
  const toast = document.createElement("div");
  toast.className = `toast ${type}`;
  toast.innerHTML = `<span>${icons[type] ?? "•"}</span><span>${message}</span>`;
  container.appendChild(toast);
  setTimeout(() => {
    toast.style.animation = "slideIn 0.3s ease reverse";
    setTimeout(() => toast.remove(), 300);
  }, duration);
}

// ─────────────────────────────────────────────────────────────
// HTML Builder
// ─────────────────────────────────────────────────────────────
function buildAppHTML() {
  return `
    <!-- ── Home Screen ── -->
    <div id="screen-home" class="screen active">
      <div class="home-container">
        <div class="logo-row">
          <img src="/connex-logo.png" alt="Connex Logo" class="logo-icon" style="object-fit: cover; border-radius: 12px;" />
          <span class="logo-text gradient-text">Connex</span>
        </div>

        <div>
          <h1>Remote Desktop,<br/><span class="gradient-text">Redefined.</span></h1>
          <p style="margin-top:12px">Lightning-fast, fully encrypted, invisible to screen sharing tools.</p>
        </div>

        <div class="role-cards">
          <div class="role-card host" id="btn-become-host">
            <div class="role-icon">🖥️</div>
            <h3>Share My Screen</h3>
            <p>Let someone control this machine. Your session stays hidden from meeting apps.</p>
          </div>
          <div class="role-card control" id="btn-become-controller">
            <div class="role-icon">🎮</div>
            <h3>Control Remote</h3>
            <p>Enter a session code to view and control another machine remotely.</p>
          </div>
        </div>

        <div style="display:flex;align-items:center;justify-content:center;gap:16px;flex-wrap:wrap">
          <span class="badge badge-disconnected"><span class="badge-dot"></span> Not connected</span>
          <span class="text-muted">End-to-end encrypted · P2P · No servers</span>
        </div>
      </div>
    </div>

    <!-- ── Host Setup Screen ── -->
    <div id="screen-host" class="screen">
      <div class="setup-container">
        <div class="setup-header">
          <button class="back-btn" id="btn-host-back">← Back</button>
          <div style="display:flex;align-items:center;justify-content:space-between">
            <h2>Share Your Screen</h2>
            <span class="badge badge-waiting" id="host-status-badge">
              <span class="badge-dot"></span> Waiting…
            </span>
          </div>
          <p>Follow these steps to let the controller connect.</p>
        </div>

        <div class="step-list">
          <!-- Step 1 -->
          <div class="step active" id="host-step-1">
            <div class="step-num">1</div>
            <div class="step-content">
              <h4>Your session payload</h4>
              <p>Copy this and send it to the controller (chat, email, etc.)</p>
              <div style="margin-top:12px;display:flex;flex-direction:column;gap:8px">
                <textarea id="host-payload-box" class="input" rows="3"
                  style="font-size:0.7rem;letter-spacing:0;text-transform:none;resize:none"
                  readonly placeholder="Generating…"></textarea>
                <button class="btn btn-primary btn-sm" id="btn-host-copy-payload">
                  📋 Copy Payload
                </button>
              </div>
            </div>
          </div>

          <!-- Step 2 -->
          <div class="step" id="host-step-2">
            <div class="step-num">2</div>
            <div class="step-content">
              <h4>Wait for controller's answer</h4>
              <p>They'll send you a short payload string back.</p>
            </div>
          </div>

          <!-- Step 3 -->
          <div class="step" id="host-step-3">
            <div class="step-num">3</div>
            <div class="step-content">
              <h4>Paste their answer &amp; connect</h4>
              <div style="margin-top:12px;display:flex;flex-direction:column;gap:8px">
                <input id="host-answer-input" class="input" type="text"
                  placeholder="Paste controller answer here…" />
                <button class="btn btn-primary" id="btn-host-apply-answer">
                  🔗 Establish Connection
                </button>
              </div>
            </div>
          </div>
        </div>

        <p class="text-muted text-center" style="font-size:0.75rem">
          🔒 Your screen is invisible to Zoom, Google Meet & all capture tools once connected.
        </p>
      </div>
    </div>

    <!-- ── Controller Screen ── -->
    <div id="screen-controller" class="screen">
      <div class="setup-container">
        <div class="setup-header">
          <button class="back-btn" id="btn-ctrl-back">← Back</button>
          <h2>Connect to Remote</h2>
          <p>Paste the host's payload to generate your answer.</p>
        </div>

        <div class="card" style="padding:24px;display:flex;flex-direction:column;gap:16px">
          <div>
            <label style="font-size:0.85rem;color:var(--text-secondary);margin-bottom:8px;display:block">
              Host Session Payload
            </label>
            <textarea id="ctrl-payload-input" class="input" rows="4"
              style="font-size:0.75rem;letter-spacing:0;text-transform:none;resize:none"
              placeholder="Paste the host's payload here…"></textarea>
          </div>
          <button class="btn btn-primary btn-lg w-full" id="btn-controller-connect">
            🔗 Connect
          </button>
        </div>

        <div id="ctrl-answer-section" class="card hidden"
          style="padding:24px;display:flex;flex-direction:column;gap:16px">
          <div>
            <h3 style="margin-bottom:4px">Your Answer</h3>
            <p style="font-size:0.8rem">Send this back to the host so they can complete the connection.</p>
          </div>
          <textarea id="ctrl-answer-box" class="input" rows="4"
            style="font-size:0.75rem;letter-spacing:0;text-transform:none;resize:none"
            readonly></textarea>
          <button class="btn btn-primary btn-sm" id="btn-ctrl-copy-answer">
            📋 Copy Answer
          </button>
          <p class="text-muted text-center">
            ⏳ Waiting for host to paste your answer and confirm connection…
          </p>
        </div>
      </div>
    </div>

    <!-- ── Remote View Screen ── -->
    <div id="screen-remote" class="screen">
      <div class="remote-toolbar">
        <span class="badge badge-connected" style="margin-right:auto">
          <span class="badge-dot"></span> Connected
        </span>
        <button class="btn btn-ghost btn-sm" id="btn-fullscreen">⛶ Fullscreen</button>
        <button class="btn btn-danger btn-sm" id="btn-disconnect">✕ Disconnect</button>
      </div>
      <div class="remote-canvas-wrap">
        <canvas id="remote-canvas"></canvas>
      </div>
      <div class="custom-cursor" id="custom-cursor" style="opacity:0"></div>
    </div>

    <!-- ── Toast Container ── -->
    <div id="toast-container"></div>
  `;
}

// ─────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────
showScreen("home");
