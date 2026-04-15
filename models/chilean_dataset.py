import torch
from torch.utils.data import Dataset
from PIL import Image
import numpy as np
import os

from data.load_data import CHARS, CHARS_DICT

class ChileanLPRDataset(Dataset):
    def __init__(self, root_dir, annotation_file, img_size=(94,24), lpr_max_len=8):
        self.root_dir = root_dir
        self.img_size = img_size
        self.lpr_max_len = lpr_max_len
        self.blank_index = len(CHARS) - 1  # last index is blank
        self.samples = []
        with open(annotation_file, 'r') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                path, label = line.split()
                # Convert label to indices
                indices = []
                for ch in label:
                    if ch in CHARS_DICT:
                        indices.append(CHARS_DICT[ch])
                    else:
                        raise ValueError(f"Character {ch} not in CHARS")
                # Pad to lpr_max_len with blank index
                if len(indices) > lpr_max_len:
                    raise ValueError(f"Label length {len(indices)} exceeds lpr_max_len {lpr_max_len}")
                padded = indices + [self.blank_index] * (lpr_max_len - len(indices))
                self.samples.append((path, label, torch.tensor(padded, dtype=torch.long), len(indices)))
        print(f"Loaded {len(self.samples)} samples from {annotation_file}")
    
    def __len__(self):
        return len(self.samples)
    
    def __getitem__(self, idx):
        path, label, indices_tensor, target_len = self.samples[idx]
        img_path = os.path.join(self.root_dir, path)
        img = Image.open(img_path).convert('RGB')
        # Ensure correct size (already 94x24)
        if img.size != self.img_size:
            img = img.resize(self.img_size)
        # Convert to numpy BGR (model expects BGR from OpenCV)
        img_np = np.array(img).astype(np.float32)  # RGB, 0-255
        img_np = img_np[:, :, [2,1,0]]  # RGB -> BGR
        # Normalize as in original LPRNet preprocessing
        img_np -= 127.5
        img_np *= 0.0078125
        img_tensor = torch.from_numpy(img_np).permute(2,0,1)  # CHW
        return img_tensor, indices_tensor, target_len, label