# Doc 03 — Intégration burn dans rag3weaver + chantier A (builds cassés)

Date : 23 août 2026
Suite des docs [01](01-reprise-etat-et-plan.md) et [02](02-spike-burn-vulkan-amd.md).

**Résumé : `BurnBgeM3Embedder` est intégré et validé, et les cibles de build cassées sont
réparées. La matrice complète compile avec `-D warnings`, 581 tests verts.**

---

## 1. L'embedder burn, intégré derrière tes traits

### Ce qui a été ajouté

| Fichier | Rôle |
|---|---|
| `src/burn_bge_m3_embedder.rs` | `BurnBgeM3Embedder` — impl `Embedder` + `SparseEmbedder` + `DualEmbedder` |
| `generated/bge_m3_onnx.rs` | modèle généré par burn-onnx (8 974 lignes, ~29 Ko compressé) |
| `generated/bge_m3_sparse_linear.bin` | tête sparse, 4 100 octets (1024 poids + biais, f32 LE) |
| `generated/README.md` | provenance, régénération, pièges |
| `examples/burn_vs_candle.rs` | parité de bout en bout, via les traits publics |
| `examples/burn_throughput.rs` | balayage de débit batch × longueur |

Feature `burn-embedder` dans `Cargo.toml`, optionnelle, qui n'affecte pas le build par
défaut. Le modèle généré est déclaré via `#[path = "../generated/bge_m3_onnx.rs"]` : il
reste hors de `src/`, ce qui garde visible qu'il n'est pas du code écrit à la main.

### La tête sparse est embarquée, pas téléchargée

`sparse_linear.pt` de BAAI est en **f16** (`HalfStorage`), et `burn_store::PytorchStore`
n'expose aucun hook de cast dtype — il applique bien `PyTorchToBurnAdapter` (transposition
`Linear` `[out,in]` → `[in,out]`) mais pas la conversion de précision. Chargement direct
impossible : `DTypeMismatch`.

Solution retenue : extraire les valeurs une fois, les élargir en f32, et les **embarquer
dans le crate** via `include_bytes!`. 4 Ko — à cette taille, ça supprime la question du
chargement, du téléchargement et du dtype d'un coup. Élargir en f32 ne perd rien : c'est
déjà ce que fait candle (`VarBuilder::from_pth(..., DTYPE=F32, ...)`).

Les poids du backbone, eux, restent externes (2,2 Go) :
**https://huggingface.co/Lucie666/bge-m3-burnpack**

### Parité confirmée via les traits publics

`DualEmbedder::embed_dual` — un seul forward pour les deux représentations :

```
DENSE   [0] cosinus 1.00000012   [1] 1.00000036   [2] 0.99999958
SPARSE  nnz 7=7, 9=9, 14=14 — indices identiques, max|Δ| < 6e-07
```

C'est le chemin de production qui est testé, pas un spike : tokenisation incluse.

### Débit mesuré

```
 batch    seq      temps       tokens        tok/s     ms/doc
     1     32     156.5ms           32          204     156.5
    64     32     274.2ms         2048         7470       4.3
    16    128     273.2ms         2048         7495      17.1   ← optimum
    64    128    1490.7ms         8192         5495      23.3
     4    512     323.4ms         2048         6332      80.9
    64    512    6052.6ms        32768         5414      94.6
```

**Le 63 ms du doc 02 sous-estimait d'un facteur 8** : il mesurait de la latence de
lancement. Le coût par token chute de **37×** entre batch 1 et batch 64. Le plateau réel
est à **~7 500 tokens/s**, atteint dès batch 16 × seq 128.

Au-delà, ça se dégrade (attention quadratique + pression mémoire) : **ne pas grossir les
batchs aveuglément**, l'optimum est batch 16, seq ~128.

Cohérence matérielle : un A100 annonce ~60 000 tok/s sur BGE-M3, soit 8× plus. Décomposé :
~6,6× d'écart de puissance brute, ×2 pour le fp16 contre fp32, plus l'absence de
flash-attention. **Aucune pathologie — c'est ce qu'on attend d'une R9700 en fp32.**

Ordres de grandeur à 7 000 tok/s sur une carte : 1 M tokens ≈ 2,5 min, 10 M ≈ 24 min,
100 M ≈ 4 h, sources kernel Linux (~400 M) ≈ 16 h. Leviers restants : fp16 (potentiel ×2,
non concluant à ce stade), flash-attention (`cubek-attention` est dans l'arbre, reste à
vérifier s'il est emprunté), et la **deuxième R9700** (×2 par sharding, adressabilité
indépendante déjà vérifiée).

---

## 2. Chantier A — les builds cassés

### La cause racine, trouvée

`.github/workflows/` contient **29 workflows, tous hérités de kuzu**. **Aucun ne couvre
`extension/rag3weaver`.** C'est l'explication complète de la dette invisible : rien ne
compilait ce crate en CI, donc rien ne signalait que trois cibles étaient mortes depuis
la migration async→sync du 17 mai.

### Ce qui était cassé — le doc 01 sous-estimait

| Cible | Diagnostic du doc 01 | Réalité |
|---|---|---|
| `postgres` | 5 appels `execute_with_params_sync` | ✅ exact |
| `postgres` | `tokio/rt` non activé | ❌ **faux** — `deadpool-postgres` l'active transitivement. Ça marche, mais c'est fragile : tu dépends des features d'une dépendance tierce |
| `wasm-emscripten` | 4 `block_on` obsolètes | ⚠️ **incomplet** — il y avait aussi `keyword_weight` disparu de `SearchOptions`, et **2 `match` non exhaustifs** sur `CypherValue::Blob` |
| `examples` | 3 exemples async | ✅ exact |
| `rag3db-native` | non mentionné | ❌ **cassé aussi**, mais pour une raison différente (voir §3) |

