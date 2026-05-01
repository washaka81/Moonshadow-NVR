// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use sysinfo::{System, Disks, Components};
use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::sync::OnceLock;

static SYSTEM_STATE: OnceLock<Arc<Mutex<System>>> = OnceLock::new();

fn get_system() -> Arc<Mutex<System>> {
    SYSTEM_STATE.get_or_init(|| {
        let mut sys = System::new_all();
        sys.refresh_all();
        Arc::new(Mutex::new(sys))
    }).clone()
}

pub fn get_sysinfo_json() -> Value {
    let sys_arc = get_system();
    let mut sys = sys_arc.lock().unwrap();
    sys.refresh_all();

    // CPU Info
    let cpu_usage = sys.global_cpu_usage();
    let cpu_brand = sys.cpus().first().map(|c| c.brand()).unwrap_or("Intel Core");
    let cpu_cores: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    
    // GPU / iGPU usage (NVTOP / i915 style)
    let mut igpu_usage = 0.0;
    let mut igpu_status = "Idle";
    let mut vram_used = 0;
    let mut vram_total = 0;

    // Intel iGPU paths
    if let Ok(s) = fs::read_to_string("/sys/class/drm/card0/device/gpu_busy_percent") {
        igpu_usage = s.trim().parse::<f32>().unwrap_or(0.0);
    } 

    // Try to get VRAM info from sysfs (Intel specific)
    if let Ok(s) = fs::read_to_string("/sys/class/drm/card0/lmem_total_bytes") {
        vram_total = s.trim().parse::<u64>().unwrap_or(0) / 1024 / 1024;
    }
    if let Ok(s) = fs::read_to_string("/sys/class/drm/card0/lmem_avail_bytes") {
        let avail = s.trim().parse::<u64>().unwrap_or(0) / 1024 / 1024;
        vram_used = vram_total.saturating_sub(avail);
    }

    if igpu_usage > 1.0 {
        igpu_status = "Accelerating";
    }

    // Fastfetch style data
    let host = fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "localhost".to_string()).trim().to_string();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    
    // Detect package manager
    let pkgs = if Command::new("pacman").arg("-V").output().is_ok() {
        Command::new("sh").arg("-c").arg("pacman -Qq | wc -l").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|_| "0".to_string())
    } else {
        Command::new("sh").arg("-c").arg("dpkg -l | wc -l").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|_| "0".to_string())
    };

    let disks = Disks::new_with_refreshed_list();
    let disk_info = disks.iter()
        .find(|d| d.mount_point().to_str() == Some("/var/lib/moonshadow-nvr") || d.mount_point().to_str() == Some("/"))
        .map(|d| json!({
            "free": d.available_space() / 1024 / 1024 / 1024,
            "total": d.total_space() / 1024 / 1024 / 1024,
            "percent": (1.0 - (d.available_space() as f32 / d.total_space() as f32)) * 100.0
        }))
        .unwrap_or(json!({"free": 0, "total": 0, "percent": 0}));

    let components = Components::new_with_refreshed_list();
    let temp = components.iter()
        .find(|c| c.label().to_lowercase().contains("package") || c.label().to_lowercase().contains("core"))
        .map(|c| c.temperature().unwrap_or(0.0))
        .unwrap_or(0.0);

    json!({
        "fastfetch": {
            "host": host,
            "os": System::name().unwrap_or_else(|| "Linux".to_string()),
            "kernel": System::kernel_version().unwrap_or_else(|| "N/A".to_string()),
            "uptime": format!("{}h {}m", System::uptime() / 3600, (System::uptime() % 3600) / 60),
            "shell": shell,
            "packages": pkgs,
            "cpu_model": cpu_brand,
        },
        "htop": {
            "cpu_total": cpu_usage,
            "cpu_cores": cpu_cores,
            "mem_used": sys.used_memory() / 1024 / 1024,
            "mem_total": sys.total_memory() / 1024 / 1024,
            "mem_percent": (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0,
            "swap_used": sys.used_swap() / 1024 / 1024,
            "swap_total": sys.total_swap() / 1024 / 1024,
        },
        "nvtop": {
            "gpu_usage": igpu_usage,
            "gpu_status": igpu_status,
            "vram_used": vram_used,
            "vram_total": vram_total,
            "temp": temp,
        },
        "disk": disk_info,
        "accelerator": "Intel OpenVINO + Vulkan Compute"
        })
        }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysinfo_json() {
        let json = get_sysinfo_json();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        assert!(json.get("fastfetch").is_some());
        assert!(json.get("htop").is_some());
        assert!(json.get("nvtop").is_some());
    }
}
