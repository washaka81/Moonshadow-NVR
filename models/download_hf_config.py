from huggingface_hub import hf_hub_download, HfApi
import json

repo_id = "0xnu/european-license-plate-recognition"
api = HfApi()
files = api.list_repo_files(repo_id)
for f in files:
    if f.endswith('.json') or f.endswith('.txt') or f.endswith('.md'):
        print(f"Downloading {f}")
        try:
            path = hf_hub_download(repo_id=repo_id, filename=f, local_dir=".")
            print(f"  -> {path}")
            if f.endswith('.json'):
                with open(path, 'r') as j:
                    data = json.load(j)
                    print(f"  Content preview: {json.dumps(data, indent=2)[:500]}")
        except Exception as e:
            print(f"  Error: {e}")