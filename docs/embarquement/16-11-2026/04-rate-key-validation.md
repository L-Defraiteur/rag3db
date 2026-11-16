# Clé de débit : qualification réelle du 16 novembre

Source exacte `07731af482ac0ed65ae3775e49900215d40ee2bf`, base
`d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6`, branche cx-rate-key-20261116-r3.
Ticket : `docs/embarquement/tickets/2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md`.
Périmètre : clé et lecture courante seulement, API historiques explicites conservées.
Aucune mesure par passe ni table enrichie livrée par ce lot.

## Jugements réellement exécutés

| Étape | Réussis | Échecs | Ignorés |
|---|---:|---:|---:|
| Neuf ciblés exacts (un test par invocation) | 9 | 0 | 0 |
| Témoin canonique | 1 | 0 | 0 |
| code_sync (hors précontrôle commun 2/0/0) | 21 | 0 | 0 |
| lectures (hors précontrôle commun 2/0/0) | 27 | 0 | 0 |
| activation à la demande (hors précontrôle commun 2/0/0) | 1 | 0 | 0 |
| Lib complète | 2604 | 0 | 72 |
| Navigateur | 45 | 0 | 0 |

CHECK_FEATURES VERT 8/8 et preuve rag3weaver revérifié, sorties originales
conservées. Les **72 ignorés ne sont pas jugés verts**. Les deux paniques du
reçu d'index dans lectures sont les refus attendus des tests négatifs : les
27 juges terminent sans échec. Ne pas additionner les précontrôles communs
répétés comme des juges distincts. Les neuf noms et commandes exacts figurent
dans phases.json et target-01 à target-09.log, couverts par le manifeste.

## Portée, budget et terminaison

Validation commune `validation-wm1493du`, 32 Gio effectifs,
memory.high=max, cpu.max=500000/100000, POSTE_FILS=5 et jobs=5.
Deadline absolue `1185936.20433228`, fixée après acquisition du poste,
jamais réinitialisée ; canonique 529.966 s. Neuf ciblés joués avant
coordination, aucun appel partagé avant grant. Verdict final autoritaire
VERT/code0, restes=[], nettoye=true. Scan final horodaté monotonic
1182407.58428124 : **1803 identités PID/starttime, owned_alive=[], unknown=[]**.
Le pic mémoire n'a pas été capturé par le pilote ; ne pas confondre plafond
prouvé et pic mesuré. Aucun zéro GPU n'est inféré de l'absence de compteur.

## SDK, services, profil privé

SDK complet :
`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
Trois empreintes revérifiées dans sdk-profile.json :

- librag3db.so : b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101
- libvector.rag3db_extension : 1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561
- include/common/build_id.h : 1cf920608514b5f3c5e90952ecc580ca6170398f288dd554d00bb0bb32b03110

Profil privé vérifié avant/après, SDK externe explicite, aucune base Lucie,
aucun API7970, aucun service/modèle changé. GET santé avant/après compatibles,
puis scan final : 7878 granite-278m/768/dense et 7879 bge-m3/1024/dual+sparse,
Flex32, non factices, max_chars_per_text12000, exécutable1794771361 inchangé,
baux=0/en_cours=0 tous deux. Slot PG fixture coordonné et rendu sans reset,
sans sonde SQL supplémentaire. Rendu explicite à Mémoire et C7, aucun appel
futur de cette passe ; ne pas inférer un nouveau créneau depuis cette archive.

## Conservation et vérification

Preuves originales PC : `~/.cache/rag3weaver-build/cx-rate-key-59ac-v1/` et
`~/.cache/rag3weaver-build/validations/validation-wm1493du/`.
Archive des 73 fichiers réguliers de preuve (journaux, profils, phases,
verdict, inventaire des enfants ; sans secret autorite.json ni fixtures)
aux deux postes : `~/.cache/embarquement-notes/rate-key-16nov/proofs-07731.tar.gz`.
Manifeste SHA256 par fichier conservé sur la branche de preuves :
[proofs-07731-manifest.json](https://github.com/L-Defraiteur/rag3db/blob/9e16944db3/docs/embarquement/16-11-2026/rate-key/proofs-07731-manifest.json), SHA256
`0dc70ae7a30ddd2efeb2b3e4437c975c499c84e73df38dce8ebe89653e1c722c`.
Une première copie récursive incluant des fixtures a refusé leurs sockets ;
elle n'est pas une archive fidèle et ne sert jamais de garde de retrait.
L'archive régulière séparée et son manifeste sont les preuves de cette passe.
Aucune suppression W/admin/target autorisée par ce rapport.

## Suite du même ticket

Relectures indépendantes ED et Mémoire sur ce contrat et ces preuves ; puis
candidat neuf sur master courant, audit modes/OID des cinq fichiers produit,
protection des apports master et range-diff. Aucun tour master déduit du vert.
La table et la mesure par passe restent ouvertes dans le ticket partagé.

### Livraison séparée des preuves

La livraison r5 repart de la seule source testée07731, avec ses17patches
reposés sur df2a5789888d16247f0dad42dcdaa3c4ceea2094. Les ajouts ultérieurs
de cette branche sont exclusivement Markdown. Les branches r3/r4 et leurs
commits de preuves9e16944db3/5e8983ad26/ed2fd93e49 restent intacts et distants.
L’[audit du candidat précédent](https://github.com/L-Defraiteur/rag3db/blob/ed2fd93e49/docs/embarquement/16-11-2026/rate-key/candidate-r4-audit.json)
reste une preuve externe ; aucun JSON n’est renommé ou reclassé en Markdown.
Le SDK profile JSON et les scripts/tests présents avant07731 sont conservés.

Relectures indépendantes ED et Mémoire favorables sur produit07731 et preuves
9e16944db3 : clés courantes, compatibilité historique, provenance inconnue,
getter pur et manifeste73/73 vérifiés. Ces avis ne donnent pas le tour master.
