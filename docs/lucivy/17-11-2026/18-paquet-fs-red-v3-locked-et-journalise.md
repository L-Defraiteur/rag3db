# Paquet FS RED v3 — locked, reçus physiques, NON LANCÉ

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Le refus E0277 source8b et la restitution réelle sont archivés3759. Le correctif
hex **0f610f437a341f745e5936492114142ff3afedd4** a sa RO source favorable,
compilation toujours NONvérifiée. Préparation du nouveau paquet seule autorisée.

## Pins et identités, vérifiés sans exécuter le pilote

PC Q : `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v3`,
UID1000/dev55/inode168958729, nonce **ecf77bd44c67d49b27b3cbcaf2373d18**.
Préflight **e986a9ec92086c46c1df5298ef143d087a4e0bf91b2c575dcb64e460d7de952d**.

W PC propre : `/home/lucied/git_workspaces/cx-lucivy-fs-admission-17nov`, branche
propre du même nom, avancé **fast-forward** de8b à0f. Aucun commit nouveau
sur PC, code déjà poussé depuis le portable. Lock effectif **dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6**
préservé, jamais remis à ce719. T unique3552, UID1000/dev55/inode164536077 et
marker5a1671 restent identiques. Aucun ancien Q réarmé ou modifié.

Portable : `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v3`,
UID1000/dev57/inode96560015, même nonce, préflight et exporteur identiques au PC.
[Neuf copies, manifestes et marqueurs](preuves/fs-red-v3-preparation/SHA256SUMS).

| Rôle | SHA256 |
|---|---|
| Pilote v3 déployé qualify-fs-admission-red.py | 43d6c14bed4e541b3c8b6645ad32e47be26dea1c3e233d79ac7626513ece4c53 |
| Admission/final PC inchangée | 166d3ff6ec640539eadeb0fe82d867fa3a6d539c5e321d39108681f2889a8b03 |
| Exporteur v3 déployé export-fs-red-ledger.py | ec619e418b4d0c8eabddadc717a01c728f9cdfc58e7c86315e0415f7688338d7 |
| Lock seed invariant | dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6 |

Common6439/poste79e3/helpers2fa59/0a269 inchangés. Le pilote et l'exporteur
déployés sont les octets Git des sources suffixées-v3, pas ceux du paquetv2.
Source-pins et cinq SHA source0f relus. SHA/AST/Git/markerT/lock/recette seulement :
**aucun Cargo/rustfmt/test/pilote/procscan/ELF/corpus/service pendant préparation**.
Scope/logs/arm/ACK/backups absents, scratch/scans vides ; au portable outputs
et physical-start/end absents. Les reçus réels ne seront produits que sous GO.

## Changements depuis le paquet closv2

- Source0f : uniquement affichage hex standard du digest sha2.11 et deux
  références dans le positif. Panel15, six négatifs/sept noms/capture/consumer
  inchangés ; les défauts d'admission restent présents avant les rouges Rust.
- Seed postérieuredbdb **invariante**, `cargo test --offline --locked --release`.
  Après compile, octets/SHA/TOML doivent rester identiques au seed, direct sha2
  déjà présent. Aucun delta « ajouter sha2 » rejoué et aucune résolution réseau.
- Référence fraîche de quiescence : Common **265273/start126184561**, terminé101,
  erreur null/restes[]/nettoyé. Le pilote contrôle aussi son identité vivante
  directement : enfants.json contient ses23 descendants, **pas le superviseur**.
  Présence de cette identité ou métadonnées propres inaccessibles bloquent ;
  les observations/UNKNOWN sont publiées avant l'assertion. Pas de proc réel
  interrogé pour cette préparation, le contrôle attend l'admission après poste.
- [Reçus physiques START/END](17-preparation-recus-physiques-portables.md) dans
  le worker surveillé, avant ARM/après cleanup. Sourceec619/probesd13 sont
  préparés mais **aucun vert CPU de cette nouvelle version n'est revendiqué**.
  Les trois verts antérieurs restent limités à51a9/a99.

## Budget physique conservé, aucun reset