### Les corrections

**postgres** — les 5 `execute_with_params_sync` → `execute_with_params`, plus 3 imports
morts (`BTreeMap`, `Arc`, `CypherValue`).

**wasm-emscripten** — 4 `block_on` retirés (`catalog.rs:2128`, `wasm_ffi.rs:916/1295/1583`),
`keywordWeight` remappé sur `FusionConfig.bm25.weight` (le champ par signal a remplacé le
champ global lors du refactoring fusion), et un doc comment déplacé à l'intérieur du
`thread_local!`.

**Le point qui mérite attention : `CypherValue::Blob`.** Deux `match` du FFI WASM ne
couvraient pas cette variante, ajoutée après coup. L'API C exposée au WASM **n'a pas de
constructeur de blob**. Deux options s'offraient : encoder en base64/texte, ou échouer.

J'ai choisi **l'échec explicite** — `bind_param` retourne une `DbError::TypeError`
nommant la taille du blob, `cypher_to_c_value` retourne null. Encoder un blob en texte
aurait corrompu la colonne cible sans que rien ne le signale.

> **Conséquence fonctionnelle à connaître : `CypherBlobStore` n'est pas utilisable via le
> chemin FFI WASM.** Ça concerne la persistance des index FTS et sparse côté navigateur.
> Ce n'est pas une régression introduite ici — le code ne compilait pas du tout avant —
> mais c'est un trou réel, désormais visible et documenté plutôt que silencieux.

**examples** — `tei_reqwest` passé à `reqwest::blocking` (feature ajoutée), `candle_local`
débarrassé de ses `.await`. Pour `tei_openai`, `async-openai` n'a pas d'API bloquante :
l'embedder **possède désormais son propre runtime tokio** et fait `block_on` dessus. C'est
volontairement le patron correct, et il vaut comme démonstration : `Handle::current()`
paniquerait hors réacteur ou depuis un worker tokio. Les `required-features` des nouveaux
exemples ont été déclarées dans `Cargo.toml`.

### Le passif de warnings, résorbé

Le CI pose `RUSTFLAGS: -D warnings`, ce qui exigeait de nettoyer 8 warnings sur la lib et
8 sur le build de test (que `cargo check --lib` ne voit pas). Traités : imports morts
supprimés, variables inutilisées préfixées, code mort marqué `#[allow(dead_code)]` plutôt
que supprimé, et deux morceaux gatés sur leur vraie feature (`best_device` et l'import
`DefaultModel` ne servent qu'aux chemins hf-hub, donc inutilisés sous `candle-wasm`).

### État final

```
lib défaut                   ✅      examples défaut              ✅
lib no-default-features      ✅      examples bge-m3              ✅
lib bge-m3                   ✅      examples burn-embedder       ✅
lib candle-wasm              ✅
lib burn-embedder            ✅      cargo test --lib   581 passed; 0 failed
lib postgres                 ✅
lib wasm-emscripten          ✅      (le tout avec -D warnings)
```

### Le garde-fou

`.github/workflows/rag3weaver-workflow.yml` — matrice de 7 combinaisons de features, en
`--lib` et `--examples`, plus les tests unitaires, plus fmt/clippy en informatif.

Une note importante y figure : **chaque combinaison est testée séparément**. Un
`--all-features` vert ne prouve rien sur les combinaisons individuelles, parce que
l'unification des features de cargo fait qu'une dépendance activée par une autre feature
peut masquer un `dep:` manquant. C'est exactement le piège dans lequel `postgres` était
tombé avec `tokio/rt`.

Le job ne construit **pas** `librag3db.so` : ce serait long et ça masquerait les
régressions Rust derrière des soucis de toolchain C++. Les 152 tests E2E restent manuels
via `run_e2e.sh`.

---

## 3. Ce qui reste ouvert

### `rag3db-native` est cassé, mais pas côté Rust

```
error: failed to run custom build command for `rag3db v0.11.1`
  tools/rust_api/rag3db-src/third_party/thrift/transport/TTransport.h:36:1:
  error: 'uint32_t' does not name a type
```

C'est le thrift vendorisé de kuzu qui ne compile plus : `#include <cstdint>` manquant,
que GCC 13+ ne pardonne plus (les includes transitifs ont été resserrés). **Ce n'est pas
du code à toi, et ça bloque aussi `run_e2e.sh --build`** — donc tout le chantier B.

Correction probable : ajouter l'include dans le thrift vendorisé, ou compiler avec
`-include cstdint`. À traiter en premier au prochain passage sur l'E2E.

### Le reste

1. **BM42** — export ONNX depuis PyTorch au build (tu as torch 2.13 + ROCm), puis le même
   pipeline que BGE-M3. Le chemin est balisé.
2. **#237** — FTS `ShardedHandle` en Rust direct, 25 call sites, patron = `dbcf494ca`.
3. **Unifier les deux chemins de search** — les generic search nodes appellent encore les
   fonctions legacy avec `&conn` brut, donc le pipeline DAG ne marchera pas sur Postgres.
4. **Tests d'intégration Postgres** — le backend compile de nouveau, mais n'a toujours
   jamais été exécuté. `docker/docker-compose.supabase.yml` n'est utilisé par rien.
5. **geo** — E2E dédié ou sortie des extensions par défaut.
