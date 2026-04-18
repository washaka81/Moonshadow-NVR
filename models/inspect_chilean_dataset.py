# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import os
from PIL import Image
import matplotlib.pyplot as plt

data_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
annot_file = os.path.join(data_dir, "annotations.txt")
with open(annot_file, 'r') as f:
    lines = f.readlines()
print(f"Total annotations: {len(lines)}")
# Show first few
for line in lines[:5]:
    print(line.strip())
# Load first image
first_line = lines[0].strip()
path, label = first_line.split()
img_path = os.path.join(data_dir, path)
print(f"Image path: {img_path}")
if os.path.exists(img_path):
    img = Image.open(img_path)
    print(f"Image size: {img.size}, mode: {img.mode}")
    # Show histogram
    # img.show()
else:
    print("Image not found")
# Check a few random images
import random
for i in random.sample(range(len(lines)), 5):
    line = lines[i].strip()
    path, label = line.split()
    img_path = os.path.join(data_dir, path)
    if os.path.exists(img_path):
        img = Image.open(img_path)
        print(f"{label}: {img.size}")
    else:
        print(f"Missing: {img_path}")