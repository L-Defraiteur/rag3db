# 1. Ce que le modèle voit

## paire s02 (attendu : same) — 173 jetons, masse de probabilité sur les trois lettres : 0.9991

```
<|im_start|>system
Apply the supplied criterion to the supplied evidence. Choose exactly one listed option. Respond with only its uppercase letter, with no explanation or reasoning.<|im_end|>
<|im_start|>user
{"evidence": "Topic A: reprise après un point de reprise interrompu\nTopic B: récupération quand le checkpoint a été coupé en cours de route", "criterion": "Do these two topic names refer to the same topic?", "options": [{"letter": "A", "description": "same: both phrases name the same topic, worded differently or in another language"}, {"letter": "B", "description": "variant: related topics: one is a part, a mechanism or a special case of the other"}, {"letter": "C", "description": "unrelated: two different topics"}]}<|im_end|>
<|im_start|>assistant
<think>

</think>

<ici le modèle écrit un jeton>
```

jetons les plus probables : "A" -0.09, "C" -2.80, "B" -3.59, "D" -7.89, "<think>" -9.15, "E" -9.98, "O" -10.46, "I" -10.62

## paire s04 (attendu : same) — 175 jetons, masse de probabilité sur les trois lettres : 0.9994

```
<|im_start|>system
Apply the supplied criterion to the supplied evidence. Choose exactly one listed option. Respond with only its uppercase letter, with no explanation or reasoning.<|im_end|>
<|im_start|>user
{"evidence": "Topic A: régime doux de la carte graphique pendant les embarquements\nTopic B: limiter la charge du GPU quand on calcule les vecteurs", "criterion": "Do these two topic names refer to the same topic?", "options": [{"letter": "A", "description": "same: both phrases name the same topic, worded differently or in another language"}, {"letter": "B", "description": "variant: related topics: one is a part, a mechanism or a special case of the other"}, {"letter": "C", "description": "unrelated: two different topics"}]}<|im_end|>
<|im_start|>assistant
<think>

</think>

<ici le modèle écrit un jeton>
```

jetons les plus probables : "A" -0.97, "C" -1.00, "B" -1.37, "D" -8.22, "<think>" -9.81, "E" -10.41, "R" -10.91, "I" -11.15

## paire s06 (attendu : variant) — 160 jetons, masse de probabilité sur les trois lettres : 0.9989

```
<|im_start|>system
Apply the supplied criterion to the supplied evidence. Choose exactly one listed option. Respond with only its uppercase letter, with no explanation or reasoning.<|im_end|>
<|im_start|>user
{"evidence": "Topic A: écritures parallèles\nTopic B: verrous par clé", "criterion": "Do these two topic names refer to the same topic?", "options": [{"letter": "A", "description": "same: both phrases name the same topic, worded differently or in another language"}, {"letter": "B", "description": "variant: related topics: one is a part, a mechanism or a special case of the other"}, {"letter": "C", "description": "unrelated: two different topics"}]}<|im_end|>
<|im_start|>assistant
<think>

</think>

<ici le modèle écrit un jeton>
```

jetons les plus probables : "C" -0.15, "A" -2.28, "B" -3.25, "D" -7.30, "E" -9.78, "<think>" -10.02, "I" -10.23, "L" -10.56

# 2. La formulation : ce qui bouge

