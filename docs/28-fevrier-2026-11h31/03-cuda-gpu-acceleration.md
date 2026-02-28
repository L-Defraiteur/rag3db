# 03 — Activation GPU CUDA pour BGE-M3

## Contexte

Le forward pass BGE-M3 sur CPU prenait ~6s/texte en mode debug (24 couches XLM-RoBERTa-large, 568M params). Inacceptable pour un usage réel.

## Infrastructure GPU

- **GPU** : NVIDIA GeForce RTX 2070, 8192 MiB VRAM, compute capability 7.5
- **Driver** : 580.126.09 (CUDA Version 13.0)
- **CUDA Toolkit** : 12.6 installé via `cuda-toolkit-12-6`
- **cuDNN** : 9.19.1.2 (`libcudnn9-dev-cuda-12`)
- **Docker GPU** : OK (nvidia-container-toolkit fonctionnel, TEI tourne bien sur GPU)

## Modifications code

### Cargo.toml — nouveau feature `cuda`

```toml
cuda = ["dep:candle-core", "dep:candle-nn", "candle-core/cuda", "candle-nn/cuda"]
```

Points importants :
- `candle-core/cuda` seul ne suffit PAS — `candle-nn/cuda` est requis pour le kernel CUDA de layer-norm
- `cudnn` n'est finalement pas nécessaire (le feature `cuda` de candle-nn utilise ses propres kernels CUDA pour layer-norm)
- Le feature `cuda` est optionnel et indépendant de `bge-m3` — on active les deux ensemble : `--features "bge-m3,cuda"`

### bge_m3_embedder.rs — auto-détection device

```rust
// Avant
let device = Device::Cpu;

// Après
let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);
```

Fallback automatique sur CPU si CUDA non disponible.

### Tests — tout sur OnceLock partagé

Les tests `as_embedder_trait` et `as_sparse_embedder_trait` créaient leur propre instance, ce qui causait 3 chargements concurrents de 2.2GB en VRAM (8GB saturée). Corrigé en utilisant `shared_embedder()` partout :

```rust
let embedder: &dyn Embedder = shared_embedder();      // au lieu de Box::new(BgeM3Embedder::new())
let embedder: &dyn SparseEmbedder = shared_embedder(); // idem
```

## Benchmarks CPU vs GPU (mode debug, unoptimized)

| Opération       | CPU      | GPU (RTX 2070) | Speedup |
|-----------------|----------|----------------|---------|
| Dense 1 texte   | 5.87s    | 0.08s          | 73x     |
| Dense 2 textes  | 11.70s   | 0.07s          | 167x    |
| Sparse 1 texte  | 23.66s   | 0.10s          | 237x    |
| Sparse 2 textes | 27.35s   | 0.12s          | 228x    |
| Suite 9 tests   | 57.3s    | 9.24s          | 6x      |

Le total est dominé par le chargement du modèle (~9s). Le forward pass est ~80-120ms/batch.

## Problèmes rencontrés et résolus

1. **CUDA toolkit absent** : driver OK mais nvcc/libcudart manquants (probablement perdus lors d'un downgrade Ubuntu). Installé `cuda-toolkit-12-6`.
2. **cuDNN header introuvable** : Debian installe `cudnn.h` dans `/usr/include/x86_64-linux-gnu/` (multiarch) mais cudarc cherche `/usr/include/cudnn.h`. Fix : symlink `sudo ln -s /usr/include/x86_64-linux-gnu/cudnn.h /usr/include/cudnn.h`.
3. **"no cuda implementation for layer-norm"** : `candle-core/cuda` seul n'active pas le kernel layer-norm qui est dans `candle-nn`. Fix : ajouter `candle-nn/cuda` au feature.
4. **Tests concurrents saturent VRAM** : 3 instances de 2.2GB en parallèle = OOM sur 8GB. Fix : tout via `OnceLock<BgeM3Embedder>`.
5. **VirtualBox DKMS** : erreur non liée, module kernel incompatible avec 6.17.0-14. S'est résolu tout seul via `dpkg --configure -a`.
6. **Espace disque** : 818MB restants sur /. Libéré ~10GB via `journalctl --vacuum-size=500M` + `apt clean` + `apt autoremove` + purge anciennes révisions snap.

## Build command

```bash
CUDA_ROOT=/usr/local/cuda-12.6 PATH=/usr/local/cuda-12.6/bin:$PATH \
  cargo test --features "bge-m3,cuda" bge_m3 -- --ignored --nocapture
```

## Prochaines étapes

- Build en mode `--release` pour mesurer les perfs réelles (forward pass devrait descendre sous 10ms)
- Commit + push de l'ensemble (BGE-M3 + CUDA)
- Continuer l'intégration rag3weaver (Phase C)
