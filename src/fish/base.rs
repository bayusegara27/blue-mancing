//! Base types for fish data

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Fish rarity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Rarity {
    Common,
    Rare,
    Mythical,
}

impl Rarity {
    /// Get display value
    pub fn value(&self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Rare => "Rare",
            Rarity::Mythical => "Mythical",
        }
    }
}

impl std::fmt::Display for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Fish category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Fish,
    SeaCreature,
    Trash,
}

/// Represents a fish that can be caught
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fish {
    pub id: String,
    pub image: String,
    pub name: String,
    pub xp: i32,
    pub rarity: Rarity,
    #[serde(default)]
    pub category: Option<Category>,
}

impl Fish {
    /// Create a new fish
    pub fn new(id: String, image: String, name: String, xp: i32, rarity: Rarity) -> Self {
        Self {
            id,
            image,
            name,
            xp,
            rarity,
            category: None,
        }
    }
}

impl std::fmt::Display for Fish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}, XP: {})", self.name, self.rarity, self.xp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rarity_value() {
        assert_eq!(Rarity::Common.value(), "Common");
        assert_eq!(Rarity::Rare.value(), "Rare");
        assert_eq!(Rarity::Mythical.value(), "Mythical");
    }

    #[test]
    fn test_fish_display() {
        let fish = Fish::new(
            "test".to_string(),
            "test.png".to_string(),
            "Test Fish".to_string(),
            10,
            Rarity::Rare,
        );
        assert_eq!(format!("{}", fish), "Test Fish (Rare, XP: 10)");
    }
}
