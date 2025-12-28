//! Input simulation module for mouse and keyboard control

#[cfg(windows)]
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
#[cfg(windows)]
use parking_lot::Mutex;
#[cfg(windows)]
use once_cell::sync::Lazy;
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
/// Global mouse controller
static MOUSE: Lazy<Mutex<Enigo>> = Lazy::new(|| {
    Mutex::new(Enigo::new(&Settings::default()).expect("Failed to create Enigo for mouse"))
});

#[cfg(windows)]
/// Global keyboard controller
static KEYBOARD: Lazy<Mutex<Enigo>> = Lazy::new(|| {
    Mutex::new(Enigo::new(&Settings::default()).expect("Failed to create Enigo for keyboard"))
});

/// Click at a position
#[cfg(windows)]
pub fn click(x: i32, y: i32) {
    tracing::debug!("[INPUT] click() - moving mouse to ({}, {}) and clicking", x, y);
    thread::sleep(Duration::from_millis(50));
    let mut mouse = MOUSE.lock();
    if let Err(e) = mouse.move_mouse(x, y, Coordinate::Abs) {
        tracing::warn!("[INPUT] Failed to move mouse to ({}, {}): {:?}", x, y, e);
    } else {
        tracing::trace!("[INPUT] Mouse moved to ({}, {})", x, y);
    }
    if let Err(e) = mouse.button(Button::Left, Direction::Click) {
        tracing::warn!("[INPUT] Failed to click mouse: {:?}", e);
    } else {
        tracing::trace!("[INPUT] Mouse left click performed");
    }
}

#[cfg(not(windows))]
pub fn click(_x: i32, _y: i32) {
    tracing::warn!("Click not implemented on this platform");
}

/// Press and release a key
#[cfg(windows)]
pub fn press_key(key: &str) {
    tracing::debug!("[INPUT] press_key('{}') - pressing and releasing key", key);
    thread::sleep(Duration::from_millis(50));
    let mut keyboard = KEYBOARD.lock();
    
    if let Some(enigo_key) = string_to_enigo_key(key) {
        if let Err(e) = keyboard.key(enigo_key, Direction::Click) {
            tracing::warn!("[INPUT] Failed to press key '{}': {:?}", key, e);
        } else {
            tracing::trace!("[INPUT] Key '{}' pressed and released", key);
        }
    } else {
        tracing::warn!("[INPUT] Unknown key '{}' - cannot convert to enigo key", key);
    }
}

#[cfg(not(windows))]
pub fn press_key(_key: &str) {
    tracing::warn!("press_key not implemented on this platform");
}

/// Hold a key down
#[cfg(windows)]
pub fn hold_key(key: &str) {
    tracing::trace!("[INPUT] hold_key('{}') - holding key down", key);
    let mut keyboard = KEYBOARD.lock();
    
    if let Some(enigo_key) = string_to_enigo_key(key) {
        if let Err(e) = keyboard.key(enigo_key, Direction::Press) {
            tracing::warn!("[INPUT] Failed to hold key '{}': {:?}", key, e);
        }
    } else {
        tracing::warn!("[INPUT] Unknown key '{}' - cannot hold", key);
    }
}

#[cfg(not(windows))]
pub fn hold_key(_key: &str) {
    tracing::warn!("hold_key not implemented on this platform");
}

/// Release a held key
#[cfg(windows)]
pub fn release_key(key: &str) {
    tracing::trace!("[INPUT] release_key('{}') - releasing held key", key);
    let mut keyboard = KEYBOARD.lock();
    
    if let Some(enigo_key) = string_to_enigo_key(key) {
        if let Err(e) = keyboard.key(enigo_key, Direction::Release) {
            tracing::warn!("[INPUT] Failed to release key '{}': {:?}", key, e);
        }
    } else {
        tracing::warn!("[INPUT] Unknown key '{}' - cannot release", key);
    }
}

#[cfg(not(windows))]
pub fn release_key(_key: &str) {
    tracing::warn!("release_key not implemented on this platform");
}

/// Press left mouse button down
#[cfg(windows)]
pub fn mouse_press() {
    tracing::debug!("[INPUT] mouse_press() - pressing left mouse button");
    let mut mouse = MOUSE.lock();
    if let Err(e) = mouse.button(Button::Left, Direction::Press) {
        tracing::warn!("[INPUT] Failed to press mouse button: {:?}", e);
    } else {
        tracing::trace!("[INPUT] Mouse button pressed");
    }
}

#[cfg(not(windows))]
pub fn mouse_press() {
    tracing::warn!("mouse_press not implemented on this platform");
}

/// Release left mouse button
#[cfg(windows)]
pub fn mouse_release() {
    tracing::debug!("[INPUT] mouse_release() - releasing left mouse button");
    let mut mouse = MOUSE.lock();
    if let Err(e) = mouse.button(Button::Left, Direction::Release) {
        tracing::warn!("[INPUT] Failed to release mouse button: {:?}", e);
    } else {
        tracing::trace!("[INPUT] Mouse button released");
    }
}

#[cfg(not(windows))]
pub fn mouse_release() {
    tracing::warn!("mouse_release not implemented on this platform");
}

/// Move mouse to position
#[cfg(windows)]
pub fn mouse_move(x: i32, y: i32) {
    tracing::debug!("[INPUT] mouse_move({}, {}) - moving mouse to position", x, y);
    let mut mouse = MOUSE.lock();
    if let Err(e) = mouse.move_mouse(x, y, Coordinate::Abs) {
        tracing::warn!("[INPUT] Failed to move mouse to ({}, {}): {:?}", x, y, e);
    } else {
        tracing::trace!("[INPUT] Mouse moved to ({}, {})", x, y);
    }
}

#[cfg(not(windows))]
pub fn mouse_move(_x: i32, _y: i32) {
    tracing::warn!("mouse_move not implemented on this platform");
}

/// Convert string key name to enigo Key
#[cfg(windows)]
fn string_to_enigo_key(key: &str) -> Option<Key> {
    // Handle single characters - use lowercase to avoid keyboard layout mapping issues
    if key.len() == 1 {
        let c = key.chars().next()?.to_ascii_lowercase();
        return Some(Key::Unicode(c));
    }
    
    // Handle special keys
    let key_upper = key.to_uppercase();
    match key_upper.as_str() {
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "ESC" | "ESCAPE" => Some(Key::Escape),
        "ENTER" | "RETURN" => Some(Key::Return),
        "SPACE" => Some(Key::Space),
        "TAB" => Some(Key::Tab),
        "BACKSPACE" => Some(Key::Backspace),
        "UP" => Some(Key::UpArrow),
        "DOWN" => Some(Key::DownArrow),
        "LEFT" => Some(Key::LeftArrow),
        "RIGHT" => Some(Key::RightArrow),
        "HOME" => Some(Key::Home),
        "END" => Some(Key::End),
        "PAGEUP" => Some(Key::PageUp),
        "PAGEDOWN" => Some(Key::PageDown),
        "DELETE" => Some(Key::Delete),
        "SHIFT" => Some(Key::Shift),
        "CTRL" | "CONTROL" => Some(Key::Control),
        "ALT" => Some(Key::Alt),
        "CAPSLOCK" => Some(Key::CapsLock),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_string_to_enigo_key() {
        assert!(string_to_enigo_key("A").is_some());
        assert!(string_to_enigo_key("F9").is_some());
        assert!(string_to_enigo_key("ESC").is_some());
        assert!(string_to_enigo_key("INVALID_KEY_NAME_THAT_DOES_NOT_EXIST").is_none());
    }
}
