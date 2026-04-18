# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from chilean_dataset import ChileanLPRDataset
from data.load_data import CHARS
import os
import time

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

def train_epoch(model, device, train_loader, criterion, optimizer, epoch):
    model.train()
    total_loss = 0
    for batch_idx, (images, targets, labels) in enumerate(train_loader):
        images = images.to(device)
        outputs = model(images)  # (batch, num_classes, seq_len)
        outputs = outputs.permute(2, 0, 1)  # (seq_len, batch, num_classes)
        target_lengths = torch.tensor([len(t) for t in targets], dtype=torch.long)
        input_lengths = torch.full((outputs.size(1),), outputs.size(0), dtype=torch.long)
        # Flatten list of lists
        targets = torch.tensor([item for sublist in targets for item in sublist], dtype=torch.long)
        loss = criterion(outputs.log_softmax(2), targets, input_lengths, target_lengths)
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        total_loss += loss.item()
        if batch_idx % 20 == 0:
            print(f'Epoch {epoch} [{batch_idx}/{len(train_loader)}] Loss: {loss.item():.4f}')
    return total_loss / len(train_loader)

def evaluate(model, device, val_loader, num_classes):
    model.eval()
    correct = 0
    total = 0
    with torch.no_grad():
        for images, targets, labels in val_loader:
            images = images.to(device)
            outputs = model(images)
            decoded = decode_predictions(outputs, num_classes)
            for pred, true in zip(decoded, labels):
                if pred == true:
                    correct += 1
                total += 1
    accuracy = correct / total if total > 0 else 0
    return accuracy

def main():
    root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
    train_annotation = os.path.join(root_dir, "train.txt")
    val_annotation = os.path.join(root_dir, "val.txt")
    batch_size = 32
    epochs = 10
    learning_rate = 0.001
    lpr_max_len = 8
    num_classes = len(CHARS)
    dropout_rate = 0.5
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using device: {device}")

    # Load pre-trained model
    model_path = "LPRNet_Pytorch/weights/Final_LPRNet_model.pth"
    print(f"Loading pre-trained model from {model_path}")
    state_dict = torch.load(model_path, map_location='cpu')
    model = build_lprnet(lpr_max_len=lpr_max_len, phase=False, class_num=num_classes, dropout_rate=dropout_rate)
    model.load_state_dict(state_dict)
    model.to(device)

    # Datasets
    train_dataset = ChileanLPRDataset(root_dir, train_annotation)
    val_dataset = ChileanLPRDataset(root_dir, val_annotation)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)  # num_workers=0 to avoid multiprocessing issues
    val_loader = DataLoader(val_dataset, batch_size=batch_size, shuffle=False, num_workers=0)

    criterion = nn.CTCLoss(blank=num_classes-1, zero_infinity=True)
    optimizer = optim.Adam(model.parameters(), lr=learning_rate)

    best_acc = 0
    for epoch in range(1, epochs+1):
        start = time.time()
        train_loss = train_epoch(model, device, train_loader, criterion, optimizer, epoch)
        val_acc = evaluate(model, device, val_loader, num_classes)
        print(f'Epoch {epoch} finished. Train loss: {train_loss:.4f}, Val accuracy: {val_acc:.4f}, Time: {time.time()-start:.2f}s')
        if val_acc > best_acc:
            best_acc = val_acc
            torch.save(model.state_dict(), f'chilean_lpr_best.pth')
            print(f'Best model saved with accuracy {val_acc:.4f}')
    torch.save(model.state_dict(), f'chilean_lpr_final.pth')
    print(f'Training complete. Best accuracy: {best_acc:.4f}')

if __name__ == '__main__':
    main()