# Des homonymes sur une même ligne partagent un seul DEFINED_IN

- **État** : fermé (16 novembre 2026) — codeparsers `d754ee5` et rag3weaver ; voir « La correction »
- **Famille** : produit
- **Gravité** : réponse incomplète (un scope sans son fichier : `usages` et le plan du fichier le taisent)
- **Atteignable en service** : oui, sur la base du dépôt (63 scopes sur 77 248)
- **Touche rag3weaver** : oui (les clés stables des scopes, `src/code.rs`) ; codeparsers sépare déjà ses uuid par l'octet
- **Pour** : codeparsers
- **Signalé par** : banc, mesure de la pile de la résolution d'arêtes (16 novembre 2026)

## Ce que c'est

Après la pile de la résolution d'arêtes (master 41144f8d22), une réindexation à neuf de la base du dépôt
laisse **63 Scope sans DEFINED_IN** (3 814 avant). Leur fichier a sa rangée File : c'est l'arête qui manque.
Trois classes, toutes des homonymes sur une même ligne :

- Python, 27 : 25 lambdas d'une ligne (`generator_expression`, `list_comprehension`) et 2 variables `s` ;
- JavaScript, 35 : le bundle minifié `extension/rag3weaver/templates/ide/app/app.js`, lignes 8, 74 et 82
  (des variables d'une lettre sous `useState`, `useMemo`…, des méthodes `parse`, `text`, `link`…) ;
- C++, 1 : une lambda de `third_party/spdlog/spdlog/details/thread_pool-inl.h:42`.

Le relevé (chemin, nom, genre, langage, lignes, parent) est à luciepc :
`~/.cache/rag3weaver-build/banc-parite/sorties/codeparsers-stack/scopes-without-defined-in.json`, la
requête dans `~/.cache/rag3weaver-build/codeparsers-63.cypher`.

## La piste

Codeparsers distingue deux homonymes par leur position (`uuid_par_position`, 15 novembre). La clé stable
de rag3weaver (`stable_scope_keys`) les confond sans doute encore quand ils partagent nom, genre et ligne :
un seul DEFINED_IN pour deux scopes.

## Le bundle minifié ne s'indexe pas

c7, 16 novembre 2026 : `app.js` n'a pas sa place dans l'index. Le manifeste de Lucie l'exclut depuis le 15 au
soir (`workspace.index_exclude {minified_line_max_bytes: 5000, code_file_max_bytes: 1048576}`) ; la base
mesurée par le banc ne portait pas cette exclusion. Avec elle, les 35 homonymes JavaScript partent ; restent
les **28** (Python et la lambda de spdlog), qui sont ce ticket.

## Pour le fermer

Un témoin rouge (deux lambdas sur une ligne d'un fichier Python), la clé qui les distingue, puis les 28
recomptés par le banc, sous le manifeste de Lucie.

## La correction (16 novembre 2026)

La piste était juste, et il y avait une seconde cause, en amont. Deux générateurs d'une même ligne Python,
au niveau du module (là où aucun scope nommé ne les replie, `fold_lambdas`), ont même nom, même genre, mêmes
lignes, **et mêmes octets de scope** (ceux de la ligne). Ils ne diffèrent que par les octets de leur nœud.

- **codeparsers** (`d754ee5`) : la table des positions des uuid (`uuid_par_position`, 15 novembre) se clait
  par les octets du scope ; les deux recevaient le même uuid. Elle se clé maintenant par les octets du nœud
  (`node_position`), et `ScopeMappingEntry` les porte (`start_byte`, `end_byte`).
- **rag3weaver** : la clé de position qui rattache un uuid à sa clé stable (`stable_scope_keys`) prend aussi
  les octets du nœud.

Témoin : `tests/e2e_usages.rs`, `two_homonyms_on_one_line_both_have_their_file`
(`print(sum(1 for x in A if x), sum(1 for y in A if not y))`), vu rouge puis vert.

La sonde (`tests/sonde_marque_nom.rs`, qui compte maintenant les scopes sans DEFINED_IN ; corpus figé de
rag3db-garde ; avant = master, codeparsers `6291939`) :

| corpus | scopes | sans DEFINED_IN avant | après |
|---|---|---|---|
| experiments/ et extension/rag3weaver/docs (Python) | 1 295 | 26 | 0 |
| third_party/spdlog | 3 040 | 1 | 0 |
| rag3weaver/src | 16 222 | 0 | 0 |
| src/ du moteur | 19 057 | 0 | 0 |

Le nombre de scopes ne change pas, et aucun couple CONSUMES ne naît ni ne part, sur aucun des quatre.

## Confirmé sur une vraie base, 18 novembre 2026

La mesure du banc sous le manifeste de Lucie (`docs/banc/18-11-2026/02-la-pile-de-codeparsers-sous-le-manifeste-de-lucie.md`,
698efc4f3a ; arbre 1de1f86d88, index neuf par le master du 18, lib 3d706f709) retrouve reliées par leur fichier les
27 lambdas Python et la lambda de spdlog que l'index du 16 laissait sans `DEFINED_IN`.

