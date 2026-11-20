# Le juge des déclarations courtes sans vecteur

20 novembre 2026, de 12 h 03 à 12 h 32, à luciepc, à la demande d'ED et au mot de c7. Le lot d'ED
`variable-not-candidate-4` (`c3105ed454`, règles d'analyse à 12) applique la règle tranchée par Lucie à 11 h 46. Une
**déclaration de valeur** (`variable`, `constant`, `type_alias` ; un champ est une `variable`) de moins de 80 caractères
perd son morceau, donc son vecteur, mais garde son document de plein texte. Les fonctions courtes gardent leur vecteur.
Le juge décide le dépôt d'ED et la re-bascule des deux bases de luciepc.

**Verdict : rouge.** Le lot tient le critère de G contre master, mais il casse la recherche exacte des noms courts.

## Le dispositif

- **La copie** : celle de G, `mesures/code-principal-migree-repassee-20261120/` (code-principal, migrée en Versions puis
  en Generations, repassée à 7 h 21 sous les règles 9). Une copie reflink par côté ; l'original n'est pas ouvert.
- **Les deux côtés** : master à la base de fusion du lot (`a531128c20`) et le lot (`c3105ed454`). Chacun a son worktree
  sous `~/git_workspaces/banc/20-11-2026-feuille-*`, son binaire sur la lib 4ad394f4d2 et son hôte jetable (7991, 7992),
  arrêté par son pid.
- **La source** : la racine du manifeste, l'arbre de Lucie à luciepc (`284cc00c0b`), en lecture seule.
- **Le jeu, par côté** : l'index de la source, avec son journal entier ; les 10 requêtes de c7 (`search_code`,
  `mode: indexed`, limite 10, clé `chemin:ligne` lue par `banc_recherche.cles` de G) ; les usages de 4 noms ; les quatre
  jumeaux de mtga ; 10 variables courtes en minuscules et 10 constantes courtes en majuscules, tirées de l'arbre par le
  grep d'ED (graine 20) et cherchées par leur nom ; trois fonctions d'une ligne cherchées par leur sens ; mes 30 cas du
  jeu commun par l'hôte.
- **Le jugement** : `hostcall.py comparer` de G, contre sa référence des règles 8 (la base vivante) pour chaque côté,
  puis le lot contre master.
- **Les sorties** : `~/.cache/rag3weaver-build/banc-parite/sorties/feuille-not-candidate-20261120/` à luciepc. Les outils
  sont `feuille-juge.sh` et `var_juge.py`, à côté.

## 1. Le lot contre master : le critère de G est tenu

| requête | top 10 commun | même premier |
|---|---|---|
| commit | 9/10 | oui |
| index du plein texte | 6/10 | oui |
| point de reprise | 9/10 | oui |
| mutex, chunk_uuid, les liens par COPY, test e2e, acquireLock, FileSource | 10/10 | oui |
| fermer un backend | 9/10 | oui |

Recouvrement moyen **93 %**, même premier **10/10**, usages **0 perdu** (acquireLock 5, FileSource 99, lockRowForWrite
3, Catalog 369, des deux côtés). Le journal d'index compte 8 801 conteneurs écartés pour le lot, contre 7 645 pour master.

**Les quatre jumeaux de mtga** (`s = index.read_text()`, `experiments/mtga/scripts/build_{plant,red,theme,ultimatum}…`)
sortent du top 50 de « index du plein texte ». Master les avait aux rangs 24 à 27.

## 2. Les noms courts : la recherche exacte est cassée

| nom | où | master | lot |
|---|---|---|---|
| research | experiments/mtga/scripts/build_bant_copies.py:69 | 1 | **absent** |
| clipboard | experiments/mtga/scripts/build_paradox_engine.py:98 | 5 | **absent** |
| echecs | tools/build-images/epreuve_demon.py:34 | 1 | **absent** |
| threads | benchmark/lsqb/benchmark_runner.py:54 | 1 | **absent** |
| mechanic_keys | experiments/mtga/scripts/prepare_engine_relations.py:18 | 1 | **absent** |
| bin_path | third_party/brotli/scripts/dictionary/step-02-rfc-to-bin.py:29 | 1 | **absent** |
| pool | experiments/mtga/scripts/build_demons_engine.py:18 | 3 | **absent** |
| rag3db_benchmark_tool | benchmark/benchmark_runner.py:56 | 1 | 1 |
| CONFIGS | …/optimiseur/3-octobre-2026-20h55/banc/diag_rangement.py:81 | 1 | **absent** |
| ROOT | experiments/mtga/scripts/build.py:14 | 2 | **absent** |
| RAG3DB_FILE_EXTENSIONS | scripts/migrate-rag3db-db.py:29 | 1 | **absent** |
| MTGA | experiments/mtga/scripts/prepare_deck_harness.py:13 | 3 | **absent** |
| ENTETE | extension/rag3weaver/charge.py:116 | 1 | 1 |

« Absent » veut dire hors du top 10. Le premier résultat du lot est alors sans rapport (`third_party/spdlog/…/async.h`
pour `pool`, `extension/algo/…/louvain.cpp` pour `clipboard`). Les 7 autres noms tirés (`_expected_dtypes`, `conn`,
`CONFIGS` de diag_jevk5.py, `ROOT` de test_backend_must_reopen.py, `CARTES`, `BINARY`, `TOKEN`) sont absents des deux
côtés. « index.read_text » ne trouve plus les jumeaux : master les avait aux rangs 1 à 4.

**Sur 14 noms que master trouve, le lot en perd 12.** Le lot promet que la déclaration « garde son plein texte » et reste
trouvable par son nom. Ce n'est pas ce que voit `search_code` : ma lecture est qu'il ne lit que le plein texte des
morceaux. Sans morceau, la déclaration sort des deux signaux, pas seulement du dense. ED cherche la cause.

**Mes 30 cas** : un seul premier perdu, « read_text » sans filtre, dont le premier sur master était le jumeau
`build_plant_variant.py:32`.

**Les fonctions d'une ligne** : `norm_path_theirs`, cherchée par « remplacer ladybug et lbug par kuzu dans un chemin », est
première des deux côtés. Mes requêtes pour `lines` et `secondary_indexes` étaient trop faibles : elles ne les trouvent
pas non plus sur master, et ne jugent donc rien.

## 3. Contre la référence des règles 8 : le critère ne départage plus rien

| | master (`a531128c20`) | lot (`c3105ed454`) |
|---|---|---|
| recouvrement moyen contre la base vivante (règles 8) | 71 % | 74 % |
| même premier | 7/10 | 7/10 |
| usages de Catalog | 38 perdus, 59 gagnés | 38 perdus, 59 gagnés |

**Master échoue contre la base vivante autant que le lot.** L'écart vient donc de l'analyse d'aujourd'hui, pas du lot :
le critère « contre la base vivante des règles 8 » ne départage plus deux codes d'aujourd'hui. C'est une pièce pour la
re-bascule : **son critère devra être « lot contre master », sur la même copie et la même source**, et non « contre la
base vivante ».

## La suite

- La tête corrigée d'ED, rejouée sur les mêmes worktrees, gardés pour cela.
- Pour les fonctions d'une ligne, des requêtes que master trouve d'abord, tirées et vérifiées avant le jeu.
