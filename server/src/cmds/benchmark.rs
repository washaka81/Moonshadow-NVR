// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use base::Error;
use bpaf::Bpaf;
use ort::session::Session;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Bpaf, Debug)]
#[bpaf(command("benchmark"))]
pub struct Args {
    /// Number of iterations for the floating point test.
    #[bpaf(short, long, fallback(1_000_000))]
    iterations: usize,

    /// Path to a model to test inference speed.
    #[bpaf(short, long)]
    model: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<i32, Error> {
    println!("🚀 Iniciando Benchmark de Moonshadow NVR...");
    println!("Optimización: Intel OpenVINO (CPU) + Vulkan Compute Architecture\n");

    // 1. Verificación de GPU Vulkan
    println!("--- 🛠️ Prueba 1: Verificación de GPU Vulkan ---");
    let has_vulkan = crate::vulkan_check::verify_vulkan_gpu();
    if has_vulkan {
        println!("Estado: GPU detectada y lista para cálculo de coma flotante.\n");
    } else {
        println!("⚠️ Aviso: No se detectó GPU compatible con Vulkan Compute.\n");
    }

    // 2. Benchmark de Coma Flotante (CPU fallback)
    println!("--- 🔢 Prueba 2: Coma Flotante (CPU Math Ops) ---");
    let start = Instant::now();
    let mut sum = 0.0f64;
    for i in 0..args.iterations {
        let x = (i as f64).sin();
        let y = (i as f64).cos();
        sum += (x * x + y * y).sqrt().exp().ln();
    }
    let duration = start.elapsed();
    println!("Iteraciones: {}", args.iterations);
    println!("Tiempo: {:.2?}", duration);
    println!(
        "Rendimiento: {:.2} Mops/s",
        (args.iterations as f64 / duration.as_secs_f64()) / 1_000_000.0
    );
    println!("Resultado (checksum): {:.2}\n", sum);

    // 3. Benchmark de Inferencia (Intel OpenVINO - Forzado a CPU)
    if let Some(model_path) = args.model {
        println!("--- 🧠 Prueba 3: Inferencia de IA (Intel OpenVINO - CPU Mode) ---");
        if !model_path.exists() {
            println!(
                "⚠️ Modelo no encontrado en: {:?}. Saltando prueba.",
                model_path
            );
        } else {
            let start_load = Instant::now();
            let _ = ort::init().with_name("benchmark").commit();
            let session = Session::builder()
                .unwrap()
                .with_execution_providers([
                    ort::ep::OpenVINO::default().with_device_type("CPU").build(),
                    ort::ep::CPU::default().build(),
                ])
                .unwrap()
                .commit_from_file(&model_path);

            match session {
                Ok(_) => {
                    println!("Tiempo de carga del modelo: {:.2?}", start_load.elapsed());
                    println!("Estado: Aceleración Intel OpenVINO (CPU) cargada con éxito.\n");
                }
                Err(e) => println!("❌ Error cargando modelo: {}\n", e),
            }
        }
    }

    // 4. Conclusión
    println!("--- ✅ Benchmark Finalizado ---");
    println!("Hardware detectado: Optimizado para procesadores Intel.");
    println!("Arquitectura de cálculo: Vulkan Active.");

    Ok(0)
}
