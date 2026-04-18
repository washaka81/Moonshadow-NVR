# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader, Subset
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from chilean_dataset import ChileanLPRDataset
from data.load_data import CHARS
import os
import time

def decode_predictions(pred, blank_index):
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
            if idx != blank_index and idx != prev:
                if idx < len(CHARS):
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

def main():
    root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
    train_annotation = os.path.join(root_dir, "train.txt")
    val_annotation = os.path.join(root_dir, "val.txt")
    batch_size = 8
    epochs = 1
    learning_rate = 0.001
    lpr_max_len = 8
    num_classes = len(CHARS)
    blank_index = num_classes - 1
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

    # Datasets (limit to 32 samples)
    full_dataset = ChileanLPRDataset(root_dir, train_annotation, lpr_max_len=lpr_max_len)
    subset_indices = list(range(0, min(32, len(full_dataset))))
    train_dataset = Subset(full_dataset, subset_indices)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)

    criterion = nn.CTCLoss(blank=blank_index, zero_infinity=True)
    optimizer = optim.Adam(model.parameters(), lr=learning_rate)

    # Single batch test
    model.train()
    batch = next(iter(train_loader))
    images, indices, target_lens, labels = batch
    print('Images shape:', images.shape)
    print('Indices shape:', indices.shape)
    print('Target lens:', target_lens)
    print('Labels:', labels)
    images = images.to(device)
    outputs = model(images)  # (batch, num_classes, seq_len)
    print('Outputs shape:', outputs.shape)
    outputs = outputs.permute(2, 0, 1)  # (seq_len, batch, num_classes)
    # Flatten indices
    targets_flat = []
    for i in range(indices.size(0)):
        targets_flat.append(indices[i, :target_lens[i]])
    targets_flat = torch.cat(targets_flat).to(device)
    input_lengths = torch.full((outputs.size(1),), outputs.size(0), dtype=torch.long, device=device)
    print('Targets flat shape:', targets_flat.shape)
    print('Input lengths:', input_lengths)
    print('Target lens:', target_lens)
    loss = criterion(outputs.log_softmax(2), targets_flat, input_lengths, target_lens.to(device))
    print('CTC Loss:', loss.item())
    optimizer.zero_grad()
    loss.backward()
    optimizer.step()
    print('Backward completed')
    
    # Decode
    model.eval()
    with torch.no_grad():
        outputs = model(images)
        decoded = decode_predictions(outputs, blank_index)
        print('Decoded:', decoded)
        print('True:', labels)
        correct = sum([1 for d, t in zip(decoded, labels) if d == t])
        print(f'Accuracy: {correct}/{len(labels)}')
    
    print('Test passed')

if __name__ == '__main__':
    main()