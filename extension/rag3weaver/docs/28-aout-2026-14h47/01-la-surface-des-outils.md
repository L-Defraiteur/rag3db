# La surface des outils : un de moins, deux paramètres de plus

*28 août 2026. Design, avant le code.*

## 1. Le fait qui déclenche tout

Trois agents, un modèle de 30 milliards de paramètres, quarante appels d'outils
sur notre propre code (`fil-search-repare.md`) :

| outil | appels |
|---|---|
| `read` | 22 |
| `grep` | 10 |
| `search` | 4 |
| `list` | 3 |
| **`search_expand`** | **0** |

Zéro. Pas une fois. Et ce n'est pas faute d'être utile : c'est le seul outil
qui rend le graphe de dépendances, la moitié de ce qui distingue notre moteur
d'un `ripgrep`.

L'hypothèse d'hier était « ils boudent `search` parce qu'il est borgne ». On
l'a réparé ce matin — hybride, provenance, docstrings, chemins relatifs — et
`search` est passé de **3 appels à 4**. La qualité de l'outil n'était pas le
problème.

**Le problème est le choix.** Face à `read` et `grep`, qu'un modèle entraîné
sur du code connaît par cœur, deux outils de recherche maison se partagent une
part déjà minuscule — et le second n'est jamais pris. Un agent qui hésite entre
deux outils proches en prend zéro.

## 2. Ce que la maquette d'origine faisait, et qu'on n'a pas reporté

`ragforge-core/src/tools/fs-tools.ts`, `grep_files` :

```
extract_hierarchy: bool   Extract dependency hierarchy for results.
analyze:           bool   Analyze matched files on-the-fly to extract scope
                          relationships (CONSUMES, CONSUMED_BY, INHERITS_FROM…).
                          Scopes are filtered to only those containing matched lines.
```

**Le graphe s'y greffe sur l'outil que le modèle utilise déjà.** L'agent fait
son `grep` — son réflexe — et lève un drapeau quand il veut les relations. Il
n'a jamais à choisir entre deux outils.

Et `analyze` va plus loin que ce qu'on a : il analyse **à la volée** les
fichiers touchés, même non indexés, et **ne garde que les scopes qui
contiennent les lignes trouvées**. C'est le doc 16 (« le monde est ouvert »)
appliqué au grep — et nous avons déjà la brique, `scopes_on_the_fly` dans
`code_tools.rs`.

Trois autres choses vues là-bas :

- **`search_files`** — un grep **flou**, à distance de Levenshtein, « quand tu
  ne connais pas l'orthographe exacte ». On a cette tolérance côté BM25, pas
  côté grep.
- **`change_directory`** — le `cd` n'est pas une idée neuve ; c'est une chose
  qu'on avait et qu'on n'a pas reportée.
- **`extract_dependency_hierarchy`** — qui prend *en entrée* un tableau de
  résultats de `grep_files`. La composition passe par les données, pas par un
  outil qui en contient un autre.

