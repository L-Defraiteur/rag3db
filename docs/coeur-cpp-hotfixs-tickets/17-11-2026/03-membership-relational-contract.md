# Membership REL séparé : contrat concret conditionnel, conception seule

Arbitrage C7/02b916fda7 ; ticket08nov rattaché417978e6b0 AVANT cette note.
Base source99fef92c figée, périmètre relu identique71fbe (note02/e4abc).
Orientation REL interne séparée du graphe ANN, plutôt qu'une phase NODE dérivée.
Aucune réservation, implémentation, fixture, prototype, exécution, format ou SDK.

## Représentation et API attestées

Troisième relation interne M de la table source vers elle-même, multiplicité
ONE_ONE et stockage FWD proposé. Le seul marqueur autorisé est (o→o), une
relation logique pour chaque membre incorporé, singleton compris. M n'entre
JAMAIS dans les NativeGraphEntry upper/lower, ni dans le parcours/pruning ANN.
Ce sont des marqueurs d'ensemble dans une table séparée, pas des self-arêtes
ajoutées au graphe de recherche. L'OID M appartient au descripteur versionné de
l'instance (upperOID,lowerOID,memberOID), pas une découverte par nom courant.

Briques réellement lues :

- RelTable::insert (rel_table.cpp:239) utilise RelTableInsertState, contrôle les
  multiplicités, ajoute à getOrCreateLocalTable et diffère le WAL au commit,
  quand les offsets source sont finaux. checkRelMultiplicityConstraint:190
  applique RelMultiplicity::ONE aux directions stockées. La syntaxe ONE_ONE
  existe dans les tests de relations ; aucun test ajouté ou joué ici.
- RelTableScanState::scanNext (rel_table.cpp:108) passe des données validées aux
  relations locales ; setToTable/initScanState configurent table/direction et
  colonnes. Lire voisin ET relID, pas seulement count/hasRels.
- RelTable::delete_:292 distingue relID local et persistant, passe les suppressions
  à localTable ou relData et produit le WAL pour les persistantes. Il renvoie
  un booléen à juger, pas une permission d'ignorer une disparition incohérente.
- HNSW utilise déjà RelTableInsertState et ces insertions directes pour ses deux
  graphes ; ne pas utiliser createRels HNSW pour M : cette fonction saute les
  self-arêtes et déclenche shrink/pruning, comportements interdits pour M.
- LocalStorage::commit:64 relève les REL après les commits NODE et ne remappe
  que les relations préexistantes au début du commit. UncommittedIndexInserter
  (node_table.cpp:196) fournit startNodeOffset+i à commitInsert, donc offsets
  SOURCE finaux ; ne pas remplacer par un offset physique de M.

Choix à prouver : ONE_ONE avec FWD limite à AU PLUS un marqueur sortant par
source/direction stockée ; zéro reste possible. Ni cette contrainte ni le nom
de multiplicité ne garantit la présence, la complétude ou o→o.
Le code doit vérifier les endpoints, l'OID de table et exactement0 ou1 relation
visible. Doubles marqueurs, destination différente ou identité invalide sont
une erreur nommée. Aucun scan brut des degrés CSR comme preuve de membre.

## Contrat de mutation : assurer présent / absent

Les noms ci-dessous sont sémantiques, pas des fonctions déjà présentes.
Chaque opération reçoit instance capturée, transaction d'écriture, offset source
final ou politique explicite pour offset local à remapper, et état REL propre
(vecteurs/selection distincts des états ANN et detachDelete).

`assurer_present(o)` : scanner M en FWD depuis (o,sourceTableOID), avec voisins
et relID. Si1marqueur exactement o→o : succès sans écriture. Si0 : créer un
RelTableInsertState et appeler RelTable::insert UNE fois avec o→o. Si>1 ou autre
destination : erreur. Toute erreur de multiplicité, I/O, conflit ou annulation
reste une erreur ; pas de catch/ignore, ni transformation du doublon en succès
sans observation prouvée. Un scan local suivant doit voir la relation insérée.

`assurer_absent(o)` : même scan,0=>succès sans écriture ;1valide=>RelTableDeleteState
avec src/dst/relID exact, puis delete_. Un false après le scan non vide est une
incohérence à expliquer/refuser, pas un succès silencieux. Pas de detachDelete
sur un graphe ANN pour réaliser cette opération ; pas de suppression de toutes
les relations source en dehors de M. Le DELETE du nœud source doit intégrer la
suppression de M sans dépendre de sa visibilité après disparition de E.

