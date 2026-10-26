// ============================================================
// core/stealth.rs — Hide app window from screen captures
// ============================================================
//
// macOS: NSWindowSharingNone (0)
//   Sets the sharing type of the NSWindow to None so that macOS WindowServer
//   and CoreGraphics / ScreenCaptureKit completely exclude the window from
//   all screen shares and recordings (Google Meet, Zoom, OBS, Teams, etc.).
//   The window remains fully visible and usable to the local user.
//
// Windows: SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)
//   Desktop Window Manager (DWM) completely excludes the window from
//   capture pipelines while keeping it interactive and visible locally.
// ============================================================

use anyhow::Result;
use log::info;

/// Enable or disable screen-capture exclusion for the Connex window.
pub fn set_stealth(enabled: bool) -> Result<()> {
    info!("[stealth] Screen-capture exclusion enabled={enabled}");

    #[cfg(target_os = "macos")]
    macos::set_enabled(enabled)?;

    #[cfg(target_os = "windows")]
    anyhow::bail!("A Connex window handle is required to change capture exclusion on Windows");

    Ok(())
}

/// Compatibility helper for non-Windows platform checks.
#[cfg(not(target_os = "windows"))]
pub fn apply_stealth() -> Result<()> {
    set_stealth(true)
}

// ─────────────────────────────────────────────────────────────
// macOS implementation
// ─────────────────────────────────────────────────────────────
#[cfg(target_os = "macos")]
mod macos {
    use anyhow::Result;
    use log::info;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};

    pub fn set_enabled(enabled: bool) -> Result<()> {
        unsafe {
            let ns_app_class = match AnyClass::get("NSApplication") {
                Some(cls) => cls,
                None => {
                    log::warn!("[stealth:macos] NSApplication class not found");
                    return Ok(());
                }
            };

            let app: *mut AnyObject = msg_send![ns_app_class, sharedApplication];
            if app.is_null() {
                return Ok(());
            }

            // Exclude every window of this app from screen capture:
            // NSWindowSharingNone = 0
            let windows: *mut AnyObject = msg_send![app, windows];
            if !windows.is_null() {
                let count: usize = msg_send![windows, count];
                for i in 0..count {
                    let win: *mut AnyObject = msg_send![windows, objectAtIndex: i];
                    if !win.is_null() {
                        let sharing_type = if enabled { 0usize } else { 2usize };
                        let _: () = msg_send![win, setSharingType: sharing_type];
                    }
                }
            }
        }

        info!("[stealth:macos] Capture sharing type updated: enabled={enabled}");
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────
// Windows implementation
// ─────────────────────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod windows {
    use anyhow::Result;
    use log::info;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };

    pub fn set_window(hwnd: HWND, enabled: bool) -> Result<()> {
        unsafe {
            let affinity = if enabled {
                WDA_EXCLUDEFROMCAPTURE
            } else {
                WDA_NONE
            };
            SetWindowDisplayAffinity(hwnd, affinity)?;
        }
        info!("[stealth:windows] Capture exclusion enabled={enabled}");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn set_stealth_for_window(
    hwnd: windows::Win32::Foundation::HWND,
    enabled: bool,
) -> Result<()> {
    windows::set_window(hwnd, enabled)
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;

    #[test]
    fn test_apply_stealth_does_not_panic() {
        // In a headless CLI test environment, applying window stealth should safely complete
        let res = apply_stealth();
        assert!(res.is_ok(), "apply_stealth should not return error or panic");
    }

    #[test]
    fn test_apply_stealth_idempotent() {
        // Calling apply_stealth repeatedly must be safe and idempotent
        for _ in 0..5 {
            let res = apply_stealth();
            assert!(res.is_ok(), "Repeated apply_stealth calls must succeed");
        }
    }

    #[test]
    fn test_apply_stealth_multithreaded() {
        // Calling apply_stealth concurrently across multiple threads should be thread-safe
        let mut handles = Vec::new();
        for _ in 0..4 {
            handles.push(std::thread::spawn(|| {
                apply_stealth()
            }));
        }

        for handle in handles {
            let res = handle.join().expect("Thread should not panic");
            assert!(res.is_ok(), "Concurrent apply_stealth should succeed");
        }
    }
}
