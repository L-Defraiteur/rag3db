# Session « embarquement » — rapport de passation du 11 novembre 2026

*Pour un agent qui reprend sans aucune mémoire des échanges. Le savoir de fond (les modèles, le
service, le régime GPU, le rattrapage, les pièges) est dans `02-knowledge-dump.md`, à côté.*

## Vocabulaire minimal

- **Embarquer** : calculer le vecteur (« embedding ») d'un texte par un modèle. **Dense** : un
  vecteur de flottants de taille fixe (384, 768 ou 1024). **Creux** (« sparse ») : une liste
  (id de jeton, poids) rendue par BGE-M3.
- **rag3db** : le moteur de base de données (fork de Kùzu), dépôt `~/git_workspaces/rag3db`.
  **rag3weaver** : la caisse Rust qui s'en sert, `extension/rag3weaver/`.
- **Le portable** : le poste où tournent les sessions Claude (celui-ci). **luciepc** : l'autre
  poste de Lucie, joint par `ssh luciepc` (shell fish ; passer par `bash -s` ou `bash -c`). Il porte
  deux cartes AMD Radeon AI PRO R9700 : **card2** (PCI 04:00.0) pour les services d'embarquement,
  **card0** (07:00.0) pour la session de jeu de Lucie (« seat1 »).
- **Service d'embarquement** : le binaire `rag3weaver-embeddings`, un serveur HTTP qui tient un
  modèle sur la carte et rend des vecteurs. Il en tourne trois à luciepc, en unités systemd.
- **Hôte** (`rag3weaver-daemon`, port 7970) : le processus qui tient les bases de code de Lucie et
  les sert à ses outils ; il embarque par les services. **G** (session « optimiseur ») le bascule.
