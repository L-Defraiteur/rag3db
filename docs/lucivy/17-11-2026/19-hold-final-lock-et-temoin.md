# HOLD final lock — contre-exemple préparé avant correctif

[Même ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Deux RO totales regroupées sur eb2f/v3 : Banc relève P1FINALLOCK, confirmé par
Recherche qui révise son premier avis favorable en **réservé sur ce point**.
C7 consigne le HOLD521411c889. Aucun drift réel du lock n'est inventé.

Le pilote43d contrôle le lock effectif avant/après compile ; après les sept
juges il rehash source/Git/copies/seed/préflight/backups, mais pas W/Cargo.lock.
L'admission166d fait de même avant final-accepted. Le lock est ignoréGit :
sourceclean et seedcopié inchangé ne prouvent pas son invariance tardive.
Le contrat18étape6 exige ce dernier contrôle **dans END**. Pas d'ancienne
contradiction tracked/Git, pas d'exception large ni de reset de lock.

Qv3/source0f/seeddbdb/T3552 et anciens ELF restent figés, non exécutés. Les
autres raccords sont reconnus par les deux lectures ; cela ne lève pas P1.
Les probes de journaux restent préparés, **sans GO**, arbitrables séparément.
Aucun GO300 ni test Rust FS acquis.

## Témoin court du vrai dernier consommateur

`outils/test-fs-final-lock-entry.py` copie l'admission166d **octet pour octet**
et exécute son vrai __main__/main via runpy, dans un TempDir privé. Le manifeste,
son SHA, la contrainte cwd, les budgets, fichiers résultat/final et hashes de
seed sont réellement traités par ce main. Aucun code source n'est réécrit.

La préparation/pilote/Common/Git et leurs métadonnées de processus sont des
modèles explicites pour atteindre ce dernier raccord, pas une qualification
de ces étapes. Common ne lance ni compilateur ni sept juges ; il produit un
résultat **fixture_only**, puis remplace tardivement la version du paquet du
Cargo.lock effectif synthétique1.0.0→2.0.0, TOML toujours valide. Le seed reste
inchangé. Gitclean est simulé pour représenter le fichier ignoré. La lecture
proc du seul PID modèle est interceptée ; aucun vrai processus/réseau/SSH/
proc/ELF/target/corpus/service, pas de clock virtuelle.

Setup exigé avant assertion discriminante : lock initial valide, seed intact,
Common modèle revenu, source Git et seed rehashés par le vrai dernier main,
preuve finale de ce main écrite. L'observateur n'ajoute pas de faux événements
de hash après coup : ses propres digests de constat sont séparés des spies.

Deux cas dans une seule future invocation CPU60 :

- Mutation tardive : le vrai main166d devrait encore publier final-accepted,
  donc le test doit **FAIL précisément** sur cette acceptation malgré le lock
  effectif modifié. Aucune erreur de fixture n'est un rouge admissible.
- Contrôle positif : lock inchangé, ce même chemin doit accepter. Il écarte
  un futur refus systématique et vérifie que le harnais atteint réellement
  le verdict. Attendu PASS sur166d.

Attendu total non observé : **1 FAIL, 1 PASS, 0 ERROR**, rc1. Observations
persistées avant les assertions. La source166d est inchangée ; aucun guard
final effectif ajouté avant publication/observation de ce contre-exemple.

[Préparation et deux pins](preuves/final-lock-entry-preparation/preparation.json).
Après RO et **GO CPU60 distinct** seulement : timeout TERM55/KILL60, harnais
figé --admission <copie166d> --proof <dossier neuf>. Tout écart de setup/motif/
rc ou timeout : refus, aucun retry. Bruts/observations/temps/rc/source à garder
deux postes avant correction. Ce témoin ne prouve aucune exécution Rust/OSkill.

Suite après rouge : contrôle du lock **effectif** contre le SHA seed au dernier
verdict dans END, puis nouveaux pins/RO totales et GO300 distinct. Aucun
ancien paquet ou GO ne sera réactivé pour contourner le HOLD.

## Pins exacts du témoin, toujours NONJOUÉ

Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-final-lock-entry-17nov-v1`,
UID1000/dev57/inode96685390. Admission166d et témoin
**ffdddc1f57a49f285a708c3dd6db99cf81d29f6496ce967d24c2d0bcdc3738a9**
figés ; proof/log absents, SHA/AST seulement contrôlés. Aucun import/runpy/test.
