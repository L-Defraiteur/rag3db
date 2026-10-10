# Les verrous — ce que V2, A3′, A4′ et l'index au commit changent réellement

10 octobre 2026, session cœur C++. Page courte avant de coder, demandée par l'orchestration
une fois la condition 2 de la stèle fermée (`ff9bad960`). La conception est celle de la note du
3 octobre (`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`), ses trois écarts
tranchés le 3 au soir (annonce en tête : oui ; hors annonce : option A, l'erreur nommée et
transitoire après attente ; une relation : verrou partagé sur ses extrémités). Rien ici ne la
rouvre ; cette page dit ce que chaque marche **branche** dans le moteur, dans quel ordre, et
ce qu'elle prouve. **[lu]** : lu dans le code à `ff9bad960` ; **[déduit]** : raisonné.

## 1. Ce qui existe déjà : V1, le gestionnaire seul (`022c78402`)

`src/transaction/lock_manager.{h,cpp}` **[lu]** : des ressources de deux genres (`ROW` : une
table et une clé ; `INDEX` : une table), deux modes (partagé, exclusif), `acquire` d'une prise
ou d'un groupe de prises avec délai et interruption, l'interblocage détecté **à la prise**
(une seule des deux transactions reçoit `deadlock detected`), `releaseAll` à la fin d'une
transaction (déjà appelé par `TransactionManager::clearTransactionNoLock`, `:225`), et trois
erreurs nommées : `deadlock detected`, `lock timeout`, `could not serialize access due to
concurrent update`. Réglage `CALL lock_timeout` (30 s par défaut).

**Aucune écriture ne prend encore de verrou.** Tout ce qui suit est du câblage, dans
`src/storage/table/` et `src/transaction/` ; le gestionnaire ne change pas.

## 2. Ce que chaque marche change

### A3′ — l'insertion : un verrou de clé, puis l'unicité contre le dernier état validé

- **Aujourd'hui** **[lu]** : `NodeTable::insert` vérifie la clé contre l'index de clé primaire
  (`validatePkNotExists`) à l'instant de l'insertion, dans l'instantané de la transaction. Deux
  transactions qui insèrent la même clé passent toutes les deux ; le second commit l'emporte
  ou laisse un doublon dans l'index — les témoins C1 et le journal à doublon du banc (lignes
  « Marche A3′ » de `known_red.txt`).
