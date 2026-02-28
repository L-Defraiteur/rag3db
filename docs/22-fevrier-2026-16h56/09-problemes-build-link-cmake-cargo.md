# 09 — Problemes de build / link entre cmake et cargo

## Architecture du build

```
rag3db (cmake)
  └── extension/tantivy_fts/ (cmake)
        ├── CMakeLists.txt ← orchestre le build Rust + C++
        ├── src/ (C++) ← extension rag3db (create/query/drop)
        └── ../tantivy/ld-tantivy/ (cargo workspace)
              ├── src/ ← fork de Tantivy (ld-tantivy crate)
              ├── tantivy_fts/rust/ ← crate FFI bridge (tantivy-fts crate)
              └── target/release/
                    ├── libtantivy_fts.a ← staticlib finale (inclut ld-tantivy + deps)
                    └── cxxbridge/ ← headers + .cc generes par cxx
```

La chaine : `cmake → cargo build → libtantivy_fts.a → link dans tantivy_fts_test`

## Probleme 1 : cmake ne re-run pas cargo

### Ancien CMakeLists.txt (avant fix)

```cmake
file(GLOB_RECURSE RUST_SOURCES "${RUST_WORKSPACE_DIR}/src/*.rs" ...)
add_custom_command(
    OUTPUT ${TANTIVY_STATIC_LIB}
    COMMAND cargo build --release
    DEPENDS ${RUST_SOURCES})
add_custom_target(tantivy_fts_rust DEPENDS ${TANTIVY_STATIC_LIB})
```

**Problemes :**
- `add_custom_command(OUTPUT ...)` ne re-run que si OUTPUT est plus vieux que DEPENDS
- Si on fait un `cargo build` manuellement, le `.a` est mis a jour et cmake skip le custom_command
- `GLOB_RECURSE` est evalue a la configuration cmake, pas au build → les nouveaux fichiers `.rs` ne sont pas detectes sans `cmake ..` re-configuration
- Resultat : apres avoir modifie un `.rs`, cmake pense que tout est a jour

### Fix applique

```cmake
add_custom_target(tantivy_fts_rust
    COMMAND cargo build --release
    BYPRODUCTS ${TANTIVY_STATIC_LIB}
    COMMENT "Building tantivy-fts Rust static library")
```

- `add_custom_target` (sans OUTPUT) est **toujours** considere out-of-date → cargo tourne a chaque build
- Cargo a sa propre detection incrementale : si rien n'a change, il finit en <0.5s
- `BYPRODUCTS` informe cmake/ninja que la commande produit le `.a`
- Plus besoin de `GLOB_RECURSE` ni de `DEPENDS` — cargo gere tout

**Statut : APPLIQUE dans le CMakeLists.txt courant**

## Probleme 2 : cmake ne re-link PAS quand le .a change

### Ancien CMakeLists.txt

```cmake
add_library(tantivy_fts_lib STATIC IMPORTED GLOBAL)
set_target_properties(tantivy_fts_lib PROPERTIES
    IMPORTED_LOCATION ${TANTIVY_STATIC_LIB})
```

**Probleme :** `STATIC IMPORTED` ne track pas le mtime du fichier `.a` pour decider de re-linker les targets dependantes. Meme si le `.a` change, cmake ne re-link pas `tantivy_fts_test`.

### Fix applique

```cmake
add_library(tantivy_fts_lib INTERFACE)
target_link_libraries(tantivy_fts_lib INTERFACE ${TANTIVY_STATIC_LIB})
add_dependencies(tantivy_fts_lib tantivy_fts_rust)
```

- `INTERFACE` library avec `target_link_libraries(INTERFACE ...)` passe le `.a` comme flag de link direct
- cmake track les full paths passes a `target_link_libraries` et re-link si le fichier change
- `add_dependencies` assure que cargo tourne AVANT le link

**Statut : APPLIQUE dans le CMakeLists.txt courant**

## Probleme 3 : le linker strip les objets de ld-tantivy du .a

### Symptome observe

Les strings de debug de `ld-tantivy/src/query/intersection.rs` sont **presentes dans libtantivy_fts.a** mais **absentes du binaire final tantivy_fts_test**.

