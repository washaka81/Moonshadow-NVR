import torch
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from data.load_data import CHARS, CHARS_DICT
import os
from PIL import Image
import numpy as np
import torch.nn.functional as F

# Load pre-trained model
model_path = "LPRNet_Pytorch/weights/Final_LPRNet_model.pth"
print(f"Loading model from {model_path}")
state_dict = torch.load(model_path, map_location='cpu')
model = build_lprnet(lpr_max_len=8, phase=False, class_num=68, dropout_rate=0.5)
model.load_state_dict(state_dict)
model.eval()
print("Model loaded")

# Dataset paths
data_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
annot_file = os.path.join(data_dir, "annotations.txt")
with open(annot_file, 'r') as f:
    lines = f.readlines()

# Shuffle and split for quick eval
import random
random.shuffle(lines)
eval_lines = lines[:100]  # evaluate on 100 samples

def decode_predictions(pred):
    # pred shape (batch, num_classes, seq_len) = (1, 68, 18)
    # Greedy decoding: take argmax over classes dimension
    pred = pred.permute(0, 2, 1)  # (batch, seq_len, num_classes)
    pred = F.softmax(pred, dim=2)
    pred_indices = torch.argmax(pred, dim=2)  # (batch, seq_len)
    # Merge repeated characters and remove blank (67)
    batch_results = []
    for b in range(pred_indices.size(0)):
        indices = pred_indices[b].cpu().numpy()
        chars = []
        prev = None
        for idx in indices:
            if idx != 67 and idx != prev:  # blank index 67
                if idx < len(CHARS):
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

correct = 0
total = 0
for line in eval_lines:
    line = line.strip()
    if not line:
        continue
    path, label = line.split()
    img_path = os.path.join(data_dir, path)
    if not os.path.exists(img_path):
        print(f"Missing: {img_path}")
        continue
    img = Image.open(img_path).convert('RGB')
    # Resize to 94x24 (already)
    img = img.resize((94, 24))
    # Convert to tensor and normalize
    img_np = np.array(img).astype(np.float32) / 255.0
    img_np = (img_np - 0.5) / 0.5  # normalize to [-1,1] maybe
    # CHW
    img_tensor = torch.from_numpy(img_np).permute(2,0,1).unsqueeze(0)  # (1,3,24,94)
    with torch.no_grad():
        pred = model(img_tensor)  # (1,68,18)
    decoded = decode_predictions(pred)[0]
    if decoded == label:
        correct += 1
    else:
        print(f"GT: {label}, Pred: {decoded}")
    total += 1

accuracy = correct / total if total > 0 else 0
print(f"Baseline accuracy on {total} samples: {accuracy:.4f}")