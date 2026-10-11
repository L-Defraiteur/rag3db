# La montre retirait tout l'index quand la racine du workspace disparaissait

- **état** : corrigé (le commit des trois défauts de la montre, 11 octobre 2026)
- **gravité** : perte (l'index vidé par lots), blocage (le catalogue tenu des minutes)
- **atteignable en service** : oui — un worktree effacé après sa fusion, un disque démonté
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Vu sur l'hôte de luciepc : la racine d'un backend effacée, la montre voyait tous les
fichiers « supprimés », et la réaction les retirait de l'index par lots en tenant le
catalogue ; `search_code` rendait « IO error … No such file or directory ».

## Le témoin

`watch::tests::a_vanished_root_pauses_the_watch` (rien n'est retiré, la montre reprend
quand la racine revient) ; `backend::tests::a_vanished_root_pauses_the_watch_and_keeps_the_index`
(l'état dit `watches.code.paused = « racine absente : … »`, l'index garde ses fichiers, la
liste de la source nomme la racine).

## La correction

Racine absente, ou liste vide alors que des fichiers étaient connus : la montre se met en
pause, n'émet rien, le dit une fois sur le bus et dans `index_state` ; elle reprend quand
la racine revient (la réconciliation rattrape). `WorkingTree::list` nomme la racine absente.
