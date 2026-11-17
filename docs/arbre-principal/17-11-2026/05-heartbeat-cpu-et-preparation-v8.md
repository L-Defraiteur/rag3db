# Ordre heartbeat : CPU et préparation v8

Source exécutable : **6c6b968832d53d76f50ae5244454fe076a8e29c2**.
Ce rapport documentaire ne change pas la source préparée. Qv7/D22 restent
figés et réservés ; rapports03/04 conservent les preuves précédentes.

GO C7-CYCLE-HB-RED-13AE-17nov60, source13ae4a9ab880d335b51da4c12d2a27c7240efd6d
propre : injection unique confirmée puis 1 FAIL rc1!=0, 0 ERROR, 0.007s.
stdout : heartbeat stale/future/outside original scope. Processus rc1,
monotonic local 872773.557463327 à 872773.752380638.

Après publication du correctif, GO C7-CYCLE-HB-GREEN-6C6B-17nov60 : source6c6b
propre, unique invocation timeout TERM59s/kill1s, python3 -B -m unittest -v -f
test_controller_heartbeat_order, cwd debt-copy-a, PYTHONPATH coordinations16/17.
Un PASS, 0 FAIL/ERROR/skip, 0.010s unittest, processus rc0 ; monotonic local
875807.203864862 à 875807.401191422.

Le juge utilise vrai superviseur/read_owned et FakeWorld existant : heartbeat
publié à now+.01 dans sa lecture après création du faux driver. Aucun
processus/proc/service/DB natif par la fixture. Un juge distinct, deux
invocations rouge/vert ; cinq juges finaux6715 et un CommonD22 conservés sans
rejeu. Leurs sources et limites restent celles des rapports03/04.

Archives hors WT sur les deux hôtes, sous ~/.cache/rag3weaver-build :

| Archive | SHA256 manifeste |
| --- | --- |
| cycle-hb-red-13ae-20261117-01 | 776aa3f6833826f3ec700b3d563739ca314989d91199a2628fa60a673a7cac74 |
| cycle-hb-green-6c6b-20261117-01 | 815077594359ce8e3c5326791de7a5f666e1b044582f809ffbf66c62e9a1dd36 |

Chacune contient stdout/stderr/reçu et manifeste. Les trois fichiers
référencés ont été réempreintés taille/SHA sur PC après copie : zéro écart.

Correctif : readHB → check_scope → monotonic frais → validateHB strict →
check_scope. Aucun relâchement des bornes futur/âge ni des identités.
Common/finaliseur inchangés ; P/Q/S et base manifest alignés v8.

PC WT propre /home/lucied/git_workspaces/rag3db-cx-arbre-cycle-hb-order-20261117-r1,
HEAD source6c6b exact. Qv8 sous
~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-control-v8,
prepared SHA256 **a5d86165be320eccfe0f2455a16ecb121abe0cdbe501c3206ea7e23bbfe8b445**.
Vingt-deux copies SHA conformes, préparation/guard rc0 ; P lifecycle-copy-v8,
S lifecycle-scopes-v8, allocation.json et PROVIDER_GO.json absents après
préparation. Aucune DB ouverte, aucun provider contacté. Anciens Q/WT conservés.

Deux revues TOTALES source+paquet demandées avant toute décision C7. Aucun
GO900/natif, autorité, créneau, qualification shutdown/LIVE ou retrait acquis.
Les anciens rouges restent rouges, les tickets restent ouverts.
