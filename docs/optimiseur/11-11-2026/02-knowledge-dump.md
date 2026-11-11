# Knowledge dump — tenir les deux postes de Lucie (G, 11 novembre 2026)

Pour un agent sans mémoire de nos échanges. L'état du moment est dans `01-rapport-de-session.md`. Les
scripts cités sont copiés dans **`docs/optimiseur/scripts/`** (dépôt rag3db) ; plusieurs ont aussi une
copie de travail à luciepc dans `~/.cache/rag3weaver-build/`, d'où les scripts de pose les appellent.
Les procédures communes vivent dans `docs/procedures/scripts/`.

## Le vocabulaire

- **Moteur** : le C++ de `src/` et `extension/vector` ; bâti, il donne `librag3db.so` (la **lib**).
- **Hôte** : `rag3weaver-backend daemon`, le binaire Rust de `extension/rag3weaver`, qui ouvre les bases
  et sert les outils sur `127.0.0.1:7970`. Unité systemd utilisateur `rag3weaver-daemon.service`, lancée
  à la demande par `rag3weaver-daemon.socket` ; il s'arrête seul après 30 min sans appel.
- **Backend** : une base ouverte par l'hôte d'après son **manifeste** (un JSON : chemin de la base,
  service d'embarquement, outils, montre…). Les bases de Lucie au portable : `code-rag3db-depot`
  (manifeste `~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json`) et `memoire-longue`
  (`~/.config/rag3weaver/memoire-longue/backends/memory/backend.json`).
- **Magasin** : `~/.cache/rag3db-libs/<id>`, une lib par dossier, `<id>` nommé d'après le **dernier
  commit moteur**, pas d'après la tête demandée. **`courant`** : le lien du magasin vers la lib par défaut.
- **RUNPATH** : le binaire de l'hôte est lié avec `-Wl,-rpath,<dossier de la lib au magasin>` ; il charge
  donc la lib de ce dossier même hors de l'unité. `LD_LIBRARY_PATH` passe devant le RUNPATH : c'est ainsi
  qu'on fait tourner un binaire sous une autre lib pour un essai.
- **check-lib** : `rag3db-check-lib <binaire> <librag3db.so>` dit « même bâti » si le binaire a été lié
  contre cette lib (même `BUILD_ID`). L'unité le joue avant chaque démarrage (le **filet**).
- **Montre** : la surveillance du dépôt par l'hôte, qui réindexe les fichiers changés.
- **Index complet** : `index` avec `{"confirm": true}`, qui réindexe tout le dépôt.
- **Contrôle des deux sens** : `docs/procedures/scripts/controle-des-deux-sens.sh <étiquette> <id de lib>`
  bâtit `tools/check_rel_directions` d'origin/master contre la lib, recopie l'instantané et lit chaque
  table de relations dans ses deux rangements (sens avant, sens arrière). « Aucun écart » = rc 0.
- **Lourd** : un travail qui charge un poste (compilation, passe de tests) ; il passe par
  `~/.cache/rag3weaver-build/poste lourd <commande>`, qui fait la file selon `lourds-max`.
- **Pose** : installer un binaire (et sa lib) et redémarrer l'hôte. **Repose binaire seul** : la lib ne
  change pas, les bases non plus, donc pas d'instantané neuf.

## Bâtir une lib

À luciepc, jamais dans l'arbre de Lucie (`~/git_workspaces/rag3db` : ni commit, ni bâti, ni changement de
branche). Le worktree de bâti est `~/.cache/rag3weaver-build/wt-lib-arbre`, gardé avec son `build/` pour
des bâtis incrémentaux.

```
cd ~/.cache/rag3weaver-build
setsid nohup bash ~/git_workspaces/rag3db-controle-G/docs/procedures/scripts/batir-lib.sh <hash> > batir-lib-<hash>.log 2>&1 < /dev/null & disown
```
- Options CMake (dans le script) : Release, `BUILD_EXTENSIONS=vector`, en-tête unique, ni tests ni shell ;
  `cmake --build … -j 8` sous `poste lourd` (**jamais `-j$(nproc)`** : deux cœurs restent libres).
- Le script compare les symboles et les tailles de vtables à `courant`. Un symbole retiré ou une vtable
  qui change de taille = **ABI changée** : on l'annonce avant toute pose. `ABI_CHANGEE=1` le force quand
  le cœur le déclare.
- Il publie au magasin **sans toucher `courant`**. L'état est dans `~/.cache/rag3weaver-notes-A/lib58/etat`,
  l'id dans `…/lib58/id`.
