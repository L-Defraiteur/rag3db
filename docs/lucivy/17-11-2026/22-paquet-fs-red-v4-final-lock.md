# Paquet FS RED v4 — final-lock contrôlé, NON LANCÉ

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Patch groupé préparé après deux RO v3 réservées et [rouge final-lock réel](20-rouge-reel-final-lock.md).
L'ancien Qv3 reste figé/HOLD, pas de reset du seed ou de l'effectif. Aucun
ancien GO CPU/Rust réutilisé. Cette recette attend ses **deux RO totales**.

## Identités exactes et neuf copies

Source Rust **0f610f437a341f745e5936492114142ff3afedd4**, RO hex favorable,
non compilée après correction. Panel15/six négatifs/sept noms et défauts FS
inchangés. W PC propre `/home/lucied/git_workspaces/cx-lucivy-fs-admission-17nov`,
HEAD0f ; lock **dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6**.

PC Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v4`,
UID1000/dev55/inode168979687 ; nonce **500867ad66cea7eb28d662c791296d6d**.
Préflight **94967917f23abc95139c6a0668892182f459a729c108e1592bc9f9dc0a6bd668**.

Portable Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v4`,
UID1000/dev57/inode96727163, même nonce ; préflight et exporteur identiques.
Répertoire de preuves, pas un target Cargo. T unique3552 inchangé :
UID1000/dev55/inode164536077, marker5a1671. Trois anciens ELF1ee RO préservés.

| Déploiement | SHA256 |
|---|---|
| qualify-fs-admission-red.py (source-v4) | 4d37a7f3ec4a2c90bbdc4286ea1da10df394147611baed4a0bea4de38f31403d |
| admit-fs-admission-red.py (source-v4) | d50856ccf19e1be02ba2a6c6e93fe4c88c6f236e10b70e8f0d4609265348647b |
| export-fs-red-ledger.py (source-v4) | 7c58b87f74781019774a3be48b363e19f9620a979d8b026df0b118c84f56f8bd |

[Manifestes et source-pins](preuves/fs-red-v4-preparation/SHA256SUMS).
Common6439/poste79e3/helpers2fa59/0a269 inchangés, seeddbdb/source-pinsaa393.
Neuf copies PC et deux portables vérifiées par SHA/AST ; source0f propre,
lock effectif égal au seed dbdb, markerT conforme. Scope/logs/arm/ACK/backups
absents et scratch/scans vides ; outputs et reçus START/END portables absents.
Aucun import/pilote/Cargo/rustfmt/test/procscan/ELF/corpus/service en préparation.

## Couverture nouvelle de P1FINALLOCK

[Deux gardes dans les budgets](21-preparation-gardes-final-lock.md) : rehash du
lock effectif après les sept juges avant result.json, puis après Common/les
autres contrôles juste avant final-accepted. SHA comparé au seed épinglé,
valeur publiée, deadline contrôlé avant/après. Aucune acceptation sur Gitclean
ou le seul hash de la copie seed. Pas de restauration/exception sur dérive.

Référence de quiescence : Common265273/start126184561 terminé101/restes[]/
nettoyé ; descendants ET superviseur propre contrôlés fraîchement après vrai
poste. Une identité propre active ou inconnue bloque, partiel publié avant
assert ; UNKNOWN nonattribués conservés et aucune quiétude universelle annoncée.

START portable est durable avant ARM, END après résultat/cleanup connus,
dans le worker sous timeout. Le parent et l'argv exact sont vérifiés. Les
trois propositions de tests de journal et les deux de final-lock n'ont pas
encore tourné : seuls les anciens trois verts51a9/a99 restent qualifiés dans
leur portée. Aucun OS timer/kill/arrêt distant ou300s déduit des modèles.

## Borne physique et séquence conservées

La commande exacte pose timeout **AVANT Python** : TERM295/KILL300 couvre
démarrage/main/hashes/I/O/ACK/journaux/retour et cleanup local. ENDtravail
local=parentStart CLOCK_BOOTTIME+285 ; duréePC=ENDlocal−débutARM−8, round-trip
<=8 sinon aucun lancement lourd. Seules des durées passent entre hôtes.
Bail poste15s ; timeout PC inclut acquisition/retour ; ENDadmission=
min(STARTaprèsPoste+285,physicalDeadlinePC−5), Common=floor(reste)−12.
32G/HIGHmax/swap0/quota4, jobs<=min(4,affinitémoins2,quotashiérarchiques).
Aucun now+300 après ARM, nouveau scope sœur ou prolongation après compile.

Après nouveau GO exact :

1. Préflight frais/source0f/lockdbdb/UID/inodes/nonce/9SHA/markerT/oldCommon et
   identités propres. Trois anciens ELF sauvegardés RO et hashés avant build.
2. Une compilation release/offline/**locked**, deux targets FS/native, JSON
   compiler-artifact ; aucun run via Cargo. Lock octets/TOML/SHA invariantdbdb.
3. Ledger deux ELF physiques write/fsync/ready, copie portable fsync+rehash et
   ACK atomique nonce/source/SHA autonome, **avant toute invocation ELF**.
4. Deux inventaires/ignorés et ledger complet ; sept FS exacts, pas d'ignore,
   v3_ground_truth_contains présent mais corps NON exécuté.
5. Six négatifs contrôlés (empty, enum/meta/read, UTF8 invalide, panel15 non
   couvert) puis positif15couvert ; chaque négatif101 avec nom/cause/assertion/
   trace et compteurs0/1/0/6filtered, positif1/0/0/6filtered. Petits FS possédés
   seulement, aucun corpus réel ou changement de panel pour obtenir des hits.
6. Tous rehashes source/ELF/copies/seed/préflight/marker/backups, scratch vide,
   **lock effectif final dans le pilote**, Common terminé/nettoyé/restes[],
   scan frais PID/start propres, **lock effectif au dernier verdict externe**,
   retour réel poste/SSH, copie finale et reçu END sous l'enveloppe locale.

Attendu non observé : six FAIL et un PASS = bilan **sémantique RED**, pas FS
correcte. Tout ERROR/compilation/délai/provenance/lock/publication/ACK/motif/
compteur inattendu ou positif rouge refuse : STOP, aucun retry, aucun secours
humain. Préserver les partiels et constater la fin réelle avant toute suite.

## Commande candidate — NON LANCÉE

```bash
PYTHONDONTWRITEBYTECODE=1 /usr/bin/timeout --signal=TERM --kill-after=5s 295s /usr/bin/python3 \
  /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v4/export-fs-red-ledger.py \
  --_physical-watchdog-child \
  --preflight /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v4/preflight.json \
  --output /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v4/elf-ledger.json \
  --launch-log /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v4/launch-ssh.log
```

Sans-B pour le parent exact, shell PC bash/cwdQ/noclobber, V3/ABSENT_OK/flags
rust/wrappers hérités refusés. Coût300 proposé, pas garanti : ancien refus
compile vers30s, link/juges pas mesurés. Les probes CPU de21 et le runtime
ont chacun besoin de leur **GO distinct C7** ; deux RO totales avant GO300.
Source0f non compilée, six rouges FS/positif toujours NONOBSERVÉS. Natifs A/B/
core/neufHOLD/ULP7.2/PR42/main/tag/publication restent exclus. Pas de fix de
gate FS avant quatre familles de rouges Rust réellement observées.
