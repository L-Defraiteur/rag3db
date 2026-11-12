# Driver runtime35ea publié et montage contrôlé sans test

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Un passage DdlEveryMinute autorisé après revue indépendante du
paquet35ea834fichiers/110723932octets. La préparation et le préflight ci-dessous
précèdent ce passage ; aucun succès runtime n'est revendiqué ici.

## Driver et argv publiés avant lancement

[Driver](outils/cx-runtime-saturation.py), SHA256
`3445e990e2e4ee2752f89eef6900f67d3ce99207da51e68392ee0c99ec4ee175` ;
[lanceur](outils/cx-lancer-runtime-saturation.sh), SHA256
`912454a56f3787ba5c3d0d95c0cd92726ca44c259a7e8e53ba85f616803d022d` ;
[plan statique et limites](outils/19-runtime-plan.json).
Python AST et bash -n conformes ; un seul site Popen natif, après sortie du mode
preflight. Aucun compile/lien/runtime natif pour ces contrôles.

Binaire : paquet privé stable
`~/.cache/rag3db-moteur-notes/annulation/cx-build-saturation-12nov-02/package/bin/concurrence_test`.
Arguments : `--gtest_filter=TwentyWriters.DdlEveryMinute`, `--gtest_color=no`,
`--gtest_output=xml:<sortie-privee>/xml.pipe`.
Le XML utilise un FIFO privé préouvert par le contrôleur, puis est conservé comme
result.xml. Cela permet de borner les logs sans imposer RLIMIT_FSIZE à la base du
test. Les données et assertions du scénario restent celles du binaire attesté.
Aucune découverte de tests, aucun filtre supplémentaire ni seconde invocation.

L'environnement natif est construit **de zéro**, avec seulement :

| Variable | Valeur |
|---|---|
| PATH | /usr/bin:/bin |
| LANG et LC_ALL | C |
| TZ | UTC |
| TMPDIR et RUNNER_TEMP | scratch/runtime/tmp |
| CX_SATURATION_OUTPUT | sortie privée persistante/capture.csv, fichier neuf |

Aucune variable HOME/SCENARIO_*/GTEST_*/BUFFER_POOL_SIZE/MAX_NUM_THREADS ni seuil/
checksum/compression héritée. Les réglages de la charge viennent du code figé :
pool2G,8threads,20Doc+2Note+4lecteurs,graines/calendrier/DDL d'origine. Les historiques
n'avaient pas une attestation complète de l'environnement : ce passage diagnostic
ne doit pas être qualifié de comparaison à charge réalisée identique.

## Préflight de montage déjà réalisé, sans test

Unité cx-coeur-runtime-saturation-preflight-12nov-01 sous poste lourd : rc0,
89ms/pic40,1M/swap0. Le mode preflight rehash les834fichiers, les dépendances
système directes et les BuildIDs ; il vérifie les montages, chemins et limites,
puis quitte **avant** Popen. [Reçu](preuves/runtime-saturation-preflight/preflight-ok.json),
[argv/env exacts](preuves/runtime-saturation-preflight/preflight-plan.json),
[limites effectives](preuves/runtime-saturation-preflight/preflight-limits.json),
[journal](preuves/runtime-saturation-preflight/preflight-unit.log).

Le paquet est monté RO au chemin source compilé :
`~/.cache/rag3weaver-build/cx-build-saturation-12nov-02/source`.
L'extension vector est donc privée, avec la même empreinte que celle du manifeste,
et accessible au chemin absolu que loadVectorExtension utilise. Aucun fallback
vers lib distribuée. BuildID35ea vérifié par lecture des octets, pas exécution.
Les neuf dépendances directes système, dont GTest/GMock1.18, sont rehashées.

Scratch tmpfs dur2G au chemin d'ancre, source/packageRO, seules sorties scratch/
runtime et dossier de preuves privéRW. Réseau sans route, /dev privé sans GPU,
aucun service/PG démarré ou appelé. Le helper reçoit RUNNER_TEMP dans ce scratch :
son nettoyage ne touche que sa passe privée. Capture hors scratch, conservée après
arrêt de l'unité. Le montage de test de préflight est détruit en fin d'unité,
aucun objet moteur ni magasin utilisateur n'y est modifié.

## Runtime : mêmes bornes autorisées, pas de nouvelle attente de revue

Le lanceur run prend **poste mesure**, prioritaire ; le budget démarre après
acquisition. RuntimeMax1200s/TimeoutStop10s, cgroup8G/swap0, KillModecontrol-group,
Restartno, scratch2G. Le contrôleur commence l'arrêt à1188s pour réserver le temps
de collecte avant le filet externe ; TERM du seul groupe natif,2s puis KILL,wait.
Il garde un marqueur attempt.json créé exclusivement avant son unique Popen.
Aucune relance, aucun autre binaire, aucun modèle/service/GPU/PG.

Le CSV direct est borné structurellement :1280samples,16uint64 chacun, schéma,
first_meta/first/first_saved et status, chacun de longueur fixe ou numérique
bornée ; moins de1Mio réservé. Stdout/stderr et XML passent par le contrôleur,
avec un budget cumulé strict15Mio ; surplus tronqué explicitement puis arrêt
nommé logs_budget_exceeded. Ce partage laisse au plus16Mio pour les trois logs,
avec marge pour les petits reçus. Pas de limite de fichier appliquée à la base.
Le CSV est surveillé aussi pour dépassement de sa borne structurelle.

La première capture est vidée par le sampler horsrollback. Sur timeout, fichiers
persistants remis au noyau conservés ; capture globale **incomplète**, même si
first_saved fournit une première observation partielle. Schéma,16colonnesnumériques,
status,compte d'échantillons et marqueur premièrecapture sont contrôlés. Statut
GTest/XML/returncode distinct de l'intégrité de capture. Un test rouge reste rouge,
même si la capture est complète ; le code de sortie du driver le reflète.

Avant run : nouvelle revalidation de toutes les empreintes et du montage. Après :
rehash du paquet, logs/XML/CSV et résultat séparés, wait du natif, état unité/PID0 et
poste rendu. Si un prérequis, montage ou hash diffère, aucun Popen ; premier autre
échec stop/diagnostic. La publication du présent driver et le préflight conforme
satisfont la condition d'exécution déjà autorisée ; pas de demande supplémentaire.

Au moment de ce commit, aucun scénario lancé. L'exécution unique et son verdict
seront documentés séparément, en conservant les résultats incomplets et sans
confondre capture/compilation avec preuve de sûreté ou explication causale.
