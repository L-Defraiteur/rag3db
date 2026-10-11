# Le rendu d'impact change d'ordre d'un processus à l'autre

- **État** : corrigé (11 octobre 2026, arbre principal) — le parcours trie chaque niveau avant le budget
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

## Correctif

La cause n'était pas le rendu mais le parcours : `parcourir` prenait les nœuds de chaque
niveau dans l'ordre des lignes rendues par la base, qui suit l'ordre d'insertion, lui-même
tiré de tables de hachage dont la graine change d'une table à l'autre (même dans un seul
processus). Le rendu changeait d'ordre, et **le budget coupait d'autres nœuds**. Chaque niveau
est maintenant trié avant le dédoublonnage et le budget (`ordre_stable` : le rang de la voie,
puis chemin, ligne, titre, uuid). Le rang garde la priorité d'avant : une arête, puis à
travers un carrefour, puis par le trait, puis par le nom seul.

Témoin : `e2e_impact::le_rendu_d_impact_est_le_meme_d_un_catalogue_neuf_a_l_autre`. Il rend
dans quatre catalogues neufs de `src/dataflow`, avec un budget court en plus, et compare les
Markdown par `==`. Rouge sans le tri : le catalogue 1 différait dès la ligne 9.

