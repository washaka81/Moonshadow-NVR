# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import sys
sys.path.append('LPRNet_Pytorch')
import torch
from model.LPRNet import build_lprnet
import onnx
import onnxruntime as ort

def main():
    lpr_max_len = 8
    num_classes = 68
    dropout_rate = 0.5
    device = torch.device("cpu")
    
    model = build_lprnet(lpr_max_len=lpr_max_len, phase=False, class_num=num_classes, dropout_rate=dropout_rate)
    model.load_state_dict(torch.load('chilean_lpr_best_full.pth', map_location='cpu'))
    model.to(device)
    model.eval()
    
    dummy_input = torch.randn(1, 3, 24, 94).to(device)
    torch.onnx.export(model, dummy_input, 'LPRNet_chilean.onnx',
                      input_names=['input'], output_names=['output'],
                      dynamic_axes={'input': {0: 'batch'}, 'output': {0: 'batch'}},
                      opset_version=11)
    print('ONNX model saved as LPRNet_chilean.onnx')
    
    # Validate ONNX model
    onnx_model = onnx.load('LPRNet_chilean.onnx')
    onnx.checker.check_model(onnx_model)
    print('ONNX model checked')
    
    ort_session = ort.InferenceSession('LPRNet_chilean.onnx')
    outputs = ort_session.run(None, {'input': dummy_input.numpy()})
    print(f'Output shape: {outputs[0].shape}')
    print('Conversion successful')

if __name__ == '__main__':
    main()