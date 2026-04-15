import sys
sys.path.append('LPRNet_Pytorch')
from data.load_data import CHARS, CHARS_DICT

print(f"Total CHARS: {len(CHARS)}")
print("First 10:", CHARS[:10])
print("Digits 0-9:", CHARS[31:41])
print("Letters A-Z:", CHARS[41:67])
print("Dash:", CHARS[67])

# Test mapping
label = "DTFT33"
indices = []
for ch in label:
    if ch in CHARS_DICT:
        idx = CHARS_DICT[ch]
        print(f"{ch} -> {idx}")
        indices.append(idx)
    else:
        print(f"Character {ch} not in CHARS")
print(f"Indices: {indices}")

# Check that all Chilean plate characters are in CHARS
import string
all_chilean = string.digits + string.ascii_uppercase
missing = []
for ch in all_chilean:
    if ch not in CHARS_DICT:
        missing.append(ch)
print(f"Missing characters for Chilean plates: {missing}")
# Should be none