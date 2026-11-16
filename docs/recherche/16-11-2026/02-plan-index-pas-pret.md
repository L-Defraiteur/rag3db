# Plan : « index pas prêt, ~N s » et l'attente bornée des outils de recherche

Session recherche, 16 novembre 2026, point 7 de la vision de Lucie. Le pas et sa notification sont
dans la page 12 d'archi (`docs/rag3weaver-archi/15-11-2026/12-conception-l-attente-d-index-du-pas.md`).
Ici, la part de recherche : l'estimation, l'attente bornée et la réponse des outils de lecture.

## Ce qui existe

- **`catalog/retard.rs` (`Lag`)** : par cellule, les fichiers changés et pas encore réindexés, avec
  l'instant du plus ancien. La montre les note, la réindexation les efface.
- **Les outils marqués `freshness: true`** (search_code, usages, outline, read_file…) disent ce retard
  dans leur réponse : `freshness.lag_files`, et une ligne en tête.
- **`ReindexFileNode`** mesure le temps tenu de chaque lot de fichiers, pour sa ligne de journal.

## Ce que le lot ajoute (noms en anglais)

1. **Le débit observé** : `Lag::observe(cell, files, held)`. `ReindexFileNode` l'appelle à la fin de
   chaque lot. Une moyenne glissante des millisecondes par fichier se tient par cellule. Aucune
   constante : sans observation, l'estimation est dite inconnue.
2. **L'estimation** : `Lag::estimate(cell) -> Option<IndexEstimate { pending_files, eta_ms: Option<u64> }>`,
   soit les fichiers en attente fois le débit observé.
3. **L'attente bornée** : `Lag::wait_caught_up(cell, deadline) -> bool`, sur une `Condvar` que `done`
   réveille.
4. **Le réglage** : `search.index_wait_s` dans le manifeste, 6 par défaut. Une clé inconnue sous `search`
   est un refus qui la nomme.
5. **La réponse d'un outil `freshness`**, dans `call_tool_on`, avant d'exécuter le graphe :
   - aucun retard : rien ne change ;
   - estimation au plus `index_wait_s`, ou inconnue : l'appel attend au plus `index_wait_s`. S'il est
     rattrapé, la réponse est complète, sans un mot sur l'attente ;
   - estimation au-dessus, ou toujours en retard au bout de l'attente : l'appel répond tout de suite, sur
     ce qui est indexé. Il dit `"index": {"ready": false, "eta_s": N | null, "pending_files": k}` et une
     ligne en tête : « index pas prêt, environ N s » (ou « durée inconnue »).
6. **Ce qui n'est pas ici** : la notification unique à l'index prêt, et le silence sur les propres
   écritures du pas, sont la part d'archi. `Lag::estimate` et `Lag::wait_caught_up` sont son contrat.

## Les témoins, rouges d'abord (`backend.rs`)

- `a_read_under_a_short_estimate_waits_and_answers_complete` : le débit semé à 100 ms par fichier, un
  fichier écrit, puis `usages` aussitôt. La réponse contient le fichier neuf, sans champ `index`.
- `a_read_under_a_long_estimate_answers_at_once_and_says_not_ready` : réactions arrêtées, débit semé à
  20 s par fichier. La réponse vient en moins d'une seconde avec `index.ready` faux, `eta_s` 20 et la
  ligne « index pas prêt, environ 20 s ».
- `a_read_without_an_observed_rate_waits_at_most_the_setting` : `index_wait_s` à 1, aucun débit,
  réactions arrêtées. La réponse vient entre 1 et 3 s, avec `index.ready` faux et `eta_s` nul.
