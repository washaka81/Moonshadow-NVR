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