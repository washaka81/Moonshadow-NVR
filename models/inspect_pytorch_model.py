# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import torch
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet

model_path = "LPRNet_Pytorch/weights/Final_LPRNet_model.pth"
print(f"Loading model from {model_path}")
state_dict = torch.load(model_path, map_location='cpu')
print("State dict keys:", state_dict.keys())
# Load model architecture
model = build_lprnet(lpr_max_len=8, phase=False, class_num=66, dropout_rate=0.5)
model.load_state_dict(state_dict)
model.eval()
print("Model loaded successfully")
# Print final layer weight shape
for name, param in model.named_parameters():
    if 'container' in name or 'backbone' in name and param.dim() == 4:
        print(f"{name}: {param.shape}")
# Determine input shape
dummy = torch.randn(1, 3, 24, 94)
with torch.no_grad():
    output = model(dummy)
print(f"Output shape: {output.shape}")
print(f"Output min/max: {output.min()}, {output.max()}")
# Check class indices mapping
# CHARS list from load_data
CHARS = ['京', '沪', '津', '渝', '冀', '晋', '蒙', '辽', '吉', '黑',
         '苏', '浙', '皖', '闽', '赣', '鲁', '豫', '鄂', '湘', '粤',
         '桂', '琼', '川', '贵', '云', '藏', '陕', '甘', '青', '宁',
         '新',
         '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
         'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K',
         'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V',
         'W', 'X', 'Y', 'Z', 'I', 'O', '-'
         ]
print(f"Total CHARS: {len(CHARS)}")
# Find indices of digits and letters
for i, ch in enumerate(CHARS):
    if ch.isalnum():
        print(f"{ch}: {i}")