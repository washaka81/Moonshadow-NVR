import sys
sys.path.append('LPRNet_Pytorch')
import torch
import torch.nn as nn
from torch.utils.data import DataLoader
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
            if idx != blank_index and idx != prev:
                if idx < len(CHARS):
                    chars.append(CHARS[idx])
                prev = idx
        batch_results.append(''.join(chars))
    return batch_results

def main():
    root_dir = "license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates"
    val_annotation = os.path.join(root_dir, "val.txt")
    lpr_max_len = 8
    num_classes = len(CHARS)
    blank_index = num_classes - 1
    dropout_rate = 0.5
    device = torch.device("cpu")
    
    model_path = "chilean_lpr_final.pth"
    print(f'Loading fine-tuned model from {model_path}')
    model = build_lprnet(lpr_max_len=lpr_max_len, phase=False, class_num=num_classes, dropout_rate=dropout_rate)
    model.load_state_dict(torch.load(model_path, map_location='cpu'))
    model.to(device)
    model.eval()
    
    print('Loading validation dataset...')
    val_dataset = ChileanLPRDataset(root_dir, val_annotation, lpr_max_len=lpr_max_len)
    val_loader = DataLoader(val_dataset, batch_size=32, shuffle=False, num_workers=0)
    print('Evaluating...')
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
    print(f'Accuracy on {total} samples: {accuracy:.4f}')
    # Print some examples
    if total > 0:
        print('First 10 predictions:')
        for i, (pred, true) in enumerate(zip(decoded[:10], labels[:10])):
            print(f'  {true} -> {pred}')

if __name__ == '__main__':
    main()