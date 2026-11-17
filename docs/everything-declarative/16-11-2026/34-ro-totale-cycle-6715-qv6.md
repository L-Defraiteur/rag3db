# RO totale du cycle6715 et Qv6

Source PC/exécutable **6715c17ed98ca708edeaecfe068d9eb881b6f722**,
prepared **7bf5cd553e2836b20446f77200647991f0884c1a62f2a1d015e2ab9c3c9cf9c0**.
Rapport documentaire03 de5840a759d4, sans déplacement du pin exécutable.
Avis initial765c93e47e favorable, **CORRIGÉ EN RÉSERVÉ/HOLD** : le raccord
argv Common avec -B est incompatible avec la signature du vrai superviseur.
La levée du P1 finalisation et les vérifications de fichiers restent bornées
à leurs propres preuves ; elles ne qualifient pas cette entrée Common.
Suite de la [RO33](33-ro-cycle-supervise-2dccc.md) et même ticket C7 OUVERT.

## Correction de l'avis : P1 de signature Common confirmé

Recherche a identifié le défaut et Arbre l'a consigné avant correction,
témoin b2bd689bf5 NONJOUÉ. ED l'a manqué dans la RO totale initiale ; lecture
indépendante confirme : lifecycle.compose appelle
`[python, '-B', confiner_validation.py, 'lancer', ...]`, alors que
`signature_superviseur`, vrai confineur6439 lignes138-140, exige args[2] égal
à lancer et le script à args[1]. Le reçu est donc refusé avant ARMED.
Cette conclusion remplace la suffisance du couple revendiquée ci-dessous,
sans effacer les vérifications réalisées ni transformer les CPU en natif.

6715/Qv6 restent HOLD. Proposition propriétaire APRÈS rouge exact autorisé :
retirer -B seulement devant Common et utiliser PYTHONDONTWRITEBYTECODE=1,
rôles acteur/contrôleur/driver inchangés. Aucun correctif mis en œuvre ici,
aucun test ou runtime lancé. Nouvelle source/paquet/RO et décision C7 distincts
nécessaires ; aucun GO900 ne découle de l'avis765.

## Chaîne complète relue

La portée physique900 proposée conserve outer898+kill2 sous vrai poste,
common830 et limites32G/HIGHmax/swap0/quota<=8. Le common lance le contrôleur
propriétaire, qui attend ARMED/loop+heartbeat après vrai exec/préflight, avec
observation OS indépendante/poll de son enfant, avant driver/copie/DB.
Interpréteur/source/prepared/allocation/autorité/CG/ascendance, deux nonces et
configSHA externe sont liés. ARM10 MIN END-500, heartbeat10 et release2 MIN
END restent des caps internes, aucune horloge900 propre à l'acteur.

Le parent surveille aussi les phases bloquantes du driver ; rc0 inattendu,
absence ou premier rouge entraînent refus. ACTION_GUARD est installé avant
copie/DB, vérifie le rôle driver et l'admission avant les actions natives et
READY. Le protocole exige producer_watcher, sans compatibilité silencieuse
avec une ancienne admission. La politique attendue n'est pas une santé
fraîche ; vrai owner/transport pinés contrôlent PID/start/exe et identité,
usage/type, READY/nonce/phase avant chaque GET borné. Admission et lease sont
distinctes ; l'acteur reste vivant jusqu'au RELEASE du driver, sans health
supplémentaire pendant cette attente. Aucun embedding/restart autorisé.

Le graphe demeure SOURCE.meta/signals[] sur copie neuve FIN sans dette,
writerCLI unique, nonce/response publique publiée avant shutdown, profil/maps
du vrai writer requis, ACK shutdown/rc0 même enfant ; puis digests globaux,
graphe/schéma/FTS/finished/pending/coldreads et sources immuables. Les scans
de quiescence sont dans la recette réelle, pas remplacés par cette lecture.

Final6715 : garde END/first-red avant publication et après file fsync/link/
directory fsync/staging cleanup, avant retour0. Refus latché, premier rouge
conservé, résultat RED/rc1. Rattrapage limité aux octets canoniques attendus ;
limite mêmeUID hostile/origine inode explicitée au rapport03 et conservée.
La primitive et q.save restent inchangés. P1 final2dccc levé sur ce correctif,
sans réécrire l'ancien verdict ni prolonger son ancienne borne.

