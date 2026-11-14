# search_code avant tout index ne balaie plus les fichiers

- **État** : ouvert (14 novembre 2026, relevé par embarquement en jouant `bindings/nodejs/test/essai.js`, ticket remis à recherche par c7)
- **Pour** : recherche
- **Famille** : produit
- **Gravité** : réponse vide en apparence. Avant le premier index, `search_code` rendait les fichiers
  qui contiennent les mots, par balayage. Il rend maintenant `results: []` et « index : jamais
  construit — lancez `index` avant de chercher » : un agent qui cherche avant d'indexer ne trouve rien.
- **Atteignable en service** : oui. Le backend de code du gabarit (`templates/backends/code/backend.json`)
  est atteint dès qu'il n'a jamais été indexé : épreuve du paquet, démo, premier usage d'un dépôt.
- **Touche rag3weaver** : oui (`src/dataflow/generic_search_nodes.rs`)

## Ce que c'est

**Le même backend, deux binaires.** C'est le gabarit de code avec `workspace: { index: "code",
commands: "off" }`, sans `run_command`, une base neuve, et la cible `Scope` :

| Binaire | Ce que rend `search_code` avant tout index |
|---|---|
| **`rag3weaver@next` 0.0.1-alpha.1** (npm), démo du 14 novembre au portable (`~/.cache/rag3db-locks/demo-nodejs/demo.log`, « search_code avant l'index ») | il **balaie** : « _index : jamais construit — réponse par balayage des fichiers (mots exacts seulement)_ », « Results: 2 », « balayage : 79 fichiers lus, 2 contiennent les mots » |
| **Binaire posé 012be4a58** (lib 0c41f9fe6), `test/essai.js` des gabarits de master `8e8e6a3b5`, sans variable de service | **pas de balayage** : `results: []`, `bm25Count: 0`, « index : jamais construit — lancez `index` avant de chercher » |

**L'épreuve** (`bindings/nodejs/test/essai.js`) attend qu'avant tout index, `search_code "bonjour"`
trouve `main.rs` ou `README.md` par balayage. Depuis le 14 novembre, c'est l'assertion « avant
l'index », marquée **rouge connue** avec ce ticket. Elle est séparée de l'assertion de
l'avertissement « pas de service », pour que l'une verdisse sans l'autre.

## La piste (lue, pas prouvée)

`adaptive_search_decision(state, has_source && !projected)` (`generic_search_nodes.rs`) : jamais
construit et `has_file_source` vrai, la recherche balaie ; faux, elle dit « lancez `index` ».

La condition `&& !projected` est arrivée par **`c67ca39c6`** (11 novembre 2026, « Isoler les files
par cellule… »). `projected` est vrai quand la cible porte une `occurrence_projection`. Si la `Scope`
du schéma de code en porte une, c'est la cause. Ce n'est pas vérifié : la recherche départage.

## Ce qu'il faut pour le fermer

- Départager : décision voulue (une projection d'occurrences ne se balaie pas sans masque de
  fichiers) ou régression pour le code.
- Voulue : `essai.js` et la démo disent la nouvelle réponse, et l'assertion « avant l'index » est
  réécrite. Régression : le balayage revient pour la `Scope` du code, rouge d'abord, et la rouge
  connue d'`essai.js` verdit.
