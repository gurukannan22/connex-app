# Connex — Remote Desktop App

A lightweight, fast remote desktop application built with **Rust + Tauri**.  
Works on **macOS Silicon (ARM)** and **Windows** natively.

Connex is a **consent-based** remote desktop tool: the host machine runs it as a
normal, visible app (it shows in the Dock/taskbar and in any screen share), and
the host must explicitly accept each incoming connection.

## Key Features

| Feature | Details |
|---|---|
| **Static Device ID** | Every machine has a fixed 6-digit ID + passcode for pairing |
| **Host Accepts** | Nothing connects until the host approves and the passcode matches |
| **Visible App** | Runs like any normal window — appears in screen shares |
| **Remote Control** | Full mouse + keyboard injection on host machine |
| **Screen Capture** | 30fps JPEG streaming |
| **P2P WebRTC** | Direct connection using Google STUN servers — no relay server needed |
| **Role System** | Same app; role (Host/Controller) determined at session time |
| **Tiny Binary** | ~5MB release build (vs 60MB Flutter) |

## Tech Stack

- **Backend**: Rust 1.82+ with Tokio async runtime
- **UI**: Tauri v2 + Vite + Vanilla JS
- **Screen Capture**: `scrap` crate (DXGI on Windows, CGDisplayStream on macOS)
- **Input Injection**: `enigo` crate (CGEvent on macOS, SendInput on Windows)
- **WebRTC**: `webrtc-rs` with Google STUN
- **Identity**: static 6-digit device ID (derived from the machine) + passcode, persisted in the app config dir

## Project Structure

```
connex-app/
├── src/                    # Frontend (Vite + JS)
│   ├── main.js             # SPA: Home → Host/Controller → Remote View
│   └── styles/main.css     # Design system (dark glassmorphism)
├── src-tauri/
│   ├── src/
│   │   ├── core/
│   │   │   ├── capture.rs      # Screen capture → JPEG frames
│   │   │   ├── input.rs        # Mouse/keyboard injection
│   │   │   ├── identity.rs     # Static 6-digit device ID + passcode
│   │   │   ├── session.rs      # Role & session management
│   │   │   ├── signaling.rs    # Serverless SDP signaling (base64)
│   │   │   └── webrtc_peer.rs  # WebRTC peer connection
│   │   ├── ipc/
│   │   │   └── commands.rs     # Tauri commands exposed to frontend
│   │   ├── lib.rs              # App entry + window management
│   │   └── main.rs             # Binary entry point
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── package.json
└── vite.config.js
```

## Development Setup

### Prerequisites
- Rust 1.82+ (`rustup` — automatically installed by setup)
- Node.js 18+
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: Visual Studio Build Tools 2022 + WebView2

### Run in development
```bash
npm install
source "$HOME/.cargo/env"   # macOS/Linux
npm run tauri dev
```

### Build for release
```bash
npm run tauri build
```
Outputs:
- `src-tauri/target/release/bundle/macos/Connex.app`
- `src-tauri/target/release/bundle/windows/Connex_installer.exe`

## How Connection Works

Each machine has a **static 6-digit device ID** and a **passcode** (shown on the
home screen). The controller enters both, and the host must accept:

```
HOST (shares screen)                  CONTROLLER (views & controls)
─────────────────────────────        ────────────────────────────
0. Shows device ID + passcode    →    0. Learns ID + passcode (out of band)
1. Generates SDP offer payload   →    2. Enters ID + passcode, pastes payload
                                      3. Generates SDP answer (carries passcode)
4. Pastes answer                 ←    3. Sends answer back
5. Verifies passcode & ACCEPTS        5. Connection completes via STUN P2P
   → nothing starts until step 5
```

> **Note:** connecting by typing the ID + passcode *alone* (no payload
> copy/paste) requires a small signaling server to look up which machine owns an
> ID. That server is not included yet — see "Roadmap" below. Today the SDP
> payload is still exchanged out of band, with the ID + passcode acting as the
> pairing/authorization check.

## Consent & Visibility

Connex deliberately runs as a **normal, visible application**:

- It appears in the macOS Dock / Windows taskbar.
- It is **not** excluded from screen capture — if the host shares their screen
  on a call, Connex shows up like any other window.
- The host must **explicitly accept** every incoming connection, and the
  controller's passcode is verified before anything starts.

## Roadmap

- **Signaling server** — connect by ID + passcode only, no payload copy/paste.
- **WebRTC data-channel frame transport** — stream frames peer-to-peer (today
  capture is wired to a local event path).

## License
MIT
