//! Screen capture service

#![allow(dead_code)]

use std::time::Duration;
use std::thread;
use image::DynamicImage;
use screenshots::Screen;
use anyhow::{Result, Context};

/// Region for screenshot capture
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    pub fn new(left: i32, top: i32, width: u32, height: u32) -> Self {
        Self { left, top, width, height }
    }
    
    /// Create from window rect (x1, y1, x2, y2)
    pub fn from_rect(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self {
            left: x1,
            top: y1,
            width: (x2 - x1).max(0) as u32,
            height: (y2 - y1).max(0) as u32,
        }
    }
}

/// Service for capturing screenshots
pub struct ScreenService {
    region: Option<Region>,
}

impl ScreenService {
    /// Create a new screen service
    pub fn new() -> Self {
        Self { region: None }
    }
    
    /// Create a new screen service with a specific region
    pub fn with_region(region: Region) -> Self {
        Self { region: Some(region) }
    }
    
    /// Set the capture region
    pub fn set_region(&mut self, region: Option<Region>) {
        self.region = region;
    }
    
    /// Take a screenshot safely with retries
    pub fn safe_screenshot(&self, region: Option<Region>, retries: u32, delay: Duration) -> Option<DynamicImage> {
        for i in 0..retries {
            match self.capture(region.or(self.region)) {
                Ok(img) => return Some(img),
                Err(e) => {
                    tracing::warn!("Screenshot failed: {}. Retrying ({}/{})", e, i + 1, retries);
                    thread::sleep(delay);
                }
            }
        }
        None
    }
    
    /// Take a screenshot of the entire screen or a region
    pub fn screenshot(&self) -> Result<DynamicImage> {
        self.capture(self.region)
    }
    
    /// Internal capture method
    fn capture(&self, region: Option<Region>) -> Result<DynamicImage> {
        let screens = Screen::all().context("Failed to get screens")?;
        
        if screens.is_empty() {
            anyhow::bail!("No screens found");
        }
        
        // Get primary screen (first one)
        let screen = &screens[0];
        
        let image = if let Some(r) = region {
            screen.capture_area(r.left, r.top, r.width, r.height)
                .context("Failed to capture area")?
        } else {
            screen.capture().context("Failed to capture screen")?
        };
        
        // Convert to image::DynamicImage
        let rgba_image = image::RgbaImage::from_raw(
            image.width(),
            image.height(),
            image.to_vec(),
        ).context("Failed to create image from raw data")?;
        
        Ok(DynamicImage::ImageRgba8(rgba_image))
    }
    
    /// Capture a specific region within a window rect
    pub fn capture_window_region(&self, window_rect: Option<(i32, i32, i32, i32)>, sub_region: Option<Region>) -> Option<DynamicImage> {
        let (x1, y1, x2, y2) = window_rect?;
        let w = (x2 - x1).max(0) as u32;
        let h = (y2 - y1).max(0) as u32;
        
        let region = if let Some(r) = sub_region {
            Region::new(
                x1 + r.left,
                y1 + r.top,
                r.width,
                r.height,
            )
        } else {
            Region::new(x1, y1, w, h)
        };
        
        self.safe_screenshot(Some(region), 3, Duration::from_millis(100))
    }
}

impl Default for ScreenService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_from_rect() {
        let region = Region::from_rect(100, 200, 500, 600);
        assert_eq!(region.left, 100);
        assert_eq!(region.top, 200);
        assert_eq!(region.width, 400);
        assert_eq!(region.height, 400);
    }

    #[test]
    fn test_screen_service_new() {
        let service = ScreenService::new();
        assert!(service.region.is_none());
    }
}