Ces opérations ne sont idempotentes que si le scan voit l'état pertinent et
aucun autre mutateur ne peut s'intercaler. Point NON RÉSOLU par une signature :
une transaction ayant déjà écrit peut garder un vieux snapshot après attente
sur le verrou d'index. Le scan à ce snapshot peut manquer un M validé depuis,
puis la multiplicité constate ce M dans sa vue LatestCommittedView et refuse.
L'exception ne peut pas servir d'upsert.

Précondition de faisabilité à prouver pour CHAQUE appel : sérialisation réelle
avec tous les mutateurs de cette instance et vue de maintenance cohérente du
dernier membership stable + propres écritures. Les API existantes apportent
lockIndexForWrite pour UPDATE/DELETE, mutex public autour du commit, et
Transaction::LatestCommittedView utilisé par les contrôles de relations ; elles
ne démontrent pas par elles-mêmes le protocole global CREATE/finalize/catch-up.
Une vue latest temporaire, strictement bornée à M sous exclusion d'écriture,
est une option de MAINTENANCE à relire, avec restauration exception-safe, pas
une modification du snapshot du diagnostic. SHOW_INDEXES continue à lire E/M
au snapshot appelant, sans LatestCommittedView/refresh/lockIndexForWrite.
Si sérialisation ou visibilité des propres écritures/remap manque : **STOP**,
périmètre cœur à reproposer. ONE_ONE reste garde d'intégrité, pas preuve suffisante.

## Carte d'incorporation et annulation

- Bâti T_B : sélection des offsets source réellement insérés, avant reset ;
  les marqueurs incluent le singleton, excluent NULL/supprimés/non incorporés.
  Créer M dans T_A avec NOTED ; publier le certificat avec READY de T_B seulement.
  Choisir le point unique de remplissage, pas un doublon batch+callback.
- commitInsert/finalize/catchUp : assurer présent après incorporation réussie,
  y compris le chemin entrypoint sans arête ; COPY validé mais non incorporé
  reste sans marqueur et donne le positif. Le positionnement doit être prouvé
  avant collecte finale REL, pas ajouté pendant/après son propre commit.
- UPDATE valide→valide : peut retirer puis remettre le marqueur dans la même
  transaction, ou conserver la présence si réincorporation atomique prouvée ;
  choisir une seule politique après analyse des états d'échec. Répétitions ne
  doublent pas M. NULL→valide ajoute après succès ; valide→NULL et DELETE retirent.
- Réparations topologiques : aucun changement de M sans changement d'incorporation.
- Rollback/échec : undo/localStorage/WAL existants doivent restaurer M et arêtes
  ensemble ; aucune annulation par watermark ou remise à zéro d'un compteur.
  M ajouté au commit utilise bien les tables REL collectées ensuite, mais la
  prise en charge des suppressions/rollback et recovery reste à qualifier.
- Reopen : membre M et état de holder reviennent ensemble ou capacité refusée
  comme incohérente. Si une extension absente a détaché l'index au recovery,
  diagnostic indisponibleNULL, jamais READY certifié avec M partiel.

## Prérequis cœur conservés : ancienne acquisition et pages

La troisième table ne corrige pas les lacunes source de note02. Avant première
capture d'un vieux lecteur, préserver par horizon MVCC le holder, le catalogue,
la REL M et les deux relations ANN nécessaires à l'identité ; résoudre l'ancienne
OID dans les versions, pas filtrer seulement l'OID de tête. Une fois acquise,
l'observation possède/retient tables, colonnes, groupes et pages jusqu'à sa fin.
Le plancher actif existe (snapshotFloorTS/oldestReaderTS), mais reclaimDroppedIndexes
et reclaimDroppedTables doivent être arbitrés pour ne pas libérer trop tôt.
Le GC ne récupère qu'après départ du plus vieux lecteur concerné ET des scans
possédés. Ne pas attendre qu'un lecteur ait déjà capturé la table pour la retenir.

Témoins distincts : ancien snapshot SANS scan→DROP/reCREATE→checkpoint/GC→
première acquisition ancienne ; scan suspendu entre lots→même séquence→reprise.
Old E/M/OIDs conservés, nouvelle instance distincte, pages non réutilisées.
Si ce prérequis échoue : STOP, pas writerlock de secours ou NULL sous capacité
annoncée. Répertoire proposé : catalog_set.*, storage_manager.*, index.h/
node_table.cpp ; PageAllocator seulement si le report de reclaim ne suffit pas.
Aucun chemin ainsi réservé, aucune généralisation de shared_ptr seule prétendue.

