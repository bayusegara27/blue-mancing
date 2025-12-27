//! UI service for creating and managing windows

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

#[cfg(all(feature = "gui", windows))]
use std::fs;
#[cfg(all(feature = "gui", windows))]
use crate::utils::path::get_data_dir;

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

/// Start the UI (main entry point)
#[cfg(all(feature = "gui", windows))]
pub fn start_ui() {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;
    
    let base = get_data_dir();
    let html_path = base.join("html");
    
    // Load HTML content
    let overlay_html = fs::read_to_string(html_path.join("overlay.html"))
        .unwrap_or_else(|_| get_default_overlay_html().to_string());
    let main_html = fs::read_to_string(html_path.join("main.html"))
        .unwrap_or_else(|_| get_default_main_html().to_string());
    
    let event_loop = EventLoop::new();
    
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
        .with_inner_size(tao::dpi::LogicalSize::new(310, 170))
        .with_resizable(false)
        .with_decorations(false)
        .with_always_on_top(true)
        .build(&event_loop)
        .expect("Failed to create overlay window");
    
    let _overlay_webview = WebViewBuilder::new()
        .with_html(&overlay_html)
        .build(&overlay_window)
        .expect("Failed to create overlay webview");
    
    register_window(Window::Overlay, WindowHandle {
        title: "bpsr-fishing Overlay".to_string(),
    });
    
    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
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

/// Default overlay HTML
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
            -webkit-app-region: drag;
        }
        .stats {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }
        .stat-row {
            display: flex;
            justify-content: space-between;
            background: rgba(255,255,255,0.1);
            padding: 8px 12px;
            border-radius: 6px;
        }
        .label { color: #aaa; }
        .value { color: #00ffc8; font-weight: bold; }
        .status {
            text-align: center;
            padding: 6px;
            background: #2d2d44;
            border-radius: 4px;
            margin-top: 8px;
        }
        .status.running { color: #4ade80; }
        .status.stopped { color: #f87171; }
    </style>
</head>
<body>
    <div class="stats">
        <div class="stat-row">
            <span class="label">Catches:</span>
            <span class="value" id="catches">0</span>
        </div>
        <div class="stat-row">
            <span class="label">Misses:</span>
            <span class="value" id="misses">0</span>
        </div>
        <div class="stat-row">
            <span class="label">XP:</span>
            <span class="value" id="xp">0</span>
        </div>
        <div class="stat-row">
            <span class="label">Rate:</span>
            <span class="value" id="rate">0%</span>
        </div>
        <div class="status stopped" id="status">Stopped</div>
    </div>
    <script>
        window.updateStats = function(stats) {
            document.getElementById('catches').textContent = stats.catches || 0;
            document.getElementById('misses').textContent = stats.misses || 0;
            document.getElementById('xp').textContent = stats.xp || 0;
            document.getElementById('rate').textContent = (stats.rate || 0).toFixed(2) + '%';
        };
        window.toggleBotStatus = function(status) {
            const el = document.getElementById('status');
            el.className = 'status ' + status;
            el.textContent = status.charAt(0).toUpperCase() + status.slice(1);
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