Shell timeout **avant Python**, TERM295/KILL300 ; worker vérifie parent
timeout/argv/PID/ticks du même hôte et pose ENDtravail=parentStart+285. START
est écrit avant lecture du manifeste/ARM, END après résultat/cleanup, dans
cette même portée. Les I/O de journaux, hashes, copies et ACK sont surveillées.

Durée transmise PC = ENDlocal−débutRPCARM−8 ; round-trip complet<=8 avant tout
lancement lourd, sinon STOP. Le PC ajoute cette **durée** à son propre now,
sans comparer les horloges des hôtes. Bail poste15s. Son timeout physique
inclut acquisition/retour poste ; ENDadmission=min(STARTaprèsPoste+285,
physicalDeadlinePC−5), Common=floor(reste)−12. Aucun reset après compile.
32G/HIGHmax/swap0/quota4, jobs<=min(4,affinitémoins2,quotashiérarchiques).

Le coût proposé reste300 physiques : compilation ratée précédente arrêtée
environ30s après admission, bibliothèques désormais en cache dans le même W/T.
Link et tests non mesurés, aucune réussite future garantie ni prolongation.

## Déroulé proposé après nouveau GO exact distinct

1. Fresh préflight après vrai poste : source0f propre/lockdbdb/empreintes/nonce/
   UID/inodes/markerT, ancien Common et identités propres/attribuées. UNKNOWN
   nonattribués conservés, aucune quiétude universelle. Sauvegarder les trois
   ELF historiques1ee en RO seulement APRÈS GO, avant compilation ; SHA exigés.
2. Une compilation release/offline/**locked** sans run de deux targets FS/native,
   JSON compiler-artifact. Échec compilation/lock/provenance/temps = refus,
   jamais rouge d'admission. Aucun changement de source pendant la passe.
3. Ledger deux ELF physiques écrit/fsync/ready, copie portable fsync/rehash,
   ACK atomique source/nonce/SHA par l'exporteur déjà actif. **Avant tout ELF**,
   même inventaire. Aucun message ou ACK humain de secours.
4. Inventaires/ignorés et leur ledger complet, sept noms FS exacts sans ignore,
   vrai v3_ground_truth_contains présent mais corps NONsélectionné.
5. Six négatifs : empty, enum/meta/read, UTF8 invalide, panel noncouvert ; puis
   positif couvrant les15. Chaque rc101 doit porter nom/cause/assertion/trace
   et0pass/1fail/0ignored/6filtered. Positif1/0/0/6filtered. Rien sur un corpus réel.
6. Rehash deux ELF, neuf copies, source, lock, préflight, marker et backups RO,
   scratch vide ; Common terminé/restes[]/nettoyé, scan frais ownPID/start,
   retour effectif poste/SSH, copie finale sous END local, reçu END durable.

Résultat attendu non observé : six tests FAIL et un PASS, bilan **sémantique
RED**, pas admission FS verte. Pilote0 signifie observation de ces échecs
attendus ; tout ERROR/rc/motif/trace/compteur inattendu, skip, positif rouge,
publication/ACK/lock/délai/provenance refuse et STOP sans retry. Préserver
partiels, constater les fins réelles avant toute suite. GOv2 définitivement clos.

## Commande complète candidate — NE PAS LANCER

```bash
PYTHONDONTWRITEBYTECODE=1 /usr/bin/timeout --signal=TERM --kill-after=5s 295s /usr/bin/python3 \
  /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v3/export-fs-red-ledger.py \
  --_physical-watchdog-child \
  --preflight /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v3/preflight.json \
  --output /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v3/elf-ledger.json \
  --launch-log /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v3/launch-ssh.log
```

Sans-B pour correspondre exactement au parent contrôlé ; flagworker obligatoire.
Shell PC bash/noclobber/cwdQ hors W/T, flags V3/ABSENT_OK/rust/wrappers refusés.
**Deux RO du couple total et GO300 C7 distinct requis avant cette commande.**
Les probes START/END proposés ont leur propre revue/GO CPU60 avant exécution.
Natifs A/B/core/neufHOLD/ULP7.2/PR42/main/tag/publication restent exclus.
Aucun timer expiré/killOS/cleanup distant/300 mesuré ni gate FS corrigée ne
sera déduit des tests CPU ou du simple rc0 du pilote.
