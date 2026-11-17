# Membership persistant : options et recommandation, avant réservation

Arbitrage C7 ef10c00f62, suite des documents10/ebcd et11/6b83, même ticket08nov.
Préparation documentaire exclusivement. Références source71fbe/moteur59ac ;
à porter et relire sur une base fraîche avant implémentation. Aucun chemin
réservé, code produit, build, base, SDK ou natif lancé. Hors périmètre C1.

## Décision proposée

Recommander un **ensemble de membres par instance dans une table interne MVCC**,
référencée par l'information persistée HNSW. Il représente l'incorporation
réussie, y compris singleton sans arête ; pas la qualité ANN ni la fraîcheur
sémantique des vecteurs. Les racines HNSW courantes ne participent plus au calcul
d'appartenance. La lecture conserve les mêmes identités d'instance que le graphe
et les mêmes versions de transaction que la table source et les relations.

Ce choix réutilise tables internes, transactions, annulation, WAL, checkpoint et
catalogue. Il ajoute une table et des écritures d'index ; ce n'est pas un petit
ajout de colonne. Sa faisabilité aux appels natifs d'insertion reste à prouver
avec une recette et des témoins avant livraison. Le diagnostic ne doit pas
contraindre les recherches existantes à devenir des parcours exhaustifs.

| Option | Préserve singleton et vieux snapshot ? | Coût / conséquence | Avis |
| --- | --- | --- | --- |
| Watermark, degré CSR ou parcours ANN | Non : preuves document11 | Peu de code, sens faux | Écartée |
| Endpoints d'arêtes + racine courante | Pas de complétude ni de version des racines | Scan des arêtes, synchronisation supplémentaire | Écartée pour le compte exact |
| Bitmap RAM/atomique + epochs | Pas après reopen ni sous anciennes versions sans historique | Nouveau protocole d'historique, WAL, GC et reprise | Écartée ici |
| Bitmap/runs persistants spécialement versionnés | Possible si tout est conçu | Compact, mais nouvelle primitive MVCC/persistance/GC à maintenir | Alternative ultérieure, pas le premier choix |
| Table interne de membres par instance | Oui sous les invariants ci-dessous | O(M) espace logique + versions/WAL ; réutilise le moteur | Recommandée |
| Colonne toujours NULL | Ne fournit pas le besoin | API seule, ticket toujours ouvert | Pas une livraison |

## Représentation et preuve de complétude

Une table interne par instance, une propriété `node_offset UINT64 PRIMARY KEY`.
Une ligne signifie : ce sommet source a été incorporé au graphe inférieur de
CETTE instance. La clé est l'offset du nœud source, pas l'offset physique de la
ligne de membership. Les offsets source ne sont pas supposés denses. Schéma
privé, pas une nouvelle table visible ni une self-relation modifiant HNSW.
Deux index de la même propriété ont deux tables distinctes.

L'identité est le triplet d'OIDs upperRel/lowerRel/membership, conservé dans le
holder et lié au catalogue du snapshot. Le nom privé sert à CREATE/DROP, pas
à choisir la bonne instance après reCREATE. La table ne porte pas de compteur
exact matérialisé ni de watermark de membership. Son schéma a une version
explicite dans le descripteur HNSW ; pas de déduction depuis un nom ressemblant.

Complétude : table créée dans T_A avec NOTED, entièrement remplie dans T_B avec
le graphe, puis publiée par le passage READY de T_B. Une existence de table
seule ne certifie rien. Après publication, chaque mutateur du graphe entretient
l'ensemble dans LA MÊME transaction. READY sans capacité désigne un ancien
index ; READY avec capacité valide garantit le protocole complet. Une table
manquante ou malformée malgré un descripteur annonçant la capacité est une
corruption/erreur nommée, pas un NULL silencieux. Pas de table inférée a posteriori.

## Invariants de transaction et de cycle de vie