- **Ce qui change** : avant la vérification, `acquire(ROW{table, clé}, EXCLUSIVE)` ; si une autre
  transaction tient la clé, on **attend** sa fin ; puis l'unicité se vérifie contre le dernier
  état **validé**, pas contre l'instantané — si l'autre a validé la même clé, `Found duplicated
  primary key value` (l'erreur d'aujourd'hui), sinon l'insertion passe. Le verrou tient jusqu'à
  la fin de la transaction.
- **Où** : `NodeTable::insert` (le point unique : CREATE, MERGE, le rejeu n'y passe pas — une
  transaction de reprise ne prend pas de verrou), et `NodeBatchInsert` pour un `COPY` dans une
  transaction explicite : **une prise `INDEX{table}` exclusive** pour tout le `COPY` plutôt
  qu'une par ligne **[déduit : à confirmer par la mesure]**.
- **Ce que ça coûte** : une prise par ligne insérée (une entrée de table de hachage, un
  mutex) ; à mesurer sur `UNWIND … CREATE` d'un million de lignes avant et après.
- **Prouve** : C1 (deux insertions de la même clé : la seconde attend, puis clé en double),
  « instantané antérieur » (la même clé sans chevauchement), le journal à doublon qui ne se
  produit plus, `LockBench.SameKeyWaitsThenGetsDuplicateKey` et `…ThenInsertsAfterRollback`.

### A4′ — la ligne : exclusif pour la mise à jour et la suppression ; partagé sur les extrémités d'une relation

- **Aujourd'hui** **[lu]** : `NodeTable::update` et `delete_` écrivent dans l'instantané ; au
  commit, « Write-write conflict » immédiat ou, pire, les deux valident (C6 : une suppression et
  une mise à jour de la même ligne valident toutes les deux ; deux mises à jour de deux colonnes
  aussi). Une relation vers un nœud que l'autre supprime : C2.
- **Ce qui change** : `update`, `delete_` : `acquire(ROW{table, clé}, EXCLUSIVE)` avant
  d'écrire ; après l'attente, si l'autre a validé une écriture sur cette ligne, l'erreur nommée
  `could not serialize access due to concurrent update` (option A : on reste en instantané par
  transaction — l'instruction ne se rejoue pas seule). La création d'une relation :
  `acquire(ROW{table, clé}, SHARED)` sur chacune des deux extrémités ; la suppression d'un nœud
  prend l'exclusif, donc attend les relations en cours et les bloque ensuite. La suppression
  en détachant (`DETACH DELETE`) : exclusif sur le nœud, et ses relations suivent.
- **Où** : `NodeTable::update`, `NodeTable::delete_`, `RelTable::insert` (les deux extrémités),
  `RelTable::delete_`/`detachDelete` (exclusif sur la relation par son identité ? — **à
  trancher au code** : la relation n'a pas de clé ; le verrou porte sur ses extrémités,
  partagé pour la créer, exclusif pour la supprimer ou la mettre à jour).
- **Ce que ça change à T0 et à l'erreur** : l'erreur après attente est transitoire : la
  transaction est en échec (T0), `ROLLBACK`, et l'appelant rejoue — c'est ce que rag3weaver
  fait déjà d'un refus.
- **Prouve** : C2, C5 (deux suppressions de la même ligne), C6, « deux colonnes »,
  `LockBench.SameRowUpdateWaitsThen…`, `DeleteThenUpdateWaitsThen…`,
  `DeleteThenNewRelationWaitsThen…`, et le nœud-carrefour (huit écrivains, verrou partagé :
  aucun n'attend).

### V2 — l'annonce en tête de transaction

- **Ce qui change** : `CALL acquire_locks(...)` prend, **en une seule prise groupée et dans
  un ordre canonique**, les ressources que la transaction annonce (des clés d'une table, ou une
  table entière) ; une transaction qui a tout annoncé ne rencontre plus ni attente surprise ni
  interblocage ni erreur de sérialisation sur ces ressources. C'est l'écart 1, tranché oui.
- **Où** : une fonction `CALL` dans `src/function/table/` (hors de mes dossiers : à écrire
  avec la seconde session ou par elle, sur mon interface), et `Transaction` qui garde ce qu'elle
  a annoncé pour que A3′/A4′ sautent la prise déjà tenue.
- **Prouve** : {A, B} et {B, A} annoncés : aucune erreur, les deux valident l'une après
  l'autre ; l'interblocage sans annonce (une erreur exactement, l'autre valide).

### L'index vectoriel au commit — une marche, pas un affinage

- **Aujourd'hui** **[lu]** : l'index HNSW sur disque reçoit ses lignes à `NodeTable::commit`
  (`commitInsert` → `insertInternal`, une à une, qui écrit les arêtes dans les tables internes
  de l'index) et ses mises à jour et suppressions **pendant** la transaction (`update`,
  `delete`, puis `finalizeDelete`). Ses points d'entrée et ses listes sont modifiés sans verrou.
- **Ce qui change** : avec A3′/A4′, deux écrivains sur une table indexée s'attendraient du
  début à la fin **[déduit]** — l'index est une ressource `INDEX{table}` que chaque écriture
  prendrait en exclusif. La marche : faire tenir toutes les écritures de l'index **au commit**
  (les insertions y sont déjà ; les mises à jour et suppressions à y ramener, avec l'état par
  instruction que le banc a conçu pour la mise à jour massive — page 04 du banc), sous un seul
  verrou `INDEX{table}` exclusif pris au commit, et pas pendant la transaction. Deux écrivains
  sur la même table indexée ne s'attendent alors qu'à la validation.
- **Prouve** : deux écrivains qui insèrent dans une table indexée en même temps : aucun
  n'attend avant son commit ; après les deux, chaque ligne se retrouve par son vecteur.

## 3. L'ordre que je propose, et pourquoi

1. **A3′** d'abord : le plus petit câblage (un point, `NodeTable::insert`), le défaut le plus
   grave qu'il ferme (le doublon de clé, qui peut empêcher de rouvrir — H4), et il met le
   gestionnaire sous charge réelle avec la mesure de son coût par ligne.
2. **A4′** ensuite : même mécanique, plus de points ; il ferme C2, C5, C6 et retire le
   « Write-write conflict » immédiat.
3. **V2** : il ne sert qu'avec A3′ et A4′ en place ; c'est une fonction `CALL` plus une liste
   dans `Transaction`.
4. **L'index au commit** en dernier : il dépend de l'état par instruction du banc (sa page
   04), et sans A3′/A4′ il n'y a pas encore d'attente à raccourcir.

Chaque marche : témoins rouges d'abord (ils existent au banc, dans `known_red.txt`), le
câblage, la mesure du coût, la liste complète une fois, relecture du banc, push. Le mode
multi-écrivains reste **éteint hors du banc** jusqu'à la condition 3 de la stèle ; le câblage
est sous ce mode et sans effet mesurable avec un seul écrivain (un verrou pris sans
concurrent : à mesurer, c'est la seule chose que le mode éteint ne doit pas payer).

## 4. Ce que je demande avant de coder

- L'ordre ci-dessus ; ou A4′ avant A3′ si le produit souffre plus de C6 que du doublon de clé.
- Pour A4′ : la relation est-elle une ressource (par son identité) ou seulement ses extrémités ?
  Je propose : seulement ses extrémités (partagé pour créer, exclusif sur le nœud pour
  supprimer ou mettre à jour ses relations), comme Neo4j verrouille les nœuds d'une relation.
- V2 : qui écrit la fonction `CALL` (hors de mes dossiers).

## 5. A3′ : faite (10 octobre 2026, soir) — ce que le code a appris de plus que la page

Le câblage est celui du §2, avec trois choses que la page ne prévoyait pas, toutes trouvées
par le banc le jour même ; **[mesuré]** sauf mention.

- **L'index en partagé pour l'insertion, en exclusif pour le COPY.** Une insertion prend, en
  une prise groupée, `INDEX{table}` partagé et `ROW{table, clé}` exclusif ; un COPY prend
  `INDEX{table}` exclusif. Un COPY écrit ses clés dans l'index et ses lignes dans les blocs de
  la table avant de valider : ni deux COPY ni un COPY et une insertion de la même table ne se
  croisent plus. C'est ce qui ferme la première limite du 4 octobre (deux COPY non validés dans
  le même bloc) — par le verrou, pas par un correctif de l'annulation ; le témoin
  `RollbackOfACopy.RemovesOnlyItsOwnKeys` est réécrit à deux fils (le second COPY attend).
- **Une seule visibilité d'unicité, à l'insertion et à la validation.** L'unicité se contrôle
  contre le dernier état validé (`Transaction::LATEST_COMMITTED_TS` : tout horodatage de
  validation est inférieur au premier identifiant de transaction ; `NodeTable::
  isVisibleToLatestCommit`). Le premier câblage ne l'avait fait qu'à l'insertion ; la validation
  (`commitInsert` de l'index de clé) regardait encore l'instantané. Sur une clé supprimée et
  validée par un autre après l'instantané, l'insertion passait, la validation levait
  « duplicated primary key » **après** l'ajout des lignes aux groupes : une transaction à moitié
  validée, puis un SIGSEGV à la validation suivante (C7, trois passes sur trois). Une seule
  fonction (`NodeTable::getUniquenessVisibleFunc`) sert aux deux. Le défaut général — une erreur
  pendant la validation après l'étape 1 — est au ticket du 10 octobre.
- **Une attente de verrou ne doit pas tenir un fil ouvrier de l'ordonnanceur.** Le moteur
  exécute toute instruction par des tâches sur un nombre fixe de fils ouvriers ; une attente de
  verrou dans un opérateur occupe l'un d'eux. Avec N attentes, les N fils sont pris, et la
  transaction qui doit valider pour les libérer ne trouve plus de fil : tout gèle jusqu'au délai
  des verrous (garde du banc expirée dans C7 et dans le COPY à deux fils ; piles prises sous
  gdb). Pire pour le COPY : son verrou était pris dans `initGlobalState`, que la tâche exécute
  sous son mutex, et l'ordonnanceur entier gelait. Deux réponses : le verrou du COPY est pris
  **avant l'ordonnancement, sur le fil du client** (`PhysicalOperator::
  acquireLocksBeforeExecution`, appelé par `QueryProcessor::execute` sur chaque tâche du plan),
  comme PostgreSQL verrouille dans le processus de la connexion ; et toute attente sur un fil
  ouvrier lance un **fil de remplacement** qui sert la file pendant l'attente
  (`TaskScheduler::BlockingWait`, le « managed blocker » des ordonnanceurs à nombre de fils
  fixe), et s'arrête à la fin, après la tâche qu'il a prise. La prise est d'abord tentée sans
  attendre : une prise libre ne lance rien. Le même mécanisme servira à A4′ et à l'attente d'un
  point de reprise forcé, qui tient elle aussi un fil ouvrier **[lu, non traité]**.

Ce qu'A3′ ne fait pas : la reprise d'un journal à doublon (il ne peut plus naître ; celui
d'avant est gardé au dépôt, `test/transaction/journal_with_duplicate_key`, et son témoin reste
rouge pour la même raison qu'avant) ; le COPY de relations ; les mises à jour, suppressions et
relations (A4′).
