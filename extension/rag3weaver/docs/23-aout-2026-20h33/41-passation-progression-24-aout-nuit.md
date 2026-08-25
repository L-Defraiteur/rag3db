# Doc 41 — Passation : progression au 24 août 2026, nuit

Point d'entrée pour la session suivante. Remplace le doc 29 (soir) là où ils
se contredisent. Compagnons : [42 — architecture](42-passation-architecture-24-aout-nuit.md),
[43 — mode d'emploi et points critiques](43-passation-tests-et-points-critiques.md).

## Git

| | |
|---|---|
| Branche | `fts-lucivy-v3`, **62 commits devant `master`** (`4cc0e46a8`), tout poussé |
| `master` | intact — fusionner est la décision de Lucie |
| `.gitmodules` | modifié localement par Lucie (routage SSH) — **ne jamais commiter** |
| Note OCR | `24-ocr-models-pour-usecase-documents.md`, non suivie, à Lucie (collision de numéro avec le doc 24 lucivy) |
| Submodule `ld-lucivy` | épinglé `e8b5414` (tête de `v3-recovery`) ; **référence seulement**, aucun build ne l'utilise |
| lucivy compilé | `~/git_workspaces/lucivy` par chemin (`lucivy-core`, `luciole`, `sparse-vector` → `lucistore`), arbre vivant de la session lucivy, **sur `wip/publication-3.0.0`** ce soir |

## Ce qui a été livré depuis le doc 29 (dans l'ordre)

1. **Fiabilisation finie.** lucivy `832c503`/`34ec432` épinglés (underflow scorer
   fuzzy corrigé, `close()` tolérant), `e2e_search` 38/38 ×3. Candle sorti des
   E2E (`tests/common/mod.rs` : embedders burn partagés ; `e2e_search` 234 s →
   23 s). `simple_register_duplicate_fails` supprimé. **Extensions C++
   `lucivy_fts` et `sparse_vector` supprimées** — plus aucune extension C++
   n'embarque de Rust, cmake ne construit que `vector` et `geo`. Le crate
   `sparse-vector` vit chez lucivy (docs 33/34/38).
2. **Quatre E2E de mai remis** (24 tests : undo, `search_with_strategy`,
   observabilité, nœuds génériques) et les bugs qu'ils cachaient corrigés :
   perte silencieuse dans l'index FTS sur mise à jour partielle
   (contre-épreuve faite), undo d'update laissant l'index périmé, fan-out des
   ports (`take_or_clone`), garde `max_rounds`, résumés d'arêtes vides, **deux
   copies de `luciole`** dans le graphe (→ dépendance par chemin, plus de
   `[patch]`).
3. **FFI** : toutes les réponses JSON par `serde_json` (un guillemet cassait le
   JSON) ; `rag3weaver_catalog_set_scope`/`get_scope` ; `scope`, `scopes`,
   `filterCondition`, `filters` lus depuis JS (jamais lus avant).
4. **Multi-tenant org × project — chantier entier** (doc 37) : deux axes
   orthogonaux, colonnes `_org`/`_project` partout (chunks compris), nœuds
   `_Org`/`_Project`, `Catalog::set_scope`, **un index FTS et sparse par
   cellule**, `SearchOptions.scope`/`scopes` (fan-out RRF), migration des
   bases d'avant. `e2e_scope` 9/9. Trois trous fermés en chemin (sparse sans
   filtre, `ingest_entities` sans flush, et **kuzu ignore la projection dans
   `QUERY_VECTOR_INDEX`** — post-filtre par colonnes + canari).
5. **Reranking — chantier 3 entier** : trait `Reranker`, `SearchOptions.rerank`
   (pool rescoré avant pagination), trois cross-encoders sur burn avec parité
   candle : `ms-marco-MiniLM-L-6-v2` (90 Mo, EN), **`mmarco-mMiniLMv2-L12-H384-v1`**
   (470 Mo, 14 langues, défaut multilingue), `bge-reranker-v2-m3` (2,2 Go).
   Suites `e2e_rerank` (3, mock), `e2e_burn_reranker` (5), `e2e_burn_xlmr_reranker` (8).
6. **Dense multilingue** : `paraphrase-multilingual-MiniLM-L12-v2` sur burn
   (470 Mo, 50+ langues), `e2e_burn_multilingual_minilm` 5/5, les tests
   « multilingual » d'`e2e_search` tournent dessus.
