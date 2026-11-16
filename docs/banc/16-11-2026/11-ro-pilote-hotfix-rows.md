# Relecture RO du pilote du rouge SHOW_INDEXES

Avis favorable **à la préparation du seul rouge binder**, pas GO d’exécution. Pilote exact `db833d135cbceb3319424f1619f219e64bfd1fc0`, APP `5099a367ddf1e48db2b4065d91a61403de994571`, moteur attendu `59ac9c86bc2b3529b590a7462ab7051e3ad26b8a`. Lecture de [doc06](../../coeur-cpp-hotfixs-tickets/16-11-2026/06-rows-outside-graph-pilot.md), prepare/launch/entry/controller/receipt et du témoin C++ APP. Même [ticket existant](../../coeur-cpp-hotfixs-tickets/tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md).

## Identité et préparation

Les six SHA des scripts du reçu CPU publié correspondent aux blobs de ce pilote ; syntaxe des cinq fichiers Python relue par AST, sans import ni exécution. Les six tests synthétiques verts restent une preuve déclarée dans ce reçu : ils n’ont pas été rejoués ici et ne qualifient ni moteur ni superviseur. Préparation PC et recipe SHA restent à produire puis à relire avant une admission C7 distincte.

prepare exige luciepc, SHA40 du pilote propre et APP propre, refuse les destinations existantes, construit un bundle autonome vérifié puis épingle les scripts et trois fichiers natifs. Le contrôle du dernier commit moteur emploie les mêmes chemins que cmake/build_id.cmake ; l’identité moteur59ac ne se confond donc pas avec le commit de tests5099. Aucun BUILD_ID n’est fabriqué ou réécrit par le pilote.

## Admission et bornes

launch enlève les overrides CPU, prend le vrai poste lourd32G/swap0 puis met entry entière sous timeout TERM2390/KILL10. Entry mesure ses échéances après cette acquisition : controller2310, native2330, finalisation2380 ; RuntimeMax est plafonné à2320 et au solde native moins réserve, Stop10. Les jobs sont bornés par8, POSTE_FILS et quota effectif. L’unité préexistante est refusée avant invocation ; le stop ne concerne que l’invocation privée engagée.

Le contrôleur vérifie mémoire/swap/quota et tmpfs32G, environnement nettoyé, réseau isolé, disque privé et threads bornés. Staging60/configure120/build1800/red30 sont chacun intersectés au solde du controller. Les groupes sont TERM/KILL/récoltés, y compris leader déjà sorti, avec contrôle des résidus cgroup par phase. L’unité entière reste sous RuntimeMax/KillMode=control-group ; la finalisation est encore dans le watchdog externe. Cette cohérence statique ne remplace pas une observation native des limites.

## Artefacts et unique rouge

Avant l’appel du test, le contrôleur exporte testELF, extension vector, archive moteur, build_id.h, CMakeCache et compile_commands, avec tailles/SHA et manifeste. Header moteur attendu sans dirty, identité présente dans ELF et extension. La copie et la publication du manifeste précèdent la seule invocation ; le manifeste reçoit ensuite native_exit. Les originaux sont réhachés après test, les copies par entry avant qualification finale.

Le seul filtre est `VectorIndexDiagnosticsTest.ACompleteIndexReportsAKnownZero`, fail_fast, XML brut, exit1 autorisé uniquement pour cette phase. Les trois autres cas ne sont pas appelés. Le C++ effectue les prémisses de chargement/fixture disque/index READY avant la projection nommée. receipt exige un unique testcase effectivement joué, une seule failure sans skip/error, BUILD_ID attendu unique, assertion rows-outside-graph-is-readable et message binder exact. Un succès natif, crash, autre colonne, prémisse ou format inattendu est refusé ; aucun retry ni élargissement automatique du parseur.

Le format réel GTest/binder reste NON QUALIFIÉ. Si le futur XML diffère des fixtures fabriquées, le résultat sera un refus conservé, à relire séparément. Ce premier rouge ne prouvera ni le zéro après FIX, ni snapshot/graphe, ni postCOPY, ni absence de verrou d’écriture, ni repli NULL ou coût.

## Fin et décision

Entry requiert reçu natif, code collecteur0 et quiescence avant de reconnaître EXPECTED_BINDER_RED_OBSERVED. État systemd, PID attribués par les prédicats du pilote, rehash et inventaire sont conservés sous échéance ; scan proc limité et erreurs ignorées ne valent pas absence universelle de processus. Le résumé se termine toujours ROUGE, y compris finalisation incomplète ; un reçu absent/partiel ne peut qualifier le rouge. Code0 signifie collecte du rouge, pas livraison verte.

