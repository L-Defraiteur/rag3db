#!/usr/bin/env python3
"""Vecteurs de référence d'un granite-embedding d'IBM, par onnxruntime (CPU, f32).

Usage : reference_granite.py <dossier avec model.onnx et tokenizer.json> <phrases.json> <sortie.json>

Chaque phrase passe seule (lot de 1, sans remplissage), tronquée à 512 jetons,
comme la fiche du modèle : `last_hidden_state[:, 0]` puis normalisation L2.
La sortie porte les phrases, le nombre de jetons de chacune et les vecteurs :
c'est contre elle qu'un `model.bpk` régénéré se mesure, par l'écart absolu max.
"""
import json
import sys

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer

dossier, chemin_phrases, sortie = sys.argv[1:4]
phrases = json.load(open(chemin_phrases))
tok = Tokenizer.from_file(f"{dossier}/tokenizer.json")
tok.no_padding()
tok.enable_truncation(max_length=512)
session = ort.InferenceSession(f"{dossier}/model.onnx", providers=["CPUExecutionProvider"])
entrees = {i.name for i in session.get_inputs()}
sorties = [o.name for o in session.get_outputs()]

vecteurs, jetons = [], []
for p in phrases:
    enc = tok.encode(p)
    ids = np.array([enc.ids], dtype=np.int64)
    masque = np.array([enc.attention_mask], dtype=np.int64)
    feed = {"input_ids": ids, "attention_mask": masque}
    feed = {k: v for k, v in feed.items() if k in entrees}
    cache = session.run(sorties, feed)[0]  # last_hidden_state [1, S, H]
    v = cache[0, 0].astype(np.float64)
    v = v / np.linalg.norm(v)
    vecteurs.append([float(x) for x in v.astype(np.float32)])
    jetons.append(len(enc.ids))

json.dump(
    {
        "source": "onnxruntime " + ort.__version__ + ", CPUExecutionProvider, f32",
        "sorties_onnx": sorties,
        "phrases": phrases,
        "jetons": jetons,
        "vecteurs": vecteurs,
    },
    open(sortie, "w"),
    ensure_ascii=False,
)
print(f"{len(phrases)} phrases, dim {len(vecteurs[0])}, jetons {min(jetons)}–{max(jetons)}, sorties {sorties}")
