//! Overview API for the overlay window

#![allow(dead_code)]

use crate::utils::keybinds::{get_keys, set_keys, key_to_str, resolve_key};

/// Overview API exposed to JavaScript
pub struct OverviewApi {
    start_key: String,
    stop_key: String,
}

impl OverviewApi {
    pub fn new() -> Self {
        let (start, stop) = get_keys();
        Self {
            start_key: start,
            stop_key: stop,
        }
    }
    
    pub fn get_start_key(&self) -> String {
        key_to_str(&self.start_key)
    }
    
    pub fn get_stop_key(&self) -> String {
        key_to_str(&self.stop_key)
    }
    
    pub fn set_start_key(&mut self, key_str: &str) -> Result<String, String> {
        let new_key = resolve_key(key_str).ok_or_else(|| format!("Invalid key: {}", key_str))?;
        self.start_key = new_key.clone();
        set_keys(&key_str, &self.get_stop_key())?;
        Ok(new_key)
    }
    
    pub fn set_stop_key(&mut self, key_str: &str) -> Result<String, String> {
        let new_key = resolve_key(key_str).ok_or_else(|| format!("Invalid key: {}", key_str))?;
        self.stop_key = new_key.clone();
        set_keys(&self.get_start_key(), key_str)?;
        Ok(new_key)
    }
}

impl Default for OverviewApi {
    fn default() -> Self {
        Self::new()
    }
}
