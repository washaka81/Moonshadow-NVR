# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import os
import random

data_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
annot_file = os.path.join(data_dir, "annotations.txt")
with open(annot_file, 'r') as f:
    lines = f.readlines()
print(f"Total lines: {len(lines)}")
# Shuffle
random.shuffle(lines)
split_idx = int(0.8 * len(lines))
train_lines = lines[:split_idx]
val_lines = lines[split_idx:]
print(f"Train: {len(train_lines)}, Val: {len(val_lines)}")
# Write to files
train_file = os.path.join(data_dir, "train.txt")
val_file = os.path.join(data_dir, "val.txt")
with open(train_file, 'w') as f:
    f.writelines(train_lines)
with open(val_file, 'w') as f:
    f.writelines(val_lines)
print(f"Written {train_file}, {val_file}")