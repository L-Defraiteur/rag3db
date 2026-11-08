# La marque « fichier » relie une locale JavaScript au premier homonyme du fichier

- **État** : ouvert (8 novembre 2026) — relevé par le rejugement de l'oracle TypeScript (21 arêtes sur
  300) ; la règle est à écrire.
- **Gravité** : réponse fausse (un usage relié à la locale d'une autre fonction).
- **Atteignable en service** : oui, tout fichier JavaScript ou TypeScript où plusieurs fonctions
  déclarent une locale du même nom (`const ctx`, `const res`).
- **Touche rag3weaver** : oui (`src/code.rs`, la résolution « fichier ») et codeparsers (les portées).
- **Famille** : produit

## Ce que c'est

Dans `extension/lucivy/ld-lucivy/playground/js/lucivy-worker.js`, plusieurs fonctions déclarent
`const ctx`, `const res`, `const json`. Une mention de `ctx` à la ligne 421 est reliée à la
déclaration des lignes 308-310 (une autre fonction) ; typescript-language-server rend celle de la
ligne 355, dans la fonction de la mention. Sur 300 arêtes TypeScript et JavaScript tirées au hasard
(graine 25, arbre a72acfeef), 21 sont de cette forme, toutes de marque « fichier ».

## Ce qu'on sait

- La marque « fichier » cherche le définisseur dans le fichier de la mention, sans tenir compte de la
  fonction qui l'enferme : le premier homonyme gagne.
- La page `extension/rag3weaver/docs/10-octobre-2026-18h20/codeparsers/11-oracles-par-langage.md`
  porte les verdicts (`rejuge-lsp/verdicts-typescript.tsv` à luciepc).

## Ce qu'il faut pour le fermer

1. Une locale (déclarée dans le corps d'une fonction) ne se résout que depuis sa portée : la
   fonction qui l'enferme et ses portées filles ; à défaut, rien plutôt qu'un homonyme d'à côté.
2. Un témoin sur un extrait à deux fonctions qui déclarent chacune `const ctx`.
3. Le rejugement des 21 arêtes par l'oracle.
