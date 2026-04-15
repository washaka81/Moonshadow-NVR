from PIL import Image, ImageDraw, ImageFont
import numpy as np
import os

# Generate a Chilean license plate image (94x24)
width, height = 94, 24
# White background
img = Image.new('RGB', (width, height), color='white')
draw = ImageDraw.Draw(img)
# Use a simple font (default)
try:
    font = ImageFont.truetype("DejaVuSans.ttf", 18)
except:
    font = ImageFont.load_default()
# Plate text: 4 letters + 2 numbers (Chilean format)
import random
import string
letters = ''.join(random.choices(string.ascii_uppercase, k=4))
digits = ''.join(random.choices(string.digits, k=2))
plate_text = letters + digits
print(f"Generated plate: {plate_text}")
# Draw text in black
bbox = draw.textbbox((0,0), plate_text, font=font)
text_width = bbox[2] - bbox[0]
text_height = bbox[3] - bbox[1]
x = (width - text_width) // 2
y = (height - text_height) // 2
draw.text((x, y), plate_text, fill='black', font=font)
# Save
img.save("test_plate.png")
print("Saved test_plate.png")
# Also save as raw array for Rust test
img_np = np.array(img).astype(np.float32) / 255.0
# Convert to CHW
img_chw = np.transpose(img_np, (2,0,1))  # (3,24,94)
# Save as raw binary (optional)
with open("test_plate.bin", "wb") as f:
    f.write(img_chw.tobytes())
print("Saved test_plate.bin")