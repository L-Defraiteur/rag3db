# Les forks de burn, cubecl et cubek

Trois crates du registre ont besoin d'une ligne chacune pour que **Flex32**
(stockage f32, matmul f16 sur les tensor cores, accumulation f32) marche de
bout en bout sur Vulkan. Elles vivent dans des forks, pas ici :

| crate | fork | fichier | la ligne |
|---|---|---|---|
| cubecl-wgpu | https://github.com/L-Defraiteur/cubecl | `crates/cubecl-wgpu/src/backend/vulkan.rs` | Vulkan inscrit `FloatKind::Flex32` parmi ses types, comme WGSL |
| burn-cubecl | https://github.com/L-Defraiteur/burn | `crates/burn-cubecl/src/ops/tensor.rs` | `float_from_data` accepte un `TensorData` en Flex32 |
| cubek-matmul | https://github.com/L-Defraiteur/cubek | `crates/cubek-matmul/src/definition/spec.rs` | une sortie Flex32 accumule en f32, comme f16 et bf16 |

Branches : `rag3weaver/pre.3` (du tag `v0.11.0-pre.3` / `v0.22.0-pre.3` /
`v0.3.0-pre.3`, ce que le lock utilise depuis le 6 septembre 2026 au soir) et
`rag3weaver/pre.2` (même commit sur la version d'avant, gardée pour comparer). `Cargo.toml`
les prend par `[patch.crates-io]`, **chaque atelier en entier** (51 crates) :
patcher un seul crate d'un dépôt tire ses voisins en double de ceux du
registre, et deux `cubecl_common::Device` ne sont pas le même trait. Tout ce
qui n'est pas la ligne du commit est identique à crates.io. cargo clone les
dépôts une fois dans `~/.cargo/git`.

Sans ces lignes, chaque trou rendait des vecteurs identiques au bit près à
f32, sans un message. Avec, BGE-M3 passe de 3 800 à 11 500 jetons/s sur un lot
de 64 × 100 mots, cosinus 0,999999 contre f32. Le récit :
`docs/issues/6-septembre-2026/03-le-chemin-vulkan-ce-qu-il-valait-et-les-lignes-qui-manquaient.md`.

Ce sont trois PR amont d'une ligne. Le jour où elles sont fusionnées, les
trois entrées `[patch]` disparaissent. Le quatrième trou,
`TensorData::convert_dtype(Flex32)` de burn-std qui ré-étiquette f32, est
contourné chez nous (`Flex32Adapter`, `src/burn_device.rs`) plutôt que patché.
