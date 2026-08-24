# Doc 29 — Passation : progression au 24 août 2026, fin de journée

Point d'entrée pour la session suivante. Trois docs : celui-ci (où on en est),
[30 — architecture et intention](30-passation-architecture-et-intention.md),
[31 — mode d'emploi](31-passation-mode-d-emploi.md). Les docs 11-13 (hier) et 14
(mémoire des ambitions) restent valables, mais celui-ci a la priorité s'ils se
contredisent.

## Git

| | |
|---|---|
| Branche | `fts-lucivy-v3`, **37 commits devant `master`**, tout poussé |
| `master` | `962ce9dc2`, intact — fusionner est la décision de Lucie, pas la mienne |
| `.gitmodules` | modifié localement par Lucie (routage SSH) — **ne pas commiter** |
| Submodule `ld-lucivy` | épinglé `34ec432` ; référence seulement (plus aucun build C++ ne l'utilise) |
| lucivy compilé par rag3weaver | `~/git_workspaces/lucivy` (path deps `lucivy-core`, `luciole`, `sparse-vector` → `lucistore`), arbre de la session lucivy, à `34ec432` en fin de soirée |

## Ce qui a été livré aujourd'hui, dans l'ordre

1. **Doc 14** — mémoire des ambitions, 323 docs relus par 5 agents. Constat clé :
   depuis mars, six chantiers d'infra, zéro des six chantiers produit.
2. **`BurnBgeM3Embedder` dans le `Catalog`** — E2E 4/4, trois signaux réels en un
   forward, requête FR sans recouvrement lexical résolue par le dense seul.
3. **`BM25Mode::Symbol`** — recherche exacte séparateurs inclus. Trouvaille : le
   moteur savait, rag3weaver n'émettait jamais `strict_separators` et forçait
   `distance ≥ 1` (qui force relaxed). 12 E2E : `foo->bar`, `};`, `c++`, generics,
   emoji + ZWJ, accents, tiret cadratin.
4. **`meta.warnings` toujours peuplé** + `ChunkAttributionMiss` (NoHighlights /
   HighlightsOutsideContent / NoOverlap / NoChunks) — on ne confond plus « pas
   de spans » et « spans sans recouvrement ».
5. **3 bugs d'enregistrement KB** (entité enregistrée après sa KB) : titre
   non déterministe (HashMap), titre lu au niveau KB, DDL `_SOURCED_` jamais
   rejouée — et le trou d'observabilité qui les cachait (`ingest_entities`
   jetait le résultat du drain d'agrégation). 22/22 stable, était 20/21 aléatoire.
6. **Repli C++ débranché, index C++ supprimé** — chaque document était indexé
   deux fois. `SEARCH()` en Cypher natif n'est plus disponible sur les tables
   rag3weaver (assumé). Suites 2-5× plus rapides.
7. **`BufferedBlobStore` + `UNWIND`** — 1 518 `save()` par drain → 114 clés → 1
   requête. Read-your-writes, deletes non tamponnés, flush aux 4 frontières
   (drain, reindex, shutdown, Drop). Flush = 4,5 ms.
8. **lucivy, quatre correctifs obtenus** par nos rapports : fsync sur cache
   jetable (`9a66fbf`), stall commit/`.managed.json` (`e6176f5`), dernier mot
   sans séparateur (`36b1edd`), `parse` booléen avec highlights (`8f14edc`),
   puis double free luciole + `request` en `Err` + handle fermé (`3675c3d`, `3c282c7`).
9. **MiniLM sur burn** — parité candle 1.0000000x, E2E 3/3 (172 ms de
   chargement), publié : `Lucie666/all-minilm-l6-v2-burnpack` (Apache-2.0,
   attribution complète, checksum vérifié).
10. **`default = []`, BM42 retiré** (−831 lignes, `CandleDualEmbedder` compris).
    Matrice : 10 combinaisons `check`, exemples sous 4 jeux, 598/601/600 tests
    lib. Le défaut candle masquait 4 trous (wasm sans garde, imports morts,
    exemples sans `required-features`, tests bge-m3 qui ne compilaient pour
    personne).
11. **Chasse à la corruption mémoire** — gdb, `MALLOC_CHECK_`, valgrind (installé,
    `RAG3DB_MAX_DB_SIZE` pour passer la réservation 8 TiB de kuzu). Double free
    d'un `Box<dyn Node>` pris par `ptr::read` dans luciole : **fermé**. Notre
    `Drop` blindé (`catch_unwind` autour de `close()`).

## Chiffres mesurés ce soir

```
profil 9 docs, drain          1331 ms  →  97 ms
e2e_symbol_search             32,1 s   →  1,2 s   (12 tests)
e2e_search (13, natif)        60,6 s   →  ~3 s
e2e_idempotent_registration   80,9 s   →  9,9 s   (22/22)
passe complète 12 suites      ~4 min   →  ~90 s
passe complète 18 suites (soir)  176/176 en 131 s   (candle sorti des E2E, 4 suites de mai remises)
drain(N) ≈ 85 ms fixes + 0,95 ms/doc   (pas un problème : sous le coût du moteur)
```

## Ce qui n'est PAS vert, et pourquoi

| | Cause | Chez qui |
|---|---|---|
| ~~`e2e_search` 37/38 aléatoire~~ | **fermé le 24 au soir** : scorer fuzzy/regex non trié (`HashMap`) corrigé côté lucivy `8a91053`, `close()` tolérant `a37d330` (doc 32). Rejoué 13/13 ×3 + 38/38. | — |
| ~~`simple_register_duplicate_fails`~~ | **supprimé** (obsolète depuis mai, `register_entity` idempotent) | — |
| ~~4 fichiers E2E ne compilent pas~~ | **réparés le 24 au soir** (24 tests, 4 sujets que rien d'autre ne couvrait : undo, `search_with_strategy`, observabilité, nœuds génériques). Voir « Bugs trouvés en les remettant ». | — |

## Bugs trouvés en remettant les 4 E2E (24 août, soir) — tous corrigés

Aucun n'était visible tant que ces suites ne compilaient pas.

| Bug | Où | Correctif |
|---|---|---|
| **Perte silencieuse dans l'index FTS sur mise à jour partielle** : `UpdateRecordNode` ré-indexait avec les seuls champs *modifiés* (`add_document` n'est pas un merge) → mettre à jour `description` seule effaçait `name`/`details` de l'index. Masqué parce que tous les tests mettaient tous les champs à jour. | `record_nodes.rs` | candidats = tous les champs de l'entité (`entity_indexed_fields`) ; test `simple_partial_update_keeps_other_fields_indexed`, **contre-épreuve faite** (échoue sans le correctif). |
| **Undo d'update laissait l'index FTS périmé** : les colonnes étaient restaurées, pas le document indexé → la recherche renvoyait encore le contenu annulé. | `UpdateRecordNode::undo` | ré-indexation factorisée (`reindex_fts_rows`) partagée par `execute`/`undo` ; `bind_services` + `bind_fts` pour rejouer un undo sur un nœud frais (reprise après crash) ; `Catalog::{dialect_arc, fts_handles, sparse_handles, entity_configs}`. |
| **Fan-out cassé sur les ports partagés** : depuis luciole, `PortValue::take()` panique si un port a plusieurs consommateurs ; or le port `query` d'une source alimente recherche *et* résolution, `results` plusieurs nœuds. Touchait `search_with_strategy` avec expansions (chemin produit). | `search_nodes.rs`, `generic_search_nodes.rs`, `port.rs` | `take_or_clone` : déplacement si seul consommateur, clone sinon. |
| **Garde `max_rounds` disparue** de `search_with_strategy`. | `catalog.rs` | `max_rounds == 0` ou `expansions > max_rounds` → erreur « max iterations guard ». |
| **Résumés d'arêtes vides** dans `ExecutionReport` : calculés après l'exécution sur des valeurs déjà déplacées. | `report.rs` | résumé depuis les instantanés `NodeStarted` (`summarize_snapshot`). |
| **Deux copies de `luciole`** dans le graphe : crates.io 0.1.0 pour notre DAG (`luciole_bridge`), la copie locale pour `lucivy-core` — le correctif du double free (`3675c3d`) n'était **pas** dans la nôtre. | `Cargo.toml` | `[patch.crates-io] luciole = { path = "../../../lucivy/luciole" }`, une seule copie dans le lock. |
| 10 tests unitaires `bge_m3_embedder.rs` en `#[tokio::test]` sur des fonctions synchrones (ne compilaient sous `bge-m3`, combinaison absente de la matrice `--lib`). | `bge_m3_embedder.rs` | `#[test]`. |

## Décisions prises aujourd'hui (ne pas rouvrir sans raison)

- `default = []` — la crate est un orchestrateur ; burn = chemin produit ; candle = oracle.
- BM42 retiré ; le sparse vient de BGE-M3 (tête apprise). MiniLM = dense seul, défaut navigateur, `LoadStrategy::Bytes`.
- `BM25Mode::Parse` gardé **opt-in** (boîte de recherche humaine), jamais défaut. Pour les agents : exposer un composite booléen **typé** (`must`/`should`/`must_not` de feuilles `Symbol`/relaxed) — pas fait.
- Le repli C++ ne revient pas. L'extension `lucivy_fts` C++ et son submodule sont du code mort à supprimer.
- org/project : **révisé le 24 août au soir** — Lucie en fait un chantier à part entière, placé juste après la fiabilisation (voir l'ordre ci-dessous). Piste à froid : une base par org (isolation par fichier) + `project` en champ filtré ; à trancher au démarrage du chantier.
- Corpus réel (kernel) : bench `#[ignore]` manuel, jamais dans la passe récurrente.

## Prochaines étapes, dans l'ordre fixé par Lucie (24 août, soir)

1. **Finir la fiabilisation / tests.** ~~Épingler lucivy `832c503`, rejouer `e2e_search`~~ fait ; ~~supprimer `simple_register_duplicate_fails`~~ fait ; ~~E2E à vrai modèle sur burn~~ fait (`tests/common/mod.rs`, `e2e_search` 234 s → 23 s, candle ne reste que dans `examples/`) ; ~~supprimer l'extension C++ `lucivy_fts`~~ fait (cmake, `extension_entries.cpp`, 16 helpers de test, README/BUILD.md) — **le submodule `extension/lucivy/ld-lucivy` reste** (le retirer touche `.gitmodules`) ; **tranché par Lucie le 24 au soir : la path dep reste sur `~/git_workspaces/lucivy`, l'arbre vivant de la session lucivy** — la v3 n'est publiée nulle part, et c'est l'usage naïf de rag3weaver qui la valide (les gros benchs côté lucivy ne voient pas les « petits trucs à la con » comme le `HashMap` non trié). Ne pas « corriger » ça vers le submodule ; `sparse_vector` avait la même anatomie (doc 33) → **réponse lucivy doc 34 : le crate vit chez eux** (`lucivy/sparse_vector`, Apache-2.0 car dérivé de Qdrant, sur `lucistore`) ; ~~bascule de la path dep, suppression de notre copie Rust et de l'extension C++ `sparse_vector`~~ fait (doc 35) — **plus aucune extension C++ n'embarque de Rust**, cmake ne construit que `vector` et `geo` ; ~~remettre les 4 E2E qui ne compilent plus~~ fait — **toutes les suites E2E compilent et passent sur burn : 176/176 en 131 s** (18 fichiers, lucivy `34ec432`, sparse via le crate lucivy, build natif `vector;geo`).
2. **org id / project id.** Décision d'architecture d'abord (base par org vs colonne), puis `project` comme champ + filtre dans l'API de recherche.
3. **Cross-encoder** (reranking) — sur burn, chemin produit, candle en oracle comme pour les embedders.
4. **OCR en usage unitaire** : un petit nœud dataflow minimal, un modèle léger embarquable (PP-OCRv6 ONNX est la piste, cf. la note OCR de Lucie) — **pas** de markitdown ni de lib lourde, pas de use case « pipeline documents » à ce stade.
4 bis. **Briques génératives, même palier** : LLM, TTS, STT — l'objectif dit par Lucie : « tout avoir pour construire n'importe quel use case de workflow agentique / RAG ». Même doctrine que les embedders : modèles open source chargés par burn (burn-onnx / burnpack), candle ou ONNX Runtime en oracle. **Streaming** dès le premier jour (tokens LLM, audio TTS par chunks, STT sur flux), et une interface de streaming compatible avec des fournisseurs cloud (ElevenLabs, Gradium) pour que le nœud soit substituable. Pistes petites et embarquables : Whisper (STT, tiny/base), Kokoro 82M ou Piper (TTS), un LLM ≤ 1-3B quantisé pour le local ; à valider un par un, le LLM sur burn est le plus lourd (KV cache, décodage autorégressif, quantisation).
5. **Se reposer la question** : passer aux use cases, ou le moteur a-t-il encore besoin de solidification ?

Horizon qui donne le sens de cet ordre : doc 36 (agent = sous-graphe compilé en workflow ; agents qui construisent des agents).

Reportés derrière ces cinq points : codeparsers (avec `project` dans le schéma dès le premier jour), composite booléen typé pour les agents, `boolean`+`filters` / `more_like_this` de lucivy, éval, Eager vs Lazy, bench corpus réel.
