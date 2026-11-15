# Rapport de session G (optimiseur), du 14 novembre au soir au 15 novembre, 1 h 40

Heures lues avec `date +%T`. Les mots de Lucie arrivent par c7.

## Ce qui a atterri sur master

| lot | master | ce qu'il fait |
|---|---|---|
| garde-pile-2 | 8565a06bf | la garde en pile (`--onto`, range-diff jugé, `revenir`), `poste` sur `nproc --all` |
| fils-du-moteur | 77b9e5f18 | `RAG3WEAVER_DB_THREADS`, le pool de fils du moteur par base, dit au `/sante` |
| garde-pile-3 | 533e5cbb3 | `PILE_TESTE` : une tête de pile reposée à la main, sans rebase par la garde |
| check-rel-ancre | a34fe130a | `check_rel_directions` lit ses bouts par tranches ancrées, avec quatre comptes par table |
| chaîne Rust | b09ce1760 | `rust-toolchain.toml` à 1.98.1, la CI à la même version |
| garde-sous-module | ac8ef511f | la garde du target note le commit des sous-modules ; `garde_target.sh --reprendre` |
| passe-epinglee | 2146956e6 | `tools/poste/passe_epinglee.sh` entre dans le dépôt, avec son témoin |
| pg-base-par-passe | 552299bec | une base PostgreSQL par processus de suite, effacée à la sortie |
| passe-ep-igpu | cf3fdd6a6 | `RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=igpu:0` à luciepc seulement |
| garde-tickets | 2fb7fdcec | la copie ouverte d'un ticket que le lot ferme est retirée dans le même push |

En file, derrière codeparsers : **garde-deja-sur-master** b3a540346. Un « < » du range-diff dont le patch est déjà sur
master y est toléré ; c7 donnera sa base.

## Les poses

- **La lib 373b5477b**, le 14 à 20 h 52, aux deux postes. `courant` et lib-des-passes y sont depuis 21 h 33. Le
  contrôle des deux sens a trouvé des demi-arêtes sur code-lourd et code-principal ; hotfixs les a réparées dans sa
  fenêtre (ticket fermé). Page 04 du 14 novembre.
- **Le binaire fac67a208 sur 373b5477b**, avec la fermeture qui tient, posé à luciepc à 0 h 25 et au portable à 1 h 15.
  Verdict : e2e_code 36/36, e2e_lectures 27/27, e2e_demon_hote 14/14, sous `RAG3WEAVER_MOTEUR_ANCIEN=1` dit.
- **Non retenu : MALLOC_ARENA_MAX=2.** Mesuré sur l'hôte réel, il gagne moins d'1 Gio et rend les appels 70 à 77 % plus
  lents (page 03 du 14 novembre).

## Reprendre

**Worktrees du portable** (`~/git_workspaces/`), tous propres, aucun commit en local :
- `rag3db-garde-dsm` : `garde-deja-sur-master` b3a540346, **en file**. Son journal est
  `~/.cache/rag3weaver-build/rejeu-garde-deja-sur-master/` (resume.log VERT), son push `wt-…` est à écrire avec le
  `PILE_AVANT` que c7 donnera.
- `rag3db-docs-G` : détaché sur master, il sert à pousser les docs.
- Lots atterris, gardés jusqu'au mot de Lucie : `rag3db-cpu-G`, `rag3db-garde-pile-2`, `rag3db-garde-pile-3`,
  `rag3db-check-rel`, `rag3db-garde-sm`, `rag3db-passe-ep`, `rag3db-pg-base`, `rag3db-garde-tickets`, `rag3db-fils-pile`.
- Plus anciens : `rag3db-plafond-G`, `rag3db-poste-G`.

**Targets** : `~/.cache/rag3weaver-build/target-rust-G` (13 Go, sous 1.98.1, noté par `rag3db-pg-base`) et `target-fils-G`
(23 Go) au portable ; `target-G/passe` (27 Go) et `target-G/pile` (9,5 Go) à luciepc.

**Worktrees de luciepc** : `rag3db-controle-G`, détaché sur fac67a208, sert aux verdicts et au bâti des binaires (jamais
les deux à la fois) ; `rag3db-fils-pile-G`, détaché sur 270b6284d, sert aux preuves.

**Binaires posés, aux deux postes** : `~/.local/bin/rag3weaver-backend` md5 158a306a (fac67a208 sur 373b5477b),
check-lib eaa1d460.
- Au portable, les anciens sont `.avant-fac67a20` (4b43c3afc, 03ab44a0) et `.avant-373b5477b` (012be4a58, 547a8f56).
- À luciepc, mêmes étiquettes.
- L'env et le filet d'avant sont dans `~/.cache/rag3weaver-build/sauvegardes-de-pose/`.

**Copies du cache, aux deux postes** (`~/.cache/rag3weaver-build/`, sha256, toutes recopiées depuis master) :

| script | sha256 | copies d'avant gardées |
|---|---|---|
| `poste` | fc78c8fbd56c43b2 | `.avant-nproc-all` et les plus anciennes |
| `push_garde.sh` | cb7331214b34d9c3 | `.avant-garde-tickets`, `.avant-garde-pile-3`, `.avant-garde-pile-2`… |
| `passe_epinglee.sh` | be664721bbceaff7 | `.avant-igpu`, `.avant-depot` |
| `controle-du-worktree.sh` | 94d5c09b6c85b737 | `.avant-cherry-pick` |

**Instantanés** : `avant-373b5477b` aux deux postes (avec `RETOUR.sh`), et ceux de la fenêtre d'hotfixs
(`avant-fenetre-<base>-20261115-*`).

**Mesures à luciepc** : `~/.cache/optimiseur-mesure/` (38 Go apparents, surtout des copies reflink des bases) contient
`arena/`, `pg-rouge`, `pg-rouge2`, `pg-vert`, `jetable/` et `un-seul-sens-copie/`. Rien ne s'efface avant le mot de Lucie.

## Ce qui reste

- **Le plafond des targets** (ticket d'archi du 15) : archi fournit un second compte au prochain plafond. Le premier
  donne 11 Go, dont 4,0 en `incremental/`.
- **La mesure du cas filtré**, sur une base à versions, avant et après `vue-publiee` (ticket d'ED) : fac67a208 contre un
  binaire ≥ e89cdb8e6.
- **lib_seule.sh et rag3db_libs.py** dans `tools/poste/`, un lot chacun (ticket du contrôle du worktree).
- **La base un-seul-sens du cœur** à ajouter au témoin de `check_rel_directions`, comme cas exact des comptes.
