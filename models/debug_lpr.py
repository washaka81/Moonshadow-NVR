import torch
import torch.nn as nn
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from chilean_dataset import ChileanLPRDataset
import os
import numpy as np

def decode_predictions(pred, blank_index):
    pred = pred.permute(0, 2, 1)
    pred = nn.functional.softmax(pred, dim=2)
    pred_indices = torch.argmax(pred, dim=2)
    batch_results = []
    for b in range(pred_indices.size(0)):
        indices = pred_indices[b].cpu().numpy()
        chars = []
        prev = None
        for idx in indices:
            if idx != blank_index and idx != prev:
                if idx < 68:
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

# Load CHARS from LPRNet_Pytorch
from data.load_data import CHARS
print(f"Total classes: {len(CHARS)}")
print(f"CHARS: {CHARS}")

root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
val_annotation = os.path.join(root_dir, "val.txt")
val_dataset = ChileanLPRDataset(root_dir, val_annotation, lpr_max_len=8)
print(f"Loaded {len(val_dataset)} validation samples")

device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
print(f"Device: {device}")

model_path = "chilean_lpr_final.pth"
state_dict = torch.load(model_path, map_location='cpu')
model = build_lprnet(lpr_max_len=8, phase=False, class_num=68, dropout_rate=0.5)
model.load_state_dict(state_dict)
model.to(device)
model.eval()

blank_index = 67

for i in range(5):
    img_tensor, indices_tensor, target_len, label = val_dataset[i]
    img_tensor = img_tensor.unsqueeze(0).to(device)
    with torch.no_grad():
        output = model(img_tensor)
    print(f"\nSample {i}: True label: {label}")
    print(f"Output shape: {output.shape}")
    # output shape (1, 68, 18)
    # apply softmax over classes dimension (dim=1)
    probs = nn.functional.softmax(output, dim=1)
    # get max indices per sequence position
    max_indices = torch.argmax(output, dim=1)
    print(f"Max indices per seq position: {max_indices[0].cpu().numpy()}")
    # decode using CTC greedy
    decoded = decode_predictions(output, blank_index)
    print(f"Decoded: {decoded[0]}")
    # also print top-3 classes per position
    for pos in range(18):
        topk = torch.topk(probs[0, :, pos], k=3)
        chars = [(CHARS[idx] if idx < len(CHARS) else '?', prob.item()) for idx, prob in zip(topk.indices.cpu(), topk.values.cpu())]
        print(f"Pos {pos:2d}: {chars}")
    print("---")