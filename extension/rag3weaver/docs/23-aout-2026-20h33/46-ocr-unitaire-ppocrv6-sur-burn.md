# Doc 46 — OCR en usage unitaire : PP-OCRv6 tiny sur burn (25 août, matin)

Chantier 4 de l'ordre de Lucie (doc 41) : « un petit nœud minimal, pas
markitdown ni lib lourde ». Livré en trois commits : `0a25239d0`
(échafaudage), `e8d08d711` (modèle), fiche HF préparée. Passe E2E complète
rejouée avant : **23 suites, 206/206**.

## Le choix

La note 24 de Lucie mettait en avant PP-OCRv6 et LightOnOCR-2. Un VLM
(LightOnOCR, GLM-OCR) est un décodeur autorégressif avec KV-cache :
burn-onnx ne le sort pas proprement et il pèse 1 Go. **PP-OCRv6 tiny**
(détection DBNet + reconnaissance SVTR-LCNet/CTC, 1,5 M params, **6,2 Mo
de poids**, 49 langues dont le français, Apache-2.0) est le seul candidat
compatible « nœud minimal embarquable ». PaddlePaddle publie les ONNX
officiellement depuis 3.7.0 (juin 2026) : `PaddlePaddle/PP-OCRv6_tiny_{det,rec}_onnx`.

Épreuve de faisabilité avant d'écrire une ligne : les six ONNX (v6
tiny/small det+rec, v5 mobile det+rec) passent burn-onnx 0.22.0-pre.1 et
s'exécutent avec dims dynamiques. Un seul accroc : trois nœuds
`auto_pad=SAME_UPPER` dans les det v6 (2 Conv + 1 MaxPool du RepLKFPN) que
burn-onnx refuse sans shape statique → réécrits en `pads=[0,0,1,1]` dans
l'ONNX (`fix_onnx.py`, 10 lignes). Le rec tiny n'a ni `Shape` ni `Reshape` :
graphe le plus simple. Si tiny ne suffit pas, **small rec** (21 Mo, 18 708
caractères) partage exactement le même post-traitement.

## Ce qui est en place

**Surface, indépendante du modèle** (`src/ocr.rs`, `src/dataflow/ocr_nodes.rs`) :

- `trait Ocr { recognize(&OcrImage) -> OcrOutput }`, service `"ocr"` comme
  `"embedder"` ; `Catalog::set_ocr`.
- `OcrImage` (RGB8 ; `decode(bytes)` PNG/JPEG/WebP/BMP/GIF/TIFF sous la
  feature `ocr`, via `image` déjà dans l'arbre), `OcrLine { text, confidence,
  quad }`, `OcrOutput::text()` = lignes jointes par `\n` (convention de
  `_content`), `sort_reading_order` **partagé** — tout modèle rend le même
  ordre.
- `OcrNode` : port `image` (`Vec<u8>` encodé ou `OcrImage`) → `text`
  (`String`) + `ocr` (`OcrOutput`) ; `min_confidence` ; métriques
  `ocr_lines/ocr_dropped/ocr_ms`. `PortType::{Image, Text, Ocr}`. 27 nœuds au
  registre. `MockOcr` pour les tests.

**Modèle** (`src/burn_ppocr.rs`, feature `burn-ocr = ocr + burn + burn-store`) :

- `BurnPpOcr::{from_bytes, from_files, from_cache_dir}` ;
  `default_cache_dir()` = `$RAG3WEAVER_PPOCR_DIR` sinon
  `~/.cache/rag3weaver/ppocrv6-tiny/{det.bpk, rec.bpk, dict.txt}`.
- Options = les `inference.yml` v6 tiny : côté min 736 (plafond 4000),
  multiple de 32, `det_thresh 0.2`, `box_thresh 0.4`, `unclip_ratio 1.4`,
  rec hauteur 48, lots de 6. Canaux **BGR** comme PaddleOCR (notre
  `OcrImage` est RGB, on swappe).
- Post-DB en Rust pur : binarisation, composantes connexes 8-voisinage,
  boîte englobante axée, score moyen, unclip `d = aire·ratio/périmètre`,
  filtres 3 px / 5 px, remise à l'échelle. CTC : blank 0, dict 1..N, espace
  N+1, confiance = moyenne des max gardés.
- `det_map` / `rec_logits` exposés pour la parité.
- `BurnDevice` sorti dans `src/burn_device.rs` (partagé embedders/OCR,
  ré-exporté à l'ancien chemin).

## Chiffres (wgpu/Vulkan, R9700, release)

```
chargement                       31–65 ms (6,2 Mo)     premier recognize 414 ms (noyaux)
det 400×120 → [1,3,736,2464]     forward 110 ms, détection complète 134 ms
rec [2,3,48,320]                 forward 9,2 ms
fixture hello.png                "Hello rag3weaver" 0,987 · "OCR 2026" 0,984 — exact
parité onnxruntime 1.29          rec max|Δ| 1,4e-5 · det max|Δ| 1,8e-3 (moy. 1,8e-6 ; 87 px > 1e-3
                                 sur 1,8 M, bords des glyphes, sigmoïde raide — ndarray fait pareil)
tests                            lib 626 (burn-ocr) · e2e_burn_ocr 4/4 (4 s) · 7 combinaisons de features
```

Le seuil de parité det est à 5e-3 (documenté dans l'exemple et
`generated/README.md`) ; rec reste à 1e-3.

## Comment s'en servir

```rust
use rag3weaver::{burn_device::BurnDevice, burn_ppocr::BurnPpOcr, ocr::{Ocr, OcrImage}};
let ocr = BurnPpOcr::from_cache_dir(BurnPpOcr::default_cache_dir(), BurnDevice::Default)?;
let out = ocr.recognize(&OcrImage::decode(&std::fs::read("page.png")?)?)?;
println!("{}", out.text());               // lignes en ordre de lecture
catalog.set_ocr(Arc::new(ocr));            // → service "ocr" pour OcrNode
```

```bash
cargo test --features burn-ocr --test e2e_burn_ocr -- --ignored --test-threads=1
cargo run --release --example burn_ppocr_vs_onnxruntime --features burn-ocr -- \
    <python-avec-onnxruntime> ppocr_ref.py det_pads.onnx rec_pads.onnx <workdir>
```

## Dettes nommées

- **Boîtes axées** : pas de `minAreaRect` ni d'offset polygonal (pyclipper) —
  le texte incliné est mal découpé. À faire si un use case documents le
  demande (calipers tournants sur l'enveloppe convexe, ~100 lignes).
- `max_candidates` garde les plus grandes composantes (cv2 ordonne ses
  contours autrement) ; resize `image` Triangle ≈ bilinéaire cv2 en
  agrandissement, différent en réduction (grandes pages).
- Pas de classifieur d'angle (texte à 180°), pas de layout, pas de PDF : le
  nœud prend une image, point. Le PDF → images est un autre nœud.
- Publication HF `Lucie666/ppocrv6-tiny-burnpack` préparée dans le
  scratchpad (`publish/ppocrv6-tiny-burnpack/` : .bpk, dict, ONNX patchés,
  yml, `fix_onnx.py`, `build.rs`, oracle, `SHA256SUMS`, fiche) — **attend le
  go de Lucie**. Empreintes : `det.bpk` 1 737 476 o `73a139fa…`, `rec.bpk`
  4 443 368 o `53bfcb22…`, `dict.txt` 27 156 o `c5cbe34e…`.
