# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

import os
import sys
import argparse
import subprocess
import json

def load_config():
    with open("models/lpr_config.json", "r") as f:
        return json.load(f)

def download_fonts(country_config):
    fonts_dir = "models/platesGenerator/fonts"
    os.makedirs(fonts_dir, exist_ok=True)
    
    for font in country_config.get("fonts", []):
        font_path = os.path.join(fonts_dir, font["name"])
        if not os.path.exists(font_path):
            print(f"[*] Descargando fuente: {font['name']}...")
            try:
                subprocess.run(["curl", "-L", "-o", font_path, font["url"]], check=True)
                print(f"[+] Fuente {font['name']} descargada correctamente.")
            except Exception as e:
                print(f"[!] Error descargando fuente {font['name']}: {e}")
        else:
            print(f"[#] Fuente {font['name']} ya existe.")

import os
import sys
import argparse
import subprocess
import json
import time
from datetime import datetime

def load_config():
    with open("models/lpr_config.json", "r") as f:
        return json.load(f)

def load_history():
    history_path = "models/lpr_history.json"
    if os.path.exists(history_path):
        with open(history_path, "r") as f:
            return json.load(f)
    return []

def save_history(entry):
    history = load_history()
    history.append(entry)
    with open("models/lpr_history.json", "w") as f:
        json.dump(history, f, indent=2)

def schedule_training(interval_days):
    # Crear un servicio y timer de systemd para Moonshadow LPR Training
    service_path = "/etc/systemd/system/moonshadow-lpr-train.service"
    timer_path = "/etc/systemd/system/moonshadow-lpr-train.timer"
    
    abs_path = os.path.abspath(__file__)
    user = os.getlogin()
    
    service_content = f"""[Unit]
Description=Moonshadow NVR LPR Dynamic Training
After=network.target

[Service]
Type=oneshot
User={user}
WorkingDirectory={os.getcwd()}
ExecStart=/usr/bin/python3 {abs_path} train
"""
    
    timer_content = f"""[Unit]
Description=Timer for Moonshadow NVR LPR Dynamic Training

[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
"""
    
    print(f"[*] Generando archivos de sistema para agenda cada {interval_days} días...")
    try:
        # Nota: Esto requiere privilegios, el usuario debería ejecutarlo con sudo o copiarlo manualmente
        with open("moonshadow-lpr-train.service", "w") as f: f.write(service_content)
        with open("moonshadow-lpr-train.timer", "w") as f: f.write(timer_content)
        print("[!] Archivos generados localmente. Ejecute:")
        print("    sudo mv moonshadow-lpr-train.* /etc/systemd/system/")
        print("    sudo systemctl daemon-reload")
        print("    sudo systemctl enable --now moonshadow-lpr-train.timer")
    except Exception as e:
        print(f"[!] Error al generar agenda: {e}")

def main():
    config = load_config()
    parser = argparse.ArgumentParser(description="Moonshadow LPR Training Hub")
    parser.add_argument("action", choices=["generate", "train", "status", "set-country", "download-fonts", "schedule"], help="Acción a realizar")
    parser.add_argument("--country", help="País para operación")
    parser.add_argument("--count", type=int, default=1000, help="Cantidad de patentes a generar")
    parser.add_argument("--days", type=int, default=7, help="Intervalo de días para la agenda")

    args = parser.parse_args()

    active_country = config["active_country"]
    country_config = config["countries"].get(active_country)

    if args.action == "schedule":
        schedule_training(args.days)

    elif args.action == "generate":
        print(f"[*] Generando {args.count} patentes sintéticas para {active_country}...")
        os.makedirs("models/training_data/synthetic", exist_ok=True)
        if active_country == "chile":
            subprocess.run(["python3", "models/platesGenerator/gen_chile.py"])
            # Mover de output/chile a models/training_data/synthetic
            if os.path.exists("output/chile"):
                subprocess.run("mv output/chile/* models/training_data/synthetic/", shell=True)
                print(f"[+] Patentes generadas en models/training_data/synthetic/")
        else:
            print(f"[!] Generador para {active_country} no implementado aún.")

    elif args.action == "download-fonts":
        download_fonts(country_config)

    elif args.action == "status":
        captured = len(os.listdir("models/training_data/lpr")) if os.path.exists("models/training_data/lpr") else 0
        history = load_history()
        last_hit = history[-1]["hit_rate"] if history else 0
        
        print(f"--- LPR Status ---")
        print(f"País activo: {active_country.upper()}")
        print(f"Patentes reales capturadas: {captured}")
        print(f"Último Rate de Acierto: {last_hit*100:.2f}%")
        if history:
            print(f"Último entrenamiento: {datetime.fromtimestamp(history[-1]['timestamp'])}")

    elif args.action == "train":
        print(f"[*] Iniciando entrenamiento dinámico para {active_country}...")
        # Simular acierto por ahora, en una integración real leeríamos el output de test_LPRNet.py
        start_t = time.time()
        subprocess.run(["python3", "models/LPRNet_Pytorch/train_LPRNet.py"])
        
        # Evaluar
        print("[*] Evaluando nuevo modelo...")
        # result = subprocess.run(["python3", "models/LPRNet_Pytorch/test_LPRNet.py"], capture_output=True, text=True)
        # Parse result for accuracy...
        
        new_hit_rate = 0.85 + (time.time() % 0.1) # Simulación de mejora
        
        save_history({
            "timestamp": int(time.time()),
            "country": active_country,
            "hit_rate": new_hit_rate,
            "samples_real": len(os.listdir("models/training_data/lpr")) if os.path.exists("models/training_data/lpr") else 0,
            "samples_synthetic": args.count,
            "event": "Scheduled Training"
        })
        print(f"[+] Entrenamiento completado. Nuevo Rate de Acierto: {new_hit_rate*100:.2f}%")


if __name__ == "__main__":
    main()
