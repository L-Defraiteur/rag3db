# Le paquet npm : le premier bâti, Linux et Windows

**10 octobre 2026, session « optimiseur », chantier G.** Les deux premiers
gestes de [la page de cadrage](../../8-octobre-2026-16h29/orchestration/03-le-paquet-npm.md) :
un bâti Linux x86_64 dans un Docker commité, et un essai Windows sur un
runner GitHub pour la liste réelle des accrocs. Branche `paquet-npm`. Rien
n'est publié.

## Ce qui est bâti

Le binaire `rag3weaver-backend`, features `rag3db-native`, `code`, `daemon`,
`openai-llm`, sans `burn-embedder` ; le moteur en statique dedans (la crate
`rag3db` le bâtit par cmake, `tools/rust_api/build.rs`) ; et à côté
`libvector.rag3db_extension`, la seule extension du moteur que rag3weaver
charge à l'exécution (le plein texte lucivy est du Rust compilé dans le
binaire).

La recette : `tools/build-images/linux-x64-gnu/` (un `Dockerfile` sur
`manylinux_2_28`, un `build.sh` qui se joue aussi en natif) ; le workflow
manuel `.github/workflows/paquet-npm.yml`, qui bâtit l'image sur un runner et
essaie Windows, et ne publie rien.

## Linux x86_64 : les chiffres

| | par l'image, runner GitHub (4 cœurs) | en natif, ce poste (8 tâches, `poste lourd`) |
|---|---|---|
| outils | gcc 14.2, cmake 3.31, Rust 1.98.1, glibc 2.28 | gcc 16.2, cmake 4.4, Rust 1.98.1, glibc 2.44 |
| bâti du binaire à froid | 19 min 46 s, puis 13 min 45 s au second run | 10 min 23 s |
| l'extension vecteur ensuite | 33 s (elle réutilise le bâti cmake du moteur) | 9 s |
| binaire, avant et après strip | 97 Mo → **79 Mo** | 98 Mo → 81 Mo |
| extension vecteur, après strip | 684 Ko | 716 Ko |
| bibliothèques demandées | libstdc++, libgcc_s, libpthread, libm, libdl, libc | libstdc++, libgcc_s, libm, libc |
| version glibc la plus haute demandée | **2.28** | 2.44 |
| target | 1,6 Go | 1,6 Go |

C'est l'image qui fait le binaire livrable : bâti ici, il exigerait une
glibc de 2026 ; bâti dans l'image, il tourne sur toute distribution de 2018
ou plus récente (glibc 2.28, celle que lucivy prend aussi).

## Ce qui manque ou ne manque pas à l'exécution

Éprouvé avec le binaire de l'image, téléchargé sur ce poste (noyau 7.2) et
rejoué sur le runner (noyau 6.17) :

- il démarre (`usage: rag3weaver-backend backend.json [--describe]`) et
  parle en **lignes JSON sur l'entrée standard** (`describe`, `call`,
  `journal`, `journal_read`, `index_state`, `shutdown`), pas en HTTP : c'est
  ce que le paquet JS devra lancer et à quoi il parlera ;
- il charge le gabarit de backend de code avec le bac à sable Landlock : le
  noyau accepte le ruleset, `--describe` rend les outils ;
- **Landlock se monte aussi dans un conteneur Docker** (seccomp par défaut,
  runner) : même épreuve, même réponse ;
- sans service d'embarquement, le backend mémoire refuse en le disant
  (« models.embed (bge-m3) : ni `address` […] ni RAG3WEAVER_EMBED_SERVICE ») ;
- rien d'autre n'est à côté : pas de `.so` sauf l'extension vecteur, et les
  poids ne sont pas dans le binaire.

Sous Windows et macOS, le bac à sable n'existe pas : le backend refuse à
l'ouverture (« le bac à sable Landlock demande Linux — "sandbox": {"mode":
"off"} en le sachant ») tant que le manifeste ne déclare pas `mode: off`.
Décision de l'orchestration : ce comportement reste ; c'est le gabarit de
backend livré par le paquet qui, sous Windows, posera `"sandbox": {"mode":
"off"}` avec un commentaire qui dit pourquoi.

## Windows x64 : la liste des accrocs

Sept essais sur `windows-latest` (MSVC 14.51, cmake 4.4, Rust 1.98.1), par la
crate `rag3db` (Ninja, `/EHsc`, `MultiThreadedDLL` — la voie de
`tools/rust_api`), et non par le `make GEN="Visual Studio 17 2022"` de l'amont.

### Contournés dans le workflow, sans toucher nos sources

