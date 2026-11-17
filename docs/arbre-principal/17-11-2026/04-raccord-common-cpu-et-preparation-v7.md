# Raccord Common : rouge, vert et préparation v7

Source exécutable préparée : **d22bf28c580910e9b1bea1065297028f9cfc75bf**.
Ce rapport documentaire ne modifie pas cette source. Suite du rapport03,
dont Qv6/source6715 restent figés et réservés pour le raccord Common.

## Deux invocations CPU distinctement autorisées

GO C7-CYCLE-COMMON-RED-B2BD-17nov60 sur b2bd689bf5c564b7820bf80cbf3db272fc619c5d
propre : contrôle positif de la vraie signature Common passé, puis FAIL sur
le vrai argv compose contenant -B devant Common. Un juge, 0 ERROR, 0.016s,
rc1 ; monotonic local 871644.724223774 à 871644.893095241.

Après correctif publié, GO C7-CYCLE-COMMON-GREEN-D22-17nov60 sur d22 propre :
unique invocation timeout TERM59s/kill1s de python3 -B -m unittest -v -f
test_common_entry_signature, mêmes PYTHONPATH coordination16/17 et cwd
debt-copy-a. Un PASS, 0 FAIL/ERROR/skip, 0.009s, rc0 ; monotonic local
872074.774591849 à 872074.902138459.

Un juge distinct Common, deux invocations rouge/vert. Les cinq juges finaux
verts sur6715 sont conservés sans rejeu ; le finaliseur est inchangé.
Le témoin Common utilise vraies expressions argv/env et vraie fonction
signature_superviseur extraites AST ; faux proc temporaires, readlink simulé.
Aucun processus/proc réel ni DB/provider/CLI/native par cette fixture.

Archives hors WT sur les deux hôtes sous ~/.cache/rag3weaver-build :

| Archive | SHA256 manifeste |
| --- | --- |
| cycle-common-red-b2bd-20261117-01 | a181c7e876f82527079d3e4dbe3353f2233aacf6cef00f85528e2e56c224f894 |
| cycle-common-green-d22-20261117-01 | e99f4e3e36145fd32dd4c28122b54d78bdabe2144fe97289ccebd797a4945cf6 |

Chaque archive contient stdout/stderr/reçu et leur manifeste ; les trois
fichiers référencés ont été réempreintés taille/SHA sur PC, zéro écart.

## Correctif et paquet métadonnées

-B retiré uniquement devant confiner_validation.py ; environnement explicite
PYTHONDONTWRITEBYTECODE=1. Les rôles contrôleur/driver/acteur gardent leur
argv -B exact. Common partagé et finaliseur6715 inchangés.
P/Q/S et base manifest passent ensemble v7.

PC nouveau WT propre :
/home/lucied/git_workspaces/rag3db-cx-arbre-cycle-common-argv-20261117-r1,
HEAD d22 exact ci-dessus. Ancien WT6715 et Qv6 conservés.
Qv7 : ~/.cache/rag3weaver-build/bascule-live-20261116/lifecycle-control-v7.
SHA256 prepared : **22ebca7c46c645f6a7bfd90b7cb6689dc1432a4832d5669ca2eb4a2f87493f5b**.
Vingt-deux copies contrôlées par SHA, guard complet et préparation rc0.
P lifecycle-copy-v7, S lifecycle-scopes-v7, allocation.json et PROVIDER_GO.json
absents après préparation. Aucune DB ouverte, aucun provider contacté.

Revue totale source+paquet v7 requise avant décision C7. Aucun GO natif900,
créneau, autorité, qualification shutdown/LIVE ni retrait déduit. Les anciens
rouges restent rouges, les tickets restent ouverts.