- **Prouver l'artefact** : lire `BUILD_ID`, comparer le md5 à l'ancienne lib, et vérifier que le `.o` du
  fichier changé a bien été recompilé (`find build/lib -name '<fichier>.cpp.o' -printf '%TT'`). Un bâti
  incrémental d'un fichier prend 4 s ; ce n'est pas suspect, mais ça se prouve.
- Copier la lib au portable : `rsync -a luciepc:.cache/rag3db-libs/<id> ~/.cache/rag3db-libs/`, puis
  comparer les md5.

## Bâtir un binaire

`docs/optimiseur/scripts/bati-lucie-luciepc.sh <id de lib> <hash de master>` (copie de travail à luciepc),
sous `poste lourd`, `JOBS=8`. Il fait un checkout détaché de `~/git_workspaces/rag3db-controle-G`, bâtit
`rag3weaver-backend` et `rag3db-check-lib` (`--features rag3db-native,code,daemon,openai-llm`) dans
`~/.cache/rag3weaver-build/target-G/hote-lucie`, avec le RUNPATH vers la lib, puis joue check-lib.
`LIB_EXPLICITE=1` bâtit contre une lib qui n'est pas encore `courant`. Il refuse un commit qui contient
0e528854a (recherche filtrée sous l'instantané) contre 17a094d12.

Le binaire est **écrasé à chaque bâti** dans `target-G/hote-lucie` : on le copie aussitôt dans
`~/.cache/rag3weaver-build/nuit-<hash>/`.

## Vérifier avant de poser

1. **Passe e2e sous la lib** : `docs/optimiseur/scripts/nuit-verdict.sh` (copie de
   `docs/procedures/scripts/verdict-e2e-code.sh` avec le worktree `rag3db-controle-G` et
   `target-G/passe`). Il épingle `RAG3DB_LIB=<id>` et joue e2e_code puis e2e_lectures sous `poste lourd`.
   Vert = « verdict VERT » **et** le moteur lié lu dans les journaux est la bonne lib **et** 0 ignoré.
   Lire `/embed` sur 7878 avant : au-delà de 1 s, la carte est tenue par quelqu'un, on attend.
2. **Hôte d'essai** : `docs/optimiseur/scripts/essai-hote-nouveau.sh <id>` (`X=`, `C=` pour un autre
   binaire ; `LUCIVY_ATTENDU=4.3.2`). Il lance l'hôte sur **7991**, avec son propre dossier, lit `/sante`
   et la lib dans `/proc/<pid>/maps`, puis l'arrête par son pid.
3. **check-manifests** : `rag3weaver-backend.nouveau check-manifests <manifestes…>` sur tous les manifestes
   de Lucie (six au portable : les deux bases, `code-essais-c7`, les deux de `code-rag3db-essai-atelier`,
   et `~/.config/rag3weaver/atelier/templates/backends/atelier/backend.json`).
4. **Contrôle des deux sens** quand la lib touche le stockage : sur des copies, jamais sur les bases
   vivantes (voir « Prouver une lib par un index »).

`docs/optimiseur/scripts/preparer-binaire.sh <hash>` enchaîne le bâti, l'essai et check-manifests aux deux
postes, et ne pose rien.

## Poser

L'ordre est toujours **luciepc d'abord, puis le portable**.

