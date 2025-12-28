#!/usr/bin/env python3
"""
Fish Detection Benchmark Test

This script tests the fish detection accuracy using test images from tests/assets/1920x1080.
Each test image is named `{expected_fish_name}_test_1920x1080.png` and should be detected
as the corresponding fish type.

This mirrors the Rust benchmark in tests/fish_detection_benchmark.rs and can be used to:
1. Validate detection accuracy before running the Rust version
2. Compare results between Python OCR and Rust template matching approaches
3. Tune the detection parameters

Run with: python tests/benchmark_fish_detection.py
"""

import os
import sys
import time
import cv2
import numpy as np
from pathlib import Path
from typing import Optional, Tuple, Dict, List
from dataclasses import dataclass

# Fish detection crop region constants - should match Rust image_service.rs
# Based on benchmark analysis of test images, fish templates appear at:
# - X: 23% to 46% of screen width (lower-left area)
# - Y: 68% to 99% of screen height (bottom portion)
FISH_CROP_X_START = 0.20
FISH_CROP_Y_START = 0.65
FISH_CROP_WIDTH = 0.30
FISH_CROP_HEIGHT = 0.35

@dataclass
class DetectionResult:
    """Result of a single fish detection test"""
    test_image: str
    expected_fish: str
    detected_fish: Optional[str]
    confidence: float
    detection_time_ms: float
    correct: bool


def load_fish_templates(fish_folder: Path) -> Dict[str, Tuple[np.ndarray, Optional[np.ndarray]]]:
    """Load all fish templates from a folder"""
    templates = {}
    
    if not fish_folder.exists():
        return templates
    
    for path in fish_folder.glob("*.png"):
        fish_name = path.stem
        
        # Load template with alpha channel
        template_img = cv2.imread(str(path), cv2.IMREAD_UNCHANGED)
        if template_img is None:
            continue
        
        # Extract grayscale template and optional mask
        if template_img.shape[2] == 4:
            # RGBA image - extract grayscale and mask
            bgr = template_img[:, :, :3]
            alpha = template_img[:, :, 3]
            gray = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY)
            _, mask = cv2.threshold(alpha, 1, 255, cv2.THRESH_BINARY)
            templates[fish_name] = (gray, mask)
        elif template_img.shape[2] == 3:
            # BGR image
            gray = cv2.cvtColor(template_img, cv2.COLOR_BGR2GRAY)
            templates[fish_name] = (gray, None)
        else:
            templates[fish_name] = (template_img, None)
    
    return templates


def find_best_matching_fish(
    img: np.ndarray,
    templates: Dict[str, Tuple[np.ndarray, Optional[np.ndarray]]],
    use_cropping: bool = True
) -> Tuple[Optional[str], float, float]:
    """Find best matching fish in a grayscale image using template matching"""
    start_time = time.time()
    
    h, w = img.shape[:2]
    
    # Convert to grayscale if needed
    if len(img.shape) == 3:
        img_gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    else:
        img_gray = img
    
    # Optionally crop the image to the fish result region
    if use_cropping:
        crop_x1 = int(w * FISH_CROP_X_START)
        crop_y1 = int(h * FISH_CROP_Y_START)
        crop_w = int(w * FISH_CROP_WIDTH)
        crop_h = int(h * FISH_CROP_HEIGHT)
        
        # Ensure crop region doesn't exceed image boundaries
        crop_w = min(crop_w, w - crop_x1)
        crop_h = min(crop_h, h - crop_y1)
        
        img_to_process = img_gray[crop_y1:crop_y1+crop_h, crop_x1:crop_x1+crop_w]
    else:
        img_to_process = img_gray
    
    best_fish = None
    best_score = 0.0
    
    for fish_name, (template, mask) in templates.items():
        # Skip if template is larger than image
        if template.shape[0] >= img_to_process.shape[0] or template.shape[1] >= img_to_process.shape[1]:
            continue
        
        # Perform template matching
        try:
            if mask is not None:
                result = cv2.matchTemplate(img_to_process, template, cv2.TM_CCOEFF_NORMED, mask=mask)
            else:
                result = cv2.matchTemplate(img_to_process, template, cv2.TM_CCOEFF_NORMED)
            
            _, max_val, _, _ = cv2.minMaxLoc(result)
            
            if max_val > best_score:
                best_score = max_val
                best_fish = fish_name
        except cv2.error:
            continue
    
    detection_time = (time.time() - start_time) * 1000  # Convert to ms
    return best_fish, best_score, detection_time