**Un point où on diverge délibérément.** `brain-tools.ts` dit : *« Use absolute
paths from tool results, not relative paths. »* On fait l'inverse depuis ce
matin — chemins relatifs au répertoire courant — parce qu'un agent sait
toujours où il est (c'est dans son invite) et que soixante caractères de
préfixe par ligne se paient en jetons à chaque tour.

## 3. La décision

### 3.1 Un seul outil de recherche, appelé `search`

`search_expand` **contient** déjà `search` — c'est le même graphe plus
`FetchRelatedNode` et `ComposeNode`. Sa valeur pour un agent tient donc à **un
paramètre**, pas à un outil.

```
search(target, query, limit, rerank, relation?, direction?, expand_limit?)
```

`relation` absente → ce qu'on a aujourd'hui. Présente → les voisins et l'arbre
ASCII.

Ce qui rend ça possible sans conditionnelle dans un graphe figé :
**`FetchRelatedNode` avec une relation vide est un passe-plat exact** — il rend
un port `children` vide et ne consulte même pas la connexion. C'est le
troisième usage du même motif aujourd'hui, après `RerankNode(candidates=0)` et
`BurnDevice::Rocm` sans la feature : *un graphe-outil n'a pas de `if`, c'est la
valeur neutre qui en tient lieu.*

Effet secondaire voulu : les relations qui comptent (`CONSUMES`,
`CONSUMED_BY`, `PARENT_OF`, `IMPLEMENTS`) deviennent visibles **dans la fiche
de l'outil qu'il utilise déjà**, au lieu d'être cachées derrière un outil
qu'il ne prend jamais. Et `DEFINED_IN` cesse d'encombrer : l'agent ne le
demandera que s'il le veut, puisque le `📍` le lui donne déjà.

**Le piège que la fusion naïve créait.** `search_expand` contient `search` sous
le type de nœud `SearchTool`. Si `search` devient le graphe étendu, il se
contient lui-même. La sortie n'est pas d'aplatir — ce serait perdre la
containment, un acquis d'architecture — mais de **séparer le gabarit de
l'outil** :

| fichier | `%% tool:` | rôle |
|---|---|---|
| `search_base.mmd` | `search_base` | lié, devient le type de nœud `SearchTool`, **jamais offert** |
| `search.mmd` | `search` | le contient, et c'est lui qu'un agent voit |

Un gabarit peut donc exister pour être **composé** sans être un outil. Fixé par
un test : `tools.get("search_base").is_none()` pendant que
`nodes.schema(SearchTool).is_some()`.

Coût honnête : `search_expand` disparaît de la surface. Un graphe qui le nomme
casse — il n'y en a que dans nos tests.

### 3.1 bis Le nom vit sur l'attachement, pas sur le gabarit

Ce qui précède force une distinction qu'on n'avait pas : **ce qu'un gabarit
*est* n'est pas ce qu'un agent en *voit***.

Première tentative, et elle était fausse : un `GraphTool::renamed()` qui clonait
l'outil entier pour changer une chaîne. Un second objet avec le même contenu —
dans un moteur de graphe, précisément la faute à ne pas commettre.

La bonne forme est celle d'une **arête** : l'outil est un nœud, l'attachement
d'un outil à un agent est une relation, et le nom d'affichage est une propriété
de cette relation.

```rust
GraphToolRegistry::attach(display, Arc<GraphTool>)
```

La clé du registre **est** le nom d'affichage ; l'`Arc` est partagé. Deux agents
adoptent le même gabarit sous deux noms sans qu'il existe deux fois.

Conséquence à ne pas rater : `graph_tool_defs_with` doit lire le nom depuis
l'**attachement**, pas depuis le gabarit. Sinon un outil adopté sous un autre
nom annonce le sien, et le modèle appelle un nom qui n'existe pas chez lui.

### 3.1 ter Le catalogue, et l'adoption

C'est ce que la distinction précédente rend possible, et c'est la suite
naturelle.

Un agent, **sur autorisation**, peut parcourir les gabarits disponibles dans un
catalogue — pas seulement ceux qu'on lui a mis dans les mains au démarrage. Et
pour s'en attacher un, il doit lui **donner un nom** : c'est son vocabulaire.

Trois choses en découlent, à écrire quand on y viendra :

- **Parcourir** est une lecture bornée, comme le reste : ce qu'un agent voit du
  catalogue relève de la même autorisation que ce qu'il voit du disque.
- **Attacher** est un acte, donc ça se trace. Le nom choisi apparaît dans la
  trace, et c'est celui-là qui a du sens pour relire ce qu'il a fait.
- **Le même gabarit sous deux noms** chez deux agents est légitime : chacun
  nomme selon son domaine. `attach` le supporte déjà.

Ce n'est pas fait. Ce qui est fait, c'est la seule chose qui le bloquait : le
nom ne vit plus dans le gabarit.

### 3.2 `grep` gagne `analyze` et `relation`

```
grep(pattern, extension?, context_lines?, analyze?, relation?)
```

- **`analyze`** — les scopes qui contiennent les lignes trouvées, avec leur
  nom, leur type et leur signature. À la volée : ça marche sur un fichier non
  indexé, comme chez eux. Brique existante : `scopes_on_the_fly`.
- **`relation`** — suivre une relation depuis ces scopes. Même paramètre, même
  sens et même liste que dans `search` : un agent l'apprend une fois.
- **`context_lines`** — les lignes autour de chaque trouvaille. Absent chez
  nous, présent chez eux, et c'est ce qui évite un `read` derrière chaque
  `grep`. Vu les 22 `read` pour 10 `grep`, c'est probablement le paramètre au
  meilleur rapport.

Tout est **éteint par défaut** et gratuit quand on ne le demande pas : sans
drapeau, `grep` rend exactement ce qu'il rend aujourd'hui.

### 3.2 bis Ce qu'un agent peut toucher, et ce qu'on garde de ce qu'il a lu

Deux besoins distincts, notés le 29 août 2026, à écrire quand on y viendra.

**Un trou connu, aujourd'hui.** `WorkingTree::list()` saute les
points-répertoires, donc `.vault/` n'est jamais indexé ni grepé. Mais
`WorkingTree::read()` ne vérifie que `check_relative` — chemin relatif, pas de
`..` — et `.vault/vertex-sa.json` satisfait les deux. **Un agent dont la racine
est le dépôt peut donc lire les identifiants.** La règle qui ferme ça est la
cohérence : *ce que la liste cache, la lecture ne le rend pas*. Un seul
prédicat, utilisé par `list`, `read` et `write` — si un jour on veut rendre
`.gitignore` lisible, on le change à un endroit et les trois suivent.

**L'approbation humaine.** Certaines actions doivent rester possibles, mais
seulement demandées. C'est le troisième étage, au-dessus de `RootPolicy` (ce
qu'on peut toucher sans rien demander) et de `WorkDomain` (ce qu'on voit).

