//! Image processing service for template matching and OCR

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::fs;
use std::time::Duration;
use image::{GrayImage, Luma};
use imageproc::template_matching::{match_template, find_extremes, MatchTemplateMethod};

use super::screen_service::{ScreenService, Region};
use super::base::get_resolution_folder;
use crate::utils::path::get_data_dir;

/// Image service for template matching and fish detection
pub struct ImageService {
    screen_service: ScreenService,
    target_images_folder: PathBuf,
    resolution_folder: String,
}

impl ImageService {
    /// Create a new image service
    pub fn new() -> Self {
        let base = get_data_dir();
        Self {
            screen_service: ScreenService::new(),
            target_images_folder: base.join("images"),
            resolution_folder: get_resolution_folder(),
        }
    }
    
    /// Update resolution folder
    pub fn update_resolution(&mut self) {
        self.resolution_folder = get_resolution_folder();
    }
    
    /// Find a single image on the screen within a given window rectangle
    /// Returns center coordinates if found, else None
    pub fn find_image_in_window(
        &self,
        window_rect: Option<(i32, i32, i32, i32)>,
        image_path: &Path,
        threshold: f32,
    ) -> Option<(i32, i32)> {
        let window_rect = window_rect?;
        let (x1, y1, x2, y2) = window_rect;
        let w = (x2 - x1).max(0) as u32;
        let h = (y2 - y1).max(0) as u32;
        
        let screenshot = self.screen_service.safe_screenshot(
            Some(Region::new(x1, y1, w, h)),
            3,
            Duration::from_millis(100),
        )?;
        
        let img_gray = screenshot.to_luma8();
        
        // Load template
        let template = match image::open(image_path) {
            Ok(t) => t.to_luma8(),
            Err(e) => {
                tracing::warn!("Template not found: {:?}: {}", image_path, e);
                return None;
            }
        };
        
        // Skip if template is larger than image
        if template.width() >= img_gray.width() || template.height() >= img_gray.height() {
            return None;
        }
        
        // Use imageproc's optimized template matching with CrossCorrelationNormalized
        // This is similar to OpenCV's TM_CCOEFF_NORMED and gives values in [0, 1] range
        let result = match_template(&img_gray, &template, MatchTemplateMethod::CrossCorrelationNormalized);
        let extremes = find_extremes(&result);
        
        if extremes.max_value >= threshold {
            let (max_x, max_y) = extremes.max_value_location;
            let click_x = x1 + max_x as i32 + template.width() as i32 / 2;
            let click_y = y1 + max_y as i32 + template.height() as i32 / 2;
            return Some((click_x, click_y));
        }
        
        None
    }
    
    /// Capture window as grayscale image
    pub fn capture_window(&self, window_rect: Option<(i32, i32, i32, i32)>, region: Option<Region>) -> Option<GrayImage> {
        let window_rect = window_rect?;
        let (x1, y1, x2, y2) = window_rect;
        let w = (x2 - x1).max(0) as u32;
        let h = (y2 - y1).max(0) as u32;
        
        let capture_region = if let Some(r) = region {
            Some(Region::new(x1 + r.left, y1 + r.top, r.width, r.height))
        } else {
            Some(Region::new(x1, y1, w, h))
        };
        
        let screenshot = self.screen_service.safe_screenshot(capture_region, 3, Duration::from_millis(100))?;
        Some(screenshot.to_luma8())
    }
    
    /// Find best matching fish using OCR-like detection
    /// Returns fish_name and confidence
    pub fn find_best_matching_fish(&self, window_rect: Option<(i32, i32, i32, i32)>, img: Option<GrayImage>) -> (Option<String>, f32) {
        let img = match img {
            Some(i) => i,
            None => match self.capture_window(window_rect, None) {
                Some(i) => i,
                None => return (None, 0.0),
            },
        };
        
        let (h, w) = (img.height(), img.width());
        
        // Crop area for fish name detection
        let crop_x1 = (w as f32 * 0.56) as u32;
        let crop_y1 = (h as f32 * 0.66) as u32;
        let crop_w = (w as f32 * 0.30) as u32;
        let crop_h = (h as f32 * 0.08) as u32;
        
        let crop = image::imageops::crop_imm(&img, crop_x1, crop_y1, crop_w, crop_h).to_image();
        
        // Simple text detection using template matching against fish name images
        // In a full implementation, this would use OCR (tesseract)
        // For now, we use template matching against known fish name patterns
        
        let fish_folder = self.target_images_folder
            .join(&self.resolution_folder)
            .join("fish");
        
        if !fish_folder.exists() {
            return (None, 0.0);
        }
        
        let mut best_fish: Option<String> = None;
        let mut best_score = 0.0f32;
        
        if let Ok(entries) = fs::read_dir(&fish_folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("png") {
                    continue;
                }
                
                if let Ok(template) = image::open(&path) {
                    let template_gray = template.to_luma8();
                    
                    // Skip if template is larger than crop
                    if template_gray.width() >= crop.width() || template_gray.height() >= crop.height() {
                        continue;
                    }
                    
                    // Use imageproc's optimized template matching
                    let result = match_template(&crop, &template_gray, MatchTemplateMethod::CrossCorrelationNormalized);
                    let extremes = find_extremes(&result);
                    
                    if extremes.max_value > best_score {
                        best_score = extremes.max_value;
                        best_fish = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
                    }
                }
            }
        }
        
