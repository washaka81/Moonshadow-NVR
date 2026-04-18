# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import torch
import torch.nn as nn
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from chilean_dataset import ChileanLPRDataset
from data.load_data import CHARS
import os

root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
val_annotation = os.path.join(root_dir, "val.txt")
val_dataset = ChileanLPRDataset(root_dir, val_annotation, lpr_max_len=8)
print(f"Loaded {len(val_dataset)} samples")

device = torch.device("cpu")
num_classes = len(CHARS)
blank_index = num_classes - 1

model_path = "LPRNet_Pytorch/weights/Final_LPRNet_model.pth"
state_dict = torch.load(model_path, map_location='cpu')
model = build_lprnet(lpr_max_len=8, phase=False, class_num=num_classes, dropout_rate=0.5)
model.load_state_dict(state_dict)
model.to(device)
model.eval()

# Get a single batch
from torch.utils.data import DataLoader
loader = DataLoader(val_dataset, batch_size=2, shuffle=False)
images, indices, target_lens, labels = next(iter(loader))
print(f"Images shape: {images.shape}")
print(f"Indices shape: {indices.shape}")
print(f"Target lengths: {target_lens}")
print(f"Labels: {labels}")

outputs = model(images)
print(f"Outputs shape: {outputs.shape}")

# Compute CTC loss
criterion = nn.CTCLoss(blank=blank_index, zero_infinity=True)
outputs_perm = outputs.permute(2, 0, 1)  # (seq_len, batch, num_classes)
targets_flat = []
for i in range(indices.size(0)):
    targets_flat.append(indices[i, :target_lens[i]])
targets_flat = torch.cat(targets_flat)
input_lengths = torch.full((outputs_perm.size(1),), outputs_perm.size(0), dtype=torch.long)
loss = criterion(outputs_perm.log_softmax(2), targets_flat, input_lengths, target_lens)
print(f"CTC loss: {loss.item()}")

# Decode predictions
pred = outputs.permute(0, 2, 1)  # (batch, seq_len, num_classes)
pred = nn.functional.softmax(pred, dim=2)
pred_indices = torch.argmax(pred, dim=2)
for b in range(pred_indices.size(0)):
    indices_seq = pred_indices[b].cpu().numpy()
    chars = []
    prev = None
    for idx in indices_seq:
        if idx != blank_index and idx != prev:
            if idx < len(CHARS):
                chars.append(CHARS[idx])
            prev = idx
    print(f"Predicted: {''.join(chars)}")
    print(f"True: {labels[b]}")