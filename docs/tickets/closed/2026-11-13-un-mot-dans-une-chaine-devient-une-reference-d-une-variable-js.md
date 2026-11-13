# Un mot dans une chaîne devient une référence d'une variable ou d'une fonction JS

- **État** : corrigé (13 novembre 2026) — codeparsers b5c91e1 : l'apostrophe ouvre une chaîne en
  JavaScript et TypeScript dans le texte que lit `ensure_import_references_tracked` ; témoin
  `codeparsers/tests/chaines_hors_references_js.rs`, rouge sans le correctif.
- **Gravité** : réponse fausse (une arête vers un import que la ligne ne mentionne pas).
- **Atteignable en service** : oui, tout fichier JavaScript ou TypeScript dont une variable ou une
  fonction contient, dans une chaîne, le nom d'un import du fichier (`'path'`, `'.result-path'`).
- **Touche rag3weaver** : non (codeparsers : la lecture des références) — sauf le pointeur.
- **Famille** : produit

- **Pour** : session codeparsers

## Ce que c'est

Le rejugement des 415 arêtes `liaison` (oracle qui interroge le nom mentionné, 13 novembre ; ticket
`docs/codeparsers/tickets/2026-11-11-l-oracle-cherche-la-cible-au-lieu-du-nom-mentionne.md`) en rend 4
« muettes » ; les quatre nomment le mot **dans une chaîne** :

| mention | appelant | la ligne |
|---|---|---|
| `tools/wasm/build.mjs:14` | `ES_BUILD_CONFIG` (variable) | `external: ['fs', 'path', 'ws', …]` |
| `extension/lucivy/ld-lucivy/playground/test-playground.mjs:71` | `firstPath` (variable) | `page.locator('.result-path')` |
| `extension/lucivy/ld-lucivy/playground/test-playground.mjs:73` | `file_scope_07` (zone) | `new Error('Expected file path, got: ' + …)` |
| `experiments/mtga/scripts/collect.cjs:21` | `payload` (variable) | `source: 'mtga-reader'` |

Chacune devient une arête `CONSUMES` vers la zone de la ligne `const path = require("path")` (ou
`reader`), marquée `liaison` : la mention n'existe pas.

## Reproduit (codeparsers a730a9b, options de rag3weaver « fichier seul »)

```js
const path = require("path");

const ES_BUILD_CONFIG = {
  external: ['fs', 'path'],
};

async function lancer(page) {
  const firstPath = await page.locator('.result-path').first();
  if (firstPath.startsWith('doc')) throw new Error('Expected file path, got: ' + firstPath);
  return ES_BUILD_CONFIG;
}

module.exports = lancer;
```

La variable `ES_BUILD_CONFIG` reçoit une référence `path` (`kind = Import`) à la ligne 4, la fonction
`lancer` une à la ligne 8 (le `path` de `'.result-path'`, coupé au tiret) ; les deux deviennent
`CONSUMES … → file_scope_01` avec `external_binding = true`. La ligne 9 (`'Expected file path'`) ne
produit pas de référence dans ce petit cas, mais en produit une dans `test-playground.mjs` (une zone).

## Ce qu'on sait

- Le correctif du 11 octobre (codeparsers **1f5506f**, branche `chaines-hors-references`, **dans master**
  depuis) efface par l'arbre chaînes et commentaires (`mask_strings_and_comments`) pour les références
  lues **sur le texte** des zones de fichier, des conteneurs et des trous d'objet littéral. Il ne couvre
  pas le chemin qui donne ses références à une **variable** (`create_variable_scope`,
  `extract_global_variables`) ou à une **fonction** ici ; ce chemin est à lire (une lecture du texte du
  nœud, ou des dépendances, qui ne masque pas les chaînes).
- Le cas d'une zone (`test-playground.mjs:73`) montre que le masquage ne prend pas toujours : à lire
  aussi (la zone est dans un fichier à `await` de premier niveau).

## Ce qu'il faut pour le fermer

1. Le témoin rouge ci-dessus dans `codeparsers/tests/` (aucune référence `path` hors des lignes 1 et
   d'un usage réel ; aucune arête `liaison` depuis `ES_BUILD_CONFIG` ni `lancer`).
2. Le chemin trouvé et masqué comme les zones (chaînes et commentaires effacés, interpolations gardées).
3. Le rejeu des 415 liaisons : les 4 muettes disparaissent, rien d'autre ne bouge.

## La correction (13 novembre 2026)

**Le chemin** : `ensure_import_references_tracked` (`base_scope_extraction_parser.rs`) ajoute à un scope
une référence pour chaque nom importé du fichier qu'il trouve dans son texte, chaînes et commentaires
effacés par `without_comments_and_strings`. Ce nettoyage textuel ne fermait une apostrophe que pour un
caractère seul (`'x'`), la règle de Rust (pour ne pas prendre une durée de vie `'a` pour une chaîne) : une
chaîne JavaScript `'path'` restait du code, et `path` y était trouvé. Le même chemin donnait la zone de
`test-playground.mjs:73` (`'Expected file path, …'`).

**Le correctif** : `without_comments_and_strings(texte, apostrophe_chaine)` ; l'apostrophe ouvre une
chaîne pour JavaScript et TypeScript ; ailleurs, la règle du caractère seul reste (Rust, C, C++, C#, Go).
Python passait déjà (il a son propre chemin).

**Le témoin** : `codeparsers/tests/chaines_hors_references_js.rs` — `ES_BUILD_CONFIG` et `lancer` sans
référence `path`, sans arête `CONSUMES` vers la zone de liaison ; `path.join(a, b)` dans `joindre` reste
un usage ; le même pour Python. Rouge sans le correctif (`ES_BUILD_CONFIG` recevait `path` à la ligne 4),
vert avec ; la suite entière de codeparsers : 47 binaires, 288 passés, 0 échec.

**Cargo.lock** : inchangé (b5c91e1 n'ajoute aucune dépendance à a730a9b).
