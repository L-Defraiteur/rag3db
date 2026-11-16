# Des homonymes sur une même ligne partagent un seul DEFINED_IN

- **État** : ouvert (16 novembre 2026)
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
