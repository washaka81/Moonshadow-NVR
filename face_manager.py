# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

import os
import cv2
import numpy as np
import json

class FaceManager:
    def __init__(self):
        self.registered_dir = "models/face_data/registered"
        self.captures_dir = "models/face_data/captures"
        os.makedirs(self.registered_dir, exist_ok=True)
        os.makedirs(self.captures_dir, exist_ok=True)
        self.identities = self.load_identities()

    def load_identities(self):
        id_file = os.path.join(self.registered_dir, "identities.json")
        if os.path.exists(id_file):
            with open(id_file, "r") as f:
                return json.load(f)
        return {}

    def register_face(self, name, image_path, embedding=None):
        identity_id = str(len(self.identities) + 1)
        self.identities[identity_id] = {
            "name": name,
            "samples": [image_path],
            "embeddings": [embedding] if embedding is not None else []
        }
        self.save_identities()
        print(f"[*] Rostro registrado: {name} (ID: {identity_id})")
        return identity_id

    def update_identity(self, identity_id, image_path, embedding=None):
        if identity_id in self.identities:
            self.identities[identity_id]["samples"].append(image_path)
            if embedding is not None:
                self.identities[identity_id].setdefault("embeddings", []).append(embedding)
            
            # Limitar a los últimos 10 samples para evitar crecimiento infinito
            if len(self.identities[identity_id]["samples"]) > 10:
                self.identities[identity_id]["samples"].pop(0)
                if "embeddings" in self.identities[identity_id] and self.identities[identity_id]["embeddings"]:
                    self.identities[identity_id]["embeddings"].pop(0)
            
            self.save_identities()
            print(f"[*] Identidad {identity_id} actualizada con nuevo sample.")
            return True
        return False

    def identify_face(self, embedding, threshold=0.6):
        if not embedding:
            return None
        
        best_match = None
        min_dist = float("inf")
        
        for id_val, data in self.identities.items():
            for stored_emb in data.get("embeddings", []):
                if not stored_emb: continue
                # Distancia euclídea simple (suponiendo embeddings normalizados)
                dist = np.linalg.norm(np.array(embedding) - np.array(stored_emb))
                if dist < min_dist:
                    min_dist = dist
                    best_match = id_val
        
        if min_dist < threshold:
            return best_match
        return None

    def save_identities(self):
        id_file = os.path.join(self.registered_dir, "identities.json")
        with open(id_file, "w") as f:
            json.dump(self.identities, f, indent=2)
    def generate_heatmap(self, coordinates, frame_size):
        # Generar un mapa térmico visual
        heatmap = np.zeros((frame_size[1], frame_size[0]), dtype=np.float32)
        for (x, y) in coordinates:
            cv2.circle(heatmap, (x, y), 20, 1, -1)
        
        heatmap = cv2.GaussianBlur(heatmap, (51, 51), 0)
        heatmap = cv2.normalize(heatmap, None, 0, 255, cv2.NORM_MINMAX)
        heatmap = cv2.applyColorMap(heatmap.astype(np.uint8), cv2.COLORMAP_JET)
        return heatmap

if __name__ == "__main__":
    import sys
    fm = FaceManager()
    if len(sys.argv) > 1 and sys.argv[1] == "register":
        fm.register_face(sys.argv[2], sys.argv[3])
