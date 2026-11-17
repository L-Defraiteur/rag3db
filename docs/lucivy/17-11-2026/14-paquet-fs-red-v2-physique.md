# Paquet FS RED v2 — préparation exacte, aucun Cargo lancé

[Ticket FS ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Préparation autorisée par C7 après les [trois verts CPU](13-trois-raccords-cpu-verts.md).
Ils qualifient uniquement raccords/arguments/décisions simulés. La source
Rust RED reste **8b9435363dbd8ec0e3973f2368b641189f0726eb**, panel15 et défauts
conservés. Aucun correctif d'admission anticipé, aucun natif/corpus réel.

## Identités et pins

PC Q neuf : `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v2`,
UID1000/dev55/inode168935806, nonce **e356dfe059756ae8358eab49872ecbb9**.
W PC propre : `/home/lucied/git_workspaces/cx-lucivy-fs-admission-17nov`,
branche `cx-lucivy-fs-admission-17nov`, HEAD8b94353. T unique inchangé :
`/home/lucied/.cache/rag3weaver-build/cx-lucivy-target-16nov`, nonce3552,
UID1000/dev55/inode164536077, marker SHA5a1671d22dc784bec78f60cf25c8b3003acc5f21e28290f05d0dfc22e0c394df.

Préflight **39ccf66b3f503dbe4f535b435ea3f5dc539794230ec4f8b72a491060ed30e4f4**.
[Préparation et neuf copies](preuves/fs-red-v2-preparation/SHA256SUMS).

| Rôle | Source/pin |
|---|---|
| Pilote FS RED | a5c74663cb2b783041fe4c76e80d2b75dcac1773521f99629b4d1e63d101c735 |
| Admission/final PC | 166d3ff6ec640539eadeb0fe82d867fa3a6d539c5e321d39108681f2889a8b03 |
| Exporteur physique v2 | 51a9b1e2cb6f3048580588ebc9dfa6d837dadfc2a011d2b78d99f4e380b268f8 |
| Lock seed | ce719087307bcafac606ca1cba2e9942f0fb68fbd75311ae47df135b90c9e6ea |

Common6439/poste79e3 et deux helpers restent inchangés. Le nom de déploiement
`export-fs-red-ledger.py` porte les octets de la source Git
`outils/export-fs-red-ledger-v2.py`, pas ceux du047111 historique.

Portable Q neuf : `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v2`,
UID1000/dev57/inode96410663, même nonce. Exporteur51a9 et préflight39cc copiés
exactement. Répertoire de preuves, pas un target Cargo.

Contrôles faits : SHA/AST uniquement, source/propreté Git, source-pins,
provenance T et neuf copies, seed lock et état historique Common1021297/
start119982634 terminé0/restes[]/nettoyé. Aucun scanproc ou lecture ELF.
Sur nouveau Q : scope/logs/arm/ACK/sauvegardes ELF absents, scratch/scans vides.
Ancien Q v1/047111, preuves rouges/ERROR/verts CPU et T/source8b non consommés.

## Une borne physique, deux rôles, aucune intervention humaine

L'entrée **exacte** ci-dessous pose timeout **avant le lancement de Python** :
TERM295/KILL300 couvre l'interpréteur, les imports, le main, les I/O, ACK,
copie finale, retours et cleanup local. Aucun now+300 après ARM. Le worker
vérifie son parent timeout exact, ses arguments, son PID et ses ticks de début
dans le même domaine local CLOCK_BOOTTIME ; les accès parent seront réels
uniquement dans la future passe autorisée, pas pendant cette préparation.
END de travail local = début parent +285 ; marge avant les signaux physiques.

Au début RPC ARM, duréePC = ENDlocal−débutRPC−8 ; le round-trip entier doit
prendre<=8s. Un ARM lent/échoué interdit le lancement lourd, sans réessai.
La durée est ajoutée au now monotonic **du PC**, pas une échéance portable
comparée à son horloge. Pour un ARM réussi, ce transfert est conservateur.
Le bail d'acquisition poste reste15s après ARM, jamais un droit de prolonger.

Sur PC, l'enveloppe physique inclut acquisition et retour poste. END admission
= min(STARTaprèsPoste+285, physicalDeadlinePC−5). Common reçoit floor(reste)−12,
avec réserve pour verdict/scan/final. Les éventuels délais de compilation,
copie/ACK et attestation consomment ce même budget. Aucun scope sœur ni reset.
Plafonds effectifs :32G/HIGHmax/swap0/quota4 ; jobs min(4,affinitémoins2,quota).

La validation300 physique reste une proposition de coût fondée sur le T déjà
chaud (compilations historiques32,20s et3,37s). Le nouveau graphe devdep et le
chemin W peuvent invalider du cache ; aucune durée ni réussite future garantie.

## Séquence Rust proposée, sans corpus réel

1. Préflight frais après vrai poste, source/copies/UID/inodes/nonce/marker,
   ancien Common et scan owned/attribué ; UNKNOWN nonattribués conservés.
   Sauvegarde RO des trois anciens ELF1ee seulement APRÈS GO, avant écrasement.
2. Une compilation release/offline sans run de `test_fs_fixture_admission` et
   `test_sfx_v3_ground_truth`, via JSON compiler-artifact, jobs bornés, même T.
   Lock avant/après archivé : seul ajout direct sha2 core, versions/checksums
   inchangés. Tout autre delta ou cache manquant refuse avant invocation ELF.
3. Ledger des deux ELF avec chemins et SHA physiques, write/fsync puis ready.
   Le lanceur/exporteur déjà actif copie sur le portable, fsync fichier/parent,
   **rehash de la copie**, puis ACK atomique source/nonce/SHA. Aucun ELF, même
   --list, avant cet ACK ; aucun collègue humain attendu pendant la fenêtre.
4. Deux inventaires/ignorés et leur ledger complet avant les juges. Présence
   du vrai `v3_ground_truth_contains` exigée ; son corps n'est pas sélectionné.
5. Six juges négatifs sur petits FS possédés : empty, IOenumerate, IOmetadata,
   IOread, UTF8 invalide, panel15 noncouvert ; puis positif couvrant panel15.
   Les six doivent être des FAIL attribués par nom/cause/assertion/trace et
   compteurs0/1/0/6filtered. Positif1/0/0/6filtered. Pas de « six rc101 donc OK ».
6. Rehash physiques/source/copies/préflight/lock et anciennes copies RO,
   scratch vide. Common réellement terminé/nettoyé/restes[], scan final owned
   frais et absence PID/start propres avant acceptation ; UNKNOWN conservés.
   Attente retour réel poste/SSH, export des preuves finales sous END local.
   Le nettoyage local est suivi dans finally, pas supposé après une erreur.

Sur six défauts attendus observés, pilote rc0 mais result.json annonce **six
tests échoués et un passé**. Tout ERROR/infrastructure/délai/provenance/lock/
publication/copie/ACK/positif rouge/mauvais motif : STOP, pas rouge attendu,
pas retry ni ACK humain de secours. Preuves partielles conservées. Constater
les fins réelles avant toute nouvelle passe ; une enveloppe expirée ne vaut
pas une preuve de cleanup ou quiétude universelle.

## Commande candidate — NON LANCÉE, nouveau GO requis

La variable d'environnement évite les pycache ; ne pas ajouter `-B` à la
commande enfant, car le worker vérifie l'argv exact de son parent.

```bash
PYTHONDONTWRITEBYTECODE=1 /usr/bin/timeout --signal=TERM --kill-after=5s 295s /usr/bin/python3 \
  /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v2/export-fs-red-ledger.py \
  --_physical-watchdog-child \
  --preflight /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v2/preflight.json \
  --output /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v2/elf-ledger.json \
  --launch-log /home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-export-17nov-v2/launch-ssh.log
```

Shell PC explicitement bash/cwdQ/noclobber, flags V3/ABSENT_OK/rust/wrappers
hérités refusés, pas de nouveau target. Ce paquet doit recevoir sa **RO du
couple complet**, puis GO exact distinct avant cette commande. Les verts CPU
ne qualifient ni timerOS/kill, cleanup distant/300 expirés, ni admission FS,
read-once/sink/flags corrigés, ni natifs A/B/core complet/neufHOLD/ULP7.2/PR42.
Aucun main/tag/publication/fusion autorisé. Aucun correctif de gate avant les
rouges Rust effectivement observés sur les quatre familles annoncées.
