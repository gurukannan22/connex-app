// ============================================================
// Connex Landing Page Scripts
// - OS auto-detection for primary download button
// - Live GitHub Release assets fetcher & dynamic link binding
// ============================================================

const GITHUB_REPO = "gurukannan22/connex-app";
const RELEASES_API = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
const LATEST_RELEASE_URL = `https://github.com/gurukannan22/connex-app/releases/latest`;

// Direct fallback download links matching Tauri v2 default bundle naming
const FALLBACK_DOWNLOADS = {
  macSilicon: `${LATEST_RELEASE_URL}/download/Connex_aarch64.dmg`,
  macIntel:   `${LATEST_RELEASE_URL}/download/Connex_x64.dmg`,
  winExe:     `${LATEST_RELEASE_URL}/download/Connex_x64-setup.exe`,
  winMsi:     `${LATEST_RELEASE_URL}/download/Connex_x64_en-US.msi`,
};

document.addEventListener("DOMContentLoaded", () => {
  setupOSDetection();
  setupMockupTabs();
  fetchLatestRelease();
});

function setupMockupTabs() {
  const tabHome = document.getElementById("tab-home");
  const tabSession = document.getElementById("tab-session");
  const viewHome = document.getElementById("view-home");
  const viewSession = document.getElementById("view-session");

  if (!tabHome || !tabSession) return;

  function switchToHome() {
    tabHome.classList.add("active");
    tabSession.classList.remove("active");
    viewHome.classList.add("active");
    viewSession.classList.remove("active");
  }

  function switchToSession() {
    tabSession.classList.add("active");
    tabHome.classList.remove("active");
    viewSession.classList.add("active");
    viewHome.classList.remove("active");
  }

  tabHome.addEventListener("click", switchToHome);
  tabSession.addEventListener("click", switchToSession);

  // Connect mock button switches to session view
  const btnConnect = document.querySelector(".btn-connect-mock");
  if (btnConnect) {
    btnConnect.addEventListener("click", switchToSession);
  }

  // Disconnect mock button switches back to home setup
  const btnDisconnect = document.querySelector(".session-btn-disconnect");
  if (btnDisconnect) {
    btnDisconnect.addEventListener("click", switchToHome);
  }

  // Mock Copy & Rotate buttons
  const credButtons = document.querySelectorAll(".cred-btn");
  credButtons.forEach((btn) => {
    btn.addEventListener("click", (e) => {
      const text = btn.textContent.trim();
      if (text.includes("Copy")) {
        const idSpan = btn.parentElement.querySelector(".code-text");
        if (idSpan && navigator.clipboard) {
          navigator.clipboard.writeText(idSpan.textContent.replace(/\s+/g, "")).catch(() => {});
        }
        const orig = btn.textContent;
        btn.textContent = "Copied!";
        btn.disabled = true;
        setTimeout(() => {
          btn.textContent = orig;
          btn.disabled = false;
        }, 1200);
      } else if (text.includes("Rotate")) {
        const passSpan = btn.parentElement.querySelector(".code-text");
        if (passSpan) {
          const rand = Math.floor(100000 + Math.random() * 900000);
          const formatted = `${String(rand).slice(0, 3)} ${String(rand).slice(3)}`;
          passSpan.textContent = formatted;
        }
      }
    });
  });
}

// ─────────────────────────────────────────────────────────────
// OS Detection
// ─────────────────────────────────────────────────────────────
function detectOS() {
  const ua = navigator.userAgent || "";
  const platform = navigator.platform || "";

  if (/Mac/i.test(platform) || /Macintosh|Mac OS X/i.test(ua)) {
    // Check for Apple Silicon if possible or default to Silicon
    return { os: "mac", arch: "arm64" };
  } else if (/Win/i.test(platform) || /Windows/i.test(ua)) {
    return { os: "windows", arch: "x64" };
  } else if (/Linux/i.test(platform) || /Linux/i.test(ua)) {
    return { os: "linux", arch: "x64" };
  }
  return { os: "mac", arch: "arm64" }; // Default fallback
}

