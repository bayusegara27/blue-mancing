//! Keybind management for the fishing bot

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use global_hotkey::hotkey::Code;

use crate::utils::path::get_data_dir;

/// Default key bindings
pub static DEFAULT_KEYS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("start_key".to_string(), "F9".to_string());
    m.insert("stop_key".to_string(), "F10".to_string());
    m.insert("rods_key".to_string(), "M".to_string());
    m.insert("bait_key".to_string(), "N".to_string());
    m.insert("fish_key".to_string(), "F".to_string());
    m.insert("esc_key".to_string(), "ESC".to_string());
    m.insert("left_key".to_string(), "A".to_string());
    m.insert("right_key".to_string(), "D".to_string());
    m
});

/// Global configuration state
static CONFIG: Lazy<Arc<RwLock<HashMap<String, String>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(load_config_from_file()))
});

/// Configuration file path
fn get_config_path() -> std::path::PathBuf {
    get_data_dir().join("config").join("settings.json")
}

/// Load configuration from file
fn load_config_from_file() -> HashMap<String, String> {
    let config_path = get_config_path();
    
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(user_settings) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&content) {
            let mut config = DEFAULT_KEYS.clone();
            for (key, value) in user_settings {
                if let Some(s) = value.as_str() {
                    config.insert(key, s.to_string());
                }
            }
            return config;
        }
    }
    
    DEFAULT_KEYS.clone()
}

/// Load config safely
pub fn load_config() -> HashMap<String, String> {
    CONFIG.read().unwrap().clone()
}

/// Save config to file
pub fn save_config(config: &HashMap<String, String>) {
    let config_path = get_config_path();
    
    if let Some(parent) = config_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = fs::write(&config_path, content);
    }
    
    // Update global state
    let mut global_config = CONFIG.write().unwrap();
    *global_config = config.clone();
}

/// Convert a key object to string representation
pub fn key_to_str(key: &str) -> String {
    key.to_uppercase()
}

/// Resolve a key name string to a validated key string
pub fn resolve_key(key_name: &str) -> Option<String> {
    if key_name.is_empty() {
        return None;
    }
    
    let key_upper = key_name.trim().to_uppercase();
    
    // Valid special keys
    let special_keys = [
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
        "ESC", "ESCAPE", "ENTER", "RETURN", "SPACE", "TAB", "BACKSPACE",
        "UP", "DOWN", "LEFT", "RIGHT",
        "HOME", "END", "PAGEUP", "PAGEDOWN", "INSERT", "DELETE",
        "SHIFT", "CTRL", "CONTROL", "ALT", "WIN", "WINDOWS",
        "CAPSLOCK", "NUMLOCK", "SCROLLLOCK",
        "PRINT", "PRINTSCREEN", "PAUSE",
    ];
    
    if special_keys.contains(&key_upper.as_str()) {
        return Some(key_upper);
    }
    
    // Single character keys (letters and digits)
    if key_upper.len() == 1 {
        let c = key_upper.chars().next().unwrap();
        if c.is_alphanumeric() {
            return Some(key_upper);
        }
    }
    
    None
}

/// Get start and stop keys
pub fn get_keys() -> (String, String) {
    let config = load_config();
    let start = config.get("start_key").cloned().unwrap_or_else(|| "F9".to_string());
    let stop = config.get("stop_key").cloned().unwrap_or_else(|| "F10".to_string());
    (start, stop)
}

/// Set start and stop keys
pub fn set_keys(start_key: &str, stop_key: &str) -> Result<(), String> {
    if resolve_key(start_key).is_none() {
        return Err(format!("Invalid start key: {}", start_key));
    }
    if resolve_key(stop_key).is_none() {
        return Err(format!("Invalid stop key: {}", stop_key));
    }
    
    let mut config = load_config();
    config.insert("start_key".to_string(), start_key.to_uppercase());
    config.insert("stop_key".to_string(), stop_key.to_uppercase());
    save_config(&config);
    
    Ok(())
}

