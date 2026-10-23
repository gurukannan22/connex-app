# Connex — Remote Desktop App

A lightweight, fast, ultra-secure remote desktop application built with **Rust + Tauri v2**.  
Runs natively on **macOS Apple Silicon (ARM64)**, **macOS Intel**, and **Windows 10/11**.

Connex provides UltraViewer-style remote desktop functionality with **native screen-sharing invisibility**: while sharing your screen on Google Meet, Zoom, Microsoft Teams, Slack, or OBS, Connex is completely excluded from the video capture.

---

## Key Features

| Feature | Details |
|---|---|
| **Meeting Window Privacy** | Keeps remote session window private during screen shares (Zoom, Meet, Teams, Slack) |
| **Device ID & Passcode** | 6-digit Device ID + rotatable 6-digit passcode for consent-based access |
| **Direct P2P WebRTC** | Direct peer-to-peer display streaming at 30 FPS |
| **Remote Control** | Low-latency mouse movement, click, scroll, and keystroke injection |
| **End-to-End Encrypted** | Encrypted with DTLS-SRTP — no central server relaying your desktop frames |
| **Ultra Lightweight** | Built in Rust. Tiny memory footprint (<30MB RAM) and instant launch |

---

## Live Website & Downloads

- **GitHub Pages**: [https://gurukannan22.github.io/connex-app/](https://gurukannan22.github.io/connex-app/)
- **Downloads Available**:
  - macOS (Apple Silicon): `Connex_aarch64.dmg`
  - macOS (Intel): `Connex_x64.dmg`
  - Windows: `Connex_x64-setup.exe` & `Connex_x64.msi`

---

## Tech Stack

- **Backend**: Rust with Tokio async runtime
- **UI & App Shell**: Tauri v2 + Vite + Vanilla CSS/JS
- **Screen Capture**: `scrap` crate
- **Input Injection**: `enigo` crate
- **Networking**: `webrtc-rs` with direct data channels (`connex-frames`, `connex-input`)

---

## Project Structure

```
connex-app/
├── .github/
│   └── workflows/
│       └── release.yml     # Builds .dmg & .exe and deploys GitHub Pages
├── website/                # GitHub Pages Landing & Download Site
│   ├── index.html          # Responsive landing page with OS detection
│   ├── styles.css          # Modern dark design system
│   ├── script.js           # Live release fetcher & download linker
│   └── assets/             # Brand logos and icons
├── src/                    # Frontend Webview (Vite + JS)
│   ├── main.js             # SPA: Home (Allow / Connect) → Remote View
│   └── styles/main.css     # Clean desktop utility design
├── src-tauri/
│   ├── icons/              # Multi-resolution macOS .icns, Windows .ico & PNGs
│   ├── src/
│   │   ├── core/
│   │   │   ├── capture.rs      # Cross-platform screen capture
│   │   │   ├── identity.rs     # Persistent 6-digit Device ID + Passcode
│   │   │   ├── input.rs        # Mouse & keyboard injection
│   │   │   ├── session.rs      # Session & role state
│   │   │   ├── signaling.rs    # Serverless SDP offer/answer encoding
│   │   │   ├── stealth.rs      # Screen-share exclusion (macOS & Windows)
│   │   │   └── webrtc_peer.rs  # WebRTC peer connection & data channels
│   │   ├── ipc/
│   │   │   └── commands.rs     # Tauri IPC commands
│   │   ├── lib.rs              # Tauri setup & stealth lifecycle
│   │   └── main.rs             # Application entry point
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
└── vite.config.js
```

---

## Local Development

```bash
# Install frontend dependencies
npm install

# Run in development mode
source "$HOME/.cargo/env"
npm run tauri dev
```

---

## Building Binaries & Releases

### Local Build:
```bash
# macOS .dmg / .app
npm run tauri build

# Windows .exe / .msi (run on Windows or via GitHub Actions)
npm run tauri build
```

### Automated CI/CD (GitHub Actions):
Push a version tag (e.g., `v0.1.0`) or trigger the workflow manually from the **Actions** tab:
```bash
git tag v0.1.0
git push origin v0.1.0
```
GitHub Actions will:
1. Matrix build macOS Apple Silicon (`.dmg`) and Windows 64-bit (`.exe` installer + `.msi`).
2. Publish an official GitHub Release with the binaries attached.
3. Deploy the modern landing page to GitHub Pages with auto-detected download links.

---

## macOS Gatekeeper Troubleshooting ("App is damaged / Move to Bin")

Because Connex is an open-source tool distributed outside the Mac App Store without a paid Apple Developer ID certificate ($99/year), macOS Gatekeeper flags downloaded binaries with a quarantine attribute.

If macOS displays:
> *"Connex is damaged and can't be opened. You should move it to the Bin / Trash."*

Run this one-time command in your macOS **Terminal**:
```bash
xattr -cr /Applications/Connex.app
```
Then launch Connex normally from your Applications folder or Spotlight.

---

## License
MIT License. Built for confidential remote assistance and diagnostics.
