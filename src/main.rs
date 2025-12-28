//! Blue Mancing - Auto fishing bot for Blue Protocol: Star Resonance
//!
//! A high-performance Rust-based automation tool for fishing in Blue Protocol: Star Resonance.
//! Features:
//! - Screen capture and template matching for game state detection
//! - Automatic fishing rod casting and fish catching
//! - Mini-game arrow detection and lane management
//! - Session statistics tracking
//! - Compact, draggable overlay UI
//! - Auto-update support

#![allow(unused_imports)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use parking_lot::Mutex;

mod fish;
mod input;
mod log_main;
mod screen_reader;
mod ui;
mod utils;
mod window;

use fish::FishService;
use input::{click, hold_key, mouse_move, mouse_press, mouse_release, press_key, release_key};
use log_main::{load_sessions, log_broken_rod, log_catch, save_sessions, Session};
use screen_reader::{get_resolution_folder, ImageService};
use utils::bot_state::{BotActivity, SHARED_STATE};
use utils::keybinds::{get_keys, get_pykey, string_to_code};
use utils::path::get_data_dir;
use utils::spelling::fix_spelling;
use utils::updater::{check_for_update_blocking, APP_VERSION};
use window::{focus_blue_protocol_window, get_window_rect, select_window};

// Constants
const TARGET_IMAGES_FOLDER: &str = "images";
const CHECK_INTERVAL: Duration = Duration::from_millis(50);
const THRESHOLD: f32 = 0.7;
const SPAM_CPS: u32 = 20;
const NO_PROGRESS_LIMIT: u64 = 45;

/// Session statistics
#[derive(Debug, Clone, Default)]
struct SessionStats {
    catches: i32,
    misses: i32,
    xp: i32,
    rate: f64,
}

/// Global state for the macro
struct MacroState {
    running: AtomicBool,
    window_title: Mutex<Option<String>>,
    saved_continue_pos: Mutex<Option<(i32, i32)>>,
    last_progress_time: Mutex<Instant>,
    session_stats: Mutex<SessionStats>,
}

impl MacroState {
    fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            window_title: Mutex::new(None),
            saved_continue_pos: Mutex::new(None),
            last_progress_time: Mutex::new(Instant::now()),
            session_stats: Mutex::new(SessionStats::default()),
        }
    }

    /// Check if bot is running - uses SHARED_STATE as the single source of truth
    fn is_running(&self) -> bool {
        SHARED_STATE.is_running()
    }

    /// Set running state - updates both local and shared state
    fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
        SHARED_STATE.set_running(running);
    }

    fn update_progress(&self) {
        *self.last_progress_time.lock() = Instant::now();
    }

    fn time_since_progress(&self) -> Duration {
        self.last_progress_time.lock().elapsed()
    }
}

/// Handle start key press
fn handle_start_key(state: &Arc<MacroState>) {
    tracing::debug!("[START] handle_start_key() called");
    SHARED_STATE.set_activity(BotActivity::SelectingWindow);
    SHARED_STATE.set_detail_message("Looking for Blue Protocol window...");

    let window_title = select_window();

    if window_title.is_none() {
        tracing::debug!("[START] No Blue Protocol window found");
        println!("No window found. Cannot start macro.");
        SHARED_STATE.set_activity(BotActivity::Idle);
        SHARED_STATE.set_detail_message("No game window found");
        return;
    }

    let mut sessions = load_sessions();

    // Check if there's already an active session
    if !sessions.is_empty() && sessions.last().unwrap().stop.is_none() {
        println!("Session already started. Press stop first.");
        SHARED_STATE.set_detail_message("Session already active");
        return;
    }

    // Start new session
    sessions.push(Session {
        start: Utc::now().to_rfc3339(),
        stop: None,
    });
    save_sessions(&sessions);

    *state.window_title.lock() = window_title.clone();
    state.set_running(true);
    state.update_progress();

    // Reset stats for new session
    *state.session_stats.lock() = SessionStats::default();
    SHARED_STATE.reset_stats();

    SHARED_STATE.set_activity(BotActivity::WaitingForDefaultScreen);
    SHARED_STATE.set_detail_message(format!(
        "Connected to: {}",
        window_title.as_ref().unwrap_or(&"Unknown".to_string())
    ));

    println!("Macro started on window: {:?}", window_title);
}

