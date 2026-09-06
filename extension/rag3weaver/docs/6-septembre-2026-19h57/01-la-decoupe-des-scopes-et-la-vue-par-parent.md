# La découpe des scopes, et la vue par parent

**6 septembre 2026, 19h57.** Doc intermédiaire du chantier « chemin
d'indexation », point 1 du cahier des charges de la session moteur
([issue 04](../issues/6-septembre-2026/04-rapport-pour-la-session-principale.md)).
Référence : `e2e_mesure_ingestion_code` sur le moteur réglé — **73 s, 5 405
chunks, 1,75 Mo embarqués pour 1,15 Mo de source**.

Deux consignes de Lucie encadrent ce doc :

> attention à ce qu'on garde la généricité de nos schémas pour décrire cette
> organisation, pas qu'on la troque contre des trucs trop hardcodés

> regrouper automatiquement plusieurs scopes d'un même parent en vue sympa
> aussi, en les montrant entourés de la signature du parent, même si le
> parent sort pas direct de la recherche

## 1. Ce que la cartographie a établi

- **Un scope non-feuille embarque ses enfants.** Le `content` d'un `impl` ou
  d'un `mod` est la tranche d'octets de son corps, méthodes comprises
  (`codeparsers`, `get_node_text`) ; les méthodes sont aussi des scopes. Une
  méthode est donc découpée et embarquée deux ou trois fois, et l'éditer
  invalide son `impl` et son `mod` à l'incrémental.
- **Le texte embarqué n'est jamais assemblé.** `content`, `docstring`,
  `signature` sont découpés séparément ; seul `_text` part au modèle. Le
  vecteur d'une méthode est celui de son corps nu, sans son nom.
- **La découpe « sémantique » est celle de la prose** (`text-splitter`), sans
  notion de bloc. À 1 000 caractères, la plupart des fonctions tiennent en un
  chunk ; ce sont les conteneurs, déjà dupliqués, qui se fragmentent.
- **La navigation existe** : `search(relation=PARENT_OF|HAS_PARENT)` liste
  enfants ou parent ; le rendu regroupe déjà par `(fichier, parent_name)`.
  Il manque le cadre par la signature du parent quand il n'est pas un
  résultat, et le regroupement se fait sur un nom, pas sur l'arête.

## 2. Ce qu'on décide, et où vit chaque chose

