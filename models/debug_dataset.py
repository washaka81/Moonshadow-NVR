import sys
sys.path.append('LPRNet_Pytorch')
from chilean_dataset import ChileanLPRDataset
from data.load_data import CHARS, CHARS_DICT

root = 'license-plate-recognition/data_plates/Synthetic_Chilean_License_Plates'
train = ChileanLPRDataset(root, root+'/train.txt')
print('Total samples:', len(train))
for i in range(5):
    img, indices, label = train[i]
    print(f'Sample {i}: label={label}, indices={indices}')
    # map indices back to chars
    decoded = ''.join([CHARS[idx] for idx in indices])
    print(f'  Decoded: {decoded}')
    # check each character mapping
    for ch in label:
        print(f'    {ch} -> {CHARS_DICT[ch]}')
    print()