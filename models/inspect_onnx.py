# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import onnxruntime as ort
import onnx
import numpy as np

model_path = "european_lpr.onnx"
model = onnx.load(model_path)
print("=== ONNX Model ===")
print(f"Inputs: {[i.name for i in model.graph.input]}")
print(f"Outputs: {[o.name for o in model.graph.output]}")
for inp in model.graph.input:
    print(f"  Input {inp.name}: {inp.type.tensor_type.shape}")
for out in model.graph.output:
    print(f"  Output {out.name}: {out.type.tensor_type.shape}")

# Use ONNX Runtime
session = ort.InferenceSession(model_path)
input_name = session.get_inputs()[0].name
output_names = [o.name for o in session.get_outputs()]
print(f"\nORT Input: {input_name} shape {session.get_inputs()[0].shape}")
for out in session.get_outputs():
    print(f"ORT Output: {out.name} shape {out.shape}")

# Test with dummy input
input_shape = session.get_inputs()[0].shape
# Replace batch dimension with 1
input_shape = tuple([1 if dim == 'batch' else dim for dim in input_shape])
# Assume 3 channel image
if len(input_shape) == 4:
    # NCHW
    dummy = np.random.randn(*input_shape).astype(np.float32)
    outputs = session.run(output_names, {input_name: dummy})
    for name, arr in zip(output_names, outputs):
        print(f"\nOutput '{name}' shape: {arr.shape}")
        print(f"  min={arr.min():.3f}, max={arr.max():.3f}, mean={arr.mean():.3f}")
        # If shape suggests class probabilities, show top classes
        if len(arr.shape) == 2 and arr.shape[1] > 1:
            # assume batch, classes
            probs = arr[0]
            top5 = np.argsort(probs)[-5:][::-1]
            print(f"  Top5 class indices: {top5}")
            print(f"  Top5 probabilities: {probs[top5]}")
else:
    print("Unexpected input shape")