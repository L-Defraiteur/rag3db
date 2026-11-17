# Témoin court du raccord CLI — proposé, non joué

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
C7 n'admet pas la passe315 proposée ; le témoin300 reste publié et **NONJOUÉ**.
Le code export047111 et son ancien paquet sont inchangés. Aucun correctif.

`outils/test-fs-export-watchdog-entry.py` exécute la **vraie entrée __main__**
via runpy sur une copie octet pour octet047111. Vrais argparse, lecture du
préflight, SHA du propre script et JSON. Aucune transformation AST du code.
Le préflight et les chemins de sortie sont privés et jetables, pas l'ancien Q.

subprocess.run est remplacé pour simuler seulement la réponse ARM ; ni le
transport ni le corps distant ne sont exécutés. Popen est remplacé et son
utilisation serait une erreur du témoin. Une sentinelle intercepte Path.open
au mode d'écriture du journal de lancement, **avant l'ouverture/écriture**.
Elle arrête le vrai CLI à cette frontière postARM ; pas de SSH/réseau/processus
enfant/scans de processus/target/ELF/corpus/service, pas de clock virtuelle.

Le raccord physique attendu est un execv de `/usr/bin/timeout` avant cette
frontière, avec TERM, kill-after5 et durée totale<=300. Le double d'execv
vérifie les arguments et entre dans le vrai enfant CLI, sans lancer de timer.
Cela compose une primitive existante, pas une troisième primitive de relais.

Sur047111, les contrôles de setup doivent être vrais : vrais args/préflight/
hash/JSON atteints, ARM simulé, une frontière écriture atteinte, aucun fichier
de sortie créé. L'assertion discriminante exige ensuite que le premier événement
soit physical-watchdog-wrapper. Le code actuel donne arm-transport-mocked puis
post-arm-write-frontier : **FAIL d'ordre/raccord attendu**, pas ERROR de fixture.
L'observation est gardée avant cette assertion.

Ce témoin ne mesure pas300s et **ne qualifie aucun timer ou kill OS**. Un futur
vert de ce raccord ne prouverait que l'ordre et les arguments du branchement.
Le budget restant conservateur vers PC et le nettoyage des enfants restent à
préparer/revoir séparément avant nouveau paquet ; aucun GO Rust n'en découle.

Source du témoin à publier AVANT correction. Recette CPU proposée sous borne
extérieure60s, seulement après RO et GO CPU60 distinct :

```text
timeout --signal=TERM --kill-after=5s 55s python3 test-fs-export-watchdog-entry.py
  --exporter <source047111 exacte> --proof <dossier neuf privé de preuves>
```

Attendu non observé : un test exécuté, un FAIL sur l'ordre watchdog/postARM,
zéro ERROR, aucun run du vrai couple PC/portable. Le journal brut et
observation.json seront archivés. Ancien Q/source8b/T3552 restent intacts.

## Préparation exacte, toujours sans exécution

Q privé : `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-export-watchdog-entry-17nov-v1`, UID1000/dev57/inode96230440.

Témoin SHA **be4913846a3d7b3f635306c426ddcc3ad1f4d628a6a6939d954b45c8eff0f836** ;
source047111 copiée inchangée. `proof/` et `witness.log` absents. Seuls
SHA/syntaxe AST ont été contrôlés ; aucun import/runpy/test encore joué.
[Marqueur](preuves/watchdog-entry-preparation/preparation.json).
