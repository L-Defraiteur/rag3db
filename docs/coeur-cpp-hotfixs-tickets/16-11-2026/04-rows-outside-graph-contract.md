# Lecture des lignes hors graphe : contrat avant implémentation

Ticket : [rows_outside_graph](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md).
Porteur hotfixs ; base lue `927941a67a5805450d49f1e04b39fdd542a6059b`.
Branche `cx-hotfixs-rows-outside-graph`. Aucun changement produit, build,
lancement ou accès à une base vivante. Fenêtre de bascule réservée à optimiseur/arbre.

## Lecture qui contraint le sens du champ

`SHOW_INDEXES` capture actuellement ses lignes dans bindFunc, sous le catalogue de
la transaction appelante (`src/function/table/show_indexes.cpp:107-153`).
Ses huit colonnes existantes doivent garder noms, ordre et types. Le champ
nouveau serait ajouté en fin, UINT64 nullable : zéro signifie connu vide,
NULL signifie information non applicable/non disponible, jamais zéro fabriqué.
Ne pas charger une extension ni reconstruire un index pour lire ses diagnostics.

**Le watermark n’est pas à lui seul un compteur de lignes hors graphe.**
`searchFromUnCheckpointed` (`extension/vector/src/index/hnsw_index.cpp:675-720`)
balaie la plage après `numCheckpointedNodes`, mais dédoublonne les résultats déjà
rendus par le graphe. Son commentaire cite un rattrapage arrêté sur une ligne
non validée qui peut précéder d’autres lignes déjà reliées. Cela signale un cas à
vérifier : compter toute la plage comme strictement absente du graphe n’est pas
une égalité démontrée. Aucun contre-exemple runtime joué dans cette reprise.

De plus, les trous, lignes supprimées, vecteurs NULL et écritures non visibles
à l’instantané ne sont pas autant de lignes à embarquer dans le graphe.
`OnDiskEmbeddings::getEmbedding` vérifie visibilité puis valeur NULL
(`hnsw_graph.cpp:124-154`). Un compteur d’offsets physiques ne prouve donc pas le
nombre exact demandé, même hors concurrence.

Le produit veut estimer le coût du balayage de secours avant reconstruction.
Il faut distinguer :
- nombre exact de lignes visibles réellement absentes du graphe ;
- nombre de candidats du balayage de secours (peut inclure des lignes du graphe) ;
- longueur physique de la plage après watermark (inclut potentiellement des trous).
Le nom `rows_outside_graph` ne doit pas masquer ces différences. Le ticket demande
le premier ; ne pas livrer silencieusement le troisième sous ce nom.

## Primitives envisagées et réservations à coordonner

Pas de dépendance du moteur générique vers une classe HNSW. Si l’implémentation
exacte se confirme, préférer une capacité virtuelle de diagnostic en lecture
sur Index, par défaut non disponible, et spécialisée dans OnDiskHNSWIndex.
L’appel doit recevoir la transaction, garder l’exemplaire de l’index vivant
(capture partagée existante de NodeTable), et respecter visibilité/buildState.
`IndexStorageInfo` est un état mutable non versionné à lui seul : justifier la
synchronisation de la lecture et la cohérence avec l’instantané, sans prendre
un verrou d’écriture ni ajouter une attente de tous les écrivains.

Chemins potentiels de production, ANNONCÉS avant édition et non modifiés ici :
- `src/function/table/show_indexes.cpp` : projection, dernier champ et NULL ;
- `src/include/storage/index/index.h` : capacité commune éventuelle ;
- `extension/vector/src/include/index/hnsw_index.h` et `extension/vector/src/index/hnsw_index.cpp` : diagnostic spécialisé ;
- `src/storage/table/node_table.cpp` seulement si les primitives de capture
  existantes ne suffisent pas, à coordonner explicitement avec cœur.

Ne pas toucher `create_hnsw_index.cpp`, `cache_column.cpp`, `lockIndexForWrite`,
string/dictionary, test_hooks ou memory_manager dans ce lot. Aucun changement de
seuil de reconstruction, de format persistant ou d’activation produit implicite.

Consommateurs relus : rag3weaver `generic_search_nodes.rs:208-234` lit les colonnes
par nom, `dialect.rs:2060` fait RETURN *, `backend.rs:2849` sélectionne
extension_error. Prévenir arbre principal avant tout changement de projection ;
ces lectures ne prouvent pas que tous les autres clients sont compatibles.

## Témoin privé initial et progression proposée

Préparer d’abord un fichier de test séparé
`test/transaction/vector_index_diagnostics_test.cpp`, inscrit à transaction_test
par une seule entrée de son CMakeLists (coordination cœur). Premier cas :
index construit sur une petite table à vecteurs manuels, lecture explicite de
rows_outside_graph, résultat non NULL égal à zéro. Échec attendu sur master :
colonne absente au binder. Ce cas vérifie l’API demandée, pas le calcul d’une
queue non vide. Il doit échouer si l’extension n’a pas été bâtie, pas se sauter.
Aucun rouge annoncé avant exécution effective autorisée, aucun correctif avant lui.

Avant d’implémenter le compteur, arrêter la définition avec orchestration/arbre,
puis préparer les cellules discriminantes : queue non vide ; vecteur NULL ;
suppression et trous ; instantané ouvert avant une écriture concurrente ; index
NOTED/INCOMPLETE/détaché ou extension absente ; lecture sous transaction READ ONLY
sans nouveau verrou d’écriture ; reconstruction qui ramène le compte exact à zéro.

Le témoin existant `VectorIndexUpdate.AForcedCopyCommittedRightAfterACreateIsReached`
(`test/transaction/concurrence/vector_index_update_test.cpp:2229`) couvre le
chemin de départ, mais utilise20k lignes et300ms de sommeil : il n’est pas une
barrière déterministe prouvée pour notre nouveau compteur. Ne pas recopier sa
charge ou son sommeil comme preuve de mise en place. Lire ses primitives de
fixture et les autres accès internes avant de proposer une injection nouvelle.

Si le contrat exige une appartenance exacte et que l’état existant ne permet pas
de la reconstruire fidèlement à coût acceptable, remonter ce manque : un état
versionné supplémentaire aurait un coût mémoire, durabilité et annulation à
justifier. Ni bitmap neuf ni champ persistant ajouté par défaut pour débloquer
une simple colonne de diagnostic.

Validation future (non lancée) : rouge du seul témoin sur base actuelle,
validation de la sémantique, correctif puis vert ; transaction_test et tests
SHOW_INDEXES impactés, avec BUILD_ID écrit par le binaire joué. Aucun lourd
pendant la fenêtre de bascule. Ticket ouvert ; cette note n’autorise aucun run.
