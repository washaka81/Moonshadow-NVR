import torch
import torch.nn as nn
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from chilean_dataset import ChileanLPRDataset
from data.load_data import CHARS
import os

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
            if idx != blank_index and idx != prev and idx >= 31:
                if idx < len(CHARS):
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
val_annotation = os.path.join(root_dir, "val.txt")
val_dataset = ChileanLPRDataset(root_dir, val_annotation, lpr_max_len=8)
print(f"Loaded {len(val_dataset)} validation samples")

device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
print(f"Device: {device}")

model_path = "chilean_lpr_final.pth"
state_dict = torch.load(model_path, map_location='cpu')
model = build_lprnet(lpr_max_len=8, phase=False, class_num=len(CHARS), dropout_rate=0.5)
model.load_state_dict(state_dict)
model.to(device)
model.eval()

blank_index = len(CHARS) - 1

from torch.utils.data import DataLoader
val_loader = DataLoader(val_dataset, batch_size=32, shuffle=False)

correct = 0
total = 0
with torch.no_grad():
    for images, indices, target_lens, labels in val_loader:
        images = images.to(device)
        outputs = model(images)
        decoded = decode_predictions(outputs, blank_index)
        for pred, true in zip(decoded, labels):
            if pred == true:
                correct += 1
            total += 1
            if total <= 10:
                print(f"Pred: {pred} | True: {true}")
accuracy = correct / total if total > 0 else 0
print(f"Accuracy with corrected decoding: {accuracy:.4f} ({correct}/{total})")