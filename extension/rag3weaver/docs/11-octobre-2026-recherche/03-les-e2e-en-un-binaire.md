# Les e2e : un binaire, un dossier par suite, un plafond

*11 octobre 2026, session recherche, pour Lucie. Sa demande au réveil :
« s'assurer que les e2e ne font pas chacun à leur sauce et n'explosent pas le
disque » — et, précisée : « bâtis une seule fois avec le même cœur, pas
instanciés 50 fois ». Tout est mesuré, rien n'est estimé. Branche
`e2e-binaire-unique`, relecture de `run_e2e.sh` par l'arbre principal.*

## 1. Les chiffres

Mesurés sur luciepc, mêmes conditions des deux côtés : target vide, moteur
partagé (`librag3db.so`, 25 Mo, chargé à l'exécution, jamais copié), les
features de `run_e2e.sh`, `scripts/mesure_e2e.sh` (dans le dépôt, à rejouer).

| | avant : 97 binaires | après : 1 binaire | écart |
|---|---|---|---|
| poids du target | 23 Go | **5,5 Go**, puis **4,4 Go** sans la staticlib (§1 bis) | −81 % |
| binaires e2e | 97 × 184 Mo = 17,9 Go | **1 × 358 Mo** | −98 % |
| bâti à froid | 312 s | 292 s | −6 % |
| rebâti après retouche d'un fichier de `src/` | 43 s, 98 éditions de liens | **25 s**, 2 éditions | −42 % |
| batterie courte (6 suites, 42 tests) | 89 s | 89 s | identique |

**Les « 100 Go pour un bâti » ne venaient pas d'un bâti.** Un bâti propre,
même avant la refonte, faisait 23 Go. Les 100 à 307 Go (vus le 10 octobre)
venaient de deux multiplicateurs : le moteur C++ **lié en statique** dans chaque
binaire (1,4 Go par binaire au lieu de 184 Mo), et l'**accumulation** — un jeu
de features différent ou un rebase change l'empreinte de chaque binaire, et
cargo n'efface jamais les anciens. `run_e2e.sh` lie déjà le moteur en partagé ;
le binaire unique retire le second multiplicateur à la source (1 empreinte au
lieu de 97), et le plafond l'arrête s'il revient.

## 1 bis. Ce que pèse un target propre, ventilé (question de Lucie)

« Purement le debug, ou d'autres trucs ? » — **d'autres trucs, surtout**.
Mesuré sur un target neuf du binaire unique (luciepc, `du` par genre de
fichier, `readelf` par section).

| part | poids | ce que c'est |
|---|---|---|
| `.rlib` + `.rmeta` de 591 crates | **3,2 Go** | le code compilé des dépendances (opt-level 3, **sans** débogage depuis le 10 octobre) et de rag3weaver ; les plus grosses : rag3weaver 417 Mo, burn_dispatch 227, ash 140, pliron_spirv 121, tracel_ash 119, cubecl_ir 73, ld_lucivy 59, burn_nn 58, burn_tensor 56, tracel_rspirv 45 |
| ~~`librag3weaver.a`~~ | ~~1,1 Go~~ | **retirée** : une `staticlib` déclarée au `crate-type`, que seul le bâti WASM emploie ; chaque bâti natif la reliait pour rien → **5,5 → 4,4 Go** (`f4e306e3a`) |
| exécutables | 1,1 Go | le binaire e2e (358 Mo), et les trois binaires que les tests lancent : backend (336), service d'embarquement (245), démon (88) — cargo les bâtit pour tout test d'intégration |
| proc-macros (`.so`) | 0,2 Go | serde, tokio, etc. ; déjà sans débogage (build-override) |
| `build/` | 0,14 Go | les objets C/C++ des build scripts (tree-sitter, QuickJS, sqlite, ring) |

Le binaire e2e par sections : code 141 Mo, **symboles 122 Mo**, débogage (DWARF)
42 Mo, déroulement 26 Mo, données 25 Mo. **Le débogage n'est plus la cause** :
il a été coupé pour les dépendances le 10 octobre, et le profil `test` est déjà
en `line-tables-only` (essayé en plus sur le profil `dev` : **aucun gain**, 4,4 Go
au mètre près — les e2e se bâtissent sous le profil `test`).

**Ce qui se gagne encore, et ce que ça coûte** :

- **gratuit, fait** : la `staticlib` (−1,1 Go, et une édition de liens de tout le
  graphe en moins à chaque bâti).
- **à Lucie, déconseillé** : retirer les symboles (`strip = "symbols"`) —
  −0,35 Go sur les quatre exécutables (122 + 115 + 88 + 22 Mo), mais une trace
  de panique n'aurait plus un seul nom de fonction. C'est la « trace muette ».
- **à Lucie, structurel** : ne plus bâtir backend, service et démon pour chaque
  bâti de tests (−0,67 Go) demande de sortir ces binaires dans leur propre
  paquet ; et les tests en lancent deux sur trois. Pas rentable.
- **non proposé** : baisser l'optimisation des dépendances — le code rétrécirait
  un peu, mais burn, cubecl et la tokenisation retourneraient en -O0 dans tous
  les bancs (la raison écrite au `Cargo.toml`, 6 septembre).

Un target propre fait donc **4,4 Go**, presque tout du code. Le plafond de
20 Go laisse quatre fois cette taille avant de refuser.

## 2. Ce qui faisait « à sa sauce », et ce que ça coûtait

Relevé du code (un agent sur les 98 suites) et des restes mesurés sur les deux
postes. **Aucun harnais commun** : chaque suite choisissait où écrire et si
elle nettoyait.

| où | qui | restes trouvés | pourquoi |
|---|---|---|---|
| `/tmp` du poste principal (**de la RAM**) | e2e de rag3weaver + tests gtest du moteur | **11 Go** | voir les lignes suivantes |
| `/tmp/rag3weaver-bulk-<pid>` | `e2e_code` | 123 dossiers, ~1 Go | `remove_dir_all` appelé sur une **base**, qui est un fichier : l'appel échoue, `let _` l'avale, rien n'est jamais effacé |
| `/tmp/rag3weaver-{prise,rouvrir,repli,rag3daemon,modeles}-*` | `e2e_prise_atomique`, `e2e_rouvrir`, `e2e_repli_de_branche`, `e2e_rag3daemon` | dizaines | le même `remove_dir_all` sur un fichier |
| `/tmp/rag3weaver_cache` | **tout `Catalog`** (`cache_base` par défaut) | 2,7 Go | jamais nettoyé, nulle part |
| `/tmp/rag3db` | tests gtest du **moteur** | 5,5 Go, 6 500 entrées | TearDown sauté par un arrêt brutal ; ticket ouvert par le cœur C++ |
| `/tmp` de luciepc (disque) | les mêmes | **10,9 Go** | idem |
| `~/.cache/rag3weaver-build/code-sync-*` | `e2e_code_sync` | 66 dossiers, ~19 Mo chacun | `remove_dir_all` sur une base |
| `~/.cache/rag3weaver-build/tx-par-paquet-arret/` | `e2e_tx_par_paquet_arret` | 741 Mo | effacé au succès seulement |
| `~/.cache/rag3weaver-build/arret-brutal/` | `e2e_arret_brutal` | 152 Mo | **gardé exprès** (« base examinable ») |
| `~/.cache/rag3weaver/checkpoints` | **tout `Catalog`**, même en mémoire | 955 Mo, 46 entrées | aucun test ne pose `checkpoint_dir` |

Les restes de `/tmp` (8,6 Go de RAM ici, 10,9 Go de disque sur luciepc) sont
**effacés** — seulement ce qu'aucun processus vivant n'utilisait (vérifié par
`/proc/*/cwd`, `fd` et `maps`, jamais par nom).

## 3. La forme, tranchée

**Un binaire de test pour les 97 suites, lancé une fois par suite.**

- `tests/e2e/main.rs` monte chaque `tests/e2e_*.rs` comme module par
  `#[path]` : les fichiers ne bougent pas (leurs `include_str!` relatifs, leur
  historique, les branches en cours des autres sessions non plus). Il est
  **généré** par `tests/e2e/generer.py`. Seule édition dans les suites :
  `mod common;` devient `use crate::common;`.
- `Cargo.toml` : `autotests = false`, la cible `e2e` et les 14 autres tests
  (sondes, rendus, client LLM) déclarés — ceux-là restent un binaire chacun.
- `run_e2e.sh` bâtit une fois, puis lance le binaire **une fois par suite**,
  filtré sur `<suite>::`. **Chaque suite garde son processus** : ses
  `set_var`, ses `OnceLock` (le runtime du crate, les embarqueurs), ses
  statics, son démon — l'isolation d'avant, intacte. Le gain est au bâti et à
  l'édition de liens, pas à l'exécution (la batterie mesurée le montre :
  identique). Lancé à la main sans filtre (`cargo test --test e2e`), le binaire
  jouerait toutes les suites dans un seul processus : c'est dit en tête du
  fichier.
- **Filets** : une suite `tests/e2e_*.rs` non montée ne serait plus bâtie du
  tout (`autotests = false`) — `run_e2e.sh` refuse alors de partir en la
  nommant, et le test `chaque_suite_est_montee` le vérifie aussi.

## 4. La garde du disque

- **Un dossier par suite, sous un parent par passe.** `TMPDIR` pointe sur
  `/tmp/rag3weaver-e2e-<pid>/<suite>/` pendant la suite : `temp_dir()`,
  `tempfile` et le `cache_base` des catalogues le respectent tous, donc toutes
  les écritures dans `/tmp` d'une suite tombent dans son dossier **sans une
  ligne de test changée** — et les `remove_dir_all` qui échouaient ne laissent
  plus rien. Le dossier part à la fin de la suite ; le parent à la sortie de la
  passe, même interrompue (piège de sortie) ; les parents de passes mortes au
  départ de la suivante. `RAG3WEAVER_GARDER=1` garde tout pour l'examen.
  Le système de fichiers ne change pas (`/tmp` reste `/tmp`) : aucune mesure ne
  change de disque sous elle.
- **Un plafond du target** : `run_e2e.sh` refuse de partir au-delà de
  `RAG3WEAVER_TARGET_MAX_GO` — **20 Go proposés** (un bâti propre en pèse 5,5 ;
  deux jeux de features et les autres tests tiennent dessous ; au-delà, c'est de
  l'accumulation). **Accepté** (orchestration, renversable par Lucie).
- **La référence commitée** : `tests/e2e/poids-reference.txt` (binaire 358 Mo,
  target 4,4 Go). Chaque bâti s'y compare et prévient au-delà de 1,5 fois.
- **Les vieux binaires par suite** (`e2e_<suite>-<empreinte>`) ne sont plus
  produits : le balayage de sortie les efface tous, au lieu d'en garder un par
  suite (~26 Go morts sinon).

## 5. Ce qui reste, et à qui

- **Les écritures sous `~/.cache/rag3weaver-build/`** (sept suites, des
  témoins d'arrêt brutal et de transaction : elles veulent un vrai disque, pas
  la RAM) ne passent pas par `TMPDIR`. Étape suivante proposée : un
  `common::dossier_de_suite()` qui lit un `RAG3WEAVER_E2E_DOSSIER` posé par
  `run_e2e.sh` (même parent, même effacement, même `GARDER`), et les sept
  aides privées qui s'y rangent. Ce sont des suites du cœur et du banc :
  à faire avec eux, pas par-dessus.
- **Le `remove_dir_all` sur une base** : sans effet en `/tmp` désormais (le
  parent part), mais encore faux dans `e2e_code_sync`, `e2e_mesure_premier_index`
  et `sonde_sens_des_relations` (sous `~/.cache`). Le correctif tombe avec
  l'étape précédente.
- **`~/.cache/rag3weaver/checkpoints`** : tout `Catalog` y déverse ses points
  de reprise, tests compris. Un `checkpoint_dir` posé par le harnais (ou
  dérivé de `TMPDIR` pour les tests) le fermerait.
- **Les tests gtest du moteur** (`/tmp/rag3db`) : ticket du cœur C++, un
  parent par passe effacé par un `Environment` gtest.

## 6. Ce que ça change pour le verrou du poste

Un seul target pour les e2e, partagé par les sessions (la demande de Lucie :
« le même pour tous ») : **cargo sérialise lui-même** les accès à un target
(son verrou de fichier, « Blocking waiting for file lock on build directory »).
Deux passes qui partagent le target ne se corrompent donc pas : la seconde
attend la fin du **bâti** de la première, puis bâtit ce qui a changé (souvent
rien) et joue ses suites pendant que l'autre joue les siennes. Le verrou du
poste (`poste lourd`, partagé) reste ce qu'il est : il protège la **machine**
(mémoire, cœurs, l'écran de Lucie), cargo protège le **target**. Une passe de
mesure (`poste mesure`, exclusif) reste seule sur la machine. Ce qui change :
le bâti d'une passe coûte une édition de liens au lieu de 97, donc une passe
qui attend derrière une autre attend beaucoup moins. Deux worktrees sur des
commits différents qui partagent un target se rebâtiront l'un l'autre le
binaire (une empreinte par chemin de paquet) : un target par worktree reste
la règle tant que les sessions travaillent sur des branches.

## 7. La relecture de l'arbre principal, et la contre-épreuve

rag3db-73 a trouvé le défaut que mes passes de preuve ne pouvaient pas voir :
**quatorze suites se relancent elles-mêmes** (`current_exe()`, `--exact <test>`
— arrêts brutaux, deux processus, rôles d'enfant). Dans le binaire unique, le
nom court ne désigne plus rien. Corrigé par `common::test_de_cette_suite!`
(le nom complet par `module_path!()`, 22 sites), avec un filet qui refuse le nom
littéral, et les filets du binaire qui jouent désormais à chaque passe.

**La contre-épreuve** (la forme d'avant, dans le binaire unique) : les fils
jouent 0 test, `e2e_tx_par_paquet_arret` tombe — **5 passés, 13 échoués** —, la
passe rend 1. Le défaut était donc rouge, pas un vert vide ; avec le correctif,
**18 passés**, et les quinze suites concernées font 73 tests verts. Elle a aussi
montré que le résumé appariait suites et résultats **par le rang** : la ligne
d'un fils décalait tout le tableau. Il prend maintenant, pour chaque suite, la
dernière ligne avant la suivante — celle du parent.
