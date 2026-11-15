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

## Reprendre

- Le worktree : `~/git_workspaces/rag3db-csr` (portable). L'arbre de bâti : `~/git_workspaces/rag3db-colonnes/build/moteur` (luciepc), sous `poste lourd`, `-j 8`, avec `rag3db_vector_extension` dans toute cible de test.
- Atterri cette nuit : l'IN haché `e5e626803`, le pliage des listes `1d5580d74`, le point de faute et les témoins des deux sens `e8472e053`. La lib suivante les portera ; la lib posée est `373b5477b`.
- En cours : le planificateur (A), branche à ouvrir depuis master.
- À faire : le dictionnaire sous ASan, la nuit prochaine, en fenêtre (ticket `2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`, core 3980144). L'ADD COLUMN sous une écriture ouverte, ticket de hotfixs. Le planificateur des sauts, ticket d'ED. `DropNodeTableReclaim`, rouge sur master (`392e63cd7`).
