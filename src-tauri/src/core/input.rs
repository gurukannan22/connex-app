// ============================================================
// core/input.rs  — Cross-platform input injection
// ============================================================
// Receives mouse/keyboard events from the Controller and
// injects them into the Host OS using `enigo` crate.
//
// macOS: CGEvent (via enigo's Quartz backend)
// Windows: SendInput Win32 API (via enigo's Windows backend)
// ============================================================

use anyhow::Result;
use enigo::{
    Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};
use log::error;
use serde::{Deserialize, Serialize};

/// Input events sent from Controller → Host over data channel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    /// Mouse moved to (x, y) — normalized 0.0–1.0 relative to screen size
    MouseMove { x: f64, y: f64 },
    /// Mouse button pressed or released
    MouseButton {
        button: MouseButton,
        pressed: bool,
    },
    /// Mouse wheel scroll
    MouseScroll { delta_x: i32, delta_y: i32 },
    /// Key pressed or released
    KeyPress { key: String, pressed: bool },
    /// Text typed (for clipboard paste / fast text)
    TypeText { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Inject an input event into the host OS
pub fn inject(event: InputEvent, screen_w: u32, screen_h: u32) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())?;

    match event {
        InputEvent::MouseMove { x, y } => {
            // Convert normalized coordinates to absolute pixels
            let abs_x = (x * screen_w as f64) as i32;
            let abs_y = (y * screen_h as f64) as i32;
            enigo.move_mouse(abs_x, abs_y, Coordinate::Abs)?;
        }

        InputEvent::MouseButton { button, pressed } => {
            let btn = match button {
                MouseButton::Left => Button::Left,
                MouseButton::Right => Button::Right,
                MouseButton::Middle => Button::Middle,
            };
            let dir = if pressed {
                Direction::Press
            } else {
                Direction::Release
            };
            enigo.button(btn, dir)?;
        }

        InputEvent::MouseScroll { delta_x, delta_y } => {
            if delta_y != 0 {
                enigo.scroll(delta_y, enigo::Axis::Vertical)?;
            }
            if delta_x != 0 {
                enigo.scroll(delta_x, enigo::Axis::Horizontal)?;
            }
        }

        InputEvent::KeyPress { key, pressed } => {
            let k = parse_key(&key);
            let dir = if pressed {
                Direction::Press
            } else {
                Direction::Release
            };
            if let Some(k) = k {
                enigo.key(k, dir)?;
            } else {
                error!("[input] Unknown key: {}", key);
            }
        }

        InputEvent::TypeText { text } => {
            enigo.text(&text)?;
        }
    }

    Ok(())
}

/// Map JS key names → enigo Key enum
fn parse_key(key: &str) -> Option<Key> {
    Some(match key {
        "Enter" => Key::Return,
        "Backspace" => Key::Backspace,
        "Tab" => Key::Tab,
        "Escape" | "Esc" => Key::Escape,
        "Delete" => Key::Delete,
        "ArrowUp" => Key::UpArrow,
        "ArrowDown" => Key::DownArrow,
        "ArrowLeft" => Key::LeftArrow,
        "ArrowRight" => Key::RightArrow,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "Control" => Key::Control,
        "Alt" => Key::Alt,
        "Shift" => Key::Shift,
        "Meta" | "Command" | "Win" => Key::Meta,
        "CapsLock" => Key::CapsLock,
        "Space" | " " => Key::Space,
        s if s.len() == 1 => Key::Unicode(s.chars().next()?),
        _ => return None,
    })
}
