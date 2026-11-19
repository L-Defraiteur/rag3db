# Le namespace devant la classe : la mesure, et une règle à décider (2.12)

Session everything declarative, 19 novembre 2026, 18 h 30. Ticket
`2026-11-16-un-scope-de-namespace-passe-devant-une-classe-du-meme-nom.md`. Cette page mesure, puis propose une règle.
**Pas de code avant le mot de c7.**

## La mesure

**Base** : une copie reflink de `banc-parite/ecr-base-ecrivain-neuf-5-lot-reindex-20261115` à luciepc (forme
`Generations`), avec son manifeste repointé sur la copie. L'original n'a pas été ouvert.

**Sonde** : `sonde_parite_search_code::releve_le_namespace_devant_la_classe`, branche `namespace-probe` (649b8f678f), sous
la lib 6e2476d41. Elle lance `search_code` avec la requête « rag3db », filtrée sur `src/include/`, et relève les 20
premiers résultats sous trois réglages : les signaux par défaut, `["vector"]` et `["bm25"]`. Le relevé complet est à
luciepc, dans `~/.cache/rag3weaver-build/ed-namespace-copie/releve-namespace.json`.

| réglage | namespaces dans les 20 | `rag3db_database` | les rangs 1 à 10 |
|---|---|---|---|
| BM25 | 18 | absente | deux enums `rag3db_state` (43 car.) et `rag3db_data_type_id`, puis huit `namespace rag3db {` |
| défaut | 18 | absente | **la même liste que BM25, au rang près** |
| dense | 6 (anonymes) | absente | des modules `file_scope_*`, des namespaces anonymes de 47 à 51 caractères, une méthode |

**Le contenu d'un namespace indexé** : « `namespace rag3db {\nnamespace common { …\n}` », soit 41 à 61 caractères.
Le scope ne porte presque que son nom : ses enfants sont repliés. Sa raison au classement est `bm25_exact`, à 8,29,
la correspondance exacte du nom. Le premier résultat, un enum, est à 10,14.

## Laquelle des trois causes pèse

1. **Le titre, égal à la requête, et son boost** : **oui, au premier chef.** Les 18 namespaces arrivent tous par
   `bm25_exact`. Leur score est le même (8,29), à la longueur près (8,17 à 61 caractères) : c'est la correspondance du
   nom qui les porte.
2. **Un texte très court** : **oui, en second.**
   - En BM25, la normalisation par la longueur classe 41 caractères devant 61, à nom égal.
   - En dense, les namespaces anonymes de 47 à 51 caractères sont parmi les plus proches : un texte qui ne dit presque
     rien ressemble à tout.
3. **Un scope conteneur embarqué seul** : **c'est la condition des deux autres.** Un conteneur replié n'a ni texte ni
   sens propre : il a un nom, qui est aussi celui de son dépôt et de ses classes.

**Un second constat, hors du ticket** : avec les signaux par défaut, cette cible ne fusionne pas le dense. La liste est
celle de BM25, au rang près. Le dense ne peut donc pas rattraper la classe. Je n'y touche pas ici ; c'est à confirmer
par la configuration de la cible (`default_signals`) avant d'en faire un ticket.

## La règle proposée

**Un scope conteneur replié ne concourt pas seul.** Un scope de type `namespace` ou `module`, dont le contenu replié
tient en moins de 80 caractères, n'entre ni dans l'index plein texte ni dans le dense comme candidat. Il reste
partout ailleurs :
- dans le graphe : `PARENT_OF`, `DEFINED_IN` ;
- dans `outline` et `usages` ;
- dans le plein texte de ses enfants, par leur `parent_name`.

La condition se mesure sur le texte : 80 caractères couvrent les 41 à 61 relevés, et un namespace qui porte du code à
lui le dépasse. Elle ne joue pas sur le nom : un namespace qui porte des fonctions libres garde son contenu, et
concourt.

**Plus étroite, à choisir si c7 préfère** : ne retirer que le boost du titre (`bm25_exact`) aux conteneurs repliés. Le
texte court garderait alors son avantage de longueur. Je ne la recommande pas : en dense, les namespaces anonymes
resteraient en tête.

**Le témoin, rouge d'abord, si la règle est prise** :
- deux scopes, un `namespace X {` replié et une classe `X_database` ;
- la requête « X » : la classe passe devant, et le namespace n'est plus candidat ;
- puis la parité du banc rejouée sur la copie, avec les écarts nommés.

## Ce que je demande à c7

1. La règle large (le conteneur replié ne concourt pas), l'étroite (pas de boost du titre), ou aucune : le comportement
   est alors accepté et le ticket se ferme par un refus écrit.
2. Le seuil : 80 caractères, mesuré sur ce dépôt C++. Ou un réglage du manifeste, avec 80 par défaut.