Pourtant les strings de `tantivy_fts/rust/src/query.rs` et `ld-tantivy/src/query/phrase_query/ngram_contains_query.rs` apparaissaient dans le binaire (quand il etait correctement linke).

### Explication probable

Le linker (ld/gold/lld) inclut les `.o` d'un `.a` **uniquement** si ils resolvent un symbole non-resolu. La chaine d'appel est :

```
C++ (cxx bridge) → tantivy-fts (bridge.rs) → ld-tantivy (query, search, etc.)
```

Si le compilateur Rust a inline `intersect_scorers` dans `boolean_weight.rs` au moment de la compilation (LTO ou inlining standard en release), l'objet `.o` contenant `intersect_scorers` n'est plus reference par aucun symbole non-resolu → le linker le drop.

Les fonctions de `ngram_contains_query.rs` et `query.rs` sont probablement dans le meme `.o` ou compilees separement et referencees directement → elles sont incluses.

### Solutions possibles

#### A. `--whole-archive` (force l'inclusion de tout le .a)

```cmake
target_link_libraries(tantivy_fts_lib INTERFACE
    -Wl,--whole-archive ${TANTIVY_STATIC_LIB} -Wl,--no-whole-archive)
```

Avantage : simple, garantit que tout le code Rust est dans le binaire.
Inconvenient : binaire plus gros (inclut du code potentiellement inutilise).

#### B. `#[inline(never)]` sur les fonctions cles

```rust
#[inline(never)]
pub fn intersect_scorers(...) { ... }
```

Avantage : pas de changement cmake, force le compilateur a garder la fonction.
Inconvenient : ne marche que si le symbole est reference (pas garanti avec LTO).

#### C. Desactiver LTO pour debug

Dans `Cargo.toml` du workspace, temporairement :
```toml
[profile.release]
lto = false
```

Avantage : chaque crate produit ses propres `.o`, pas de cross-crate inlining.
Inconvenient : binaire potentiellement plus gros/lent, changement temporaire.

#### D. Utiliser `cdylib` au lieu de `staticlib` pour tantivy-fts

Au lieu de compiler en `.a` (staticlib), compiler en `.so` (cdylib). Un `.so` inclut tout le code necessaire et n'est pas strip par le linker.

Avantage : resout definitivement le probleme de stripping.
Inconvenient : ajoute une dependance runtime a un .so, complexifie le deploiement.

### Recommendation

Pour le debug immediat : **option A** (`--whole-archive`).
Pour la production : **option A** avec un flag cmake ON/OFF, ou laisser tel quel si les debug prints sont supprimes.

## Probleme 4 : miniconda LD_LIBRARY_PATH

### Symptome
```
libstdc++.so.6: version 'GLIBCXX_3.4.31' not found
```

### Cause
`~/miniconda3/lib/libstdc++.so.6` est dans `LD_LIBRARY_PATH` et est trop vieux.

### Workaround
```bash
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./tantivy_fts_test
```

### Fix propre
Retirer miniconda du PATH/LD_LIBRARY_PATH dans le shell de dev, ou ajouter le workaround dans un script de test.

## Resume des actions

| Probleme | Statut | Action |
|---|---|---|
| cmake ne re-run pas cargo | FAIT | `add_custom_target` au lieu de `add_custom_command` |
| cmake ne re-link pas | FAIT | `INTERFACE` lib au lieu de `IMPORTED STATIC` |
| Linker strip objets Rust | A FAIRE | `--whole-archive` dans CMakeLists.txt |
| miniconda LD_LIBRARY_PATH | WORKAROUND | `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu` |

## Workflow de build recommande apres les fix

```bash
# Une seule commande — cmake invoque cargo, cargo est incremental, cmake re-link si besoin
cd packages/rag3db/build/release
cmake --build . --target tantivy_fts_test -j$(nproc)

# Lancer le test
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/tantivy_fts/test/tantivy_fts_test
```

Plus besoin de :
- `cargo build --release` manuellement
- `rm -f tantivy_fts_test` pour forcer le re-link
- `touch` des fichiers `.rs`
- `cmake --build . -j$(nproc)` complet
