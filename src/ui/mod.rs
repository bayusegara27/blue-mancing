//! UI module for the fishing bot

#![allow(unused_imports)]

pub mod ui_service;
pub mod stats_api;
pub mod overview_api;

pub use ui_service::{start_ui, Window, get_window};
pub use stats_api::StatsApi;
pub use overview_api::OverviewApi;
