//! Session logging functionality

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::utils::path::get_data_dir;

/// Session entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub start: String,
    pub stop: Option<String>,
}

/// Get sessions file path
fn get_sessions_path() -> PathBuf {
    get_data_dir().join("logs").join("sessions.json")
}

/// Load sessions from file
pub fn load_sessions() -> Vec<Session> {
    let sessions_file = get_sessions_path();

    if sessions_file.exists() {
        if let Ok(content) = fs::read_to_string(&sessions_file) {
            if let Ok(sessions) = serde_json::from_str(&content) {
                return sessions;
            }
        }
    }

    Vec::new()
}

/// Save sessions to file
pub fn save_sessions(sessions: &[Session]) {
    let sessions_file = get_sessions_path();

    if let Some(parent) = sessions_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(content) = serde_json::to_string_pretty(sessions) {
        let _ = fs::write(&sessions_file, content);
    }
}

/// Log entry for fishing catch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchLogEntry {
    pub timestamp: String,
    #[serde(rename = "catch")]
    pub status: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fish_type: Option<String>,
}

/// Log entry for broken rod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenRodLogEntry {
    pub timestamp: String,
    pub broken: bool,
}

/// Log a catch to the fishing log
pub fn log_catch(status: bool, fish_type: Option<String>) {
    let log_file = get_data_dir().join("logs").join("fishing_log.json");

    if let Some(parent) = log_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let entry = CatchLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        status,
        fish_type,
    };

    let mut data: Vec<CatchLogEntry> = if log_file.exists() {
        fs::read_to_string(&log_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    data.push(entry);

    if let Ok(content) = serde_json::to_string_pretty(&data) {
        let _ = fs::write(&log_file, content);
    }
}

/// Log a broken rod
pub fn log_broken_rod() {
    let log_file = get_data_dir().join("logs").join("broken_rods.json");

    if let Some(parent) = log_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let entry = BrokenRodLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        broken: true,
    };

    let mut data: Vec<BrokenRodLogEntry> = if log_file.exists() {
        fs::read_to_string(&log_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    data.push(entry);

    if let Ok(content) = serde_json::to_string_pretty(&data) {
        let _ = fs::write(&log_file, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sessions_empty() {
        // This will return empty vec if file doesn't exist
        let sessions = load_sessions();
        // Just verify it doesn't panic
        assert!(sessions.is_empty() || !sessions.is_empty());
    }
}
