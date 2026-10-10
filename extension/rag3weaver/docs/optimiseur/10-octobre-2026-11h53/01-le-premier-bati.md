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

## Windows x64 : le binaire se lie

**Dixième essai, 10 octobre 2026, 11 h 25 UTC** : `rag3weaver-backend.exe`
est lié, en MSVC pur (cl 14.51, cmake 4.4, Ninja, Rust 1.98.1 sur
`windows-latest`), tree-sitter-scss accepté par `cl` avec la copie de
codeparsers.

| | valeur |
|---|---|
| durée du bâti | 31 min 50 s (4 cœurs, à froid) |
| binaire | 60 Mo (62 950 912 octets), non strippé |
| essais pour y arriver | dix (sept en clang-cl, trois en cl) |

Ce que le dixième essai n'a pas donné : l'artefact (le binaire était hors de
l'espace de travail du runner, `C:/rs`, et `upload-artifact` le refuse), et
l'épreuve `--describe`. Le onzième (38048487047, 47 min 38 s à froid sans
cache) copie le binaire sous `dist/windows-x64/` et l'a téléversé (artefact
`bati-windows-x64-journal`, 20 Mo compressé ; gardé sous
`~/.cache/rag3weaver-build/paquet-npm/artefacts/windows-11/`). Tel quel,
le binaire refuse comme prévu, en nommant quoi faire :

```
le bac à sable Landlock demande Linux — "sandbox": {"mode": "off"} en le sachant
```

Ce refus sort en code 1 et l'étape bash s'est arrêtée dessus : l'épreuve
avec `"sandbox": {"mode": "off"}` n'a pas joué au onzième. Au douzième
(38051420894, 47 min 32 s), elle a joué et rendu, tout nu :

```
The system cannot find the path specified. (os error 3)
```

C'était l'épreuve, pas le binaire : le manifeste était écrit sous `essai/`
alors que les graphes du gabarit lui sont relatifs (`../../tools/…`). Deux
corrections : le manifeste s'écrit à côté du gabarit, comme sous Linux ; et
les lectures du chargement nomment le chemin dans leur erreur
(`lire_octets`, `lire_texte` dans backend.rs) — un « os error 3 » sans
chemin ne dit pas lequel des graphes manque. Le treizième essai les joue.
L'extension vecteur n'est pas encore bâtie sous Windows (le job ne lance
que cargo) : c'est le même geste que sous Linux, à ajouter.

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
| `tree-sitter-scss` (voir plus haut) : copie locale `vendor/tree-sitter-scss` avec `flag_if_supported`, dépendance par chemin | fait par codeparsers (branche `scss-msvc`, 0a09f02) | codeparsers |

### Le moteur C++ sous clang-cl : une impasse, isolée

Au sixième essai, **les 1 007 objets du moteur compilent** sous clang-cl et
`rag3db.lib` (statique) est produite ; au septième, `atomic` retiré, c'est
`rag3db_shared.dll` seule qui échoue ; au huitième, la crate ne bâtissant
plus que la cible statique, c'est l'édition de liens finale de
`rag3weaver-backend.exe` (link.exe de MSVC) qui échoue : LNK2019 sur des
destructeurs de `std::variant` (`_Variant_storage_<…InMemoryExceptionChunk<float>…>`,
`IndexBuilderGlobalQueues::Queue<…>`) que clang-cl n'émet pas dans les objets
du moteur. Le défaut tient à clang-cl avec la STL de MSVC, pas à nos sources :
la même STL et le même code se lient avec cl. **Retour à MSVC (cl) pour tout,
comme l'amont.** Les contournements clang-cl, « clang-cl pour le C++ aussi »
et `/EHsc` sortent du workflow ; l'accroc qui y avait mené, le drapeau GCC de
tree-sitter-scss, se règle à la source chez codeparsers (copie locale
`vendor/tree-sitter-scss` avec `flag_if_supported` et `-utf-8`, branche
`scss-msvc`, en dépendance par chemin — aucun `[patch]` à la racine n'est
nécessaire). Le dixième essai, en MSVC pur sur cette base, dit si le binaire
Windows se lie.

## Le cache du bâti entre deux versions

lucivy met en cache le registre cargo et le target par `Swatinem/rust-cache`.
Chez nous, le gros du temps est le moteur C++ bâti par cmake dans le target
de cargo (`target/release/build/rag3db-*/out/build`) : le même cache, ou
`actions/cache` sur ce dossier avec une clé sur l'empreinte de `src/` et
`third_party/`, le garderait entre deux versions. Non fait ; vingt minutes
par cible et par version restent tenables.

