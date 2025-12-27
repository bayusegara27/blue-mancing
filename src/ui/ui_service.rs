//! UI service for creating and managing windows

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

#[cfg(all(feature = "gui", windows))]
use std::fs;
#[cfg(all(feature = "gui", windows))]
use crate::utils::path::get_data_dir;
#[cfg(all(feature = "gui", windows))]
use crate::utils::bot_state::{SHARED_STATE, BotActivity};

/// Window types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Window {
    Main,
    Overlay,
}

/// Global window registry
static WINDOWS: Lazy<Arc<RwLock<HashMap<Window, WindowHandle>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

/// Window handle wrapper
#[derive(Clone)]
pub struct WindowHandle {
    pub title: String,
    // In a full implementation, this would hold actual window handle
}

/// Get a window by type
pub fn get_window(window_type: Window) -> Option<WindowHandle> {
    WINDOWS.read().ok()?.get(&window_type).cloned()
}

/// Register a window
pub fn register_window(window_type: Window, handle: WindowHandle) {
    if let Ok(mut windows) = WINDOWS.write() {
        windows.insert(window_type, handle);
    }
}

/// Handle IPC message from JavaScript
#[cfg(all(feature = "gui", windows))]
fn handle_ipc_message(message: &str) -> Option<String> {
    use crate::utils::bot_state::SHARED_STATE;
    use crate::utils::bot_state::BotActivity;
    
    // Parse the message as JSON
    let parsed: serde_json::Value = serde_json::from_str(message).ok()?;
    let action = parsed.get("action")?.as_str()?;
    
    match action {
        "start" => {
            println!("UI: Start button clicked");
            if !SHARED_STATE.is_running() {
                SHARED_STATE.set_running(true);
                SHARED_STATE.set_activity(BotActivity::SelectingWindow);
                SHARED_STATE.set_detail_message("Starting from UI...");
            }
            Some(r#"{"success": true, "action": "start"}"#.to_string())
        }
        "stop" => {
            println!("UI: Stop button clicked");
            SHARED_STATE.set_running(false);
            SHARED_STATE.set_activity(BotActivity::Stopped);
            SHARED_STATE.set_detail_message("Stopped from UI");
            Some(r#"{"success": true, "action": "stop"}"#.to_string())
        }
        "getStatus" => {
            Some(SHARED_STATE.to_json())
        }
        "minimize" => {
            // Window minimize is handled by the window itself
            Some(r#"{"success": true, "action": "minimize"}"#.to_string())
        }
        "close" => {
            // Window close is handled by the event loop
            Some(r#"{"success": true, "action": "close"}"#.to_string())
        }
        _ => {
            println!("UI: Unknown action: {}", action);
            None
        }
    }
}

/// Custom event types for the event loop
#[cfg(all(feature = "gui", windows))]
#[derive(Debug, Clone)]
enum UserEvent {
    UpdateOverlay,
}

/// Start the UI (main entry point)
#[cfg(all(feature = "gui", windows))]
pub fn start_ui() {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    
    let base = get_data_dir();
    let html_path = base.join("html");
    
    // Load HTML content - prefer the updated overlay HTML with IPC support
    let overlay_html = fs::read_to_string(html_path.join("overlay.html"))
        .unwrap_or_else(|_| get_default_overlay_html().to_string());
    let main_html = fs::read_to_string(html_path.join("main.html"))
        .unwrap_or_else(|_| get_default_main_html().to_string());
    
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    
    // Create main window
    let main_window = WindowBuilder::new()
        .with_title("bpsr-fishing Stats")
        .with_inner_size(tao::dpi::LogicalSize::new(940, 600))
        .with_min_inner_size(tao::dpi::LogicalSize::new(900, 600))
        .with_resizable(true)
        .build(&event_loop)
        .expect("Failed to create main window");
    
    let _main_webview = WebViewBuilder::new()
        .with_html(&main_html)
        .build(&main_window)
        .expect("Failed to create main webview");
    
    register_window(Window::Main, WindowHandle {
        title: "bpsr-fishing Stats".to_string(),
    });
    
    // Create overlay window
    let overlay_window = WindowBuilder::new()
        .with_title("bpsr-fishing Overlay")
        .with_inner_size(tao::dpi::LogicalSize::new(380, 220))
        .with_resizable(false)
        .with_decorations(false)
        .with_always_on_top(true)
        .build(&event_loop)
        .expect("Failed to create overlay window");
    
    let overlay_window_id = overlay_window.id();
    
    // Build overlay webview with IPC handler
    let overlay_webview = WebViewBuilder::new()
        .with_html(&overlay_html)
        .with_ipc_handler(move |request| {
            let message = request.body();
            if let Some(response) = handle_ipc_message(message) {
                tracing::debug!("IPC response: {}", response);
            }
        })
        .build(&overlay_window)
        .expect("Failed to create overlay webview");
    
    // Store webview in Arc for thread-safe access
    let overlay_webview = Arc::new(parking_lot::Mutex::new(overlay_webview));
    let overlay_webview_clone = overlay_webview.clone();
    
    register_window(Window::Overlay, WindowHandle {
        title: "bpsr-fishing Overlay".to_string(),
    });
    
    // Spawn a thread to periodically update the overlay with bot status
    // Using 250ms interval to balance responsiveness and CPU usage
    let proxy_clone = proxy.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(250));
            
            // Send update event to main thread
            let _ = proxy_clone.send_event(UserEvent::UpdateOverlay);
        }
    });
    
    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::UserEvent(UserEvent::UpdateOverlay) => {
                // Update overlay with current bot status
                let status_json = SHARED_STATE.to_json();
                let js = format!(
                    "if (window.updateFromRust) {{ window.updateFromRust({}); }}",
                    status_json
                );
                
                if let Some(webview) = overlay_webview_clone.try_lock() {
                    let _ = webview.evaluate_script(&js);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                if window_id == overlay_window_id {
                    // Don't close the entire app when overlay is closed
                    // Just hide the overlay (in a full implementation)
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}

/// Start the UI (stub for non-Windows/non-GUI builds)
#[cfg(not(all(feature = "gui", windows)))]
pub fn start_ui() {
    tracing::info!("GUI not available on this platform. Running in headless mode.");
    tracing::info!("Press Ctrl+C to exit.");
    
    // Block forever (until interrupted)
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// Default overlay HTML with IPC support for wry
fn get_default_overlay_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>BPSR Fishing Overlay</title>
    <style>
        body {
            font-family: 'Segoe UI', sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            color: #fff;
            margin: 0;
            padding: 10px;
        }
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 8px;
        }
        .title {
            font-size: 14px;
            font-weight: bold;
            color: #00ffc8;
        }
        .controls {
            display: flex;
            gap: 8px;
        }
        .btn {
            padding: 6px 14px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
            font-weight: bold;
            transition: all 0.2s;
        }
        .btn-start {
            background: #22c55e;
            color: white;
        }
        .btn-start:hover { background: #16a34a; }
        .btn-stop {
            background: #ef4444;
            color: white;
        }
        .btn-stop:hover { background: #dc2626; }
        .stats {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 6px;
            margin-bottom: 8px;
        }
        .stat-row {
            display: flex;
            justify-content: space-between;
            background: rgba(255,255,255,0.1);
            padding: 6px 10px;
            border-radius: 4px;
        }
        .label { color: #aaa; font-size: 12px; }
        .value { color: #00ffc8; font-weight: bold; font-size: 12px; }
        .status-section {
            background: rgba(0,0,0,0.3);
            border-radius: 6px;
            padding: 8px;
        }
        .status-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 6px;
        }
        .status-indicator {
            display: flex;
            align-items: center;
            gap: 6px;
        }
        .status-dot {
            width: 10px;
            height: 10px;
            border-radius: 50%;
            background: #ef4444;
        }
        .status-dot.running { background: #22c55e; animation: pulse 1.5s infinite; }
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        .status-text {
            font-size: 13px;
            font-weight: bold;
        }
        .status-text.running { color: #22c55e; }
        .status-text.stopped { color: #ef4444; }
        .activity {
            font-size: 11px;
            color: #94a3b8;
            margin-top: 4px;
            padding: 4px 8px;
            background: rgba(255,255,255,0.05);
            border-radius: 4px;
            min-height: 16px;
        }
        .timer {
            font-size: 12px;
            color: #60a5fa;
        }
    </style>
</head>
<body>
    <div class="header">
        <span class="title">BPSR Fishing</span>
        <div class="controls">
            <button class="btn btn-start" id="start-btn">Start</button>
            <button class="btn btn-stop" id="stop-btn">Stop</button>
        </div>
    </div>
    
    <div class="stats">
        <div class="stat-row">
            <span class="label">Catches</span>
            <span class="value" id="catches">0</span>
        </div>
        <div class="stat-row">
            <span class="label">Misses</span>
            <span class="value" id="misses">0</span>
        </div>
        <div class="stat-row">
            <span class="label">XP</span>
            <span class="value" id="xp">0</span>
        </div>
        <div class="stat-row">
            <span class="label">Rate</span>
            <span class="value" id="rate">0%</span>
        </div>
    </div>
    
    <div class="status-section">
        <div class="status-header">
            <div class="status-indicator">
                <div class="status-dot" id="status-dot"></div>
                <span class="status-text stopped" id="status-text">Stopped</span>
            </div>
            <span class="timer" id="timer">00:00:00</span>
        </div>
        <div class="activity" id="activity">Waiting for start...</div>
    </div>
    
    <script>
        let startTime = null;
        let timerInterval = null;
        let isRunning = false;
        
        // IPC handler for wry - sends message to Rust backend
        function sendToRust(action) {
            if (window.ipc) {
                window.ipc.postMessage(JSON.stringify({ action: action }));
            }
        }
        
        // Called from Rust to update the UI
        window.updateFromRust = function(data) {
            // Update stats
            if (data.stats) {
                document.getElementById('catches').textContent = data.stats.catches || 0;
                document.getElementById('misses').textContent = data.stats.misses || 0;
                document.getElementById('xp').textContent = data.stats.xp || 0;
                document.getElementById('rate').textContent = (data.stats.rate || '0') + '%';
            }
            
            // Update running status
            const wasRunning = isRunning;
            isRunning = data.running;
            
            const dot = document.getElementById('status-dot');
            const text = document.getElementById('status-text');
            
            if (isRunning) {
                dot.classList.add('running');
                text.classList.remove('stopped');
                text.classList.add('running');
                text.textContent = 'Running';
                
                // Start timer if just started
                if (!wasRunning) {
                    startTime = new Date();
                    if (!timerInterval) {
                        timerInterval = setInterval(updateTimer, 1000);
                    }
                }
            } else {
                dot.classList.remove('running');
                text.classList.remove('running');
                text.classList.add('stopped');
                text.textContent = 'Stopped';
                
                // Stop timer if just stopped
                if (wasRunning) {
                    if (timerInterval) {
                        clearInterval(timerInterval);
                        timerInterval = null;
                    }
                }
            }
            
            // Update activity text
            if (data.activity) {
                document.getElementById('activity').textContent = data.activity;
            }
            if (data.detail) {
                document.getElementById('activity').textContent = data.activity + ' - ' + data.detail;
            }
        };
        
        function updateTimer() {
            if (!startTime) return;
            const diff = new Date() - startTime;
            const h = String(Math.floor(diff / 3600000)).padStart(2, '0');
            const m = String(Math.floor((diff % 3600000) / 60000)).padStart(2, '0');
            const s = String(Math.floor((diff % 60000) / 1000)).padStart(2, '0');
            document.getElementById('timer').textContent = h + ':' + m + ':' + s;
        }
        
        // Button click handlers
        document.getElementById('start-btn').addEventListener('click', function() {
            sendToRust('start');
        });
        
        document.getElementById('stop-btn').addEventListener('click', function() {
            sendToRust('stop');
        });
        
        // Legacy pywebview compatibility - map to new IPC
        window.pywebview = {
            api: {
                start_script: function() { sendToRust('start'); },
                stop_script: function() { sendToRust('stop'); },
                minimize_window: function() { sendToRust('minimize'); },
                close_window: function() { sendToRust('close'); }
            }
        };
    </script>
</body>
</html>"#
}

/// Default main HTML
fn get_default_main_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>BPSR Fishing Stats</title>
    <style>
        body {
            font-family: 'Segoe UI', sans-serif;
            background: #0f0f23;
            color: #fff;
            margin: 0;
            padding: 20px;
        }
        h1 {
            color: #00ffc8;
            text-align: center;
        }
        .container {
            max-width: 900px;
            margin: 0 auto;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>BPSR Fishing Stats</h1>
        <p style="text-align: center; color: #aaa;">Loading...</p>
    </div>
</body>
</html>"#
}