| Opération | Invariant à établir et témoin nécessaire |
| --- | --- |
| CREATE T_A | Crée identité/table vide et NOTED ensemble ; les lecteurs rendent NULL ; abandon conserve INCOMPLETE, pas un certificat vide |
| Bâti T_B | Enregistre les offsets réellement incorporés, pas tous les offsets dimensionnés ; sélection du bâti utilisée avant destruction, singleton compris ; remplissage et READY validés atomiquement |
| Rattrapage avant commit CREATE | Incorpore les lignes supplémentaires et les membres dans la transaction de rattrapage existante ; aucune lecture LatestCommittedView dans SHOW_INDEXES |
| INSERT/COPY différé | Ligne visible mais pas encore incorporée => absente de M ; elle contribue au positif. Dès incorporation réussie, ajout M dans la transaction qui publie les arêtes/racine ; ne pas marquer à la simple réception du lot |
| UPDATE valide→valide | Retrait/réinsertion du graphe et membership atomiques ; pas de fenêtre publiée d'absence. L'ordre actuel index avant colonne ne change pas l'observation après instruction ; échec doit annuler toutes les écritures |
| UPDATE valide→NULL / DELETE | Retrait de M avec retrait logique du graphe ; anciens snapshots conservent les versions antérieures. Les arêtes mortes résiduelles ne créent pas de membre |
| UPDATE NULL→valide | Ajout de M seulement à l'incorporation réussie ; un NULL absent de M n'est jamais un candidat éligible |
| Rollback | Annule membres et relations via MVCC existant, y compris versions antérieures de UPDATE/DELETE ; aucune restauration manuelle par watermark ; rollback des racines ne définit pas M |
| Checkpoint | Persiste table/catalogue/descripteur via primitives existantes ; ne supprime aucune version nécessaire aux lecteurs ouverts ; ne transforme pas un vieux holder sans capacité en certifié |
| Reopen/recovery | WAL et catalogue rendent ensemble identité/état/table ; un crash avant READY ne certifie rien ; aucun rebuild/catch-up déclenché par la lecture |
| Ancien lecteur | Lit E et M au même snapshot + ses propres écritures, sans snapshot refresh ; commits d'autrui ne changent pas le compte ancien |
| DROP/reCREATE | Tombstones et rétention MVCC de l'ancienne table comme des anciennes relations/holder ; nouvelles OIDs, jamais capture par nom seul ; rejet si cette rétention n'est pas assurée |

Racines et membership ont des rôles différents : cette table ne répare pas à
elle seule les racines historiques des recherches ANN. Pour le diagnostic, une
racine courante n'est plus consultée. Les identités et versions des membres
certifient l'incorporation, les relations suivent le même commit ; leur lecture
exhaustive ne sert pas de substitut au journal de membership.

## Format natif et compatibilité à arbitrer explicitement

Source relue : HNSWStorageInfo sérialise cinq scalaires (hnsw_index.cpp:474).
Proposition : conserver ce préfixe, puis un trailer borné magic/version de
capacité/OID membership. Le lecteur nouveau reconnaît exactement l'ancienne
longueur comme « sans capacité », valide taille/version/OID du trailer nouveau,
et refuse les longueurs tronquées/versions inconnues. BufferReader expose
size/readSize mais read ne contrôle pas lui-même la borne : vérifier les tailles
avant désérialisation du payload HNSW, sans élargir ce lot au Reader commun.
Aucun trailer facultatif toléré arbitrairement, aucun nouveau champ ABI public C.

Un ancien lecteur HNSW pourrait ignorer le trailer et écrire les arêtes sans
membership : **inacceptable**. Recommander une nouvelle STORAGE_FORMAT_VERSION
(source71 courante43 ; numéro suivant à attribuer sur la vraie base fraîche),
avec barrières existantes DatabaseHeader et WAL FormatRecord. Les anciens
moteurs refusent cette base/le WAL avant modification ; ce refus doit être
prouvé après création mais aussi avant le premier checkpoint. Pas de confiance
dans un simple marqueur d'extension que l'ancien moteur n'interprète pas.

Le nouveau moteur garde la plage des anciens formats lisibles. Une ancienne
base/index s'ouvre sans migration implicite du graphe ; le descripteur ancien
reste sans capacité et rend NULL, même après passage du format global au nouveau
checkpoint. Les écritures sur un ancien index ne fabriquent pas un certificat
complet. Obtention de la capacité : DROP/CREATE explicite autorisé, ou migration
explicite ultérieure avec sa propre preuve ; scanner les vecteurs ne suffit pas
à reconstruire l'historique réel d'incorporation. Pas d'installation SDK dans ce
lot documentaire, pas d'ouverture des trois bases en migration d'Arbre.

## Lecture commune, connu / inconnu et coût

Index expose une capacité optionnelle d'observation possédée : identité de
holder/instance + snapshot + enumération complète M. NodeTable capture le holder
par MVCC ; HNSW fournit une lecture de sa table interne. SHOW_INDEXES ne fait
aucun static_cast HNSW dans le cœur. Pas d'exception générale avalée : capacité
non disponible/nonREADY=>NULL ; incohérence ou I/O défectueuse=>erreur nommée.
La valeur est |E(S) privé de M(I,S)| ; seule une observation complète dans le
budget autorise0 ou >0. Les quatre témoins existants ne sont pas assouplis.
Les huit anciennes colonnes restent stables, la neuvième UINT64 nullable.

