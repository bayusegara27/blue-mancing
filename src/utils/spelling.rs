//! Spelling corrections for fish names in logs

use crate::utils::path::get_data_dir;
use serde_json::Value;
use std::fs;

/// Fix spelling mistakes in fishing logs
pub fn fix_spelling() {
    let filename = get_data_dir().join("logs").join("fishing_log.json");

    if !filename.exists() {
        tracing::info!("No log file found at {:?}", filename);
        return;
    }

    let content = match fs::read_to_string(&filename) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to read log file: {}", e);
            return;
        }
    };

    let data: Value = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Invalid JSON file, cannot fix: {}", e);
            return;
        }
    };

    let fixed_data = fix_item(data);

    if let Ok(output) = serde_json::to_string_pretty(&fixed_data) {
        if let Err(e) = fs::write(&filename, output) {
            tracing::warn!("Failed to write fixed log file: {}", e);
        } else {
            tracing::info!("Fixed naming issues in: {:?}", filename);
        }
    }
}

/// Apply corrections to a value
fn fix_item(item: Value) -> Value {
    match item {
        Value::String(s) => Value::String(correct_text(&s)),
        Value::Array(arr) => Value::Array(arr.into_iter().map(fix_item).collect()),
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| {
                    let fixed_v = if v.is_string() {
                        Value::String(correct_text(v.as_str().unwrap()))
                    } else {
                        fix_item(v)
                    };
                    (k, fixed_v)
                })
                .collect(),
        ),
        other => other,
    }
}

/// Apply spelling corrections to text
fn correct_text(text: &str) -> String {
    let corrections = [("astercad", "asterscad"), ("aluminium", "aluminum")];

    let mut result = text.to_string();
    for (wrong, right) in corrections {
        result = result.replace(wrong, right);
        // Also handle capitalized versions
        let wrong_cap = capitalize_first(wrong);
        let right_cap = capitalize_first(right);
        result = result.replace(&wrong_cap, &right_cap);
    }
    result
}

/// Capitalize first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correct_text() {
        assert_eq!(correct_text("astercad"), "asterscad");
        assert_eq!(correct_text("Astercad"), "Asterscad");
        assert_eq!(correct_text("aluminium"), "aluminum");
    }
}
