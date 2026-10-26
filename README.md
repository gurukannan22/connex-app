# Connex — Remote Desktop App

A remote support desktop app built with **Rust + Tauri v2**, taking its host/connect workflow from tools such as UltraViewer.

Connex currently uses a manual WebRTC offer/reply exchange: the host shares an encoded session payload, and the controller returns an encoded reply. It does not yet connect using only a partner ID and password; that flow requires a deployed signaling service.

---

## Key Features

| Feature | Details |
|---|---|
| **Host and controller** | 6-digit device ID and rotatable passcode, with host approval before capture starts |
| **Screen sharing** | Peer-to-peer WebRTC stream after exchanging offer and reply payloads |
| **Remote control** | Mouse, scroll, and keyboard input over a WebRTC data channel |
| **Session chat** | Peer-to-peer text chat while connected |
| **File handoff** | Explicitly selected files up to 2 MB, sent over the encrypted session and saved by the receiver |
| **Display and quality** | Choose a monitor and data-saver, balanced, or high JPEG quality before sharing |
| **Stealth option** | Explicit opt-in exclusion of the Connex window from supported capture APIs; Connex stays visible and usable locally |
| **Recovery feedback** | Connected, reconnecting, and failed peer state are surfaced, with guidance to create a fresh payload after failure |
| **Windows setup** | NSIS and MSI installers include the WebView2 offline installer |

The current connection setup requires both users to exchange payloads manually. Connex uses STUN for direct peer discovery and does not include a TURN relay, so some firewall and NAT combinations will prevent a connection.

## Connection workflows

### Happy flow

1. The host checks the displayed ID and passcode, chooses a display and stream quality, then selects **Start sharing**.
2. The host copies the offer payload and sends it to the controller through a trusted channel.
3. The controller enters the host passcode, optionally verifies the ID, pastes the offer, and selects **Connect**.
4. The controller sends the generated reply payload back to the host.
5. The host pastes it and selects **Accept & connect**. The app verifies the current passcode, then starts screen capture and remote control.
6. Either peer can chat, send a file (the receiver chooses **Save file**), or disconnect. The host can stop sharing at any time.

### Negative and recovery flows

- **Malformed offer/reply or wrong ID:** the app rejects the payload and shows a corrective message. The host can regenerate the offer.
- **Incorrect or rotated passcode:** the answer is rejected; the host shares the current passcode and a fresh offer.
- **No ICE gathering / slow network:** session preparation times out with a network hint. Retry on a stable network.
- **Firewall/NAT blocks direct WebRTC:** the peer state reports failure; Connex currently has no TURN relay, so the peers may need a different network.
- **Connection drops:** the app reports reconnecting or connection lost. A failed session should be stopped and restarted with a fresh offer/reply exchange.
- **Capture/display unavailable:** capture reports an error and the host must select an available display and start a new share.
- **File is over 2 MB:** the app refuses it before transfer. Incoming files are only written to disk when the receiver chooses **Save file**.
- **Stealth unsupported or OS refuses the setting:** the app disables it on unsupported platforms or reports the native error; it never enables itself silently.

## Enhancements still requiring external setup

- **Direct ID + passcode connection:** requires a deployed signaling service with authenticated device registration, short-lived session codes, rate limits, and abuse controls. The current six-digit ID is not globally registered and must not be treated as a unique public routing address.
- **Reliable connections across restrictive networks:** requires TURN relay infrastructure, credentials, and operational monitoring.
- **Signed installers and macOS notarization:** require Windows code-signing certificate secrets and Apple Developer signing/notarization credentials. The current release job builds unsigned packages.
- **Unattended access:** intentionally remains out of scope for this build. Supporting it safely needs an explicit opt-in policy, OS-protected credential storage, visible service/activity controls, and a revocation path.
- **Large file transfer, clipboard sync, multi-monitor viewing, and adaptive bitrate:** not included; file handoff is currently limited to 2 MB and one selected monitor per share.

---

## Live Website & Downloads

- **GitHub Pages**: [https://gurukannan22.github.io/connex-app/](https://gurukannan22.github.io/connex-app/)
- **Downloads Available**:
  - macOS (Apple Silicon): DMG
  - Windows (x64): NSIS EXE and MSI

The release workflow does not currently build Intel macOS or Linux packages.

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
3. Deploy the landing page to GitHub Pages from pushes to `main`.

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