| configuration | exactitude à trois choix | même trouvés (15) | variantes trouvées (15) | sans rapport trouvés (12) | AUC « même » contre le reste | verdicts changés par rapport à la base | écart moyen sur P(même) | ms |
|---|---|---|---|---|---|---|---|---|
| base | 28 sur 42 | 13 | 3 | 12 | 0.97 | 0 | 0.00 | 227 |
| critere_1 | 32 sur 42 | 14 | 6 | 12 | 0.97 | 5 | 0.04 | 245 |
| critere_2 | 30 sur 42 | 11 | 7 | 12 | 0.95 | 7 | 0.06 | 242 |
| critere_3 | 27 sur 42 | 8 | 7 | 12 | 0.93 | 12 | 0.09 | 243 |
| options_sans_description | 25 sur 42 | 13 | 0 | 12 | 0.94 | 7 | 0.11 | 230 |
| options_courtes | 29 sur 42 | 14 | 3 | 12 | 0.95 | 6 | 0.10 | 234 |
| options_longues | 29 sur 42 | 11 | 6 | 12 | 0.96 | 6 | 0.06 | 247 |
| critere_francais | 23 sur 42 | 11 | 0 | 12 | 0.93 | 8 | 0.05 | 230 |
| avec_exemples | 27 sur 42 | 11 | 4 | 12 | 0.97 | 7 | 0.08 | 253 |
| deux_options | — | 10 | — | — | 0.95 | — | 0.05 | 245 |
| sous_sujet | 29 sur 42 | 11 | 6 | 12 | 0.95 | 6 | 0.03 | 236 |
| sous_sujet_dirige | 27 sur 42 | 6 | 9 | 12 | 0.95 | 14 | 0.12 | 241 |
| oui_non_meme | — | 0 | — | — | 0.95 | — | 0.07 | 233 |
| oui_non_partie | — | 0 | — | — | 0.94 | — | 0.11 | 237 |
| ordre_same-unrelated-variant | 30 sur 42 | 13 | 5 | 12 | 0.96 | 6 | 0.04 | 238 |
| ordre_variant-same-unrelated | 26 sur 42 | 13 | 1 | 12 | 0.96 | 5 | 0.06 | 236 |
| ordre_variant-unrelated-same | 26 sur 42 | 14 | 0 | 12 | 0.94 | 6 | 0.16 | 244 |
| ordre_unrelated-same-variant | 28 sur 42 | 13 | 3 | 12 | 0.96 | 5 | 0.05 | 242 |
| ordre_unrelated-variant-same | 26 sur 42 | 13 | 1 | 12 | 0.95 | 4 | 0.08 | 237 |

## Le biais de position (les six ordres des trois options)

