# Trois rendus d'arbre à fondre

- **État** : ouvert — deux sur trois fondus (6380dd8a5)
- **Gravité** : aucune pour l'agent aujourd'hui (même forme partout) ; dette
- **Atteignable en service** : non (pas de défaut visible)
- **Touche rag3weaver** : oui (rendu des résultats et des outils)

## Ce que c'est

Les arbres que l'agent lit sont dessinés à trois endroits avec la même forme
(titre, `├── [RELATION]`, voisins `nom (genre) @ lieu`) :

1. le « Dependency Graph » de `templates/render/results.md.jinja` ;
2. le « Graphe » de `src/code_tools.rs` ;
3. la section Liens de `src/dataflow/links_nodes.rs`.

Depuis 6380dd8a5, 2 et 3 passent par `crate::arbre::arbre`. Le gabarit
jinja garde sa propre boucle : une retouche de la forme d'un côté doit être
recopiée de l'autre à la main, sans qu'aucun test ne compare les deux.

## Recette

Aucune : pas de défaut aujourd'hui, les sorties concordent.

## Témoin

`arbre::tests::la_forme_du_graphe_de_dependances` fixe la forme côté Rust.
Il manque un test qui rende le même petit graphe par le gabarit et par
`arbre()` et compare les octets.

## Pour le fermer

Au choix : exposer `arbre()` comme filtre ou fonction minijinja et l'appeler
depuis le gabarit, ou bien écrire le test de concordance ci-dessus. La
première voie supprime la dette ; la seconde la surveille.