Aucun point bloquant trouvé pour cette préparation bornée. Restent obligatoires recette préparée exacte relue, capacité C7 neuve, puis observation native et revue des preuves ; aucun GO implicite dans cet avis. Aucune allocation, compilation, unité, test, base ou service lancé ou modifié pendant cette relecture. Aucun fichier C++ modifié.

## Complément : recette préparée PC

Publication documentaire exacte `37d03c0cbd45331e1e437a58bc8b732527abc8f8`, pilote inchangé db833d135cbceb3319424f1619f219e64bfd1fc0. Vérification indépendante RO sur luciepc, fichiers/Git seulement : recette PC SHA256 `a31745c578c38bc93a8cf25366ca72019121cede92a42b6d388f73a900aef44d`, identique octet pour octet à la recette publiée dans Git ; les quatre scripts copiés ont les empreintes attendues. Source.bundle fait bien657484714 octets, SHA256 `69688d86790e6ef6b3d5552deee8ae8af5995b492a05e73ce898c2dc2aa42ba3` recalculé ; list-heads annonce uniquement HEAD5099. Le bundle n’a pas été extrait ni exécuté par cette relecture ; bundle verify reste attesté par la préparation du porteur.

Les deux worktrees PC privés source/pilot-v1 sont aux HEAD5099/db833 exacts et `git status --porcelain` est vide. Les trois empreintes natives de la recette correspondent à la fois aux fichiers PC et aux blobs APP5099. Raw, resume.log et allocation.json sont absents dans le paquet ; le scratch cx-hotfixs-rows-red existe vide au moment du contrôle.

La recette conserve les bornes et l’unique cas relus ci-dessus. Avis favorable à cette **préparation exacte** ; l’attente de production/relecture de recette est levée. Aucune admission n’est donnée ici : un GO2400 distinct de C7 reste nécessaire, et les garanties dépendant du poste/unité ne seront observables que dans les preuves du futur run. Aucun CMake, test, poste, systemd, base, SDK magasin ou service appelé pendant ce complément.

## Complément : fixture et paquet v2

Publication exacte `a2b9ddaca886205a9b4387d7b5dfd24ce9706c66` ; APP `71fbe786fcf0b2314e72b3eb4b8484faf590d78c`, PILOTE `b307f74cc21587cb982d6ef44cdb95763f992cba`. Le refus de compilation v1 demeure un refus de fixture, pas reproduction du binder manquant. Il ne qualifie aucune correction produit ; le nouveau C++ reste NON COMPILÉ/NON JOUÉ dans cette revue.

Le seul diff hors docs depuis APP5099 est dans vector_index_diagnostics_test.cpp : la construction initializer_list du vector de LogicalType devient reserve(names.size()) puis neuf emplace_back. Les mêmes fabriques et le même ordre sont conservés ; pas de changement SQL, assertion, quatre cas, CMake ou production. Le diff des scripts depuis db833 ne modifie que SOURCE et les destinations/unité v2 ; controller.py et le corps du parseur sont inchangés. Les garanties statiques et limites de la première revue restent applicables.

Les six SHA de scripts du reçu CPU v2 correspondent aux blobs pilote ; AST seulement, aucun test synthétique rejoué. Recipe publiée et fichier PC recalculé ont le même SHA256 `37e29c250113c12bd4ff0385a1ca06590c41cfa519268c3590bf699223387867`. Les quatre scripts copiés correspondent, ainsi que les trois fichiers natifs PC/recipe/blobsAPP71. Contrôle SSH RO du bundle :657545040 octets, SHA256 `42f19dc05811589c8f4180c0c1546e85862e76a1c9f31ab769107a1d747ed280`, unique HEAD71fbe786fcf0b2314e72b3eb4b8484faf590d78c. Pas d’extraction ou de réexécution du bundle ; verify reste attesté par le porteur.

Les deux worktrees PC source-v2/pilot-v2 sont propres aux HEAD exacts71/b307. raw/resume.log/allocation.json absents, scratchrows-red-v2 vide lors de ce contrôle. Le paquet v1 n’a pas été modifié par cette revue ; sa conservation est rapportée par le porteur, pas réinventoriée ici.

Avis favorable **à la préparation exacte v2**, sans blocage statique supplémentaire. GO2400 neuf de C7 encore nécessaire avant build/test. Aucun CMake, build, test, poste, unité, SDK magasin, service ou DB appelé par cette revue. Le rouge produit binder et le format XML natif restent à observer ; un nouveau refus doit être conservé sans retry.
