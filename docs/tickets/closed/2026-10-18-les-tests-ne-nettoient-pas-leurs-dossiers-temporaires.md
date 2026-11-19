# Les tests lancés hors de run_e2e.sh laissent leurs bases dans /tmp, qui est de la mémoire vive

- **État** : fermé (19 novembre 2026, session recherche) : les cinq suites qui laissaient des résidus à chaque passe sont propres, et sorties de la liste. Ouvert le 18 octobre 2026 (seconde session cœur C++, à la demande de l'orchestration) ; le 8 novembre, le cache d'index avait quitté /tmp et le témoin de suite était né (session recherche).
- **Gravité** : confort, mais `/tmp` est un tmpfs sur le poste principal : ce qui y reste occupe
  la RAM, puis l'échange.
- **Atteignable en service** : non (tests seulement), à une exception près : `Catalog` écrit son
  cache par défaut dans `/tmp/rag3weaver_cache`, en service aussi.
- **Touche rag3weaver** : oui (ses e2e, ses tests et `Catalog`).
- **Famille** : produit
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

## Ce que c'est

Le 11 octobre, la garde de `run_e2e.sh` a donné à chaque suite e2e son propre `TMPDIR`, effacé
derrière elle (`extension/rag3weaver/docs/11-octobre-2026-recherche/03-les-e2e-en-un-binaire.md`,
§4). Le même jour, les tests gtest du moteur ont reçu leur parent par passe (ticket fermé
`closed/2026-10-11-les-tests-gtest-du-moteur-laissent-leurs-bases-dans-tmp.md`).

Ces gardes ne couvrent qu'un test lancé par `run_e2e.sh`. Une suite jouée à la main
(`cargo test --test e2e`, une boucle de rejeu, un script de session) écrit à la racine de `/tmp`,
et ses `remove_dir_all` échouent comme avant le 11. Le 15 octobre, l'orchestration a effacé
**755 dossiers morts** de `/tmp` (familles `rag3weaver-bulk-*`, `rag3weaver_cache`,
`/tmp/rag3db`). L'échange du poste est passé de 22 à 10 Go, avec l'arrêt de l'hôte : voir
[le ticket de la mémoire de l'hôte](2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md).

## Ce qui reste le 18 octobre (poste principal, relevé en lecture seule)

4 124 entrées à la racine de `/tmp`. Les familles de rag3weaver :

| famille | entrées | dates | poids | qui l'écrit |
|---|---|---|---|---|
| `rag3weaver-acces-<pid>-<ns>` (+ `.wal`, `.readers`) | 2 332, soit 804 bases | 11 oct. 13 h 42 → 12 oct. 14 h 45 | 6,3 Mo | `e2e_rag3daemon::un_lecteur_choisit_son_chemin_et_le_dit` |
| `rag3weaver-rag3daemon-<pid>-<ns>` (+ `.wal`) | 1 610 | même plage | 13 Mo | `e2e_rag3daemon` |
| `rag3weaver-bulk-<pid>` | 91 | 11 oct. 10 h 54 → 12 oct. 16 h 28 | 122 Mo | `e2e_code` (`tests/e2e_code.rs:738`) |
| `rag3weaver-lignes-*` | 14 | 11 → 12 oct. | 0,2 Mo | (non cherché) |
| `rag3weaver_cache` | 1 dossier | — | 16 Mo | `Catalog` (`src/catalog.rs:643`, `cache_base` par défaut) |

`/tmp/rag3db` est vide aujourd'hui : le parent par passe du moteur tient.

Les restes de `e2e_rag3daemon` viennent d'environ 800 processus distincts (deux bases par pid),
tous **après** la garde de `run_e2e.sh` (11 octobre, 12 h 27) : ils ont donc été lancés sans elle.
On ne sait pas encore qui les a lancés (une boucle de rejeu, sans doute). Ces restes pèsent peu.
Le volume vient des bases d'index des suites `e2e_code`, et de 34 fichiers de tests ou de `src/`
qui écrivent sous `std::env::temp_dir()`.

**Effacés le 18 octobre** (seconde session cœur C++, à la demande de l'orchestration) :

- **Ce qui est parti** : les trois familles des 11 et 12 octobre, soit 4 033 entrées (2 332
  `rag3weaver-acces-*`, 1 610 `rag3weaver-rag3daemon-*`, 91 `rag3weaver-bulk-*`). Toutes
  appartenaient à l'uid 1000.
- **Comment** : d'abord la preuve que rien ne les tenait (rien dans `/proc/*/fd`, `cwd` et `maps`,
  ni par `fuser`, ni par `lsof`). Puis l'effacement depuis une liste écrite dans un fichier,
  jamais par un motif.
- **Leur poids** : 141 Mio au `du` de la liste, 144 Mio rendus au `df` de `/tmp`. C'est faible,
  ce qui **écarte cette piste pour l'échange** : les 10 Go relevés le 15 ne venaient pas de ces
  restes (voir le ticket de la mémoire de l'hôte).
- **Ce qui reste** : `rag3weaver-lignes-*` et `rag3weaver_cache`, hors du périmètre fixé, ainsi
  que les dossiers des sessions. `/tmp` compte maintenant 92 entrées.

## La cause

Elle est la même qu'au 11 octobre :

- `let _ = std::fs::remove_dir_all(&dossier)` est appelé sur une **base**, qui est un fichier
  (`e2e_rag3daemon.rs:181`, `:294`, `e2e_code.rs:739`). L'appel échoue, `let _` avale l'erreur,
  et la base reste, avec son `.wal` et son `.readers` à côté.
- Rien n'efface `/tmp/rag3weaver_cache` (le `cache_base` par défaut de `Catalog`).

La garde de `run_e2e.sh` masque ces défauts sans les corriger.

## Témoin

Aucun pour l'instant. Il faudrait :

1. jouer `e2e_rag3daemon` et `e2e_code` **sans** `run_e2e.sh`, avec un `TMPDIR` neuf ;
2. vérifier qu'il n'y reste rien.

## Pour le fermer

1. Remplacer le `remove_dir_all` sur une base par l'effacement de la base et de ses voisins
   (`.wal`, `.readers`), ou créer chaque base dans un dossier à elle et effacer ce dossier. Le
   faire partout où un test crée une base sous `temp_dir()`.
2. Faire que l'effacement ait lieu même quand le test échoue, par une garde à `Drop`, sauf avec
   `RAG3WEAVER_GARDER=1`.
3. Donner à `Catalog` un `cache_base` de test dérivé de `TMPDIR`, ou l'effacer à la fermeture
   quand il est temporaire.
4. Une fois rien de vivant dessus (vérifié par `/proc/*/cwd`, `fd` et `maps`, jamais par nom),
   effacer les restes listés plus haut. L'effacement revient à qui Lucie ou l'orchestration
   désigne. Ceux de `e2e_rag3daemon` et `e2e_code` ont été effacés le 18 octobre.

## Le 8 novembre 2026 (session recherche, lot residus-de-suite)

- **Le cache d'index de `Catalog` quitte /tmp, en service comme en test** : `catalog::index_cache_dir`
  rend `RAG3WEAVER_CACHE_DIR` s'il est donné, sinon `<cache XDG>/rag3weaver/cache` ; `temp_dir()`
  seulement sans dossier de cache. À l'ouverture d'un catalogue, une fois par processus et par
  dossier, les dossiers `<pid>` des processus morts y sont effacés (`sweep_dead_index_caches`,
  par `daemon_lifecycle::pid_state` : un pid à l'état illisible reste). Sous run_e2e.sh, chaque
  suite a le sien, sur le disque, effacé avec elle. Le 8 novembre à midi, `/tmp/rag3weaver_cache`
  du poste principal tenait 96 dossiers de pid, 119 Mo.
- **Le témoin de suite** : run_e2e.sh regarde le TMPDIR neuf de chaque suite avant de l'effacer et
  dit ce qu'elle y laisse ; hors de `tests/e2e/suites_qui_laissent_des_residus.txt`, la suite est
  rouge ; une suite de la liste qui finit propre (passe entière) est rouge aussi. La liste maigrit
  à chaque lot qui rend une suite propre, et ce ticket le dit.
- **La mesure de départ** (passe entière de master avec le témoin, liste vide, luciepc) : 70 suites
  laissaient des résidus, dont 64 le seul `rag3weaver_cache`. Restent, une fois le cache sorti :

  | suite | ce qu'elle laisse |
  |---|---|
  | e2e_code | `rag3weaver-bulk-<pid>` et ses `.extensions`, `.fts`, `.readers` (29 Mo) |
  | e2e_juge | deux dossiers `.tmp*` de tempfile |
  | e2e_lectures | un dossier `.tmp*` de tempfile (44 Ko) |
  | e2e_prise_atomique | 36 entrées : `rag3weaver-catalogue-lecteur-*`, `-insistant-*`, `-lecteurs-*`, `-marque-*`, `-modeles-*` (65 Mo) |
  | e2e_rag3daemon | `rag3weaver-acces-<pid>-<n>` et ses `.ecrivain`, `.readers`, `.wal` |
  | e2e_repli_de_branche | `rag3weaver-repli-hybride-<pid>` et ses fichiers (24 Mo) |
  | e2e_rouvrir | `rag3weaver-rouvrir-<pid>` et ses fichiers (8 Mo) |

  Pendant cette mesure, `e2e_banc_qualite` a paniqué faute de granite-278m sur 127.0.0.1:7878 :
  c'est la panne de ce service à luciepc, de 10 h 47 à 12 h 07, déjà en ticket (f66ad4449, cause
  illisible, journaux persistants posés). Rien à voir avec les résidus, et rien à chercher ici.

  Trois passes par suite ont montré que trois d'entre elles vont et viennent : e2e_cited_by
  3/3, e2e_juge 3/3, e2e_lectures 0/3. Leur résidu est un TempDir dont l'effacement a buté (un fil du
  backend écrit encore après son Drop) : ticket
  `2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md`. Elles sont dans la liste
  avec ce ticket, en rouge connu du témoin (« résidu connu : … », propre reste propre). Les cinq
  autres (e2e_code, e2e_prise_atomique, e2e_rag3daemon, e2e_repli_de_branche, e2e_rouvrir) en
  laissent à chaque passe : propres, elles sont rouges.

  La branche de l'équipier `agent/tests-bases-dans-tmp` (relue par hotfixs, qui la complète) vise
  une partie de ces suites : chaque suite qu'elle rend propre sort de la liste dans son commit.

- **Le 9 novembre 2026** (lot arret-du-backend) : e2e_cited_by et e2e_lectures sortent de la liste,
  leur cause corrigée (un backend qu'on laisse tomber attend ses fils, ticket fermé
  `closed/2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md`). e2e_juge reste en
  rouge connu, une autre cause (`2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`),
  avec les cinq suites qui laissent leurs résidus à chaque passe.

- **Le 13 novembre 2026** (lot journal-apres-arret) : e2e_juge sort de la liste, sa cause corrigée
  (l'arrêt de l'hôte écrit les journaux en attente et attend leurs fils, ticket fermé
  `closed/2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`). Restent les cinq
  suites de dette fixe et e2e_tx_par_paquet_arret en rouge connu (le CSV d'un COPY tué).

## Le 14 novembre 2026, vers 14 h : ce qui reste dans /tmp aux deux postes (hotfixs, lecture seule)

Inventaire par motif de nom, avec la taille réelle (blocs), le nombre d'entrées et l'âge de la plus
vieille. On ouvre un dossier anonyme pour lire ce qu'il contient, sans rien effacer. Au portable,
`/tmp` est un tmpfs de 61 Gio, rempli à 16 Gio. **À luciepc, `/tmp` est sur le disque** (la racine btrfs du
NVMe), et non en mémoire.

**Portable : 14,9 Gio au total, dont 1,9 Gio de plus de 24 h, en 14 236 entrées.**

| groupe | taille | entrées | dont > 24 h | depuis | à qui, d'après le nom ou le contenu |
|---|---:|---:|---:|---|---|
| `claude-1000/` | 7,0 Gio | 1 | 0 | 13/11 | dossiers des sessions Claude Code : sairen 3,0 Gio, rag3db 2,0 Gio (dont la session `25b7ff4b…` 1,7 Gio), `~` 1,9 Gio. Ce ne sont pas des tests. |
| `jest_rs/` | 5,6 Gio | 1 | 0 | 13/11 | cache de jest (haste-map) du dépôt sairen, hors de rag3db |
| `tmpxxxxxxxx` Python, à `outils/` | 961 Mio | 1 150 | 775 Mio | 09/11 | Sairen, hors de rag3db |
| `tmpxxxxxxxx` Python, à `secrets/` | 43 Mio | 1 025 | 39 Mio | 09/11 | Sairen, tests de la voix ; **ils contiennent des fichiers d'identifiants en clair** ; le fait est porté à Lucie par c7, et rien n'en est recopié ici |
| `tmpxxxxxxxx` Python, audio et texte | 18 Mio | 2 481 | 2 Mio | 09/11 | Sairen, cache de la synthèse vocale (`.pcm`, « Maison Aurèle, bonsoir. ») |
| `.tmpXXXXXX`, profils Chrome | 327 Mio | 2 233 | 251 Mio | 07/11 | `src/daemon_host/navigateur.rs` (`user_data_dir`) |
| `com.google.Chrome…url_fetcher…` | 253 Mio | 5 497 | 162 Mio | 06/11 | Chrome, lancé par les mêmes tests ou par l'utilisatrice |
| `.tmpXXXXXX`, tests rag3weaver | ≈ 455 Mio | ≈ 380 | ≈ 340 Mio | 07/11 | dossiers à `config`, `data`, `petit-depot`, `code`, `depot`, `taches`, `chats` ou `runs` : `src/daemon_host/` (`ide_sur_un_depot`, `teammate_tests`, `chat_qui_code`, `mes_recherches`, `worktrees`, `ide_par_le_flux`) et `tests/e2e_equipier.rs` |
| `rag3weaver-cibles-de-test-*` | ┐ | 268 | | | `src/commande.rs` |
| `rag3weaver-hook-temoins-*` | ├ 167 Mio | 165 | 131 Mio | 07/11 | `src/claude_code_hook.rs` |
| `rag3weaver-etat-des-temoins-*` | ┘ | 86 | | | `src/builtin_templates.rs` |
| `rag3weaver_cache/` | 129 Mio | 1 | 129 Mio | 09/11 | l'ancien cache par défaut de `Catalog`, que le lot du 8 novembre a sorti de `/tmp` : un reste d'un binaire plus ancien |
| `lucivy-demo-index*` | 75 Mio | 3 | 0 | 11/11 | lucivy |

**luciepc : 6,3 Gio au total, dont 3,2 Gio de plus de 24 h, en 14 353 entrées.**

| groupe | taille | entrées | dont > 24 h | depuis | à qui |
|---|---:|---:|---:|---|---|
| `.tmpXXXXXX`, profils Chrome | 2,9 Gio | 2 298 | 1,9 Gio | 07/11 | `src/daemon_host/navigateur.rs` |
| `org.chromium…url_fetcher…` et autres `org.chromium…` | 0,55 Gio | 9 208 | 0,28 Gio | 07/11 | Chromium des mêmes tests |
| `test_rag3db_clone` | 574 Mio | 1 | 574 Mio | 07/11 | lucivy (`lucivy_core/tests/test_fuzzy_monotonicity.rs`, `test_playground_repro.rs`) |
| `test_monotonicity_segments`, `test_playground_repro`, `test_merge_contains_with` et `_without`, `test_fuzzy_ground_truth`, `test_regex_ground_truth` | 1,17 Gio | 6 | 0 | 14/11 | lucivy, tests du jour (`lucivy_core/tests/`), sous des noms fixes |
| `.tmpXXXXXX`, tests rag3weaver | 478 Mio | 840 | 212 Mio | 07/11 | `src/daemon_host/…`, comme au portable |
| `rag3weaver-bulk*` | 283 Mio | 40 | 196 Mio | 07/11 | `tests/e2e_code.rs` |
| `rag3weaver_cache/` | 101 Mio | 1 | 101 Mio | 11/11 | l'ancien cache par défaut de `Catalog` |
| `rag3weaver-etat-des-temoins-*` | 52 Mio | 213 | 13 Mio | 09/11 | `src/builtin_templates.rs` |
| `rag3weaver-cibles-de-test-*` | 14 Mio | 623 | 14 Mio | 08/11 | `src/commande.rs` |

**Les suites qui récidivent** (elles laissent encore des dossiers datés d'après le 8 novembre) :

1. `src/daemon_host/navigateur.rs`, par ses profils Chrome : la plus lourde, 2,9 Gio à luciepc et
   0,3 Gio au portable.
2. Les tests de `src/daemon_host/` et `tests/e2e_equipier.rs` (`.tmpXXXXXX` à `taches`, `chats`,
   `petit-depot`…).
3. `src/commande.rs` (`rag3weaver-cibles-de-test-*`), `src/claude_code_hook.rs`
   (`rag3weaver-hook-temoins-*`), `src/builtin_templates.rs` (`rag3weaver-etat-des-temoins-*`).
4. `tests/e2e_code.rs` (`rag3weaver-bulk*`).
5. Hors de rag3weaver : les tests de `lucivy_core`, sous des noms fixes ; Sairen, ses tests Python et
   son cache jest.

**Rien n'a été effacé.** Ce qui est à une session se dit à elle. À luciepc, les tests du jour de
lucivy tournent peut-être encore. Les orphelins de plus de 24 h sont à lister pour le mot de Lucie :
1,9 Gio au portable et 3,2 Gio à luciepc, surtout des profils Chrome et des dossiers de tests
rag3weaver. Les deux restes de `rag3weaver_cache` ne servent plus.

### L'effacement du 14 novembre (hotfixs, sur le oui de Lucie)

Lucie a dit oui, d'abord par c7, puis dans la session hotfixs elle-même. Le script `purge-tmp.py`
(au portable dans le scratchpad de la session, à luciepc dans `~/.cache/rag3db-tickets-notes/`) a d'abord
dressé la liste sans rien effacer, puis effacé chaque entrée. Juste avant chaque effacement, il a vérifié,
par un relevé de `/proc/*/fd`, `cwd` et `root`, qu'aucun processus n'y tenait de fichier ouvert. Il n'a
jamais touché `claude-1000/` ni `jest_rs/`, ni le Chrome de Lucie au portable
(`com.google.Chrome…url_fetcher…`), ni rien de moins de 24 h, sauf les dossiers de Sairen qu'elle
avait nommés. Les journaux, une ligne par entrée, sont dans `~/.cache/rag3db-locks/purge-tmp/` aux
deux postes.

| poste | catégorie | parties | rendus | restées (moins de 24 h) |
|---|---|---:|---:|---:|
| portable | dossiers Python de Sairen (`outils/`, `secrets/` effacés sans être lus, audio et texte) | 4 656 | 1 022 Mio | 0 |
| portable | tests rag3weaver `.tmp` | 191 | 345 Mio | 229 (108 Mio) |
| portable | profils Chrome de test | 1 622 | 251 Mio | 699 (86 Mio) |
| portable | `rag3weaver_cache` d'avant le 8 nov. | 1 | 128 Mio | 0 |
| portable | lucivy, worktree Codex (`lucivy-demo-index`, `lucivy-433-scheduler.rs`, `lucivy-close-433.log`) | 3 | 75 Mio | 0 |
| portable | `rag3weaver-hook-temoins-*` et `rag3weaver-cibles-de-test-*` | 289 | 1 Mio | 152 |
| luciepc | profils Chrome de test | 1 679 | 1 897 Mio | 627 (995 Mio) |
| luciepc | Chromium de test (`org.chromium.Chromium…`) | 6 130 | 277 Mio | 3 150 (273 Mio) |
| luciepc | tests rag3weaver `.tmp` | 493 | 212 Mio | 354 (288 Mio) |
| luciepc | `rag3weaver-bulk*` (`e2e_code`) | 28 | 196 Mio | 16 (118 Mio) |
| luciepc | `rag3weaver_cache` d'avant le 8 nov. | 1 | 101 Mio | 0 |
| luciepc | `rag3weaver-cibles-de-test-*` et `rag3weaver-hook-temoins-*` | 616 | 16 Mio | 305 |

**Rendus : 1,78 Gio au portable** (6 762 entrées ; `/tmp` est passé de 16 à 14 Gio pris sur 61) **et
2,64 Gio à luciepc** (8 947 entrées ; il y reste 1,8 Gio dans `/tmp`). Aucune entrée n'était tenue
par un processus, et aucun effacement n'a échoué.

**Ce qui reste, et pourquoi :**
- `claude-1000/` (7,0 Gio) : les sessions vivantes ;
- `jest_rs/` (5,6 Gio) : le cache du travail de Lucie, qu'elle n'a pas nommé ;
- le Chrome de Lucie au portable ;
- tout ce qui avait moins de 24 h ;
- les tests lucivy du jour à luciepc (`test_*`, sous des noms fixes), que c7 lui a signalés ;
- `rag3weaver-etat-des-temoins-*` et les `.tmp` sans contenu reconnu, qui n'étaient pas dans la liste.

Les suites qui récidivent (section précédente) en remettront : le correctif reste à faire dans les
suites elles-mêmes.

## Le 19 novembre 2026 : les cinq suites sont propres (session recherche, entrée 7 de la passation)

Chaque suite a reçu deux commits : le premier la sort de
`extension/rag3weaver/tests/e2e/suites_qui_laissent_des_residus.txt`, et le témoin de suite la juge rouge en
nommant ce qu'elle laisse ; le second la rend propre. Le défaut est le même partout. Un test effaçait le dossier de
sa base par `remove_dir_all`, mais pas ses voisins (`.fts`, `.extensions`, `.readers`, `.ecrivain`, `.wal`), et
parfois rien du tout : un journal du démon à nom fixe, une base qu'un `return` anticipé quittait. Désormais, chaque
base vit dans un `tempfile::TempDir`. Il est déclaré avant les connexions, donc il tombe après elles, et il emporte
la base et tous ses voisins.

| Suite | Ce qu'elle laissait (sonde du 19 nov., 3 tests jusqu'à 13) | Commits |
|---|---|---|
| `e2e_code` | `rag3weaver-bulk-<pid>` et ses voisins | branche `tmpdir-residues` (4b6fff375b, a0820e54c1) |
| `e2e_rouvrir` | 3 entrées, 8,2 Mo | 4319418110, 61c8505490 |
| `e2e_repli_de_branche` | 12 entrées, 24 Mo | 05ff734787, b756720e0e |
| `e2e_rag3daemon` | 9 entrées : deux bases et deux journaux à nom fixe | 5af5ce575e, a3c20ccd51 |
| `e2e_prise_atomique` | 36 entrées, 65 Mo | 5a595629ee, 3bdd11da57 |

Chaque suite a été rouge sans sa ligne sur la sonde `probe-residues-4` (ed13e5a978), puis verte, sans résidu, avec
son correctif (validations vertes du 19 novembre, lib 6e2476d41).

**Ce qui reste, hors de ce ticket :**
- **la ligne intermittente** d'`e2e_tx_par_paquet_arret` : un COPY tué ne peut plus effacer son CSV. Elle a son
  propre ticket (`2026-11-11-un-copy-tue-laisse-son-csv-dans-le-dossier-temporaire.md`) ;
- **le test `aucune_ouverture_n_est_croisee_sous_l_ecrivain_qui_affamait`** (`e2e_prise_atomique`) : il écrit sa
  base sous `~/.cache/rag3weaver-build/affame-*`, hors du TMPDIR, et n'efface que son dossier. Le témoin de suite ne
  le voit pas.