/// Handle stop key press
fn handle_stop_key(state: &Arc<MacroState>) {
    tracing::debug!("[STOP] handle_stop_key() called");
    let mut sessions = load_sessions();

    if sessions.is_empty() || sessions.last().unwrap().stop.is_some() {
        println!("No active session to stop.");
        return;
    }

    // End session
    if let Some(last) = sessions.last_mut() {
        last.stop = Some(Utc::now().to_rfc3339());
    }
    save_sessions(&sessions);

    state.set_running(false);
    *state.saved_continue_pos.lock() = None;
    *state.window_title.lock() = None;

    SHARED_STATE.set_activity(BotActivity::Stopped);
    SHARED_STATE.set_detail_message("Bot stopped by user");

    println!("Macro stopped");
}

/// Post-catch loop - handles the fishing minigame
fn post_catch_loop(
    state: &Arc<MacroState>,
    image_service: &ImageService,
    fish_service: &FishService,
    window_title: &str,
) {
    tracing::debug!("[MINIGAME] Fish caught - starting minigame");
    println!("Fish took the bait");
    SHARED_STATE.set_activity(BotActivity::FishDetected);
    SHARED_STATE.set_detail_message("Fish took the bait!");
    state.update_progress();

    let mut counter = 0;
    let mut last_print_time = Instant::now();
    let mut last_check_time = Instant::now();
    let mut lane = 0i32;

    mouse_press();

    SHARED_STATE.set_activity(BotActivity::PlayingMinigame);
    SHARED_STATE.set_detail_message("Holding click for minigame...");

    while state.is_running() {
        // Check for no progress timeout
        if state.time_since_progress().as_secs() > NO_PROGRESS_LIMIT {
            handle_no_progress_loop(state, image_service, window_title);
            return;
        }

        counter += 1;
        thread::sleep(Duration::from_millis(1000 / SPAM_CPS as u64));

        let rect = match get_window_rect(window_title) {
            Some(r) => r,
            None => continue,
        };

        // Check for arrows in minigame
        let (arrow, score) = image_service.find_minigame_arrow(Some(rect), None);

        if let Some(ref arrow_name) = arrow {
            if score > 0.8 {
                state.update_progress();
                SHARED_STATE.set_activity(BotActivity::DetectingArrow);

                if arrow_name.contains("right") {
                    lane = (lane + 1).min(1);
                    SHARED_STATE.set_activity(BotActivity::MovingRight);
                    SHARED_STATE
                        .set_detail_message(format!("Arrow RIGHT detected, lane = {}", lane));
                } else if arrow_name.contains("left") {
                    lane = (lane - 1).max(-1);
                    SHARED_STATE.set_activity(BotActivity::MovingLeft);
                    SHARED_STATE
                        .set_detail_message(format!("Arrow LEFT detected, lane = {}", lane));
                }

                thread::sleep(Duration::from_millis(200));
            }
        }

        // Handle lane movement
        match lane {
            -1 => {
                if let Some(key) = get_pykey("left_key") {
                    hold_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }
            }
            0 => {
                SHARED_STATE.set_activity(BotActivity::CenterLane);
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }
            }
            1 => {
                if let Some(key) = get_pykey("right_key") {
                    hold_key(&key);
                }
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
            }
            _ => {}
        }

        // Print tick count periodically (reduced frequency)
        if last_print_time.elapsed() >= Duration::from_secs(5) {
            tracing::trace!("[MINIGAME] {} ticks, lane={}", counter, lane);
            SHARED_STATE
                .set_detail_message(format!("Minigame: {} ticks, lane = {}", counter, lane));
            last_print_time = Instant::now();
        }

        // Check for continue button or default screen periodically
        if last_check_time.elapsed() >= Duration::from_millis(300) {
            let base = get_data_dir();
            let res_folder = get_resolution_folder();

            // Check for continue button
            let continue_path = base
                .join(TARGET_IMAGES_FOLDER)
                .join(&res_folder)
                .join("continue.png");
            let continue_hl_path = base
                .join(TARGET_IMAGES_FOLDER)
                .join(&res_folder)
                .join("continue_highlighted.png");

            let mut continue_found =
                image_service.find_image_in_window(Some(rect), &continue_path, 0.8);
            if continue_found.is_none() {
                continue_found =
                    image_service.find_image_in_window(Some(rect), &continue_hl_path, 0.8);
            }

            // Check for default screen (minigame failed)
            let default_path = base
                .join(TARGET_IMAGES_FOLDER)
                .join(&res_folder)
                .join("default_screen.png");
            let default_found = image_service.find_image_in_window(Some(rect), &default_path, 0.9);

            last_check_time = Instant::now();

            if let Some(pos) = continue_found {
                state.update_progress();
                SHARED_STATE.set_activity(BotActivity::WaitingForContinue);
                SHARED_STATE.set_detail_message("Continue button found!");

                // Save continue position
                {
                    let mut saved_pos = state.saved_continue_pos.lock();
                    if saved_pos.is_none() {
                        *saved_pos = Some(pos);
                    }
                }

                println!("Continue button found, releasing click");
                mouse_release();

                // Detect fish type
                SHARED_STATE.set_activity(BotActivity::DetectingFishType);
                SHARED_STATE.set_detail_message("Detecting fish type...");

                let fish_folder = base
                    .join(TARGET_IMAGES_FOLDER)
                    .join(&res_folder)
                    .join("fish");
                let mut fish_type: Option<String> = None;

                // Wait a bit for the fish result screen to fully render
                thread::sleep(Duration::from_millis(500));

                if fish_folder.exists() {
                    // Detection attempts
                    let max_attempts = 5;
                    for attempt in 0..max_attempts {
                        SHARED_STATE.set_detail_message(format!(
                            "Detecting fish (attempt {}/{})...",
                            attempt + 1,
                            max_attempts
                        ));
                        let (detected, score) =
                            image_service.find_best_matching_fish(Some(rect), None);
                        if let Some(ref ft) = detected {
                            // Validate detected fish against fish_config.json
                            let exists_in_config = fish_service.fish_exists(ft);
                            if !exists_in_config {
                                tracing::debug!(
                                    "[CONFIG] Fish '{}' not found in fish_config.json",
                                    ft
                                );
                            }

                            // Accept detection with score >= 0.7 (like Python version)
                            if score >= 0.7 {
                                println!("Detected fish: {} (score: {:.3})", ft, score);
                                SHARED_STATE.set_detail_message(format!(
                                    "Caught: {} ({:.0}% match)",
                                    ft,
                                    score * 100.0
                                ));
                                fish_type = Some(ft.clone());
                                break;
                            }
                        }
                        thread::sleep(Duration::from_millis(500));
                    }
                }

                // Update stats
                {
                    let mut stats = state.session_stats.lock();
                    let xp = if let Some(ref ft) = fish_type {
                        fish_service.get_xp_by_type(ft)
                    } else {
                        1
                    };
                    stats.xp += xp;
                    stats.catches += 1;
                    let total = stats.catches + stats.misses;
                    stats.rate = if total > 0 {
                        (stats.catches as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    println!(
                        "Catch recorded: catches={}, misses={}, xp={}, rate={:.1}%",
                        stats.catches, stats.misses, stats.xp, stats.rate
                    );

                    // Sync to shared state for UI
                    SHARED_STATE.update_stats(stats.catches, stats.misses, stats.xp);
                }

                // Log the catch with fish type
                log_catch(true, fish_type);

                // Click continue button with retries
                SHARED_STATE.set_activity(BotActivity::ClickingContinue);
                SHARED_STATE.set_detail_message("Clicking continue button...");

                for retry in 0..3 {
                    // Focus window before clicking
                    focus_blue_protocol_window();

                    if let Some(continue_pos) = *state.saved_continue_pos.lock() {
                        click(continue_pos.0, continue_pos.1);
                        SHARED_STATE.set_detail_message(format!("Click attempt {}/3", retry + 1));
                        thread::sleep(Duration::from_millis(500));
                    }

                    // Check if continue button is still there
                    let still_there = image_service
                        .find_image_in_window(Some(rect), &continue_path, 0.75)
                        .or_else(|| {
                            image_service.find_image_in_window(Some(rect), &continue_hl_path, 0.75)
                        });

                    if still_there.is_none() {
                        break;
                    }
                }

                // Release any held movement keys before returning
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }

                SHARED_STATE.set_activity(BotActivity::WaitingForDefaultScreen);
                SHARED_STATE.set_detail_message("Ready for next catch");
                return;
            } else if default_found.is_some() {
                println!("Minigame failed. Fish escaped.");
                SHARED_STATE.set_activity(BotActivity::MinigameFailed);
                SHARED_STATE.set_detail_message("Minigame failed, fish escaped!");
                mouse_release();

                // Release any held movement keys before returning
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }

                // Update stats
                {
                    let mut stats = state.session_stats.lock();
                    stats.misses += 1;
                    let total = stats.catches + stats.misses;
                    stats.rate = if total > 0 {
                        (stats.catches as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };

                    // Sync to shared state for UI
                    SHARED_STATE.update_stats(stats.catches, stats.misses, stats.xp);
                }

                log_catch(false, None);

                thread::sleep(Duration::from_millis(500));
                SHARED_STATE.set_activity(BotActivity::WaitingForDefaultScreen);
                SHARED_STATE.set_detail_message("Ready for next catch");
                return;
            }
        }
    }
}

