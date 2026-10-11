# Des receveurs restent sans type

- **État** : en cours — chantier B2d de la session codeparsers (11 oct. 2026), pas à pas
- **Gravité** : réponse fausse (une arête « nom » vers une méthode homonyme)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (usages : « par le nom » ; Liens et impact les taisent)

## Ce que c'est

Après les paliers A, 2 et 3 (codeparsers 4abafd2), un appel sur un receveur
dont le type ne se lit pas est relié par le seul nom. `tests/sonde_racines.rs`
(src/ et tests/ de rag3weaver, avant le palier 3) : 53 318 appels qualifiés
sans type lu, dont 39 % d'une chaîne sur une variable non typée, 28 % d'une
variable seule non typée, 11 % d'une expression (parenthèse, `as`, littéral).
`tests/sonde_marques.rs` : 1 949 des 3 026 receveurs « nom » visent un nom de
méthode std (estimation par une liste) — presque tous faux.

Ce qui est lu aujourd'hui : annotation, paramètre, constructeur (enveloppes
comprises), chaîne std à retour certain (verrous, unwrap, itérateurs), champ
et retour du même fichier pelés par leur chaîne, motifs `Some` / `Ok`.

## Ce qui manque

- Le type des éléments : variable de boucle (`for x in v` avec `v: Vec<T>`),
  paramètre de fermeture (`v.iter().map(|x| …)`), autres motifs (tuples,
  structs). L'itérateur devrait garder son type d'élément (`Iterator<T>`).
- Un champ ou un retour déclaré dans un autre fichier : la résolution « fichier
  seul » de rag3weaver ne le lit pas (les tables du résolveur entre fichiers
  n'ont que des noms de base).
- Une méthode du projet dans une chaîne (`n.enfants().iter()`) : son type de
  retour, en différé, comme pour une fonction.

## Fait le 11 octobre 2026 (B2d)

- La variable de boucle : `for x in v` (`&v`, `v.iter()`, `iter_mut`,
  `into_iter` sur Vec, VecDeque, HashSet, tranche…) a le type de l'élément ;
  `vec![T, …]` et une struct unité (`let m = Marteau;`) nomment leur type
  (codeparsers `1fc380a`).
- Un appel par chemin sur un type (`Catalog::Get(…)`) a ce type pour
  receveur (codeparsers `f91fc58`).
- Sonde rag3db/src : CONSUMES « nom » 8 720 → 7 215, « type » 3 377 → 5 517.
- `banc_couverture --receveurs` range chaque méthode non résolue par
  l'origine de son receveur : le prochain pas se choisit là (retours
  d'appel, paramètres de fermeture, indexation).

Reste : les paramètres de fermeture, les retours d'une méthode du projet
dans une chaîne, l'élément d'un itérateur (`Iterator<T>`), C++ `for (auto&
x : v)`. Et un choix pour l'orchestration : un appel sur une locale qui
reste non typée est écarté en silence (règle d'origine du crate) — le
rendre visible, non attribué, changerait ce qu'affiche `usages`.

## Pour le fermer

Ce n'est pas un défaut à fermer d'un coup : chaque pas se mesure à la sonde des
marques (part de « nom ») et au banc des relations, et ne garde que des règles
à sémantique certaine (une règle fausse fait perdre une arête, jamais en
inventer).
