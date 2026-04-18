# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import sys
sys.path.append('LPRNet_Pytorch')
from torch.utils.data import DataLoader
from chilean_dataset import ChileanLPRDataset
import os
import time

root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
train_annotation = os.path.join(root_dir, "train.txt")
print("Loading dataset...")
dataset = ChileanLPRDataset(root_dir, train_annotation, lpr_max_len=8)
print(f"Dataset size: {len(dataset)}")
loader = DataLoader(dataset, batch_size=32, shuffle=True, num_workers=0)
print("Starting iteration...")
start = time.time()
for i, batch in enumerate(loader):
    images, indices, target_lens, labels = batch
    print(f"Batch {i}: images {images.shape}, indices {indices.shape}, target_lens {target_lens.shape}, labels {len(labels)}")
    if i >= 5:
        break
print(f"Iteration took {time.time() - start:.2f}s")