**Et un classifieur qui parle à l'humain, pas à l'agent.** Au moment
d'approuver, être averti : *« cette lecture peut contenir un identifiant, ne
pas en conserver la trace »*. Deux exigences dans cette phrase :

- **Il se déduit, il ne devine pas.** Le `.gitignore` est la source : ce qu'un
  dépôt refuse de publier est ce qu'une trace ne doit pas garder. Passer un
  modèle sur tout et n'importe quoi serait lourd et flou ; une règle tirée d'un
  fichier que le projet tient déjà est gratuite et vérifiable.
- **Il agit sur la trace, pas sur la permission.** Il n'interdit rien —
  l'humain décide. Il dit ce que garder coûterait.

La question qui reste ouverte, et qui est amusante : **qui observe le
classifieur ?** Dans cette architecture, il est un graphe comme les autres, donc
son run est tracé comme les autres — c'est précisément ce à quoi sert la trace,
et c'est la boucle étrange du doc 01 sous un angle de plus.

### 3.3 Ce qu'on ne fait pas maintenant

- Le grep flou (`search_files`). Intéressant, pas urgent : la tolérance existe
  déjà côté BM25, et un agent qui cherche une orthographe incertaine a `search`.
- L'approbation humaine pour sortir des racines autorisées — elle attend le
  terminal.

## 4. Ce qui accompagne, et qui est déjà écrit

- **`Cwd`** — un chemin absolu, `~` va vraiment à `$HOME`, et la permission est
  celle de `RootPolicy`, qui canonise avant d'autoriser. `cd ~` ne ment pas :
  il refuse, **avec la liste**. Fait et testé ce matin.
- **`RootPolicy::describe()`** — pour qu'un agent puisse *demander* sa
  frontière au lieu de la découvrir en s'y cognant. Fait ; reste à l'exposer.
- **L'outil `cd`** — le service existe, rien ne l'expose encore.
- **L'outil qui dit où je suis / ce que je touche / ce que je vois** — trois
  notions séparées (`Cwd`, `RootPolicy`, `WorkDomain`), une seule réponse.

**Séparer, toujours.** Quatre notions s'appelaient « racine », et deux étaient
confondues dans le code. Les séparer a fait apparaître deux mensonges en une
heure — `cd ~` qui ramenait à la racine de la source, `cd ..` qui prétendait
qu'il n'y a rien au-dessus. Le coût d'un type de plus est un fichier ; celui
d'une notion floue est une famille de bugs qui ne font échouer aucun test.

## 5. Comment on saura que c'était juste

Le même montage, le même modèle, la même question : `e2e_conversation_a_plusieurs`.
Ce qu'on regardera dans l'artefact suivant, et qui doit bouger :

| mesure | aujourd'hui | attendu |
|---|---|---|
| appels à `search` | 4 / 40 | en hausse |
| arbres de dépendances produits | **0** | > 0 |
| `read` après un `grep` | 22 pour 10 | en baisse, si `context_lines` sert |

Et si rien ne bouge, ce sera un résultat aussi : ça voudra dire que le
problème n'est pas la surface des outils mais leur description — la seule
hypothèse qui resterait, et qui se teste en changeant **elle seule**.
