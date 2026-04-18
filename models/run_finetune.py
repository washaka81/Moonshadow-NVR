# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import sys
sys.argv = [
    'train_LPRNet.py',
    '--train_img_dirs', 'chilean_plates/train',
    '--test_img_dirs', 'chilean_plates/val',
    '--pretrained_model', 'LPRNet_Pytorch/weights/Final_LPRNet_model.pth',
    '--batch_size', '32',
    '--epoch', '10',
    '--lpr_max_len', '8',
    '--phase_train', '1',
    '--phase_test', '0',
]
sys.path.insert(0, 'LPRNet_Pytorch')
exec(open('LPRNet_Pytorch/train_LPRNet.py').read())