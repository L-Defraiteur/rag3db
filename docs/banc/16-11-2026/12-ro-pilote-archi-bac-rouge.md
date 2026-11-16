# Relecture groupée du pilote des deux rouges Bac

APP `049153945e318784353c9cbd61e17beee67fa2d6`, PILOTE `81d0660563ddad1ac9453e34505af46b62599f03`. Lecture de [recette20](../../rag3weaver-archi/16-11-2026/20-pilote-rouge-raccord-bac.md), de ses huit fichiers doc-bac-red et des trois sources Rust témoins. Même [ticket porteur](../../rag3weaver-archi/tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md). Avis favorable à la structure de cette préparation **avec une réserve de borne à régler avant GO**, pas admission ni qualification native.

## Intégrité et témoins réels

Le paquet local `~/.cache/rag3weaver-build/cx-archi-doc-bac-red-package-r1` a pour manifeste SHA256 `ad2251454dc65dcfc249cc6793ce26443fba9930b5778e1b95401458f673f507`. Les 15 fichiers ont été recalculés : toutes tailles/SHA correspondent et chacun est identique au blob Git source/ref déclaré. AST des Python seulement, aucun import ou test exécuté.

Les témoins Rust appellent les vrais setups depuis les deux points lib/e2e, puis lancent le même ELF en enfant avec déclaration privée. L’enfant compare réellement sandbox du manifeste au sandbox déclaré, avant le contrôle du manifeste committé. Le parent conserve stdout/stderr enfant et fixture-failure si refus. APP ajoute ces témoins et leur helper ; il ne corrige pas les setups. Aucun rouge réel n’est encore établi par cette lecture.

## Ordre des opérations et qualification du rouge

Deux compilations no-run offline/locked, features exactes code/daemon/rag3db-native, cap600 chacune. Le pilote exige un artifactJSON test unique par cible, chemin canonique sous T, ELF et copie égaux, préfixeAPP9 présent. Les DEUX compilations finissent avant les quatre inventaires list/ignored cap15, puis ledger. Ces inventaires ne sont pas des juges.

Le ledger contient les deux ELF et les deux noms. Pause bornée min(END-30, maintenant+120), reçu exact source/pilote/ledgerSHA/2ELFSHA/copies PC+local et timestamp entre création et maintenant. Marker runtime-permitted publié après contrôle du reçu et rehash. Cela impose l’ordre de publication ; la réalité des copies sur deux hôtes devra être attestée au futur run, elle n’est pas observée par la préparation.

Ensuite deux exacts cap45, lib puis e2e. Chaque exact appelle son enfant mêmeELF : **deux juges distincts, quatre invocations de test**, plus quatre exécutions séparées d’inventaire. Le parseur exige exit101, deux sommaires0/1/0, deux noms identiques attendus, deux panics et les marqueurs parent/enfant. Présence des sorties enfant et motif du JSON conservé également requis. Un compile error, timeout, nom absent, panic supplémentaire ou autre échec ne devient pas rouge attendu. Premier rouge attendu immutable ; unexpected-red séparé interdit la qualification attendue.

Driver101, scope ROUGE101 sans erreur/restes/nettoyé, autorité terminée et espace privé absent sont requis avant composé EXPECTED_SETUP_REDS_OBSERVED. Extérieur1 est intentionnel ; ce résultat ne peut devenir une livraison verte. Le contrôle après ouverture de result.json rétrograde le résultat en UNEXPECTED_FAILURE si END est dépassée, et la valeur retournée/imprimée est celle écrite.

## Propriétés de conservation et provenance

Préparation Git séparée : source/archive physiques, HEAD détachés et admins sous-module indépendants, trackedSHA récursifs, références distantes et anciens ELF/preuves réattestés. Elle change uniquement le checkout privé propriétaire canonique, pas l’arbre de Lucie. Sous-module exact propre requis ; aucune préparation ni changement de checkout effectué par cette revue.

Une seule racine target session et son enfant actif existant ; identité device/inode/uid et propriétaire sont épinglés. Le préflight et la vraie garde SAMEOWNER doivent laisser l’inventaire complet de l’enfant strictement inchangé, sans sortie d’invalidation. L’ancien état hors enfant est réhaché à chaque garde. L’ELF ancien reste conservé dans ses copies ; sa position sous T peut être reconstruite après admission. Cumul session+runtime limité32Gio par du pendant les phases ; aucun nettoyage automatique.

Le constructeur partagé porte les commandes et les trois rôles launch/poste/watchdog. Identités stables, PID/start/ppid/cgroup/uid/exe/hash/argv/cwd et chaîne arrêtée au lanceur capturé sont contrôlés. UNKNOWN attribué ou erreur d’ascendance bloque ; UNKNOWN global non attribué demeure inconnu. L’exception de FD source W est limitée aux ouvertures RO vérifiées ; elle ne prouve pas absence globale d’autres processus. Après scope : owned historiques absents, scan frais, garde source/archives/SDK et rehash des deux ELF avant vérification END finale.

