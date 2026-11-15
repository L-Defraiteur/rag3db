# Cœur c++, 15 novembre 2026 : rapport de session

## Le planificateur (A) de l'IN : la conception, avant le code

Ticket : `docs/everything-declarative/tickets/2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md`.
La forme (B), une liste constante hachée, est sur master (`e5e626803`) : le N² a disparu, mais la
table reste balayée, ce qui coûte cher pour 10 clés sur 10 millions de lignes.

1. **L'optimiseur.** Dans `FilterPushDownOptimizer`, sur une seule table de nœuds,
   `LIST_CONTAINS(<liste constante>, n.pk)` (un `IN` sur la clé, la liste littérale ou en paramètre)
   devient un `PRIMARY_KEY_SCAN` dont la clé est cette liste. C'est la même réécriture que
   `n.pk = <constante>` aujourd'hui, avec un drapeau « liste » dans `PrimaryKeyScanInfo`. Hors de ce
   cas (une clé castée, plusieurs tables, une liste qui dépend de la ligne), le FILTER reste.
2. **L'opérateur.** `PrimaryKeyScanNodeTable`, en mode liste, évalue la liste une fois, en
   parcourt les éléments, saute les NULL, cherche chaque clé par l'index et rend une ligne par clé
   trouvée.
3. **La sémantique.** Un nœud est rendu au plus une fois : les doublons de la liste sont écartés
   par l'ensemble des décalages déjà rendus. Les lignes viennent dans l'ordre de la liste, ce que
   permet un `IN`, qui ne promet aucun ordre. Une liste vide ne rend rien.
4. **Le témoin, rouge d'abord.**
   - Le plan d'un `IN` sur la clé : `PRIMARY_KEY_SCAN_NODE_TABLE` dans l'EXPLAIN, et plus de
     FILTER.
   - Le coût de 10 clés sur 200 000 lignes : au plus 3 fois la forme `UNWIND … MATCH {k: u}`.
   - La sémantique existante (absentes, doublons, entiers, liste qui suit la ligne, NULL, liste
     vide) reste verte avant et après.

## L'ADD COLUMN sous une écriture ouverte : la conception, avant le code

Ticket : `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-ecriture-ouverte.md`.
Aujourd'hui, le gestionnaire de verrous (`transaction/lock_manager.h`) ne connaît que des lignes
(`ROW`) et des index (`INDEX`). Un `ALTER` ne prend rien qui le fasse attendre un écrivain ouvert :
il change le catalogue, qui est versionné, et le COMMIT de l'écrivain rattrape le défaut de la
colonne neuve.

1. **Ce que voit l'écrivain ouvert.** Le schéma du début de sa transaction, et lui seul. Avec le
   correctif, l'`ALTER` attend que cet écrivain ait fini : aucune colonne n'apparaît ni ne
   disparaît pendant sa transaction. Son COMMIT n'a plus de défaut à rattraper.
2. **Ce que voit le lecteur d'instantané.** Le catalogue de son instantané, comme aujourd'hui. Il
   ne prend aucun verrou, n'est pas attendu par l'`ALTER`, et ne le fait pas attendre.
