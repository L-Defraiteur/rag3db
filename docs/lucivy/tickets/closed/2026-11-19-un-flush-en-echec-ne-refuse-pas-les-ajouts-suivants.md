# Un flush en échec ne refuse pas les ajouts suivants (deux écrivains)

- **État** : fermé (20 novembre 2026). Pas de perte : le défaut était dans le test (PR lucivy #56).
- **Gravité** : à départager. Ce peut être un vrai défaut (un écrivain dont un fil a échoué continue d'accepter des
  documents) ou un témoin périmé.
- **Atteignable en service** : inconnu.
- **Touche rag3weaver** : inconnu.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

Dans la suite `failpoints` de lucivy, `test_fail_on_flush_segment_but_one_worker_remains` rougit **sur main**
(792f514, joué à luciepc le 19 novembre) :
`cargo test --features failpoints --test failpoints`.

Le test ouvre un écrivain à deux fils, fait échouer une fois `FieldSerializer::close_term`, puis ajoute jusqu'à
100 000 documents. Il attend qu'un `add_document` finisse par rendre une erreur. Aucun ne la rend, et le test
panique à « add_document should have returned an error » (`tests/failpoints/mod.rs:107`).

Ce rouge a été vu en jouant le correctif du sursis de compaction (PR lucivy #50), mais il est le même sans lui.
La suite `failpoints` ne semble pas jouée par la CI, ce qui expliquerait qu'il soit passé inaperçu.

## Ce qu'il faut trancher

- Ce que l'écrivain doit faire quand un de ses fils a échoué : tout refuser, ou continuer sur les fils restants.
  Le nom du test dit « but one worker remains » et attend pourtant une erreur.
- Si la suite `failpoints` doit entrer dans la CI de lucivy.

## Le départage (20 novembre 2026)

Le témoin neuf `test_failpoints_documents_accepted_after_a_failed_flush_are_indexed_or_the_commit_fails` vérifie
qu'après un flush en échec, chaque document accepté est indexé, ou que le commit rend une erreur. Les deux issues ont
été vues à luciepc :
- 93 148 documents acceptés, puis le commit échoue en le disant (« simulatederror ») ;
- 100 000 acceptés, 100 000 indexés, cinq passages de suite.

**Aucune perte silencieuse.** Le vieux témoin exigeait qu'un `add_document` échoue après le flush raté, ce que lucivy
ne promet pas : l'erreur l'a atteint au 93 149e document une fois, et jamais en un million une autre. Il part (PR
lucivy #56), avec un commentaire qui dit pourquoi. La suite `failpoints` passe à 7 sur 7, cinq fois de suite. Rester
hors de la CI est sans conséquence ici : elle se joue à la main avec `--features failpoints`.
