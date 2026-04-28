use lazy_static::lazy_static;
use rdev::{listen, Event, EventType, Key};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputActivity {
    pub keyboard_events: u32,
    pub mouse_clicks: u32,
    pub mouse_moves: u32,
    pub inferred_text: String,
    pub recent_keys: Vec<String>,
    pub last_input_secs_ago: Option<u64>,
}

#[derive(Debug, Clone)]
enum InputEventKind {
    Key {
        key_name: String,
        text: Option<String>,
    },
    MouseClick,
    MouseMove,
}

#[derive(Debug, Clone)]
struct InputEventRecord {
    at: Instant,
    kind: InputEventKind,
}

const MAX_BUFFER_EVENTS: usize = 5000;

lazy_static! {
    static ref INPUT_EVENTS: Mutex<VecDeque<InputEventRecord>> = Mutex::new(VecDeque::new());
}

static START_LISTENER: Once = Once::new();

pub fn ensure_started() {
    START_LISTENER.call_once(|| {
        std::thread::spawn(|| {
            let callback = |event: Event| {
                let kind = match event.event_type {
                    EventType::KeyPress(key) => {
                        let (key_name, text) = normalize_key(key);
                        InputEventKind::Key { key_name, text }
                    }
                    EventType::ButtonPress(_button) => InputEventKind::MouseClick,
                    EventType::MouseMove { .. } => InputEventKind::MouseMove,
                    _ => return,
                };

                if let Ok(mut buffer) = INPUT_EVENTS.lock() {
                    while buffer.len() >= MAX_BUFFER_EVENTS {
                        buffer.pop_front();
                    }
                    buffer.push_back(InputEventRecord {
                        at: Instant::now(),
                        kind,
                    });
                }
            };

            if let Err(err) = listen(callback) {
                eprintln!("输入监听启动失败: {:?}", err);
            }
        });
    });
}

pub fn snapshot(window_secs: u64, max_keystrokes: usize, include_raw_keys: bool) -> InputActivity {
    let now = Instant::now();
    let window = Duration::from_secs(window_secs.max(1));
    let mut keyboard_events = 0u32;
    let mut mouse_clicks = 0u32;
    let mut mouse_moves = 0u32;
    let mut inferred_text_parts: Vec<String> = Vec::new();
    let mut recent_keys: Vec<String> = Vec::new();
    let mut last_input_secs_ago: Option<u64> = None;

    let records: Vec<InputEventRecord> = match INPUT_EVENTS.lock() {
        Ok(buffer) => buffer.iter().cloned().collect(),
        Err(_) => Vec::new(),
    };

    for record in records.iter().rev() {
        let elapsed = now.saturating_duration_since(record.at);
        if elapsed > window {
            break;
        }
        if last_input_secs_ago.is_none() {
            last_input_secs_ago = Some(elapsed.as_secs());
        }
        match &record.kind {
            InputEventKind::Key { key_name, text } => {
                keyboard_events += 1;
                if include_raw_keys && recent_keys.len() < max_keystrokes {
                    recent_keys.push(key_name.clone());
                }
                if let Some(t) = text {
                    inferred_text_parts.push(t.clone());
                }
            }
            InputEventKind::MouseClick => {
                mouse_clicks += 1;
            }
            InputEventKind::MouseMove => {
                mouse_moves += 1;
            }
        }
    }

    recent_keys.reverse();
    if recent_keys.len() > max_keystrokes {
        let len = recent_keys.len();
        recent_keys = recent_keys[len - max_keystrokes..].to_vec();
    }

    inferred_text_parts.reverse();
    let mut inferred_text = inferred_text_parts.concat();
    if inferred_text.chars().count() > max_keystrokes {
        inferred_text = inferred_text.chars().take(max_keystrokes).collect::<String>();
    }

    InputActivity {
        keyboard_events,
        mouse_clicks,
        mouse_moves,
        inferred_text,
        recent_keys,
        last_input_secs_ago,
    }
}

fn normalize_key(key: Key) -> (String, Option<String>) {
    let key_name = format!("{:?}", key);
    let text = match key {
        Key::KeyA => Some("a".to_string()),
        Key::KeyB => Some("b".to_string()),
        Key::KeyC => Some("c".to_string()),
        Key::KeyD => Some("d".to_string()),
        Key::KeyE => Some("e".to_string()),
        Key::KeyF => Some("f".to_string()),
        Key::KeyG => Some("g".to_string()),
        Key::KeyH => Some("h".to_string()),
        Key::KeyI => Some("i".to_string()),
        Key::KeyJ => Some("j".to_string()),
        Key::KeyK => Some("k".to_string()),
        Key::KeyL => Some("l".to_string()),
        Key::KeyM => Some("m".to_string()),
        Key::KeyN => Some("n".to_string()),
        Key::KeyO => Some("o".to_string()),
        Key::KeyP => Some("p".to_string()),
        Key::KeyQ => Some("q".to_string()),
        Key::KeyR => Some("r".to_string()),
        Key::KeyS => Some("s".to_string()),
        Key::KeyT => Some("t".to_string()),
        Key::KeyU => Some("u".to_string()),
        Key::KeyV => Some("v".to_string()),
        Key::KeyW => Some("w".to_string()),
        Key::KeyX => Some("x".to_string()),
        Key::KeyY => Some("y".to_string()),
        Key::KeyZ => Some("z".to_string()),
        Key::Num0 => Some("0".to_string()),
        Key::Num1 => Some("1".to_string()),
        Key::Num2 => Some("2".to_string()),
        Key::Num3 => Some("3".to_string()),
        Key::Num4 => Some("4".to_string()),
        Key::Num5 => Some("5".to_string()),
        Key::Num6 => Some("6".to_string()),
        Key::Num7 => Some("7".to_string()),
        Key::Num8 => Some("8".to_string()),
        Key::Num9 => Some("9".to_string()),
        Key::Space => Some(" ".to_string()),
        Key::Return => Some("\n".to_string()),
        Key::Tab => Some("\t".to_string()),
        Key::Minus => Some("-".to_string()),
        Key::Equal => Some("=".to_string()),
        Key::LeftBracket => Some("[".to_string()),
        Key::RightBracket => Some("]".to_string()),
        Key::SemiColon => Some(";".to_string()),
        Key::Quote => Some("'".to_string()),
        Key::BackSlash => Some("\\".to_string()),
        Key::Comma => Some(",".to_string()),
        Key::Dot => Some(".".to_string()),
        Key::Slash => Some("/".to_string()),
        Key::BackQuote => Some("`".to_string()),
        _ => None,
    };
    (key_name, text)
}
