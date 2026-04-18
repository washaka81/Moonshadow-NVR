# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

from huggingface_hub import HfApi
api = HfApi()
repo_id = "0xnu/european-license-plate-recognition"
files = api.list_repo_files(repo_id)
for f in files:
    print(f)
    if f.endswith('.onnx'):
        print("  -> ONNX model")
    if f.endswith('.txt') or f.endswith('.json'):
        # maybe read config
        pass