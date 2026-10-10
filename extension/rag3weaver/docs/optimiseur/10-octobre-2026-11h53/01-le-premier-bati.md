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

## macOS arm64 : premier essai

« Go, on peut essayer macOS aussi » (Lucie, 10 octobre, par l'orchestration).
Job `macos-arm64` sur `macos-latest` (même recette que Windows : moteur en
statique par la crate rag3db avec le clang d'Apple, Ninja par brew,
`--describe` tel quel puis avec le bac à sable fermé, journal et binaire en
artefact `bati-macos-arm64-journal`). **Premier essai, lié et vert, sans un
accroc** (38064174599) :

| | valeur |
|---|---|
| runner | macOS 26.6.2 arm64, Apple clang 21, cmake 4.4.3, Ninja par brew |
| durée du bâti | 15 min 34 s (3 tâches) |
| binaire | Mach-O arm64, 65 Mo (67 699 600 octets), non strippé |
| bibliothèques liées | libc++, libiconv, libSystem — rien d'autre |
| tel quel | le refus Landlock nommé, comme sous Windows |
| `"sandbox": {"mode": "off"}` | `--describe` entier du gabarit de code |

x64 ensuite (job `macos-15-large` ou `macos-13`, même recette).

**Deuxième essai, le sous-paquet** (38068584216) : l'extension vecteur se
bâtit par le cmake de cargo en 17 s (697 Ko), `--describe` passe, `npm test`
est rouge au `LOAD EXTENSION` : « symbol not found in flat namespace :
TableFunction::emptyTableFunc ». **La cause, en une ligne : ld64 purge
(`-dead_strip`, passé par rustc) tout ce que l'exécutable n'appelle pas
lui-même, et nos extensions se lient contre lui par dlopen
(`-undefined dynamic_lookup`)** — le binaire macOS exporte 9 836 symboles
contre 21 672 pour le binaire Linux, où lld garde ce que `--export-dynamic`
exporte. Le `-rdynamic` de build.rs (que clang traduit en
`-export_dynamic`) ne suffit pas sous ld64 à protéger les globaux de la
purge. Correctif au troisième essai (38070177768) : `RUSTFLAGS=-C
link-dead-code` (plus de `-dead_strip`) ; si le binaire grossit trop, la
variante à essayer est `-C link-args=-Wl,-export_dynamic` seule, ou une
liste de symboles exportés. Le cache du bâti se sauve désormais même quand
l'épreuve est rouge (`actions/cache/restore` + `save` en `always()`).

**Troisième essai** (38070177768) : les symboles tiennent, l'extension se
charge, le backend démarre — mais le binaire passe de 65 à 119 Mo, et
`index` refuse « tampon du moteur trop petit… ce processus en a 0,0 Gio » :
rag3weaver lisait la mémoire vive dans `/proc/meminfo`, qui n'existe pas
sous macOS (ni Windows). Correctif (390ea9510) : une lecture par système
dans `connection::total_memory` (`/proc/meminfo`, `sysctl hw.memsize`,
`GlobalMemoryStatusEx`), hors feature, sa source nommée, un test unitaire
qui exige plus de zéro sur l'hôte, et le refus qui dit « mémoire vive lue :
X Gio par … » ou « inconnue ».

**Quatrième essai** (38072561734) : **le sous-paquet macOS arm64 est vert de
bout en bout.** Cache du moteur restauré, 11 min 21 s ; `strip -x` ramène le
binaire de 119 à **81 Mo** (les globaux restent) ; le backend démarre en
178 ms, `npm test` sur le runner passe (index plein texte, dette lue,
« not available » sur le dense, arrêt propre). `rag3weaver-darwin-arm64`
s'emballe depuis l'artefact `bati-macos-arm64-journal` (`dist/darwin-arm64/`).

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
chemin ne dit pas lequel des graphes manque. Le treizième essai
(38054741151, 37 min 25 s) a nommé le fichier, et la vraie cause avec :

```
\\?\D:\a\rag3db\tools\edit.mmd : The system cannot find the path specified. (os error 3)
```

Sous Windows, `canonicalize` rend un chemin **verbatim** (`\\?\D:\…`), que
le noyau prend à la lettre : les `..` d'un graphe relatif au gabarit
(`../../tools/edit.mmd`) n'y sont plus résolus — le fichier cherché n'est
pas celui qu'on croit. Le dossier du manifeste reprend sa forme ordinaire
(`sans_prefixe_verbatim`, `D:\…`) ; sous Linux rien ne change.

Le quatorzième essai (38057374360, 48 min 24 s) a rendu le chemin ordinaire,
`D:\a\rag3db\rag3db\essai\../../tools/edit.mmd`, et montré ma faute : le
correctif « manifeste à côté du gabarit » annoncé au douzième n'avait pas
été écrit (un script arrêté avant la ligne, et un « yaml ok » sur le
fichier inchangé). Les deux causes étaient réelles, l'une masquait l'autre.
Le quinzième essai (38060705890, 47 min 42 s) les a joués : **`--describe`
passe sous Windows avec `"sandbox": {"mode": "off"}`** — le binaire charge le
manifeste du gabarit de code, ses graphes et ses outils, et rend sa
description (`capabilities`, `embeddings`, `fts: lucivy`, les onze outils).
Windows est tranché : le binaire se bâtit, se lie et démarre sur
`windows-latest`, en MSVC pur. Les accents sortent en mojibake dans le
journal du runner (console en cp1252, JSON en UTF-8) : ce n'est pas le
binaire, le paquet lit stdout en UTF-8.

Ce qui reste pour un sous-paquet `rag3weaver-win32-x64-msvc` : l'extension
vecteur bâtie sous Windows (même geste que sous Linux, à partir du bâti
cmake de cargo), un bâti plus court (47 min à froid : le cache de cargo,
`Swatinem/rust-cache`, comme lucivy), `--describe` puis l'épreuve JS entière
sur le runner, et le gabarit livré avec `"sandbox": {"mode": "off"}` sous
Windows, avec le pourquoi.
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

**Emballé à blanc et installé dans un dossier vide** (17 h 35) :
`npm pack` rend `rag3weaver-0.1.0.tgz` (25 ko, 37 fichiers : index, types,
README, 33 gabarits) et `rag3weaver-linux-x64-gnu-0.1.0.tgz` (29 Mo
compressés, 85 Mo déballés : le binaire et l'extension vecteur). Dans un
dossier vide, `npm install` des deux archives, puis `require('rag3weaver')`
trouve le binaire et l'extension dans `node_modules/rag3weaver-linux-x64-gnu/`
et les gabarits dans `node_modules/rag3weaver/templates/`.

**De bout en bout depuis le dossier vide** (17 h 48) : le binaire Linux
rebâti dans l'image Docker sur luciepc depuis la branche (dépôt ebbe6a8cc,
39 s grâce au cache de cargo monté sous `/cache`, 79 Mo strippé, glibc 2.28,
extension vecteur 684 Ko), rapatrié dans `dist/`, réemballé, installé dans
le dossier vide ; l'épreuve jouée depuis ce dossier avec
`require('rag3weaver')` : démarrage en 58 ms, avertissement dans `describe`,
trois fichiers, grep, balayage, index en fond, `File.text = ready`,
`vectors = never`, le mot trouvé par l'index, arrêt propre. C'est le geste
3 du cadrage (« installer dans un dossier vide, indexer trois fichiers,
chercher ») — vert sur la branche, encore derrière la porte du rouge attendu
pour la raison dite plus haut (le mock dans le catalogue jusqu'au rebase).

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

**L'embarqueur absent est écrit** (arbre principal, branche
`embarqueur-absent`, c98d4ff9b ; `AbsentEmbedder::new(model, dim)`, porte
le nom et la dimension du modèle attendu, n'écrit aucun vecteur, laisse la
dette se poser contre le vrai modèle, `is_absent()`). Le témoin a été joué
sur une branche d'essai locale (`paquet-npm-absent`, non poussée : paquet-npm
+ master + cette branche), porte de `PreparedBackend::open` sur
`AbsentEmbedder` pour tout `None`, `allow_mock_embedder` retiré, et
l'épreuve rendue au témoin demandé (signaux hybrides par défaut, une
bibliothèque importée) : backend ouvert sans service en 100 ms, trois
fichiers indexés en plein texte, `vectors: never (0 %)` sur File, Library,
Scope et Symbol, `search_code` trouve le mot par l'index et dit « not
available » sur le dense, arrêt propre — **aucun vecteur factice**. Le
patch de la porte attend sur disque
(`~/.cache/rag3weaver-build/paquet-npm/porte-absent.patch`) ; il se pose
sur `paquet-npm` au rebase, quand l'embarqueur est sur master (après la
batterie complète demandée par l'orchestration), et la porte « rouge
attendu » de `npm test` tombe alors.

