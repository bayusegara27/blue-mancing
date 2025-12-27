//! Window management using Windows API

#![allow(dead_code)]

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, RECT};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowRect, SetForegroundWindow, ShowWindow, SW_SHOW,
};
#[cfg(windows)]
use windows::core::PCWSTR;

/// Window title for Blue Protocol
const TARGET_TITLE: &str = "Blue Protocol: Star Resonance";

/// Find the Blue Protocol window
#[cfg(windows)]
pub fn find_blue_protocol_window() -> Option<HWND> {
    let title_wide: Vec<u16> = TARGET_TITLE.encode_utf16().chain(std::iter::once(0)).collect();
    
    unsafe {
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr())).ok()?;
        if hwnd.0 as usize == 0 {
            None
        } else {
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
    let hwnd = find_blue_protocol_window()?;
    
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    
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
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    
    unsafe {
        let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr())).ok()?;
        if hwnd.0 as usize == 0 {
            tracing::warn!("Window '{}' not found.", title);
            return None;
        }
        
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some((rect.left, rect.top, rect.right, rect.bottom))
        } else {
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
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        use windows::core::PCWSTR;
        
        let title_wide: Vec<u16> = "NonExistentWindow12345".encode_utf16().chain(std::iter::once(0)).collect();
        
        unsafe {
            let hwnd = FindWindowW(PCWSTR::null(), PCWSTR(title_wide.as_ptr()));
            assert!(hwnd.is_err() || hwnd.unwrap().0 as usize == 0);
        }
    }
}
