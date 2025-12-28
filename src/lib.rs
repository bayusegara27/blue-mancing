//! Blue Mancing - Auto fishing bot for Blue Protocol: Star Resonance
//!
//! This application automates fishing in the game Blue Protocol: Star Resonance.
//! It uses screen capture and template matching to detect game states and
//! simulates mouse/keyboard input to control the fishing minigame.

pub mod fish;
pub mod input;
pub mod log_main;
pub mod screen_reader;
pub mod ui;
pub mod utils;
pub mod window;

// Re-exports for convenience
pub use fish::{Fish, FishService, Rarity};
pub use screen_reader::{get_resolution_folder, ImageService, ScreenService};
pub use ui::{start_ui, OverviewApi, StatsApi, Window};
pub use utils::{bot_state, keybinds, path::get_data_dir, spelling, updater};
