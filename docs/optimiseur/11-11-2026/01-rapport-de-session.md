# Rapport de session — G, « l'optimiseur » (11 novembre 2026, 16 h 25)

## Mise à jour du 11 novembre, après 22 h 04

Le relevé de 16 h 25 ci-dessous reste historique. Le binaire installé aux deux
postes est désormais **68b9ede17c182487d3f823c2e5cc5906b62a4a46**, SHA256
`6af3ec67450863021735359132b8d68e2afb97a8e0c50f2b182fd94a0bc44e2e`, contre la
lib complète 29a7bc52a inchangée. Pose coordonnée : luciepc 22:03:08–22:03:11,
portable 22:03:56–22:04:04. Atelier principal : 20 outils effectivement exposés,
UI servie conforme au commit et rendu navigateur sans erreur ; atelier d’essai
inchangé à trois outils. Ancien binaire d134 préparé mais jamais posé.

Preuves, sauvegardes, contrôles et limites :
`docs/tickets/closed/2026-11-11-preparer-et-poser-le-binaire-regroupe-aux-deux-postes.md`.
La préparation future repart de `~/git_workspaces/cx-optimiseur-controle` à luciepc,
créé pour respecter la convention cx- ; l’ancien worktree contenant des sous-modules
reste intact. Aucun nettoyage de base, d’instantané ou d’ancien artefact effectué.

Écrit pour un agent qui reprend sans aucune mémoire de nos échanges. Le savoir-faire (comment bâtir,
poser, revenir en arrière, lire la carte, les pièges) est dans `02-knowledge-dump.md`, à côté. Ici :
l'état, relevé le 11 novembre entre 16 h 24 et 16 h 26.

## Qui je suis et ce que je tiens

G est la session qui tient **les deux postes** de Lucie :
- **le portable**, nom d'hôte `lucie-tablette` : celui où Lucie travaille ;
- **luciepc** : la tour, joignable par `ssh luciepc` sur tailscale, où l'on bâtit et où tournent les
  services d'embarquement (GPU).

Sur chacun tourne **l'hôte** : le processus `rag3weaver-backend daemon`, qui ouvre les bases et sert les
outils (MCP, IDE) sur `127.0.0.1:7970`. Mon travail consiste à bâtir la **lib** du moteur (`librag3db.so`,
le C++ de `src/`) et le **binaire** de l'hôte (le Rust de `extension/rag3weaver`), à les **poser** (les
installer et redémarrer l'hôte), et à **prouver** que la pose est saine. Les ordres viennent de la session
d'orchestration « c7 » ; les mots de Lucie arrivent relayés par elle.

## Les deux postes

### La lib et le magasin

Le **magasin** est le dossier `~/.cache/rag3db-libs/`, un sous-dossier par lib bâtie, nommé
`rag3db-build-<sha complet du dernier commit moteur>-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
Chaque sous-dossier contient `librag3db.so`, `libvector.rag3db_extension`, `rag3db.hpp`, `BUILD_ID` et
`MANIFESTE`. Le lien `~/.cache/rag3db-libs/courant` désigne la lib que prennent par défaut les passes
de tests et les bâtis ; **il est le même aux deux postes**.

- **`courant` = 29a7bc52a** aux deux postes depuis le 9 nov. à 12 h 12 min 46 s :
  `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
  `librag3db.so` md5 b65cc5be, `libvector` md5 7ca6d4ec. Elle porte le **correctif des deux sens**
  (commit 29a7bc52a, `src/storage/table/csr_node_group.cpp` : une région CSR dont les listes changent
  de place réécrit toutes ses colonnes). Avant lui, un index complet laissait des petites tables de
  relations différentes selon qu'on les lit dans un sens ou dans l'autre (voir le ticket
  `docs/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md`).
- Libs gardées pour revenir en arrière, toutes au magasin des deux postes :

  | lib (début du sha) | md5 librag3db / libvector | ce qu'elle porte |
  |---|---|---|
  | 9bae60786 | 88084c87 / a588f58a | les gardes contre le SIGSEGV de l'hôte (pages rendues sous des segments vivants), G3 |
  | cbc06c3e1 | 389935e1 / a21b5bb2 | le correctif du use-after-free HNSW (`shrinkForNode`) et du rejeu |
  | dadbf0041 | 253c898f / 5ac6d668 | projection sous READ ONLY ; **SIGSEGV de l'hôte**, ne pas y revenir |
  | 17a094d12 | 16d68492 / a4531954 | format de base 43, la lib de repli du 8 nov. au matin |
  | 04063ee06 | 0de1dd0f / d7402369 | la dernière lib au **format 42** : ne lit pas une base 43 |

  Le magasin du portable en compte 33 ; rien n'y a été effacé.

