# Compte hors graphe : proposition de capacité et de chemins, avant production

Préparation autorisée par C7 ; proposition à relire, aucune réservation acquise,
aucune édition de production, aucun build/rejeu/SDK/service. Lecture source
figée sur 71fbe786fcf0b2314e72b3eb4b8484faf590d78c, moteur59ac.
Le rouge binder acquis sous b307 reste archivé dans le document09/2508291714.
Les quatre fixtures existantes restent inchangées ; trois sont compilées non jouées.

## Sens et frontière publique

Pour l'instance I désignée par le catalogue du snapshot S appelant :
E(S) = offsets de lignes visibles portant un vecteur valide selon l'index ;
M(I,S) = ensemble complet des offsets effectivement incorporés au graphe I à S.
La valeur est |E(S) privé de M(I,S)|. Appartenance et accessibilité depuis une
racine sont deux propriétés différentes. Ce contrat ne mesure ni le rappel
ANN, ni les résultats d'une recherche, ni les candidats physiques du fallback.
Une mise à jour d'un vecteur déjà membre ne signifie pas à elle seule « absent » ;
la fidélité des vecteurs et la réparation de l'index sont d'autres diagnostics.

Neuvième colonne `rows_outside_graph`, UINT64 nullable, huit noms/positions/types
existants inchangés : table_name STRING, index_name STRING, index_type STRING,
property_names STRING[], extension_loaded BOOL, index_definition STRING,
state STRING, extension_error STRING. NULL signifie connaissance insuffisante,
pas zéro ; aucune substitution par rows-watermark ou COUNT physique. Une autre
mesure de candidats au fallback doit porter un autre nom et un autre contrat.
Lecture consommateur réelle par nom à relire au SHA rebasé ; aucun chemin Rust
produit/catalogue/generic_search_nodes réservé ou modifié dans cette proposition.

## Ce que les primitives actuelles garantissent

- NodeTable::captureVisibleIndexes garde un shared_ptr et filtre visibilité MVCC,
  chargement et READY sous indexesMtx ; le mutex est relâché au retour. Cela
  garantit la durée de vie, pas l'immuabilité du graphe ou de ses racines.
- OnDiskHNSWIndex::captureForQuery reconnaît l'instance par les deux identités
  upperRelTableID/lowerRelTableID. Réutiliser cette règle, jamais le premier nom
  correspondant après DROP/CREATE ; conserver aussi l'identité du holder capturé.
- OnDiskEmbeddings::getEmbedding utilise visibilité/lookup de la transaction et
  rejette NULL ; réutiliser ce chemin et les règles de validité effectives de
  l'index, sans nouvel accès brut ni snapshot LatestCommittedView/DUMMY.
- numCheckpointedNodes et les entrypoints sont des métadonnées mutables non
  versionnées. La plage du fallback peut contenir des membres déjà incorporés.
  Un watermark à zéro ne prouve pas une seule absence.
- LockManager expose acquire et releaseAll, des verrous de transaction, sans
  libération isolée. lockIndexForWrite peut rafraîchir le snapshot. Ni cette
  fonction ni releaseAll ne convient à une observation READ ONLY : releaseAll
  pourrait relâcher les verrous préexistants de l'appelant.

## Primitive proposée : observation d'appartenance au snapshot

Contrat interne commun sur Index, nom de travail `tryReadMembershipSnapshot` :
retourne une observation possédée, attachée à l'identité d'instance et au snapshot
appelant, ou une absence de capacité. L'observation fournit une énumération
COMPLÈTE de M, avec preuve de complétude et budget partagé ; une liste partielle
ne produit jamais de nombre. Un index non chargé/non READY/sans capacité rend
NULL. Le cœur ne connaît aucune classe HNSW, aucun watermark ni format d'arêtes.
Le détail interne du refus peut distinguer indisponible/budget/cohérence, sans
ajouter une dixième colonne ou masquer une erreur réelle d'I/O/corruption.

Préférer un ensemble versionné lu avec les primitives MVCC existantes : il
sépare identité d'instance, version de membership et vecteur visible, évite un
nouveau verrou lecteur et reste utilisable par d'autres diagnostics. Le bail
possédé garantit lifetime et cohérence de l'observation, sans écrire, rafraîchir
le snapshot ou garder un verrou jusqu'au COMMIT. Publication/remplacement de
l'observation doit être atomique ; un epoch relu après une lecture non protégée
de champs non atomiques ne suffit pas (data race C++).

**Capacité manquante, pas implémentation déjà trouvée :** la lecture actuelle
n'a pas établi de source exhaustive de membership MVCC. Des arêtes incidentes
prouvent une présence, mais une absence d'arêtes ne prouve pas l'absence du graphe
(sommet isolé). Une exploration depuis l'entrypoint mesure l'accessibilité ;
aucun invariant de connexité totale n'est démontré ici. Ces raccourcis sont
écartés. Une simple garde des racines actuelles ne restaure pas leurs anciennes
versions pour un lecteur ancien.

