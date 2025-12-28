//! Window management using Windows API

#![allow(dead_code)]

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{HWND, RECT};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowRect, SetForegroundWindow, ShowWindow, SW_SHOW,
};

/// Window title for Blue Protocol
const TARGET_TITLE: &str = "Blue Protocol: Star Resonance";

/// Find the Blue Protocol window
#[cfg(windows)]
pub fn find_blue_protocol_window() -> Option<HWND> {
    tracing::trace!(
        "[WINDOW] find_blue_protocol_window() - searching for '{}'",
        TARGET_TITLE
    );
    let title_wide: Vec<u16> = TARGET_TITLE
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr())).ok()?;
        if hwnd.0 as usize == 0 {
            tracing::trace!("[WINDOW] Window '{}' not found (hwnd=0)", TARGET_TITLE);
            None
        } else {
            tracing::trace!(
                "[WINDOW] Window '{}' found with hwnd={:?}",
                TARGET_TITLE,
                hwnd.0
            );
            Some(hwnd)
        }
    }
}

#[cfg(not(windows))]
pub fn find_blue_protocol_window() -> Option<()> {
    tracing::warn!("Window finding not implemented on this platform");
    None
}

/// Focus the Blue Protocol window
#[cfg(windows)]
pub fn focus_blue_protocol_window() -> Option<HWND> {
    tracing::debug!("[WINDOW] focus_blue_protocol_window() - attempting to focus game window");
    let hwnd = find_blue_protocol_window()?;

    unsafe {
        tracing::trace!(
            "[WINDOW] Calling ShowWindow and SetForegroundWindow for hwnd={:?}",
            hwnd.0
        );
        ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }

    tracing::debug!("[WINDOW] Game window focused successfully");
    Some(hwnd)
}

#[cfg(not(windows))]
pub fn focus_blue_protocol_window() -> Option<()> {
    tracing::warn!("Window focusing not implemented on this platform");
    None
}

/// Select and focus the game window
pub fn select_window() -> Option<String> {
    #[cfg(windows)]
    match focus_blue_protocol_window() {
        Some(_) => {
            tracing::info!("Automatically selected Blue Protocol window.");
            Some(TARGET_TITLE.to_string())
        }
        None => {
            tracing::warn!("Could not find Blue Protocol window. Waiting...");
            None
        }
    }

    #[cfg(not(windows))]
    {
        tracing::warn!("Window selection not implemented on this platform");
        None
    }
}

/// Get window rectangle (x1, y1, x2, y2)
#[cfg(windows)]
pub fn get_window_rect(title: &str) -> Option<(i32, i32, i32, i32)> {
    tracing::trace!(
        "[WINDOW] get_window_rect('{}') - getting window bounds",
        title
    );
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr())).ok()?;
        if hwnd.0 as usize == 0 {
            tracing::debug!("[WINDOW] Window '{}' not found.", title);
            return None;
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let result = (rect.left, rect.top, rect.right, rect.bottom);
            tracing::trace!(
                "[WINDOW] Window rect: left={}, top={}, right={}, bottom={} ({}x{})",
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                rect.right - rect.left,
                rect.bottom - rect.top
            );
            Some(result)
        } else {
            tracing::warn!("[WINDOW] GetWindowRect failed for '{}'", title);
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_window_rect(_title: &str) -> Option<(i32, i32, i32, i32)> {
    tracing::warn!("Window rect not implemented on this platform");
    None
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_find_nonexistent_window() {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

        let title_wide: Vec<u16> = "NonExistentWindow12345"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr()));
            assert!(hwnd.is_err() || hwnd.unwrap().0 as usize == 0);
        }
    }
}
