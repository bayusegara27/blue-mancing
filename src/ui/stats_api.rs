//! Stats API for the fishing bot UI

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Timelike};

use crate::utils::path::get_data_dir;
use crate::utils::keybinds::get_key;
use crate::fish::FishService;

/// Log entry for a fishing catch
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FishLogEntry {
    #[serde(default)]
    pub timestamp: String,
    #[serde(rename = "catch", default)]
    pub caught: bool,
    pub fish_type: Option<String>,
}

/// Log entry for broken rod
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrokenRodEntry {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub broken: bool,
}

/// Hourly statistics
#[derive(Debug, Clone, Default)]
pub struct HourlyStats {
    pub catch: i32,
    pub fail: i32,
    pub xp: i32,
    pub fish_types: HashMap<String, i32>,
}

/// Fish statistics manager
pub struct FishStats {
    fish_logs: Vec<FishLogEntry>,
    broken_logs: Vec<BrokenRodEntry>,
    fish_xp: HashMap<String, i32>,
    fish_summary: HashMap<String, HashMap<String, HourlyStats>>,
    fish_types: Vec<String>,
    broken_summary: HashMap<String, HashMap<String, i32>>,
}

impl FishStats {
    pub fn new() -> Self {
        let mut stats = Self {
            fish_logs: Vec::new(),
            broken_logs: Vec::new(),
            fish_xp: HashMap::new(),
            fish_summary: HashMap::new(),
            fish_types: Vec::new(),
            broken_summary: HashMap::new(),
        };
        stats.refresh();
        stats
    }
    
    pub fn refresh(&mut self) {
        let base = get_data_dir();
        self.fish_logs = Self::load_json(&base.join("logs").join("fishing_log.json"));
        self.broken_logs = Self::load_json(&base.join("logs").join("broken_rods.json"));
        self.fish_xp = self.get_fish_xp_map();
        let (summary, types) = self.summarize_fishing();
        self.fish_summary = summary;
        self.fish_types = types;
        self.broken_summary = self.summarize_broken_rods();
    }
    
    fn load_json<T: for<'de> Deserialize<'de> + Default>(path: &std::path::Path) -> Vec<T> {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }
    
    fn get_fish_xp_map(&self) -> HashMap<String, i32> {
        let base = get_data_dir();
        let config_path = base.join("config").join("fish_config.json");
        let mut service = FishService::new(config_path);
        if service.load_fishes().is_ok() {
            service.get_all().iter().map(|f| (f.id.clone(), f.xp)).collect()
        } else {
            HashMap::new()
        }
    }
    
