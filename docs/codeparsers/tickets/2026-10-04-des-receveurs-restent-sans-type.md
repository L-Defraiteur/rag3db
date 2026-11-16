# Des receveurs restent sans type

- **État** : en cours — chantier B2d de la session codeparsers (11 oct. 2026), pas à pas
- **Gravité** : réponse fausse (une arête « nom » vers une méthode homonyme)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (usages : « par le nom » ; Liens et impact les taisent)
- **Famille** : produit

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

## Le prix de la marque « nom » (15 novembre 2026)

Depuis codeparsers `fad1b6f`, un appel de méthode **standard** sur une locale sans type lu ne se relie
plus par le nom entre fichiers ([la marque « nom »](../../tickets/closed/2026-11-07-la-marque-nom-relie-un-appel-de-methode-standard-a-un-homonyme.md)).
Une méthode du projet homonyme d'une méthode std y perd son lien : une vingtaine sur rag3weaver/src
(`catalog.drain()`, `files.append(…)`, `outil.bind(…)`). Lire le type de ces locales les rend : c'est
la prochaine mesure de ce ticket. La sonde `tests/sonde_marque_nom.rs` en donne les TSV.

## Fait le 15 novembre 2026 : les chaînes de méthodes du projet

Codeparsers `65b1a93`, rag3db `cee48f2d9`. Une locale liée à une chaîne qui passe par des **méthodes du
projet**, sur une racine de type connu, lit son type :

- `self.teammate_files()?` dans un `impl Host` : la racine est le type englobant ;
- `ctx.cancellation()` avec `ctx: &Ctx` : la racine est un paramètre typé ;
- `GraphTool::from_mermaid(t).map_err(…)?` ou `.unwrap()`, `Self::f()` : la racine est un chemin sur un type.

`DeferredType::OfType { ty, peel }` le dit. Le pelage (`receveur::peel_complet`) lit un maillon que la
table std ignore par le retour écrit de la méthode du type courant (`Self` y vaut le type) ; la table std
apprend `map_err`, `or_else`, `ok_or`, `ok_or_else` et `ok`. Le rendez-vous de rag3weaver charge
`(type, méthode) → retour` comme les champs. `OfType` est **une lecture au mieux** : non lu, l'appel suit
la règle d'avant, sans devenir un receveur non typé. Deux gardes : une méthode dont le retour nomme un de
ses paramètres de gabarit (`fn with_backend<R>(…) -> Result<R, _>`) ne type rien (les fonctions Rust
relèvent désormais leurs `generic_parameters`) ; un chemin de module (`facts::digest`) ne prend pas le
type d'une locale homonyme.

Témoins : `tests/e2e_usages.rs` (`une_locale_liee_a_*_lit_son_type`, `un_retour_generique_ne_type_pas_la_chaine`,
`un_chemin_de_module_ne_prend_pas_le_type_d_une_locale_homonyme`), vus rouges puis verts ;
`codeparsers/tests/chaines_du_projet.rs`.

La sonde (`tests/sonde_marque_nom.rs`, corpus figé de rag3db-garde 343ead799 ; avant = le lot de la marque
« nom », codeparsers `fad1b6f`) :

| corpus | marque | avant | après |
|---|---|---|---|
| rag3weaver/src | type | 7 116 | 9 070 |
| | nom | 10 114 | 9 048 |
| | fichier | 18 934 | 18 954 |
| | rendez-vous non typés | 9 918 | 9 476 |
| src/ du moteur | type | 10 214 | 10 345 |
| | nom | 2 705 | 2 654 |

Par couple (mentionneur, cible), sur rag3weaver/src : 864 « nom » deviennent « type », 1 071 couples
naissent (1 049 « type »), 178 partent. Ceux qui partent sont presque tous des méthodes std reliées à tort à
un homonyme : `collect` 123 (vers un `collect` d'openai_llm.rs), `join` 16, `keys` 9, `position` 6. Trois
`to_mermaid` changent de cible pour la bonne (`GraphTool::to_mermaid`). Six sont des agrégats au niveau de
la classe (`Catalog → admet`), par un paramètre de fermeture. Sur le moteur, un seul couple part
(`isPrimary`, un accès de champ).

**Le prix de la marque « nom » est rendu en partie** : `files.append` (2), les `c.wait_timeout` des tests
(12), `back.description()`.

## Fait le 16 novembre 2026 : le motif de tuple et la locale redéclarée

Codeparsers `6291939`.

- **Le motif de tuple** : dans `let (mut catalog, _store) = make_pair();`, chaque nom prend le différé de la
  valeur, suivi d'un pas `#i` qui lit l'élément de rang `i` du tuple écrit (`(Catalog, Store)` → `Catalog`).
- **Les liaisons différées sont situées** (`DeferredBindings`) : une liaison vaut de la fin de son `let` à la
  fin de son bloc, et une référence lit celle qui est visible là où elle est. `let outil = …?; let outil =
  outil.bind()?;` avait deux liaisons d'un même nom, qui s'annulaient ; chacune vaut maintenant à sa place.

Témoins : `a_tuple_pattern_local_reads_its_element_type` et
`a_shadowed_local_reads_the_binding_visible_where_it_is_used` (`tests/e2e_usages.rs`, vus rouges puis
verts) ; `codeparsers/tests/tuple_and_shadowed_locals.rs`.

La sonde (`tests/sonde_marque_nom.rs`, corpus figé de rag3db-garde ; avant = master, codeparsers `edfc72d`) :

| corpus | « type » avant | après | « nom » avant | après |
|---|---|---|---|---|
| rag3weaver/src | 9 070 | 9 572 | 9 059 | 8 736 |
| src/ du moteur | 10 912 | 10 912 | 2 653 | 2 653 |

Sur rag3weaver/src, par couple : 245 naissent (228 « type »), 278 passent de « nom » à « type », 47 partent.
Ceux qui partent : ~25 méthodes std reliées à tort (`collect` 14, `join` 5, `starts_with`), et **une
vingtaine de liens justes**, presque tous de `daemon_host/teammate_tests.rs` (`host.hosted`,
`host.with_backend`, `host.teammate_files`, `host.run_teammate`…).

**Ces vingt-là, et pourquoi on les laisse** (décision de c7, 16 novembre) : `let host = hote(…).with_runs_root(…)`
puis, plus loin, `let host = host.with_chat_model(…)`. Avant, les deux `host` s'annulaient : pas de différé, et
le lien par le nom. Maintenant la première liaison porte `ReturnOf hote` ; or `hote` est défini quatre fois,
avec quatre types de retour (`mcp.rs`, `worktrees.rs`, `fichier_nodes.rs`, `chat_qui_code.rs`) : le différé ne
se lit pas, et la règle du 11 octobre (une locale à différé non lu est visible, jamais reliée) s'applique.
On ne revient pas sur cette décision de Lucie.

## Reste

- **`ReturnOf` d'une fonction définie plusieurs fois** : quand les définisseurs ont des types de retour
  différents, le `hote` que le fichier importe désigne le bon. Résoudre par l'import rendrait les vingt liens
  de `teammate_tests.rs` sans lever l'abstention.
- Les paramètres de fermeture, l'élément d'un itérateur (`Iterator<T>`), C++ `for (auto& x : v)`.
