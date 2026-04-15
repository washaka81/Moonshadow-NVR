import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader
import torchvision.transforms as transforms
import sys
sys.path.append('LPRNet_Pytorch')
from model.LPRNet import build_lprnet
from chilean_dataset import ChileanLPRDataset
from data.load_data import CHARS
import os
import time
import numpy as np

class AugmentedChileanLPRDataset(torch.utils.data.Dataset):
    """Dataset con aumentación de datos para LPR chileno."""
    def __init__(self, root_dir, annotation_file, lpr_max_len=8, augment=True):
        self.base_dataset = ChileanLPRDataset(root_dir, annotation_file, img_size=(94,24), lpr_max_len=lpr_max_len)
        self.augment = augment
        # Transformaciones de aumento (solo para entrenamiento)
        if augment:
            self.transform = transforms.Compose([
                transforms.ColorJitter(brightness=0.3, contrast=0.3, saturation=0.3),
                transforms.RandomAffine(degrees=5, translate=(0.05, 0.05), scale=(0.9, 1.1)),
                transforms.ToTensor(),
            ])
        else:
            self.transform = None
    
    def __len__(self):
        return len(self.base_dataset)
    
    def __getitem__(self, idx):
        img_tensor, indices_tensor, target_len, label = self.base_dataset[idx]
        if self.augment and self.transform is not None:
            # Convertir tensor CHW a PIL Image (ya está normalizado BGR)
            # Primero desnormalizar para convertir a PIL (0-255)
            # La normalización original: (img - 127.5) * 0.0078125
            # Desnormalizar: img = img_np / 0.0078125 + 127.5
            img_np = img_tensor.numpy()
            img_np = img_np / 0.0078125 + 127.5
            img_np = np.clip(img_np, 0, 255).astype(np.uint8)
            # Transponer a HWC y convertir a PIL (canales BGR?)
            # PIL espera RGB, pero nuestras imágenes son BGR. Convertir a RGB para transformaciones.
            img_rgb = img_np[[2,1,0], :, :]  # BGR -> RGB
            img_rgb = np.transpose(img_rgb, (1,2,0))  # CHW -> HWC
            from PIL import Image
            pil_img = Image.fromarray(img_rgb)
            # Aplicar transformaciones
            pil_img = self.transform(pil_img)
            # Convertir de nuevo a BGR y renormalizar
            img_np_aug = np.array(pil_img) * 255.0  # ToTensor convierte a [0,1]
            img_np_aug = img_np_aug.astype(np.float32)
            # RGB -> BGR
            img_np_aug = img_np_aug[:, :, [2,1,0]]
            img_np_aug = (img_np_aug - 127.5) * 0.0078125
            img_tensor = torch.from_numpy(img_np_aug.transpose(2,0,1))
        return img_tensor, indices_tensor, target_len, label

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
            # Skip blank and Chinese characters (indices 0-30)
            if idx != blank_index and idx != prev and idx >= 31:
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
    batch_size = 32
    epochs = 2
    lpr_max_len = 8
    num_classes = len(CHARS)
    blank_index = num_classes - 1
    dropout_rate = 0.5
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using device: {device}")

    # Load pre-trained model (use our fine-tuned model if exists)
    model_path = "chilean_lpr_final.pth"
    print(f"Loading model from {model_path}")
    state_dict = torch.load(model_path, map_location='cpu')
    model = build_lprnet(lpr_max_len=lpr_max_len, phase=False, class_num=num_classes, dropout_rate=dropout_rate)
    model.load_state_dict(state_dict)
    model.to(device)

    # Datasets with augmentation
    train_dataset = AugmentedChileanLPRDataset(root_dir, train_annotation, lpr_max_len=lpr_max_len, augment=False)
    val_dataset = ChileanLPRDataset(root_dir, val_annotation, img_size=(94,24), lpr_max_len=lpr_max_len)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, num_workers=0)
    val_loader = DataLoader(val_dataset, batch_size=batch_size, shuffle=False, num_workers=0)

    criterion = nn.CTCLoss(blank=blank_index, zero_infinity=True)
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    scheduler = optim.lr_scheduler.StepLR(optimizer, step_size=5, gamma=0.5)

    best_acc = 0
    for epoch in range(1, epochs+1):
        start = time.time()
        train_loss = train_epoch(model, device, train_loader, criterion, optimizer, epoch, blank_index)
        val_acc = evaluate(model, device, val_loader, blank_index)
        scheduler.step()
        epoch_time = time.time() - start
        print(f'Epoch {epoch} finished. Train loss: {train_loss:.4f}, Val accuracy: {val_acc:.4f}, Time: {epoch_time:.2f}s')
        if val_acc > best_acc:
            best_acc = val_acc
            torch.save(model.state_dict(), f'chilean_lpr_best_augmented.pth')
            print(f'Best model saved with accuracy {val_acc:.4f}')
        # Save checkpoint every 5 epochs
        if epoch % 5 == 0:
            torch.save(model.state_dict(), f'chilean_lpr_epoch{epoch}_augmented.pth')
    torch.save(model.state_dict(), f'chilean_lpr_final_augmented.pth')
    print(f'Training complete. Best accuracy: {best_acc:.4f}')

if __name__ == '__main__':
    main()