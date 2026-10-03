# Vision — explorer les relations pendant la recherche

3 octobre 2026. Demande de Lucie : « on n'a pas beaucoup réfléchi à comment
explorer les relations : les relations les plus courtes entre les différents
résultats, donner plus de poids à ceux qui sont proches relationnellement,
trouver tous les appels d'un symbole et sa déclaration… produire une vision
pour ça ».

Ce document dit ce qui existe, ce qu'on pourrait vouloir, et dans quel ordre.
Rien ici n'est décidé ni codé.

## 1. Ce qui existe aujourd'hui

État lu sur master (`721a56610`).

**Côté rag3weaver : un seul saut à la fois, composable.**

- `FetchRelatedNode` suit **une** relation nommée, dans **un** sens, à **un**
  saut, depuis des résultats. Il ne lit pas les propriétés de l'arête.
- `RelatedResultsNode` fait des voisins des résultats à part entière : un
  voisin hérite du meilleur score de ses parents, sans récompense du nombre
  de chemins (choix du 19 septembre).
- Le verbe `follow` du langage de requête structuré enchaîne ces sauts, un par
  un, relation par relation. C'est ainsi que MTG va d'une mécanique aux cartes
  d'un deck (trois sauts écrits à la main, puis une intersection).
- Les gabarits `search.mmd` et `grep.mmd` offrent à l'agent un paramètre
  `relation`. Le backend de code d'exemple ne l'expose que par `grep`.

**Côté moteur : presque tout, et rien n'est branché.**

- Le Cypher du moteur sait les chemins de longueur variable (`*1..3`), le plus
  court chemin, tous les plus courts chemins, et le plus court chemin pondéré
  par une propriété d'arête.
- L'extension `algo` est dans le dépôt (PageRank, Louvain, composantes
  connexes, k-cœur) mais n'est pas bâtie sur ce poste.
- rag3weaver n'appelle aucun des deux.

**Côté schéma.**

- Une relation déclarée peut porter des propriétés, donc un poids. Rien ne les
  lit pendant une recherche.
- Dans le schéma de code, il n'y a pas d'arête « appelle » : `CONSUMES` mêle
  l'appel, l'usage d'un type et la lecture d'une constante. L'analyseur connaît
  le contexte de chaque usage, mais on le jette à l'ingestion.
- Trouver la déclaration et les usages d'un nom demande aujourd'hui deux
  appels d'outil à la main (chercher le `Symbol`, puis suivre `DEFINES` ou
  `MENTIONS` à l'envers). Aucun outil ne fait le trajet
  scope → symbole → scopes.

Une leçon déjà écrite (`vision_roadmap_09_2026/12`) : un outil de graphe
séparé, proposé à l'agent, a été appelé zéro fois. Les relations doivent venir
**dans** la recherche et son rendu, pas à côté.

## 2. Cinq capacités

Chacune est générique : elle parle d'entités et de relations déclarées, pas de
code. Les exemples prennent le code et MTG tour à tour.

### A. Les usages et la définition d'une chose

« Tous les appels de `begin_snapshot` et sa déclaration. » « Toutes les cartes
qui portent cette capacité, et sa fiche. »

La forme générique : un **pivot**. Deux entités se rejoignent par une
troisième qui sert de rendez-vous (le `Symbol` pour le code, la capacité pour
MTG). L'outil prend une chose, traverse le pivot, et rend d'un côté ce qui la
définit, de l'autre ce qui s'en sert, groupé et compté.

Ce qu'il faut :
- un nœud de pivot (deux sauts déclarés comme un seul geste) ;
- lire les propriétés de l'arête, pour dire **comment** on s'en sert ;
- pour le code, garder le contexte que l'analyseur donne déjà (appel, type,
  import, héritage) comme propriété de l'arête, plutôt que créer une relation
  par cas.

### B. Les chemins entre les résultats

« Comment ces cinq résultats tiennent-ils ensemble ? » La recherche rend des
choses éparses ; le plus court chemin entre elles est souvent l'explication
que l'agent cherchait (exemple imaginé) : `sync_source` → utilise → `finish_snapshot` → utilise →
`apply_snapshot_finish`.

La forme : après la recherche, prendre les k premiers résultats et demander au
moteur les plus courts chemins entre eux, bornés (trois ou quatre sauts), sur
une liste de relations permises. Le rendu ajoute une section « liens » sous
les résultats. Le moteur sait déjà le faire.

Deux garde-fous :
- **Les carrefours.** Un nom comme `new` ou un fichier comme `lib.rs` relie
  tout à tout ; un chemin qui y passe n'explique rien. Il faut pouvoir écarter
  les nœuds trop connectés, par un plafond de degré.
- **Le coût.** k résultats font k² paires. On borne k (cinq à huit) et la
  profondeur.

### C. Peser par la proximité

Trois variantes, de la plus simple à la plus riche.

1. **La cohésion.** Un résultat lié à d'autres résultats du même lot monte un
   peu. Quand trois des dix premiers s'appellent entre eux, c'est sans doute
   le bon endroit. Un saut suffit, tout se calcule sur le lot.
