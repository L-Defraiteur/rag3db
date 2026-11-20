#!/usr/bin/env python3
"""Reference vectors of BGE-M3 (dense), by onnxruntime (CPU, f32) — 20 November 2026.

Usage : reference_bge_m3.py <folder with model.onnx and tokenizer.json> <texts.json> <out.json>

Each text alone (batch of 1, no padding), truncated at 8192 tokens like the model card ;
the dense vector is the ONNX `sentence_embedding` output, L2-normalised. The output has
the texts, their token counts and the vectors, under the keys of the granite references
(`phrases`, `jetons`, `vecteurs`) : `scripts/install_embedder_reference.py` adds the
weights' sha256 and the tolerance, and the service checks itself against it at start.
"""
import json
import sys

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer

folder, texts_path, out = sys.argv[1:4]
texts = json.load(open(texts_path))
if isinstance(texts, dict):
    texts = texts.get("phrases") or texts["texts"]
tok = Tokenizer.from_file(f"{folder}/tokenizer.json")
tok.no_padding()
tok.enable_truncation(max_length=8192)
session = ort.InferenceSession(f"{folder}/model.onnx", providers=["CPUExecutionProvider"])
vectors, tokens = [], []
for t in texts:
    enc = tok.encode(t)
    feed = {"input_ids": np.array([enc.ids], dtype=np.int64), "attention_mask": np.array([enc.attention_mask], dtype=np.int64)}
    dense = session.run(["sentence_embedding"], feed)[0][0].astype(np.float64)
    dense = dense / np.linalg.norm(dense)
    vectors.append([float(x) for x in dense.astype(np.float32)])
    tokens.append(len(enc.ids))
json.dump({"source": "onnxruntime " + ort.__version__ + ", CPUExecutionProvider, f32, sentence_embedding",
           "phrases": texts, "jetons": tokens, "vecteurs": vectors}, open(out, "w"), ensure_ascii=False)
print(f"{len(texts)} textes, dim {len(vectors[0])}, jetons {min(tokens)}–{max(tokens)}")