Lecture via NodeTable scan et OnDiskEmbeddings au snapshot appelant, sans
lockIndexForWrite, garde transactionnelle nouvelle, releaseAll, refreshSnapshot,
DUMMY ou LatestCommittedView. Une propriété MVCC référencée ne doit pas être
libérée sous un vieux lecteur ; réutiliser la rétention stockage existante,
et la tester. Un nouveau verrou global autour du scan serait un recul inutile.

Implémentation de référence proposée : charger M dans un set borné, puis scanner
les offsets source et leurs vecteurs visibles. O(M+N) attendu en calcul avec
hashing ; N inclut les trous physiques examinés, et la lecture de vecteurs ajoute
O(N*d) en volume si dimension d. Mémoire O(M) ; pas de garantie de temps disque.
Le budget partagé proposé document10 (65 536 offsets, 1 048 576 éléments,
16Mio supplémentaires) est une politique provisoire à mesurer ; compter les
allocations réelles, pas seulement sizeof(offset). Dépassement => NULL avant
résultat partiel, annulation/échéance entre lots. Les versions retenues et index
primaire de la table ajoutent espace/WAL/CPU et coût checkpoint. Aucun chiffre
de performance réel acquis ; construire plus tard une mesure isolée avec budget.

## Portée précise proposée pour revue, pas réservation

- src/include/storage/index/index.h : capacité commune possédée et défaut absent.
- src/include/storage/table/node_table.h et src/storage/table/node_table.cpp :
  capture identité/snapshot/lifetime ; vérifier accès natif au stockage interne.
- src/function/table/show_indexes.cpp : observation à l'exécution et neuvième
  nullable ; vérifier préparation réutilisée entre snapshots.
- extension/vector/src/include/index/hnsw_index.h et src/index/hnsw_index.cpp :
  trailer/capacité, fournisseur membership, tous mutateurs et callbacks rollback.
- extension/vector/src/function/create_hnsw_index.cpp et drop_hnsw_index.cpp :
  création et retrait table interne, identité T_A/T_B/READY et ancien format.
- extension/vector/src/include/index/hnsw_index_utils.h et
  src/index/hnsw_index_utils.cpp : nom privé/découverte par OID, pas par seul nom.
- extension/vector/src/include/index/hnsw_rel_batch_insert.h et
  src/index/hnsw_rel_batch_insert.cpp : transfert sélection/incorporation au bâti,
  avant reset ; choisir le point unique d'écriture des membres, pas un double ajout.
- src/include/storage/storage_version_info.h : barrière de format, numéro à fixer.
- Nouveau test/transaction/vector_index_membership_diagnostics_test.cpp et
  test/transaction/CMakeLists.txt ; témoins format dans un fichier séparé
  test/transaction/vector_index_membership_recovery_test.cpp.

Lecture de dépendances à confirmer avant implémentation : catalogue interne,
NodeTable insert/delete et rétention des tables supprimées. Si un changement de
ces primitives devient nécessaire, arrêter ce périmètre et proposer les fichiers
exacts au cœur. Pas de modification directe du WAL/DatabaseHeader prévue si leurs
barrières et persistances existantes suffisent. Aucun chemin Rust produit,
cache_column, verrou global, chaîne/dictionnaire ou hook C1 réservé.

## Preuves à préparer avant correctif / validation future séparée

Prépublier les témoins : singleton sans arête connu0 ; COPY déterministe avec
un membre manquant connu>0 puis incorporation0 ; NULL/delete/trous ; ancienne
transaction vs nouveau commit et rollback ; deux index même propriété ;
DROP/reCREATE ; ancien payload sans trailer=>NULL ; nouveau payload valide,
tronqué/inconnu=>refus ; ancienne version du moteur refuse le nouveau format
avant checkpoint ; reopening/recovery avant/aprèsREADY ; budgets et annulation.

Un crochet test-only déterministe au vrai chemin d'incorporation COPY et une
fixture ancien format doivent être proposés au cœur, jamais fabriqués par une
colonne const0. Voir chaque témoin pertinent rouge avant fix, puis vert ; aucune
liste de résultats inventée depuis les quatre cas0 historiques. Bâtis/SDK et
expériences de crash exigent une recette isolée, RO et GO distincts ultérieurs.
Cette recommandation n'en demande ni ne les autorise.

## Revue indépendante Mémoire : trois préconditions avant décision

Revue totale de 588284a313dba223f2a3906dcea3415e89182cf9 reçue en trois
groupes. Orientation table interne MVCC favorable seulement ; **faisabilité
non démontrée et aucun chemin réservé**. Les points ci-dessous précisent les
obligations de la recommandation, sans annoncer que le moteur les remplit.

### 1. Mutateurs, idempotence et vraie frontière de commit