| ordre des options | choisit la 1re | la 2e | la 3e | exactitude |
|---|---|---|---|---|
| same / variant / unrelated | 15 | 3 | 24 | 28 sur 42 |
| same / unrelated / variant | 15 | 22 | 5 | 30 sur 42 |
| variant / same / unrelated | 1 | 17 | 24 | 26 sur 42 |
| variant / unrelated / same | 0 | 23 | 19 | 26 sur 42 |
| unrelated / same / variant | 22 | 17 | 3 | 28 sur 42 |
| unrelated / variant / same | 23 | 1 | 18 | 26 sur 42 |
| **total** | 76 | 83 | 93 | (un tiers chacun = 84 si l'ordre ne comptait pas et les réponses étaient équilibrées) |

Paires dont le verdict est le même sous les six ordres : 34 sur 42.

En moyennant les probabilités sur les six ordres : 26 sur 42 ; AUC « même » 0.96.

# 3. D'autres découpes de la même décision

- **deux oui/non enchaînés** (« même sujet ? » puis « l'un est-il une partie de l'autre ? ») : 24 sur 42 (même 12/15, variante 0/15, sans rapport 12/12).
- **deux options** (même / différent) : 36 sur 42 justes ; 1 fusion(s) à tort ; 10 vraies « même » trouvées sur 15.
- **sous-sujet dirigé** : pour les 15 « variantes », il répond : same ×0, b_in_a ×9, a_in_b ×0, unrelated ×6.

## Les « variantes », paire par paire, sous toutes les formulations à trois choix

(16 configurations)

| paire | les deux sujets | jugée variante | jugée même | jugée sans rapport | lecture |
|---|---|---|---|---|---|
| s06 | écritures parallèles // verrous par clé | 0 | 0 | 16 | étiquette douteuse : « sans rapport » avec constance |
| s07 | index vectoriel HNSW // points d'entrée de l'index vectoriel hors transaction | 7 | 6 | 3 | instable |
| s08 | synchronisation par périmètre // suppression des lignes sorties de leur périmètre | 6 | 10 | 0 | instable |
| s09 | modèle d'embarquement par défaut // choix du petit modèle au premier index sur un GPU faible | 8 | 4 | 4 | instable |
| s10 | journal d'écriture anticipée (WAL) // WAL illisible après un arrêt brutal | 6 | 9 | 1 | instable |
| s20 | concurrent writers // per-key locking | 0 | 0 | 16 | étiquette douteuse : « sans rapport » avec constance |
| s21 | vector index // deleting a row from the vector index | 12 | 0 | 4 | tenue |
| s22 | full-text search // tokenizer used by the full-text index | 10 | 0 | 6 | tenue |
| s23 | upstream pull requests to burn // AI disclosure line in a pull request | 0 | 0 | 16 | étiquette douteuse : « sans rapport » avec constance |
| s24 | embedding daemon // GPU duty cycle of the embedding daemon | 8 | 0 | 8 | instable |
| s34 | écritures parallèles // per-key locks | 0 | 0 | 16 | étiquette douteuse : « sans rapport » avec constance |
| s35 | index vectoriel // entry points of the HNSW graph | 3 | 13 | 0 | étiquette douteuse : « même » avec constance |
| s36 | mémoire longue de l'agent // deduplicating memories at write time | 1 | 0 | 15 | étiquette douteuse : « sans rapport » avec constance |
| s37 | poids des modèles // regenerating the granite weights from ONNX | 0 | 0 | 16 | étiquette douteuse : « sans rapport » avec constance |
| s38 | synchronisation par périmètre // missing-ratio threshold of a snapshot | 0 | 0 | 16 | étiquette douteuse : « sans rapport » avec constance |

# 4. Les fautes une par une

Configuration : `base`.

| paire | langue | les deux sujets | attendu | répondu | score |
|---|---|---|---|---|---|
| s03 | fr | poids des modèles publiés sur Hugging Face // mise en ligne des fichiers de poids sur le hub | same | unrelated | 0.49 (P(même) 0.21) |
| s04 | fr | régime doux de la carte graphique pendant les embarquements // limiter la charge du GPU quand on calcule les vecteurs | same | unrelated | 0.38 (P(même) 0.36) |
| s06 | fr | écritures parallèles // verrous par clé | variant | unrelated | 0.76 (P(même) 0.16) |
| s07 | fr | index vectoriel HNSW // points d'entrée de l'index vectoriel hors transaction | variant | unrelated | 0.37 (P(même) 0.33) |
| s08 | fr | synchronisation par périmètre // suppression des lignes sorties de leur périmètre | variant | same | 0.38 (P(même) 0.38) |
| s09 | fr | modèle d'embarquement par défaut // choix du petit modèle au premier index sur un GPU faible | variant | unrelated | 0.37 (P(même) 0.35) |
| s20 | en | concurrent writers // per-key locking | variant | unrelated | 0.79 (P(même) 0.11) |
| s22 | en | full-text search // tokenizer used by the full-text index | variant | unrelated | 0.50 (P(même) 0.07) |
| s23 | en | upstream pull requests to burn // AI disclosure line in a pull request | variant | unrelated | 0.88 (P(même) 0.08) |
| s34 | mixte | écritures parallèles // per-key locks | variant | unrelated | 0.69 (P(même) 0.19) |
| s35 | mixte | index vectoriel // entry points of the HNSW graph | variant | same | 0.45 (P(même) 0.45) |
| s36 | mixte | mémoire longue de l'agent // deduplicating memories at write time | variant | unrelated | 0.62 (P(même) 0.17) |
| s37 | mixte | poids des modèles // regenerating the granite weights from ONNX | variant | unrelated | 0.66 (P(même) 0.22) |
| s38 | mixte | synchronisation par périmètre // missing-ratio threshold of a snapshot | variant | unrelated | 0.86 (P(même) 0.09) |

# Mémoires : l'ordre des quatre options

- base (same / completes / contradicts / new) : 17 sur 20 ; position choisie : [3, 8, 5, 4]
- ordre_inverse (new / contradicts / completes / same) : 14 sur 20 ; position choisie : [2, 5, 11, 2]
- ordre_tourne (contradicts / new / same / completes) : 15 sur 20 ; position choisie : [5, 2, 3, 10]
