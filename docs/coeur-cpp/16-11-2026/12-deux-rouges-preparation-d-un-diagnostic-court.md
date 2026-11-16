# Deux rouges : préparation d'un diagnostic court

**Préparation uniquement, aucun C++ compilé ni joué.** Suite au refus global888, C7 demande un diagnostic distinct, sans réessai identique ni changement de production. Tickets déjà publiés sur cx-coeur-dictionary-append-atomic, dernier suivi eda793fcec : 2026-11-16-le-controle-des-chaines-apres-des-add-et-drop-releve-228-anomalies.md et 2026-11-16-le-point-de-reprise-finit-avant-la-lecture-censee-l-observer.md. Ils restent ouverts ; cette préparation ne les ferme pas.

## Paire concrète publiée

[Recette et empreintes communes](preuves/two-reds-preparation/recipe.json).

- Référence : source `50412d9c501708d8464efd85e94273fdf91e7f80`, branche cx-coeur-two-reds-baseline, base68590270e21d00285b29800bd9cdecf0cb921461, moteur attendu59ac9c86bc2b3529b590a7462ab7051e3ad26b8a.
- Candidat : source `a3f3e3bce46de8864f140ae21c2dff6b539bf838`, branche cx-coeur-two-reds-diagnostic, base888c81825e4cd6556261233f0e238af3b95375c9, moteur attendu888c81825e4cd6556261233f0e238af3b95375c9.

Les deux fichiers de tests modifiés sont identiques octet pour octet. Dans les chemins réellement utilisés par build_id.cmake, le seul delta entre ces deux sources est dictionary_chunk.cpp (correctif append). Aucune instrumentation dans src, aucun changement de known_red/probabilistic/long. Les worktrees sont privés ; aucun SDK ou service touché. Les sources de tests différentes de la passe globale sont annoncées, sans attribuer rétroactivement un rouge à ces sondes non jouées.

## Chaînes : distinguer instructions, checkpoint et accès

Filtre exact `StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops`, environnement explicite RAG3DB_STRING_DIAGNOSTIC=1. Le même témoin et ses assertions originales restent. Sans ce drapeau, pas de nouvelles lectures ni remplacement de checkpointer.

La sonde observe après chargement, avant/après chaque UNWIND de64, avant/après chaque ADD/DROP, après les checkpoints automatiques, après checkpoint explicite et après réouverture. Attendu partiel : préfixe déjà réécrit au tour1, suffixe encore au tour0, marqueurs correspondants. Les données, tailles, ordre des écritures et lecteur de fond restent ceux du témoin ; les lectures de diagnostic ajoutées changent néanmoins l'ordonnancement, limite importante.

Chaque étape compare lecture SQL sans WHERE et lecture avec WHERE préfixe. Pour les quatre premières anomalies de chaque mode, elle relève clé, valeur NULL ou longueur/FNV-1a64/hex intégral attendu et lu, marqueur attendu et lu, puis sonde la même clé. Le hash n'est pas une preuve d'identité : les octets complets sont conservés. Cardinalité et valeurs fausses restent des échecs explicites. Les sondages par clé sont ciblés sur les anomalies ; aucune prétention à une vérification exhaustive par clé, ni à un accès physique indépendant du plan SQL.

FlakyCheckpointer et checkpointStorage virtual existent déjà : la sonde test seule journalise séquence et instruction courante immédiatement avant/après le stockage de chaque checkpoint. **Aucune requête SQL réentrante dans le checkpoint.** Le relevé de valeurs « avant » est avant l'instruction pouvant provoquer le checkpoint, pas une lecture interne juste avant son déclenchement. Cette distinction doit rester dans le verdict. Les séquences donnent les frontières réellement atteintes ; elles ne prouvent pas que l'erreur naît à cette frontière.

## Lecteur : fenêtre contrôlée distincte

