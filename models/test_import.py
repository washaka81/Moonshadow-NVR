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