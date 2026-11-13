# Une mention d'un import ES externe n'a pas de cible à elle (la sorte `import_binding`)

- **État** : ouvert (11 novembre 2026) — **compté le 13 novembre** (+9,2 % de scopes, +6,5 % si on ne
  le fait que pour les modules externes) : au-delà de « quelques pour cent », la décision revient à Lucie
  (règle de c7 du 8 novembre).
- **Gravité** : réponse incomplète (une mention sans cible, ou une cible moins précise que son nom).
- **Atteignable en service** : oui, tout fichier JavaScript ou TypeScript qui importe d'un module hors de
  l'arbre (`import path from "path"`), et sans doute les autres langages pour leurs imports externes.
- **Touche rag3weaver** : oui (une sorte de scope nouvelle : `src/code.rs`, les gabarits, l'IR, les vues
  de l'IDE) et codeparsers (la produire).
- **Famille** : produit

## Ce que c'est

Une ligne `import x from "y"` ne fait **aucun** scope dans l'index : les noms qu'elle lie n'ont pas de
nœud à eux. Une mention de `x` dans le fichier n'a donc rien à viser quand le module `y` est hors de
l'arbre. Le lot `require` (branche rag3db `codeparsers/require-js`, non poussé au 11 novembre ; voir
`docs/codeparsers/11-11-2026/01-rapport-de-session.md`) prend une forme moins large : la mention va à la
**zone de fichier** (`file_scope_NN`, le texte entre deux scopes) qui contient la ligne d'import, avec la
marque `resolution = "liaison"`. On obtient la zone, pas le nom.

La forme plus précise serait une **sorte de scope `import_binding`** : un scope par nom importé, à la
ligne de son import, cible des mentions quand l'import ne se résout pas.

## Ce qu'on sait déjà (mesure du lot `require`, 9 novembre 2026)

Sur code-lourd (l'arbre rag3db a72acfeef, indexé deux fois à luciepc, résultats dans
`~/.cache/codeparsers-notes/require-mesure-2/resume.txt`), la seule forme « zone » ajoute **415 arêtes
`liaison`** pour les mentionneurs TypeScript/JavaScript ; les CONSUMES de ces mentionneurs passent de
1 381 à 1 878 (+36 %). L'oracle TypeScript les rend « muettes » : le serveur de langage ne désigne rien
pour ces noms de modules externes. C'est un ordre de grandeur des mentions concernées en TS/JS, pas le
nombre de scopes qu'`import_binding` créerait (un scope par nom importé, pas par mention).

## Ce qu'il faut pour le fermer

1. **Le compte** sur code-lourd : le nombre de noms importés (par langage : TS/JS, Python, Rust, Go), la
   part qui vient de modules hors de l'arbre, et ce que cela ajouterait en scopes, en arêtes et en
   vecteurs d'embarquement (si ces scopes sont embarqués). Par exemple une sonde sur les
   `import_references` de codeparsers sur l'arbre, sans indexer.
2. **La décision**, avec ce chiffre : la sorte `import_binding`, la zone (forme du lot `require`), ou rien.
3. Si `import_binding` : un témoin par langage, la sorte dans les gabarits et `code.rs`, une mesure A/B
   avec l'oracle.

## Le compte (13 novembre 2026)

Une sonde de codeparsers (master a730a9b), jouée à luciepc sous `poste lourd`, **sans indexer** et sans
toucher aucune base : elle analyse les 3 461 fichiers de code de code-lourd (l'arbre rag3db a72acfeef,
`git ls-files` filtré par extension) et compte, par langage, les noms que lient les imports (alias, sinon
le nom importé ; les imports sans nom, `SideEffect`, exclus). Sonde et résultat :
`~/.cache/codeparsers-notes/import-binding/` à luciepc (`sonde_import_binding.rs`, `sonde.log`) ;
3,6 s d'analyse ; 1 erreur d'analyse.

| langage | fichiers | scopes | noms liés par un import | dont modules externes | noms distincts par fichier | poids (distincts / scopes) |
|---|---|---|---|---|---|---|
| Rust | 356 | 21 504 | 5 774 | 3 490 | 5 370 | 25,0 % |
| Python | 174 | 2 842 | 1 083 | 1 054 | 1 083 | 38,1 % |
| TypeScript/JavaScript | 68 | 948 | 123 | 88 | 123 | 13,0 % |
| C | 1 538 | 22 767 | 0 | 0 | 0 | — |
| C++ | 1 324 | 23 172 | 0 | 0 | 0 | — |

En C et C++, un import est un `#include` : il ne lie aucun nom.

**Ce que pèserait `import_binding`** sur code-lourd :

- un scope par nom distinct importé dans chaque fichier : **6 576 scopes de plus sur 71 233, +9,2 %** ;
- pour les seuls modules externes (le cas où la mention n'a aucune autre cible) : environ **4 632, +6,5 %** ;
- autant de vecteurs d'embarquement si ces scopes sont embarqués comme les autres ; sans embarquement,
  seulement des nœuds et une arête `DEFINED_IN` chacun.
- Rust porte l'essentiel (5 370) ; c'est aussi le langage où la règle n'a pas encore été demandée (le
  ticket est né de JS/TS, 123 noms).

**À décider** (avec Lucie, puisque c'est au-delà de quelques pour cent) : `import_binding` pour tous les
langages, pour JS/TS seulement (123 scopes, +0,2 %), pour les seuls externes, ou garder la zone de
liaison du lot `require` (déjà sur master), dont 98 % des arêtes sont saines au juge (ticket de l'oracle).