/// Handle no progress timeout - recovery loop
fn handle_no_progress_loop(
    state: &Arc<MacroState>,
    image_service: &ImageService,
    window_title: &str,
) {
    tracing::debug!("[RECOVERY] No progress timeout - starting recovery");
    println!("No progress detected, performing recovery...");
    SHARED_STATE.set_activity(BotActivity::RecoveringFromTimeout);
    SHARED_STATE.set_detail_message("No progress for 45s, recovering...");

    // Release mouse button and any held movement keys before recovery
    mouse_release();
    if let Some(key) = get_pykey("left_key") {
        release_key(&key);
    }
    if let Some(key) = get_pykey("right_key") {
        release_key(&key);
    }

    let esc_key = get_pykey("esc_key");
    let fish_key = get_pykey("fish_key");

    while state.is_running() {
        let rect = match get_window_rect(window_title) {
            Some(r) => r,
            None => {
                state.set_running(false);
                SHARED_STATE.set_activity(BotActivity::Stopped);
                SHARED_STATE.set_detail_message("Game window lost");
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        // Check for default screen
        let base = get_data_dir();
        let res_folder = get_resolution_folder();
        let default_path = base
            .join(TARGET_IMAGES_FOLDER)
            .join(&res_folder)
            .join("default_screen.png");

        if image_service
            .find_image_in_window(Some(rect), &default_path, 0.9)
            .is_some()
        {
            println!("Default screen detected, stopping recovery loop.");
            SHARED_STATE.set_detail_message("Recovery successful, restarting...");
            state.update_progress();

            // Restart macro
            handle_stop_key(state);
            thread::sleep(Duration::from_millis(500));
            handle_start_key(state);
            break;
        }

        // Perform recovery actions
        SHARED_STATE.set_detail_message("Pressing ESC and fish key...");

        if let Some(ref key) = esc_key {
            press_key(key);
            thread::sleep(Duration::from_secs(1));
        }

        if let Some(ref key) = fish_key {
            press_key(key);
            thread::sleep(Duration::from_secs(1));
        }

        state.update_progress();
        thread::sleep(Duration::from_secs(1));
    }
}

/// Main fishing loop
fn main_loop(state: Arc<MacroState>, image_service: ImageService, fish_service: FishService) {
    println!("Macro waiting for START key ({:?})", get_keys().0);
    SHARED_STATE.set_activity(BotActivity::WaitingForStart);
    SHARED_STATE.set_detail_message(format!("Press {} to start", get_keys().0));

    loop {
        // Sync local state with shared state (UI can start/stop the bot)
        let shared_running = SHARED_STATE.is_running();
        let local_running = state.running.load(Ordering::SeqCst);

        // Check if UI started the bot (shared is running but local isn't initialized)
        if shared_running && !local_running && state.window_title.lock().is_none() {
            // UI started the bot, trigger start sequence
            handle_start_key(&state);
        }

        // Check if UI stopped the bot
        if !shared_running && local_running {
            // UI stopped the bot, trigger stop sequence
            handle_stop_key(&state);
        }

        if !state.is_running() {
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        let window_title = match state.window_title.lock().clone() {
            Some(t) => t,
            None => {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        // Check for no progress timeout
        let time_since_progress = state.time_since_progress().as_secs();
        if time_since_progress > NO_PROGRESS_LIMIT {
            handle_no_progress_loop(&state, &image_service, &window_title);
            continue;
        }

        let rect = match get_window_rect(&window_title) {
            Some(r) => r,
            None => {
                SHARED_STATE.set_detail_message("Waiting for game window...");
                thread::sleep(CHECK_INTERVAL);
                continue;
            }
        };

        let base = get_data_dir();
        let res_folder = get_resolution_folder();

        // Check for default screen
        let default_path = base
            .join(TARGET_IMAGES_FOLDER)
            .join(&res_folder)
            .join("default_screen.png");

        if image_service
            .find_image_in_window(Some(rect), &default_path, THRESHOLD)
            .is_some()
        {
            state.update_progress();
            SHARED_STATE.set_activity(BotActivity::WaitingForDefaultScreen);
            SHARED_STATE.set_detail_message("Fishing spot found");
            thread::sleep(Duration::from_millis(200));

            // Check for broken rod
            let broken_path = base
                .join(TARGET_IMAGES_FOLDER)
                .join(&res_folder)
                .join("broken_pole.png");

            if image_service
                .find_image_in_window(Some(rect), &broken_path, 0.9)
                .is_some()
            {
                println!("Broken pole detected -> pressing rods key");
                SHARED_STATE.set_activity(BotActivity::HandlingBrokenRod);
                SHARED_STATE.set_detail_message("Broken rod! Selecting new rod...");
                state.update_progress();

                log_broken_rod();

                if let Some(key) = get_pykey("rods_key") {
                    press_key(&key);
                }

                thread::sleep(Duration::from_millis(200));

                // Check for use rod button
                let use_rod_path = base
                    .join(TARGET_IMAGES_FOLDER)
                    .join(&res_folder)
                    .join("use_rod.png");

                if let Some(pos) =
                    image_service.find_image_in_window(Some(rect), &use_rod_path, 0.9)
                {
                    SHARED_STATE.set_activity(BotActivity::SelectingNewRod);
                    SHARED_STATE.set_detail_message("Clicking Use Rod button...");
                    state.update_progress();
                    click(pos.0, pos.1);
                    thread::sleep(Duration::from_secs(1));
                }

                continue;
            }

            // Start fishing - left click at center of window
            let center_x = rect.0 + (rect.2 - rect.0) / 2;
            let center_y = rect.1 + (rect.3 - rect.1) / 2;

            SHARED_STATE.set_activity(BotActivity::CastingLine);
            SHARED_STATE.set_detail_message("Casting fishing line...");

            click(center_x, center_y);
            state.update_progress();

            thread::sleep(Duration::from_secs(1));

            SHARED_STATE.set_activity(BotActivity::WaitingForFish);
            SHARED_STATE.set_detail_message("Waiting for fish to bite...");

            // Wait for fish to bite
            while state.is_running() {
                if state.time_since_progress().as_secs() > NO_PROGRESS_LIMIT {
                    handle_no_progress_loop(&state, &image_service, &window_title);
                    break;
                }

                let catch_path = base
                    .join(TARGET_IMAGES_FOLDER)
                    .join(&res_folder)
                    .join("catch_fish.png");

                if let Some(pos) = image_service.find_image_in_window(Some(rect), &catch_path, 0.9)
                {
                    state.update_progress();
                    mouse_move(pos.0, pos.1);
                    thread::sleep(Duration::from_millis(50));
                    post_catch_loop(&state, &image_service, &fish_service, &window_title);
                    break;
                }

                thread::sleep(CHECK_INTERVAL);
            }
        }

        thread::sleep(CHECK_INTERVAL);
    }
}

fn main() {
    // Initialize logging with file output to debug/log folder
    let base = get_data_dir();
    let log_dir = base.join("debug").join("log");
    let _ = std::fs::create_dir_all(&log_dir);

    // Create a file appender for debug logs
    let log_file_path = log_dir.join("debug.log");
    let file_result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path);

    // Configure logging with both stdout and file output
    // Use env-filter to reduce log spam from external crates
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    // Log filter configuration:
    // - Sets default level to 'info'
    // - Sets verbose external crates to 'warn' to filter out their debug/trace logs
    const LOG_FILTER: &str = "info,blue_mancing=info,reqwest=warn,hyper=warn,hyper_util=warn,wry=warn,tao=warn,mio=warn,want=warn,rustls=warn";

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(LOG_FILTER));

    match file_result {
        Ok(file) => {
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Mutex::new(file))
                .with_ansi(false)
                .with_span_events(FmtSpan::CLOSE);

            let stdout_layer = tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer)
                .with(stdout_layer)
                .init();

            tracing::info!("[INIT] Logging initialized, file: {:?}", log_file_path);
        }
        Err(e) => {
            // Fallback: stdout-only logging with same filter
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::new(LOG_FILTER))
                .init();
            eprintln!(
                "[INIT] Failed to create debug log file at {:?}: {}",
                log_file_path, e
            );
        }
    }

    println!("Blue Mancing {}", APP_VERSION);
    println!("================================");

    // Initialize shared state
    SHARED_STATE.set_activity(BotActivity::Idle);
    SHARED_STATE.set_detail_message("Initializing...");

    // Fix spelling in logs
    fix_spelling();

    // Check for updates
    SHARED_STATE.set_detail_message("Checking for updates...");
    if let Some(update) = check_for_update_blocking() {
        println!("New version available: {}", update.version);
        println!("Please download the latest version from GitHub.");
    } else {
        println!("App is up to date.");
    }

    // Initialize services
    SHARED_STATE.set_detail_message("Loading configuration...");
    // Reuse base from logging initialization above
    let config_path = base.join("config").join("fish_config.json");

    let mut fish_service = FishService::new(config_path.clone());
    if let Err(e) = fish_service.load_fishes() {
        tracing::warn!("[INIT] Failed to load fish config: {}", e);
    } else {
        println!("Fish config loaded: {} entries", fish_service.count());
    }

    let image_service = ImageService::new();
    let state = Arc::new(MacroState::new());

    // Set up hotkey manager
    let manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");

    let (start_key_str, stop_key_str) = get_keys();
    println!("Hotkeys: START={}, STOP={}", start_key_str, stop_key_str);

    // Register hotkeys
    if let Some(start_code) = string_to_code(&start_key_str) {
        let start_hotkey = HotKey::new(None, start_code);
        if let Err(e) = manager.register(start_hotkey) {
            tracing::warn!("[INIT] Failed to register start hotkey: {}", e);
        }
    }

    if let Some(stop_code) = string_to_code(&stop_key_str) {
        let stop_hotkey = HotKey::new(None, stop_code);
        if let Err(e) = manager.register(stop_hotkey) {
            tracing::warn!("[INIT] Failed to register stop hotkey: {}", e);
        }
    }

    // Clone state for hotkey handler
    let state_clone = state.clone();

    // Spawn hotkey listener thread
    let _hotkey_thread = thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();

        loop {
            if let Ok(event) = receiver.recv() {
                let (start_str, stop_str) = get_keys();

                if let Some(start_code) = string_to_code(&start_str) {
                    let start_hotkey = HotKey::new(None, start_code);
                    if event.id == start_hotkey.id() {
                        handle_start_key(&state_clone);
                    }
                }

                if let Some(stop_code) = string_to_code(&stop_str) {
                    let stop_hotkey = HotKey::new(None, stop_code);
                    if event.id == stop_hotkey.id() {
                        handle_stop_key(&state_clone);
                    }
                }
            }
        }
    });

    // Spawn main loop thread
    let state_for_main = state.clone();
    let main_thread = thread::spawn(move || {
        main_loop(state_for_main, image_service, fish_service);
    });

    // Start UI (blocks until UI closes)
    ui::start_ui();

    // Cleanup
    println!("App is closing, cleaning up...");
    handle_stop_key(&state);

    let _ = main_thread.join();
}
