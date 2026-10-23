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

        let mut rgb = Vec::new();

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
                    // Convert BGRA (4 bytes) → RGB (3 bytes) in-place (saves 25% bandwidth)
                    let expected_rgb_len = (frame.len() / 4) * 3;
                    if rgb.len() != expected_rgb_len {
                        rgb.resize(expected_rgb_len, 0);
                    }
                    for (dst, src) in rgb.chunks_exact_mut(3).zip(frame.chunks_exact(4)) {
                        dst[0] = src[2]; // R (was B)
                        dst[1] = src[1]; // G
                        dst[2] = src[0]; // B (was R)
                    }

                    // Encode to JPEG
                    if let Some(b64) = encode_jpeg(&rgb, width as u32, height as u32) {
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

/// Encode raw RGB bytes to base64-encoded JPEG string
fn encode_jpeg(rgb: &[u8], width: u32, height: u32) -> Option<FrameB64> {
    if width == 0 || height == 0 || rgb.len() != (width as usize * height as usize * 3) {
        return None;
    }
    let mut buf = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    encoder
        .encode(rgb, width, height, image::ExtendedColorType::Rgb8)
        .ok()?;

    Some(B64.encode(buf.into_inner()))
}

/// Returns (width, height) of the primary display
pub fn display_size() -> Result<(usize, usize)> {
    let display = Display::primary().context("No primary display")?;
    Ok((display.width(), display.height()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_jpeg_valid_rgb() {
        // Create synthetic 2x2 RGB image (3 bytes per pixel)
        let width = 2;
        let height = 2;
        let rgb = vec![
            255, 0, 0,    // Red
            0, 255, 0,    // Green
            0, 0, 255,    // Blue
            255, 255, 255 // White
        ];

        let encoded = encode_jpeg(&rgb, width, height);
        assert!(encoded.is_some(), "JPEG encoding of valid RGB must succeed");
        let b64 = encoded.unwrap();
        assert!(!b64.is_empty());

        // Verify it can be decoded back to an image
        let bytes = B64.decode(&b64).expect("Base64 decode must succeed");
        let img = image::load_from_memory(&bytes).expect("Decoded bytes must form valid image");
        assert_eq!(img.width(), width);
        assert_eq!(img.height(), height);
    }

    #[test]
    fn test_encode_jpeg_invalid_dimensions() {
        let rgb = vec![255, 0, 0];
        // 0 dimensions should safely return None without panicking
        assert!(encode_jpeg(&rgb, 0, 1).is_none());
        assert!(encode_jpeg(&rgb, 1, 0).is_none());
        // Dimension mismatch should safely return None
        assert!(encode_jpeg(&rgb, 2, 2).is_none());
    }

    #[test]
    fn test_encode_jpeg_magic_bytes() {
        let rgb = vec![128, 128, 128]; // 1x1 pixel
        let encoded = encode_jpeg(&rgb, 1, 1).expect("1x1 encoding must succeed");
        let bytes = B64.decode(&encoded).expect("Decode base64");

        // JPEG standard Start of Image (SOI) is 0xFF, 0xD8
        assert!(bytes.len() >= 4);
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
        // End of Image (EOI) is 0xFF, 0xD9
        let len = bytes.len();
        assert_eq!(bytes[len - 2], 0xFF);
        assert_eq!(bytes[len - 1], 0xD9);
    }

    #[test]
    fn test_encode_jpeg_various_dimensions() {
        // Odd dimensions: 3 width x 5 height = 15 pixels = 45 bytes
        let rgb = vec![100u8; 3 * 5 * 3];
        let encoded = encode_jpeg(&rgb, 3, 5).expect("Odd dimension encoding must succeed");
        let bytes = B64.decode(&encoded).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!(img.width(), 3);
        assert_eq!(img.height(), 5);
    }

    #[test]
    fn test_encode_jpeg_buffer_off_by_one() {
        let width = 4;
        let height = 4;
        let expected_len = (width * height * 3) as usize; // 48 bytes

        // One byte short
        let too_short = vec![0u8; expected_len - 1];
        assert!(encode_jpeg(&too_short, width, height).is_none());

        // One byte extra
        let too_long = vec![0u8; expected_len + 1];
        assert!(encode_jpeg(&too_long, width, height).is_none());
    }
}