| notion | où elle se décrit | pourquoi là |
|---|---|---|
| **le texte propre d'un scope** : sa déclaration et son corps **sans le corps de ses enfants**, chaque enfant remplacé par sa ligne de signature | l'**analyse** (`code.rs`, sur les empans d'octets de `codeparsers`) | c'est l'analyseur qui *produit* le contenu d'un scope, comme ragforge le fait dans le sien ; le catalogue ne sait pas ce qu'est du code et ne doit pas l'apprendre. Le schéma ne change pas : `content` reste le champ de contenu |
| **la découpe par lignes** : 30 lignes ou 1 500 caractères, recouvrement de 5 lignes seulement au-delà, fenêtre glissante | `ChunkingConfig` — nouvelle stratégie `ChunkStrategy::Lines`, avec `max_lines` et `overlap_lines` | générique : toute entité peut la déclarer ; `default_scope_chunking()` la choisit |
| **la hiérarchie** | les relations déclarées `PARENT_OF` / `HAS_PARENT` | elles existent, rien à ajouter |
| **relister les enfants d'un résultat** | `search(relation=PARENT_OF)` | existe, rien à ajouter |
| **la vue par parent** : les scopes d'un même parent rendus ensemble, encadrés par la signature du parent, que le parent soit ou non un résultat | `EntityConfig.group_by` : la relation qui mène au parent et le champ qui sert de cadre (`HAS_PARENT`, `signature`) ; le graphe de recherche va chercher les parents par cette relation, et le rendu regroupe par **arête**, plus par nom | générique : une entité de conversation regrouperait ses messages par fil, encadrés par le titre du fil, avec la même déclaration |

**Ce qu'on refuse** : un `if scope_type == "class"` dans le catalogue ou les
nœuds ; un chemin de rendu propre au code. Le seul endroit qui connaît le
code est l'analyseur, et il ne fait que produire des données.

**L'alternative écartée**, pour mémoire : soustraire les empans des enfants
dans un nœud du dataflow, à partir de champs d'empan déclarés dans le schéma.
Générique en apparence, fragile en pratique : `content` est dé-indenté par
l'analyseur, donc les offsets d'octets du fichier ne s'y appliquent plus. Le
jour où une autre entité hiérarchique en a besoin, la question se repose
avec un cas réel.

## 3. Attendu

| | aujourd'hui | attendu |
|---|---|---|
| chunks | 5 405 | ~1 800 |
| texte embarqué | 1,75 Mo (1,5×) | ~1,15 Mo (1×) |
| temps | 73 s | ~30 s, à mesurer |
| une méthode éditée | ré-embarque impl et mod | ré-embarque la méthode |

Et deux effets qui ne se mesurent pas en secondes : le vecteur d'une
fonction contient désormais son nom et sa signature ; un `impl` est trouvé
par ce qu'il déclare, pas par le corps de sa quarantième méthode.

## 4. Les étapes

| | quoi | comment on saura |
|---|---|---|
| D1 | le texte propre, dans `analyze_with` : par fichier, empans triés, chaque enfant direct remplacé par sa signature ; dé-indentation après | test de bibliothèque sur un `impl` à deux méthodes : le `content` de l'impl contient les deux signatures et aucun corps ; celui d'une méthode est inchangé |
| D2 | `ChunkStrategy::Lines` (`max_lines`, `overlap_lines`) dans `ChunkingConfig` et `Chunker` ; `default_scope_chunking()` → Lines 30 / 1 500 / 5 | tests du découpeur : un texte de 20 lignes = 1 chunk ; 70 lignes = 3 chunks avec 5 lignes partagées ; les offsets `core_*` et `start_*` tiennent |
| D3 | `EntityConfig.group_by { relation, frame_field }` ; `Scope` le déclare ; le gabarit `search` va chercher les parents par cette relation (port `parents`) ; le rendu regroupe par uuid de parent et écrit la signature du parent en cadre | e2e : deux méthodes d'un même impl trouvées seules → une section encadrée par `impl Catalog { … }` ; sans `group_by`, rendu inchangé |
| D4 | la mesure, et les suites de code (`e2e_code`, `e2e_symbol_search`, `e2e_highlight_long_text`) | chunks, Mo, secondes ; les suites vertes ou leurs attentes ajustées avec la raison |

## 5. Mesuré

`e2e_mesure_ingestion_code`, moteur réglé, carte TV, seul sur le poste :

| | avant | après D1 + D2 |
|---|---|---|
| chunks | 5 405 | **2 607** (2 173 de contenu, 434 de docstring) |
| texte embarqué | 1,75 Mo (1,5×) | **1,26 Mo (1,1×)** |
| temps | 73 s | **47 s** |
| occupation de la carte | 36 % en moyenne | idem — c'est le point 3 |

Le ÷3 attendu sur les chunks est un ÷2 : 540 scopes dépassent 30 lignes et
font plusieurs chunks, et la docstring reste embarquée à part (elle précède
la déclaration, donc elle est hors de l'empan). En route, une décision de
schéma de plus : **`signature` n'est plus un champ de contenu**, c'est la
première ligne du texte propre — 1 633 vecteurs de moins pour un texte que le
premier chunk porte déjà. Elle reste rendue et filtrable.

Suites vertes : `e2e_code` 23, `e2e_symbol_search` 12,
`e2e_highlight_long_text` 8, bibliothèque 986.

## 6. État

| étape | état |
|---|---|
| D1 | fait |
| D2 | fait |
| D3 | fait — `EntityConfig.group_by`, `GroupFrameNode` dans `search`, regroupement par arête au rendu, cadre `┌ …` ; le parent d'une méthode par `HAS_PARENT` est le scope nommé comme son impl (l'enum ou la struct), résolu par nom dans codeparsers |
| D4 | fait : `e2e_code` 24, `e2e_graph_tool` 4, `e2e_agent_loop` 8 (un nœud de plus dans la trace), bibliothèque 986 |

## 7. La suite du cahier des charges, le même soir

Après la découpe, les points 2 et 3 du cahier de la session moteur, et le
premier chiffre avec granite. Toujours `src/dataflow`, 30 fichiers, seul sur
la carte TV.

| étape | temps | embarquement | ce qui a changé |
|---|---|---|---|
| découpe (D1 + D2) | 47 s | 42 s | 2 607 chunks au lieu de 5 405 |
| **pipeline** (`embed_pipeline`, les six boucles) | 36 s | 31 s | la carte calcule le lot suivant pendant qu'on écrit le précédent ; 36 % → 94 % en pointe |
| lots par modèle (`budget_conseille`, puissances de deux, surface d'attention) | — | — | BGE-M3 à remesurer ; granite ci-dessous |
| **granite-107m** (384 dimensions, lots 256 × 512 bornés en surface) | **11,8 s** | 5,9 s | le modèle : 12 × 384 au lieu de 24 × 1 024 |

À 11,8 s pour 30 fichiers, un dépôt de 5 000 fichiers prend **~33 minutes** —
la cible de Lucie (30 min) est à portée, et la carte n'est plus qu'à 32 % :
le temps restant est l'analyse (tree-sitter), les insertions, les symboles
(2,7 s), pas le modèle.

Trouvé en route : le conseil du modèle pris tel quel (256 séquences de 512
jetons) a demandé un tampon de 2,6 Go à la carte — la matrice d'attention,
256 × 12 têtes × 512² × 4 octets. D'où la troisième borne des lots, en
surface d'attention (`LotBudget::max_area`).

Ce que la qualité dira : granite-107m contre BGE-M3 sur des requêtes de code
en français, c'est le banc de la session moteur. Lucie tranche sur les deux
chiffres.

**Reste, dans l'ordre** : la garde du modèle dans `_catalog_meta` (écrite,
test en attente de compilation), l'incrémental par hash à trois niveaux
(l'analyse re-parse tout aujourd'hui), la déduplication et les exclusions.