Nouveau filtre exact `CheckpointOpenReader.OpenReaderOnRelsAcrossAControlledCheckpointWindow`. Le témoin historique au sleep30ms reste inchangé, ni supprimé ni reclassé.

La primitive test seule FlakyCheckpointer installe un Checkpointer qui appelle le stockage normal, annonce une barrière puis attend sa libération au maximum10s. Le test attend au maximum5s l'entrée, vérifie la barrière tenue avant/après la lecture de la transaction déjà ouverte, libère puis joint le checkpoint. Expiration ou fenêtre non atteinte donne un échec de plan explicite. Les données et modifications de relations sont celles du témoin historique. Assertions séparées : écritures visibles ailleurs, checkpoint réussi, instantané pendant/après identique à l'ancien.

Cette fenêtre est **après checkpointStorage et avant la suite du checkpoint** ; elle prouve une lecture pendant un checkpoint suspendu à cet endroit, pas pendant des E/S ni pendant applyShadowPages. Elle ne ferme pas à elle seule une garantie de concurrence sur toutes les phases. Les pauses sont bornées et la jointure est faite avant les assertions fatales du plan. Pas de sleep augmenté pour rendre vert le témoin historique.

## Proposition bornée, pilote à publier avant admission

Un vrai poste lourd pour toute la paire, fenêtre neuve globale2400s arrêt/finalisation inclus, mémoire32Gio/swap0, scratch privé32Gio, réseau absent, sources en lecture seule et extensions privées, j=min(8,POSTE_FILS,quota effectif). Mêmes options Release/rcOFF/stOFF/ASANOFF/TSANOFF/UBSANOFF/LTOOFF, mêmes données et environnement tests MAX_NUM_THREADS2, OMP/OPENBLAS bornés au poste.

Par variante : configuration120s, compilation800s du seul concurrence_test (sans extensions si aucune requise), chaînes60s puis lecteur60s ; deux variantes et320s réservées à l'arrêt/artefacts. Aucun banc complet, transaction_test, E2E, Cargo ou service. Une exécution par cas ; aucune répétition automatique. La comparaison doit conserver les deux variantes même en cas de rouge GTest attendu, uniquement si celui-ci est un résultat complet et recevable : un refus de ressources, identité, timeout, crash ou résidu arrête toute la paire. Cette politique diagnostique doit être validée explicitement dans le pilote, elle ne réutilise pas le verdict vert global.

Le pilote exact doit encore être préparé, publié et relu RO avant toute admission : limites globales/cgroups, source et BUILD_ID attendus distincts59/888, paquet source et trois étapes d'identité (avant build, ELF/header, stdout natif), export avant assertions, XML et logs complets, arrêt et réempreinte après quiescence. Aucun GO ne peut être déduit de ce document. Ne pas reprendre le pilote global0de tel quel : ses filtres, cibles et première erreur de phase ne correspondent pas à cette comparaison.

## Contrôles réellement faits

Format clang18 des deux fichiers, git diff --check, égalité des deux fichiers sur la paire et delta moteur limité à dictionary_chunk.cpp. **Ni syntaxe C++ compilée ni résultats natifs**, donc aucun cas annoncé vert. Revue Hotfixs demandée avant de figer le pilote.

## Réserve RO lecteur levée dans les sources, exécution encore absente

Hotfixs a relevé qu’une égalité de trois messages ERREUR pouvait satisfaire les comparaisons d’instantané. La paire révisée exige le succès explicite des quatre requêtes before/during/after/others, exactement deux colonnes non nulles et une seule ligne, ainsi que COMMIT réussi. Elle exige aussi les valeurs initiales300000|45000150000 et les valeurs de l’écrivain300001|45008149994, issues des données fixes et des deux modifications ; les valeurs pendant/après doivent égaler l’initial. Une erreur marque GTest en échec même si le texte retourné est identique. Les sources antérieures448/c4eb sont remplacées, pas amendées. Format18/diff contrôlés, aucun nouveau résultat compilé ou natif. Nouvelle RO demandée.
