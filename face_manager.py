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

    def register_face(self, name, image_path):
        # En una implementación real, aquí se extraería el embedding
        # y se guardaría junto con la identidad
        identity_id = len(self.identities) + 1
        self.identities[identity_id] = {
            "name": name,
            "samples": [image_path]
        }
        with open(os.path.join(self.registered_dir, "identities.json"), "w") as f:
            json.dump(self.identities, f, indent=2)
        print(f"[*] Rostro registrado: {name} (ID: {identity_id})")

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
