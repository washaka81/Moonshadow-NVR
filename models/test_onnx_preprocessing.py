# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import onnxruntime as ort
import numpy as np
from PIL import Image
import sys
sys.path.append('LPRNet_Pytorch')
from data.load_data import CHARS
import os

def decode_output(output, blank_index=67):
    # output shape (1, 68, 18)
    seq_len = output.shape[2]
    num_classes = output.shape[1]
    plate_chars = []
    for i in range(seq_len):
        max_prob = -np.inf
        max_idx = 0
        for c in range(num_classes):
            prob = output[0, c, i]
            if prob > max_prob:
                max_prob = prob
                max_idx = c
        if max_idx != blank_index and max_idx < 31:
            continue  # ignore Chinese
        if max_idx != blank_index:
            if 31 <= max_idx <= 40:
                ch = chr(ord('0') + (max_idx - 31))
            elif 41 <= max_idx <= 66:
                ch = chr(ord('A') + (max_idx - 41))
            else:
                continue
            plate_chars.append(ch)
    # CTC greedy merge duplicates
    plate = ''
    prev = None
    for ch in plate_chars:
        if ch != prev:
            plate += ch
            prev = ch
    return plate if plate else 'UNKNOWN'

def main():
    # Load ONNX model
    sess = ort.InferenceSession('LPRNet_chilean.onnx')
    # Load a sample image from validation set
    root = 'license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates'
    val_file = os.path.join(root, 'val.txt')
    with open(val_file, 'r') as f:
        lines = f.readlines()
    # Take first image
    first = lines[0].strip().split()
    img_path = os.path.join(root, first[0])
    label = first[1]
    print(f'Testing image: {img_path}, true label: {label}')
    img = Image.open(img_path).convert('RGB')
    img = img.resize((94,24))
    # Convert to BGR and normalize as in Rust
    img_np = np.array(img).astype(np.float32)  # RGB, 0-255
    # BGR
    bgr = img_np[:, :, [2,1,0]]
    # Normalize (img - 127.5) * 0.0078125
    bgr = (bgr - 127.5) * 0.0078125
    # CHW
    input_tensor = bgr.transpose(2,0,1)[np.newaxis, ...]  # (1,3,24,94)
    
    outputs = sess.run(None, {'input': input_tensor})
    output = outputs[0]  # (1,68,18)
    print('Output shape:', output.shape)
    plate = decode_output(output)
    print(f'Predicted plate: {plate}')
    
    # Also test with original preprocessing (RGB /255) for comparison
    input_rgb = img_np.transpose(2,0,1)[np.newaxis, ...] / 255.0
    outputs2 = sess.run(None, {'input': input_rgb})
    plate2 = decode_output(outputs2[0])
    print(f'Predicted plate (RGB/255): {plate2}')

if __name__ == '__main__':
    main()