### Les binaires posés

| poste | binaire de l'hôte | md5 / check-lib | contre la lib | posé le |
|---|---|---|---|---|
| portable | tête de master **0b0ff221a** | b872f3fd / 277b744b | 29a7bc52a | 9 nov., 13 h 41 min 19 s |
| luciepc | tête de master **0b0ff221a** | b872f3fd / 277b744b | 29a7bc52a | 9 nov., 13 h 39 min 34 s |

Le binaire est `~/.local/bin/rag3weaver-backend` ; son vérificateur est `~/.local/bin/rag3db-check-lib`.
Chaque pose garde l'ancien sous `~/.local/bin/rag3weaver-backend.avant-<8 premiers caractères du hash
posé>`. Exemple : `.avant-0b0ff221` est le binaire **d'avant** la pose de 0b0ff221a, c'est-à-dire celui
de 833c89539. Au portable : `.avant-0b0ff221`, `.avant-29a7bc52`, `.avant-9bae6078`, `.avant-f16628f8`,
`.avant-e59e5983`, `.avant-cbc06c3e`, `.avant-eb154bf5`, et de plus anciens.

L'hôte du portable tourne : MainPID 1105060, lib 29a7bc52a lue dans `/proc/<pid>/maps`. Celui de luciepc
est **inactif** (il s'arrête seul après 30 min sans appel), et sa socket écoute pour le relancer au
premier appel.

Le réglage de l'hôte vit dans `~/.config/rag3weaver/rag3weaver-daemon.env` (`RAG3DB_LIBRARY_DIR` = la lib
du magasin, `RAG3WEAVER_EMBED_SERVICE`, le projet Vertex). Son **filet** est
`~/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf` : avant de démarrer, l'unité vérifie
que le binaire et la lib sont du même bâti, et refuse sinon.

### Les instantanés (copies de retour) et RETOUR.sh

Un **instantané** est une copie instantanée par reflink (btrfs, sans place disque au départ) des
fichiers d'une base, prise hôte arrêté, dans `~/.local/share/rag3weaver/instantanes/<étiquette>/<base>/`.
À côté, `RETOUR.sh` est la recette qui remet les bases dans cet état.

- Portable (bases de Lucie : `code-rag3db-depot`, `memoire-longue`) : `avant-29a7bc52a` (9 nov. 12 h 18,
  6,0 Gio), `avant-9bae60786` (9 nov. 9 h 30, 6,0 Gio), `avant-cbc06c3e1` (8 nov. 15 h 41, 5,1 Gio),
  `avant-43` (3,2 Gio), `controle-portable-8nov` et sa copie `-controle` (4,5 Gio chacune).
- luciepc (bases d'essai `code-lourd`, `code-principal`) : `avant-29a7bc52a`, `avant-9bae60786` (+ `-controle`),
  `avant-43`, `controle-29a` (+ `-controle` : l'index de preuve du correctif, 0 écart),
  `departage-*` (le départage du 8 nov.), `nuit-9bae60786-ouvert` (+ `-controle`), `preuve-cbc06c3e1`
  (+ `-controle`), et trois `hotfixs-trace-*` qui sont à la session hotfixs.
- Preuve gardée : `~/.local/share/rag3weaver/preuves/2026-11-08-code-rag3db-depot-wal-dadbf0041/` au
  portable (la base dont le journal refusait le rejeu, le 8 nov.).

Rien de tout cela n'a été effacé : c'est à Lucie de dire ce qui peut partir.

### Les services d'embarquement (GPU, luciepc)

Un **service d'embarquement** calcule les vecteurs (embeddings) des textes ; chacun est un processus
`~/.local/bin/rag3weaver-embeddings --adresse 127.0.0.1:<port>`, lancé par une unité systemd utilisateur
`rag3weaver-embeddings@<modèle>.service` dont le réglage est dans `~/.config/rag3weaver/embeddings/<modèle>.env`.

| port | modèle | unité | VRAM tenue (11 nov.) |
|---|---|---|---|
| 7878 | granite-278m (768 dim.) | `rag3weaver-embeddings@granite-278m` | 6,1 Gio |
| 7879 | bge-m3 (1024 dim., dense + creux, relecteur, OCR) | `@bge-m3` | 8,6 Gio |
| 7880 | granite-107m (384 dim.) | `@granite-107m` | 5,8 Gio |
| 7882 | granite-278m, **à la session hotfixs**, lancé à la main | — | 6,9 Gio |

Tous sont sur **card2** (PCI 04:00.0 ; `GPU[0]` pour `rocm-smi` ; `gpu:0` pour Vulkan), à 77 % de VRAM
le 11 nov. **card0** (07:00.0, `GPU[1]`) est la carte de l'écran de seat1, où **Lucie joue** : le
11 nov. à 16 h 25 elle était à 100 % d'usage et 33 % de VRAM.

### Les tunnels

Le portable n'a pas de GPU de service : l'unité `rag3weaver-tunnels.service` (portable,
`~/.config/systemd/user/rag3weaver-tunnels.service`) tient un `ssh -N` vers luciepc qui renvoie
**7979 → 7878, 7980 → 7879, 7981 → 7880** (`Restart=always`, toutes les 10 s). Le 9 nov. vers 1 h 50,
deux coupures réseau (tailscale passé en relais) l'ont fait redémarrer quatre fois ; elle s'est rétablie
seule.

### Les unités systemd utilisateur

- Portable : `rag3weaver-daemon.socket` (écoute 7970), `rag3weaver-daemon.service` (l'hôte),
  `rag3weaver-tunnels.service`.
- luciepc : les mêmes `daemon.socket` et `daemon.service`, et les trois `rag3weaver-embeddings@…`.

### Les lourds de luciepc

`~/.cache/rag3weaver-build/poste` est le verrou des postes : `poste lourd <commande>` fait la file pour
les travaux lourds (compilation, passe de tests). Le nombre de places est le fichier
`~/.cache/rag3weaver-build/lourds-max` (le dossier des verrous) (lu à chaque tour ; 2 s'il manque ;
`POSTE_LOURDS_MAX` l'emporte). Le 11 nov. : **4** à luciepc. `poste --qui` dit qui tient et qui attend.

### Le disque de luciepc (TRIM)

- Le NVMe de luciepc (btrfs, `/` et `/home` sur `/dev/nvme0n1p2`) est monté **sans `discard`** depuis le
  8 nov. au soir : le TRIM à la volée occupait 95 % du temps du disque et faisait rougir les tests sur des
  délais de 60 s.
- Reste **`fstrim.timer`** : actif, hebdomadaire (lundi), `Persistent`. Le lundi 9 nov. à 1 h 35, il a
  rendu ~200 000 petits extents à 20 commandes par seconde : disque à 100 %, commits btrfs de 6 à 9 min,
  tâches bloquées (Steam compris), pendant plus d'une heure.
- **Ce que Lucie doit poser (sudo, pas posé le 11 nov.)** : un drop-in pour que le service ne rende
  que les grands extents :
  ```
  sudo mkdir -p /etc/systemd/system/fstrim.service.d
  printf '[Service]\nExecStart=\nExecStart=/usr/bin/fstrim --listed-in /etc/fstab:/proc/self/mountinfo --verbose --quiet-unsupported --minimum 16M\n' | sudo tee /etc/systemd/system/fstrim.service.d/minimum.conf
  sudo systemctl daemon-reload
  ```
  Vérifier ensuite : `systemctl cat fstrim.service` montre `--minimum 16M`.
- Le 11 nov. : PSI io à 0, `fstrim.service` inactif, 1,3 Tio libres. Portable : 318 Gio libres.

## Ce qui est bloqué, et sur quoi

1. **L'atelier `code-rag3db-essai-atelier`** (portable, `~/.config/rag3weaver/code-rag3db-essai-atelier/`)
   n'a pas de note de pose (`livre.json`) : il ne suit pas le binaire. Il faut retirer son manifeste
   puis relancer `rag3weaver-backend atelier`. **Ma session s'est vu refuser cette commande** (motif
   « Modify Shared Resources ») : c'est à Lucie de la jouer. L'atelier principal de Lucie
   (`~/.config/rag3weaver/atelier/`) suit, lui (11 outils depuis le 9 nov., 13 h 41).
2. **Le drop-in de `fstrim.service`** à luciepc : sudo, donc Lucie (ci-dessus).
3. **Le lot d'archi be319fa75** (une commande mise en attente se décide sur la carte d'accord) est sur
   master mais pas posé : il attend le mot de c7 pour une pose « binaire seul ».
4. **Les bases existantes gardent leur écart des deux sens** : le correctif empêche d'en créer, il ne
   répare pas. Les bases de luciepc (`code-lourd`, `code-principal`, indexées le 8 nov. à midi) l'ont ;
   celle du portable n'en avait pas au contrôle du 8 nov. (14 h 15). Réparer = réindexer complètement
   sous 29a7bc52a ou plus récent, sur décision de c7 ou de Lucie.

## Mes tickets ouverts (`docs/tickets/`)

- `2026-11-08-le-plein-texte-de-code-lourd-compte-plus-de-documents-que-de-lignes.md` : l'index plein
  texte (lucivy) de code-lourd compte plus de documents que la table n'a de lignes (Scope : 82 958 pour
  82 855), relevé par moi ; pour la recherche et lucivy.

J'ai aussi fourni les preuves du ticket des deux sens (porté par hotfixs, corrigé par 29a7bc52a, pas
encore fermé) : instantanés `departage-*` et `controle-29a` à luciepc.

## Reprendre

Aucun travail de code n'est à moitié fait : tous mes lots sont sur master. Ce qui reste, ce sont des
poses (pas 1 à 3 ci-dessous). Voici mes worktrees et bâtis, dans l'état du 11 nov. à 16 h 25.

**Portable**

- `/home/lucied/git_workspaces/rag3db-docs-G` : mon worktree de docs, HEAD détaché sur
  `origin/master` (3c0ec28b5 au relevé, avant le commit de ces pages). Il ne sert qu'à écrire et pousser
  mes pages : `git -C … fetch origin master && git -C … checkout --detach origin/master`, écrire, puis
  `git commit -- <chemins>` et `~/.cache/rag3weaver-build/poste push optimiseur-G -- bash docs/optimiseur/scripts/push-docs.sh`.
- `/home/lucied/git_workspaces/rag3db-plafond-G` (branche `plafond-target-32`, d845744a2) et
  `/home/lucied/git_workspaces/rag3db-poste-G` (branche `poste-n-detenteurs`, e006f4cff) : **finis**,
  tous deux contenus dans master, sans modification. Retirer les worktrees m'a été refusé (« Git
  Destructive ») : c'est à Lucie de les retirer, ou de les garder.

**luciepc**

- `/home/lucied/git_workspaces/rag3db-controle-G` : HEAD détaché sur 0b0ff221a (9 nov., 13 h 37), sans
  modification. C'est le worktree des bâtis de binaires et des passes e2e ; `bati-lucie-luciepc.sh` y fait
  `git checkout --detach <hash>` à chaque bâti. Rien à reprendre.
- `/home/lucied/.cache/rag3weaver-build/wt-lib-arbre` : HEAD détaché sur 29a7bc52a (9 nov., 10 h 30),
  sans modification, `build/lib` de 183 Mio gardé pour l'incrémental. La prochaine lib :
  `batir-lib.sh <hash>` (voir le dump, « Bâtir une lib »).
- Targets cargo : `~/.cache/rag3weaver-build/target-G/hote-lucie` (13 Gio, le binaire de l'hôte, contient
  le bâti de 0b0ff221a), `target-G/hote-lucie-tests` (3,9 Gio), `target-G/passe` (5,0 Gio, les e2e). À
  garder pour les bâtis incrémentaux.
- Binaires copiés : `~/.cache/rag3weaver-build/nuit-0b0ff221a/` (le binaire posé),
  `nuit-29a/`, `nuit-9bae/`.
- Dossiers d'essai : `~/.cache/rag3weaver-build/departage/` (les hôtes d'essai du départage) et
  `embed-7883/` (le journal du service de 10 h 32 qui a fait le reset de ring ; **aucun processus
  7883 ne tourne**).

Pour reprendre la prochaine pose (be319fa75 ou plus récent, binaire seul) depuis le portable :
```
S=/home/lucied/git_workspaces/rag3db-docs-G/docs/optimiseur/scripts   # ou une copie des scripts
H=$(git -C /home/lucied/git_workspaces/rag3db-docs-G rev-parse --short=9 origin/master)
bash $S/preparer-binaire.sh $H         # bâti à luciepc, essai et check-manifests aux deux postes
bash $S/poser-binaire.sh $H luciepc    # après le mot de c7
# préavis d'une minute, puis :
bash $S/poser-binaire.sh $H portable
python3 $S/outils-atelier.py           # 11 outils attendus, ou plus si le lot en ajoute
```
`poser-binaire.sh luciepc` appelle les copies `~/.cache/rag3weaver-build/bascule-pose.sh`,
`bascule-reveil.sh` et `ouvrir-bases.py` de luciepc : elles sont à jour (garde nuancée comprise).

## Les trois prochains pas

1. Au mot de c7 : poser **be319fa75 ou la tête de master** en binaire seul contre 29a7bc52a, par
   `docs/optimiseur/scripts/preparer-binaire.sh <hash>` puis `poser-binaire.sh <hash> luciepc`, puis
   `… portable` avec un préavis d'une minute (voir le dump, « Poser »).
2. Quand Lucie aura joué `rag3weaver-backend atelier` pour `code-rag3db-essai-atelier` : vérifier avec
   `outils-atelier.py` (adapté au manifeste) qu'il expose ses outils.
3. Quand Lucie aura posé le drop-in de `fstrim.service` : relire `systemctl cat fstrim.service` à
   luciepc, et le lundi suivant, `cat /proc/pressure/io` pendant le trim.
