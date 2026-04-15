import onnxruntime as ort
import numpy as np

session = ort.InferenceSession("LPRNet_chinese.onnx")
input_name = session.get_inputs()[0].name
output_name = session.get_outputs()[0].name
print(f"Input shape: {session.get_inputs()[0].shape}")
print(f"Output shape: {session.get_outputs()[0].shape}")

# Random input
input_data = np.random.randn(1, 3, 24, 94).astype(np.float32)
output = session.run([output_name], {input_name: input_data})[0]
print(f"Output shape: {output.shape}")
# output shape (1, 68, 18)
# Sum across dimension 1 (classes) and dimension 2 (seq_len)
sum_over_classes = output.sum(axis=1)  # shape (1, 18)
sum_over_seq = output.sum(axis=2)      # shape (1, 68)
print("Sum over classes (should be ~1 per seq position):", sum_over_classes[0])
print("Sum over seq (should be ~seq_len per class):", sum_over_seq[0])
# Determine which dimension is class probabilities
# If softmax applied over classes, sum over classes ~1
if np.allclose(sum_over_classes[0], 1.0, atol=0.1):
    print("Softmax over classes dimension (axis=1). Classes dimension = 1")
    # So shape (batch, classes, seq_len) with softmax over classes.
    # Need to transpose to (batch, seq_len, classes)
    transposed = output.transpose(0, 2, 1)
    print(f"Transposed shape: {transposed.shape}")
elif np.allclose(sum_over_seq[0], 18.0, atol=0.1):
    print("Softmax over seq dimension (axis=2). Classes dimension = 2")
else:
    print("Cannot determine softmax dimension")
    # Print first few values
    print("Output slice [0, :5, :5]:")
    print(output[0, :5, :5])