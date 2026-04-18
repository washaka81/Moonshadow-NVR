# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import sys
sys.path.append('LPRNet_Pytorch')
import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader
from chilean_dataset import ChileanLPRDataset
from model.LPRNet import build_lprnet
from data.load_data import CHARS

def decode_predictions(pred, num_classes):
    # pred shape (batch, num_classes, seq_len)
    pred = pred.permute(0, 2, 1)  # (batch, seq_len, num_classes)
    pred = nn.functional.softmax(pred, dim=2)
    pred_indices = torch.argmax(pred, dim=2)  # (batch, seq_len)
    batch_results = []
    for b in range(pred_indices.size(0)):
        indices = pred_indices[b].cpu().numpy()
        chars = []
        prev = None
        for idx in indices:
            if idx != num_classes-1 and idx != prev:  # blank is last index
                if idx < len(CHARS):
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

def main():
    root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
    train_annotation = f"{root_dir}/train.txt"
    batch_size = 8
    lpr_max_len = 8
    num_classes = len(CHARS)
    dropout_rate = 0.5
    device = torch.device("cpu")
    print(f"Using device: {device}")

    # Load pre-trained model
    model_path = "LPRNet_Pytorch/weights/Final_LPRNet_model.pth"
    print(f"Loading pre-trained model from {model_path}")
    state_dict = torch.load(model_path, map_location='cpu')
    model = build_lprnet(lpr_max_len=lpr_max_len, phase=False, class_num=num_classes, dropout_rate=dropout_rate)
    model.load_state_dict(state_dict)
    model.to(device)

    # Dataset
    train_dataset = ChileanLPRDataset(root_dir, train_annotation)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)
    print(f"Dataset size: {len(train_dataset)}")

    criterion = nn.CTCLoss(blank=num_classes-1, zero_infinity=True)
    optimizer = optim.Adam(model.parameters(), lr=0.001)

    # Single batch test
    model.train()
    batch = next(iter(train_loader))
    images, targets, labels = batch
    print(f"Images shape: {images.shape}")
    print(f"Targets lengths: {[len(t) for t in targets]}")
    print(f"Labels: {labels}")
    images = images.to(device)
    outputs = model(images)  # (batch, num_classes, seq_len)
    print(f"Outputs shape: {outputs.shape}")
    outputs = outputs.permute(2, 0, 1)  # (seq_len, batch, num_classes)
    target_lengths = torch.tensor([len(t) for t in targets], dtype=torch.long)
    input_lengths = torch.full((outputs.size(1),), outputs.size(0), dtype=torch.long)
    # Flatten list of lists
    targets_flat = torch.tensor([item for sublist in targets for item in sublist], dtype=torch.long)
    print(f"Target lengths: {target_lengths}")
    print(f"Input lengths: {input_lengths}")
    print(f"Targets flat shape: {targets_flat.shape}")
    loss = criterion(outputs.log_softmax(2), targets_flat, input_lengths, target_lengths)
    print(f"CTC Loss: {loss.item()}")

    # Backward
    optimizer.zero_grad()
    loss.backward()
    optimizer.step()
    print("Backward completed")

    # Decode predictions
    model.eval()
    with torch.no_grad():
        outputs = model(images)
        decoded = decode_predictions(outputs, num_classes)
        print("Decoded:", decoded)
        print("True:", labels)
        correct = sum([1 for d, t in zip(decoded, labels) if d == t])
        print(f"Accuracy: {correct}/{len(labels)}")

if __name__ == '__main__':
    main()