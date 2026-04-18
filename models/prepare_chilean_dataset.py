# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import os
import shutil
from collections import defaultdict

data_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
output_dir = "chilean_plates"
os.makedirs(os.path.join(output_dir, "train"), exist_ok=True)
os.makedirs(os.path.join(output_dir, "val"), exist_ok=True)

# Count per plate for suffix
train_counts = defaultdict(int)
val_counts = defaultdict(int)

def process_split(split_file, split_name):
    with open(split_file, 'r') as f:
        lines = f.readlines()
    for line in lines:
        line = line.strip()
        if not line:
            continue
        path, label = line.split()
        src = os.path.join(data_dir, path)
        if not os.path.exists(src):
            print(f"Missing: {src}")
            continue
        # Determine suffix index
        if split_name == 'train':
            idx = train_counts[label]
            train_counts[label] += 1
        else:
            idx = val_counts[label]
            val_counts[label] += 1
        dst_filename = f"{label}.{idx}.jpg"
        dst = os.path.join(output_dir, split_name, dst_filename)
        shutil.copy2(src, dst)
    print(f"Processed {split_name}: {len(lines)} images")

process_split(os.path.join(data_dir, "train.txt"), "train")
process_split(os.path.join(data_dir, "val.txt"), "val")
print(f"Total train images: {sum(train_counts.values())}")
print(f"Total val images: {sum(val_counts.values())}")