## Le paquet JS, en cours

`extension/rag3weaver/bindings/nodejs/` : `index.js` choisit le binaire
(`RAG3WEAVER_BACKEND`, le sous-paquet de la plateforme, ou `dist/`), prépare
un manifeste depuis les gabarits livrés (chemins de graphes rendus absolus)
et parle au backend en lignes JSON (`Backend.open`, `describe`, `call`,
`journal`, `journalRead`, `indexState`, `shutdown`) ; sous-paquet
`rag3weaver-linux-x64-gnu` (`os`, `cpu`, `libc`) ; `scripts/preparer.sh` y
copie le binaire et l'extension de `dist/` et les gabarits ; rien n'est
publié.

### Le service d'embarquement devient optionnel au démarrage

L'épreuve `npm test` — un dossier vide, trois fichiers, une recherche — a
buté sur une chose de fond : sans service d'embarquement, un backend de code
refusait de s'ouvrir (« un service d'embarquement est requis »). Décision de
l'orchestration (10 octobre) : le service est optionnel au démarrage, et
**jamais de vecteur factice dans le produit** — un vecteur factice est un
résultat faux en silence.

Ce qui est écrit sur la branche, les deux portes :

- `src/bin/rag3weaver-backend.rs` : si `connect_embedder()` échoue, stderr
  « pas de service d'embarquement : index en plein texte seul, vecteurs en
  dette (<raison>) », et le backend s'ouvre sans embarqueur. Une identité
  modèle/dimension différente reste un refus. `rag3daemon` lance ce binaire :
  même règle.
- `PreparedBackend::open` : le refus devient la même phrase
  (`AVERTISSEMENT_SANS_SERVICE`), dite sur stderr **et gardée dans le
  backend** : `describe` et `index_state` la rendent sous `warnings`, parce
  que stderr n'est lu par personne quand le backend tourne en service
  (leçon de la session mémoire : 26 avertissements du catalogue que rien ne
  lit en production).

Ce qui n'y est pas, et pourquoi la branche ne fusionne pas encore : le
catalogue exige un `Embedder` ; sans service, `open` pose aujourd'hui le
`MockEmbedder` des tests, et l'arbre principal a lu que ses vecteurs nuls
**seraient écrits** (phase 2 de l'outil `index`, écriture en ligne, débit
rangé ; `Library` reste HYBRID même avec `index_signals` sur File et Scope ;
risque de plantage HNSW sur des vecteurs identiques). Le cœur est à l'arbre
principal, après `defauts-bascules-2` : un embarqueur **absent**, type
dédié (`is_absent()`), dont `embed` refuse en le nommant, que le catalogue
reconnaît — aucun vecteur écrit, la dette posée et lisible, le plein texte
en ligne, pas de débit rangé, « signal is not available » porté par le repli
de la branche dense. Ma porte posera cet embarqueur à la place du mock quand
son lot sera là. D'ici là, **`npm test` est rouge attendu** : l'épreuve
s'ignore en le disant, et ne joue que sous
`RAG3WEAVER_ESSAI_ROUGE_ATTENDU=1` ; elle attend `warnings` dans `describe`
et dans `index_state`.

Jouée quand même sur le binaire natif porteur des deux portes (13 h 56) :
**elle passe de bout en bout** — démarrage en 226 ms, `describe` avec
l'avertissement, les trois fichiers listés, `grep_files` et le balayage de
`search_code` trouvent le mot, `index` lance l'indexation en fond,
`index_state` rend `busy` puis `File.text = ready` et
`File.vectors = never`, `search_code` trouve le mot par l'index, arrêt
propre. Elle reste derrière sa porte parce que le vert ne prouve pas le bon
chemin : sur trois fichiers sans bibliothèque, le mock n'a rien eu à
embarquer ; avec une `Library`, il écrirait. Deux accrocs de l'épreuve
elle-même en passant : `search_code` exige `options` (vide, c'est
`SearchOptions`) ; et pendant l'indexation, `index_state` rend
`{"busy": true}` — il porte maintenant les `warnings` aussi dans ce cas.

## Ce qui vient ensuite

0. La fusion de `paquet-npm` dans master attend l'embarqueur absent de
   l'arbre principal ; la porte de `PreparedBackend::open` le pose alors à
   la place du mock, et `npm test` sort de son rouge attendu.
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
