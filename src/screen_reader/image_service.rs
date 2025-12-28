//! Image processing service for template matching and OCR

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::fs;
use std::time::Duration;
use image::GrayImage;
use opencv::{
    core::{Mat, MatTraitConst, Point, CV_8UC1, CV_32FC1, min_max_loc, no_array, Scalar},
    imgcodecs,
    imgproc,
    prelude::*,
};

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
    
    /// Convert image::GrayImage to OpenCV Mat
    fn gray_image_to_mat(img: &GrayImage) -> opencv::Result<Mat> {
        let (width, height) = (img.width() as i32, img.height() as i32);
        let data = img.as_raw();
        
        // Create a Mat by copying the data to ensure proper ownership
        // Step parameter is width * 1 byte per pixel for single-channel grayscale (CV_8UC1)
        let step = width as usize * 1;  // 1 byte per pixel for 8-bit grayscale
        let mat = unsafe {
            Mat::new_rows_cols_with_data_unsafe(
                height,
                width,
                CV_8UC1,
                data.as_ptr() as *mut std::ffi::c_void,
                step,
            )?
        };
        
        // Clone the Mat to ensure the data is owned by the Mat
        // This is necessary because the original data is owned by the GrayImage
        let owned_mat = mat.clone();
        Ok(owned_mat)
    }
    
    /// Load image as OpenCV Mat in grayscale
    fn load_template_grayscale(path: &Path) -> opencv::Result<Mat> {
        let path_str = path.to_str().ok_or_else(|| {
            opencv::Error::new(opencv::core::StsError, "Invalid path: non-UTF8 characters")
        })?;
        imgcodecs::imread(path_str, imgcodecs::IMREAD_GRAYSCALE)
    }
    
    /// Load image with alpha channel
    fn load_template_unchanged(path: &Path) -> opencv::Result<Mat> {
        let path_str = path.to_str().ok_or_else(|| {
            opencv::Error::new(opencv::core::StsError, "Invalid path: non-UTF8 characters")
        })?;
        imgcodecs::imread(path_str, imgcodecs::IMREAD_UNCHANGED)
    }
    
    /// Find a single image on the screen within a given window rectangle
    /// Returns center coordinates if found, else None
    pub fn find_image_in_window(
        &self,
        window_rect: Option<(i32, i32, i32, i32)>,
        image_path: &Path,
        threshold: f32,
    ) -> Option<(i32, i32)> {
        let image_name = image_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        tracing::trace!("[IMAGE] find_image_in_window('{}', threshold={:.2})", image_name, threshold);
        
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
        
        // Convert to OpenCV Mat
        let img_mat = Self::gray_image_to_mat(&img_gray).ok()?;
        
        // Load template
        let template = Self::load_template_grayscale(image_path).ok()?;
        if template.empty() {
            tracing::warn!("[IMAGE] Template not found or empty: {:?}", image_path);
            return None;
        }
        
        // Skip if template is larger than image
        if template.cols() >= img_mat.cols() || template.rows() >= img_mat.rows() {
            tracing::trace!("[IMAGE] Template larger than image, skipping");
            return None;
        }
        
        // Perform template matching using TM_CCOEFF_NORMED (same as Python cv2.TM_CCOEFF_NORMED)
        let mut result = Mat::default();
        imgproc::match_template(
            &img_mat,
            &template,
            &mut result,
            imgproc::TM_CCOEFF_NORMED,
            &no_array(),
        ).ok()?;
        
        // Find maximum value and location
        let mut min_val = 0.0;
        let mut max_val = 0.0;
        let mut min_loc = Point::new(0, 0);
        let mut max_loc = Point::new(0, 0);
        
        min_max_loc(
            &result,
            Some(&mut min_val),
            Some(&mut max_val),
            Some(&mut min_loc),
            Some(&mut max_loc),
            &no_array(),
        ).ok()?;
        
        if max_val >= threshold as f64 {
            let click_x = x1 + max_loc.x + template.cols() / 2;
            let click_y = y1 + max_loc.y + template.rows() / 2;
            tracing::debug!("[IMAGE] FOUND '{}' at ({}, {}) with score={:.3} >= threshold={:.2}", 
                image_name, click_x, click_y, max_val, threshold);
            return Some((click_x, click_y));
        }
        
        tracing::trace!("[IMAGE] '{}' NOT FOUND - score={:.3} < threshold={:.2}", 
            image_name, max_val, threshold);
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
        
        // Convert cropped image to OpenCV Mat
        let crop_mat = match Self::gray_image_to_mat(&crop) {
            Ok(m) => m,
            Err(_) => return (None, 0.0),
        };
        
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
                
                // Load template with OpenCV
                let template = match Self::load_template_grayscale(&path) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                
                if template.empty() {
                    continue;
                }
                
                // Skip if template is larger than crop
                if template.cols() >= crop_mat.cols() || template.rows() >= crop_mat.rows() {
                    continue;
                }
                
                // Perform template matching using TM_CCOEFF_NORMED
                let mut result = Mat::default();
                if imgproc::match_template(
                    &crop_mat,
                    &template,
                    &mut result,
                    imgproc::TM_CCOEFF_NORMED,
                    &no_array(),
                ).is_err() {
                    continue;
                }
                
                // Find maximum value
                let mut max_val = 0.0;
                if min_max_loc(
                    &result,
                    None,
                    Some(&mut max_val),
                    None,
                    None,
                    &no_array(),
                ).is_err() {
                    continue;
                }
                
                if max_val as f32 > best_score {
                    best_score = max_val as f32;
                    best_fish = path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
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
        
        // Convert cropped image to OpenCV Mat
        let crop_mat = match Self::gray_image_to_mat(&img_crop) {
            Ok(m) => m,
            Err(_) => return (None, 0.0),
        };
        
        let arrow_folder = self.target_images_folder.join(&self.resolution_folder);
        
        let templates = ["left-high.png", "right-high.png"];
        let mut best_match: Option<String> = None;
        let mut best_score = 0.0f32;
        
        for template_name in &templates {
            let template_path = arrow_folder.join(template_name);
            if !template_path.exists() {
                continue;
            }
            
            // Load template with alpha channel support (like Python cv2.IMREAD_UNCHANGED)
            let template_img = match Self::load_template_unchanged(&template_path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            
            if template_img.empty() {
                continue;
            }
            
            // Handle alpha channel if present (4-channel image)
            let (template, mask): (Mat, Option<Mat>) = if template_img.channels() == 4 {
                // Extract BGR and alpha channel
                let mut channels = opencv::core::Vector::<Mat>::new();
                if opencv::core::split(&template_img, &mut channels).is_err() {
                    continue;
                }
                
                // Verify we have at least 4 channels
                if channels.len() < 4 {
                    continue;
                }
                
                // Convert BGR to grayscale
                let mut bgr = Mat::default();
                let mut gray = Mat::default();
                
                // Merge BGR channels (first 3) - explicitly get each channel
                let ch0 = match channels.get(0) { Ok(c) => c, Err(_) => continue };
                let ch1 = match channels.get(1) { Ok(c) => c, Err(_) => continue };
                let ch2 = match channels.get(2) { Ok(c) => c, Err(_) => continue };
                let bgr_channels = opencv::core::Vector::<Mat>::from_iter([ch0, ch1, ch2]);
                
                if opencv::core::merge(&bgr_channels, &mut bgr).is_err() {
                    continue;
                }
                
                if imgproc::cvt_color(&bgr, &mut gray, imgproc::COLOR_BGR2GRAY, 0).is_err() {
                    continue;
                }
                
                // Create mask from alpha channel (alpha > 0)
                let alpha = match channels.get(3) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                let mut mask = Mat::default();
                if imgproc::threshold(&alpha, &mut mask, 1.0, 255.0, imgproc::THRESH_BINARY).is_err() {
                    continue;
                }
                
                (gray, Some(mask))
            } else {
                // Convert to grayscale if not already
                let mut gray = Mat::default();
                if template_img.channels() == 3 {
                    if imgproc::cvt_color(&template_img, &mut gray, imgproc::COLOR_BGR2GRAY, 0).is_err() {
                        continue;
                    }
                } else {
                    gray = template_img;
                }
                (gray, None)
            };
            
            // Skip if template is larger than crop
            if template.cols() >= crop_mat.cols() || template.rows() >= crop_mat.rows() {
                continue;
            }
            
            // Perform template matching with optional mask
            let mut result = Mat::default();
            let match_result = match &mask {
                Some(m) => imgproc::match_template(
                    &crop_mat,
                    &template,
                    &mut result,
                    imgproc::TM_CCOEFF_NORMED,
                    m,
                ),
                None => imgproc::match_template(
                    &crop_mat,
                    &template,
                    &mut result,
                    imgproc::TM_CCOEFF_NORMED,
                    &no_array(),
                ),
            };
            
            if match_result.is_err() {
                continue;
            }
            
            // Find maximum value
            let mut max_val = 0.0;
            if min_max_loc(
                &result,
                None,
                Some(&mut max_val),
                None,
                None,
                &no_array(),
            ).is_err() {
                continue;
            }
            
            if max_val as f32 > best_score {
                best_score = max_val as f32;
                best_match = Some(template_name.replace(".png", ""));
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
    use image::Luma;

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
    fn test_gray_image_to_mat() {
        // Create a simple 10x10 grayscale image
        let mut img = GrayImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, Luma([((x + y) * 12 % 256) as u8]));
            }
        }
        
        // Convert to Mat
        let mat = ImageService::gray_image_to_mat(&img);
        assert!(mat.is_ok());
        
        let mat = mat.unwrap();
        assert_eq!(mat.cols(), 10);
        assert_eq!(mat.rows(), 10);
    }
    
    #[test]
    fn test_opencv_template_matching() {
        // Create a 20x20 image with a unique pattern at position (5,5)
        let mut img = GrayImage::new(20, 20);
        
        // Fill with gray background
        for y in 0..20 {
            for x in 0..20 {
                img.put_pixel(x, y, Luma([128u8]));
            }
        }
        
        // Create a unique 5x5 pattern at position (5,5)
        for dy in 0..5 {
            for dx in 0..5 {
                let val = (dx * 50 + dy * 40) as u8;
                img.put_pixel(5 + dx, 5 + dy, Luma([val]));
            }
        }
        
        // Create the template from position (5,5)
        let template_img = image::imageops::crop_imm(&img, 5, 5, 5, 5).to_image();
        
        // Convert both to Mat
        let img_mat = ImageService::gray_image_to_mat(&img).unwrap();
        let template_mat = ImageService::gray_image_to_mat(&template_img).unwrap();
        
        // Perform template matching
        let mut result = Mat::default();
        imgproc::match_template(
            &img_mat,
            &template_mat,
            &mut result,
            imgproc::TM_CCOEFF_NORMED,
            &no_array(),
        ).unwrap();
        
        // Find maximum value and location
        let mut max_val = 0.0;
        let mut max_loc = Point::new(0, 0);
        min_max_loc(
            &result,
            None,
            Some(&mut max_val),
            None,
            Some(&mut max_loc),
            &no_array(),
        ).unwrap();
        
        // The score should be very close to 1.0 for exact match
        assert!(max_val > 0.99, "Score {} should be > 0.99 for exact match", max_val);
        // Location should be approximately where we cropped from
        assert_eq!(max_loc.x, 5, "X location {} should be 5", max_loc.x);
        assert_eq!(max_loc.y, 5, "Y location {} should be 5", max_loc.y);
    }
}
