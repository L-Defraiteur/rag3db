# Prochain changement minimal proposé : fusionner seulement des undo adjacents prouvés équivalents

Porteur cœur C++, [même ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Lecture et proposition seulement : **aucun changement moteur, build, test natif ou mesure supplémentaire**.

## Ce que les mesures permettent de viser

Le nouveau retrait parcourt l’index local entier une fois par plage. Les points
à Rconstant confirment le coût proche de N×U pour ce mécanisme, pas une cause
de la pression DDL. La correction de sûreté — retirer par identité de ligne,
sans recopier les chaînes dans un tampon neuf — reste à préserver.

Le prochain changement proposé vise d’abord le nombre de parcours, en réutilisant
les descripteurs d’insertion déjà présents dans l’undo. Il ne réintroduit pas
un scan matérialisant des clés ni une réserve mémoire forfaitaire.

## Lecture de code qui contraint le plan

- `UndoBuffer::rollback` parcourt les enregistrements en sens inverse et marque
  `started` avant les effets ; cet ordre doit être conservé aux barrières.
- `VersionRecord` contient déjà startRow, numRows, nodeGroupIdx et le handler.
- `NodeTableVersionRecordHandler::rollbackInsert` retire les clés primaires,
  annule les index secondaires, retire les versions/rangs, puis recule le compte
  de la collection pour toute la réservation, même partiellement écrite.
- `UndoBufferIterator::reverseIterate` construit déjà un `std::vector` système
  d’entrées **par tampon d’undo**. Il serait faux de promettre aujourd’hui que
  le rollback intégral ne fait aucune allocation système. Le retrait des clés
  ne doit demander aucun nouveau bloc au pool ; le regroupement ne doit ajouter
  aucune allocation. L’allocation système préexistante de l’itérateur reste
  une limite à traiter si un contrat global zéro-allocation est exigé.
- `createUndoRecord` réserve sous mutex, mais les appelants écrivent le payload
  après la réservation : fusionner à la création demanderait de revoir cette
  publication/concurrence, et toucherait aussi le commit. Ce n’est pas le premier
  changement proposé.

## Option A recommandée pour revue : regroupement local au parcours inverse

À partir de la liste d’entrées déjà présente pour **un tampon d’undo**, regarder
les INSERT_INFO consécutifs et former un seul descripteur scalaire temporaire,
sans modifier le journal existant ni le parcours de commit.

Fusion autorisée seulement si toutes les conditions sont établies :

1. Handler explicitement apte à ce regroupement (défaut refus pour les autres
   handlers ; premier candidat NodeTable, pas d’hypothèse sur les relations).
2. Même handler exact, même groupe de nœuds, même portée d’annulation.
3. Plages non vides, disjointes et exactement adjacentes dans l’ordre inverse ;
   contrôles de bornes sans débordement. Aucun trou ni plage d’un autre écrivain.
4. Aucun UPDATE, DELETE, catalogue, colonne, index ou autre type intercalé.
5. Aucune traversée de frontière de tampon d’undo : il faut préserver l’ordre
   vis-à-vis de la construction potentiellement faillible de la liste suivante.
6. Équivalence de **tous** les effets du handler démontrée : clés, secondaire,
   versions/troncature et somme des réservations, pas seulement des clés primaires.

L’itérateur générique doit rester un parcours d’enregistrements ; préférer un
petit parcours de rollback local par tampon ou une vue de ses entrées existantes,
avec une capacité sémantique d’opt-in du handler, plutôt que changer silencieusement
la signification de reverseIterate pour tous les futurs appelants.

Le descripteur fusionné est sur la pile et de taille constante. `started` est
marqué avant le premier effet appliqué, pas seulement parce qu’un descripteur
est examiné ; aucun recul de la garde d’annulation à moitié faite. Les frontières
ne sont jamais déplacées pour gagner une fusion. Les enregistrements originaux
restent ceux du commit et du diagnostic.

**Limite explicite** : cette option améliore les suites contiguës dans un même
tampon. Elle ne supprime pas N×U lorsque les plages sont fragmentées ou séparées
par des barrières. Elle n’est donc pas présentée comme une solution générale
au scénario de vingt écrivains, encore moins comme son correctif validé.

## Invariants à prouver avant opt-in NodeTable

- Aucun nouvel appel au pool ou copie de chaîne lors du retrait ; aucune allocation
  de structure de regroupement pendant rollback.
- Retirer exactement les clés des réservations annulées ; préserver les clés
  validées, doublons vers un survivant et insertions d’un autre écrivain.
- Compte de rangs reculé de la somme des réservations, écrites ou non ; même
  état final de versions/troncature et index secondaires qu’en parcours unitaire.
- Ordre inchangé autour des autres types d’undo ; autres handlers inchangés.
- Échec au même type de frontière : pas de succès apparent ni suppression
  d’exception après des effets partiels ; BEGIN/relecture/réouverture vérifiés.

Si l’équivalence des index secondaires ou des versions ne tient pas, **ne pas
activer l’opt-in**. Ne pas ajouter un booléen qui saute arbitrairement une moitié
du handler, ni garder des vues de chaînes sans preuve de durée de vie/verrouillage.

## Preuves qui départagent A et les alternatives

À préparer seulement après revue du plan, pas exécutées ici :

- Témoin structurel rouge/vert : 64 INSERT_INFO adjacents du même handler/groupe
  dans un tampon, nombre de parcours PK attendu réduit à un, sans seuil d’horloge.
  Compteur/trace dans le témoin privé, pas une API de produit créée pour un banc.
- Contrôles de barrières : trou, autre handler/groupe, UPDATE/DDL/index intercalé,
  tampon différent et bornes ; chaque cas doit refuser la fusion et préserver
  l’ordre. Un cas fragmenté avec rangs survivants dans les trous ne doit jamais
  être traité comme une plage pleine.
- Fixture réelle longue STRING sous pool plein (le témoin rouge d’origine),
  voisin INT64, doublons vers rangs validés, plusieurs COPY, clés réutilisables,
  index secondaire si présent, BEGIN/relecture et réouverture.
- Cas de réservation partiellement écrite/refusée : compte et index doivent
  correspondre à la suite unitaire ; ce n’est pas un simple cas de réussite.

Si les suites réellement coûteuses se révèlent surtout fragmentées, A est
insuffisante. L’alternative B serait un retrait par ensemble de plages en un
parcours, avec métadonnées **préparées pendant l’écriture avant publication**
et sans allocation pendant l’annulation, tout en rejouant les autres effets
dans leur ordre. Elle exige davantage de données et une preuve d’échec atomique
à leur allocation : pas à ajouter par défaut avant le témoin discriminant.

Une optimisation par clés empruntées aux colonnes n’est pas retenue d’emblée :
il faudrait prouver résidence, durée de vie des dictionnaires et ordre des verrous,
et garder un fallback sans demande de page pour le cas non résident. Ce serait
plus large que cette première proposition.

## Conditions de fin du prochain sous-travail

Un regroupement ne serait accepté qu’avec équivalence démontrée et le témoin
structurel rouge avant/vert après, puis les validations de sûreté pertinentes.
La mesure de performance ultérieure doit être proposée séparément après revue ;
les quatre points actuels ne donnent aucune autorisation supplémentaire.
Même ticket ouvert, aucun tour de push/lib/mesure demandé ici.

## Restriction après lecture du scénario vingt écrivains

La [lecture sourcée du scénario](11-adjacence-undo-du-scenario-vingt-ecrivains.md)
ne trouve qu'au plus une plage Doc ou Note par transaction ordinaire. Le choix A
n'est donc pas retenu comme prochain correctif de ce scénario sur la seule preuve
micro N×U. Les éventuelles séries de relations du DDL ont d'autres handlers et
leur adjacence réelle n'est pas consignée dans les traces conservées.

## Revue croisée des invariants — opt-in non démontré

La revue en lecture seule de 0511dece7 identifie des obstacles supplémentaires :

- `node_table.cpp:97–115` entrelace PK, index secondaires, versions/troncature et
  compte par record. Fusionner le handler entier change cet entrelacement.
  L'union des plages PK conserve survivants et compte, sans garantir la même
  représentation physique des slots.
- `node_table.cpp:1037–1049` reprend indexesMtx et filtre les index chargés,
  détachés et visibles par appel. `Index::rollbackInsert` (index.h:191) n'a pas
  de contrat de composition ni noexcept. HNSW actuel compose un seuil minimal
  (hnsw_index.cpp:798–810), mais cela ne certifie pas tous les index : une
  implémentation comptant les appels ou levant au second seuil est permise.
  L'opt-in du seul type NodeTable ne suffit donc pas.
- `node_group.cpp:1065–1090` visite les blocs dans l'ordre croissant. Deux undo
  adjacents sont annulés haut puis bas ; leur union visite bas puis haut.
  `ChunkedNodeGroup::rollbackInsert` (:594–619) traite versions puis troncature ;
  `VersionInfo::rollbackInsert` (:621–635) peut passer par getOrCreateVersionInfo
  (:412–419, resize/make_unique). En cas d'exception dans le bloc bas, le préfixe
  des effets peut différer. C'est un contre-exemple conditionnel, pas un incident
  observé. L'équivalence exacte des échecs n'est pas établie.
- Conserver started avant le premier effet, les frontières de tampon et les
  barrières : UndoBuffer construit déjà un vector par tampon avant callbacks.
  Transaction::rollback transforme started en halfDoneRollback ; le manager
  interdit alors checkpoint et impose une réouverture.
- La soustraction des réservations se compose seulement avec bornes sans overflow
  et total initial au moins égal à la somme. Les réservations partiellement
  écrites, groupes absents et versions retenues par lecteurs restent à prouver.

Aucun opt-in n'est recommandé à ce stade. Cette revue, ajoutée à l'absence de
série Doc/Note à fusionner dans TwentyWriters, retire la justification d'un
passage immédiat à l'implémentation A. Aucun nouveau run ni changement moteur.
