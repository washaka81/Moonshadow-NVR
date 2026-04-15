from huggingface_hub import HfApi, list_datasets
import sys

api = HfApi()
# Search for datasets
print("--- Searching for 'chilean license plate' datasets ---")
datasets = list_datasets(search="chilean license plate", limit=20)
for ds in datasets:
    print(f"Dataset: {ds.id} - Tags: {ds.tags}")
    # Try to get more info
    try:
        ds_info = api.dataset_info(ds.id)
        print(f"  Description: {ds_info.description[:200]}...")
        # List files?
        files = api.list_repo_files(ds.id, repo_type='dataset')
        image_files = [f for f in files if f.endswith(('.jpg', '.jpeg', '.png', '.bmp'))]
        print(f"  Image files: {len(image_files)}")
        if image_files:
            print(f"  Sample: {image_files[:5]}")
    except Exception as e:
        print(f"  Error: {e}")
    print()

# Search for Spanish license plates
print("\n--- Searching for 'Spanish license plate' datasets ---")
datasets2 = list_datasets(search="Spanish license plate", limit=20)
for ds in datasets2:
    print(f"Dataset: {ds.id} - Tags: {ds.tags}")
    print()

# Search for "license plate" generally
print("\n--- Searching for 'license plate' datasets ---")
datasets3 = list_datasets(search="license plate", limit=30)
for ds in datasets3:
    tags = str(ds.tags).lower()
    if 'es' in tags or 'chile' in tags or 'spanish' in tags or 'latin' in tags:
        print(f"Dataset: {ds.id} - Tags: {ds.tags}")
        try:
            ds_info = api.dataset_info(ds.id)
            print(f"  Description: {ds_info.description[:200]}...")
        except:
            pass
        print()