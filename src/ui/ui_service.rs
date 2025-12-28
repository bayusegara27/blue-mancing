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

/// Handle IPC message from dashboard JavaScript - supports full API
#[cfg(all(feature = "gui", windows))]
fn handle_dashboard_ipc(message: &str) -> String {
    use crate::utils::keybinds::get_key;
    use crate::ui::stats_api::StatsApi;
    use pulldown_cmark::{Parser, Options, html};
    
    // Parse the message as JSON
    let parsed: serde_json::Value = match serde_json::from_str(message) {
        Ok(v) => v,
        Err(_) => return r#"{"error": "Invalid JSON"}"#.to_string(),
    };
    
    let action = match parsed.get("action").and_then(|a| a.as_str()) {
        Some(a) => a,
        None => return r#"{"error": "Missing action"}"#.to_string(),
    };
    
    match action {
        "get_guide" => {
            // Load and convert GUIDE.md to HTML
            let base = get_data_dir();
            let guide_path = base.join("GUIDE.md");
            
            let markdown = match fs::read_to_string(&guide_path) {
                Ok(content) => content,
                Err(_) => {
                    // Try current directory as fallback
                    match fs::read_to_string("GUIDE.md") {
                        Ok(content) => content,
                        Err(_) => "# Guide\n\nGuide content not found.".to_string(),
                    }
                }
            };
            
            // Convert markdown to HTML
            let mut options = Options::empty();
            options.insert(Options::ENABLE_STRIKETHROUGH);
            let parser = Parser::new_ext(&markdown, options);
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            
            // Wrap in a div with styling
            let result = format!(r#"<div class="intro-card">{}</div>"#, html_output);
            serde_json::to_string(&result).unwrap_or_else(|_| r#""""#.to_string())
        }
        "get_daily_table" => {
            let mut stats = StatsApi::new();
            let html = stats.get_daily_table();
            serde_json::to_string(&html).unwrap_or_else(|_| r#""""#.to_string())
        }
        "get_overall_summary" => {
            let mut stats = StatsApi::new();
            let html = stats.get_overall_summary();
            serde_json::to_string(&html).unwrap_or_else(|_| r#""""#.to_string())
        }
        "get_resolution" => {
            let stats = StatsApi::new();
            let res = stats.get_resolution();
            serde_json::to_string(&res).unwrap_or_else(|_| r#""1920x1080""#.to_string())
        }
        "set_resolution" => {
            let res = parsed.get("value").and_then(|v| v.as_str()).unwrap_or("1920x1080");
            let mut stats = StatsApi::new();
            stats.set_resolution(res);
            r#"{"success": true}"#.to_string()
        }
        "get_key" => {
            let key_name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            match get_key(key_name) {
                Some(k) => serde_json::to_string(&k).unwrap_or_else(|_| r#""""#.to_string()),
                None => r#""""#.to_string(),
            }
        }
        "capture_key_for" => {
            // For now, return a placeholder - actual key capture requires native keyboard hooks
            // This would need to be implemented with a proper key capture mechanism
            let key_name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let current = get_key(key_name).unwrap_or_else(|| "F9".to_string());
            serde_json::to_string(&current).unwrap_or_else(|_| r#""F9""#.to_string())
        }
        "set_debug_overlay" => {
            let value = parsed.get("value").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut stats = StatsApi::new();
            stats.set_show_debug_overlay(value);
            r#"{"success": true}"#.to_string()
        }
        "get_debug_overlay" => {
            let stats = StatsApi::new();
            let value = stats.get_show_debug_overlay();
            serde_json::to_string(&value).unwrap_or_else(|_| "true".to_string())
        }
        "set_overlay_on_top" => {
            let value = parsed.get("value").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut stats = StatsApi::new();
            stats.set_overlay_always_on_top(value);
            r#"{"success": true}"#.to_string()
        }
        "get_overlay_on_top" => {
            let stats = StatsApi::new();
            let value = stats.get_overlay_always_on_top();
            serde_json::to_string(&value).unwrap_or_else(|_| "true".to_string())
        }
        "set_show_overlay" => {
            let value = parsed.get("value").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut stats = StatsApi::new();
            stats.set_show_overlay(value);
            r#"{"success": true}"#.to_string()
        }
        "get_show_overlay" => {
            let stats = StatsApi::new();
            let value = stats.get_show_overlay();
            serde_json::to_string(&value).unwrap_or_else(|_| "true".to_string())
        }
        "set_auto_bait" | "set_auto_rod" => {
            // TODO: These settings need full implementation
            r#"{"success": true}"#.to_string()
        }
        _ => {
            format!(r#"{{"error": "Unknown action: {}"}}"#, action)
        }
    }
}

/// Inject the pywebview API bridge into HTML
#[cfg(all(feature = "gui", windows))]
fn inject_api_bridge(html: &str) -> String {
    // JavaScript bridge that provides pywebview.api compatible interface
    let api_bridge = r#"
<script>
// API Bridge for Blue Mancing Dashboard
(function() {
    // Promise-based API that uses IPC
    const pendingRequests = new Map();
    let requestId = 0;
    
    // Create the pywebview.api object
    window.pywebview = {
        api: {
            get_guide: function() {
                return callApi('get_guide', {});
            },
            get_daily_table: function() {
                return callApi('get_daily_table', {});
            },
            get_overall_summary: function() {
                return callApi('get_overall_summary', {});
            },
            get_resolution: function() {
                return callApi('get_resolution', {});
            },
            set_resolution: function(value) {
                return callApi('set_resolution', { value: value });
            },
            get_key: function(name) {
                return callApi('get_key', { name: name });
            },
            capture_key_for: function(name) {
                return callApi('capture_key_for', { name: name });
            },
            set_auto_bait: function(value) {
                return callApi('set_auto_bait', { value: value });
            },
            set_auto_rod: function(value) {
                return callApi('set_auto_rod', { value: value });
            },
            set_debug_overlay: function(value) {
                // Update preloaded value
                window.__bluemancing_settings.show_debug_overlay = value;
                return callApi('set_debug_overlay', { value: value });
            },
            get_debug_overlay: function() {
                return callApi('get_debug_overlay', {});
            },
            set_overlay_on_top: function(value) {
                // Update preloaded value
                window.__bluemancing_settings.overlay_always_on_top = value;
                return callApi('set_overlay_on_top', { value: value });
            },
            get_overlay_on_top: function() {
                return callApi('get_overlay_on_top', {});
            },
            set_show_overlay: function(value) {
                // Update preloaded value
                window.__bluemancing_settings.show_overlay = value;
                return callApi('set_show_overlay', { value: value });
            },
            get_show_overlay: function() {
                return callApi('get_show_overlay', {});
            }
        }
    };
    
    // Call API via IPC - uses synchronous XMLHttpRequest workaround for wry
    function callApi(action, params) {
        return new Promise((resolve, reject) => {
            try {
                const message = JSON.stringify({ action: action, ...params });
                
                // Use ipc.postMessage for wry
                if (window.ipc) {
                    window.ipc.postMessage(message);
                }
                
                // Note: wry IPC is one-way (JS -> Rust), so we use preloaded data
                // for immediate display. The IPC call triggers a refresh on the Rust side.
                // Data is preloaded at startup and returned synchronously from cache.
                const result = callApiSync(action, params);
                resolve(result);
            } catch (e) {
                console.error('API call failed:', e);
                reject(e);
            }
        });
    }
    
    // Get data from preloaded cache (loaded at startup by Rust)
    function callApiSync(action, params) {
        const message = JSON.stringify({ action: action, ...params });
        
        // Send via IPC
        if (window.ipc) {
            window.ipc.postMessage(message);
        }
        
        // Return cached/inline data for immediate response
        // The actual data will be loaded on first call
        return getInlineData(action, params);
    }
    
    // Get inline data (preloaded by Rust)
    function getInlineData(action, params) {
        switch(action) {
            case 'get_guide':
                return window.__bluemancing_guide || '';
            case 'get_daily_table':
                return window.__bluemancing_daily || '<p>Loading daily data...</p>';
            case 'get_overall_summary':
                return window.__bluemancing_summary || '<p>Loading summary...</p>';
            case 'get_resolution':
                return window.__bluemancing_resolution || '1920x1080';
            case 'get_key':
                return window.__bluemancing_keys && window.__bluemancing_keys[params.name] || '';
            case 'get_debug_overlay':
                return window.__bluemancing_settings && window.__bluemancing_settings.show_debug_overlay;
            case 'get_overlay_on_top':
                return window.__bluemancing_settings && window.__bluemancing_settings.overlay_always_on_top;
            case 'get_show_overlay':
                return window.__bluemancing_settings && window.__bluemancing_settings.show_overlay;
            default:
                return null;
        }
    }
    
    // Signal that pywebview is ready
    setTimeout(function() {
        window.dispatchEvent(new Event('pywebviewready'));
    }, 100);
})();
</script>
"#;
    
    // Also inject preloaded data
    let guide_data = get_guide_html();
    let daily_data = get_daily_html();
    let summary_data = get_summary_html();
    let resolution = get_resolution_value();
    let keys_data = get_keys_json();
    let settings_data = get_overlay_settings_json();
    
    let preload_script = format!(r#"
<script>
// Preloaded data for immediate display
window.__bluemancing_guide = {};
window.__bluemancing_daily = {};
window.__bluemancing_summary = {};
window.__bluemancing_resolution = {};
window.__bluemancing_keys = {};
window.__bluemancing_settings = {};
</script>
"#, 
        serde_json::to_string(&guide_data).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(&daily_data).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(&summary_data).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::to_string(&resolution).unwrap_or_else(|_| "\"1920x1080\"".to_string()),
        keys_data,
        settings_data
    );
    
    // Inject before </head> tag
    if let Some(pos) = html.find("</head>") {
        let mut result = html.to_string();
        result.insert_str(pos, &preload_script);
        result.insert_str(pos, api_bridge);
        result
    } else {
        // If no </head> found, prepend
        format!("{}{}{}", api_bridge, preload_script, html)
    }
}

/// Get guide HTML content
#[cfg(all(feature = "gui", windows))]
fn get_guide_html() -> String {
    use pulldown_cmark::{Parser, Options, html};
    
    let base = get_data_dir();
    let guide_path = base.join("GUIDE.md");
    
    let markdown = match fs::read_to_string(&guide_path) {
        Ok(content) => content,
        Err(_) => {
            // Try current directory as fallback
            match fs::read_to_string("GUIDE.md") {
                Ok(content) => content,
                Err(_) => "# Guide\n\nGuide content not found.".to_string(),
            }
        }
    };
    
    // Convert markdown to HTML
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    format!(r#"<div class="intro-card">{}</div>"#, html_output)
}

/// Get daily stats HTML
#[cfg(all(feature = "gui", windows))]
fn get_daily_html() -> String {
    use crate::ui::stats_api::StatsApi;
    let mut stats = StatsApi::new();
    stats.get_daily_table()
}

/// Get summary HTML
#[cfg(all(feature = "gui", windows))]
fn get_summary_html() -> String {
    use crate::ui::stats_api::StatsApi;
    let mut stats = StatsApi::new();
    stats.get_overall_summary()
}

/// Get resolution value
#[cfg(all(feature = "gui", windows))]
fn get_resolution_value() -> String {
    use crate::ui::stats_api::StatsApi;
    let stats = StatsApi::new();
    stats.get_resolution()
}

/// Get all keys as JSON
#[cfg(all(feature = "gui", windows))]
fn get_keys_json() -> String {
    use crate::utils::keybinds::get_key;
    
    let key_names = ["start_key", "stop_key", "fish_key", "bait_key", "rods_key", "esc_key", "left_key", "right_key"];
    let mut keys = std::collections::HashMap::new();
    
    for name in &key_names {
        if let Some(value) = get_key(name) {
            keys.insert(name.to_string(), value);
        }
    }
    
    serde_json::to_string(&keys).unwrap_or_else(|_| "{}".to_string())
}

/// Get overlay settings as JSON
#[cfg(all(feature = "gui", windows))]
fn get_overlay_settings_json() -> String {
    use crate::ui::stats_api::StatsApi;
    
    let stats = StatsApi::new();
    let mut settings = std::collections::HashMap::new();
    
    settings.insert("show_debug_overlay".to_string(), stats.get_show_debug_overlay());
    settings.insert("overlay_always_on_top".to_string(), stats.get_overlay_always_on_top());
    settings.insert("show_overlay".to_string(), stats.get_show_overlay());
    
    serde_json::to_string(&settings).unwrap_or_else(|_| r#"{"show_debug_overlay":true,"overlay_always_on_top":true,"show_overlay":true}"#.to_string())
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
        .with_title("Blue Mancing - Dashboard")
        .with_inner_size(tao::dpi::LogicalSize::new(940, 600))
        .with_min_inner_size(tao::dpi::LogicalSize::new(900, 600))
        .with_resizable(true)
        .build(&event_loop)
        .expect("Failed to create main window");
    
    // Inject the pywebview API bridge into the main HTML
    let main_html_with_api = inject_api_bridge(&main_html);
    
    // Build main webview with IPC handler for dashboard API calls
    let main_webview = WebViewBuilder::new()
        .with_html(&main_html_with_api)
        .with_ipc_handler(move |request| {
            let message = request.body();
            // IPC handler processes requests but responses are preloaded in HTML
            // The response is logged for debugging purposes
            let response = handle_dashboard_ipc(message);
            tracing::debug!("Dashboard IPC: {} -> {}", message, response);
        })
        .build(&main_window)
        .expect("Failed to create main webview");
    
    // Store main webview for evaluating scripts
    let main_webview = Arc::new(parking_lot::Mutex::new(main_webview));
    
    register_window(Window::Main, WindowHandle {
        title: "Blue Mancing - Dashboard".to_string(),
    });
    
    // Create overlay window - compact size to fit HTML content (240px width + padding)
    let overlay_window = WindowBuilder::new()
        .with_title("Blue Mancing")
        .with_inner_size(tao::dpi::LogicalSize::new(244, 260))
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
        title: "Blue Mancing".to_string(),
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
    <title>Blue Mancing Overlay</title>
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
            color: #38c6ff;
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
        <span class="title">Blue Mancing</span>
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
    <title>Blue Mancing - Dashboard</title>
    <style>
        body {
            font-family: 'Segoe UI', sans-serif;
            background: #0a1628;
            color: #fff;
            margin: 0;
            padding: 20px;
        }
        h1 {
            color: #38c6ff;
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
        <h1>Blue Mancing</h1>
        <p style="text-align: center; color: #aaa;">Loading...</p>
    </div>
</body>
</html>"#
}
