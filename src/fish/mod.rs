//! Fish module for managing fish data

#![allow(unused_imports)]

pub mod base;
pub mod fish_service;

pub use base::{Fish, Rarity};
pub use fish_service::FishService;
