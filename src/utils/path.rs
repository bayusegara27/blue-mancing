//! Path utilities for finding data directories

use std::env;
use std::path::PathBuf;

/// Returns the folder where data files should be stored.
/// Uses current directory if running as script.
/// Uses executable directory if running as frozen executable.
pub fn get_data_dir() -> PathBuf {
    // Check if we're running as a bundled executable
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            // Check if this looks like a bundled app (has config folder nearby)
            let config_path = parent.join("config");
            if config_path.exists() {
                return parent.to_path_buf();
            }
        }
    }
    
    // Fall back to current working directory
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_dir() {
        let dir = get_data_dir();
        assert!(dir.exists() || dir == PathBuf::from("."));
    }
}
