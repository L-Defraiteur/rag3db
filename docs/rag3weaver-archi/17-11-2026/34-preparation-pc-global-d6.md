# Préparation PC global+D6 : code, T0 et copies, aucun lancement natif

APP f1e8a8d502614d10f7cdb40eaea330f08b5d3f37.
PILOTE PC figé 229b397d8d1cb28b8ac3075479e49a1ab72b400b, code Python
BYTEidentique au pin CPU a0d74de57fe8bec364372264e4520b0b736a28a8 ; seul ajout
documentaire des résultats41verts. Le présent rapport est publié après préparation,
sans déplacer le contrôleur PC de229b. Ticket existant
[documentaire/Vertex](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md)
toujours ouvert. Recette32/33 et limites des RO restent applicables.

## Les preuves CPU précédant cette préparation

GO C7-ARCHI-CPU-GREEN-A0D74-17nov60 consommé UNEfois :41OK/0FAIL/0ERROR/0skip,
18+16+7, unittest0,062s/enveloppe0,286407600s/rc0/0retry. Preuves
R/cx-archi-cpu-green-a0d74-17nov-r1 :3fichiers personnellement réhashés sur
LOCAL+PC,0écart ; manifest3213a6842a455c7837f10005216416b77b0ad37a3602ed5687e29d8d86c3d957.
Mémoire/Banc RO preuves favorables sans rejeu. AncienERROR3839/36OK1ERROR
+4nonjoués reste intact. Rouge3a786/1FAIL0ERROR et publicationa797 AVANTce
rouge restent décrits honnêtement : pas de red-before-fix historique inventé.
Ces41juges sont des simulations CPU, pas D6/Vertex/SDK/native/service qualifiés.

## Préparation Git et gel du cache

Nouveau contrôleur PC détaché et propre :
`~/git_workspaces/rag3db-cx-archi-global-pilot-r2`, pin229b, gitlink
bd133850f89d6b12a3ca2b2f6ce238785ed37e3a initialisé propre.

Avant transition, source742 propre et72preuves réhashées, autorité ancienne
TERMINE/portéeVERT/restes[]/busabsent,132observations owned désormais absentes
et0unknown-owned. Archive physique source742
`~/git_workspaces/rag3db-cx-archi-doc-742-proof-r1`, HEAD détaché7425375d66a64b0782672225c59516f048682b00,
administration parent distincte, sous-module privé détaché àbd133 et administration
distincte. Tracked modes/OID/contenus identiques au source742 avant bascule.
Les archives plus anciennes et preuves72 restent inchangées.

Le worktree canonique `~/git_workspaces/rag3db-cx-archi-doc-supervisor-r1`
garde son chemin : branche NEUVE cx-archi-global-source-pc-r2, APPf1e8,
propre/gitlinkbd133 propre. Aucun retrait/cleanup/archive supprimée.

R/cx-archi-global-preparation-r1 contient5JSON. T0 avant/après Git STRICTEMENT
égal,13416entrées session/5246entrées enfantactif ; identité T : device55,
inode164550643,UID1000, même chemin
R/target-cx-archi-pas-va-au-bout-r2/native-doc-ea670-v1, même marqueurC.
Aucun appel garde_target ni Cargo ni mutation de target dans cette transition.
SHA T0avant c983f535d32e4e3b18365dda12e3d37c990591169d31891fd7c11fb570f32ab7 ;
archive742 JSON a476af5f91be55918f41d4b419169ed1a7738479ddc61d45f4848ed93313c014.

## Configuration et paquet exacts

Préparation --prepare terminée code0, aucune autorité créée :

- Q R/cx-archi-global-controller-r1 :21copies exécutables/scripts/provenance
  réempreintées, configuration0e6278fc874ee3d9723e0388ebffd4e2b469a2dc5887d7d0e3a610356fe136c1.
- P R/cx-archi-global-v1 VIDE ; F R/pg8 ABSENT ; task-targets ABSENT ;
  Q/launch.json ABSENT. Aucun poste réservé/lancé.
- Paquet R/cx-archi-global-package-r1 sur LOCAL+PC :29fichiers/19884210octets,
  manifeste [35](35-paquet-global-pins-manifest.json),
  SHA f5c5eb6c55f8775f1e8f45961fe351a975f58833aca52f55b9b331ae57663d77,
  pins63723b1c7cdaeb238721c9ca7e95bcbd180304f25bd94f9778a64a42696695a1.
- Tar1a2256e7689e5ad98e50ddd5e20d4dd654a3950b03ed3bb66fc0f21649c36e72.
  Chaque entrée réhashée sur chaque hôte :29/29,0écart. Aucun reçu/jeton ni
  fichier de credentials copié ; configuration contient des chemins et SHA publics.

Outils pointés et hashés dans configuration, lecture de `rustup which cargo`
uniquement : Cargo effectif `/home/lucied/.rustup/toolchains/1.98.1-x86_64-unknown-linux-gnu/bin/cargo`,
SHAda77c8b33849312255ccde3179198ada4c8deb370488d050286146b1d1b27e14.
Ce n'est pas l'ancien stable historique ; aucun coût de build n'est extrapolé.
Python3.14/bash/timeout/git/rustup/zstd également pinés. Aucun Cargo/rustc réel
n'a été exécuté. Canonique2150 et Common6439 vérifiés par SHA, pas exécutés.
SDK59 vérifié en lecture de fichiers, jamais exporté/chargé.

## Ce qui reste avant une exécution

RO TOTALE couple exact APPf1e8/PILOTE229b + paquet/configuration/T0/copies,
puis GO C7 natif distinct avec créneau exclusif et ACKs ressources frais à leur
premier usage. La préparation n'accorde aucune ressource ni ACK Vertex/7878/7879/PG.
Borne proposée Common7200MAX/TERM7440+KILL15 physique7455, MESURE800%/FILS>=8,
ledger2postes avant runtime et deadline finale restent du code NONQUALIFIÉ natif.
Les scans frais sous vrai poste, garde réelle, F8/build/ELF/SDK/D6/canonique,
quiescence/finalisation appartiennent à cette future qualification, pas au paquet.
Ne pas lancer --launch ou réutiliser un ancien GO CPU/natif sur ce pin.
