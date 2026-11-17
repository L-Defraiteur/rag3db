# Six rouges Rust FS observés, positif vert — gate non réparée

[Ticket FS ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
GO distinct `C7-LUCIVY-FS-RED-V4-17nov300`, C709726a3640, consommé une fois
après GO1+GO2 verts et archives deux postes. Commande22 exacte, source
**0f610f437a341f745e5936492114142ff3afedd4**/couple77a, source et pins inchangés.

## Résultat réel, pas une gate verte

Deux ELF compilés release/offline/locked en **2,98s**. Sept juges **Rust natifs
réellement exécutés** dans test_fs_fixture_admission : **6 FAIL sémantiques
attendus, 1 PASS, 0 ignored/skip**. Chaque négatif101 a son nom, sa cause,
assertion et trace attendus,0/1/0/6filtered ; positif1/0/0/6filtered.

| Cas | Observation réelle |
|---|---|
| Vue vide | Ok(None), refus Empty manquant |
| Énumération refusée | sous-arbre ignoré, Ok(Some(1)), continuation builder/report appelée |
| Métadonnées refusées | candidat ignoré, Ok(Some(1)), même effet |
| Lecture refusée | candidat ignoré, Ok(Some(1)), même effet |
| UTF8 invalide | candidat ignoré, Ok(Some(1)), même effet |
| Panel15 non couvert | continuation appelée malgré absences, Ok(Some(1)) |
| Positif15 couvert | mêmes octets capturés, hash connu/vecteurs empty+abc, read1/builder1 |

Les opérations FS sont réelles sur petits fichiers possédés, les fautes sont
injectées par SpyFs et les continuations builder/report sont des compteurs.
**Aucun index n'est construit, aucun corps v3_ground_truth_contains ou autre
test du second ELF n'est exécuté, aucun corpus réel n'est touché.** Le second
ELF est seulement compilé/inventorié. Les champs bruts native_test_bodies...
false désignent ici ces corps de comparaison/indexation, pas les sept juges
FS en instructions natives ; ne pas leur faire dire « aucun Rust exécuté ».

Le pilote/Common/CLI rendent0 parce que les six défauts attendus sont observés.
Ce bilan reste **RED sémantique**, pas admission FS corrigée, parité réelle,
neufHOLD/corecomplet/ULP7.2/PR42 ni fonctionnement de corpus qualifiés.
Le refus E0277 historique3759 est conservé ; le helper hex0f est maintenant
compilé et ses valeurs connues ont passé dans le positif, sans nouveau refus.

## Chaîne ELF et verrouillage du graphe

| ELF source0f | SHA256 |
|---|---|
| test_fs_fixture_admission-9128a97001eece0d | 0801ea5ecc0d02844edff7ef032c2aafff6a25ad4dab712b4f2d65106c23bea9 |
| test_sfx_v3_ground_truth-71572a69e410ce61 | ee1965d705b1e41e85fbf7645d856793a235b9024686a41af0c95685d7890a04 |

Compiler JSON → ledger durable PCobservé1276408.851121676 → copie portable
fsync/rehash et ACK atomique source/nonce500867/SHA → inventaires et ledger
PCobservé1276409.161644828 → sept exacts. SHA du ledger copié identique
**1ab5164e26969c7beebd7f4627cc87e26d361a7368889b77fe68d937fecb813b**.
Le transfert/ACK était autonome, aucun collègue humain requis.

Rehash physique deux ELF final identique ; rehash lock **effectif** après les
juges dans le pilote et au dernier verdict externe : **dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6**,
seed et neuf copies inchangés, W0f propre, markerT3552 conservé, scratch vide.
Trois anciens ELF1ee sauvegardés RO avant compilation et toujours SHA identiques
dans leurs originaux T. Aucun purge/import/target frère, aucun replay mémoire9.

## Fins réelles dans leurs propres horloges

Portable CLOCK_BOOTTIME : watchdog **2502082**, début kernel930110.51,
worker2502097, START observé930110.606052820, END observé930115.077668558,
soit **4,567668558s** depuis le début parent. ENDtravail930395.51, TERM930405.51,
KILL930410.51. Résultat0/error null/cleanup_error null ; enfant SSH2502107
réapécode0. Ces temps sont des observations de programme sous watchdog,
pas une mesure d'un timeout expiré/killOS.

PC monotonic : START admission1276405.502095567, END1276677.103524762.
Common **2593434/start127640555**, échéance1276664.591858144,
**TERMINE0/erreur null/restes[]/nettoyé true**. Fin externe après retour Common
1276409.312277359, puis dernier verdict1276409.324987473, dans les bornes.
Common propre absent,0bloqué/11UNKNOWN nonattribués conservés. Pas de comparaison
des timestamps absolus entre hôtes ni de quiétude universelle inventée.

Retour réel poste/SSH puis session locale81490 rc0 observés ; fichier final
exporté durable, reçu END réel. Le pending de result.json et le
actual_poste_return_pending du verdict PC sont levés par ces preuves **ultérieures**,
sans réécrire les anciens bruts. Restitution réelle signalée à C7 et Recherche
avant archivage, aucun slot conservé en attendant la rédaction.

## Conservation avant tout fix de gate

[Preuves PC](preuves/fs-red-v4-0f610/SHA256SUMS),
[reçus portables réels et sources/ELF privés](preuves/fs-red-v4-0f610/PORTABLE_SHA256SUMS).
Journaux compiler/inventaires/sept juges, sept rows et SHA, captures de scans,
recette/enfants/verdict Common et autorité publique seule : aucune capacité
autorite.json exportée. Le recontrôle d'archivage1276981.213 est **après END**,
explicitement hors validation, source/copies/lock/marker/scratch conformes,
Common toujours absent/0bloqué11UNKNOWN ; aucun nouveau budget ni extension.

Sur **les deux postes**, cache privé
`~/.cache/rag3weaver-build/cx-lucivy-fs-red-v4-17nov-v1` : deux ELF actuels,
neuf copies d'outils/seed/sourcepins et cinq fichiers Rust, soit16 copies
relues par SHA/en444. Preuves publiques textuelles aussi conservées deux
postes. Ce cache est une archive de preuve, pas un target ou binaire à rejouer.

Les quatre familles requises sont désormais réellement rouges, les trois
sous-cas I/O séparés. Aucun correctif de gate n'a précédé cette conservation.
Revue des preuves demandée à Recherche et Banc **sans rejeu/DB/OS/proc/corpus**.
La préparation du fix source est dans le périmètre C7, avec admission avant
consommation/refus précis/UTF8/couverture15/absence sans skip ; nouveau pin,
RO et GO distinct avant toute nouvelle exécution. GO3 est clos, aucun natif
corpus/A-B ou « vert FS » n'en découle.


## Revue Recherche des preuves bb5707c6a5

Verdict reçu : favorable borné **aux preuves RED FS**, sans rejeu. Recherche a
rehashé les objets Git des quatre listes : 62 entrées PC, 7 portables,
5 CPU lock et 6 CPU journal, soit **80 entrées conformes**. Elle a aussi
rehashé les 16 copies de l’archive privée **locale** (sources et deux ELF,
16 801 267 octets), sans écart. La conservation PC reste ici la preuve du
porteur ; Recherche ne revendique pas de rehash PC intégral indépendant.

Les sept journaux et leurs SHA corroborent les six FAIL101 sémantiques précis
et le PASS, avec les compteurs attendus. Ledger/ACK avant inventaires et juges,
lock effectif et seed DBDB, derniers recontrôles, fins Common et physique,
retour SSH réapé0 et cleanup sans erreur sont cohérents à sa lecture.
Les champs pending historiques restent intacts, complétés par les reçus
ultérieurs. Les 11 UNKNOWN globaux ne prouvent aucune quiétude universelle.
Les cinq PASS CPU concernent leurs modèles annoncés ; les sept juges FS
sont bien du Rust natif exécuté. Aucun corps groundtruth/index/corpus exécuté.

Aucun rejeu, import, test, proc, OS, DB ou corpus par Recherche. Cette revue
permet la préparation source ; **elle ne qualifie pas la gate FS corrigée**.
Revue Banc encore attendue ; nouveau pin, RO et GO distincts avant exécution.


## Revue Banc des preuves bb5707c6a5

Avis favorable borné publié par Banc dans **d32884b667**, rapport
`docs/banc/17-11-2026/03-ro-couple-fs-red.md`, section « Preuves exécutées v4 ».
Les 62+7 entrées publiques FS et 5+6 CPU sont conformes aux blobs Git.
Banc a rehaché indépendamment les **16 fichiers privés sur PC et portable**,
sources/outils/deux ELF : zéro écart. Les sept logs et argv exacts corroborent
six FAIL sémantiques et un PASS ; ledger/ACK avant inventaires/juges et deux
verdicts lock effectif DBDB sont cohérents. Les fins Common et physique,
retour SSH réapé0 et pending complétés sans réécriture sont corroborés.
Les 11 UNKNOWN restent inconnus ; aucune expiration du timeout ou quiétude
universelle n’est qualifiée. Lecture/hash seulement, aucun rejeu, test,
import, build, DB, procscan, service ou corpus par Banc.

**Les deux revues des preuves sont maintenant favorables**, chacune dans sa
portée explicite. La gate reste défectueuse ; le ticket reste ouvert.
La préparation source peut poursuivre le contrat C7. Nouveau pin, nouvelles
RO et GO distinct avant toute exécution ; aucun ancien GO réutilisable.