        // Normalize fish name to match the format used in fish_config.json
        // - Spaces are replaced with underscores (e.g., "Glass Bottle" -> "glass_bottle")
        // - Hash symbols are removed (e.g., "Legacy Part #1" -> "legacy_part_1")
        // - Converted to lowercase for consistent matching
        if let Some(ref name) = best_fish {
            let normalized = name.replace(" ", "_").replace("#", "").to_lowercase();
            return (Some(normalized), best_score);
        }
        
        (None, 0.0)
    }
    
    /// Detect arrows in minigame
    pub fn find_minigame_arrow(&self, window_rect: Option<(i32, i32, i32, i32)>, img: Option<GrayImage>) -> (Option<String>, f32) {
        let img = match img {
            Some(i) => i,
            None => match self.capture_window(window_rect, None) {
                Some(i) => i,
                None => return (None, 0.0),
            },
        };
        
        let (h, w) = (img.height(), img.width());
        
        // Crop area for arrow detection
        let crop_width = (w as f32 * 0.40) as u32;
        let crop_height = (h as f32 * 0.20) as u32;
        let crop_x1 = (w as f32 * 0.30) as u32;
        let crop_y1 = (h as f32 * 0.40) as u32;
        
        let img_crop = image::imageops::crop_imm(&img, crop_x1, crop_y1, crop_width, crop_height).to_image();
        
        let arrow_folder = self.target_images_folder.join(&self.resolution_folder);
        
        let templates = ["left-high.png", "right-high.png"];
        let mut best_match: Option<String> = None;
        let mut best_score = 0.0f32;
        
        for template_name in &templates {
            let template_path = arrow_folder.join(template_name);
            if !template_path.exists() {
                continue;
            }
            
            if let Ok(template_img) = image::open(&template_path) {
                let template = template_img.to_luma8();
                
                // Skip if template is larger than crop
                if template.width() >= img_crop.width() || template.height() >= img_crop.height() {
                    continue;
                }
                
                // Use imageproc's optimized template matching
                let result = match_template(&img_crop, &template, MatchTemplateMethod::CrossCorrelationNormalized);
                let extremes = find_extremes(&result);
                
                if extremes.max_value > best_score {
                    best_score = extremes.max_value;
                    best_match = Some(template_name.replace(".png", ""));
                }
            }
        }
        
        (best_match, best_score)
    }
    
    /// Get path to a target image
    pub fn get_image_path(&self, name: &str) -> PathBuf {
        self.target_images_folder
            .join(&self.resolution_folder)
            .join(name)
    }
}

impl Default for ImageService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_service_new() {
        let service = ImageService::new();
        assert!(!service.resolution_folder.is_empty());
    }
    
    #[test]
    fn test_get_image_path() {
        let service = ImageService::new();
        let path = service.get_image_path("test.png");
        assert!(path.to_string_lossy().contains("test.png"));
    }
    
    #[test]
    fn test_match_template_exact_match() {
        // Create a simple 10x10 image with a pattern
        let mut img = GrayImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                // Use modulo to avoid overflow
                img.put_pixel(x, y, Luma([((x + y) * 12 % 256) as u8]));
            }
        }
        
        // Create a 3x3 template from the center of the image
        let template = image::imageops::crop_imm(&img, 3, 3, 3, 3).to_image();
        
        // Use imageproc's match_template
        let result = match_template(&img, &template, MatchTemplateMethod::CrossCorrelationNormalized);
        let extremes = find_extremes(&result);
        
        // The score should be very close to 1.0 for exact match
        assert!(extremes.max_value > 0.99, "Score {} should be > 0.99 for exact match", extremes.max_value);
        // Location should be approximately where we cropped from
        assert_eq!(extremes.max_value_location, (3, 3), "Location {:?} should be (3, 3)", extremes.max_value_location);
    }
    
    #[test]
    fn test_match_template_no_match() {
        // Create a gradient image
        let mut img = GrayImage::new(20, 20);
        for y in 0..20 {
            for x in 0..20 {
                img.put_pixel(x, y, Luma([(x * 10).min(255) as u8]));
            }
        }
        
        // Create a template with inverted pattern (should not match well)
        let mut template = GrayImage::new(5, 5);
        for y in 0..5 {
            for x in 0..5 {
                template.put_pixel(x, y, Luma([(255 - (x * 40)).min(255) as u8]));
            }
        }
        
        // Use imageproc's match_template
        let result = match_template(&img, &template, MatchTemplateMethod::CrossCorrelationNormalized);
        let extremes = find_extremes(&result);
        
        // Score should be low for poor match
        assert!(extremes.max_value < 0.5, "Score {} should be < 0.5 for poor match", extremes.max_value);
    }
}
