//! Base types and settings for screen reader

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use crate::utils::path::get_data_dir;

/// Default settings
pub static DEFAULT_SETTINGS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("resolution".to_string(), "1920x1080".to_string());
    m.insert("auto_bait_purchase".to_string(), "T1".to_string());
    m.insert("auto_rods_purchase".to_string(), "T1".to_string());
    m.insert("start_key".to_string(), "F9".to_string());
    m.insert("stop_key".to_string(), "F10".to_string());
    m.insert("rods_key".to_string(), "M".to_string());
    m.insert("bait_key".to_string(), "N".to_string());
    m.insert("fish_key".to_string(), "F".to_string());
    m.insert("esc_key".to_string(), "ESC".to_string());
    m
});

/// Settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub resolution: String,
    pub auto_bait_purchase: String,
    pub auto_rods_purchase: String,
    pub start_key: String,
    pub stop_key: String,
    pub rods_key: String,
    pub bait_key: String,
    pub fish_key: String,
    pub esc_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            resolution: "1920x1080".to_string(),
            auto_bait_purchase: "T1".to_string(),
            auto_rods_purchase: "T1".to_string(),
            start_key: "F9".to_string(),
            stop_key: "F10".to_string(),
            rods_key: "M".to_string(),
            bait_key: "N".to_string(),
            fish_key: "F".to_string(),
            esc_key: "ESC".to_string(),
        }
    }
}

/// Get settings file path
fn get_settings_path() -> std::path::PathBuf {
    get_data_dir().join("config").join("settings.json")
}

/// Get current settings
pub fn get_settings() -> HashMap<String, String> {
    let settings_file = get_settings_path();
    
    if settings_file.exists() {
        if let Ok(content) = fs::read_to_string(&settings_file) {
            if let Ok(user_settings) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&content) {
                let mut settings = DEFAULT_SETTINGS.clone();
                for (key, value) in user_settings {
                    if let Some(s) = value.as_str() {
                        settings.insert(key, s.to_string());
                    }
                }
                return settings;
            }
        }
    }
    
    DEFAULT_SETTINGS.clone()
}

/// Get resolution folder based on current settings
pub fn get_resolution_folder() -> String {
    get_settings().get("resolution").cloned().unwrap_or_else(|| "1920x1080".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.resolution, "1920x1080");
        assert_eq!(settings.start_key, "F9");
    }

    #[test]
    fn test_get_resolution_folder() {
        let folder = get_resolution_folder();
        assert!(!folder.is_empty());
    }
}
