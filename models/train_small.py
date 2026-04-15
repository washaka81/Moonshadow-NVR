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
train_annotation = os.path.join(root_dir, "train.txt")
val_annotation = os.path.join(root_dir, "val.txt")
batch_size = 8
epochs = 5
learning_rate = 0.0005
lpr_max_len = 8
num_classes = len(CHARS)
blank_index = num_classes - 1
device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
print(f"Using device: {device}")

# Load pre-trained model
model_path = "LPRNet_Pytorch/weights/Final_LPRNet_model.pth"
print(f"Loading pre-trained model from {model_path}")
state_dict = torch.load(model_path, map_location='cpu')
model = build_lprnet(lpr_max_len=lpr_max_len, phase=False, class_num=num_classes, dropout_rate=0.5)
model.load_state_dict(state_dict)
model.to(device)

# Full datasets
train_dataset_full = ChileanLPRDataset(root_dir, train_annotation, img_size=(94,24), lpr_max_len=lpr_max_len)
val_dataset_full = ChileanLPRDataset(root_dir, val_annotation, img_size=(94,24), lpr_max_len=lpr_max_len)
# Take subset for speed
train_indices = list(range(0, min(200, len(train_dataset_full))))
val_indices = list(range(0, min(100, len(val_dataset_full))))
train_dataset = Subset(train_dataset_full, train_indices)
val_dataset = Subset(val_dataset_full, val_indices)
train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)
val_loader = DataLoader(val_dataset, batch_size=batch_size, shuffle=False, num_workers=0)

criterion = nn.CTCLoss(blank=blank_index, zero_infinity=True)
optimizer = optim.Adam(model.parameters(), lr=learning_rate)

for epoch in range(1, epochs+1):
    model.train()
    total_loss = 0
    start = time.time()
    for batch_idx, (images, indices, target_lens, labels) in enumerate(train_loader):
        images = images.to(device)
        outputs = model(images)
        outputs = outputs.permute(2, 0, 1)
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
        if batch_idx % 5 == 0:
            print(f'Epoch {epoch} [{batch_idx}/{len(train_loader)}] Loss: {loss.item():.4f}')
    avg_loss = total_loss / len(train_loader)
    # Evaluate
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
    epoch_time = time.time() - start
    print(f'Epoch {epoch} finished. Avg loss: {avg_loss:.4f}, Val accuracy: {accuracy:.4f}, Time: {epoch_time:.2f}s')
    if accuracy > 0.5:
        torch.save(model.state_dict(), f'chilean_lpr_acc{accuracy:.2f}.pth')
        print(f'Model saved.')

torch.save(model.state_dict(), f'chilean_lpr_small.pth')
print('Training done.')