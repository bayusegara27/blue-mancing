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
    tracing::info!("[START] handle_start_key() called - attempting to start macro");
    SHARED_STATE.set_activity(BotActivity::SelectingWindow);
    SHARED_STATE.set_detail_message("Looking for Blue Protocol window...");

    tracing::debug!("[START] Searching for Blue Protocol window...");
    let window_title = select_window();

    if window_title.is_none() {
        tracing::warn!("[START] No Blue Protocol window found - cannot start macro");
        println!("No window found. Cannot start macro.");
        SHARED_STATE.set_activity(BotActivity::Idle);
        SHARED_STATE.set_detail_message("No game window found");
        return;
    }

    tracing::info!("[START] Found window: {:?}", window_title);

    let mut sessions = load_sessions();
    tracing::debug!("[START] Loaded {} existing sessions", sessions.len());

    // Check if there's already an active session
    if !sessions.is_empty() && sessions.last().unwrap().stop.is_none() {
        tracing::warn!("[START] Session already active - press stop first");
        println!("Session already started. Press stop first.");
        SHARED_STATE.set_detail_message("Session already active");
        return;
    }

    // Start new session
    tracing::info!("[START] Creating new session...");
    sessions.push(Session {
        start: Utc::now().to_rfc3339(),
        stop: None,
    });
    save_sessions(&sessions);
    tracing::debug!("[START] Session saved to file");

    *state.window_title.lock() = window_title.clone();
    state.set_running(true);
    state.update_progress();

    // Reset stats for new session
    *state.session_stats.lock() = SessionStats::default();
    SHARED_STATE.reset_stats();
    tracing::debug!("[START] Stats reset for new session");

    SHARED_STATE.set_activity(BotActivity::WaitingForDefaultScreen);
    SHARED_STATE.set_detail_message(format!(
        "Connected to: {}",
        window_title.as_ref().unwrap_or(&"Unknown".to_string())
    ));

    tracing::info!(
        "[START] Macro started successfully on window: {:?}",
        window_title
    );
    println!("Macro started on window: {:?}", window_title);
}

/// Handle stop key press
fn handle_stop_key(state: &Arc<MacroState>) {
    tracing::info!("[STOP] handle_stop_key() called - attempting to stop macro");
    let mut sessions = load_sessions();

    if sessions.is_empty() || sessions.last().unwrap().stop.is_some() {
        tracing::warn!("[STOP] No active session to stop");
        println!("No active session to stop.");
        return;
    }

    // End session
    tracing::debug!("[STOP] Ending current session...");
    if let Some(last) = sessions.last_mut() {
        last.stop = Some(Utc::now().to_rfc3339());
    }
    save_sessions(&sessions);
    tracing::debug!("[STOP] Session saved to file");

    state.set_running(false);
    *state.saved_continue_pos.lock() = None;
    *state.window_title.lock() = None;
    tracing::debug!("[STOP] State cleared: running=false, continue_pos=None, window_title=None");

    SHARED_STATE.set_activity(BotActivity::Stopped);
    SHARED_STATE.set_detail_message("Bot stopped by user");

    tracing::info!("[STOP] Macro stopped successfully");
    println!("Macro stopped");
}

