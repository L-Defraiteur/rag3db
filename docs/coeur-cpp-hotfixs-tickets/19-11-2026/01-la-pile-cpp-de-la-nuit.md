# La pile C++ de la nuit du 18 au 19 novembre

## Ce qu'elle porte

Quatre lots, une seule passe sur la tête, un seul push de la tête (décision de c7, accord du cœur) :

| ordre | branche | tête |
|---|---|---|
| 1 | `add-column-under-readers-2` (cœur) | 97aaa8eb2 |
| 2 | `dropped-column-under-old-reader-2` (cœur) | aff9ffd05 |
| 3 | `insert-after-forced-copy-keeps-rows-scanned-2` (hotfixs, avec (a) `rows_outside_graph_at_most`) | 131fd6d171 |
| 4 | `create-index-reads-its-cached-column-3` (hotfixs, l'entrée 2) | 22f813f91a |

Base : master 094ad1fc4. Un push unique de la tête, en mode normal, par la voie C++ transférable : le mode pile de
la garde n'a pas l'exception C++ (ticket
[le mode pile de la garde…](../tickets/2026-11-18-le-mode-pile-de-la-garde-n-a-pas-l-exception-cpp.md)).

## Le faux rouge du juge de concurrence

La passe de la tête (`journaux-hotfixs-pile-matin` à luciepc) s'est arrêtée deux fois sur le juge
`concurrence_test.known_red`, vert les deux fois :
1. `ctest --test-dir <build>` à la racine du build ne trouve aucun test : le juge s'enregistre dans
   `<build>/test/transaction/concurrence`.
2. Depuis ce dossier, il a passé (664 s), mais le contrôle attendait « 100% tests passed, 0 tests failed out of 1 » ;
   ctest écrit « 100% tests passed out of 1 ».

Pour tout script de passe : `cd <build>/test/transaction/concurrence && ctest -R '^concurrence_test\.known_red$'`,
juger le code de sortie, et ne grepper que `100% tests passed(, 0 tests failed)? out of 1`.

## Un piège de témoin : un BEGIN abandonne un point de reprise automatique en attente

Relevé en écrivant le témoin du dernier lecteur en quarantaine (`QuarantineOpenTransactions`, branche
`quarantine-last-reader-checkpoint`). Quand une validation a posé une demande de point de reprise automatique,
les nouvelles transactions sont retenues. Un `BEGIN` fait à ce moment-là attend au plus `AUTO_CHECKPOINT_WAIT`, puis
**abandonne la demande** (`cancelAutoCheckpointRequestNoLock`) avant de continuer. Un contrôle de mise en place placé
là (un BEGIN pour vérifier la quarantaine) retirait l'objet du cas : le dernier lecteur n'avait plus rien à écrire, et
le témoin passait vert sans rien prouver.

Pour tout témoin d'un point de reprise automatique : ne rien ouvrir entre la demande et le départ du dernier, et
vérifier que la demande est bien là au moment voulu (`FlakyCheckpointer::autoCheckpointRequested`). Une
contre-épreuve sans la faute montre que la recette exerce le chemin.