## L'alpha à publier, et la démo

Lucie (par l'orchestration, 10 octobre en soirée) : « le but maintenant,
tester l'installation depuis npm et vérifier qu'on sait faire des choses
cool avec ». Le nom `rag3weaver` est réservé sur npm (compte
luciformresearch, étiquette `next`) ; c'est l'orchestration qui publie, avec
les OTP de Lucie.

- **Branche** : `paquet-npm` fond master et `embarqueur-absent` (c98d4ff9b)
  ; la porte de `PreparedBackend::open` pose `AbsentEmbedder` pour tout
  démarrage sans service, plus aucun mock dans le produit ; `npm test` sort
  de son rouge attendu et passe (dette lue, « not available » sur le dense).
- **Les deux paquets en `0.0.1-alpha.1`**, emballés sous
  `~/.cache/rag3weaver-build/paquet-npm/publier/` : `rag3weaver` (25 ko,
  README honnête : alpha, Linux x64 seulement, ce qui marche et ce qui
  manque) et `rag3weaver-linux-x64-gnu` (28 Mo compressés : le binaire
  Docker de luciepc, dépôt 6788935a7, 79 Mo strippé, glibc 2.28, et
  `libvector` à côté). Ordre de publication : le sous-paquet d'abord.
- **La démo** : `bindings/nodejs/demo/demo.sh [dépôt]` — depuis un dossier
  vide, `npm install rag3weaver@next` (ou `DEMO_SOURCE=<archive>` pour
  répéter avant publication), puis `demo.js` deux fois sur
  `codeparsers/src` (80 fichiers Rust, 26 700 lignes) : avec le service par
  les tunnels (`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,…`) et sans. Chaque
  passe montre `describe` (les outils en une ligne chacun, l'avertissement
  s'il y en a), `search_code` avant l'index (balayage), l'index et son état
  (texte, vecteurs, dette), une question en langue naturelle, `usages` et
  `impact` sur un symbole, avec la section Liens que le backend rend ; tout
  par le `presentation` des réponses et la section `after` (les Liens, en
  arbre), lisible. **Répétée trois fois depuis un dossier vide avec les
  archives** (`DEMO_SOURCE`) : avec le service, index de `codeparsers/src`
  en 21 s (vecteurs à 100 % sur File, Library, Scope), la question « comment
  les relations entre symboles sont-elles résolues ? » rend
  `RelationshipResolver::resolve_relationships` en tête (vector+bm25), les
  Liens en arbre (Consumes / Consumed by), `usages` et `impact` nomment
  `parse_project` ; sans service, index en 4 s, l'avertissement dans
  `describe` et `index_state`, la même question rend ses résultats BM25 avec
  « dense signal is not available: no embedding service (embarqueur absent…) ».
  **Publié** (orchestration, OTP de Lucie, 10 octobre en soirée) :
  `rag3weaver@0.0.1-alpha.1` et `rag3weaver-linux-x64-gnu@0.0.1-alpha.1`,
  étiquette `next` (npm a d'abord mis le binaire en « staged publishing »,
  placeholder 0.0.0-stage, puis la version est apparue). Première démo
  depuis npm, verte de bout en bout (install en ligne, index en 19 s avec
  le service, 4 s sans) — après un accroc : `npm install rag3weaver@next`
  prenait le packument du cache local de npm et installait l'alpha.0
  (« added 1 package in 104ms », puis MODULE_NOT_FOUND) ; la démo installe
  maintenant avec `--prefer-online`, affiche la version installée et
  s'arrête si ce n'est pas celle que `npm view rag3weaver@next version`
  rend (ou `DEMO_VERSION`).
  Deux accrocs de la démo corrigés en passant : l'attente des vecteurs
  guettait `vectors_seconds_left` que `Symbol` (BM25) laisse non nul (dix
  minutes perdues au premier essai) ; et la section Liens n'est pas dans
  `presentation` mais sous `after` — le client doit la rendre lui-même.

### L'alpha.2 : les trois plateformes

Préparé le 10 octobre au soir, rien de publié (séance d'OTP avec Lucie en
une fois, quand Windows est vert) : sous
`~/.cache/rag3weaver-build/paquet-npm-garde/publier/`, la tête
`rag3weaver-0.0.1-alpha.2.tgz` (README **en anglais**, demande de Lucie :
work in progress, l'idée, l'agent runtime, les trois plateformes,
l'embarqueur optionnel ; les trois sous-paquets épinglés `0.0.1-alpha.1`),
`rag3weaver-darwin-arm64-0.0.1-alpha.1.tgz` emballé depuis l'artefact du
quatrième essai macOS (28,4 Mo, Mach-O arm64 strippé), les deux archives
alpha.1 déjà publiées, et le sous-paquet Windows à venir du run vert. La démo
rejouée contre l'archive de tête locale (`DEMO_SOURCE`, qui n'installe plus
que la tête) prouve que les sous-paquets épinglés se résolvent depuis npm et
que les non publiés n'empêchent rien (optionnels, autre plateforme). Ordre
de publication : windows, darwin, puis la tête.

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
