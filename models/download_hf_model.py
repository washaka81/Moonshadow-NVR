from huggingface_hub import hf_hub_download
import os

repo_id = "0xnu/european-license-plate-recognition"
filename = "model.onnx"
local_dir = "./"
try:
    model_path = hf_hub_download(repo_id=repo_id, filename=filename, local_dir=local_dir)
    print(f"Downloaded to: {model_path}")
    # Move to models directory
    import shutil
    target = os.path.join("models", "european_lpr.onnx")
    shutil.move(model_path, target)
    print(f"Moved to: {target}")
except Exception as e:
    print(f"Error downloading: {e}")
    # Try other possible files
    from huggingface_hub import HfApi
    api = HfApi()
    files = api.list_repo_files(repo_id)
    for f in files:
        if f.endswith('.onnx'):
            print(f"Found ONNX: {f}")
            try:
                path = hf_hub_download(repo_id=repo_id, filename=f, local_dir=local_dir)
                print(f"Downloaded: {path}")
            except Exception as e2:
                print(f"Failed to download {f}: {e2}")