//! Overview API for the overlay window

#![allow(dead_code)]

use crate::utils::bot_state::{BotActivity, SHARED_STATE};
use crate::utils::keybinds::{get_keys, key_to_str, resolve_key, set_keys};

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

    /// Start the fishing bot from UI
    pub fn start_bot(&self) {
        if !SHARED_STATE.is_running() {
            SHARED_STATE.set_running(true);
            SHARED_STATE.set_activity(BotActivity::SelectingWindow);
            SHARED_STATE.set_detail_message("Starting from UI...");
        }
    }

    /// Stop the fishing bot from UI
    pub fn stop_bot(&self) {
        if SHARED_STATE.is_running() {
            SHARED_STATE.set_running(false);
            SHARED_STATE.set_activity(BotActivity::Stopped);
            SHARED_STATE.set_detail_message("Stopped from UI");
        }
    }

    /// Get current bot status as JSON
    pub fn get_status(&self) -> String {
        SHARED_STATE.to_json()
    }

    /// Check if bot is running
    pub fn is_running(&self) -> bool {
        SHARED_STATE.is_running()
    }

    /// Get current activity description
    pub fn get_activity(&self) -> String {
        SHARED_STATE.get_activity().description().to_string()
    }

    /// Get detailed status message
    pub fn get_detail(&self) -> String {
        SHARED_STATE.get_detail_message()
    }
}

impl Default for OverviewApi {
    fn default() -> Self {
        Self::new()
    }
}