Proposition pour la suite : examiner d'abord si le stockage de relations possède
une marque de présence de sommet déjà versionnée et exploitable, y compris pour
le singleton, indépendamment des arêtes. Si oui, exposer cette lecture via la
capacité commune et prouver sa complétude. Sinon il faut un membership explicite
versionné, maintenu atomiquement avec insertion/suppression/UPDATE/reconstruction,
commit/rollback et reprise disque. Cette seconde option élargit le lot aux
écritures et à la persistance : revenir avec les chemins exacts, le format,
les migrations et l'accord cœur/C7 avant de l'implémenter. Un bitmap RAM courant
ou un cache reconstruit depuis les vecteurs ne tient pas ce contrat. Les anciens
index sans preuve d'appartenance complète rendent NULL ; compatibilité et moyen
d'obtenir la capacité sans reconstruction silencieuse restent à décider.

Une colonne NULL partout serait un échafaudage, pas la livraison du ticket :
elle ne satisfait pas les quatre attentes zéro. Aucun correctif const0 ni
assouplissement des attentes existantes pour contourner cette frontière.

## Coûts et refus borné proposés

Lecture exacte potentiellement linéaire : O(N + M) avec membership trié,
N offsets examinés (trous compris), M membres énumérés ; une construction depuis
les arêtes ajouterait O(A). Ne pas annoncer O(queue) ou un coût constant.
Proposition initiale de plafond interne PAR APPEL SHOW_INDEXES, partagé entre
index : 65 536 offsets, 1 048 576 éléments d'appartenance/arêtes, 16 Mio
supplémentaires, contrôle d'annulation et d'échéance de requête entre lots.
Ces nombres sont une politique à relire/mesurer, non un résultat de performance.
Dépassement avant de démarrer un lot => NULL pour le compte non achevé ; jamais
un compte partiel. Pas de thread de rattrapage ni I/O de maintenance implicite.
Un budget d'éléments ne borne pas à lui seul la latence d'une page disque :
aucune promesse de temps mural strict sans garantie d'annulation des lectures.
Ne pas retenir un verrou d'écriture pendant un scan ; préférer une observation
MVCC stable. Une garde nécessaire doit être courte/non bloquante et coordonnée
avec TOUS les mutateurs, sous peine de ne fournir aucune preuve.

## Chemins proposés à réserver après revue

Premier périmètre précis, pas de réservation acquise :

- src/include/storage/index/index.h : contrat commun d'observation/capacité et
  défaut indisponible ; sans dépendance du cœur vers vector.
- src/include/storage/table/node_table.h et src/storage/table/node_table.cpp :
  capture liée au holder/snapshot sans modifier les chemins d'écriture existants.
- extension/vector/src/include/index/hnsw_index.h et
  extension/vector/src/index/hnsw_index.cpp : fournisseur HNSW de l'observation,
  identité des relations, refus explicite si la preuve n'existe pas.
- src/function/table/show_indexes.cpp : neuvième valeur nullable et orchestration
  de lecture à la transaction appelante ; pas de donnée périmée acquise lors
  d'une préparation puis réutilisée à une exécution ultérieure.
- Nouveau test/transaction/vector_index_membership_diagnostics_test.cpp et
  test/transaction/CMakeLists.txt : témoins supplémentaires dans un fichier
  séparé, quatre fixtures historiques inchangées.

Lecture seule supplémentaire autorisée : src/include/graph/on_disk_graph.h,
src/graph/on_disk_graph.cpp, stockage des relations et hnsw_graph.*. Aucune
réservation d'écriture sur ces chemins. Ni lock_manager.*, create_hnsw_index.cpp,
cache_column.cpp, memory manager, string/dictionary ou hooks C1 réservés.
Si membership persistant requis, ce premier périmètre est insuffisant : pas de
passage à une modification cachée de CREATE/COPY/commit/checkpoint.

## Nouveaux témoins à publier avant correctif, puis voir rouges

1. Positif réel : graphe complet puis lot COPY validé dont un offset éligible
   n'est pas incorporé. Barrière déterministe au vrai chemin d'incorporation,
   sans sleep ; prouver séparément membership réel et lignes visibles ; attendu
   exact >0 puis 0 après incorporation autorisée. Réserver la barrière/test-only
   avec cœur avant toute instrumentation. Aucun changement des quatre témoins.
2. Même lot : NULL, suppression/trou et vecteur valide ; seul le valide visible
   absent compte. Cas singleton et sommet sans arêtes : leur présence ne dépend
   pas du degré ou du résultat ANN.
3. Transaction READ ONLY ancienne : commit concurrent d'insertion, NULL et delete ;
   ancien lecteur garde son ensemble E et M, nouveau lecteur voit le nouveau.
   Ajouts/annulations propres d'une transaction d'écriture suivent son snapshot
   sans compter les données non validées d'autrui. Zéro verrou retenu au retour.
4. DROP/reCREATE homonyme concurrent : ancienne instance ou NULL justifié,
   jamais mélange des deux identités relationnelles ; ancienne occurrence vivante.
5. Capacité absente, extension non chargée, NOTED/INCOMPLETE ou observation non
   cohérente => NULL, pas0. I/O réelle en erreur reste une erreur, pas NULL.
6. Budgets épuisés, annulation, table très trouée : pas de résultat partiel,
   aucune écriture/maintenance/snapshot refresh ; plafond partagé entre index.
7. Schéma réel : huit anciens champs stables + neuvième nullable, lectures par
   nom/projection et préparation réutilisée dans deux snapshots distincts.

Le positif réel et les interleavings ne sont pas encore construits ni joués.
Le choix de la source exhaustive de M est le prochain point de revue source,
pas un GO production. Porteur hotfixs ; coordination cœur pour les primitives et
C7 pour le périmètre. Migration trois bases/Glib/pose prioritaire côté Arbre.
