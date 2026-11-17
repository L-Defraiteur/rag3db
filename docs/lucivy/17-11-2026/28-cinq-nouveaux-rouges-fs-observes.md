# Cinq nouveaux rouges FS observés — sorties, identité et flags

[Ticket ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
GO **C7-LUCIVY-FS-OUTPUT-RED-E75-17nov300**, publié acfd30b407, section complète
lue avant UNE commande27 exacte, parent sans-B. Couple e75f586146/source
**bdf83ad76cf73a74cdde8358affb364b857e27e2**, pins336deb/d1d527/56e99,
préflight54ebfdf0/nonce075f inchangés. Session réelle19943 **retour0**, sans
retry. Le coupon est consommé et clos ; aucun GOvert implicite.

## Résultat réellement joué

Compilation release/offline/locked deux cibles **3,22s**, puis inventaire16/
ignored0 et **six juges Rust natifs réellement exécutés**. CinqFAIL sémantiques
attendus/101, chacun0PASS/1FAIL/0ignored/15filtered ; contrôle1PASS/0FAIL/
0ignored/15filtered/rc0. Aucune erreur d’infrastructure ou mauvais motif.

| Témoin | Observation qui fait échouer l’assertion |
|---|---|
| collision reçu/rapport | Ok(Ok(1)), builder1/report, reçu réellement tronqué égal aux octets index-report |
| rapport dans entrée | Ok(Ok(1)), reçu ADMITTED, rapport créé dans le TempDir input |
| parent de reçu aliasé | Ok(Ok(1)), ADMITTED, reçu créé sous input via lien du TempDir sorties |
| identité manquante | document_ordinal(None,1)=Ok(0), refus Missing absent |
| flags invalides sans effet | Err Flags précis mais Event::Manifest et reçu REFUSED réellement créés |
| sorties distinctes/Some(0) | PASS, builder1, reçu ADMITTED intact/hash vérifié, rapport distinct |

Les chemins sont seulement les petits TempDir possédés. Les opérations FS
rapport/manifest sont réelles, les builders sont des compteurs.
**Aucun moteur/index construit, aucun corpus réel touché**, aucun corps du
second ELF ni des dix anciens tests joué. Ce deuxième ELF est compilé et
inventorié seulement. Source bdf reste défectueuse ; contrôleur0/CommonVERT0
signifie cinq défauts reproduits et un contrôle, PAS gate FS réparée.
Les six rouges0f antérieurs et leurs ELF restent des preuves distinctes.

## Artefacts et ordre des barrières

| Nouveau ELF bdf | SHA256 |
|---|---|
| test_fs_fixture_admission-9128a97001eece0d | 542924c1e30da42ea95f4d208ee34fa300020910107d31549130552a191fa056 |
| test_sfx_v3_ground_truth-71572a69e410ce61 | a50adc2473feebc94e52a9335686e2c5241485a3801b39ab9515dc43c013c470 |

CargoJSON → ledger durable PC1287608.893214784 → copie portable/fsync/rehash/
ACK source/nonce/SHA → inventaires et ledger PC1287609.303076012 → six exacts.
Ledger copié identique
**b42ec21d4edfa1c2da29b4089764cfc6878852220ac8818af0c05b466b4af95a**.
Chaque log de juge et sa row SHA sont conservés et rehachés ; pas d’ACK humain.

Cinq anciens ELF sauvegardés et hashés en RO AVANT compile, puis rehash final
sur leurs **copies**. Les deux chemins de cibles0f dans T contiennent désormais
les nouveaux ELF bdf : les anciens0f sont préservés dans backups/archives,
pas prétendument toujours aux mêmes originaux T. Les trois1ee restent aussi
leur historique conservé. Source bdf propre, neuf copies figées inchangées,
marker T identique/scratch vide. Lock effectif et seed
**dbdb560202562f17536624472a0e730627df163f4bb00c53b2baf604e77637f6**,
contrôle après six juges ET au dernier verdict externe, invariant vrai.

## Fins observées et restitution

PC : START1287605.323879012, ENDadmission1287877.043666828, Common
**186154/start128760537**,
validation-n5ozz9b7/END1287864.412135928. TERMINE/VERT0/erreur null/
restes[]/nettoyé true/ownCommon absent. Fin externe1287609.495564914,
acceptation1287609.509280635, toutes deux avant les deux bornes.
Scan attribué final0bloqué/**11UNKNOWN non attribués**, pas de quiétude universelle.

Portable CLOCK_BOOTTIME : parent3083866/start941309.28,
worker3083881, STARTobservé941309.329884167, ENDobservé941314.237170273,
soit **4.957170273s depuis le début parent**,
travailEND941594.28, TERM941604.28, KILL941609.28.
Résultat0/error null/cleanup_error null, SSH3083906
réapé0. Fin observée de programme, pas mesure d’une expiration/kill OS.
Retour réel poste/SSH et session19943 rc0 vus. Restitution annoncée à C7 et
Recherche **avant archives**, aucun slot gardé pendant rédaction.
Les pending historiques final-accepted/result sont complétés par les reçus
ultérieurs (retour réel/exported-final/physical-end), sans réécrire les bruts.

## Conservation avant correctif

[57 entrées PC](preuves/fs-output-red-bdf83ad-observed/SHA256SUMS) et
[6 entrées portables](preuves/fs-output-red-bdf83ad-observed/PORTABLE_SHA256SUMS)
rehachées conformes. Autorité publique seule, jamais autorite.json/capacité.
Journaux compilation/inventaires/juges, scans attribués, Common recette/
enfants/verdict, scope, ARM/ACK/ledger/fins/locks/bruts sont conservés.

Archives privées PC ET portable :
`/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-output-red-bdf83ad-17nov-v1`.
**18 copies privées en444** (neuf outils/seed/pins + sept fichiers README/Rust
sélectionnés + deux nouveaux ELF), **17 108 104 octets**, SHA conformes sur
les deux postes avant tout fix groupé. Les reçus portables et le reçu
archive-receipt.json sont aussi conservés et comparés sur les deux postes.
Les sources Git complètes sont publiées ; ce sous-ensemble d’archives n’est
pas rebaptisé copie intégrale du dépôt. Pas un target ni des ELF à rejouer.

Le recontrôle d’archivage PC1287720.809376657 est après le
retour réel de la commande, **hors exécution de validation** ; il reste avant
la deadline nominale1287877.043666828. Le champ outside_validation_window du
reçu d’archive décrit cette séparation d’exécution, PAS une assertion qu’une
deadline aurait expiré. Métadonnées/hash sans nouveau scanproc ni nouveau
budget/Common. Un nom de destination publique était déjà pris par les pins
de préparation : l’export a refusé FileExistsError sans écraser ; les deux
archives privées étaient déjà complètes et rehachées. Reprise des copies
publiques sous `-observed` seulement, **aucun rejeu du GO/tests**.

## Suite du même ticket

Actions RO des preuves confiées Recherche+Banc **sans rejeu/import/test/DB/
proc/OS/corpus**. Après conservation, correctif GROUPÉ sorties complètes
prévalidées/distinctes/hors input et non-troncature, traitement des alias,
identité manquante et flags sans effet conformément à30561. Alignement
explicite de l’ancien témoin flags/reçu sur l’exception configuration sans
reçu, pas de contradiction cachée. Nouvelle source/pins/RO totales et nouveau
GO vert distinct avant exécution. Replay absent/globalFS/core/neufHOLD/A-B/
corpus natif/ULP7.2/PR42 restent ouverts, aucune qualification par ces six.
