# Connex — Remote Desktop App

A lightweight, fast, and stealthy remote desktop application built with **Rust + Tauri**.  
Works on **macOS Silicon (ARM)** and **Windows** natively.

## Key Features

| Feature | Details |
|---|---|
| **Remote Control** | Full mouse + keyboard injection on host machine |
| **Screen Capture** | 30fps JPEG streaming via WebRTC DataChannel |
| **Stealth Mode** | Host is invisible to Zoom, Google Meet, OBS, screenshot tools |
| **No Dock/Taskbar** | Host process has zero visual presence on the OS |
| **P2P WebRTC** | Direct connection using Google STUN servers — no relay server needed |
| **Role System** | Same app; role (Host/Controller) determined at session time |
| **Tiny Binary** | ~5MB release build (vs 60MB Flutter) |

## Tech Stack

- **Backend**: Rust 1.82+ with Tokio async runtime
- **UI**: Tauri v2 + Vite + Vanilla JS
- **Screen Capture**: `scrap` crate (DXGI on Windows, CGDisplayStream on macOS)
- **Input Injection**: `enigo` crate (CGEvent on macOS, SendInput on Windows)
- **WebRTC**: `webrtc-rs` with Google STUN
- **Stealth (macOS)**: `NSWindowSharingNone` + `NSApplicationActivationPolicyProhibited`
- **Stealth (Windows)**: `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` + `WS_EX_TOOLWINDOW`

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
│   │   │   ├── session.rs      # Role & session management
│   │   │   ├── signaling.rs    # Serverless SDP signaling (base64)
│   │   │   ├── stealth.rs      # OS stealth APIs (macOS + Windows)
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

## How Connection Works (No Server!)

```
HOST (person sharing screen)          CONTROLLER (person viewing)
─────────────────────────────         ────────────────────────────
1. Generates SDP offer                
2. Encodes as base64 payload          
3. User copies & sends via chat  →    4. Pastes payload
                                       5. Generates SDP answer
                                  ←    6. User copies & sends back
7. Pastes answer, connection          
   completes via STUN P2P             7. Receives screen frames
```

## Stealth Details

### macOS
- `NSApplicationActivationPolicyProhibited` — no Dock icon, no menu bar
- `[NSWindow setSharingType: NSWindowSharingNone]` — excluded from all screen capture APIs

### Windows  
- `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` — invisible to capture tools
- `SetWindowLong(GWL_EXSTYLE, WS_EX_TOOLWINDOW)` — hidden from taskbar & Alt+Tab
- `windows_subsystem = "windows"` — no console window

## License
MIT
