# Un `require` relatif s'arrête à la liaison au lieu de suivre le module

- **État** : ouvert (8 novembre 2026) — relevé par la mesure des locales JavaScript (8 faux sur 324
  arêtes, 4 mentions) ; la règle est à écrire.
- **Gravité** : réponse fausse (un usage relié à la ligne du `require`, pas à la définition).
- **Atteignable en service** : oui, tout dépôt CommonJS dont un fichier lie une classe d'un autre fichier
  de l'arbre par `const X = require("./x.js")`.
- **Touche rag3weaver** : non (codeparsers : les imports et la résolution) — sauf le pointeur.
- **Famille** : produit

## Ce que c'est

`tools/wasm/src_js/connection.js:9` lie `const PreparedStatement = require("./prepared_statement.js")`
et l'emploie ligne 139. Notre arête CONSUMES (marque « fichier ») va à la ligne 9 du même fichier ;
typescript-language-server rend la classe, `tools/wasm/src_js/prepared_statement.js:10`. Un `import`
ES se suit jusqu'à son module ; un `require` relatif reste une variable du fichier.

Les 4 mentions (juge strict, arbre a72acfeef, `locales-mesure/verdicts-B.tsv` à luciepc) :

- `PreparedStatement`, `tools/wasm/src_js/connection.js:139` → `prepared_statement.js:10` ;
- `PreparedStatement`, `tools/wasm/src_js/sync/connection.js:110` → `sync/prepared_statement.js:10` ;
- `PreparedStatement`, `tools/nodejs_api/src_js/connection.js:236` → `prepared_statement.js:5` ;
- `QueryResult`, `tools/wasm/src_js/connection.js:158` → `query_result.js:8`.

Une cinquième de la même forme, `dispatcher` (`tools/wasm/src_js/index.js:25` → `dispatcher.js:64`).

## Deux faux voisins, notés ici

- `var entries = []` (`opfs-bridge.js:90`) : le serveur rend `lib.dom.asynciterable.d.ts` pour la
  mention de la ligne 91, sans doute un `.entries()` d'itérateur lu comme la variable.
- `m.charIdx` (`playground/test_multibyte.mjs:41`, un accès de champ dans un gabarit de chaîne) relié à
  la globale `let charIdx` de la ligne 13 : un champ ne devrait pas se relier à une variable.

## Ce qu'il faut pour le fermer

1. Un `const X = require("<chemin relatif>")` se traite comme `import X from "<chemin>"` : la mention de
   `X` va à l'export du module quand il est dans l'arbre ; sinon, la liaison reste la cible.
2. Un témoin à deux fichiers (`connection.js`, `prepared_statement.js`).
3. L'oracle sur les 4 mentions.
