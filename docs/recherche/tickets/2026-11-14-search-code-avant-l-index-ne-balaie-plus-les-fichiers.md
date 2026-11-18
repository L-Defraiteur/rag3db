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

## Le départage (recherche, 18 novembre 2026)

**La piste est la bonne.** La condition `has_source && !projected` vient de c67ca39c6 (11 novembre, « Isoler
les files par cellule… »). Elle est **voulue** : son témoin `projected_search_never_falls_back_to_unmasked_files`
(`dataflow/generic_search_nodes.rs`) dit « auto must preserve the occurrence admission mask even for an empty
index ». Un balayage des fichiers ignore le masque d'admission des occurrences, et donc la vue d'un appel
(2b-C).

**Ce que ça coûte.** Depuis les versions de scope, la cible `Scope` du code porte une `occurrence_projection`
sur toute base neuve. La règle supprime donc le balayage avant index pour **tout** backend de code, pas
seulement pour une vue demandée. C'est ce que la démo et `bindings/nodejs/test/essai.js` voient.

**La proposition, à trancher par Lucie, portée par c7** :
- **Quand rien n'est encore indexé** (`text: never`) **et qu'aucune `view` n'est demandée**, le masque
  d'admission n'existe pas encore : la seule vue possible est le worktree du backend, et ses fichiers sont
  exactement ce que le balayage lit. Le balayage peut alors revenir, en le disant, comme avant ;
- **sous une `view` explicite**, ou dès qu'un index existe, la règle de c67ca39c6 reste : pas de repli sur des
  fichiers non masqués ;
- les témoins : `projected_search_never_falls_back_to_unmasked_files` garde son cas sous vue et son refus du
  mode `scan`. Un témoin neuf, rouge d'abord, attend : un backend de code neuf, sans index et sans vue,
  `search_code` trouve un mot d'un fichier par balayage. La rouge connue d'`essai.js` verdit alors.

**Sinon**, si Lucie garde la règle telle quelle : `essai.js` et la démo disent la réponse d'aujourd'hui
(« jamais construit — lancez `index` avant de chercher »), avec embarquement.
