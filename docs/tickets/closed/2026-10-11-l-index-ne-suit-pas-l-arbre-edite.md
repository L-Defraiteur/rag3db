# L'index ne suit pas l'arbre que l'agent édite

- **État** : fermé (20 novembre 2026), au mot de c7 : le minimum et la forme 2 sont livrés ; la vue par défaut d'une
  session vit dans le ticket `arbre-principal/tickets/2026-11-16-les-vues-des-worktrees-des-sessions.md`.
- **Gravité** : réponse fausse (les outils répondent sur un autre code que celui que l'agent édite, sans le dire)
- **Atteignable en service** : oui, dès qu'un agent travaille dans un worktree ou sur une branche autre que celle indexée
- **Touche rag3weaver** : oui (hôte, manifeste du gabarit `code`, montre)
- **Famille** : produit

## Ce que c'est

La base `code-rag3db` indexe l'arbre principal de ce poste, sur master, à 15 h 22. Le cœur
édite sa branche dans un worktree sur luciepc. `usages` et `impact` lui répondent sur master,
et ses propres changements n'entrent jamais dans l'index. Rien dans la réponse ne dit de quel
arbre ni de quel commit elle vient.

## Ce qu'il faut pour le fermer

Les formes possibles sont dans la page « ce qui manque à un agent dans cet IDE »
(`extension/rag3weaver/docs/10-octobre-2026-18h20/codeparsers/04-…`, en tête) :
1. une base par arbre de travail, montée par l'hôte à la demande, avec la montre dessus ;
2. un index qui suit la branche et réindexe le delta au changement ;
3. pour un arbre distant, l'index sur le poste de l'arbre : un hôte par poste, et un client qui
   sait lequel interroger.

Au minimum, dès maintenant : chaque réponse dit l'arbre et le commit indexés.

## La fermeture (20 novembre 2026)

- **Le minimum demandé** est fait : chaque réponse de lecture dit l'arbre et le commit indexés
  (`Catalog::indexed_tree_line`).
- **La forme 2** (un index qui suit la branche et réindexe le delta) est livrée par 2a′-3 : une couche d'histoire par
  branche suivie, une couche de travail par worktree, le sélecteur `worktree:<motif>`, et une vue (`view`) sur chacune
  dans les outils (2b-C). Une vue sans montre voit son retard depuis le lot `arbre-vue-sans-montre`.
- **Ce qui reste** : qu'un agent qui appelle sans `view` lise par défaut le worktree de sa session, et non le
  backend. C'est le ticket `arbre-principal/tickets/2026-11-16-les-vues-des-worktrees-des-sessions.md`, destiné à
  Codex ; ce ticket-ci s'y renvoie.
