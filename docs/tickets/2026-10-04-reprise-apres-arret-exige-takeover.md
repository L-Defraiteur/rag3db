# Une synchronisation reprise après un arrêt brutal exige `takeover`

- **État** : ouvert (accepté aujourd'hui ; décision de l'orchestration : ticket, pas maintenant).
- **Gravité** : blocage (la reprise refuse tant qu'on ne dit pas `takeover`).
- **Atteignable en service** : oui, après tout arrêt brutal d'une synchronisation.
- **Touche rag3weaver** : oui (`code_sync::sync_source`, sessions de snapshot).

## Ce que c'est

Un processus tué pendant une synchronisation laisse sa session de snapshot ouverte. La
synchronisation suivante, dans un processus neuf, refuse : « une session est déjà ouverte sur
ce périmètre (…) ; une seule à la fois — la finir, l'abandonner, ou takeover pour la
reprendre ». Il faut passer `SourceSyncOptions::takeover`, même quand le propriétaire de la
session est mort.

## Recette

Pas de Cypher brut : c'est le registre de sessions de rag3weaver. `e2e_tx_par_paquet_arret`
la joue : un écrivain tué par SIGKILL au paquet 2 (`RAG3WEAVER_TEST_KILL_IN_BATCH=2`),
puis une synchronisation dans un processus neuf. Sans `takeover`, elle rend l'erreur
ci-dessus.

## Témoin

`extension/rag3weaver/tests/e2e_tx_par_paquet_arret.rs` passe `takeover: true` au
repreneur : c'est la voie actuelle. Un témoin du comportement voulu reprendrait sans
`takeover` et attendrait les mêmes comptes qu'une passe sans arrêt.

## Ce qu'il faut pour le fermer

Savoir que le propriétaire d'une session est mort : un identifiant de processus et d'hôte
noté à l'ouverture de la session, ou un bail renouvelé, à la manière d'un verrou consultatif
de PostgreSQL qui tombe avec sa connexion. Une session dont le propriétaire est mort se
reprend alors d'elle-même, et le dit au rapport. Une session vivante reste refusée.
