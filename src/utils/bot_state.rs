//! Shared bot state for communication between UI and macro threads

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use serde::Serialize;

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
}

impl SharedBotState {
    fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            activity: RwLock::new(BotActivity::Idle),
            stats: RwLock::new(SharedStats::default()),
            detail_message: RwLock::new(String::new()),
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
        }).to_string()
    }
}

/// Global shared state instance
pub static SHARED_STATE: Lazy<SharedBotState> = Lazy::new(SharedBotState::new);
