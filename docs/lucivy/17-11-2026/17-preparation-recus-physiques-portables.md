# Reçus physiques portables — code proposé, non exécuté

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
C7 autorise la préparation corrective après refus3759 ; ancien GOv2 clos.
Le début/fin portable n'était pas journalisé dans le refus précédent. Cette
préparation ajoute les reçus **dans** le worker déjà sous son parent timeout,
sans reconstruire un journal de la passe ancienne.

`outils/export-fs-red-ledger-v3.py` SHA
**ec619e418b4d0c8eabddadc717a01c728f9cdfc58e7c86315e0415f7688338d7**.
Recette toujours shelltimeout AVANTPython, sans-B, flag enfant contrôlé et
parent exact. Les domaines d'horloges et le transfert conservateur de durée
ne changent pas : parent local CLOCK_BOOTTIME/ticks, ENDparent+285, wireR−8,
ARMroundtrip<=8 avant lancement distant, CommonMIN sur PC.

- physical-start.json, create_new/fsync fichier et parent **avant ARM** :
  parentPID/start/commande, PID worker, observation, échéances travail/TERM/KILL,
  propre SHA et argv. Aucune capacité ni environnement privé recopiés.
- physical-end.json, create_new/fsync dans finally **après cleanup** : même
  parent/échéance, observation, source/nonce validés, résultat ou classe
  d'exception, classe d'erreur cleanup, enfants locaux connus et leur code.
  Il ne prétend pas une quiétude universelle ni l'arrêt de processus inconnus.

Les I/O de reçus sont sous le même watchdog physique, avec vérifications
avant/après publication. Un échec de publication refuse la passe ; un blocage
peut laisser un reçu absent/partiel et ne doit pas être repeint en fin propre.
END est une observation dans le worker avant son retour, pas une mesure d'un
kill OS ou de l'instant exact où le noyau récole le processus. Aucun timer
expiré/cleanup distant n'est qualifié par lecture.

## Probes de raccord et de journal, proposés seulement

`outils/test-fs-export-boundaries-v4.py` SHA
**d13f6b479d223f4fb1861cdd1fc65bcd02eb7f87ccfe43b8fe4465bd6e0eadbd**.
Même ordre/durée/erreur I/O que les trois verts51a9/a99, avec manifeste de
fixture0f/Qv3 et vérifications des deux vrais reçus écrits par le programme.
ARM simulé exige START existant et END absent. Les assertions portent aussi
sur SHA, parent, échéance conservée, contexte source/nonce, erreur attendue et
enfants simulés réapés. Les parents/transport/processus sont des doubles ;
aucun clock virtuel, proc/SSH/signal/processus réel, Cargo, target ou corpus.

[Préparation CPU figée](preuves/journal-v3-cpu-preparation/preparation.json) :
Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-export-journal-17nov-v1`,
UID1000/dev57/inode96524117. Deux SHA et syntaxe AST vérifiés seulement.
Ancienne passe verte15cee4 reste source51a9/a99 ; **aucun nouveau test joué**.

Proposition après revue et GO CPU60 distinct : une invocation TERM55/KILL60
du harnaisv4, sourceec619/SHA exact, case all, proof neuf. Attendus trois PASS,
zéro FAIL/ERROR/skip. Tout écart STOP, aucun retry. Le cas315 reste NONJOUÉ,
aucun vieux GO CPU ou Rust n'est réutilisé. La recette du couple complet et
ses nouveaux pins sont [18](18-paquet-fs-red-v3-locked-et-journalise.md).
