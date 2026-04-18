# This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
# Copyright (C) 2025 Moonshadow NVR Contributors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

from huggingface_hub import HfApi
import sys

api = HfApi()
# Search for models with search term
models = api.list_models(search="chilean license plate", limit=20)
print("--- Chilean license plate models ---")
for model in models:
    print(f"{model.modelId} - {model.tags}")
    # Check for ONNX files
    try:
        files = api.list_repo_files(model.modelId)
        for f in files:
            if f.endswith('.onnx'):
                print(f"    ONNX file: {f}")
    except:
        pass

# Also search for "license plate recognition"
models2 = api.list_models(search="license plate recognition", limit=30)
print("\n--- License plate recognition models ---")
for model in models2:
    # Look for Chilean or Spanish tags
    tags = str(model.tags).lower()
    if 'es' in tags or 'chile' in tags or 'spain' in tags:
        print(f"{model.modelId} - {model.tags}")
        try:
            files = api.list_repo_files(model.modelId)
            for f in files:
                if f.endswith('.onnx'):
                    print(f"    ONNX file: {f}")
        except:
            pass