/// Post-catch loop - handles the fishing minigame
fn post_catch_loop(
    state: &Arc<MacroState>,
    image_service: &ImageService,
    fish_service: &FishService,
    window_title: &str,
) {
    tracing::info!("[MINIGAME] ========== FISH CAUGHT - STARTING MINIGAME ==========");
    println!("Fish took the bait");
    SHARED_STATE.set_activity(BotActivity::FishDetected);
    SHARED_STATE.set_detail_message("Fish took the bait!");
    state.update_progress();

    let mut counter = 0;
    let mut last_print_time = Instant::now();
    let mut last_check_time = Instant::now();
    let mut lane = 0i32;

    tracing::debug!("[MINIGAME] Pressing and holding left mouse button...");
    mouse_press();

    SHARED_STATE.set_activity(BotActivity::PlayingMinigame);
    SHARED_STATE.set_detail_message("Holding click for minigame...");
    tracing::info!("[MINIGAME] Minigame started - holding click, initial lane=0");

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
                tracing::info!(
                    "[MINIGAME] Arrow detected: '{}' with score={:.3}",
                    arrow_name,
                    score
                );

                if arrow_name.contains("right") {
                    let old_lane = lane;
                    lane = (lane + 1).min(1);
                    SHARED_STATE.set_activity(BotActivity::MovingRight);
                    SHARED_STATE
                        .set_detail_message(format!("Arrow RIGHT detected, lane = {}", lane));
                    tracing::debug!(
                        "[MINIGAME] RIGHT arrow: lane changed from {} to {}",
                        old_lane,
                        lane
                    );
                    println!("Right arrow detected, lane = {}", lane);
                } else if arrow_name.contains("left") {
                    let old_lane = lane;
                    lane = (lane - 1).max(-1);
                    SHARED_STATE.set_activity(BotActivity::MovingLeft);
                    SHARED_STATE
                        .set_detail_message(format!("Arrow LEFT detected, lane = {}", lane));
                    tracing::debug!(
                        "[MINIGAME] LEFT arrow: lane changed from {} to {}",
                        old_lane,
                        lane
                    );
                    println!("Left arrow detected, lane = {}", lane);
                }

                thread::sleep(Duration::from_millis(200));
            }
        }

        // Handle lane movement
        match lane {
            -1 => {
                tracing::trace!("[MINIGAME] Lane=-1: holding LEFT key, releasing RIGHT key");
                if let Some(key) = get_pykey("left_key") {
                    hold_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }
            }
            0 => {
                tracing::trace!("[MINIGAME] Lane=0: releasing both LEFT and RIGHT keys");
                SHARED_STATE.set_activity(BotActivity::CenterLane);
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }
            }
            1 => {
                tracing::trace!("[MINIGAME] Lane=1: holding RIGHT key, releasing LEFT key");
                if let Some(key) = get_pykey("right_key") {
                    hold_key(&key);
                }
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
            }
            _ => {}
        }

        // Print tick count periodically
        if last_print_time.elapsed() >= Duration::from_secs(1) {
            tracing::debug!(
                "[MINIGAME] Progress: {} ticks, current lane={}, time_since_progress={:.1}s",
                counter,
                lane,
                state.time_since_progress().as_secs_f32()
            );
            println!("Held for {} ticks", counter);
            SHARED_STATE
                .set_detail_message(format!("Minigame: {} ticks, lane = {}", counter, lane));
            last_print_time = Instant::now();
        }

        // Check for continue button or default screen periodically
        if last_check_time.elapsed() >= Duration::from_millis(300) {
            tracing::trace!("[MINIGAME] Checking for continue button and default screen...");
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
                tracing::info!(
                    "[MINIGAME] ========== CONTINUE BUTTON FOUND at ({}, {}) ==========",
                    pos.0,
                    pos.1
                );
                state.update_progress();
                SHARED_STATE.set_activity(BotActivity::WaitingForContinue);
                SHARED_STATE.set_detail_message("Continue button found!");

                // Save continue position (use a block to drop the lock immediately)
                {
                    let mut saved_pos = state.saved_continue_pos.lock();
                    if saved_pos.is_none() {
                        tracing::debug!(
                            "[MINIGAME] Saving continue button position: ({}, {})",
                            pos.0,
                            pos.1
                        );
                        *saved_pos = Some(pos);
                    } else {
                        tracing::debug!(
                            "[MINIGAME] Using previously saved continue button position"
                        );
                    }
                }

                tracing::debug!("[MINIGAME] Releasing mouse button...");
                println!("Continue button found, releasing click");
                mouse_release();

                // Detect fish type
                tracing::info!("[FISH] Starting fish type detection...");
                SHARED_STATE.set_activity(BotActivity::DetectingFishType);
                SHARED_STATE.set_detail_message("Detecting fish type...");

                let fish_folder = base
                    .join(TARGET_IMAGES_FOLDER)
                    .join(&res_folder)
                    .join("fish");
                let mut fish_type: Option<String> = None;

                if fish_folder.exists() {
                    tracing::debug!("[FISH] Fish folder exists: {:?}", fish_folder);
                    for attempt in 0..3 {
                        tracing::debug!("[FISH] Detection attempt {}/3...", attempt + 1);
                        println!("[FISH] Detection attempt {}/3...", attempt + 1);
                        SHARED_STATE.set_detail_message(format!(
                            "Detecting fish (attempt {}/3)...",
                            attempt + 1
                        ));
                        let (detected, score) =
                            image_service.find_best_matching_fish(Some(rect), None);
                        if let Some(ref ft) = detected {
                            tracing::info!("[FISH] Found match: '{}' with score={:.3}", ft, score);
                            println!("[FISH] Found match: '{}' with score={:.3}", ft, score);
                            // Lower threshold from 0.7 to 0.6 for better detection
                            if score >= 0.6 {
                                tracing::info!(
                                    "[FISH] Fish detected: '{}' (score: {:.3})",
                                    ft,
                                    score
                                );
                                println!(
                                    "[FISH] ✓ Detected fish type: {} (score: {:.3})",
                                    ft, score
                                );
                                SHARED_STATE.set_detail_message(format!(
                                    "Caught: {} ({:.0}% match)",
                                    ft,
                                    score * 100.0
                                ));
                                fish_type = Some(ft.clone());
                                break;
                            } else {
                                tracing::debug!(
                                    "[FISH] Score {:.3} below threshold 0.6, retrying...",
                                    score
                                );
                                println!(
                                    "[FISH] Score {:.3} below threshold 0.6, retrying...",
                                    score
                                );
                            }
                        } else {
                            tracing::debug!(
                                "[FISH] No fish match found on attempt {}",
                                attempt + 1
                            );
                            println!("[FISH] No fish match found on attempt {}", attempt + 1);
                        }
                        thread::sleep(Duration::from_millis(200));
                    }

                    // Log final result
                    if fish_type.is_none() {
                        tracing::warn!("[FISH] Failed to detect fish type after 3 attempts");
                        println!("[FISH] ✗ Failed to detect fish type after 3 attempts");
                    }
                } else {
                    tracing::warn!("[FISH] Fish folder does not exist: {:?}", fish_folder);
                    println!("[FISH] ✗ Fish folder does not exist: {:?}", fish_folder);
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
                    tracing::info!(
                        "[STATS] Catch recorded: catches={}, misses={}, xp={}, rate={:.1}%",
                        stats.catches,
                        stats.misses,
                        stats.xp,
                        stats.rate
                    );
                    println!(
                        "[STATS] Catch recorded: catches={}, misses={}, xp={}, rate={:.1}%",
                        stats.catches, stats.misses, stats.xp, stats.rate
                    );

                    // Sync to shared state for UI
                    SHARED_STATE.update_stats(stats.catches, stats.misses, stats.xp);
                }

                // Log the catch with fish type
                tracing::info!("[LOG] Logging catch to file: fish_type={:?}", fish_type);
                println!("[LOG] Logging catch to file: fish_type={:?}", fish_type);
                log_catch(true, fish_type);

                // Click continue button with retries
                tracing::info!("[CLICK] Starting continue button click sequence...");
                SHARED_STATE.set_activity(BotActivity::ClickingContinue);
                SHARED_STATE.set_detail_message("Clicking continue button...");

                for retry in 0..3 {
                    tracing::debug!("[CLICK] Attempt {}/3: focusing window...", retry + 1);
                    // Focus window before clicking to ensure click goes to the right place
                    focus_blue_protocol_window();

                    if let Some(continue_pos) = *state.saved_continue_pos.lock() {
                        tracing::info!(
                            "[CLICK] Clicking continue button at ({}, {}) - attempt {}/3",
                            continue_pos.0,
                            continue_pos.1,
                            retry + 1
                        );
                        click(continue_pos.0, continue_pos.1);
                        SHARED_STATE.set_detail_message(format!("Click attempt {}/3", retry + 1));
                        thread::sleep(Duration::from_millis(500));
                    } else {
                        tracing::warn!("[CLICK] No saved continue position available!");
                    }

                    // Check if continue button is still there
                    tracing::debug!("[CLICK] Checking if continue button is still visible...");
                    let still_there = image_service
                        .find_image_in_window(Some(rect), &continue_path, 0.75)
                        .or_else(|| {
                            image_service.find_image_in_window(Some(rect), &continue_hl_path, 0.75)
                        });

                    if still_there.is_none() {
                        tracing::info!(
                            "[CLICK] Continue button no longer visible - click successful!"
                        );
                        break;
                    } else {
                        tracing::debug!("[CLICK] Continue button still visible, retrying...");
                    }
                }

                // Release any held movement keys before returning
                tracing::debug!("[CLEANUP] Releasing held movement keys...");
                if let Some(key) = get_pykey("left_key") {
                    release_key(&key);
                }
                if let Some(key) = get_pykey("right_key") {
                    release_key(&key);
                }

                tracing::info!("[MINIGAME] ========== MINIGAME COMPLETE - SUCCESS ==========");
                SHARED_STATE.set_activity(BotActivity::WaitingForDefaultScreen);
                SHARED_STATE.set_detail_message("Ready for next catch");
                return;
            } else if default_found.is_some() {
                tracing::info!("[MINIGAME] ========== MINIGAME FAILED - FISH ESCAPED ==========");
                println!("Default screen detected, minigame failed. Releasing click.");
                SHARED_STATE.set_activity(BotActivity::MinigameFailed);
                SHARED_STATE.set_detail_message("Minigame failed, fish escaped!");
                mouse_release();

                // Release any held movement keys before returning
                tracing::debug!("[CLEANUP] Releasing held movement keys...");
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
                    tracing::info!(
                        "[STATS] Miss recorded: catches={}, misses={}, xp={}, rate={:.1}%",
                        stats.catches,
                        stats.misses,
                        stats.xp,
                        stats.rate
                    );

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
    tracing::warn!("[RECOVERY] ========== NO PROGRESS TIMEOUT - STARTING RECOVERY ==========");
    tracing::warn!(
        "[RECOVERY] No progress for {} seconds, initiating recovery sequence",
        NO_PROGRESS_LIMIT
    );
    println!("No progress detected, performing recovery...");
    SHARED_STATE.set_activity(BotActivity::RecoveringFromTimeout);
    SHARED_STATE.set_detail_message("No progress for 45s, recovering...");

    // Release mouse button and any held movement keys before recovery
    tracing::debug!("[RECOVERY] Releasing mouse button and movement keys...");
    mouse_release();
    if let Some(key) = get_pykey("left_key") {
        release_key(&key);
    }
    if let Some(key) = get_pykey("right_key") {
        release_key(&key);
    }

    let esc_key = get_pykey("esc_key");
    let fish_key = get_pykey("fish_key");
    tracing::debug!(
        "[RECOVERY] Recovery keys: esc_key={:?}, fish_key={:?}",
        esc_key,
        fish_key
    );

    let mut recovery_attempt = 0;
    while state.is_running() {
        recovery_attempt += 1;
        tracing::info!("[RECOVERY] Recovery attempt #{}", recovery_attempt);

        let rect = match get_window_rect(window_title) {
            Some(r) => {
                tracing::debug!("[RECOVERY] Window rect: {:?}", r);
                r
            }
            None => {
                tracing::error!("[RECOVERY] Game window lost! Stopping bot.");
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

        tracing::debug!("[RECOVERY] Checking for default screen...");
        if image_service
            .find_image_in_window(Some(rect), &default_path, 0.9)
            .is_some()
        {
            tracing::info!("[RECOVERY] Default screen detected - recovery successful!");
            println!("Default screen detected, stopping recovery loop.");
            SHARED_STATE.set_detail_message("Recovery successful, restarting...");
            state.update_progress();

            // Restart macro
            tracing::info!("[RECOVERY] Restarting macro...");
            handle_stop_key(state);
            thread::sleep(Duration::from_millis(500));
            handle_start_key(state);
            break;
        }

        // Perform recovery actions
        tracing::debug!("[RECOVERY] Default screen not found, performing recovery actions...");
        println!("No progress detected, performing recovery actions...");
        SHARED_STATE.set_detail_message("Pressing ESC and fish key...");

        if let Some(ref key) = esc_key {
            tracing::debug!("[RECOVERY] Pressing ESC key: '{}'", key);
            press_key(key);
            thread::sleep(Duration::from_secs(1));
        }

        if let Some(ref key) = fish_key {
            tracing::debug!("[RECOVERY] Pressing fish key: '{}'", key);
            press_key(key);
            thread::sleep(Duration::from_secs(1));
        }

        state.update_progress();
        thread::sleep(Duration::from_secs(1));
    }
    tracing::info!("[RECOVERY] ========== RECOVERY LOOP ENDED ==========");
}

/// Main fishing loop
fn main_loop(state: Arc<MacroState>, image_service: ImageService, fish_service: FishService) {
    tracing::info!("[MAIN] ========== MAIN FISHING LOOP STARTED ==========");
    tracing::info!("[MAIN] Waiting for START key: {:?}", get_keys().0);
    println!("Macro waiting for START key ({:?})", get_keys().0);
    SHARED_STATE.set_activity(BotActivity::WaitingForStart);
    SHARED_STATE.set_detail_message(format!("Press {} to start", get_keys().0));

    let mut loop_counter: u64 = 0;
    loop {
        loop_counter += 1;

        // Sync local state with shared state (UI can start/stop the bot)
        let shared_running = SHARED_STATE.is_running();
        let local_running = state.running.load(Ordering::SeqCst);

        // Check if UI started the bot (shared is running but local isn't initialized)
        if shared_running && !local_running && state.window_title.lock().is_none() {
            tracing::info!("[MAIN] UI triggered start - initializing...");
            // UI started the bot, trigger start sequence
            handle_start_key(&state);
        }

        // Check if UI stopped the bot
        if !shared_running && local_running {
            tracing::info!("[MAIN] UI triggered stop - shutting down...");
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
                tracing::trace!(
                    "[MAIN] Loop #{}: No window title set, waiting...",
                    loop_counter
                );
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        // Check for no progress timeout
        let time_since_progress = state.time_since_progress().as_secs();
        if time_since_progress > NO_PROGRESS_LIMIT {
            tracing::warn!(
                "[MAIN] No progress timeout! time_since_progress={}s > limit={}s",
                time_since_progress,
                NO_PROGRESS_LIMIT
            );
            handle_no_progress_loop(&state, &image_service, &window_title);
            continue;
        }

        let rect = match get_window_rect(&window_title) {
            Some(r) => r,
            None => {
                tracing::debug!(
                    "[MAIN] Loop #{}: Window not found, waiting...",
                    loop_counter
                );
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
            tracing::info!("[MAIN] Default screen detected - at fishing spot");
            println!("Default screen detected");
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
                tracing::warn!("[MAIN] Broken rod detected!");
                println!("Broken pole detected -> pressing rods key");
                SHARED_STATE.set_activity(BotActivity::HandlingBrokenRod);
                SHARED_STATE.set_detail_message("Broken rod! Selecting new rod...");
                state.update_progress();

                log_broken_rod();
                tracing::debug!("[MAIN] Broken rod logged to file");

                if let Some(key) = get_pykey("rods_key") {
                    tracing::debug!("[MAIN] Pressing rods key: '{}'", key);
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
                    tracing::info!(
                        "[MAIN] Use Rod button found at ({}, {}), clicking...",
                        pos.0,
                        pos.1
                    );
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

            tracing::info!(
                "[MAIN] Casting fishing line at center ({}, {})",
                center_x,
                center_y
            );
            SHARED_STATE.set_activity(BotActivity::CastingLine);
            SHARED_STATE.set_detail_message("Casting fishing line...");

            click(center_x, center_y);
            println!("Started fishing -> waiting for catch_fish.png");
            state.update_progress();

            thread::sleep(Duration::from_secs(1));

            tracing::info!("[MAIN] Waiting for fish to bite...");
            SHARED_STATE.set_activity(BotActivity::WaitingForFish);
            SHARED_STATE.set_detail_message("Waiting for fish to bite...");

            // Wait for fish to bite
            let mut wait_counter = 0;
            while state.is_running() {
                wait_counter += 1;

                if state.time_since_progress().as_secs() > NO_PROGRESS_LIMIT {
                    tracing::warn!("[MAIN] Timeout while waiting for fish!");
                    handle_no_progress_loop(&state, &image_service, &window_title);
                    break;
                }

                let catch_path = base
                    .join(TARGET_IMAGES_FOLDER)
                    .join(&res_folder)
                    .join("catch_fish.png");

                if let Some(pos) = image_service.find_image_in_window(Some(rect), &catch_path, 0.9)
                {
                    tracing::info!(
                        "[MAIN] Fish detected! catch_fish.png found at ({}, {})",
                        pos.0,
                        pos.1
                    );
                    tracing::debug!("[MAIN] Waited {} iterations for fish to bite", wait_counter);
                    state.update_progress();
                    tracing::debug!("[MAIN] Moving mouse to fish position...");
                    mouse_move(pos.0, pos.1);
                    thread::sleep(Duration::from_millis(50));
                    post_catch_loop(&state, &image_service, &fish_service, &window_title);
                    break;
                }

                if wait_counter % 100 == 0 {
                    tracing::trace!(
                        "[MAIN] Still waiting for fish... iteration #{}",
                        wait_counter
                    );
                }

                thread::sleep(CHECK_INTERVAL);
            }
        }

        thread::sleep(CHECK_INTERVAL);
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("========================================");
    tracing::info!("Blue Mancing {} - Starting up", APP_VERSION);
    tracing::info!("========================================");
    println!("Blue Mancing {}", APP_VERSION);
    println!("================================");

    // Initialize shared state
    tracing::debug!("[INIT] Initializing shared state...");
    SHARED_STATE.set_activity(BotActivity::Idle);
    SHARED_STATE.set_detail_message("Initializing...");

    // Fix spelling in logs
    tracing::debug!("[INIT] Running spelling fix...");
    fix_spelling();

    // Check for updates
    tracing::info!("[INIT] Checking for updates...");
    SHARED_STATE.set_detail_message("Checking for updates...");
    if let Some(update) = check_for_update_blocking() {
        tracing::info!("[INIT] New version available: {}", update.version);
        println!("New version available: {}", update.version);
        // In full implementation, would show update UI and download
        // For now, just inform the user
        println!("Please download the latest version from GitHub.");
    } else {
        tracing::info!("[INIT] App is up to date");
        println!("App is up to date.");
    }

    // Initialize services
    tracing::info!("[INIT] Loading configuration...");
    SHARED_STATE.set_detail_message("Loading configuration...");
    let base = get_data_dir();
    let config_path = base.join("config").join("fish_config.json");
    tracing::debug!("[INIT] Config path: {:?}", config_path);

    let mut fish_service = FishService::new(config_path);
    if let Err(e) = fish_service.load_fishes() {
        tracing::warn!("[INIT] Failed to load fish config: {}", e);
    } else {
        tracing::info!("[INIT] Fish config loaded successfully");
    }

    tracing::debug!("[INIT] Creating ImageService...");
    let image_service = ImageService::new();
    let state = Arc::new(MacroState::new());
    tracing::debug!("[INIT] MacroState initialized");

    // Set up hotkey manager
    tracing::info!("[INIT] Setting up hotkey manager...");
    let manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");

    let (start_key_str, stop_key_str) = get_keys();
    tracing::info!(
        "[INIT] Hotkeys configured: START='{}', STOP='{}'",
        start_key_str,
        stop_key_str
    );

    // Register hotkeys
    if let Some(start_code) = string_to_code(&start_key_str) {
        let start_hotkey = HotKey::new(None, start_code);
        if let Err(e) = manager.register(start_hotkey) {
            tracing::warn!(
                "[INIT] Failed to register start hotkey '{}': {}",
                start_key_str,
                e
            );
        } else {
            tracing::debug!(
                "[INIT] Start hotkey '{}' registered successfully",
                start_key_str
            );
        }
    } else {
        tracing::warn!(
            "[INIT] Could not convert start key '{}' to hotkey code",
            start_key_str
        );
    }

    if let Some(stop_code) = string_to_code(&stop_key_str) {
        let stop_hotkey = HotKey::new(None, stop_code);
        if let Err(e) = manager.register(stop_hotkey) {
            tracing::warn!(
                "[INIT] Failed to register stop hotkey '{}': {}",
                stop_key_str,
                e
            );
        } else {
            tracing::debug!(
                "[INIT] Stop hotkey '{}' registered successfully",
                stop_key_str
            );
        }
    } else {
        tracing::warn!(
            "[INIT] Could not convert stop key '{}' to hotkey code",
            stop_key_str
        );
    }

    // Clone state for hotkey handler
    let state_clone = state.clone();

    // Spawn hotkey listener thread
    tracing::info!("[INIT] Spawning hotkey listener thread...");
    let _hotkey_thread = thread::spawn(move || {
        tracing::debug!("[HOTKEY] Hotkey listener thread started");
        let receiver = GlobalHotKeyEvent::receiver();

        loop {
            if let Ok(event) = receiver.recv() {
                tracing::debug!("[HOTKEY] Hotkey event received: id={}", event.id);
                let (start_str, stop_str) = get_keys();

                if let Some(start_code) = string_to_code(&start_str) {
                    let start_hotkey = HotKey::new(None, start_code);
                    if event.id == start_hotkey.id() {
                        tracing::info!("[HOTKEY] START key pressed");
                        handle_start_key(&state_clone);
                    }
                }

                if let Some(stop_code) = string_to_code(&stop_str) {
                    let stop_hotkey = HotKey::new(None, stop_code);
                    if event.id == stop_hotkey.id() {
                        tracing::info!("[HOTKEY] STOP key pressed");
                        handle_stop_key(&state_clone);
                    }
                }
            }
        }
    });

    // Spawn main loop thread
    tracing::info!("[INIT] Spawning main fishing loop thread...");
    let state_for_main = state.clone();
    let main_thread = thread::spawn(move || {
        main_loop(state_for_main, image_service, fish_service);
    });

    // Start UI (blocks until UI closes)
    tracing::info!("[INIT] Starting UI - this will block until UI closes");
    tracing::info!("========================================");
    tracing::info!("INITIALIZATION COMPLETE - Bot ready!");
    tracing::info!("========================================");
    ui::start_ui();

    // Cleanup
    tracing::info!("[SHUTDOWN] UI closed, starting cleanup...");
    println!("App is closing, cleaning up...");
    handle_stop_key(&state);

    // Wait for main thread (though UI blocking means this won't execute until after UI close)
    tracing::debug!("[SHUTDOWN] Waiting for main thread to finish...");
    let _ = main_thread.join();
    tracing::info!("[SHUTDOWN] Cleanup complete, exiting.");
}
