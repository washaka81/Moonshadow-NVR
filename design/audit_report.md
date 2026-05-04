# Moonshadow NVR - Codebase Audit Report

## 1. Architecture Overview
Moonshadow NVR utilizes a Rust backend focused on hardware-accelerated AI (OpenVINO, CUDA, Vulkan). It manages video storage via a custom SQLite schema and serves a React frontend. The system includes a dynamic retraining loop for License Plate Recognition (LPR) managed by Python scripts.

## 2. Optimization Opportunities

### 2.1 Backend (Rust)
*   **Video Decoding Bottleneck:** The `Detector::decode_any_codec_to_image` function in `server/src/detector.rs` is a major bottleneck. It currently writes H.264/H.265 data to `/tmp`, calls the `ffmpeg` system binary as a sub-process, and reads a PNG back for every frame processed.
    *   **Recommendation:** Replace this with in-process decoding using libraries like `ffmpeg-next` or `openh264` to eliminate sub-process overhead and disk I/O.
*   **Vulkan Preprocessing:** The compute shader in `server/src/vulkan_engine.rs` uses nearest-neighbor sampling for image scaling.
    *   **Recommendation:** Implement bilinear interpolation in the compute shader to improve AI detection accuracy with minimal performance cost.

### 2.2 Frontend (React)
*   **Buffer Management:** The `LiveCamera.tsx` component manually manages the `MediaSource` buffer frequently (`tryTrimBuffer`).
    *   **Recommendation:** Transitioning to `ManagedMediaSource` (where supported) or optimizing the frequency of buffer trimming could reduce CPU usage in the browser.

## 3. Security Vulnerabilities

### 3.1 Unsafe Unwraps (Denial of Service Risk)
*   Numerous `.unwrap()` calls exist outside of test code, particularly in `server/db/auth.rs` and `server/src/detector.rs`. A corrupted database state or malformed model output could cause the entire server to panic and crash.
    *   **Recommendation:** Implement proper global error handling using `Result` propagation instead of unwrapping.

### 3.2 Command Injection / Path Traversal Risk
*   The `decode_any_codec_to_image` function constructs filenames in `/tmp` using camera IDs. While these are currently integers, any future change exposing this to unsanitized user input could lead to command injection or path traversal vulnerabilities.
    *   **Recommendation:** Ensure camera IDs are strictly typed and sanitized before being used in file paths or commands.

### 3.3 Symlink Creation Risks
*   The `repair_openvino_bridge` function creates symlinks in the executable directory based on system library paths.
    *   **Recommendation:** Restrict and validate the target paths to ensure this mechanism cannot be exploited to overwrite critical system files.

## 4. Bugs and Code Smells

### 4.1 Fragile External Dependencies
*   Relying on the system-level `ffmpeg` binary for core AI functionality makes deployment fragile and dependent on the host environment's configuration.

### 4.2 Hardcoded Constants
*   Disk block size is hardcoded to `4096` in `db.rs`. This may lead to inaccurate disk space accounting on filesystems with different block sizes.

### 4.3 Incomplete Implementations
*   Several AI methods in `detector.rs` (e.g., `reid` and `detect_faces`) are currently placeholders that return empty results.
