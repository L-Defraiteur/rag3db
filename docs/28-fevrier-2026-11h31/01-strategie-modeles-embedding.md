# 01 — Stratégie modèles d'embedding : BGE-M3 natif + MiniLM WASM

## Contexte

Session précédente (23-24 février) : extension `sparse_vector` (6/6 E2E) + `Bm42Embedder` (6/6 tests) terminés et pushés. Le BM42 utilise all-MiniLM-L6-v2 (~22MB) pour extraire des sparse vectors via les attention weights CLS. Ça fonctionne mais c'est un hack — les attention weights ne sont pas entraînées pour mesurer l'importance lexicale.

Réflexion sur une session mobile (non aboutie) + discussion LinkedIn sur les modèles récents → décision de passer à une stratégie par tiers.

---

## Décision : deux tiers selon la plateforme

### Natif (Node.js, C++, Rust) → BGE-M3

**Modèle** : `BAAI/bge-m3` (~2.2GB, XLM-RoBERTa-large)

Un seul forward pass produit trois types de représentations :
- **Dense** : 1024 dimensions, L2-normalized
- **Sparse** : poids lexicaux appris (style SPLADE), token_ids comme dimensions
- **ColBERT** : token-level embeddings pour late interaction (pas prioritaire pour nous)

Avantages vs la stack actuelle (MiniLM + BM42) :
- Sparse **appris** au lieu d'un hack attention → meilleure qualité retrieval
- Multilingual (100+ langues) vs anglais-centré
- Un seul modèle au lieu de deux (ou un modèle + un hack)
- Un seul forward pass pour dense + sparse (l'évolution "single forward pass" du doc 10 est incluse nativement)

Architecture : XLM-RoBERTa-large, pas BERT standard. Nécessite une implémentation candle spécifique (ou adaptation de l'existante dans candle-transformers).

### WASM → MiniLM (choix configurable)

Deux options pour l'utilisateur :

| Modèle | Taille | Langues | Dimensions | Layers |
|---|---|---|---|---|
| `multilingual-MiniLM-L12-v2` | ~120MB | 100+ | 384 | 12 |
| `all-MiniLM-L6-v2` | ~22MB | anglais | 384 | 6 |

- **Défaut** : multilingual-MiniLM-L12-v2 (120MB — acceptable pour du WASM, download one-shot avec cache)
- **Option légère** : all-MiniLM-L6-v2 (22MB — pour les cas anglais-only ou contraintes réseau fortes)

Les deux utilisent la même architecture BERT → même code d'inférence, seuls les poids changent.

**Sparse en WASM** : BM42 (extraction attention CLS) sur le MiniLM choisi. BGE-M3 est trop lourd pour WASM. Le BM42 reste branché pour l'instant — on expérimentera plus tard si le gain sparse justifie le coût en WASM vs un simple dense + BM25.

---

## Tableau récapitulatif

| | Natif | WASM (défaut) | WASM (léger) |
|---|---|---|---|
| **Modèle** | BGE-M3 | multilingual-MiniLM-L12-v2 | all-MiniLM-L6-v2 |
| **Taille** | ~2.2GB | ~120MB | ~22MB |
| **Dense** | 1024d (appris) | 384d (appris) | 384d (appris) |
| **Sparse** | Appris (style SPLADE) | BM42 (hack attention) | BM42 (hack attention) |
| **Langues** | 100+ | 100+ | anglais |
| **Architecture** | XLM-RoBERTa-large | BERT | BERT |

---

## Impact sur le code existant

### Ce qui reste tel quel

- `SparseVector`, `SparseIndex`, `SparseEmbedder` trait → inchangés
- Extension `sparse_vector` (C++ + Rust) → inchangée (stocke/cherche des `Vec<u32>` + `Vec<f32>`, agnostique du modèle)
- Fusion 3-way (RRF/Weighted/Boost) → inchangée
- `Bm42Model` + `Bm42Embedder` → gardés pour WASM

### Ce qui change

- **Nouveau** : `BgeM3Embedder` — implémente à la fois `Embedder` (dense) et `SparseEmbedder` (sparse) via un seul forward pass
- **Nouveau** : modèle XLM-RoBERTa dans candle (si pas déjà disponible dans candle-transformers)
- **Config** : le choix dense/sparse model devient configurable par plateforme dans `CatalogConfig`
- **Dimensions dense** : passage de 384 à 1024 en natif → impacte la taille des index HNSW

### Points d'attention

- **HNSW 384 vs 1024** : les index vectoriels natifs et WASM auront des dimensions différentes. Si un index est créé en natif (1024d) il ne sera pas requêtable en WASM (384d). C'est acceptable si natif et WASM ne partagent pas la même DB, mais à documenter clairement.
- **Sparse dimensions** : BGE-M3 utilise des token_ids XLM-RoBERTa (~250k vocab) vs MiniLM (~30k vocab WordPiece). Les sparse vectors natifs et WASM ne sont pas compatibles entre eux (pas le même espace de dimensions). Même remarque que ci-dessus.

---

## Modèles LinkedIn mentionnés (pour mémoire)

Perplexity a sorti 4 modèles :
- **pplx-embed-v1** (0.6B et 4B) : dense standard
- **pplx-embed-context-v1** (0.6B et 4B) : dense avec contexte document global — chaque chunk reçoit un embedding qui intègre l'information du document entier

Intéressant conceptuellement (le context-aware embedding résout le problème du chunk isolé) mais :
- API cloud only (pas local, pas candle/WASM)
- Dense seulement (pas de sparse)
- Pas pertinent pour notre stack locale

---

## Prochaine étape

Implémenter `BgeM3Embedder` dans rag3weaver :
1. Comprendre l'architecture XLM-RoBERTa et le sparse head de BGE-M3
2. Implémenter ou adapter le modèle dans candle
3. Implémenter l'embedder (dense + sparse en un pass)
4. Tests unitaires + intégration
5. Feature gate `bge-m3` séparé de `candle-embedder`