function setupOSDetection() {
  const detected = detectOS();
  const ctaBtn = document.getElementById("btn-primary-dl");
  const ctaLabel = document.getElementById("cta-label");
  const ctaSub = document.getElementById("cta-sub");
  const osIcon = document.getElementById("os-icon");

  if (detected.os === "windows") {
    ctaLabel.textContent = "Download for Windows";
    ctaSub.textContent = "Windows 10 & 11 • .exe Installer";
    ctaBtn.href = FALLBACK_DOWNLOADS.winExe;
    osIcon.innerHTML = `<path d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801"/>`;
  } else {
    // macOS
    ctaLabel.textContent = "Download for macOS";
    ctaSub.textContent = "Apple Silicon • .dmg";
    ctaBtn.href = FALLBACK_DOWNLOADS.macSilicon;
    osIcon.innerHTML = `<path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M15.97 6.37c.61-.75 1.04-1.8 1.01-2.87-.96.04-2.1.65-2.77 1.44-.59.69-.99 1.74-.94 2.8.06.01.12.02.19.02.94 0 1.9-.64 2.51-1.39z"/>`;
  }
}

// ─────────────────────────────────────────────────────────────
// GitHub Releases API Fetcher
// ─────────────────────────────────────────────────────────────
async function fetchLatestRelease() {
  const linkMacArm = document.getElementById("link-mac-arm");
  const linkMacIntel = document.getElementById("link-mac-intel");
  const linkWinExe = document.getElementById("link-win-exe");
  const linkWinMsi = document.getElementById("link-win-msi");
  const relTag = document.getElementById("rel-tag");
  const relPublished = document.getElementById("rel-published");
  const relLink = document.getElementById("rel-link");

  try {
    const res = await fetch(RELEASES_API);
    if (!res.ok) {
      applyDefaultLinks();
      return;
    }

    const data = await res.json();
    const tag = data.tag_name || "v0.1.0";
    relTag.textContent = `Release ${tag}`;

    if (data.published_at) {
      const date = new Date(data.published_at).toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
      relPublished.textContent = `Published on ${date}`;
    }

    if (data.html_url) {
      relLink.href = data.html_url;
    }

    // Map assets to matching buttons
    const assets = data.assets || [];
    let foundArm = false;
    let foundIntel = false;
    let foundExe = false;
    let foundMsi = false;

    assets.forEach((asset) => {
      const name = asset.name.toLowerCase();
      const url = asset.browser_download_url;

      if (name.endsWith(".dmg")) {
        if (name.includes("aarch64") || name.includes("arm64") || name.includes("silicon")) {
          linkMacArm.href = url;
          foundArm = true;
        } else {
          linkMacIntel.href = url;
          foundIntel = true;
        }
      } else if (name.endsWith(".exe")) {
        linkWinExe.href = url;
        foundExe = true;
      } else if (name.endsWith(".msi")) {
        linkWinMsi.href = url;
        foundMsi = true;
      }
    });

    // Fallbacks for any missing specific asset
    if (!foundArm) linkMacArm.href = `${LATEST_RELEASE_URL}/download/Connex_${tag}_aarch64.dmg`;
    if (!foundIntel) linkMacIntel.href = `${LATEST_RELEASE_URL}/download/Connex_${tag}_x64.dmg`;
    if (!foundExe) linkWinExe.href = `${LATEST_RELEASE_URL}/download/Connex_${tag}_x64-setup.exe`;
    if (!foundMsi) linkWinMsi.href = `${LATEST_RELEASE_URL}/download/Connex_${tag}_x64_en-US.msi`;

    // Also update hero primary download button URL
    const detected = detectOS();
    const ctaBtn = document.getElementById("btn-primary-dl");
    if (detected.os === "windows") {
      ctaBtn.href = linkWinExe.href;
    } else {
      ctaBtn.href = linkMacArm.href;
    }
  } catch (err) {
    applyDefaultLinks();
  }
}

function applyDefaultLinks() {
  document.getElementById("link-mac-arm").href = FALLBACK_DOWNLOADS.macSilicon;
  document.getElementById("link-mac-intel").href = FALLBACK_DOWNLOADS.macIntel;
  document.getElementById("link-win-exe").href = FALLBACK_DOWNLOADS.winExe;
  document.getElementById("link-win-msi").href = FALLBACK_DOWNLOADS.winMsi;
}
