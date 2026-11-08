# La marque « fichier » relie une locale JavaScript au premier homonyme du fichier

- **État** : corrigé (8 novembre 2026) — codeparsers c660893 (`visible_until_line`, le repli du
  résolveur par la portée) ; à l'oracle, 30 faux tombent à 10 sur les mêmes mentions, 20 deviennent
  justes, aucun juste ne devient faux.
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

## La correction

- **La cause, en deux temps.** Le gestionnaire `self.onmessage = async (e) => …` n'est pas un scope :
  les `const` de ses blocs passent par `extract_global_variables`, comme des globales. Puis le repli de
  `resolve_local_scope_references` prend le premier homonyme du fichier (candidats triés par ligne).
- **La règle** : `ScopeInfo::visible_until_line`, la dernière ligne où une variable se voit (`const` et
  `let` : la fin de leur bloc ; `var` : la fin de leur fonction ; une globale du programme : `None`).
  Le repli prend, parmi les homonymes du fichier, la déclaration la plus proche qui couvre la mention ;
  aucune ne la couvre, aucune locale — jamais l'homonyme d'un autre bloc.
- **Les témoins** : `codeparsers/tests/locales_js.rs`, quatre ; trois rougissent sans le filtre.
- **La mesure** (luciepc, code-lourd a72acfeef indexé deux fois sur master aed8184b4, lib cbc06c3e1) :
  sur les mêmes 298 mentions que le rejugement (324 arêtes), juste 259 → 279, faux 30 → 10 ; les 20
  changements vont tous de faux à juste. Sur tout le TypeScript et le JavaScript, 78 arêtes ajoutées
  (78 justes) et 82 ôtées (79 fausses, 2 décl, 1 muette).
