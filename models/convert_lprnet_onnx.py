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
# Build model with correct class_num (68)
model = build_lprnet(lpr_max_len=8, phase=False, class_num=68, dropout_rate=0.5)
model.load_state_dict(state_dict)
model.eval()
print("Model loaded successfully")

# Input shape: (batch, 3, 24, 94)
dummy_input = torch.randn(1, 3, 24, 94)
with torch.no_grad():
    output = model(dummy_input)
print(f"Output shape: {output.shape}")  # Expected (1, 68, 18) maybe

# Export to ONNX
onnx_path = "LPRNet_chinese.onnx"
torch.onnx.export(
    model,
    dummy_input,
    onnx_path,
    export_params=True,
    opset_version=11,
    do_constant_folding=True,
    input_names=['input'],
    output_names=['output'],
    dynamic_axes={'input': {0: 'batch_size'}, 'output': {0: 'batch_size'}}
)
print(f"Exported ONNX model to {onnx_path}")

# Verify ONNX model
import onnx
onnx_model = onnx.load(onnx_path)
print(f"ONNX model inputs: {[i.name for i in onnx_model.graph.input]}")
print(f"ONNX model outputs: {[o.name for o in onnx_model.graph.output]}")
# Check output shape
import onnxruntime as ort
session = ort.InferenceSession(onnx_path)
input_name = session.get_inputs()[0].name
output_name = session.get_outputs()[0].name
print(f"ORT input shape: {session.get_inputs()[0].shape}")
print(f"ORT output shape: {session.get_outputs()[0].shape}")

# Test inference
input_data = dummy_input.numpy()
output = session.run([output_name], {input_name: input_data})[0]
print(f"Test output shape: {output.shape}")
print("Conversion successful.")