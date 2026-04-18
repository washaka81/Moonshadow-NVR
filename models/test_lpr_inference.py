# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import onnxruntime as ort
import numpy as np
from PIL import Image

# Load model
session = ort.InferenceSession("LPRNet.onnx")
input_name = session.get_inputs()[0].name
output_name = session.get_outputs()[0].name
print(f"Input shape: {session.get_inputs()[0].shape}")
print(f"Output shape: {session.get_outputs()[0].shape}")

# Load test plate image
img = Image.open("test_plate.png").convert('RGB')
img = img.resize((94, 24))
img_np = np.array(img).astype(np.float32) / 255.0
# Convert to NCHW
input_data = np.transpose(img_np, (2,0,1))[np.newaxis, ...]  # (1,3,24,94)
print(f"Input data shape: {input_data.shape}")

# Run inference
output = session.run([output_name], {input_name: input_data})[0]
print(f"Output shape: {output.shape}")
# shape (1, 68, 18)

# Decode using same logic as Rust
shape = output.shape
batch, num_classes, seq_len = shape[0], shape[1], shape[2]
transposed = True  # our model outputs (batch, num_classes, seq_len)
blank_class = 67
plate_chars = []
for i in range(seq_len):
    max_prob = -np.inf
    max_idx = 0
    for c in range(num_classes):
        prob = output[0, c, i]
        if prob > max_prob:
            max_prob = prob
            max_idx = c
    if max_idx != blank_class and max_idx < 31:
        # Chinese character, ignore
        continue
    if max_idx != blank_class:
        if 31 <= max_idx <= 40:
            ch = chr(ord('0') + (max_idx - 31))
        elif 41 <= max_idx <= 66:
            ch = chr(ord('A') + (max_idx - 41))
        else:
            continue
        plate_chars.append(ch)
# Merge consecutive duplicates
plate = ""
prev_char = None
for ch in plate_chars:
    if ch != prev_char:
        plate += ch
        prev_char = ch
if plate == "":
    plate = "UNKNOWN"
print(f"Decoded plate: {plate}")

# Also compute softmax and show top classes per position (optional)
import scipy.special
softmax = scipy.special.softmax(output[0, :, :], axis=0)  # across classes? axis=0 is classes? Actually shape (68,18), softmax across classes axis=0
print("\nTop predictions per position:")
for i in range(seq_len):
    top5 = np.argsort(softmax[:, i])[-5:][::-1]
    top5_probs = softmax[top5, i]
    chars = []
    for idx in top5:
        if idx == 67:
            chars.append('-')
        elif idx < 31:
            chars.append('C')
        elif idx <= 40:
            chars.append(chr(ord('0') + (idx - 31)))
        elif idx <= 66:
            chars.append(chr(ord('A') + (idx - 41)))
        else:
            chars.append('?')
    print(f"Pos {i:2d}: {list(zip(top5, chars, top5_probs))}")