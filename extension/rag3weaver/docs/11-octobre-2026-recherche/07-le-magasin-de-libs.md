# Le magasin de libs par identifiant de bâti

*11 octobre 2026, session recherche, à la demande de l'orchestration. L'enquête
et la forme proposée, pour G, qui l'écrira. Ce soir, deux rebâtis en place de
`build/lecteurs-csv` ont arrêté toutes les sessions : l'une pour libérer la
lib, et l'autre parce qu'une lib sans `fileLockWaitMs` a cassé le bâti Rust
de quiconque avait rebasé sur B2.*

## 1. Ce qui existe, relevé ce soir

- **Un seul dossier partagé** : `build/lecteurs-csv` (180 Mo sur disque, dont
  `librag3db.so` 26 Mo et l'en-tête unique `rag3db.hpp` 329 Ko). Les passes e2e,
  les hôtes, les clients MCP et les scripts Python l'emploient. `run_e2e.sh
  --build` le rebâtit **en place**, sous le verrou du poste, mais un
  `cmake --build` à la main et les processus déjà lancés n'y passent pas.
- **L'extension vector est rebâtie au même chemin, quel que soit le dossier de
  bâti** : `extension/CMakeLists.txt:14` l'écrit dans
  `extension/vector/build/libvector.rag3db_extension` (0,9 Mo), dans l'arbre des
  sources. Tout dossier `build/*` qui bâtit vector écrase la même.
- **Un binaire ne garde pas le chemin de sa lib.** `tools/rust_api/build.rs:220`
  émet `-Wl,-rpath,<lib>` par `cargo:rustc-link-arg`, qui ne vaut que pour les
  cibles de la crate `rag3db` elle-même. Les binaires de rag3weaver (tests e2e,
  `rag3weaver-backend` du target et de `~/.local/bin`) **de ce poste** n'ont ni
  RPATH ni RUNPATH (`readelf -d`) : la lib chargée s'y décide au lancement, par
  `LD_LIBRARY_PATH` seul. **Sur luciepc, les binaires de G en portent un**
  (`RUNPATH [~/git_workspaces/rag3db-lourd/build/lecteurs-csv/src]`, vérifié
  le soir même par ssh). Il vient de sa recette de bâti
  (`~/.cache/rag3weaver-build/hote-bati3.sh`, page
  `docs/optimiseur/11-octobre-2026-16h40/01-l-hote-sur-luciepc.md`) :
  `RUSTFLAGS="-C link-args=-Wl,-rpath,$L"` avec
  `L=~/git_workspaces/rag3db-lourd/build/lecteurs-csv/src`. L'unité systemd
  qui lance l'hôte ne pose pas de `LD_LIBRARY_PATH` : le RUNPATH est le seul
  chemin. Il vise le dossier rebâti en place, et un rebâti y change donc la
  lib sous l'hôte au prochain redémarrage. **Le paquet npm lie le moteur en
  statique** : il n'a ni RUNPATH ni lib partagée, et le magasin ne le concerne
  pas.
- **Les en-têtes compilés ne sont pas liés à la lib chargée.** Le pont cxx
  inclut `rag3db.hpp` depuis `RAG3DB_INCLUDE_DIR` au bâti. `build.rs` ne
  surveille ni `RAG3DB_LIBRARY_DIR`, ni `RAG3DB_INCLUDE_DIR`, ni l'en-tête
  (pas de `rerun-if-env-changed`, pas de `rerun-if-changed` sur `rag3db.hpp`).
  Un en-tête remplacé en place ne fait pas recompiler le pont. C'est l'incident
  de ce soir, dans les deux sens : un pont neuf contre une lib vieille (champ
  absent), et le ticket ouvert d'un binaire lié à une autre lib qui échoue sur
  un symbole manquant au lieu d'un refus nommé.
- **L'identifiant de bâti existe déjà, et il est bon.**
  - `cmake/build_id.cmake` écrit `RAG3DB_BUILD_ID` =
    `rag3db-build-<commit>[-dirty-<sha12>]-<options d'ABI>`, où le commit est
    le dernier qui touche le moteur ;
  - `CALL build_id()` le rend (`48b74ead1`) ;
  - l'extension exporte le sien, et le moteur refuse nommément une extension
    d'un autre bâti (`checkBuildId`, `458ff7157`) ;
  - rag3weaver le lit au démarrage du démon, mais seulement pour `/sante` :
    rien ne le compare au bâti contre lequel le binaire a été compilé.
- **Un processus long garde la lib qu'il a mappée.** Un rebâti en place ne le
  touche pas, jusqu'à son redémarrage. En revanche, un `LOAD EXTENSION` tardif
  lit l'extension neuve, que le contrôle d'identifiant refuse.

## 2. La forme proposée

**Un dossier par identifiant de bâti, immuable, et une seule chose qui
bouge : un lien `courant`.**

```
~/.cache/rag3db-libs/
  rag3db-build-<commit>-Release-…/   ← immuable une fois publié
    librag3db.so
    rag3db.hpp
    libvector.rag3db_extension
    libgeo.rag3db_extension
    BUILD_ID                          ← la chaîne, relue par CALL build_id()
    MANIFESTE                         ← commit, date, md5 de chaque fichier
  courant -> rag3db-build-…            ← changé par un rename atomique
```

- **Publier** : un rebâti (`run_e2e.sh --build`, ou un script de G) bâtit où il
  veut, puis copie les quatre fichiers dans `…/<id>.tmp-<pid>/`, vérifie que
  `CALL build_id()` sur la lib copiée et le `build_id` de l'extension copiée
  rendent tous deux `<id>`, puis fait `rename` vers `…/<id>/`, et enfin bascule
  `courant` (`ln -sfn` dans un temporaire, puis `rename`). Si `<id>` existe
  déjà, rien n'est republié : le même identifiant donne le même binaire.
- **Consommer** : une passe, un hôte ou un client **résout `courant` une fois,
  au départ**, puis pose `RAG3DB_LIBRARY_DIR`, `RAG3DB_INCLUDE_DIR`,
  `LD_LIBRARY_PATH` et le chemin de l'extension sur le dossier **résolu**
  (`…/<id>/`), jamais sur `courant`. Toute la passe tient alors un seul bâti,
  même si un autre est publié pendant qu'elle tourne. `moteur_inchange` devient
  vrai par construction.
- **Plus personne ne s'arrête pour un rebâti.** L'ancien dossier reste ; les
  processus qui le tiennent continuent ; les nouveaux prennent le nouveau.
  Le « dis-moi fini » de ce soir disparaît.

### 2.1 Tranché avec G (12 octobre, avant le lot 3)

- **Le magasin ne sert qu'aux postes de développement Linux** (ici et
  luciepc). La portabilité (Windows, macOS) passe par le paquet npm, qui lie
  le moteur en statique : ni lib partagée ni RUNPATH. Un RUNPATH absolu vers
  le dossier d'un identifiant convient ; `$ORIGIN` n'apporterait rien (le
  binaire est dans `~/.local/bin`, le magasin ailleurs).
- **L'emplacement est un cache** : tout s'y rebâtit.
  `${XDG_CACHE_HOME:-~/.cache}/rag3db-libs`, ou `RAG3DB_LIBS_DIR` pour les
  témoins (sur le modèle de `POSTE_LOCK_DIR`).
- **Un binaire installé et son unité suivent un identifiant fixe**, choisi au
  bâti du binaire, jamais `courant` : c'est exactement la panne de 20 h 39.
  L'`ExecStartPre` (`rag3db-check-lib`) compare le binaire à la lib de son
  RUNPATH. `courant` reste aux passes, résolu une fois au départ.
- **Le ménage épargne aussi les identifiants référencés**, pas seulement ceux
  qui sont mappés : un binaire de `~/.local/bin`, ou le binaire d'`ExecStart`
  d'une unité utilisateur active ou activée, qui vise un dossier par RUNPATH,
  le garde, même arrêté. Sinon un hôte arrêté proprement ne redémarre plus
  après un ménage (le binaire prêt de luciepc, posé à côté sans être lancé,
  est dans ce cas).
- **L'extension vector vit dans le dossier de l'identifiant**, et
  `vector_extension_path` la cherche d'abord à côté de la lib
  (`RAG3DB_LIBRARY_DIR`), puis comme aujourd'hui. Un dossier de bâti sans
  extension (`build/lecteurs-csv/src`) retombe donc sur l'ordre d'avant : rien
  ne change pour qui n'emploie pas le magasin.
- **Les consommateurs** : `run_e2e.sh` et les e2e cargo (`LD_LIBRARY_PATH`,
  `RAG3DB_LIBRARY_DIR`), les scripts Python qui lancent le binaire, le client
  MCP (le même binaire, qui lance l'hôte : les deux doivent avoir le même
  identifiant, ce que `/sante` dit déjà), et `rag3db-check-lib`.
- **Le dossier d'un identifiant porte aussi `include/common/build_id.h`** :
  c'est là que le `build.rs` de rag3weaver lit l'identifiant compilé.

## 3. Ce qu'il faut changer pour que ce soit vrai

1. **`tools/rust_api/build.rs`** : `cargo:rerun-if-env-changed` sur
   `RAG3DB_LIBRARY_DIR` et `RAG3DB_INCLUDE_DIR`, et
   `cargo:rerun-if-changed=<include>/rag3db.hpp`. Sous le magasin, un chemin
   neuf veut dire un contenu neuf, et le pont se recompile. Sans le magasin,
   c'est déjà un correctif : il aurait évité la moitié de l'incident de ce soir.
2. **Le binaire dit contre quel bâti il a été compilé.** `build.rs` lit
   `RAG3DB_BUILD_ID` dans l'en-tête (ou dans `BUILD_ID`) et l'émet par
   `cargo:rustc-env=RAG3DB_COMPILED_AGAINST=…`. Au premier `Database::new`, la
   crate compare à `CALL build_id()`. Un écart est un **refus nommé** (« ce
   binaire a été compilé contre X, la lib chargée est Y »), et non un symbole
   manquant. C'est la fermeture du ticket ouvert. Le démon le dit déjà dans
   `/sante` ; il le comparera.
3. **L'extension se cherche d'abord à côté de la lib.**
   `vector_extension_path` (`backend.rs:1378`) essaie aujourd'hui
   `RAG3DB_ROOT/extension/…/build` avant `RAG3DB_LIBRARY_DIR`. Sous le
   magasin, il faut l'ordre inverse, pour que la paire lib et extension vienne
   du même dossier. Le contrôle d'identifiant refuse déjà l'autre cas, mais il
   vaut mieux ne pas le provoquer.
4. **Un rpath, ou pas.** Deux choix :
   - (a) poser le rpath sur les binaires de rag3weaver eux-mêmes (un
     `build.rs` de rag3weaver qui émet `-Wl,-rpath,<lib résolue>`). Le binaire
     garde alors le chemin de son bâti, et `LD_LIBRARY_PATH` ne sert plus que
     pour forcer un autre bâti ;
   - (b) garder `LD_LIBRARY_PATH`, posé par les scripts sur le dossier résolu.
   Je proposerais (a) : un binaire installé dans `~/.local/bin` trouverait seul
   la lib de son bâti, et le contrôle du point 2 dirait l'écart si on le force
   ailleurs. Comme l'ancien dossier reste au magasin, son rpath reste valable.
   Sur luciepc, (a) existe déjà sous la forme du RUNPATH de G : avec le magasin,
   il devra viser `…/rag3db-libs/<id>/`, jamais `courant` ni le dossier de bâti.
   Ajouter un rpath aux bâtis de ce poste est un changement de conception, à
   décider par l'orchestration.
5. **`templates/mcp/*` et les scripts** qui écrivent
   `build/lecteurs-csv/src` en dur : ils passent à `~/.cache/rag3db-libs/courant`,
   résolu à l'installation. Avec (a), ils n'ont plus de variable à poser.

## 4. Ce que ça coûte

- **Disque** : environ **27 Mo par identifiant** (lib 26 Mo, extension
  0,9 Mo, en-tête 0,3 Mo ; geo en plus si bâtie). Dix identifiants gardés font
  270 Mo. C'est négligeable devant un target cargo (4 à 20 Go) et devant
  `build/tests-wal` (495 Mo).
- **Le ménage** : on garde les N derniers (proposé : 10), plus tout identifiant
  encore tenu. Un dossier est tenu si un processus vivant le mappe (lu dans
  `/proc/*/maps`, aucun verrou à poser), ou si un fichier `ÉPINGLÉ` y est posé
  (une mesure qu'on refera). Le ménage passe à la publication, jamais pendant
  une passe.
- **Le temps** : la copie de 27 Mo et les deux `CALL build_id()` prennent
  moins d'une seconde. Le bâti ne change pas.

## 5. Les pièges

- **`-dirty-<sha12>`** : un arbre modifié a un identifiant à lui. Deux
  sessions qui bâtissent chacune un arbre sale produisent deux dossiers, et
  aucune n'écrase l'autre. C'est voulu.
- **Le même identifiant, deux binaires** : c'est impossible si l'identifiant
  porte tout ce qui change l'ABI. `build_id.cmake` y met déjà le type de bâti
  et les options de page et de vecteur. Un compilateur différent n'y est pas.
  Le manifeste garde le md5, et une publication sur un identifiant existant
  dont le md5 diffère est **refusée** et dite, sans écraser.
- **luciepc** a son magasin à lui : le chemin est le même, sous `~/.cache` de
  ce poste-là. Il ne faut rien partager par le réseau.
- **`extension/vector/build`** reste où CMake l'écrit. Le magasin n'en garde
  qu'une copie : ne rien changer au CMake du moteur (rien vers l'amont) évite
  d'y toucher.
- **Linux seulement chez nous** : `check_features.sh` ne compile que sous
  Linux. Le seul contrôle de Windows et de macOS, ce sont les runners du
  paquet npm (chez G). Le 12 octobre, ils ont refusé `build_check.rs` pour du
  code mort hors Linux : `LEN` et `nul_terminated` ne servaient qu'à la
  section ELF, gardée par `#[cfg(target_os = "linux")]`, mais eux ne
  l'étaient pas (run 38197365376). G l'a corrigé dans son lot C
  (`2f052d5db`, relu). Règle : ce qui ne sert qu'à un code sous `cfg` porte
  le même `cfg`.

## 6. Les témoins, rouges d'abord

1. **Le pont se recompile** : changer `RAG3DB_INCLUDE_DIR` vers un dossier dont
   l'en-tête diffère, puis `cargo check` doit montrer
   `Compiling rag3db (tools/rust_api)`. Aujourd'hui rien ne se passe.
2. **Le refus nommé** : un binaire compilé contre le bâti X, lancé avec la lib
   du bâti Y, rend « compilé contre X, chargé Y » au lieu de
   `undefined symbol`.
3. **Une publication pendant une passe** : la passe garde son identifiant du
   départ à la fin, et `moteur_inchange` est vrai sans cksum.
4. **Le ménage** épargne un dossier mappé par un processus vivant.
5. **La paire** : sous le magasin, la lib et l'extension viennent du même
   dossier, même si `RAG3DB_ROOT` désigne un arbre dont l'extension est d'un
   autre bâti.

## 7. L'ordre proposé

1. `build.rs` : les `rerun-if` (point 3.1). Petit, utile tout de suite, et
   indépendant du reste. **Fait** : `rerun-if-env-changed` sur
   `RAG3DB_LIBRARY_DIR` et `RAG3DB_INCLUDE_DIR`, `rerun-if-changed` sur
   `rag3db.hpp` et sur la lib. Témoin `scripts/test_pont_recompile.sh` (trois
   cas : rien ne change et rien ne se recompile, un autre dossier d'en-têtes,
   l'en-tête modifié en place). Les deux derniers étaient rouges avant le
   correctif.
2. Le refus nommé binaire ↔ lib (point 3.2), qui ferme le ticket ouvert.
   **Fait**, avec une forme qui change : nos binaires sont liés en BIND_NOW, et
   un contrôle dans le processus viendrait après le symbole manquant. La lib
   porte son identifiant dans la section ELF `.rag3db_build_id` (une ligne
   dans `src/function/table/build_id.cpp`, relue par le cœur). Le binaire porte
   le sien dans `.rag3db_compiled_against`, lu par `build.rs` dans le
   `build_id.h` du bâti. `rag3db-check-lib <binaire> <lib>`, qui ne lie pas la
   lib, les compare avant tout chargement (l'`ExecStartPre` de G), et
   `build_check::check_loaded` reste le filet dans le processus. Témoins
   `build_check::tests`, deux rouges avant. Ticket fermé.
3. Le magasin, la publication et le ménage (script de G, appelé par
   `run_e2e.sh --build`). **Fait, 3a** (porté par la session recherche, pas
   G) : `scripts/rag3db_libs.py` (`publier`, `courant [--poser]`, `liste`,
   `menage [--garder N] [--simuler]`). La publication refuse une lib sans
   section, un en-tête d'un autre bâti, une extension d'un autre bâti, et un
   identifiant déjà publié avec une autre lib (rien n'est écrasé). Le dossier
   se pose par `rename`, `courant` par un lien changé atomiquement. Le ménage
   épargne les N plus récents, `courant`, `ÉPINGLÉ`, ce qui est mappé et ce
   que vise un RUNPATH installé ou une unité. Et `vector_extension_path`
   cherche d'abord à côté de la lib. Témoins : `scripts/test_rag3db_libs.py`
   (quatre cas, rouges sans l'outil) et
   `the_vector_extension_is_looked_up_next_to_the_lib_first`, rouge avec
   l'ordre d'avant.
4. Les consommateurs : `run_e2e.sh`, les modèles MCP, les scripts. Puis le
   rpath (point 3.4), une fois le magasin en place. **3b, fait pour
   `run_e2e.sh`** : `RAG3DB_LIB=courant` (ou un identifiant) prend la lib, les
   en-têtes et l'extension d'un dossier du magasin, résolu une fois au départ
   (`scripts/lib_du_magasin.sh`). Sans la variable, rien ne change.
   `--build` publie la lib neuve au magasin. Témoin
   `scripts/test_lib_du_magasin.sh`, six cas ; il a été écrit après l'outil,
   et n'a donc pas été vu rouge. **Limite dite** : une vingtaine de suites
   bâtissent le chemin de l'extension par `RAG3DB_ROOT`. Sous le magasin, la
   passe refuse donc en nommant les deux identifiants si l'extension de
   l'arbre n'est pas du bâti du magasin. **3c, fait** : les 97 sites de 82
   suites passent par `common::vector_extension()`, qui cherche d'abord à côté
   de la lib, puis sous `RAG3DB_ROOT` comme avant. Hors magasin, le chemin
   rendu est celui d'avant. Les fonctions de racine devenues inutiles ont été
   retirées. CHECK_FEATURES VERT 5/5 ; échantillon de sept suites qui couvrent
   les formes migrées (`LOAD EXTENSION` en ligne, manifeste JSON, formes sur
   plusieurs lignes) : 82 passés, seul le banc à services rouge, sans service.
   Le refus de `run_e2e.sh` reste en filet pour une suite écrite à l'ancienne.

**Prouvé de bout en bout** (12 octobre, 02 h 34 - 03 h, sur la lib rebâtie par
l'arbre principal sur 7b54c5f84) :
- la lib publiée au magasin : `rag3db-build-6ffec5a82…-Release`, la section
  lisible, `courant` posé ;
- un backend bâti contre le dossier du magasin porte la même section
  `.rag3db_compiled_against` ;
- `rag3db-check-lib` dit « même bâti » (code 0), et refuse en nommant les deux
  identifiants devant une lib d'un autre bâti (code 2) ;
- une passe réelle sous `RAG3DB_LIB=courant`, avec le lot 3c, sur six suites :
  82 passés.
Reste chez G, à son rythme : sa recette de luciepc sur le magasin, et
l'`ExecStartPre`.

## 8. Le magasin par défaut (décision de l'orchestration, 12 octobre)

**`run_e2e.sh` prend sa lib au magasin par défaut**, puisque le rebâti de
l'arbre principal publie lui-même au magasin et pose `courant` (sa recette,
`~/.cache/rag3weaver-notes-A/lib/batir.sh`, juste après un `cmake --build`
vert). L'ordre (`choisir_la_lib`, scripts/lib_du_magasin.sh) :
1. `RAG3DB_LIB` posé : `bati` (le dossier de bâti), `courant` ou un identifiant ;
2. `RAG3WEAVER_MOTEUR_DU_WORKTREE=1` (le cœur éprouve son moteur) : le dossier de bâti ;
3. le `courant` du magasin s'il existe, sinon le dossier de bâti, dit comme un repli.

`RAG3DB_BUILD` ne dit plus que où `--build` bâtit. Une extension de l'arbre
d'un autre bâti n'est plus un refus, mais une note : les suites prennent
celle du magasin depuis le lot 3c. Vérifié par une passe réelle lancée comme
les sessions la lancent (`RAG3DB_BUILD` posé, rien d'autre) : la lib du
magasin est prise par défaut, 35 passés. Témoins : `test_lib_du_magasin.sh`,
onze cas.

**Les binaires installés** portent un RUNPATH vers le dossier d'un
identifiant (fixé au bâti, jamais `courant`) : `~/.local/bin` ici, chez la
mémoire au matin, et la recette de G sur luciepc. Les tests et le target n'en
ont pas. Le refus nommé (`rag3db-check-lib`, et le filet dans le processus)
reste en place. **Reste à prouver** : une passe réelle
   sous `RAG3DB_LIB=courant`, après le rebâti du matin (la première lib qui
   porte sa section). Les modèles MCP et la recette de G suivront, avec G.
