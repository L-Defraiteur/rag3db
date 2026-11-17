# Pilote CLI99 : seize témoins CPU, phase A non admise

GO exact **C7-ARBRE-CLI99-CPU16-4F17-17nov60**, C7dd1e21a57a, consommé UNE
fois sur source propre **4f17c4cc00ede65756c7b8f27e98bdc0d7ba1cca**.
Préflight privé SHA256
785fd82d4526430ea0ceedc7336626bf8a17015f703d12ca5db82913d941ae5a,
pins7inputs et Python/timeout locaux relus avant invocation ; pas de transfert
des pins PC pour l'interpréteur local.

Commande exacte, cwd docs/arbre-principal/17-11-2026/qualification-99 du WT
cx-arbre-prepa-trois-bases-20261117-r1 :

```text
/usr/bin/timeout --signal=TERM --kill-after=1s 59s \
 /usr/bin/python3.14 -B -m unittest -v -f test_select_artifacts test_compose
```

Résultat **16 PASS,0 FAIL/ERROR/skip**, rc0. Huit fonctions sélecteur de vrais
événements CargoJSON synthétiques, huit fonctions du VRAI compose avec Common/
parent/child simulés et write_frame réel. Durée unittest0.091s ; monotonic
local891534.981857845→891535.227269872, soit0.245412027s pour le processus.
Les vrais helpers E9 sont fournis et SHA vérifiés dans chaque fixtureQtemp,
imports isolés : aucun helper ambiant requis. Aucun retry, aucun autre juge.

Cela ne qualifie pas prepare/launch entiers ni le vrai poste/Common/cleanup,
ni Cargo, compilateur, ELF, ISA, SDK réellement chargé, DB ou provider. Aucun
de ces programmes/ressources n'a été lancé par les fixtures ; fichiers de test
temporaires seulement. La lecture du pilote source n'a pas exécuté ses modes.
F8/lib acquis ED restent conservés sans rejeu, anciens rouges inchangés.

## Conservation avant suite

Aux deux hôtes : ~/.cache/rag3weaver-build/qualification-cli99-cpu16-result-01/.
stdout, stderr, receipt.json et preflight.json : quatre fichiers référencés,
taille/SHA réempreintés au PC après copie, zéro écart. Manifeste SHA256
**cf1c3d0d43d5c927a3d11e8edcdb14d669b69a75da0488066e833de1b50c450c**.
Préflight original conservé séparément dans qualification-cli99-cpu16-preflight-01.
Pas de copie d'ELF/SDK/base ni de création du target ou du paquet phaseA.

## Suite encore préparatoire

Config A reste PENDING, WsourceAPP99/Q/P/S/T d'exécution non créés par cette
passe CPU. Aucun GO2400/build/CLI/provider/DB/natif/slot déduit. Recette11 et
sources driver/launcher inchangées par ce rapport. Les deux RO sources ont
levé la réserve d'import statiquement avant ce GO ; le présent résultat donne
désormais le témoin CPU réel, sans convertir l'ancien risque statique en rouge.
Paquet/source/pins à geler et relire, puis éventuel GO A DISTINCT ; B runtime
et C trois bases/oracle restent indépendants. Parent toujours OUVERT.

## Paquet SOURCE de revue, sans préparer l'exécution A

Sous autorisation PRÉPA Q_SOURCE de C7 : PC
~/.cache/rag3weaver-build/qualification-cli99-source-review-r1,
huit copies de sources/JSON vérifiées Git4f17 ou helpersE9, neuf entrées avec
source-review.json SHAe9b67a2ad54520679d7fa246b3307f0673553e7aeb295a673c07aa78e7fbfb97.
Copie du reçu [source-review-pc.json](qualification-99/source-review-pc.json).
kind=SOURCE_REVIEW_ONLY_NOT_EXECUTION_PACKET, allocationPENDING ; driver/
launcher SHA conformes à la configuration proposée. Aucun import ni mode
prepare/launch exécuté. Aucune donnée/ELF/SDK/base copiée.
WsourceAPP/T/P/Qexecution/S réattestés ABSENTS en métadonnées après ce dépôt.
Ce Q_SOURCE ne remplace pas Qruntime : launch.prepare attend toujours APP99
existant, conditions/pins et allocation exacts ; leur admission n'est pas
déduite du présent paquet. Deux RO preuvesCPU/Q_SOURCE demandées avant suite.