## Contrôles indépendants des fichiers PC

- HEAD6715 propre du WT cx-arbre-cycle-finaliseur-borne-20261117-r1.
- Qv6 :23 entrées exactes, prepared +22 copies régulières UID utilisateur,
  SHA égaux au prepared ET aux blobs Git6715 ; pas d'entrée supplémentaire.
- Python physique /usr/bin/python3.14 SHAa38210…449d, timeout pin conforme.
  Acteur7d7db…6612, contrôleur, protocole, policy, owner83139 et transportedf45
  identiques à2dccc hors raccord final déjà relu.
- P/S/allocation.json/PROVIDER_GO.json absents, y compris liens pendants.
  Manifeste vise précisément P/v6/copy/code-principal.rag3db.
- Neuf références COPYA, cinq AUDIT, cinq FIN et huit sources carry réhashées.
  Ancien prepared CC18 et Qv5/e374 restent conformes et figés, pas de remise à zéro.
- Sept outils/en-tête/SDK pinés recoupés par hash seulement : CLI0e310,
  readeradd010, ids, chunker, rag3db.hpp et les deux bibliothèques59ac.
  Aucun de ces binaires ni bibliothèque n'est exécuté/chargé par cette RO.
- Profil provider attendu et provenance fichier conformes, sans lire /proc
  ni contacter le service ; aucun constat actuel de santé/PID n'est inféré.

Lecteurs indépendants hors W : R/cx-ed-ro-cycle-6715.py et son résumé JSON,
cx-ed-ro-cycle-6715-extra.py ; aucun import du pilote ou des tests. Seuls
Git/fichiers/hashes sont lus. Aucune DB ouverte, SQL, signal/processus runtime,
Cargo/rustc, service/health/SDK activé, réservation ou autorité créée.

## Preuves CPU lues, sans rejeu

Les deux archives sont présentes LOCAL+PC, trois fichiers chacune, rehashés
taille/SHA indépendamment sur les DEUX hôtes :

| Archive | Manifeste |
| --- | --- |
| cycle-final-red-d31-20261117-01 | d098d0a93cf366e8c0a6c70c3ad054afe9efaa7237d31bc6b5558482231fd846 |
| cycle-final-green-6715-20261117-01 | 559e5358721fccebbd61bc411ce90e5d74b178dd00ba91ca55846bcb909a7f1f |

Rouge D31 sous GO distinct : seul fsync-late, 1FAIL/0ERROR, assertion0==0,
rc1, quatre autres non joués. Vert6715 sous autre GO :5PASS/0FAIL/ERROR/skip,
0.038s unittest/rc0. Cinq fonctions distinctes, six invocations historiques ;
late-link ne devient pas rétroactivement un témoin causal de l'ancien code.
Les124 CPU de l'armement restent l'acquisition publiée2dccc, pas un nouveau
run de cette revue. Ces preuves ne qualifient ni processus acteur réel, ni
DB/shutdown/LIVE. À C7 seul un éventuel GO900/slot/ressources exact distinct ;
aucun ancien GO3c2 ni GOCPU ne vaut admission native.

## Delta Common d22bf28c58, avant CPU vert

RO statique favorable sur ce SEUL correctif : -B retiré uniquement devant
Common, argv effectif désormais python/script/lancer, compatible avec les
positions exigées par la vraie signature_superviseur inchangée. Environnement
de cet appel pose explicitement PYTHONDONTWRITEBYTECODE=1. Le suffixe
python/-B/lifecycle.py/--controller et rôles acteur/driver restent inchangés,
de même que finaliseur et primitive Common. P/Q/S et manifeste passent ensemble
àv7 ; ancien Qv6 demeure figé et HOLD, aucun paquetv7 validé par cette lecture.

Le témoin lit le vrai argvAST, la vraie signatureAST et le vrai envAST, contrôle
positif sans-B en premier puis comparaison du raccord et du suffixe contrôleur.
RougeB2BD relu et trois fichiers taille/SHA rehashés LOCAL+PC, manifeste
a181c7e876f82527079d3e4dbe3353f2233aacf6cef00f85528e2e56c224f894 :
1FAIL après contrôle positif, 0ERROR/rc1/0.016s. Aucun rejeu ici, aucun vert
d22 ni admission native déduit. Décision C7 CPU vert exacte DISTINCTE, puis
source/paquet complets et RO avant éventuel GO900.
