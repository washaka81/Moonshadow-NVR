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
            if idx != blank_index and idx != prev:  # blank is last index
                if idx < len(CHARS):
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

def train_epoch(model, device, train_loader, criterion, optimizer, epoch, blank_index):
    model.train()
    total_loss = 0
    for batch_idx, (images, indices, target_lens, labels) in enumerate(train_loader):
        images = images.to(device)
        outputs = model(images)  # (batch, num_classes, seq_len)
        outputs = outputs.permute(2, 0, 1)  # (seq_len, batch, num_classes)
        # Flatten indices (already padded, but we need to remove padding for CTC)
        # indices shape (batch, lpr_max_len), target_lens (batch)
        # Create flat targets (concatenate non-padded parts)
        targets_flat = []
        for i in range(indices.size(0)):
            targets_flat.append(indices[i, :target_lens[i]])
        targets_flat = torch.cat(targets_flat).to(device)
        input_lengths = torch.full((outputs.size(1),), outputs.size(0), dtype=torch.long, device=device)
        loss = criterion(outputs.log_softmax(2), targets_flat, input_lengths, target_lens.to(device))
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        total_loss += loss.item()
        if batch_idx % 20 == 0:
            print(f'Epoch {epoch} [{batch_idx}/{len(train_loader)}] Loss: {loss.item():.4f}')
    return total_loss / len(train_loader)

def evaluate(model, device, val_loader, blank_index):
    model.eval()
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
    accuracy = correct / total if total > 0 else 0
    return accuracy

def main():
    root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
    train_annotation = os.path.join(root_dir, "train.txt")
    val_annotation = os.path.join(root_dir, "val.txt")
    batch_size = 16
    epochs = 2
    learning_rate = 0.001
    lpr_max_len = 8
    num_classes = len(CHARS)
    blank_index = num_classes - 1
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
    train_dataset = ChileanLPRDataset(root_dir, train_annotation, lpr_max_len=lpr_max_len)
    val_dataset = ChileanLPRDataset(root_dir, val_annotation, lpr_max_len=lpr_max_len)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)
    val_loader = DataLoader(val_dataset, batch_size=batch_size, shuffle=False, num_workers=0)

    criterion = nn.CTCLoss(blank=blank_index, zero_infinity=True)
    optimizer = optim.Adam(model.parameters(), lr=learning_rate)

    best_acc = 0
    for epoch in range(1, epochs+1):
        start = time.time()
        train_loss = train_epoch(model, device, train_loader, criterion, optimizer, epoch, blank_index)
        val_acc = evaluate(model, device, val_loader, blank_index)
        print(f'Epoch {epoch} finished. Train loss: {train_loss:.4f}, Val accuracy: {val_acc:.4f}, Time: {time.time()-start:.2f}s')
        if val_acc > best_acc:
            best_acc = val_acc
            torch.save(model.state_dict(), f'chilean_lpr_best.pth')
            print(f'Best model saved with accuracy {val_acc:.4f}')
    torch.save(model.state_dict(), f'chilean_lpr_final.pth')
    print(f'Training complete. Best accuracy: {best_acc:.4f}')

if __name__ == '__main__':
    main()