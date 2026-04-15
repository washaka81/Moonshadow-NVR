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

def main():
    root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
    train_annotation = os.path.join(root_dir, "train.txt")
    batch_size = 32
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

    # Dataset
    print("Loading training dataset...")
    train_dataset = ChileanLPRDataset(root_dir, train_annotation, lpr_max_len=lpr_max_len)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)
    print(f"Training samples: {len(train_dataset)}")

    criterion = nn.CTCLoss(blank=blank_index, zero_infinity=True)
    optimizer = optim.Adam(model.parameters(), lr=0.001)

    # Train one epoch
    model.train()
    total_loss = 0
    start_epoch = time.time()
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
        if batch_idx % 10 == 0:
            print(f'Batch {batch_idx}/{len(train_loader)} Loss: {loss.item():.4f}')
    epoch_time = time.time() - start_epoch
    avg_loss = total_loss / len(train_loader)
    print(f'Epoch 1 finished. Avg loss: {avg_loss:.4f}, Time: {epoch_time:.2f}s')
    
    # Save model
    torch.save(model.state_dict(), 'chilean_lpr_epoch1.pth')
    print('Model saved.')

if __name__ == '__main__':
    main()