//! BPSR Fishing - Auto fishing bot for Blue Protocol: Star Resonance
//! 
//! This application automates fishing in the game Blue Protocol: Star Resonance.
//! It uses screen capture and template matching to detect game states and
//! simulates mouse/keyboard input to control the fishing minigame.

pub mod fish;
pub mod screen_reader;
pub mod ui;
pub mod utils;
pub mod log_main;
pub mod input;
pub mod window;

// Re-exports for convenience
pub use fish::{Fish, Rarity, FishService};
pub use screen_reader::{ScreenService, ImageService, get_resolution_folder};
pub use ui::{start_ui, Window, StatsApi, OverviewApi};
pub use utils::{path::get_data_dir, keybinds, updater, spelling, bot_state};
