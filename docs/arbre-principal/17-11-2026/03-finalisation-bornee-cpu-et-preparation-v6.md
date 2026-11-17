# Finalisation bornée : CPU et préparation v6

Source exécutable et Qv6 : **6715c17ed98ca708edeaecfe068d9eb881b6f722**.
Ce rapport est documentaire ; la source préparée reste ce SHA exact.

## Rouge puis vert CPU autorisés séparément

Sous C7-CYCLE-FINAL-RED-D31-17nov60, source propre
d31d3e57496945fdd13637bd6b933270240dc823 : unique invocation du vrai tail de
compose avec fsync franchissant END. Un test, 1 FAIL (`0 == 0`), 0 ERROR,
0.006s unittest, processus rc1 ; monotonic local 870811.561708173 à
870811.686078058. Quatre autres fonctions non jouées à cette étape.

Après publication du correctif, sous C7-CYCLE-FINAL-GREEN-6715-17nov60 :
unique invocation `python3 -B -m unittest -v -f test_lifecycle_finalization`,
timeout TERM59s/kill1s, PYTHONPATH des coordinations 16 et 17 novembre.
Cinq fonctions PASS, 0 FAIL/ERROR/skip, 0.038s unittest, processus rc0 ;
monotonic local 871156.943776444 à 871157.128687742.
Ce sont cinq juges distincts, six invocations au total avec le rouge initial.

Les deux archives hors WT, présentes sur les deux hôtes sous
`~/.cache/rag3weaver-build/`, contiennent chacune stdout, stderr, reçu et manifeste :

| Archive | SHA256 du manifeste |
| --- | --- |
| cycle-final-red-d31-20261117-01 | d098d0a93cf366e8c0a6c70c3ad054afe9efaa7237d31bc6b5558482231fd846 |
| cycle-final-green-6715-20261117-01 | 559e5358721fccebbd61bc411ce90e5d74b178dd00ba91ca55846bcb909a7f1f |

Les trois fichiers référencés par chaque manifeste ont été réempreintés sur
le PC après copie : taille/SHA, zéro écart. Horloge et fsync sont injectés dans
la fixture ; fichiers temporaires réels, aucun processus/service/DB natif.
Late-fsync est le contre-exemple causal ancien ; late-link est le contrat
atomique futur, maintenant exercé dans le correctif. Aucun lien tardif ancien
n'est revendiqué.

## Correctif local et limites

`lifecycle.finalize_composed` réutilise `grant_watch_actor.write_frame` existant.
Deadline effective et premier rouge sont contrôlés avant publication,
avant lien exclusif et après file/directory fsync et nettoyage, juste avant
retour positif. Un refus conserve le premier rouge et reclassifie son propre
composé RED, rc1. Le remplacement exige les octets canoniques attendus :
égalité de contenu, sans garantie de provenance inode contre un acteur
hostile du même UID. Aucune nouvelle primitive générique, aucun changement
de qualify.save, de l'acteur ou du contrôleur commun.

## Préparation PC uniquement métadonnées

WT propre `/home/lucied/git_workspaces/rag3db-cx-arbre-cycle-finaliseur-borne-20261117-r1`,
HEAD exact source6715 ci-dessus. Q :
`~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-control-v6`.
Préparé SHA256 **7bf5cd553e2836b20446f77200647991f0884c1a62f2a1d015e2ab9c3c9cf9c0**.
Vingt-deux copies contrôlées contre leurs SHA prepared ; guard complet rc0.
P `lifecycle-copy-v6`, S `lifecycle-scopes-v6`, allocation.json et PROVIDER_GO.json
absents après préparation. Base déclarée alignée sur P/copy/code-principal.rag3db.
La préparation n'a ouvert aucune DB et contacté aucun provider.

Anciens 2dccc/Qv5 et D31 restent figés, anciens rouges restent rouges.
Revue totale source+Qv6 demandée avant toute décision C7 : aucun GO natif,
aucun budget/autorité/créneau, aucune qualification shutdown/LIVE acquise ici.
