# Le registre « indexation en cours » confondait les copies de travail

- **état** : corrigé (le commit des trois défauts de la montre, 11 octobre 2026)
- **gravité** : réponse fausse (une édition reprise dans le mauvais catalogue)
- **atteignable en service** : oui — un hôte qui sert plusieurs backends de code
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Le registre de `code_sync` (une indexation en cours, ses fichiers lus, ceux à reprendre)
était clé par `source_id` ; toutes les copies de travail ont le même, `local`. Une
indexation du backend A faisait mettre en reprise les éditions du backend B, reprises
ensuite dans le catalogue de A. Trouvé par les témoins de la montre, plusieurs backends
dans le même processus.

## Le témoin

Les témoins de la montre joués ensemble (`the_index_follows_files_edited_by_hand`,
`a_file_created_by_hand_is_indexed`, `the_watch_reconciles_what_moved_before_it_started`,
`the_lag_is_said_between_an_edit_and_its_reindexing`) pendant qu'un autre backend synchronise
400 fichiers (`stopping_cuts_a_long_reaction_and_the_restart_resumes_it`) : rouges avant,
verts après.

## La correction

`code_sync::registry_key(source)` = le curseur de la source (`worktree:<racine>`),
employé partout où le registre est consulté.
