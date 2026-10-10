# V2 — l'annonce des verrous en tête de transaction : une page avant le code

10 octobre 2026, nuit, session cœur C++. L'écart 1 de la note des verrous
(`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`), tranché **oui** par Lucie le
3 octobre ; A3′ et A4′ sont sur master (`67ff3816b`). **[lu]** : vu dans le code à ce commit ;
**[déduit]** : raisonné ; rien n'est mesuré encore.

## 1. Ce que c'est, et pourquoi

PostgreSQL et Neo4j découvrent leurs verrous instruction par instruction : d'où l'interblocage
(deux transactions dans l'ordre inverse) et l'échec après attente (celui qui a attendu lit un
état périmé — c'est l'option A d'A4′ : `could not serialize access due to concurrent update`).
rag3weaver connaît ses clés avant d'écrire. L'annonce en tête fait ce que leurs applications ne
peuvent pas :

1. la transaction s'ouvre et **annonce** ses clés ;
2. le moteur les trie et les prend **en une prise groupée**, dans un ordre unique, en attendant
   s'il le faut (le gestionnaire V1 le fait déjà : `LockManager::acquire(span<LockRequest>)`,
   « deux prises groupées ne s'interbloquent pas entre elles ») ;
3. **l'instantané de la transaction est pris après**, quand elle tient tout.

Deux transactions qui annoncent ne s'interbloquent jamais ; celle qui a attendu lit l'état
d'après l'attente et **n'échoue jamais** à la sérialisation sur ce qu'elle a annoncé.

## 2. La forme

`CALL acquire_locks('Table', [clé, clé, …]);` — en **première instruction** d'une transaction
explicite (`BEGIN TRANSACTION` ; en auto-commit, l'instruction est sa propre transaction et
l'annonce n'a rien à protéger : refus nommé). Une table de nœuds par appel ; plusieurs appels
pour plusieurs tables **se cumulent en une seule prise groupée tant qu'aucune instruction n'a
écrit** — non : plus simple et plus sûr, **un seul appel par transaction**, avec la forme
`CALL acquire_locks(['Table', [clés]], ['Autre', [clés]])` si plusieurs tables ; une seconde
annonce est refusée en le nommant. **[à trancher au code : la forme à plusieurs tables n'est
prise que si le banc ou rag3weaver la demande ; le témoin du banc n'annonce qu'une table.]**

Ce que l'annonce prend, pour chaque clé : `ROW{table, clé}` en **exclusif** et `INDEX{table}`
en **partagé** — exactement ce qu'A3′ prend pour une insertion et A4′ pour une écriture ; les
instructions qui suivent « reprennent un verrou tenu » sans attendre (V1). Une relation à créer
entre deux nœuds annoncés est couverte (A4′ prend les extrémités en partagé : déjà tenues en
exclusif). Un `COPY` n'a pas besoin d'annonce (il prend l'index en exclusif avant son plan).

La clé est donnée dans le type de la clé primaire de la table (ou convertible : un entier pour
un `INT64`, une chaîne pour un `STRING`) ; sa forme de verrou est celle de `NodeTable::lockKeyOf`
(la valeur imprimée), pour que l'annonce et l'écriture désignent la même ressource.

## 3. L'instantané pris après l'attente

C'est le point qui demande une chose nouvelle au moteur. Aujourd'hui `startTS` est fixé à
`beginTransaction` **[lu : transaction_manager.cpp]**. L'annonce, une fois ses verrous tenus,
**rafraîchit l'instantané** : `startTS` prend le dernier horodatage de validation, sous le mutex
du gestionnaire de transactions (`TransactionManager::refreshSnapshot(transaction)`, à écrire :
`lastTimestamp` est privé). Conditions, vérifiées avant de rafraîchir, sinon refus nommé :

- la transaction n'a **rien écrit** (stockage local vide, tampon d'annulation vide, journal
  local vide) — rafraîchir l'instantané sous des écritures changerait ce qu'elles ont vu ;
- l'annonce est la **première instruction** : une lecture faite avant verrait un état plus
  ancien que les lectures d'après. Le moteur ne compte pas les lectures ; la règle est dite à
  l'appelant (rag3weaver appelle en tête), et le témoin la joue. **[déduit : si un compteur de
  lectures existe quelque part dans `ClientContext`, le refus peut être exact ; à voir au code.]**

Sous le mode multi-écrivains éteint (le produit aujourd'hui), l'annonce vérifie ses arguments
et **ne fait rien** : pas de verrou (il n'y a qu'un écrivain), pas de rafraîchissement.

## 4. Où

- `src/function/table/acquire_locks_function.cpp` (ou dans le fichier des fonctions `CALL`
  existantes, selon la forme des `CALL` à effet) et son enregistrement dans
  `function_collection.cpp` — hors de mes dossiers habituels : rag3db-50 (hotfixs/tickets) y
  travaille (analyze) ; prévenu avant d'y entrer.
- `Transaction` : rien de plus que `acquireLocks` (A3′) ; `TransactionManager::refreshSnapshot`.
- `NodeTable::lockKeyOf` pour la forme de la clé ; le catalogue pour la table et le type de sa
  clé.

## 5. Les témoins (rouges d'abord)

- Le banc : `LockBench.AnnouncedLocksNeverDeadlockAndBothCommitInTurn` (rouge
  `acquire-locks-exists` aujourd'hui) : {1, 2} et {2, 1} annoncés, aucune erreur, les deux
  valident l'une après l'autre.
- Un fil, deux connexions (`insert_lock_test.cpp`) :
  1. **l'instantané après l'attente** : A `BEGIN`, A `SET` la ligne 1 ; B `BEGIN`, B annonce
     [1] (attend) ; A `COMMIT` ; B lit la ligne 1 avec la valeur de A, B la met à jour **sans
     erreur de sérialisation**, B valide — c'est ce que l'option A refuserait ;
  2. l'annonce après une écriture : refus nommé, la transaction continue ;
  3. une seconde annonce : refus nommé ;
  4. une table inconnue, une clé d'un autre type : refus nommés ;
  5. hors du mode multi-écrivains : l'annonce passe et ne prend rien
     (`getNumResources() == 0`) ;
  6. l'annonce puis le `ROLLBACK` : tout est rendu.

## 6. Ce que ça coûte, et ce que ça ne fait pas

Une prise groupée par transaction annoncée (triée : n log n) ; les écritures suivantes
reprennent leurs verrous en O(1) par ressource. L'annonce ne couvre pas ce qu'elle n'a pas
nommé : une écriture hors annonce suit A3′/A4′ (attente, puis erreur de sérialisation
possible). Elle ne change rien au `COPY`, à l'index vectoriel, ni au rejeu (une transaction de
reprise n'annonce pas).

## 7. Fait (11 octobre 2026, 0 h 15) — ce que le code a tranché

- **Un appel par transaction, une table** : la forme à plusieurs tables n'est pas prise ; une
  seconde annonce est refusée (« already called »).
- **« Rien écrit »** se lit sans compteur nouveau : `Transaction::hasWritten` = stockage local non
  vide ou tampon d'annulation non vide (une ligne, une version, le catalogue) ; le journal local
  ne compte pas (il porte toujours le `BEGIN`). Le moteur ne compte pas les lectures : la règle
  « en tête » est dite à l'appelant.
- **Le rafraîchissement** : `TransactionManager::refreshSnapshot` pose `startTS = lastTimestamp`
  sous le mutex du gestionnaire.
- **Le second argument est déclaré `ANY`** et vérifié liste au liage : un type `LIST` sans type
  d'élément ne se construit pas (« Trying to create nested type LIST without child
  information ») ; `project_graph` fait pareil.
- **Une table indexée** (index vectoriel) : l'annonce prend l'index en **exclusif**, sans que
  l'appelant le sache — deux annonces de clés différentes s'attendent, puis valident toutes les
  deux (`IndexLockBench.AnnouncedWritersOfOneIndexedTableWaitThenBothCommit`, le genre « index »
  de V1). C'est la règle tant que la maintenance de l'index n'est pas tenue au commit (la marche
  suivante) ; l'annonce le sait par le catalogue (`getIndexEntries` non vide au liage).
- **Témoins** : les trois d'un fil (l'attente dans un fil, puis la lecture de ce que le détenteur
  a validé et l'écriture sans erreur de sérialisation ; les refus ; rien hors du mode) et celui du
  banc, `AnnouncedLocksNeverDeadlockAndBothCommitInTurn`, vert en 39 ms — sa ligne sort de
  `known_red.txt`.
