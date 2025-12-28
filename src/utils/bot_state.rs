//! Shared bot state for communication between UI and macro threads

#![allow(dead_code)]

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

/// A detected region for visualization (ESP-like box)
#[derive(Debug, Clone, Serialize)]
pub struct DetectionBox {
    /// X coordinate (screen coordinates)
    pub x: i32,
    /// Y coordinate (screen coordinates)
    pub y: i32,
    /// Width of the box
    pub width: i32,
    /// Height of the box
    pub height: i32,
    /// Label for the detection (e.g., "fish", "continue_btn", "arrow")
    pub label: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Color in hex format (e.g., "#FF0000" for red)
    pub color: String,
}

/// Bot activity status for detailed status display
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum BotActivity {
    Idle,
    WaitingForStart,
    SelectingWindow,
    WaitingForDefaultScreen,
    CastingLine,
    WaitingForFish,
    FishDetected,
    PlayingMinigame,
    DetectingArrow,
    MovingLeft,
    MovingRight,
    CenterLane,
    WaitingForContinue,
    ClickingContinue,
    DetectingFishType,
    RecoveringFromTimeout,
    HandlingBrokenRod,
    SelectingNewRod,
    MinigameFailed,
    Stopped,
}

impl BotActivity {
    /// Get human-readable description of the activity
    pub fn description(&self) -> &'static str {
        match self {
            BotActivity::Idle => "Idle",
            BotActivity::WaitingForStart => "Waiting for Start (F9)",
            BotActivity::SelectingWindow => "Selecting game window...",
            BotActivity::WaitingForDefaultScreen => "Looking for fishing spot...",
            BotActivity::CastingLine => "Casting fishing line...",
            BotActivity::WaitingForFish => "Waiting for fish to bite...",
            BotActivity::FishDetected => "Fish detected! Starting minigame...",
            BotActivity::PlayingMinigame => "Playing fishing minigame...",
            BotActivity::DetectingArrow => "Detecting arrow direction...",
            BotActivity::MovingLeft => "Moving LEFT in minigame",
            BotActivity::MovingRight => "Moving RIGHT in minigame",
            BotActivity::CenterLane => "Holding CENTER lane",
            BotActivity::WaitingForContinue => "Waiting for continue button...",
            BotActivity::ClickingContinue => "Clicking continue button...",
            BotActivity::DetectingFishType => "Detecting fish type...",
            BotActivity::RecoveringFromTimeout => "Recovering from timeout...",
            BotActivity::HandlingBrokenRod => "Broken rod detected!",
            BotActivity::SelectingNewRod => "Selecting new fishing rod...",
            BotActivity::MinigameFailed => "Minigame failed, restarting...",
            BotActivity::Stopped => "Bot stopped",
        }
    }
}

/// Session statistics shared between UI and macro
#[derive(Debug, Clone, Default, Serialize)]
pub struct SharedStats {
    pub catches: i32,
    pub misses: i32,
    pub xp: i32,
    pub rate: f64,
}

/// Global shared bot state
pub struct SharedBotState {
    running: AtomicBool,
    activity: RwLock<BotActivity>,
    stats: RwLock<SharedStats>,
    detail_message: RwLock<String>,
    /// Detection boxes for ESP-like visualization
    detection_boxes: RwLock<Vec<DetectionBox>>,
    /// Game window rectangle (x, y, width, height)
    game_window_rect: RwLock<Option<(i32, i32, i32, i32)>>,
}

impl SharedBotState {
    fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            activity: RwLock::new(BotActivity::Idle),
            stats: RwLock::new(SharedStats::default()),
            detail_message: RwLock::new(String::new()),
            detection_boxes: RwLock::new(Vec::new()),
            game_window_rect: RwLock::new(None),
        }
    }

    /// Check if bot is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Set bot running state
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
        if running {
            self.set_activity(BotActivity::WaitingForDefaultScreen);
        } else {
            self.set_activity(BotActivity::Stopped);
        }
    }

    /// Get current activity
    pub fn get_activity(&self) -> BotActivity {
        self.activity.read().clone()
    }

    /// Set current activity
    pub fn set_activity(&self, activity: BotActivity) {
        *self.activity.write() = activity;
    }

    /// Get detailed message
    pub fn get_detail_message(&self) -> String {
        self.detail_message.read().clone()
    }

    /// Set detailed message
    pub fn set_detail_message(&self, message: impl Into<String>) {
        *self.detail_message.write() = message.into();
    }

    /// Get current stats
    pub fn get_stats(&self) -> SharedStats {
        self.stats.read().clone()
    }

    /// Update stats
    pub fn update_stats(&self, catches: i32, misses: i32, xp: i32) {
        let mut stats = self.stats.write();
        stats.catches = catches;
        stats.misses = misses;
        stats.xp = xp;
        let total = catches + misses;
        stats.rate = if total > 0 {
            (catches as f64 / total as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Increment catches
    pub fn increment_catch(&self, xp_gain: i32) {
        let mut stats = self.stats.write();
        stats.catches += 1;
        stats.xp += xp_gain;
        let total = stats.catches + stats.misses;
        stats.rate = if total > 0 {
            (stats.catches as f64 / total as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Increment misses
    pub fn increment_miss(&self) {
        let mut stats = self.stats.write();
        stats.misses += 1;
        let total = stats.catches + stats.misses;
        stats.rate = if total > 0 {
            (stats.catches as f64 / total as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Reset stats for new session
    pub fn reset_stats(&self) {
        *self.stats.write() = SharedStats::default();
    }

    /// Add a detection box for visualization
    pub fn add_detection_box(&self, detection: DetectionBox) {
        self.detection_boxes.write().push(detection);
    }

    /// Clear all detection boxes
    pub fn clear_detection_boxes(&self) {
        self.detection_boxes.write().clear();
    }

    /// Get all detection boxes
    pub fn get_detection_boxes(&self) -> Vec<DetectionBox> {
        self.detection_boxes.read().clone()
    }

    /// Set detection boxes (replaces all)
    pub fn set_detection_boxes(&self, boxes: Vec<DetectionBox>) {
        *self.detection_boxes.write() = boxes;
    }

    /// Set game window rectangle
    pub fn set_game_window_rect(&self, rect: Option<(i32, i32, i32, i32)>) {
        *self.game_window_rect.write() = rect;
    }

    /// Get game window rectangle
    pub fn get_game_window_rect(&self) -> Option<(i32, i32, i32, i32)> {
        *self.game_window_rect.read()
    }

    /// Get status as JSON string for UI
    pub fn to_json(&self) -> String {
        let stats = self.get_stats();
        let activity = self.get_activity();
        let detail = self.get_detail_message();

        serde_json::json!({
            "running": self.is_running(),
            "activity": activity.description(),
            "detail": detail,
            "stats": {
                "catches": stats.catches,
                "misses": stats.misses,
                "xp": stats.xp,
                "rate": format!("{:.2}", stats.rate)
            }
        })
        .to_string()
    }

    /// Get detection boxes as JSON string for ESP overlay
    pub fn detection_boxes_to_json(&self) -> String {
        let boxes = self.get_detection_boxes();
        let window_rect = self.get_game_window_rect();

        serde_json::json!({
            "boxes": boxes,
            "window": window_rect
        })
        .to_string()
    }
}

/// Global shared state instance
pub static SHARED_STATE: Lazy<SharedBotState> = Lazy::new(SharedBotState::new);