2. **L'ancre.** L'appelant donne un point de départ (le fichier qu'il édite,
   le deck qu'il construit) et les résultats proches de ce point montent, avec
   un poids qui décroît à chaque saut.
3. **La diffusion.** Le score se propage le long des arêtes depuis les
   meilleurs résultats (un PageRank personnalisé). Plus fin, plus cher, et
   plus dur à expliquer.

Où cela se branche : `FuseResultsNode` a déjà un rôle `boost`, qui module le
score fusionné sans entrer dans la fusion. La proximité y entre comme un
signal de plus, entre `fuse` et `rerank`, à côté de `weigh`. Le poids se règle
comme les autres depuis le pas C : appelant, graphe, entité, défaut.

Il faudra aussi un **poids par relation** : hériter de quelqu'un rapproche
plus que partager un fichier. Il se déclare avec la relation, dans la
configuration, pas dans le moteur.

### D. Le voisinage d'une chose

« Qu'est-ce qui casse si je change cette fonction ? » C'est la fermeture des
usages entrants, à deux ou trois sauts, avec un budget. « De quoi dépend-elle ? »
est la même question dans l'autre sens.

La forme : un nœud de voisinage (relations permises, sens, profondeur, budget
de nœuds), rendu en niveaux — touché directement, puis à deux sauts — avec les
comptes quand le budget coupe.

### E. La structure d'ensemble

Ce que le graphe dit sans qu'on le lui demande : les nœuds centraux, les
grappes. PageRank donne un champ « importance » qui peut peser dans toute
recherche ; Louvain donne des grappes qui font une carte du dépôt (ou des
familles de cartes).

C'est un calcul hors ligne, relancé après une synchronisation. L'extension
`algo` existe ; il faut la bâtir et décider où ranger ses résultats.

## 3. Ce que cela demande dessous

| Besoin | Sert à | État |
|---|---|---|
| Lire les propriétés d'arête dans `fetch_related` | A, C | à faire, petit |
| Sens « les deux » et plusieurs relations par appel | B, C, D | à faire, petit |
| Nœud de chemin (plus courts chemins du moteur) | B | à faire ; le moteur sait |
| Nœud de voisinage à profondeur bornée | D, C2 | à faire ; le moteur sait |
| Signal de proximité en rôle `boost` | C | à faire ; le point de branchement existe |
| Poids et plafond de degré par relation, déclarés | B, C, D | à concevoir |
| Contexte de l'usage gardé sur l'arête (code) | A | l'analyseur le donne, l'ingestion le jette |
| Extension `algo` bâtie et appelée | E | présente, non bâtie |

Trois fragilités connues pèsent sur tout cela :
- l'insertion des relations ralentit quand la base grossit (confié à la
  session cœur C++ le 3 octobre) ;
- un nom unique qui devient ambigu garde ses anciennes arêtes ; un graphe
  qu'on explore davantage rend ces arêtes fausses plus visibles ;
- les arêtes sortantes d'un scope gardé ne sont jamais nettoyées (au journal).

## 4. Ce que l'agent voit

Pas cinq outils de plus. La proposition :

- **`search`** garde `relation`, gagne `near` (l'ancre de C2) et une section
  « liens » dans son rendu (B) quand les résultats sont liés. La cohésion (C1)
  agit sans paramètre, par un poids par défaut faible.
- **`usages`** : un seul outil neuf pour A, parce que la question est
  fréquente et que sa réponse a une forme propre.
- **`neighborhood`** (D) : à décider après avoir vu un agent travailler ;
  peut-être un paramètre `depth` sur `search` suffit.
- E n'a pas d'outil : c'est un champ et une carte dans `schema`.

## 5. Ordre proposé

1. **A, les usages.** Le besoin le plus net d'un agent de code, le moins
   risqué, et il force les deux petites pièces (propriétés d'arête, pivot).
2. **C1 et B ensemble.** La cohésion et les liens entre résultats se
   calculent sur le même lot ; l'un pèse, l'autre explique.
3. **C2, l'ancre**, quand l'agent a un espace de travail qui dit où il est.
4. **D**, selon ce que l'agent demande vraiment.
5. **E**, plus tard.

La mesure : le banc de recherche n'a aucune question relationnelle. Avant de
régler un poids de proximité, il lui faut des questions dont la bonne réponse
dépend du graphe (« qui appelle X », « qu'est-ce qui relie X et Y »), sinon on
règle à l'aveugle.

## 6. Ce qui attend un choix de Lucie

1. L'ordre ci-dessus, ou autre chose d'abord.
2. La cohésion active par défaut (poids faible) ou seulement sur demande.
3. Pour le code : garder une seule relation `CONSUMES` avec le contexte en
   propriété (recommandé), ou créer une vraie arête « appelle ».
4. Un nom unique devenu ambigu : laisser les anciennes arêtes (aujourd'hui) ou
   préférer « une relation manquante vaut mieux qu'une fausse ».