## Transition de format recommandée

Adopter la séquence documentaire C7 : ancien WAL rejoué → checkpoint du format
nouveau durable → rotation/fermeture de l'ancien WAL → nouveau WAL versionné →
capacité M autorisée. Le premier checkpoint est un upgrade préalable, pas un
checkpoint de M déjà écrite. La base READ ONLY ne fait aucun upgrade ; les
index hérités restent sans capacité et rendent NULL, sans scan/rebuild implicite.
Un nouveau CREATE explicite obtient la capacité une fois l'upgrade terminé.

Le trailer HNSW contient magic/version/OID M et conserve exactement l'ancien
préfixe à cinq scalaires. Ancien payload reconnu=>sans capacité ; inconnu ou
tronqué=>erreur. Nouvelle version globale à attribuer sur base fraîche, refus
ancien moteur en lecture ET écriture via header/WAL. Pas de numéro fixé ici.
Les API recovery/checkpoint/FormatRecord existent ; la transition durable,
sa fermeture WAL et sa reprise après interruption ne sont PAS encore une API
atomique démontrée. Ne pas appeler clear/reset arbitrairement en laissant
l'état ancien durable perdu. Chemins conditionnels note02 : Database/
StorageManager/Checkpointer, WAL/WALReplayer et storage_version_info.h ; les
réduire après preuve du point d'admission et de reprise choisi. Aucun bump/code.

## Surface minimale et preuves prépubliables avant décision de code

Surface extension conditionnelle : hnsw_index.{h,cpp} (capacité/mutateurs/trailer),
create_hnsw_index.cpp/drop_hnsw_index.cpp (REL interne/NOTED/READY/durée de vie),
hnsw_index_utils.{h,cpp} (nom privé et IDs), hnsw_rel_batch_insert.{h,cpp} si
nécessaire pour le transfert de sélection avant reset. Plus capacité commune
index.h, capture NodeTable, show_indexes.cpp ; aucun chemin Rust consommateur.
Pas de nouvelle phase globale NODE. Pas de changement REL core supposé tant
que l'idempotence et la collecte sont prouvées avec l'API actuelle ; sinon STOP.
Rétention/résolution OID et transition de format sont des extensions cœur
séparément à arbitrer, jamais cachées dans « écrire une table M ».

Prépublier sans encore créer/jouer :

1. Singleton sans arête connu0 ; COPY déterministe réellement non incorporé>0
   puis incorporation0, anciens quatre témoins inchangés.
2. Réinsertions répétées bâti/catch-up/UPDATE même offset, multiplicité1 ; NULL,
   suppression, rollback après M avant finrelations ; erreur volontaire remontée.
3. Transaction ayant déjà écrit autre table puis attente index et commit d'autrui :
   idempotence M malgré vieux snapshot, restauration de la vue temporaire si retenue.
4. Offsets provisoires déplacés au commit : marqueurs source finals exacts,
   relations anciennes remappées et nouvelles non remappées deux fois.
5. Deux oldtx/DROP/checkpoint/GC ci-dessus et deux index même propriété.
6. Anciennebase+WALnonvide, interruptions à chaque borne de l'upgrade puis
   T_A/T_B/READY ; refus vieux reader/writer avant toute écriture ignorante de M.
7. Budget commun incluant CSR/versions/M/E/vecteurs, multiindex et prepared
   réutilisée ; dépassementNULL sans partiel, annulation/I/O/corruption distinctes.

Coûts : un marqueur REL par membre + CSR, relIDs, versions, WAL et suppressions ;
retention majorée par vieux lecteurs ; checkpoint d'upgrade. Lecture par source
via scan M peut rester O(N + nombre de relations/versions examinées) plus O(N*d)
en volume de vecteurs, sans set global si la multiplicité1 est validée et les
scans sont bornés ; ne pas annoncer que les pages CSR sont gratuites. Le choix
scan par offset vs enumération globale sera mesuré plus tard. Budgets provisoires
note01, allocations réelles et échéance partagés par appel, pas par index ;
NULL signifie capacité/budget indisponible, pas résultat approché.

Conclusion de conception : REL évite la lacune de collecte NODE attestée, mais
unicité/idempotence de TOUS les mutateurs, rétention et upgrade restent des
préconditions. RO Mémoire/source et arbitrage C7 avant tout GO code. Pas de
faisabilité acquise ni résultat nouveau, aucune activité C1.
