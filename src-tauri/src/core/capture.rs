// ============================================================
// core/capture.rs  — Cross-platform screen capture
// ============================================================
// Uses `scrap` crate: zero-copy DXGI (Windows) / CGDisplayStream (macOS)
// Captures at ~30 fps and encodes frames to JPEG for WebRTC data channel
// ============================================================

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

use log::{error, info};
use scrap::{Capturer, Display};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub const TARGET_FPS: u64 = 30;
const JPEG_QUALITY: u8 = 75; // balance quality vs bandwidth

/// A screen frame encoded as base64 JPEG
pub type FrameB64 = String;

/// Start capturing the primary display.
/// Returns a receiver channel for frames and a stop signal.
pub fn start_capture(
    stop_signal: Arc<AtomicBool>,
) -> Result<mpsc::Receiver<FrameB64>> {
    let (tx, rx) = mpsc::channel::<FrameB64>(4); // small buffer to avoid lag

    // Get primary display
    let display = Display::primary().context("No primary display found")?;
    let width = display.width();
    let height = display.height();
    info!("[capture] Display {}×{} @ {}fps", width, height, TARGET_FPS);

    let capturer = Capturer::new(display).context("Failed to create capturer")?;
    let frame_interval = Duration::from_millis(1000 / TARGET_FPS);

    // SAFETY: Capturer uses raw pointers internally (CGDisplayStream / DXGI);
    // it is only accessed from this one spawned thread.
    struct SendCapturer(Capturer);
    unsafe impl Send for SendCapturer {}

    let capturer = SendCapturer(capturer);

    std::thread::spawn(move || {
        let mut capturer = capturer;
        let capturer = &mut capturer.0;
        let mut last_frame = Instant::now();

        loop {
            if stop_signal.load(Ordering::Relaxed) {
                info!("[capture] Stop signal received, exiting capture loop");
                break;
            }

            // Throttle to target FPS
            let elapsed = last_frame.elapsed();
            if elapsed < frame_interval {
                std::thread::sleep(frame_interval - elapsed);
            }
            last_frame = Instant::now();

            match capturer.frame() {
                Ok(frame) => {
                    // Convert BGRA → RGBA
                    let mut rgba = Vec::with_capacity(frame.len());
                    for chunk in frame.chunks_exact(4) {
                        rgba.push(chunk[2]); // R (was B)
                        rgba.push(chunk[1]); // G
                        rgba.push(chunk[0]); // B (was R)
                        rgba.push(chunk[3]); // A
                    }

                    // Encode to JPEG
                    if let Some(b64) = encode_jpeg(&rgba, width as u32, height as u32) {
                        if tx.try_send(b64).is_err() {
                            // Receiver full or dropped — skip frame (no lag accumulation)
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Frame not ready yet, try again
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => {
                    error!("[capture] Frame capture error: {}", e);
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    });

    Ok(rx)
}

/// Encode raw RGBA bytes to base64-encoded JPEG string
fn encode_jpeg(rgba: &[u8], width: u32, height: u32) -> Option<FrameB64> {
    let mut buf = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    encoder
        .encode(rgba, width, height, image::ExtendedColorType::Rgba8)
        .ok()?;

    Some(B64.encode(buf.into_inner()))
}

/// Returns (width, height) of the primary display
pub fn display_size() -> Result<(usize, usize)> {
    let display = Display::primary().context("No primary display")?;
    Ok((display.width(), display.height()))
}
