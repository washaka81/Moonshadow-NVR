# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import sys
sys.path.append('LPRNet_Pytorch')
print('Importing torch...')
import torch
print('Importing model...')
from model.LPRNet import build_lprnet
print('Importing dataset...')
from chilean_dataset import ChileanLPRDataset
print('Importing data...')
from data.load_data import CHARS
print('All imports successful')