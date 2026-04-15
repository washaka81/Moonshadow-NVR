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