- **c7** (session d'orchestration, nom actuel `rag3db-5a`) : donne l'ordre de passage des pushes et
  les jetons de travail lourd.
- **`poste lourd` / `poste mesure` / `poste push`** : le script `~/.cache/rag3weaver-build/poste`
  (sur les deux postes) qui sérialise les travaux lourds, les mesures et les pushes.
- **Lib** : `librag3db.so`, la bibliothèque du moteur, rangée dans un **magasin**
  (`~/.cache/rag3db-libs/<identifiant de bâti>`) ; une passe l'**épingle** par `RAG3DB_LIB=<id>`.

## Ce qui est sur master (atterri)

| hash | lot |
|---|---|
| `59f02b636` | `/embed_sparse` rend sur option (`"spans": true`) le poids de chaque position du texte : plages d'octets et poids avant la fusion ; `SparseEmbedder::embed_sparse_spans` |
| `9231f07dc` | le binaire `rag3weaver-embeddings` se rebâtit sous ses propres features (la vérification à blanc de `backend.rs` ouvrait une base rag3db hors de `rag3db-native`) |
| `1b629d749` | la doc de l'option et la page du lot : `extension/rag3weaver/docs/07-novembre-2026-embarquements/00-spans-du-creux.md` |
| `f66ad4449`, `e8a91105f` | le ticket de l'arrêt de granite-278m à luciepc (8 novembre, 10 h 47), sa version finale |

Plus ancien (octobre, même session sous le nom « chantier F ») : le chaînage des formes de l'IR, les
éventails, le Cypher brut de l'indexation retiré, le saut en deux temps posé puis retiré
(`c56cb3aa6`), le correctif PostgreSQL du saut sans table d'arrivée (`ec1707ee2`) ; pages dans
`extension/rag3weaver/docs/11-octobre-2026-embarquements/` et `18-octobre-2026-embarquements/`.

## Chantiers en cours

**Aucun.** Pas de lot entamé, pas de commit non poussé, pas de page en attente. La seule chose
ouverte est le ticket de granite-278m (plus bas), qui attend un événement.

## Les services d'embarquement (état lu le 11 novembre à 16 h 25)

Trois unités `systemd --user` à luciepc, actives et activées au démarrage de la session :

| unité | port | modèle | dimension | lancée depuis |
|---|---|---|---|---|
| `rag3weaver-embeddings@granite-278m` | 7878 | granite-278m | 768 | 8 nov. 12 h 07 m 24 s |
| `rag3weaver-embeddings@bge-m3` | 7879 | bge-m3 + relecteur bge-reranker-v2-m3 + OCR ppocrv6-tiny | 1024 (et creux) | 7 nov. 21 h 18 m 04 s |
| `rag3weaver-embeddings@granite-107m` | 7880 | granite-107m | 384 | 8 nov. 8 h 30 m 55 s |

- Unité modèle : `~/.config/systemd/user/rag3weaver-embeddings@.service` (luciepc). Réglages par
  instance : `~/.config/rag3weaver/embeddings/<modèle>.env` (`RAG3WEAVER_EMBEDDINGS_ADDR`,
  `RAG3WEAVER_EMBED_MODEL`, `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0`, `RAG3WEAVER_REGIME=plein`,
  `ROCM_PATH=/opt/rocm`, et pour bge-m3 le relecteur et l'OCR). `Restart=always`, `RestartSec=5`,
  au plus 3 démarrages en 5 min (`StartLimitBurst=3`), `TimeoutStopSec=30`.
- Surcharge `rag3weaver-embeddings@.service.d/journal.conf` : la sortie de chaque unité va en
  `append` dans `~/.cache/rag3weaver-unites-embarquement/journal-<modèle>.log`. **Posée le 8 nov.
  à 12 h 08, après les derniers démarrages** : elle ne vaut qu'au prochain départ de chaque unité.
- Binaire : `~/.local/bin/rag3weaver-embeddings` (sha256 `65ec1d1db17d48ee`, bâti sur la branche
  des spans), commun aux trois unités. L'ancien (sha256 `bb5a9ecab6f635ed`, lot 5 du 3 octobre) est
  gardé en `~/.local/bin/rag3weaver-embeddings.avant-spans`.
- Un quatrième service, **7882** (granite-278m, pid 912225), tourne aussi sur card2 : il n'est pas à
  cette session (lancé à la main, sans unité, pour la session hotfixs). Ne pas le confondre.
- **Cartes** : card2 tenait 26,4 Go sur 34,2 à 16 h 25 (les quatre services), à 0 % ; card0 était à
  11,3 Go et 100 % (une autre charge, pas les services). En Vulkan, `gpu:0` = card2.
- **Régime** : les unités tournent en `RAG3WEAVER_REGIME=plein` ; les passes qui embarquent posent
  côté client `RAG3WEAVER_EMBED_CHAR_BUDGET=4096` et `RAG3WEAVER_GPU_DUTY=70` (voir le dump, §3).
- **Tunnels** : sur le portable, l'unité `rag3weaver-tunnels.service` (active) relie
  `127.0.0.1:7979 → luciepc:7878`, `7980 → 7879`, `7981 → 7880` (`ssh -N`, `ExitOnForwardFailure`).
- **`catch_up_on_open` et rattrapage** : rien de propre à cette session ; le mécanisme est décrit au
  dump, §4. L'hôte de luciepc (`rag3weaver-daemon`) dort par sa socket (`rag3weaver-daemon.socket`
  écoute, le service se lance à la première connexion).

## Branches, worktrees, targets encore sur disque

Rien n'y est inachevé : chaque tête est dans master. Rien n'a été effacé (consigne).

| où | chemin | état | taille |
|---|---|---|---|
| portable | worktree `/home/lucied/git_workspaces/rag3db-spans` | branche `ticket-granite-sans-seat1` à `e8a91105f`, propre, dans master | — |
| luciepc | worktree `/home/lucied/git_workspaces/rag3db-spans` | détaché sur `1b629d749` (spans-creux-5), propre, dans master | 440 Mo |
| luciepc | target `/home/lucied/.cache/rag3weaver-build/target-spans` | target cargo des passes du lot des spans | 13 Go |
| luciepc | `/home/lucied/git_workspaces/rag3db-service` | l'ancien worktree du service (lot 5, `4cb2d04cc`) ; plus rien ne le lance ; **à effacer seulement sur le mot de Lucie** | 5,5 Go |
| luciepc | `/home/lucied/.cache/rag3weaver-build/spans/` | scripts du lot et du redéploiement (`passe.sh`, `redeployer.sh`, `granites.sh`) et leurs journaux | petit |
| luciepc | `/home/lucied/.cache/rag3weaver-unites-embarquement/` | scripts de bascule et de témoin (`bascule-embarquement.sh`, `temoin-embed.py`, `essai-embed.py`), témoins JSON, journal de la bascule du 25 oct. | 168 Ko |
| portable | `/home/lucied/.cache/rag3weaver-build/embarquements/` | scripts et sorties de mesure d'octobre (`latence-embed.py`, `chainage.sh`, `pousser.sh` — ce dernier rejoue sous le verrou : **ne plus l'utiliser**) | 11 Mo |
| portable | `/home/lucied/.cache/rag3weaver-build/spans/pousser-spans.sh` | le dernier pas de push conforme à la règle du 7 nov. (fetch, arrêt si du code arrive, push, trois contrôles) | petit |

Branches distantes de cette session (les supprimer est une décision de Lucie, ne pas le faire) :
`spans-creux`, `spans-creux-2` à `spans-creux-5`, `ticket-granite-10h47`, `ticket-granite-seat1`,
`ticket-granite-sans-seat1` ; et en octobre `saut-en-deux-temps*`, `chainage-1` (déjà supprimées
pour certaines). Branches locales du portable : `spans-creux`, `ticket-granite-10h47`,
`ticket-granite-seat1`, `ticket-granite-sans-seat1`.

## Tickets