    fn summarize_fishing(&self) -> (HashMap<String, HashMap<String, HourlyStats>>, Vec<String>) {
        let mut summary: HashMap<String, HashMap<String, HourlyStats>> = HashMap::new();
        let mut all_fish_types = std::collections::HashSet::new();
        
        for entry in &self.fish_logs {
            let dt = match DateTime::parse_from_rfc3339(&entry.timestamp) {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            let date_str = dt.format("%Y-%m-%d").to_string();
            let hour_str = format!("{:02}:00", dt.hour());
            
            let day_stats = summary.entry(date_str).or_default();
            let hour_stats = day_stats.entry(hour_str).or_default();
            
            if entry.caught {
                hour_stats.catch += 1;
                
                let fish_type = entry.fish_type.clone().unwrap_or_else(|| "undefined".to_string());
                let xp_value = self.fish_xp.get(&fish_type).copied().unwrap_or(1);
                
                *hour_stats.fish_types.entry(fish_type.clone()).or_insert(0) += 1;
                hour_stats.xp += xp_value;
                all_fish_types.insert(fish_type);
            } else {
                hour_stats.fail += 1;
            }
        }
        
        let mut types: Vec<_> = all_fish_types.into_iter().collect();
        types.sort();
        
        (summary, types)
    }
    
    fn summarize_broken_rods(&self) -> HashMap<String, HashMap<String, i32>> {
        let mut summary: HashMap<String, HashMap<String, i32>> = HashMap::new();
        
        for entry in &self.broken_logs {
            if !entry.broken {
                continue;
            }
            
            let dt = match DateTime::parse_from_rfc3339(&entry.timestamp) {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            let date_str = dt.format("%Y-%m-%d").to_string();
            let hour_str = format!("{:02}:00", dt.hour());
            
            *summary.entry(date_str).or_default().entry(hour_str).or_insert(0) += 1;
        }
        
        summary
    }
    
    pub fn get_daily_table(&mut self, date: &str) -> String {
        self.refresh();
        
        let hours = match self.fish_summary.get(date) {
            Some(h) => h,
            None => return format!("<p>No data for {}</p>", date),
        };
        
        let _broken = self.broken_summary.get(date).cloned().unwrap_or_default();
        
        let total_caught: i32 = hours.values().map(|h| h.catch).sum();
        let total_missed: i32 = hours.values().map(|h| h.fail).sum();
        let total_xp: i32 = hours.values().map(|h| h.xp).sum();
        let total_hours = hours.len() as f64;
        let avg_xp_hour = if total_hours > 0.0 { total_xp as f64 / total_hours } else { 0.0 };
        
        let total_all = total_caught + total_missed;
        let catch_rate = if total_all > 0 { (total_caught as f64 / total_all as f64) * 100.0 } else { 0.0 };
        
        let mut fish_counts: HashMap<String, i32> = HashMap::new();
        for h in hours.values() {
            for (ft, cnt) in &h.fish_types {
                *fish_counts.entry(ft.clone()).or_insert(0) += cnt;
            }
        }
        
        let mut sorted_fish: Vec<_> = fish_counts.iter().filter(|(_, &v)| v > 0).collect();
        sorted_fish.sort_by(|a, b| b.1.cmp(a.1));
        
        let summary_html = format!(r#"
        <div class="daily-summary">
            <div class="daily-metric"><span class="label">Caught</span><span class="value">{}</span></div>
            <div class="daily-metric"><span class="label">Missed</span><span class="value">{}</span></div>
            <div class="daily-metric"><span class="label">XP/hour</span><span class="value">{:.0}</span></div>
            <div class="daily-metric"><span class="label">Catch Rate</span><span class="value">{:.2}%</span></div>
        </div>
        "#, total_caught, total_missed, avg_xp_hour, catch_rate);
        
        let max_fish = sorted_fish.iter().map(|(_, &v)| v).max().unwrap_or(1);
        let mut bars_html = String::new();
        for (fish, count) in sorted_fish {
            let width = (*count as f64 / max_fish as f64) * 100.0;
            let display_name = fish.replace("_", " ");
            bars_html.push_str(&format!(r#"
            <div class="fish-row">
                <span class="fish-name">{}</span>
                <div class="fish-bar">
                    <div class="fish-fill" style="width:{:.1}%;"></div>
                </div>
                <span class="fish-count">{}</span>
            </div>
            "#, display_name, width, count));
        }
        
        format!(r#"
        <div class="daily-box">
            <h3 class="daily-date">{}</h3>
            {}
            <div class="fish-graph">{}</div>
        </div>
        "#, date, summary_html, bars_html)
    }
    
    pub fn get_all_daily_tables(&mut self) -> String {
        self.refresh();
        let mut dates: Vec<_> = self.fish_summary.keys().cloned().collect();
        dates.sort();
        
        let mut html = String::new();
        for date in dates {
            html.push_str(&self.get_daily_table(&date));
        }
        html
    }
    
    pub fn get_overall_summary(&mut self) -> String {
        self.refresh();
        
        let mut total_caught = 0;
        let mut total_failed = 0;
        let mut total_broken = 0;
        let mut total_xp = 0;
        let mut total_fish_types: HashMap<String, i32> = HashMap::new();
        
        for (date, hours) in &self.fish_summary {
            let broken = self.broken_summary.get(date).cloned().unwrap_or_default();
            
            for (hour, counts) in hours {
                total_caught += counts.catch;
                total_failed += counts.fail;
                total_broken += broken.get(hour).copied().unwrap_or(0);
                total_xp += counts.xp;
                
                for (ft, c) in &counts.fish_types {
                    *total_fish_types.entry(ft.clone()).or_insert(0) += c;
                }
            }
        }
        
        let total_fish = total_caught + total_failed;
        let overall_rate = if total_fish > 0 { (total_caught as f64 / total_fish as f64) * 100.0 } else { 0.0 };
        
        // Calculate average fish per minute based on total hours of data
        // Each hour entry represents data from that hour block
        let total_hours = self.fish_summary.values()
            .map(|hours| hours.len())
            .sum::<usize>() as f64;
        let avg_fpm = if total_hours > 0.0 { total_caught as f64 / (total_hours * 60.0) } else { 0.0 };
        
        format!(r#"
        <h3 style='margin-bottom: 6px;'>Overall Stats</h3>
        <div style='overflow-x:auto; background:#111827; border-radius:8px; padding:10px;'>
            <table class="data-table">
                <thead>
                    <tr>
                        <th>TOTAL CAUGHT</th>
                        <th>TOTAL MISSED</th>
                        <th>TOTAL BROKEN RODS</th>
                        <th>OVERALL CATCH RATE</th>
                        <th>AVG FISH/MIN</th>
                        <th>TOTAL XP</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{:.2}%</td>
                        <td>{:.2}</td>
                        <td>{}</td>
                    </tr>
                </tbody>
            </table>
        </div>
        "#, total_caught, total_failed, total_broken, overall_rate, avg_fpm, total_xp)
    }
    
    pub fn get_dates(&self) -> Vec<String> {
        let mut dates: Vec<_> = self.fish_summary.keys().cloned().collect();
        dates.sort();
        dates
    }
}

impl Default for FishStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Stats API exposed to JavaScript
pub struct StatsApi {
    stats: FishStats,
    settings: HashMap<String, String>,
}

impl StatsApi {
    pub fn new() -> Self {
        Self {
            stats: FishStats::new(),
            settings: Self::load_settings(),
        }
    }
    
    fn load_settings() -> HashMap<String, String> {
        let base = get_data_dir();
        let settings_file = base.join("config").join("settings.json");
        
        if settings_file.exists() {
            if let Ok(content) = fs::read_to_string(&settings_file) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        
        HashMap::new()
    }
    
    fn save_settings(&self) {
        let base = get_data_dir();
        let settings_file = base.join("config").join("settings.json");
        
        if let Some(parent) = settings_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        if let Ok(content) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(&settings_file, content);
        }
    }
    
    pub fn set_resolution(&mut self, res: &str) {
        self.settings.insert("resolution".to_string(), res.to_string());
        self.save_settings();
    }
    
    pub fn get_resolution(&self) -> String {
        self.settings.get("resolution").cloned().unwrap_or_else(|| "1920x1080".to_string())
    }
    
    // Overlay settings
    pub fn set_show_debug_overlay(&mut self, value: bool) {
        self.settings.insert("show_debug_overlay".to_string(), value.to_string());
        self.save_settings();
    }
    
    pub fn get_show_debug_overlay(&self) -> bool {
        self.settings.get("show_debug_overlay")
            .map(|s| s == "true")
            .unwrap_or(true)
    }
    
    pub fn set_overlay_always_on_top(&mut self, value: bool) {
        self.settings.insert("overlay_always_on_top".to_string(), value.to_string());
        self.save_settings();
    }
    
    pub fn get_overlay_always_on_top(&self) -> bool {
        self.settings.get("overlay_always_on_top")
            .map(|s| s == "true")
            .unwrap_or(true)
    }
    
    pub fn set_show_overlay(&mut self, value: bool) {
        self.settings.insert("show_overlay".to_string(), value.to_string());
        self.save_settings();
    }
    
    pub fn get_show_overlay(&self) -> bool {
        self.settings.get("show_overlay")
            .map(|s| s == "true")
            .unwrap_or(true)
    }
    
    pub fn get_daily_table(&mut self) -> String {
        self.stats.get_all_daily_tables()
    }
    
    pub fn get_overall_summary(&mut self) -> String {
        self.stats.get_overall_summary()
    }
    
    pub fn get_dates(&self) -> Vec<String> {
        self.stats.get_dates()
    }
    
    pub fn get_key(&self, name: &str) -> Option<String> {
        get_key(name)
    }
}

impl Default for StatsApi {
    fn default() -> Self {
        Self::new()
    }
}