Cartographier explicitement commitInsert (hnsw_index.cpp:775), finalize:813,
catchUpCommittedAfter:847, updateInternal:1276, delete_/finalizeDelete et les
réparations qui peuvent réincorporer un même offset. La clé primaire seule ne
rend pas le protocole idempotent : un ajout répété ne doit ni créer un doublon
ni faire échouer une réinsertion légitime. Définir les opérations sémantiques
« assurer présent » / « assurer absent » au snapshot d'écriture, et leur ordre
avec les arêtes, sans transformer une erreur réelle en succès silencieux.
Le mécanisme concret doit être choisi après lecture des API de table, pas par
un INSERT/ignore arbitraire. Les réparations topologiques sans changement de
membre doivent conserver M.

La possibilité d'écrire M depuis les callbacks existants doit être démontrée
par l'ordre réel de collecte des tables locales, de production WAL, de commit
et de rollback. Passer le même Transaction* ne prouve pas qu'une table touchée
à ce stade sera intégrée au commit ou annulée correctement. Si le callback se
situe après cette frontière, STOP et nouvelle proposition de périmètre cœur.

Témoins à prépublier : offset source sparse incorporé au bâti puis au catch-up,
UPDATE répété de ce même offset sans doublon ni erreur PK ; rollback injecté
après écriture M mais avant fin des relations, vérifiant restauration de M,
des arêtes et de E pour l'appelant et les autres snapshots. Le transfert du
bâti utilise la sélection d'offsets SOURCE avant reset, jamais les offsets
physiques des lignes de M.

### 2. Ancien snapshot, DROP/checkpoint/GC et acquisition possédée

Point relevé par Mémoire sur la source de revue :
StorageManager::reclaimDroppedTables:172–214 appelle reclaimStorage puis retire
la table du conteneur sur la vue catalogue DUMMY. Un shared_ptr<Index> ne prouve
pas la rétention de Table, colonnes et pages de M. La recommandation dépend donc
d'une garantie réelle de rétention MVCC, pas d'une simple prolongation du holder.

Exiger deux témoins distincts :

- Ancien snapshot ouvert **sans capture préalable de M**, puis DROP/reCREATE,
  checkpoint et GC ; sa première lecture doit encore acquérir l'ancienne table
  et ses pages à partir des OIDs de son instance.
- Observation déjà acquise, scan suspendu entre deux lots, puis même séquence
  DROP/reCREATE/checkpoint/GC ; la reprise garde l'ancien E et l'ancien M.

Le nouveau lecteur distingue les nouvelles OIDs ; l'ancien compte reste inchangé,
aucune consultation par nom courant. L'observation possédée doit couvrir tous
les objets/pages nécessaires ou s'appuyer sur une garantie de rétention déjà
prouvée avant acquisition. Si cette garantie manque, STOP et proposition
explicite des changements cœur ; pas de verrou écrivain pour masquer la lacune,
ni NULL silencieux d'une capacité annoncée complète. Aucun changement à
reclaimDroppedTables autorisé par cette préparation.

### 3. Transition de format/WAL et totalité du budget

Observation Mémoire : WAL::initWriter ne crée FormatRecord que pour un fichier
vide. Cela ne démontre ni défaut avéré ni barrière suffisante dans le scénario
upgrade d'ancienne base avec WAL déjà non vide. Avant d'ajouter M, démontrer
recovery/vidage/transition durable vers la nouvelle version **avant la première
écriture de la capacité**. Une constante de version changée ne suffit pas.

Témoins : ancienne base/WAL non vide, ouverture par nouveau moteur, création de
capacité et crash avant checkpoint ; ancien moteur lecteur ET écrivain refusent
la nouvelle représentation. Tester aussi crashes après T_A, pendant T_B et autour
de READY, reopen/recovery, anciens index toujours NULL, payload tronqué/version
inconnue refusés. Rien de cela n'est joué dans cette préparation. Le numéro de
format, son protocole d'upgrade et tout chemin WAL supplémentaire restent à
arbitrer avant réservation.

Le budget commun couvre construction de M, allocations réelles du set, examen
des versions et index primaire, scan E et vecteurs, et tous les index du même
SHOW_INDEXES. Prévoir plusieurs index et instruction préparée réutilisée entre
snapshots ; ne pas cacher ces coûts dans une étape dite préalable. Dépassement
=> NULL sans compte partiel ; corruption/I/O restent erreurs, annulation reste
annulation (ne pas la convertir en inconnu). E vient de la visibilité et du
validateur réel, jamais du degré ni du watermark. Les plafonds proposés ne sont
pas une mesure de latence ou une preuve d'annulation d'I/O.

Ces trois groupes épuisent la revue reçue ; ils ne lèvent pas leurs préconditions.
Attente de l'arbitrage C7 sur la prochaine lecture autorisée et le périmètre.
Aucun code, format, réservation, prototype, DB, build ou natif engagé.