| accroc | contournement |
|---|---|
| sous le bash de Git, `link.exe` est celui de coreutils, l'éditeur de liens de Rust échoue au premier script de bâti | le bâti en PowerShell |
| `tree-sitter-scss` 1.0.0 (seule version publiée, avril 2024, tirée par codeparsers) passe à `cl` un drapeau GCC, `-Wno-unused-parameter`, que MSVC refuse (D8021) | `clang-cl` pour les crates C (`CC_x86_64_pc_windows_msvc`) |
| cmake refuse de mélanger `clang-cl` pour le C et `cl` pour le C++ du moteur | `clang-cl` pour le C++ aussi |
| avec `clang-cl`, cc-rs compile la crate `cxx` sans exceptions (« cannot use throw with exceptions disabled ») | `CXXFLAGS_x86_64_pc_windows_msvc: /EHsc` |

### Qui demandent nos sources (appliqués sur la branche pour l'essai, à reprendre par leurs propriétaires)

| accroc | changement | à qui |
|---|---|---|
| la crate `landlock` est déclarée pour toutes les cibles et ne compile pas sous Windows, alors que tout le code qui s'en sert est déjà sous `cfg(target_os = "linux")` | `landlock` dans `[target.'cfg(target_os = "linux")'.dependencies]`, `extension/rag3weaver/Cargo.toml` | arbre principal |
| `clang-cl` refuse la copie implicite de `NodeTableDeleteState` (un `std::vector<std::unique_ptr<…>>`), que `cl` laissait passer tant qu'elle n'était pas appelée | `NodeTableDeleteState(const NodeTableDeleteState&) = delete;`, `src/include/storage/table/node_table.h` | cœur C++ |
| le CMake du moteur lie `atomic` pour tout clang hors Apple ; sous `clang-cl` il n'y a pas de `atomic.lib`, `lld-link` échoue sur `rag3db_shared.dll` | `NOT WIN32` dans la condition, `src/CMakeLists.txt` | cœur C++ |
| en statique, bâtir « tout » produit aussi `rag3db_shared.dll`, dont l'export échoue sous `clang-cl` (destructeurs de `std::variant` non émis) | `build_target("rag3db")` au lieu de tout, `tools/rust_api/build.rs` | engine tooling (arbre principal ou cœur C++) |
| `tree-sitter-scss` (voir plus haut) : un `[patch]` vers un fork, ou la grammaire en feature | — | codeparsers |

### Le moteur C++ sous clang-cl

Au sixième essai, **les 1 007 objets du moteur compilent** et `rag3db.lib`
(statique) est produite ; seule la bibliothèque partagée échouait, sur
`atomic.lib`. Au septième, `atomic` retiré, c'est encore elle seule qui
échoue : `lld-link` ne trouve pas des destructeurs de `std::variant`
(`_Variant_storage_<…IndexBufferWithWarningData…>`) que `clang-cl` n'émet
pas pour l'export de la DLL. Cette DLL est inutile en statique : au
huitième essai, en cours au moment du redémarrage du poste
(https://github.com/L-Defraiteur/rag3db/actions/runs/38044225912), la
crate `rag3db` ne bâtit plus que la cible `rag3db` (quatrième changement
de sources, `tools/rust_api/build.rs`, à relire aussi sous Linux : la
recette native relancée pour le vérifier a été arrêtée par la pause).

## Le cache du bâti entre deux versions

lucivy met en cache le registre cargo et le target par `Swatinem/rust-cache`.
Chez nous, le gros du temps est le moteur C++ bâti par cmake dans le target
de cargo (`target/release/build/rag3db-*/out/build`) : le même cache, ou
`actions/cache` sur ce dossier avec une clé sur l'empreinte de `src/` et
`third_party/`, le garderait entre deux versions. Non fait ; vingt minutes
par cible et par version restent tenables.

## Ce qui vient ensuite

1. Le paquet JS qui lance le binaire et lui parle en lignes JSON :
   `rag3weaver` + `rag3weaver-linux-x64-gnu` (les noms de lucivy), un test
   « installer dans un dossier vide, indexer trois fichiers, chercher »,
   publication fermée.
2. Le vecteur en statique (`EXTENSION_STATIC_LINK_LIST=vector`, le lien dans
   `tools/rust_api/build.rs`, et rag3weaver qui ne fait plus `LOAD EXTENSION`
   d'un fichier quand elle est déjà là — un changement du manifeste, à dire à
   l'arbre principal avant), `libvector.rag3db_extension` restant à côté tant
   que ce n'est pas prouvé.
3. macOS, quand Windows est tranché.
