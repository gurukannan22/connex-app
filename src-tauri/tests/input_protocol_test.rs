// ============================================================
// tests/input_protocol_test.rs — Integration tests for Input wire protocol
// ============================================================

use connex_lib::core::input::{InputEvent, MouseButton};

#[test]
fn test_input_wire_protocol_roundtrips() {
    let events = vec![
        InputEvent::MouseMove { x: 0.0, y: 0.0 },
        InputEvent::MouseMove { x: 1.0, y: 1.0 },
        InputEvent::MouseMove { x: 0.5432, y: 0.9876 },
        InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        },
        InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: false,
        },
        InputEvent::MouseButton {
            button: MouseButton::Right,
            pressed: true,
        },
        InputEvent::MouseButton {
            button: MouseButton::Middle,
            pressed: false,
        },
        InputEvent::MouseScroll {
            delta_x: 0,
            delta_y: -100,
        },
        InputEvent::MouseScroll {
            delta_x: 50,
            delta_y: 50,
        },
        InputEvent::KeyPress {
            key: "Enter".to_string(),
            pressed: true,
        },
        InputEvent::KeyPress {
            key: "Control".to_string(),
            pressed: false,
        },
        InputEvent::TypeText {
            text: "The quick brown fox jumps over the lazy dog 1234567890!@#$%^&*()".to_string(),
        },
    ];

    for original in &events {
        let serialized = serde_json::to_string(original).expect("Serialization must succeed");
        assert!(!serialized.is_empty());

        let deserialized: InputEvent = serde_json::from_str(&serialized).expect("Deserialization must succeed");

        // Verify variant match
        match (original, &deserialized) {
            (InputEvent::MouseMove { x: x1, y: y1 }, InputEvent::MouseMove { x: x2, y: y2 }) => {
                assert!((x1 - x2).abs() < f64::EPSILON);
                assert!((y1 - y2).abs() < f64::EPSILON);
            }
            (
                InputEvent::MouseButton { button: b1, pressed: p1 },
                InputEvent::MouseButton { button: b2, pressed: p2 },
            ) => {
                assert_eq!(format!("{:?}", b1), format!("{:?}", b2));
                assert_eq!(p1, p2);
            }
            (
                InputEvent::MouseScroll { delta_x: dx1, delta_y: dy1 },
                InputEvent::MouseScroll { delta_x: dx2, delta_y: dy2 },
            ) => {
                assert_eq!(dx1, dx2);
                assert_eq!(dy1, dy2);
            }
            (
                InputEvent::KeyPress { key: k1, pressed: p1 },
                InputEvent::KeyPress { key: k2, pressed: p2 },
            ) => {
                assert_eq!(k1, k2);
                assert_eq!(p1, p2);
            }
            (InputEvent::TypeText { text: t1 }, InputEvent::TypeText { text: t2 }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Variant mismatch between original and deserialized event"),
        }
    }
}