1. **Annoncer** la coupure du portable à c7 et aux sessions qui utilisent 7970, **cinq minutes avant**
   pour une lib, **une minute** pour un binaire seul. **Lire l'heure avec `date +%T` avant de l'écrire**
   (le 8 nov., une heure écrite de tête m'a fait couper quatre minutes trop tôt).
2. **Pose** : `docs/optimiseur/scripts/bascule-pose.sh <id de lib> <étiquette avant-…>`. Il attend
   `~/.local/bin/rag3weaver-backend.nouveau` et `rag3db-check-lib.nouveau`, arrête la socket et l'hôte
   proprement (systemctl stop, surveillé 5 min, **rien n'est tué**), garde l'ancien sous `.<étiquette>`,
   pose le nouveau, réécrit `RAG3DB_LIBRARY_DIR` dans l'env, réinstalle le filet
   (`extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh --filet-seul` ; à luciepc,
   `INSTALLEUR=$HOME/git_workspaces/rag3db-controle-G/extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh`).
   La socket reste arrêtée. S'il voit un écart, il remet l'ancien.
3. **Instantané** des bases si la lib change :
   `bash ~/.cache/rag3weaver-notes-A/instantanes.sh avant-<lib> <base.rag3db>…` (refuse si un processus
   tient la base). Pour une lib, la session hotfixs prend aussi sa copie de `memoire-longue` : lui écrire
   « arrêté », attendre « copié » **deux minutes au plus**, puisque mon instantané la contient.
4. **Réveil** : `docs/optimiseur/scripts/bascule-reveil.sh <id> <adresses des services>` (au portable :
   `127.0.0.1:7979 127.0.0.1:7980 127.0.0.1:7981` ; à luciepc : 7878, 7879, 7880). Il démarre la socket,
   lit `/sante`, le MainPID, la lib dans les maps, `LISTEN_FDS`, l'env et le jeton.
   **La garde des services**, telle que nuancée le 9 nov. : un service **absent** (rien en 30 s) **refuse**
   le réveil ; un service **lent** (plus de 5 s) **prévient et réveille**. Le 9 nov., bge-m3 répondait en
   17 à 19 s, tenu par un e2e d'ingestion : c7 a tranché que l'hôte de Lucie coupé pèse plus qu'un
   embarquement lent. Appeler le script sans adresses saute la garde : à faire seulement sur mot de c7.
5. **Rouvrir les bases** : `docs/optimiseur/scripts/ouvrir-bases.py <manifestes…>` (POST `/backend/open` ;
   attendre `"ok": true` pour chacun, et lire `journalctl --user -u rag3weaver-daemon --since <heure>`).
6. **Témoin de l'IDE** : `JETON=$(cat ~/.local/share/rag3weaver/daemon.token) SCRATCH=<dossier> node
   docs/optimiseur/scripts/rendu-ide.mjs 7970 /ide/` rend la page dans Chrome sans tête par CDP ; on lit
   « Dépôts », le nombre d'éléments et « erreurs de console : aucun ».
7. **`courant`** aux deux postes si la lib change : `docs/procedures/scripts/courant-bascule.sh <id> <md5>`
   (refuse si `ide-fusion` tourne à luciepc ou si les md5 diffèrent).
8. Dire « posé » à c7 avec : hash, md5, heure de coupure et de réveil, MainPID, et les lectures.

`docs/optimiseur/scripts/poser-binaire.sh <hash> luciepc|portable` enchaîne les étapes 2, 4 et 5 d'une
repose binaire seul.

### L'atelier de Lucie et son livre.json

L'atelier (`~/.config/rag3weaver/atelier/templates/backends/atelier/backend.json`) est un gabarit livré
par le binaire. La pose le note dans **`livre.json`**, hors de l'arbre :
`~/.local/state/rag3weaver/manifestes/<blake3(chemin canonique du manifeste)[..16]>/livre.json` (pour
l'atelier de Lucie : `bb347ff93e835435`). À la réouverture par l'hôte, `builtin_templates::refresh`
compare le manifeste à la note. S'il n'a pas été modifié à la main, il le remet au livré du nouveau binaire
(nouveaux outils compris) et réécrit la note. Le commentaire de la struct `Livre` qui parle de
`<manifeste>.livre` est périmé (archi le corrige). Contrôle :
`docs/optimiseur/scripts/outils-atelier.py` liste les outils ; le 9 nov. on est passé de 5 à 11.
Un atelier sans note (`NotRecorded`) ne suit pas ; il se repose par `rag3weaver-backend atelier`, une
commande que **mon mode me refuse** : c'est à Lucie.

## Revenir en arrière

- **Binaire** : `bascule-pose.sh` avec l'ancien en `.nouveau` (copier `rag3weaver-backend.avant-<x>` en
  `.nouveau`), puis le réveil.
- **Lib** : idem, avec l'id de l'ancienne lib (toujours au magasin), puis `courant-bascule.sh` vers elle.
- **Bases** : hôte arrêté, `bash ~/.local/share/rag3weaver/instantanes/<étiquette>/RETOUR.sh`. Il déplace
  les bases actuelles dans `apres-<étiquette>-<base>/` et recopie l'instantané. **Une base touchée par une
  lib d'un format plus récent est refusée par toute lib plus ancienne** : revenir de 43 à 42, c'est
  revenir à l'instantané.
- Un hôte qui tombe (SIGSEGV) laisse un `.wal` (journal d'écriture). Si le rejeu le refuse, on **garde
  l'original** (le déplacer dans `~/.local/share/rag3weaver/preuves/<date>-<base>/`) et on remet
  l'instantané. On ne l'efface jamais.

## Prouver une lib par un index (sans toucher les bases de Lucie)

`docs/optimiseur/scripts/departage.sh <étiquette> <binaire> <id de lib>` (à luciepc, sous `poste lourd`) :
1. copie reflink neuve de `~/.cache/rag3db-locks/coupure-12h06/code-lourd` (copie saine, 0 écart,
   prise par hotfixs le 8 nov.) dans `instantanes/<étiquette>` ;
2. réécrit le `.extensions` vers le `libvector` de la lib ;
3. copie **tout l'arbre des gabarits** (le manifeste a des chemins relatifs `../../tools/*.mmd` : copié
   seul, il est refusé à l'ouverture) ;
4. retire montre et bilans, lance un hôte d'essai 7991 sous `LD_LIBRARY_PATH` et `RAG3DB_LIBRARY_DIR`,
   puis `index confirm`, TERM, et le contrôle.

Le script s'arrête en ROUGE si l'ouverture ou l'index échoue, et ne contrôle qu'après « indexation
terminée ». **`EMBED=<adresse>`** l'embarque ailleurs que sur 7878 : **un index de mesure a son propre
service, jamais 7878**, par où passent les worktrees de Lucie (règle du 8 nov.). Le 9 nov., j'ai emprunté
le 7882 d'hotfixs.

Sans index : `docs/optimiseur/scripts/nuit-controle.sh <id> <binaire>` ouvre des copies 90 s sous la lib
neuve, puis contrôle ; on compare à l'instantané contrôlé sous l'ancienne lib (même écart = rien
d'ajouté).

## Lire la carte (GPU) et le disque

- **Qui charge la carte** : `rocm-smi --showbus --showuse --showmemuse`, où `GPU[0]` = card2 (04:00.0,
  services) et `GPU[1]` = card0 (07:00.0, écran de seat1). **L'ordre Vulkan n'est pas le même** :
  `gpu:0` = card2.
- **Quel service tient le moteur** : la somme de `drm-engine-gfx` dans `/proc/<pid>/fdinfo/*`, relevée
  deux fois à 5 s d'écart (le delta en ns / 5·10⁹ = part du temps). La VRAM d'un processus :
  `drm-total-vram`.
- **Qui sont ses clients** : `ss -tnp state established '( dport = :7879 )'`, puis
  `ps -o args -p <pid>`.
- **Lenteur d'un service** : `curl -X POST http://127.0.0.1:7878/embed -d '{"texts":["fn main() {}"]}'`,
  chronométré. Vers 20 ms, il est libre. Plusieurs secondes : la carte est tenue (le 8 et le 9 nov., par
  des e2e qui embarquent en creux, `e2e_sparse_dump` et `e2e_charge_ingestion`, ou par un banc de modèles).
- **Seat1** : `loginctl seat-status seat1 | grep -o 'drm:card[0-9]'` (card0) et
  `loginctl list-sessions --no-legend | awk '$4=="seat1"'`. Une charge de card0 = Lucie joue.
- **`docs/optimiseur/scripts/lecture-poste.sh`** (copie à luciepc) : charge, usage et VRAM des deux cartes,
  seat1. À noter à côté de toute heure ou mesure prise à luciepc.
- **Disque** : `cat /proc/pressure/io` (PSI : la part du temps où des tâches attendent le disque),
  `systemctl is-active fstrim.service`, des processus en état D sur `wait_log_commit` = btrfs bloqué. Voir
  le rapport pour le TRIM et le drop-in.

## Les procédures et les règles

- **Un push par `poste push`** : `~/.cache/rag3weaver-build/poste push optimiseur-G -- bash
  docs/optimiseur/scripts/push-docs.sh` (rebase sur master, n'accepte que mes chemins de docs, push direct
  `HEAD:master`). « Poussé » seulement après trois contrôles : `git status` propre, aucun rebase en cours,
  `HEAD` ancêtre d'`origin/master`.
- Commits : docs seules, **par chemins** (`git commit -- <chemins>`), identité `luciedefraiteur@gmail.com`
  (jamais l'adresse pro), messages en français, **sans ligne d'attribution**.
- **Jamais de push en force**, jamais de suppression de branche distante, jamais d'effacement
  d'instantané ou de base sans le mot de Lucie.
- **Ne tuer que ses propres pids** (`$!`, `ps --ppid`), jamais par nom ; `pgrep -f <mot>` se trouve lui-même.
- **`/tmp` est de la mémoire vive** (tmpfs) : jamais de target cargo ni de base dedans.
- Un `nohup` lancé dans un ssh tailscale meurt avec la session : lancer par `setsid … & disown`, puis
  vérifier par `pgrep`/`poste --qui`. À luciepc, le shell de login est **fish** : passer par
  `ssh luciepc 'bash -s' <<'EOF' … EOF`.
- Bash ne relit pas un `for` déjà lu, mais relit un script au fil de l'eau : pour changer un script qui
  tourne, écrire une copie et la poser par `mv` (nouvel inode).
- **Un refus de permission ne se contourne pas** et ne se fait pas faire par une autre session. Refus vus :
  `rag3weaver-backend atelier`, l'écriture de `task_writes`/`edit_file` dans le manifeste de Lucie,
  `git worktree remove`, `git push --delete`, l'effacement d'instantanés. Après un refus, vérifier que
  rien n'a bougé.

## Les pièges appris

- **Aucun service de plus sur card2** : à 77-82 % de VRAM avec quatre services, un cinquième met l'anneau
  gfx en reset au premier calcul (9 nov., 10 h 32, de ma faute). Lire la VRAM avant. card0 est la carte de
  jeu de Lucie : seulement sur son mot.
- Un e2e qui embarque en creux est un **lourd GPU** : pas pendant une file de pushes ni une passe de lib.
- Une lib de format plus récent rend une base illisible aux anciennes : instantané **avant** toute pose de
  lib.
- `dadbf0041` fait tomber l'hôte (use-after-free HNSW) : ne pas y revenir.
- Un « 0 écart » sur une copie qui n'a pas été indexée ne prouve rien : vérifier « indexation terminée »
  dans le journal de l'index (`~/.cache/rag3weaver/commandes/index/<n>.out`, dernière ligne).
- Le temps d'un index dépend de la file de 7878, pas du moteur (5 min ou 41 min le même jour).
- `fstrim.timer` du lundi bloque le disque de luciepc tant que le drop-in n'est pas posé.
- `essai-hote-nouveau.sh` vérifie la version de lucivy du registre (`LUCIVY_ATTENDU`, 4.3.2 le 9 nov.).

## Mes scripts (`docs/optimiseur/scripts/`)

| script | rôle |
|---|---|
| `bati-lucie-luciepc.sh` | binaire de l'hôte contre une lib (luciepc) |
| `preparer-binaire.sh` | bâti, essai et check-manifests aux deux postes, sans pose |
| `poser-binaire.sh` | repose binaire seul, luciepc ou portable |
| `bascule-pose.sh` / `bascule-reveil.sh` | pose et réveil d'un poste (aussi à luciepc dans `~/.cache/rag3weaver-build/`) |
| `essai-hote-nouveau.sh` | hôte d'essai 7991 |
| `ouvrir-bases.py` | rouvrir des backends par l'hôte 7970 |
| `outils-atelier.py` | outils de l'atelier de Lucie |
| `rendu-ide.mjs` | rendu de l'IDE par CDP |
| `departage.sh` | index complet sur une copie saine, puis contrôle |
| `nuit-controle.sh` | copies ouvertes sous une lib neuve, puis contrôle |
| `nuit-verdict.sh` | passe e2e d'une lib à luciepc |
| `guet-index.sh` | attend la fin de la phase d'index d'un run |
| `index-force.py` | index complet d'un backend de l'hôte 7970 (pas sur les bases de Lucie) |
| `repli-17a.sh` | le repli du 8 nov. sur 17a094d12 |
| `lecture-poste.sh` | charge, cartes, seat1 (luciepc) |
| `push-docs.sh` | rebase et push de mes docs (étendre la liste des chemins permis au besoin) |

Les autres fichiers du scratchpad (journaux de pose `pose-*.log`, `reveil-*.log`, rendus `rendu-*.txt`)
restent dans `/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/13068ada-ca9b-4752-8b48-bf8f40ed08a2/scratchpad/`,
en mémoire vive : ils partiront au prochain redémarrage.

## Mes tickets ouverts

- `docs/tickets/2026-11-08-le-plein-texte-de-code-lourd-compte-plus-de-documents-que-de-lignes.md` : le
  plein texte de code-lourd a plus de documents que de lignes (Scope 82 958 / 82 855) ; à la recherche et
  à lucivy.

## Mes pages précédentes

- `extension/rag3weaver/docs/optimiseur/7-novembre-2026/01-la-reprise.md` et `02-le-8-novembre.md` : le
  récit du 7 au 9 novembre (départage des écarts, poses, nuit de la lib, incidents).
