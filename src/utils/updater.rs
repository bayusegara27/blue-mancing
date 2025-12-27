//! Auto-updater for the fishing bot

#![allow(dead_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::Deserialize;
use anyhow::{Result, Context};

/// Application version
pub const APP_VERSION: &str = "v1.2.1";

/// URL to check for updates
const LATEST_URL: &str = "https://raw.githubusercontent.com/rdsp04/bpsr-fishing/main/latest.json";

/// Update information
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

/// Update API for managing download progress
pub struct UpdateApi {
    pub progress: Arc<Mutex<f32>>,
    pub downloaded_mb: Arc<Mutex<f32>>,
    pub total_mb: Arc<Mutex<f32>>,
}

impl UpdateApi {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(0.0)),
            downloaded_mb: Arc::new(Mutex::new(0.0)),
            total_mb: Arc::new(Mutex::new(0.0)),
        }
    }
    
    pub fn set_progress(&self, percent: f32, downloaded: Option<f32>, total: Option<f32>) {
        *self.progress.lock() = percent;
        if let Some(d) = downloaded {
            *self.downloaded_mb.lock() = d;
        }
        if let Some(t) = total {
            *self.total_mb.lock() = t;
        }
    }
    
    pub fn get_progress(&self) -> (f32, f32, f32) {
        (
            *self.progress.lock(),
            *self.downloaded_mb.lock(),
            *self.total_mb.lock(),
        )
    }
}

impl Default for UpdateApi {
    fn default() -> Self {
        Self::new()
    }
}

/// Check for updates
pub async fn check_for_update() -> Option<UpdateInfo> {
    let client = reqwest::Client::new();
    
    match client.get(LATEST_URL).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(response) => {
            if let Ok(latest) = response.json::<UpdateInfo>().await {
                if latest.version != APP_VERSION {
                    return Some(latest);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to check for updates: {}", e);
        }
    }
    
    None
}

/// Check for updates (blocking version)
pub fn check_for_update_blocking() -> Option<UpdateInfo> {
    let client = reqwest::blocking::Client::new();
    
    match client.get(LATEST_URL).timeout(std::time::Duration::from_secs(5)).send() {
        Ok(response) => {
            if let Ok(latest) = response.json::<UpdateInfo>() {
                if latest.version != APP_VERSION {
                    return Some(latest);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to check for updates: {}", e);
        }
    }
    
    None
}

/// Download update with progress tracking
pub async fn download_update(update_info: &UpdateInfo, api: &UpdateApi) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("bpsr_fishing_update.exe");
    
    // Remove existing file if present
    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
    }
    
    let client = reqwest::Client::new();
    let response = client.get(&update_info.url).send().await?;
    
    let total_size = response.content_length().unwrap_or(0);
    let total_mb = total_size as f32 / (1024.0 * 1024.0);
    api.set_progress(0.0, Some(0.0), Some(total_mb));
    
    let mut file = File::create(&temp_path)?;
    let mut downloaded: u64 = 0;
    
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        
        let percent = if total_size > 0 {
            (downloaded as f32 / total_size as f32) * 100.0
        } else {
            0.0
        };
        let downloaded_mb = downloaded as f32 / (1024.0 * 1024.0);
        api.set_progress(percent, Some(downloaded_mb), Some(total_mb));
    }
    
    Ok(temp_path)
}

/// Run the installer
pub fn run_installer(installer_path: &PathBuf) -> Result<()> {
    Command::new(installer_path)
        .args(["/UPDATER", "/S"])
        .spawn()
        .context("Failed to run installer")?;
    
    Ok(())
}

/// Get HTML for update progress window
pub fn get_update_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Updating</title>
    <style>
      body {
        font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
        text-align: center;
        background-color: #f0f0f0;
        margin: 0;
        padding: 20px;
      }
      h3 {
        margin-bottom: 20px;
        color: #333;
      }
      #progress-container {
        width: 80%;
        height: 30px;
        margin: auto;
        background-color: #ddd;
        border-radius: 15px;
        box-shadow: inset 0 2px 5px rgba(0,0,0,0.2);
      }
      #bar {
        height: 100%;
        width: 0%;
        background: linear-gradient(90deg, #4caf50, #81c784);
        border-radius: 15px;
        transition: width 0.2s;
      }
      #percent {
        margin-top: 10px;
        font-weight: bold;
        color: #555;
      }
    </style>
  </head>
  <body>
    <h3>Updating...</h3>
    <div id="progress-container">
      <div id="bar"></div>
    </div>
    <p id="percent">0%</p>
  </body>
  <script>
    function setProgress(percent, text){
      document.getElementById('bar').style.width = percent + '%';
      document.getElementById('percent').innerText = text ? text : percent + '%';
    }
  </script>
</html>"#
}
