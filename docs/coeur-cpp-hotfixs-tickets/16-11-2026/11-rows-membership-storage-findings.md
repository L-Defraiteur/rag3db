# Membership HNSW : lecture du stockage, sans modification

Suite de la proposition10/ebcd3d46df, même ticket08nov. Source figée71fbe786,
moteur59ac ; uniquement fichiers source, aucune DB/proc/commande native.
Aucun chemin réservé, aucune analyse de C1 ou du crash chaînes. Les références
ci-dessous sont celles de ce SHA, à relire lors d'un rebase.

## Preuves locales de la frontière manquante

1. **Singleton sans relation réel.** hnsw_index.cpp:36, InMemHNSWLayer::insert :
   première insertion pose l'entrypoint atomique puis retourne sans arête.
   Même mécanisme disque à :1720, OnDiskHNSWIndex::insertToLayer : quand aucune
   racine vivante ne reste, écrit lower/upperEntryPoint=offset puis return,
   sans RelTable::insert. Ce n'est donc pas une hypothèse d'un graphe abstrait.
2. **CSR ne garde pas une marque de sommet HNSW.**
   hnsw_rel_batch_insert.cpp:70, populateCSRLengths : initialise TOUTES les
   longueurs à0 puis remplace par le degré des offsets sélectionnés. Le degré
   d'un singleton est aussi0. writeToTable:99 réserve et écrit des lignes de
   relations pour les voisins seulement. InMemChunkedCSRHeader::fillDefaultValues
   (src/storage/table/csr_chunked_node_group.cpp:304) ajoute également des
   longueurs0 pour les valeurs nouvelles ; copyFrom:375 remplit le reste à0.
   La présence d'un emplacement CSR ou d'un offset n'est donc pas le bit
   « incorporé au HNSW ». Le MVCC des arêtes ne rétablit pas un bit absent.
3. **Le mapping du bâti n'est pas une preuve persistée.**
   NodeToHNSWGraphOffsetMap (hnsw_graph.cpp:353) connaît les offsets sélectionnés,
   puis moveToPartitionState (hnsw_index.cpp:346) transfère graphe et map au
   partitionneur. HNSWLayerPartitionerSharedState::resetState
   (hnsw_rel_batch_insert.cpp:26) détruit graph ET graphSelectionMap. Le mapping
   décrit le bâti, pas les commits ultérieurs ni les snapshots anciens.
4. **Format lu : cinq scalaires, pas de membership MVCC.**
   HNSWStorageInfo::serialize/deserialize (hnsw_index.cpp:474) écrit/lit les deux
   IDs de relations, les deux entrypoints et numCheckpointedNodes. CREATE
   (create_hnsw_index.cpp:375) construit précisément cette information, puis
   complète le holder et organise le rattrapage avant commit. Aucun ensemble
   de membres n'est transmis dans ce format. C'est une observation des
   structures inspectées, pas un audit de tous les formats du moteur.
5. **L'API Graph compte les lignes de table.**
   src/graph/on_disk_graph.cpp:168 : getMaxOffset appelle getNumTotalRows(tx).
   getNumNodes:173 somme ces maxima, ou les masques fournis par l'appelant.
   Ce nombre n'est pas |M|. prepareVertexScan utilise la table de nœuds ;
   scanner les vecteurs reconstruirait E, sans prouver leur incorporation.
6. **Les métadonnées courantes ne suffisent pas pour un vieux lecteur.**
   commitInsert (hnsw_index.cpp:775) incorpore puis écrit watermark=offset+1 ;
   rollbackInsert:798 tronque le watermark et invalide des racines. Les arêtes
   ont leur annulation propre. findLiveNode:1039 cherche une ligne éligible
   sous le watermark, et repairEntryPoint:1082 peut sélectionner une racine
   depuis les candidats/lignes. Ces chemins utilisent des invariants de
   maintenance ; ils ne publient pas une version complète de membership par
   snapshot. Verrouiller les scalaires actuels ne restitue pas leur histoire.

Conclusion de lecture : aucune marque COMPLETE/versionnée de membership n'est
établie dans ces chemins. Le singleton démontre pourquoi « pas d'arête » ne
suffit pas. Des endpoints d'arêtes prouvent une présence représentée mais une
énumération d'endpoints ne prouve pas la complétude de M. L'accessibilité ANN
ne remplace pas ce contrat. Aucun zéro ou positif exact n'est déduit ici.

## Profondeur d'un correctif possible, à arbitrer

Une capacité exacte générale exige de conserver la différence entre
« sommet incorporé, degré0 » et « jamais incorporé », et de rendre cette
information à la version de l'appelant. Une nouvelle colonne SQL seule ou
un mutex autour de SHOW_INDEXES ne la crée pas.

Option à étudier, pas choisie : ensemble d'appartenance explicite par instance,
porté par une table interne ou un stockage versionné existant. Utiliser les
transactions/MVCC/WAL/récupération existants plutôt qu'un fichier annexe ou un
bitmap RAM ayant ses propres règles. L'identité doit être liée aux IDs de
l'instance, y compris DROP/CREATE homonymes. Le singleton obtient une entrée
même sans arête ; l'insertion ne la publie qu'avec l'incorporation réussie.

Ce choix doit couvrir : CREATE/batch et son rattrapage, INSERT/COPY/commit,
UPDATE vers/de NULL et suppression, rollback (y compris restauration après
UPDATE), abandon de bâti, DROP, checkpoint, réouverture et lecteurs anciens.
Une propriété sur la table des nœuds seule confondrait plusieurs index ; une
self-relation ajoutée au graphe changerait ses algorithmes sans contrat relu.
Le coût est au moins une maintenance par changement de membership, un espace
proportionnel aux membres/versions retenues et les coûts de journal/récupération.
Il ne peut pas être présenté comme « lecture gratuite d'un compteur existant ».

Périmètre forcément plus profond que proposition10 si cette option est retenue :
mutateurs hnsw_index.cpp, bâti create_hnsw_index.cpp et partitionneur
hnsw_rel_batch_insert.*, identité/catalogue HNSW et stockage/sérialisation si
nécessaires. Ce sont des zones À COORDONNER, pas une réservation ni une
permission d'y éditer. Les chemins exacts et la compatibilité ne seront fixés
qu'après choix de la primitive persistante. Aucun changement lock_manager,
chaînes/dictionnaire ou autre lot cœur proposé par cette lecture.

## Variante NULL et compatibilité, sans faux achèvement

Sans certificat complet correspondant à l'instance et au snapshot : NULL.
Les index existants peuvent rester sans capacité, explicitement ; aucune
reconstruction/catch-up/writerlock implicite pendant SHOW_INDEXES. Un parcours
permettant exceptionnellement de prouver un compte devrait publier son
certificat et ses préconditions, jamais se généraliser à partir de quatre0.

Ajouter la colonne et rendre NULL faute de capacité peut constituer une étape
API à discuter, mais PAS clore ce ticket : les quatre attentes0 restent telles
quelles et le positif réel reste à construire. Aucun compromis « nom exact,
valeur tail candidates » proposé. Si le consommateur veut entre-temps un
nombre de candidats fallback, c'est un autre nom et un autre contrat.

Prochaine décision réelle cœur/C7 : accepter d'étudier un membership versionné
persistant avec maintenance/compatibilité, ou garder ce diagnostic indisponible
et le ticket ouvert. Aucun résultat de performance, allocation ou nouveau
runtime demandé par cette lecture ; priorité migrations Arbre inchangée.
