# L'avis « index prêt » dans la fenêtre détachée du dépôt

- **État** : fermé le 20 novembre 2026 par la session tty. La fenêtre détachée du dépôt dit « l'index … est prêt » dans son en-tête, une fois, au flux seul (`IndexPretFenetre`, `#index-pret-fenetre`). Témoin `daemon_host::chat::tests::a_detached_window_of_the_repository_says_the_index_is_ready_once` : rouge à luciepc (b8d94bd939, la fenêtre ne disait rien), vert avec le code (465dd089dd), comme la page du chat, les accords, la fenêtre guidée, `ide_par_le_flux` et le bundle.
- **Famille** : produit
- **Gravité** : faible (l'avis est dit dans le chat ; seule la fenêtre détachée ne le dit pas).
- **Touche rag3weaver** : oui (l'IDE, `ui/ide`).
- **Atteignable en service** : oui (une fenêtre détachée du dépôt, pendant un index en retard).

## Le défaut

La tâche de nuit de c7 (20 novembre) demande l'avis « index prêt » dans la fenêtre du chat **et** dans la fenêtre de la vue concernée. Le lot `index-pas-pret` le dit dans le chat. La fenêtre détachée du dépôt (`?fenetre=depot`) n'existe que dans la chaîne `ide-fenetres`, absente de la base de recherche sur laquelle `index-pas-pret` est posé.

## Pourquoi pas tout de suite

Le pousseur ne prend qu'un parent. Poser ce lot sur les deux demandait de reposer les 50 commits de la chaîne des fenêtres sur la tête de l'index : le bundle de l'IDE est en conflit à presque chaque pas, et des patches résolus à la main ne sont plus identiques à ceux qui atterrissent. La repose ne les écarterait plus par patch.

## Ce qu'il faut faire

- Dans l'en-tête d'une fenêtre détachée (`main.tsx`, à côté de `AccordsEnAttente`), écouter `IndexReady` sur le sujet `index`, pour le backend de la fenêtre (`d.backend === manifest`). Le dire une fois, dans le même style que la ligne `index-pret` du chat.
- Un témoin navigateur rouge d'abord, sur le banc `BancPasPret` (`daemon_host::chat::tests`) : une fenêtre du dépôt ouverte avant le tour, puis la couche rattrapée. La fenêtre et le chat le disent chacun une fois.
- Rien au repos : l'avis vient du flux seul.
