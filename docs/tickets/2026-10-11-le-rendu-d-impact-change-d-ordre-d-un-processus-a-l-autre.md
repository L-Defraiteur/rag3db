# Le rendu d'impact change d'ordre d'un processus à l'autre

- **État** : ouvert (11 octobre 2026) — l'arbre principal le ferme par un tri stable du rendu
- **Gravité** : réponse trompeuse (un agent qui compare deux rendus croit à un changement)
- **Atteignable en service** : oui, à chaque appel d'impact dans un nouveau processus
- **Touche rag3weaver** : oui (`NeighborhoodReport::markdown`, `src/dataflow/neighborhood_nodes.rs`)

## Ce que c'est

Sur le même catalogue, le même nom et la même profondeur, le Markdown d'impact n'a pas
le même ordre de lignes d'un processus à l'autre : l'ordre suit des tables de hachage,
dont la graine change à chaque processus. Vu par la mesure d'impact
(`e2e_mesure_impact`, corpus `src/daemon`) : deux passes de suite, sans changer le
code, rendent pour `joindre` et `servir_lie` des empreintes différentes. Les lignes,
une fois triées, sont les mêmes ; la mesure compare désormais les lignes triées.

## Ce qu'il faut pour le fermer

Un tri stable de ce que le rendu parcourt (par niveau, chemin, ligne, titre), et un
témoin qui rend deux fois dans deux catalogues neufs et compare les textes.