7. **Quatre burnpacks publiés** ce soir sur HF (`Lucie666/*-burnpack`), fiches
   complètes, empreintes vérifiées après téléchargement. Tous Apache-2.0.
8. Docs : 33 (sparse jumeau), 35 (réponse lucivy), 36 (**vision** : agent =
   sous-graphe compilé en workflow, agents qui construisent des agents, RAG
   embarqué comme substrat distribué), 37 (conception multi-tenant), 39/40
   (lucivy : filtre routé ; publication **3.0.0**).

## Chiffres de référence

```
passe complète E2E (18 suites, burn)     185/185 en ~138 s   (avant les rerankers)
suites ajoutées depuis                   e2e_rerank 3, e2e_burn_reranker 5,
                                         e2e_burn_xlmr_reranker 8, e2e_burn_multilingual_minilm 5
tests lib                                604 (606 sous burn-embedder, 606 sous wasm-emscripten)
e2e_search 38 tests                      ~25 s sur burn
parité burn/candle                       MiniLM 2e-7 · multilingual 1.4e-7 · rerankers ≤ 1.3e-5 (logits)
```

## À faire dans l'immédiat

1. ~~Passe complète après les rerankers~~ — **faite le 25 au matin : 23 suites,
   206/206**.
2. **Le go de Lucie pour lucivy 3.0.0** (`cargo publish`, irréversible) ; à la
   publication, remplacer nos path deps par `lucivy-core = "3"`,
   `sparse-vector = "0.3"`, `luciole = "0.2"` — et vérifier qu'il n'y a
   **qu'une** entrée `luciole` dans `Cargo.lock`.
3. ~~OCR en usage unitaire~~ — **livré le 25 au matin (doc 46)** : `OcrNode`,
   trait `Ocr`, PP-OCRv6 tiny sur burn (6,2 Mo, feature `burn-ocr`),
   e2e_burn_ocr 4/4, parité onnxruntime. Reste : le go pour publier
   `Lucie666/ppocrv6-tiny-burnpack` (fiche prête dans le scratchpad).
4. **4 bis en cours (doc 47)** : repérage fait et étape 1 livrée (trait `Llm`
   par puits, `MockLlm`, `LlmNode`, outils depuis les `NodeSchema`).
   burn-onnx **sait** faire un décodeur à cache KV (Qwen2.5-0.5B à 25 j/s,
   `Luciole-1B` francophone réexporté et vérifié) ; Zipformer fr et Kokoro
   passent aussi. Reste les étapes 2 à 6, 11 à 14 j-h.
5. Puis **bilan** : use cases, ou encore de la solidification ?

## Dettes nommées (ne pas les oublier, ne pas les rouvrir sans raison)

- **kuzu / extension vector** : `QUERY_VECTOR_INDEX` sur graphe projeté rend
  des nœuds hors projection → tous les filtres vectoriels utilisateur sont
  des post-filtres aujourd'hui. Canari `e2e_scope::canary_kuzu_projected_graph_vector_filter_is_ignored`.
  Investigation C++ : `extension/vector/src/function/query_hnsw_index.cpp`
  (masques sémantiques).
- **Build WASM non revalidé** depuis mai ; liaisons emscripten
  `setScope`/`getScope` écrites, pas compilées ; les extensions C++ retirées
  de la liste statique WASM.
- Le reranker **remplace** le score de fusion (blend possible plus tard).
- RBAC : pas maintenant ; charnière = `set_scope` + future vue restreinte
  (`restrict_to(cells)`), rôles = données du graphe avec le chantier MCP.
- **`ShardedHandle::compact(max_docs)`** (lucivy, doc 45) : à appeler après un
  chargement en masse (−21 % de sidecars, moins de FST à ouvrir par requête) —
  exposer un `Catalog::compact()` ou l'accrocher à la fin de `drain`/`reindex`.
- `sparse-vector` : plan lucivy en 4 étapes vers lucistore (BlobDirectory,
  ShardStorage, sharding, delta) — après la fiabilisation, avec eux.
- Reportés derrière tout ça : codeparsers (avec `project` dès le premier
  jour), composite booléen typé pour agents, éval par replay, Eager vs Lazy,
  bench corpus réel.