Les deux SO SDK59 et header sont épinglés ; lib_du_magasin est appelé une fois par entrée native parent, l’enfant hérite l’environnement. Cela ne prouve ni maps, ni ABI globale. Profil privé : TMP court disque vérifié, XDG/RAG états frères et chemins config/data/daemon/journaux dédiés, liste d’environnement autorisée, CPU Burn, jobs min8/POSTE_FILS. Le quota et memory32G/HIGHmax sont vérifiés sous admission. Aucun modèle, cloud ou service n’est qualifié.

## Réserve unique : sens de WHOLE1920

Le constructeur réel est `timeout --signal=TERM --kill-after=15s 1920s poste lourd …`. **1920s est l’échéance TERM ; le plafond dur nominal avant KILL est1935s**, pas1920. Le timeout commence avant la file du poste et enveloppe aussi les précontrôles/composition. L’autorité commune1800 est neuve après acquisition, mais l’attente et le temps hors scope peuvent consommer la marge extérieure : le pilote ne garantit pas1800s intégralement utilisables. Somme des caps explicites1530 hors coûts de hashes/copies/setup/finale ; tous sont des plafonds intersectés, pas une promesse d’achèvement.

Avant GO, le contrat d’allocation doit donc nommer explicitement TERM1920/KILL1935 et la consommation de file, ou le constructeur/politique/doc doivent être alignés si WHOLE1920 désigne réellement le plafond dur. Cette réserve concerne la borne déclarée, sans proposer de relancer ou modifier le paquet en cours de revue. Les hashes et le cleanup final sont bien dans la composition surveillée, et le contrôle post-open/avantdump est présent ; ils ne changent pas cette arithmétique extérieure.

Aucune autre réserve bloquante trouvée dans ce périmètre statique. Préparation PC/configuration exacte encore à produire et relire, puis décision C7 distincte après résolution de la borne. Aucun CPU/natif, allocation, Cargo, garde, sonde proc, modèle, base ou service lancé par cette relecture. Les six tests hermétiques publiés restent NON JOUÉS ici.

## Complément delta final dde6cfb9c0 : réserve levée

Arbitrage C7 : TERM1920/KILL1935, attente incluse, Common1800 maximum sans garantie d’une durée intégrale après acquisition. Pilote final exact `dde6cfb9c06bdba6340ef05e0a7a6e02ebe1539e`, APP049 inchangée. Le delta depuis81d066 porte qualify_red.py, test_contracts.py et primitive-provenance.json ; le constructeur original controller_policy.py reste inchangé. Les garanties précédemment relues hors delta restent applicables, sans refaire une nouvelle qualification des15fichiers initiaux.

Le code ajoute WHOLE1935/TERM1920/FINAL120, fixe l’échéance du driver à min(CommonEND, soft_end extérieur), borne les phases et la pause avec cette échéance effective, et recontrôle celle-ci avant marker et après les hashes finaux. Après retour Common, final_started doit précéder l’échéance effective ; la finale reçoit min(final_started+120, soft_end). Elle ne rallonge pas le temps des tests ; le résultat porte séparément CommonEND, soft_end, hard_end et origine whole_start. Le contrôle post-open/avantdump porte cette échéance finale. La configuration publie les mêmes cinq paramètres de budget. Le timeout extérieur continue à inclure l’attente du poste.

Archive locale lue sans extraction ni exécution : `~/.cache/rag3weaver-build/cx-archi-doc-bac-red-package-final-r1.tar`,9605120octets, SHA256 recalculé `a10d3af127f1df5456c3366b0d08f5208888c5437cad681a17cdf8e75220f96c`. **61 entrées de manifeste toutes size/SHA conformes**, auxquelles s’ajoute le manifeste lui-même dans le tar. Les entrées déclarant une provenance Git correspondent à leurs blobs exacts. Configuration recalculée `99a7e8b7b09e106cbfa724c4e46df815a70dd1c1b84c5caa513f936c5886e9fd`, source049/pilotedde6/budget1800-1920-15-1935-120 ; ses sept copies sont conformes.

Le source runtime final qualify_red.py est identique dans prepared-Q et cpu-final. Les logs CPU archivés rapportent6/0 en0.002s pour le contrat finald195 ; l’ancien rouge085 est conservé séparément et n’est pas repeint vert. Ces témoins ne sont pas rejoués ici. Les deux captures cache-before-git/cache-after-git sont identiques octet pour octet. L’attestation préparatoire publie source/pilote propres et gitlinkbd133850, Pvide/launch-runtime-autorité absents et identité du seulT conservée ; il s’agit d’observations archivées, pas d’un nouveau contrôle PC/proc de ma part.

**Réserve de borne levée pour ce delta exact et cette configuration. Avis préparatoire favorable sans GO.** Allocation C7 distincte encore nécessaire pour les deux rouges attendus ; aucune compilation, garde, native, modèle, service ou nouvelle autorité lancé par cette revue.