À cette session (ouverts, dans `docs/tickets/`, **non déplacés**) :

- `2026-11-08-granite-278m-a-luciepc-s-est-arrete-huit-fois-cause-illisible.md` — l'unité 7878 est
  sortie en erreur huit fois à 10 h 47 le 8 nov. ; cause illisible (journal tourné). Attend la
  prochaine sortie d'une unité pour lire son `journal-<modèle>.log` ; sans sortie d'ici le 8 déc.,
  le fermer en le disant.

Liés à l'embarquement mais portés par d'autres sessions (pour mémoire, ne pas les prendre sans c7) :
`2026-11-08-des-morceaux-refuses-par-le-service-ne-sont-jamais-repris.md` (everything declarative),
`2026-11-08-le-delai-du-service-d-embarquement-est-un-chiffre-rond.md`,
`2026-11-07-un-test-de-la-lib-qui-parle-au-service-d-embarquement-attend-sans-delai.md`,
`2026-11-08-le-temoin-de-l-embarquement-hors-verrou-rougit-sous-charge.md` (recherche),
`2026-11-09-le-banc-de-charge-d-ingestion-tient-la-carte-pres-de-deux-heures-dans-l-e2e-entier.md`
(recherche), `2026-11-09-cinq-clients-de-production-attendent-sans-delai.md` (archi).

## Ce qui est bloqué

Rien n'est bloqué. Deux choses attendent un mot ou un événement :
- l'effacement de `rag3db-service` à luciepc : le mot de Lucie ;
- le ticket de granite-278m : la prochaine sortie d'une unité.

## Reprendre

Aucun worktree ne porte de travail inachevé : un agent qui reprend part de **master**. Pour rouvrir
un chantier d'embarquement :

1. Worktree à soi (jamais dans `~/git_workspaces/rag3db`, l'arbre de Lucie) :
   `cd ~/git_workspaces/rag3db && git fetch origin && git worktree add -b <branche> ../rag3db-<nom> origin/master && cd ../rag3db-<nom> && git submodule update --init extension/rag3weaver/codeparsers && git config user.email luciedefraiteur@gmail.com`.
   Le worktree `rag3db-spans` existant peut servir après `git fetch && git switch -c <branche> origin/master && git submodule update`.
2. Passe (à luciepc, où le target `target-spans` est chaud) : le modèle est
   `~/.cache/rag3weaver-build/spans/passe.sh`. Il épingle la lib (`RAG3DB_LIB=<id complet>` ;
   `RAG3WEAVER_MOTEUR_ANCIEN=1` seulement si master porte un commit moteur plus récent que la lib),
   joue la lib entière **sans** `RAG3WEAVER_EMBED_SERVICE`, `scripts/check_features.sh`,
   `FEATURES=… scripts/temoins_navigateur.sh` (avec `RAG3WEAVER_PAQUETS_REACT=/home/lucied/.cache/rag3weaver-build/I-paquets-react`),
   et les e2e du changement par `./run_e2e.sh --test <suite>` **avec** les services. Lancer sous
   `systemd-run --user --unit=<nom> --collect bash -c "~/.cache/rag3weaver-build/poste lourd timeout 3h <script> > <sortie> 2>&1"`.
3. Push : rebase sur `origin/master` du moment, rejeu court hors verrou si du code est arrivé, puis
   le dernier pas seul sous le verrou :
   `~/.cache/rag3weaver-build/poste push embarquement -- bash /home/lucied/.cache/rag3weaver-build/spans/pousser-spans.sh`
   (il travaille dans `/home/lucied/git_workspaces/rag3db-spans` : adapter `cd` s'il change).
   Ensuite les trois contrôles : `git status` vide, aucun rebase en cours, `origin/master` = `HEAD`.
4. Redéployer le binaire d'un service : annoncer 5 min avant (banc, lucivy, hooks-essai, G, c7), puis
   `~/.cache/rag3weaver-build/spans/redeployer.sh` (bge-m3) ou `granites.sh` (les deux granite) : ils
   notent un vecteur témoin, remplacent le binaire, redémarrent l'unité et exigent un écart absolu
   nul ; sur un rouge, retour à l'ancien binaire.

## Les trois prochains pas

1. À la prochaine sortie d'une unité d'embarquement : lire
   `~/.cache/rag3weaver-unites-embarquement/journal-<modèle>.log` à luciepc et compléter (ou fermer)
   le ticket de granite-278m.
2. Sur le mot de Lucie : effacer `~/git_workspaces/rag3db-service` à luciepc (5,5 Go), puis
   `target-spans` (13 Go) et le worktree `rag3db-spans` des deux postes si aucun lot ne les reprend.
3. Vérifier (non joué) le soupçon noté au dump §8 : `SUITES_LOCALES` ne reconnaîtrait plus les
   suites locales depuis que les e2e sont un seul binaire `e2e-<hash>`.
