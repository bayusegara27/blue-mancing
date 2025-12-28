//! Fish Detection Benchmark Test
//!
//! This test validates fish detection accuracy using test images from tests/assets/1920x1080.
//! Each test image is named `{expected_fish_name}_test_1920x1080.png` and should be detected
//! as the corresponding fish type.
//!
//! Run with: cargo test --test fish_detection_benchmark -- --nocapture

use image::io::Reader as ImageReader;
use image::GrayImage;
use opencv::{
    core::{min_max_loc, no_array, Mat, MatTraitConst, CV_8UC1},
    imgcodecs, imgproc,
    prelude::*,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

// ========== Fish Detection Crop Region Constants ==========
// These should match the values in image_service.rs
// Based on benchmark analysis of test images, fish templates appear at:
// - X: 23% to 46% of screen width (lower-left area)
// - Y: 68% to 99% of screen height (bottom portion)
const FISH_CROP_X_START: f32 = 0.20;
const FISH_CROP_Y_START: f32 = 0.65;
const FISH_CROP_WIDTH: f32 = 0.30;
const FISH_CROP_HEIGHT: f32 = 0.35;

/// Convert image::GrayImage to OpenCV Mat
fn gray_image_to_mat(img: &GrayImage) -> opencv::Result<Mat> {
    let (width, height) = (img.width() as i32, img.height() as i32);
    let data = img.as_raw();
    let step = width as usize * 1;
    let mat = unsafe {
        Mat::new_rows_cols_with_data_unsafe(
            height,
            width,
            CV_8UC1,
            data.as_ptr() as *mut std::ffi::c_void,
            step,
        )?
    };
    Ok(mat.clone())
}

/// Load image with alpha channel
fn load_template_unchanged(path: &Path) -> opencv::Result<Mat> {
    let path_str = path.to_str().ok_or_else(|| {
        opencv::Error::new(opencv::core::StsError, "Invalid path")
    })?;
    imgcodecs::imread(path_str, imgcodecs::IMREAD_UNCHANGED)
}

/// Extract grayscale template and optional alpha mask from an image
fn extract_template_and_mask(template_img: &Mat) -> Option<(Mat, Option<Mat>)> {
    if template_img.empty() {
        return None;
    }

    if template_img.channels() == 4 {
        let mut channels = opencv::core::Vector::<Mat>::new();
        if opencv::core::split(template_img, &mut channels).is_err() {
            return None;
        }

        if channels.len() < 4 {
            return None;
        }

        let ch0 = channels.get(0).ok()?;
        let ch1 = channels.get(1).ok()?;
        let ch2 = channels.get(2).ok()?;
        let alpha = channels.get(3).ok()?;

        let mut bgr = Mat::default();
        let mut gray = Mat::default();
        let bgr_channels = opencv::core::Vector::<Mat>::from_iter([ch0, ch1, ch2]);

        if opencv::core::merge(&bgr_channels, &mut bgr).is_err() {
            return None;
        }

        if imgproc::cvt_color(&bgr, &mut gray, imgproc::COLOR_BGR2GRAY, 0).is_err() {
            return None;
        }

        let mut mask = Mat::default();
        if imgproc::threshold(&alpha, &mut mask, 1.0, 255.0, imgproc::THRESH_BINARY).is_err() {
            return None;
        }

        Some((gray, Some(mask)))
    } else if template_img.channels() == 3 {
        let mut gray = Mat::default();
        if imgproc::cvt_color(template_img, &mut gray, imgproc::COLOR_BGR2GRAY, 0).is_err() {
            return None;
        }
        Some((gray, None))
    } else {
        Some((template_img.clone(), None))
    }
}

/// Load all fish templates from a folder
fn load_fish_templates(fish_folder: &Path) -> HashMap<String, (Mat, Option<Mat>)> {
    let mut templates: HashMap<String, (Mat, Option<Mat>)> = HashMap::new();

    let entries = match fs::read_dir(fish_folder) {
        Ok(e) => e,
        Err(_) => return templates,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e != "png").unwrap_or(true) {
            continue;
        }

        let fish_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        let template_img = match load_template_unchanged(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if let Some((template, mask)) = extract_template_and_mask(&template_img) {
            templates.insert(fish_name, (template, mask));
        }
    }

    templates
}

/// Result of a single fish detection test
#[derive(Debug)]
struct DetectionResult {
    test_image: String,
    expected_fish: String,
    detected_fish: Option<String>,
    confidence: f32,
    detection_time_ms: f64,
    correct: bool,
}

/// Find best matching fish in a grayscale image using template matching
fn find_best_matching_fish(
    img: &GrayImage,
    templates: &HashMap<String, (Mat, Option<Mat>)>,
    use_cropping: bool,
) -> (Option<String>, f32, f64) {
    let start_time = Instant::now();

    let (h, w) = (img.height(), img.width());
    
    // Optionally crop the image to the fish result region
    let img_to_process = if use_cropping {
        let crop_x1 = (w as f32 * FISH_CROP_X_START) as u32;
        let crop_y1 = (h as f32 * FISH_CROP_Y_START) as u32;
        let crop_w = (w as f32 * FISH_CROP_WIDTH) as u32;
        let crop_h = (h as f32 * FISH_CROP_HEIGHT) as u32;
        
        let max_crop_w = w.saturating_sub(crop_x1);
        let max_crop_h = h.saturating_sub(crop_y1);
        let crop_w = crop_w.min(max_crop_w);
        let crop_h = crop_h.min(max_crop_h);

        image::imageops::crop_imm(img, crop_x1, crop_y1, crop_w, crop_h).to_image()
    } else {
        img.clone()
    };

    let img_mat = match gray_image_to_mat(&img_to_process) {
        Ok(m) => m,
        Err(_) => return (None, 0.0, start_time.elapsed().as_secs_f64() * 1000.0),
    };

    let mut best_fish: Option<String> = None;
    let mut best_score: f32 = 0.0;

    for (fish_name, (template, mask)) in templates.iter() {
        if template.cols() >= img_mat.cols() || template.rows() >= img_mat.rows() {
            continue;
        }

        let mut result = Mat::default();
        let match_result = match mask {
            Some(m) => imgproc::match_template(
                &img_mat,
                template,
                &mut result,
                imgproc::TM_CCOEFF_NORMED,
                m,
            ),
            None => imgproc::match_template(
                &img_mat,
                template,
                &mut result,
                imgproc::TM_CCOEFF_NORMED,
                &no_array(),
            ),
        };

        if match_result.is_err() {
            continue;
        }

        let mut max_val = 0.0;
        if min_max_loc(&result, None, Some(&mut max_val), None, None, &no_array()).is_err() {
            continue;
        }

        if max_val as f32 > best_score {
            best_score = max_val as f32;
            best_fish = Some(fish_name.clone());
        }
    }

    let detection_time = start_time.elapsed().as_secs_f64() * 1000.0;
    (best_fish, best_score, detection_time)
}

/// Extract expected fish name from test image filename
fn extract_expected_fish_name(filename: &str) -> Option<String> {
    // Format: {fish_name}_test_1920x1080.png
    let name = filename.strip_suffix("_test_1920x1080.png")?;
    Some(name.to_string())
}

#[test]
fn benchmark_fish_detection() {
    println!("\n========== FISH DETECTION BENCHMARK ==========\n");

    // Define paths
    let test_images_dir = Path::new("tests/assets/1920x1080");
    let fish_templates_dir = Path::new("images/1920x1080/fish");

    // Check if directories exist
    if !test_images_dir.exists() {
        println!("ERROR: Test images directory not found: {:?}", test_images_dir);
        println!("Please ensure test images are in tests/assets/1920x1080/");
        return;
    }

    if !fish_templates_dir.exists() {
        println!("ERROR: Fish templates directory not found: {:?}", fish_templates_dir);
        println!("Please ensure fish templates are in images/1920x1080/fish/");
        return;
    }

    // Load all fish templates
    println!("Loading fish templates from {:?}...", fish_templates_dir);
    let templates_start = Instant::now();
    let templates = load_fish_templates(fish_templates_dir);
    let templates_load_time = templates_start.elapsed().as_secs_f64() * 1000.0;
    println!("Loaded {} templates in {:.2}ms\n", templates.len(), templates_load_time);

    // List available templates
    println!("Available templates:");
    let mut template_names: Vec<&String> = templates.keys().collect();
    template_names.sort();
    for name in &template_names {
        println!("  - {}", name);
    }
    println!();

    // Collect test images
    let test_images: Vec<_> = fs::read_dir(test_images_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "png")
                .unwrap_or(false)
        })
        .collect();

    println!("Found {} test images\n", test_images.len());

    // Run detection tests WITH cropping (optimized)
    println!("========== TEST WITH CROPPING (OPTIMIZED) ==========\n");
    let mut results_cropped: Vec<DetectionResult> = Vec::new();

    for entry in &test_images {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        
        // Load test image
        let img = match ImageReader::open(&path) {
            Ok(reader) => match reader.decode() {
                Ok(img) => img.to_luma8(),
                Err(e) => {
                    println!("  Failed to decode {}: {}", filename, e);
                    continue;
                }
            },
            Err(e) => {
                println!("  Failed to open {}: {}", filename, e);
                continue;
            }
        };

        // Extract expected fish name
        let expected_fish = extract_expected_fish_name(filename)
            .unwrap_or_else(|| "unknown".to_string());

        // Run detection with cropping
        let (detected_fish, confidence, detection_time) = 
            find_best_matching_fish(&img, &templates, true);

        let correct = detected_fish
            .as_ref()
            .map(|d| d == &expected_fish)
            .unwrap_or(false);

        results_cropped.push(DetectionResult {
            test_image: filename.to_string(),
            expected_fish,
            detected_fish,
            confidence,
            detection_time_ms: detection_time,
            correct,
        });
    }

    // Print results with cropping
    let mut correct_count = 0;
    let mut total_time = 0.0;

    println!("{:<45} | {:<25} | {:<25} | {:>8} | {:>10}",
        "Test Image", "Expected", "Detected", "Score", "Time (ms)");
    println!("{}", "-".repeat(125));

    for result in &results_cropped {
        let status = if result.correct { "✓" } else { "✗" };
        let detected = result.detected_fish.as_deref().unwrap_or("NONE");
        
        println!("{} {:<42} | {:<25} | {:<25} | {:>7.3} | {:>10.2}",
            status,
            result.test_image,
            result.expected_fish,
            detected,
            result.confidence,
            result.detection_time_ms
        );

        if result.correct {
            correct_count += 1;
        }
        total_time += result.detection_time_ms;
    }

    let accuracy = (correct_count as f64 / results_cropped.len() as f64) * 100.0;
    let avg_time = total_time / results_cropped.len() as f64;

    println!("\n{}", "=".repeat(125));
    println!("RESULTS WITH CROPPING:");
    println!("  Accuracy: {}/{} ({:.1}%)", correct_count, results_cropped.len(), accuracy);
    println!("  Average detection time: {:.2}ms", avg_time);
    println!("  Total detection time: {:.2}ms", total_time);

    // Run detection tests WITHOUT cropping (full image)
    println!("\n\n========== TEST WITHOUT CROPPING (FULL IMAGE) ==========\n");
    let mut results_full: Vec<DetectionResult> = Vec::new();

    for entry in &test_images {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        
        let img = match ImageReader::open(&path) {
            Ok(reader) => match reader.decode() {
                Ok(img) => img.to_luma8(),
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        let expected_fish = extract_expected_fish_name(filename)
            .unwrap_or_else(|| "unknown".to_string());

        // Run detection WITHOUT cropping
        let (detected_fish, confidence, detection_time) = 
            find_best_matching_fish(&img, &templates, false);

        let correct = detected_fish
            .as_ref()
            .map(|d| d == &expected_fish)
            .unwrap_or(false);

        results_full.push(DetectionResult {
            test_image: filename.to_string(),
            expected_fish,
            detected_fish,
            confidence,
            detection_time_ms: detection_time,
            correct,
        });
    }

    // Print results without cropping
    correct_count = 0;
    total_time = 0.0;

    println!("{:<45} | {:<25} | {:<25} | {:>8} | {:>10}",
        "Test Image", "Expected", "Detected", "Score", "Time (ms)");
    println!("{}", "-".repeat(125));

    for result in &results_full {
        let status = if result.correct { "✓" } else { "✗" };
        let detected = result.detected_fish.as_deref().unwrap_or("NONE");
        
        println!("{} {:<42} | {:<25} | {:<25} | {:>7.3} | {:>10.2}",
            status,
            result.test_image,
            result.expected_fish,
            detected,
            result.confidence,
            result.detection_time_ms
        );

        if result.correct {
            correct_count += 1;
        }
        total_time += result.detection_time_ms;
    }

    let accuracy_full = (correct_count as f64 / results_full.len() as f64) * 100.0;
    let avg_time_full = total_time / results_full.len() as f64;

    println!("\n{}", "=".repeat(125));
    println!("RESULTS WITHOUT CROPPING (FULL IMAGE):");
    println!("  Accuracy: {}/{} ({:.1}%)", correct_count, results_full.len(), accuracy_full);
    println!("  Average detection time: {:.2}ms", avg_time_full);
    println!("  Total detection time: {:.2}ms", total_time);

    // Compare results
    println!("\n\n========== COMPARISON ==========\n");
    println!("                    | WITH CROPPING | WITHOUT CROPPING | DIFFERENCE");
    println!("{}", "-".repeat(75));
    println!("Accuracy            | {:>12.1}% | {:>15.1}% | {:>+10.1}%", 
        accuracy, accuracy_full, accuracy - accuracy_full);
    println!("Avg Detection Time  | {:>11.2}ms | {:>14.2}ms | {:>+10.2}ms", 
        avg_time, avg_time_full, avg_time - avg_time_full);

    // Summary and recommendations
    println!("\n\n========== SUMMARY ==========\n");

    if accuracy >= 90.0 {
        println!("✓ Detection accuracy with cropping is EXCELLENT ({:.1}%)", accuracy);
    } else if accuracy >= 70.0 {
        println!("⚠ Detection accuracy with cropping is GOOD ({:.1}%), but could be improved", accuracy);
    } else {
        println!("✗ Detection accuracy with cropping is LOW ({:.1}%), needs tuning", accuracy);
    }

    if accuracy_full > accuracy {
        println!("⚠ Full image detection has higher accuracy - consider adjusting crop region");
        println!("  Suggestion: Try different FISH_CROP_* constants");
    }

    if avg_time < 100.0 {
        println!("✓ Detection speed is FAST ({:.2}ms average)", avg_time);
    } else if avg_time < 500.0 {
        println!("⚠ Detection speed is ACCEPTABLE ({:.2}ms average)", avg_time);
    } else {
        println!("✗ Detection speed is SLOW ({:.2}ms average), needs optimization", avg_time);
    }

    println!("\n========== END OF BENCHMARK ==========\n");

    // Assert minimum accuracy threshold
    assert!(
        accuracy >= 50.0 || accuracy_full >= 50.0,
        "Fish detection accuracy should be at least 50%"
    );
}

#[test]
fn test_template_loading_performance() {
    println!("\n========== TEMPLATE LOADING PERFORMANCE ==========\n");

    let fish_templates_dir = Path::new("images/1920x1080/fish");
    
    if !fish_templates_dir.exists() {
        println!("Skipping: Fish templates directory not found");
        return;
    }

    // First load (cold)
    let start = Instant::now();
    let templates = load_fish_templates(fish_templates_dir);
    let cold_load_time = start.elapsed().as_secs_f64() * 1000.0;

    println!("Cold load: {} templates in {:.2}ms", templates.len(), cold_load_time);

    // Second load (simulating cache hit - still from disk in this test)
    let start = Instant::now();
    let templates2 = load_fish_templates(fish_templates_dir);
    let second_load_time = start.elapsed().as_secs_f64() * 1000.0;

    println!("Second load: {} templates in {:.2}ms", templates2.len(), second_load_time);

    assert_eq!(templates.len(), templates2.len());
    println!("\n✓ Template loading works correctly\n");
}