/// Get any key from config by name
pub fn get_key(name: &str) -> Option<String> {
    let config = load_config();
    config.get(name).cloned().or_else(|| DEFAULT_KEYS.get(name).cloned())
}

/// Get key resolved for pynput-like usage
pub fn get_pykey(name: &str) -> Option<String> {
    get_key(name).and_then(|k| resolve_key(&k))
}

/// Set any key in config
pub fn set_key(name: &str, key_value: &str) -> Result<(), String> {
    if !DEFAULT_KEYS.contains_key(name) {
        return Err(format!("Invalid setting name: {}", name));
    }
    
    let key_str = key_to_str(key_value);
    let mut config = load_config();
    config.insert(name.to_string(), key_str);
    save_config(&config);
    
    Ok(())
}

/// Convert key string to global_hotkey Code
pub fn string_to_code(key: &str) -> Option<Code> {
    let key_upper = key.to_uppercase();
    match key_upper.as_str() {
        "A" => Some(Code::KeyA),
        "B" => Some(Code::KeyB),
        "C" => Some(Code::KeyC),
        "D" => Some(Code::KeyD),
        "E" => Some(Code::KeyE),
        "F" => Some(Code::KeyF),
        "G" => Some(Code::KeyG),
        "H" => Some(Code::KeyH),
        "I" => Some(Code::KeyI),
        "J" => Some(Code::KeyJ),
        "K" => Some(Code::KeyK),
        "L" => Some(Code::KeyL),
        "M" => Some(Code::KeyM),
        "N" => Some(Code::KeyN),
        "O" => Some(Code::KeyO),
        "P" => Some(Code::KeyP),
        "Q" => Some(Code::KeyQ),
        "R" => Some(Code::KeyR),
        "S" => Some(Code::KeyS),
        "T" => Some(Code::KeyT),
        "U" => Some(Code::KeyU),
        "V" => Some(Code::KeyV),
        "W" => Some(Code::KeyW),
        "X" => Some(Code::KeyX),
        "Y" => Some(Code::KeyY),
        "Z" => Some(Code::KeyZ),
        "0" => Some(Code::Digit0),
        "1" => Some(Code::Digit1),
        "2" => Some(Code::Digit2),
        "3" => Some(Code::Digit3),
        "4" => Some(Code::Digit4),
        "5" => Some(Code::Digit5),
        "6" => Some(Code::Digit6),
        "7" => Some(Code::Digit7),
        "8" => Some(Code::Digit8),
        "9" => Some(Code::Digit9),
        "F1" => Some(Code::F1),
        "F2" => Some(Code::F2),
        "F3" => Some(Code::F3),
        "F4" => Some(Code::F4),
        "F5" => Some(Code::F5),
        "F6" => Some(Code::F6),
        "F7" => Some(Code::F7),
        "F8" => Some(Code::F8),
        "F9" => Some(Code::F9),
        "F10" => Some(Code::F10),
        "F11" => Some(Code::F11),
        "F12" => Some(Code::F12),
        "ESC" | "ESCAPE" => Some(Code::Escape),
        "ENTER" | "RETURN" => Some(Code::Enter),
        "SPACE" => Some(Code::Space),
        "TAB" => Some(Code::Tab),
        "BACKSPACE" => Some(Code::Backspace),
        "UP" => Some(Code::ArrowUp),
        "DOWN" => Some(Code::ArrowDown),
        "LEFT" => Some(Code::ArrowLeft),
        "RIGHT" => Some(Code::ArrowRight),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_key() {
        assert_eq!(resolve_key("F9"), Some("F9".to_string()));
        assert_eq!(resolve_key("a"), Some("A".to_string()));
        assert_eq!(resolve_key("ESC"), Some("ESC".to_string()));
        assert_eq!(resolve_key(""), None);
    }

    #[test]
    fn test_get_keys() {
        let (start, stop) = get_keys();
        assert!(!start.is_empty());
        assert!(!stop.is_empty());
    }
}