3. **L'attente, pas le refus**, comme PostgreSQL (`ACCESS EXCLUSIVE`) et le reste du gestionnaire,
   où un écrivain qui trouve la ressource prise attend.
   - Un genre de ressource neuf, `TABLE`.
   - Un écrivain prend `TABLE` en partagé à sa première écriture dans une table : dans
     `LocalStorage::getOrCreateLocalTable`, et sur les chemins qui la contournent, le COPY et
     les écritures directes des index.
   - Un `ALTER` sur une table (ADD, DROP ou RENAME d'une colonne, DROP TABLE) la prend en
     exclusif, et attend.
   - L'attente est bornée par le délai de verrou habituel. Le graphe d'attente existant détecte
     les interblocages : un écrivain qui demande un `ALTER` sur une table qu'un autre écrit.
4. **Le témoin, rouge d'abord.**
   - `AnOpenWriterCommitsAfterAnAdd` réécrit en contrat : l'ADD de B finit **après** le COMMIT de A.
   - Un DROP d'une colonne que A ouvert a écrite : il attend, puis A valide ses lignes entières.
   - Un ADD puis un point de reprise automatique, pendant que A est ouvert.
   - Un écrivain qui arrive pendant l'`ALTER` attend sa fin.
   - **Ce qui change pour le banc** : `DdlEveryMinute` mesure les attentes des écrivains pendant
     un DDL (`…-holds-writers-at-most-its-duration`). Avec l'attente, un DDL dure aussi longtemps
     que l'écrivain le plus long ouvert sur sa table. Les bornes de ces contrôles sont à revoir
     avec une mesure, pas à la main.

**Le choix à trancher (c7, Lucie)** : l'attente ci-dessus, ou garder le comportement d'aujourd'hui
(l'`ALTER` passe, et le COMMIT rattrape le défaut), écrit comme un contrat. L'attente ferme la
famille des cas non joués (un DROP d'une colonne écrite par un écrivain ouvert), dont le plantage
du dictionnaire est peut-être un membre. Le contrat ne coûte rien, mais laisse ces cas ouverts.
Je recommande l'attente.

## Reprendre

- **Le worktree** : `~/git_workspaces/rag3db-csr` (portable). **L'arbre de bâti** :
  `~/git_workspaces/rag3db-colonnes/build/moteur` (luciepc), sous `poste lourd`, `-j 8`, avec
  `rag3db_vector_extension` dans toute cible de test.
- **Atterri cette nuit** : l'IN haché `e5e626803`, le pliage des listes `1d5580d74`, le point de
  faute et les témoins des deux sens `e8472e053`, l'IN sur la clé `7897f2453`.
- **En cours : le lot du verrou de schéma**, branche `alter-attend-les-ecrivains`, tête
  `d5f5325f0`, base `a2f9c9b65` (master n'a pas bougé en code depuis).
  - Ce qu'il fait : un genre TABLE au gestionnaire de verrous. Les écrivains le prennent en
    partagé à leur première écriture dans une table, un CREATE_VECTOR_INDEX aussi ; ALTER et DROP
    TABLE le prennent en exclusif et attendent. Une écriture dont la table a changé de
    disposition de colonnes depuis le départ de sa transaction est refusée sous le préfixe stable
    `Schema changed, retry:` (`Transaction::SCHEMA_CHANGED_RETRY`) et se recommence.
  - Changement de contrat : de deux ALTER d'une même table, le second attend le premier, est
    refusé sous ce nom, puis passe relancé. Il était refusé tout de suite avant. La ligne va aux
    nouveautés avec le push.
  - Ce qui est vert sur `b7251cb77` : transaction_test entier (385), le témoin du commentaire
    (rouge avant), le témoin d'index (une garde, verte avant aussi). Dans DdlEveryMinute, les ADD
    durent 14 à 152 ms, sous la borne du paquet.
  - **Ce qui bloque le push** (c7, 10 h 5x) :
    - SIGSEGV dans DdlEveryMinute (core 2034048, pile lue : `readCompressedValuesToVector` sous un
      balayage filtré, dans les relectures des colonnes ajoutées). Départage en cours : trois tours
      sur master `a2f9c9b65`, binaire gardé (`~/.cache/rag3db-moteur-notes/ddl-base/`). Si master
      plante aussi : ticket avec la pile, le lot pousse. Sinon : lire le chemin de l'ADD COLUMN
      sous écrivain.
    - `ChangesCommittedDuringACreateAreSeen` rouge une fois : 24 tours sur master et sur le lot
      (`~/.cache/rag3db-moteur-notes/moved/`).
    - `create-rel-table-holds-writers-at-most-its-duration`, 5 ms au-dessus de sa borne au tour 1 :
      à lire.
  - Le push : mode simple, base `a2f9c9b65`, sous `poste push`, au « à toi » de c7. Note d'ABI
    pour la bascule de G : la disposition de `Transaction` change (un mutex, un ensemble et un
    booléen de plus), `LockResource` gagne le genre `TABLE`.
- **Ensuite, dans l'ordre** :
  - la bissection du CREATE_VECTOR_INDEX à quatre minutes (ticket
    `2026-11-15-un-create-vector-index-sous-quarante-ecrivains-dure-quatre-minutes.md`) ;
  - le dictionnaire sous ASan, en fenêtre ;
  - le verrou pris à la liaison (ticket
    `2026-11-15-une-ecriture-se-lie-avant-de-prendre-le-verrou-de-sa-table.md`) ;
  - `DropNodeTableReclaim`, rouge sur master (`392e63cd7`).
