//! Screen reader module for capturing and analyzing screen content

#![allow(unused_imports)]

pub mod base;
pub mod screen_service;
pub mod image_service;

pub use base::{get_resolution_folder, get_settings, Settings, DEFAULT_SETTINGS};
pub use screen_service::ScreenService;
pub use image_service::ImageService;
