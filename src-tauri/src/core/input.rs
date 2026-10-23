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
        s if s.chars().count() == 1 => Key::Unicode(s.chars().next()?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_common_keys() {
        assert_eq!(parse_key("Enter"), Some(Key::Return));
        assert_eq!(parse_key("Backspace"), Some(Key::Backspace));
        assert_eq!(parse_key("Tab"), Some(Key::Tab));
        assert_eq!(parse_key("Escape"), Some(Key::Escape));
        assert_eq!(parse_key("Esc"), Some(Key::Escape));
        assert_eq!(parse_key("Delete"), Some(Key::Delete));
        assert_eq!(parse_key("ArrowUp"), Some(Key::UpArrow));
        assert_eq!(parse_key("ArrowDown"), Some(Key::DownArrow));
        assert_eq!(parse_key("ArrowLeft"), Some(Key::LeftArrow));
        assert_eq!(parse_key("ArrowRight"), Some(Key::RightArrow));
        assert_eq!(parse_key("Space"), Some(Key::Space));
        assert_eq!(parse_key(" "), Some(Key::Space));
    }

    #[test]
    fn test_parse_modifiers_and_function_keys() {
        assert_eq!(parse_key("Control"), Some(Key::Control));
        assert_eq!(parse_key("Alt"), Some(Key::Alt));
        assert_eq!(parse_key("Shift"), Some(Key::Shift));
        assert_eq!(parse_key("Meta"), Some(Key::Meta));
        assert_eq!(parse_key("Command"), Some(Key::Meta));
        assert_eq!(parse_key("F1"), Some(Key::F1));
        assert_eq!(parse_key("F12"), Some(Key::F12));
    }

    #[test]
    fn test_parse_single_characters() {
        assert_eq!(parse_key("a"), Some(Key::Unicode('a')));
        assert_eq!(parse_key("Z"), Some(Key::Unicode('Z')));
        assert_eq!(parse_key("9"), Some(Key::Unicode('9')));
        assert_eq!(parse_key("@"), Some(Key::Unicode('@')));
    }

    #[test]
    fn test_parse_invalid_key() {
        assert_eq!(parse_key("SomeVeryLongUnrecognizedKeyName"), None);
        assert_eq!(parse_key(""), None);
    }

    #[test]
    fn test_input_event_serialization() {
        let mouse_move = InputEvent::MouseMove { x: 0.45, y: 0.82 };
        let json = serde_json::to_string(&mouse_move).unwrap();
        assert!(json.contains(r#""type":"mouse_move""#));

        let deserialized: InputEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            InputEvent::MouseMove { x, y } => {
                assert!((x - 0.45).abs() < f64::EPSILON);
                assert!((y - 0.82).abs() < f64::EPSILON);
            }
            _ => panic!("Expected MouseMove variant"),
        }
    }

    #[test]
    fn test_parse_all_function_keys() {
        assert_eq!(parse_key("F1"), Some(Key::F1));
        assert_eq!(parse_key("F2"), Some(Key::F2));
        assert_eq!(parse_key("F3"), Some(Key::F3));
        assert_eq!(parse_key("F4"), Some(Key::F4));
        assert_eq!(parse_key("F5"), Some(Key::F5));
        assert_eq!(parse_key("F6"), Some(Key::F6));
        assert_eq!(parse_key("F7"), Some(Key::F7));
        assert_eq!(parse_key("F8"), Some(Key::F8));
        assert_eq!(parse_key("F9"), Some(Key::F9));
        assert_eq!(parse_key("F10"), Some(Key::F10));
        assert_eq!(parse_key("F11"), Some(Key::F11));
        assert_eq!(parse_key("F12"), Some(Key::F12));
    }

    #[test]
    fn test_parse_navigation_and_control_keys() {
        assert_eq!(parse_key("Home"), Some(Key::Home));
        assert_eq!(parse_key("End"), Some(Key::End));
        assert_eq!(parse_key("PageUp"), Some(Key::PageUp));
        assert_eq!(parse_key("PageDown"), Some(Key::PageDown));
        assert_eq!(parse_key("CapsLock"), Some(Key::CapsLock));
        assert_eq!(parse_key("Win"), Some(Key::Meta));
    }

    #[test]
    fn test_parse_unicode_characters() {
        assert_eq!(parse_key("€"), Some(Key::Unicode('€')));
        assert_eq!(parse_key("ñ"), Some(Key::Unicode('ñ')));
        assert_eq!(parse_key("π"), Some(Key::Unicode('π')));
    }

    #[test]
    fn test_all_input_event_variants_serde() {
        // MouseButton
        let btn = InputEvent::MouseButton {
            button: MouseButton::Right,
            pressed: true,
        };
        let json = serde_json::to_string(&btn).unwrap();
        assert!(json.contains(r#""type":"mouse_button""#));
        assert!(json.contains(r#""button":"right""#));
        let de: InputEvent = serde_json::from_str(&json).unwrap();
        match de {
            InputEvent::MouseButton { button, pressed } => {
                assert!(matches!(button, MouseButton::Right));
                assert!(pressed);
            }
            _ => panic!("Expected MouseButton"),
        }

        // MouseScroll
        let scroll = InputEvent::MouseScroll {
            delta_x: -120,
            delta_y: 240,
        };
        let json = serde_json::to_string(&scroll).unwrap();
        assert!(json.contains(r#""type":"mouse_scroll""#));
        let de: InputEvent = serde_json::from_str(&json).unwrap();
        match de {
            InputEvent::MouseScroll { delta_x, delta_y } => {
                assert_eq!(delta_x, -120);
                assert_eq!(delta_y, 240);
            }
            _ => panic!("Expected MouseScroll"),
        }

        // KeyPress
        let key_press = InputEvent::KeyPress {
            key: "Escape".to_string(),
            pressed: false,
        };
        let json = serde_json::to_string(&key_press).unwrap();
        assert!(json.contains(r#""type":"key_press""#));
        let de: InputEvent = serde_json::from_str(&json).unwrap();
        match de {
            InputEvent::KeyPress { key, pressed } => {
                assert_eq!(key, "Escape");
                assert!(!pressed);
            }
            _ => panic!("Expected KeyPress"),
        }

        // TypeText
        let type_text = InputEvent::TypeText {
            text: "Hello, Connex P2P!".to_string(),
        };
        let json = serde_json::to_string(&type_text).unwrap();
        assert!(json.contains(r#""type":"type_text""#));
        let de: InputEvent = serde_json::from_str(&json).unwrap();
        match de {
            InputEvent::TypeText { text } => {
                assert_eq!(text, "Hello, Connex P2P!");
            }
            _ => panic!("Expected TypeText"),
        }
    }
}
