//! Fish service for loading and querying fish data

#![allow(dead_code)]

use super::base::{Fish, Rarity};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Fish configuration file structure
#[derive(Debug, Deserialize)]
struct FishConfig {
    fishes: Vec<Fish>,
}

/// Service for managing fish data
pub struct FishService {
    config_path: PathBuf,
    fishes: Vec<Fish>,
}

impl FishService {
    /// Create a new fish service
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            fishes: Vec::new(),
        }
    }

    /// Load fish data from config file
    pub fn load_fishes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(&self.config_path)?;
        let config: FishConfig = serde_json::from_str(&content)?;
        self.fishes = config.fishes;
        Ok(())
    }

    /// Get all fish
    pub fn get_all(&self) -> &[Fish] {
        &self.fishes
    }

    /// Get fish by rarity
    pub fn get_by_rarity(&self, rarity: Rarity) -> Vec<&Fish> {
        self.fishes.iter().filter(|f| f.rarity == rarity).collect()
    }

    /// Get XP value for a given fish name or ID
    pub fn get_xp_by_type(&self, fish_type: &str) -> i32 {
        let fish_type_lower = fish_type.to_lowercase();
        for fish in &self.fishes {
            if fish.name.to_lowercase() == fish_type_lower || fish.id == fish_type {
                return fish.xp;
            }
        }
        0
    }

    /// Get fish by name
    pub fn get_by_name(&self, name: &str) -> Option<&Fish> {
        let name_lower = name.to_lowercase();
        self.fishes
            .iter()
            .find(|f| f.name.to_lowercase() == name_lower)
    }

    /// Get fish by ID
    pub fn get_by_id(&self, id: &str) -> Option<&Fish> {
        let id_lower = id.to_lowercase();
        self.fishes.iter().find(|f| f.id.to_lowercase() == id_lower)
    }

    /// Check if a fish exists by ID or name
    pub fn fish_exists(&self, fish_type: &str) -> bool {
        let fish_type_lower = fish_type.to_lowercase();
        self.fishes.iter().any(|f| {
            f.id.to_lowercase() == fish_type_lower || f.name.to_lowercase() == fish_type_lower
        })
    }

    /// Get total number of fish in config
    pub fn count(&self) -> usize {
        self.fishes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fish_service_empty() {
        let service = FishService::new(PathBuf::from("nonexistent.json"));
        assert!(service.get_all().is_empty());
    }
}