def extract_expected_fish_name(filename: str) -> Optional[str]:
    """Extract expected fish name from test image filename"""
    # Format: {fish_name}_test_1920x1080.png
    if not filename.endswith("_test_1920x1080.png"):
        return None
    return filename.replace("_test_1920x1080.png", "")


def run_benchmark():
    """Run the fish detection benchmark"""
    print("\n========== FISH DETECTION BENCHMARK (Python) ==========\n")
    
    # Define paths
    base_dir = Path(__file__).parent.parent
    test_images_dir = base_dir / "tests" / "assets" / "1920x1080"
    fish_templates_dir = base_dir / "images" / "1920x1080" / "fish"
    
    # Check if directories exist
    if not test_images_dir.exists():
        print(f"ERROR: Test images directory not found: {test_images_dir}")
        print("Please ensure test images are in tests/assets/1920x1080/")
        return
    
    if not fish_templates_dir.exists():
        print(f"ERROR: Fish templates directory not found: {fish_templates_dir}")
        print("Please ensure fish templates are in images/1920x1080/fish/")
        return
    
    # Load all fish templates
    print(f"Loading fish templates from {fish_templates_dir}...")
    templates_start = time.time()
    templates = load_fish_templates(fish_templates_dir)
    templates_load_time = (time.time() - templates_start) * 1000
    print(f"Loaded {len(templates)} templates in {templates_load_time:.2f}ms\n")
    
    # List available templates
    print("Available templates:")
    for name in sorted(templates.keys()):
        print(f"  - {name}")
    print()
    
    # Collect test images
    test_images = list(test_images_dir.glob("*.png"))
    print(f"Found {len(test_images)} test images\n")
    
    # Run detection tests WITH cropping (optimized)
    print("========== TEST WITH CROPPING (OPTIMIZED) ==========\n")
    results_cropped: List[DetectionResult] = []
    
    for path in test_images:
        filename = path.name
        
        # Load test image
        img = cv2.imread(str(path))
        if img is None:
            print(f"  Failed to load {filename}")
            continue
        
        # Extract expected fish name
        expected_fish = extract_expected_fish_name(filename) or "unknown"
        
        # Run detection with cropping
        detected_fish, confidence, detection_time = find_best_matching_fish(img, templates, use_cropping=True)
        
        correct = detected_fish == expected_fish if detected_fish else False
        
        results_cropped.append(DetectionResult(
            test_image=filename,
            expected_fish=expected_fish,
            detected_fish=detected_fish,
            confidence=confidence,
            detection_time_ms=detection_time,
            correct=correct
        ))
    
    # Print results with cropping
    print(f"{'Test Image':<45} | {'Expected':<25} | {'Detected':<25} | {'Score':>8} | {'Time (ms)':>10}")
    print("-" * 125)
    
    correct_count = 0
    total_time = 0.0
    
    for result in results_cropped:
        status = "✓" if result.correct else "✗"
        detected = result.detected_fish or "NONE"
        
        print(f"{status} {result.test_image:<42} | {result.expected_fish:<25} | {detected:<25} | {result.confidence:>7.3f} | {result.detection_time_ms:>10.2f}")
        
        if result.correct:
            correct_count += 1
        total_time += result.detection_time_ms
    
    accuracy = (correct_count / len(results_cropped)) * 100 if results_cropped else 0
    avg_time = total_time / len(results_cropped) if results_cropped else 0
    
    print(f"\n{'=' * 125}")
    print("RESULTS WITH CROPPING:")
    print(f"  Accuracy: {correct_count}/{len(results_cropped)} ({accuracy:.1f}%)")
    print(f"  Average detection time: {avg_time:.2f}ms")
    print(f"  Total detection time: {total_time:.2f}ms")
    
    # Run detection tests WITHOUT cropping (full image)
    print("\n\n========== TEST WITHOUT CROPPING (FULL IMAGE) ==========\n")
    results_full: List[DetectionResult] = []
    
    for path in test_images:
        filename = path.name
        
        img = cv2.imread(str(path))
        if img is None:
            continue
        
        expected_fish = extract_expected_fish_name(filename) or "unknown"
        
        # Run detection WITHOUT cropping
        detected_fish, confidence, detection_time = find_best_matching_fish(img, templates, use_cropping=False)
        
        correct = detected_fish == expected_fish if detected_fish else False
        
        results_full.append(DetectionResult(
            test_image=filename,
            expected_fish=expected_fish,
            detected_fish=detected_fish,
            confidence=confidence,
            detection_time_ms=detection_time,
            correct=correct
        ))
    
    # Print results without cropping
    print(f"{'Test Image':<45} | {'Expected':<25} | {'Detected':<25} | {'Score':>8} | {'Time (ms)':>10}")
    print("-" * 125)
    
    correct_count_full = 0
    total_time_full = 0.0
    
    for result in results_full:
        status = "✓" if result.correct else "✗"
        detected = result.detected_fish or "NONE"
        
        print(f"{status} {result.test_image:<42} | {result.expected_fish:<25} | {detected:<25} | {result.confidence:>7.3f} | {result.detection_time_ms:>10.2f}")
        
        if result.correct:
            correct_count_full += 1
        total_time_full += result.detection_time_ms
    
    accuracy_full = (correct_count_full / len(results_full)) * 100 if results_full else 0
    avg_time_full = total_time_full / len(results_full) if results_full else 0
    
    print(f"\n{'=' * 125}")
    print("RESULTS WITHOUT CROPPING (FULL IMAGE):")
    print(f"  Accuracy: {correct_count_full}/{len(results_full)} ({accuracy_full:.1f}%)")
    print(f"  Average detection time: {avg_time_full:.2f}ms")
    print(f"  Total detection time: {total_time_full:.2f}ms")
    
    # Compare results
    print("\n\n========== COMPARISON ==========\n")
    print("                    | WITH CROPPING | WITHOUT CROPPING | DIFFERENCE")
    print("-" * 75)
    print(f"Accuracy            | {accuracy:>12.1f}% | {accuracy_full:>15.1f}% | {accuracy - accuracy_full:>+10.1f}%")
    print(f"Avg Detection Time  | {avg_time:>11.2f}ms | {avg_time_full:>14.2f}ms | {avg_time - avg_time_full:>+10.2f}ms")
    
    # Summary and recommendations
    print("\n\n========== SUMMARY ==========\n")
    
    if accuracy >= 90.0:
        print(f"✓ Detection accuracy with cropping is EXCELLENT ({accuracy:.1f}%)")
    elif accuracy >= 70.0:
        print(f"⚠ Detection accuracy with cropping is GOOD ({accuracy:.1f}%), but could be improved")
    else:
        print(f"✗ Detection accuracy with cropping is LOW ({accuracy:.1f}%), needs tuning")
    
    if accuracy_full > accuracy:
        print("⚠ Full image detection has higher accuracy - consider adjusting crop region")
        print("  Suggestion: Try different FISH_CROP_* constants")
    
    if avg_time < 100.0:
        print(f"✓ Detection speed is FAST ({avg_time:.2f}ms average)")
    elif avg_time < 500.0:
        print(f"⚠ Detection speed is ACCEPTABLE ({avg_time:.2f}ms average)")
    else:
        print(f"✗ Detection speed is SLOW ({avg_time:.2f}ms average), needs optimization")
    
    # Find misdetections for analysis
    print("\n\n========== MISDETECTIONS ANALYSIS ==========\n")
    misdetections = [r for r in results_cropped if not r.correct]
    if misdetections:
        print(f"Found {len(misdetections)} misdetections with cropping:\n")
        for r in misdetections:
            detected = r.detected_fish or "NONE"
            print(f"  {r.test_image}")
            print(f"    Expected: {r.expected_fish}")
            print(f"    Detected: {detected} (score: {r.confidence:.3f})")
            print()
    else:
        print("No misdetections with cropping! 🎉")
    
    print("\n========== END OF BENCHMARK ==========\n")
    
    return accuracy, accuracy_full


if __name__ == "__main__":
    run_benchmark()
