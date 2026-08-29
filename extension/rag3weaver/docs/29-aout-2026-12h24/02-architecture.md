# 02 — L'architecture aujourd'hui

*29 août 2026. Ce qui a bougé depuis le doc 12 du 27, et ce qui tient.*

## 1. Les trois couches

**Le catalogue** (`src/catalog.rs`) — le schéma déclaré, l'ingestion, la
recherche. Il ne sait rien des agents.

**Le dataflow** (`src/dataflow/`) — des nœuds, des ports, un ordonnanceur. Un
graphe est une donnée : sérialisable, empreinte BLAKE3, `to_mermaid`.

**L'agent** (`src/agent.rs`) — une boucle qui appelle un modèle, exécute des
outils, et trace. Un outil **est** un graphe.

## 2. Les quatre notions qu'on appelait « racine »

Séparées cette semaine, et chacune répond à une seule question. C'est la
discipline la plus rentable de la période : les fondre en cachait deux
mensonges.

| notion | la question | où |
|---|---|---|
| `RootPolicy` | qu'ai-je le droit de **toucher** | `code_tools.rs` |
| `WorkDomain` | qu'est-ce que je **vois** dans l'index | `work_domain.rs` |
| `Cwd` | **où** je me tiens | `code_tools.rs` |
| `PathLens` | par rapport à quoi j'**écris** un chemin | `dataflow/render_nodes.rs` |

Et une cinquième, nommée mais pas écrite : **`WorkPalette`** — qu'ai-je **sous
la main**. Le catalogue est la boutique, la palette est ce qui est posé sur la
table. `GraphToolRegistry::attach` en est déjà l'acte.

`Cwd` est **absolu**, comme dans un vrai terminal. `~` va à `$HOME`, et
`RootPolicy` décide — il canonise **avant** d'autoriser, donc un détour qui
retombe dans le permis est permis, et un refus porte la liste.

## 3. La recherche

Un seul outil, `search`, avec sept paramètres. Le graphe :

```
source → bm25   ↘
source → vector → fuse → rerank → resolve → fetch → compose → render
```

- **hybride** — jusqu'au 27 août ce graphe n'avait qu'un nœud BM25 : le moteur
  portait un vrai embedder que l'outil n'appelait jamais.
- **la provenance survit à la fusion** — un résultat sort en disant
  `bm25`, `vector` ou `bm25+vector`, pas le nom du nœud qui les a mêlés.
- **les poids penchent vers le plein texte** (0,6 / 0,4) contre le défaut de
  `FusionConfig` : ici on cherche souvent un identifiant, et une correspondance
  exacte doit gagner.
- **`relation` est facultative** — donnée, elle rend l'arbre des voisins.
  `FetchRelatedNode` sans relation est un passe-plat exact.

**Le motif de la valeur neutre**, utilisé quatre fois cette semaine : *un
graphe-outil n'a pas de conditionnelle, c'est la valeur neutre qui en tient
lieu.* `RerankNode(candidates=0)`, `FetchRelatedNode(relation="")`,
`VectorSearchNode` sur une cible sans vecteurs, `BurnDevice::Rocm` sans la
feature.

## 4. Gabarit ≠ outil

```
templates/tools/search_base.mmd   %% tool: search_base   → type de nœud SearchTool, jamais offert
templates/tools/search.mmd        %% tool: search        → le contient, et c'est lui qu'on offre
```

Un gabarit peut exister pour être **composé** sans être un outil. Et **le nom
vit sur l'attachement** : `GraphToolRegistry::attach(display, Arc<GraphTool>)`
— la clé du registre est le nom d'affichage, l'`Arc` est partagé. Un gabarit
n'est pas cloné pour être renommé ; c'est une propriété de l'arête.

## 5. Le rendu est un gabarit

`src/dataflow/render_nodes.rs` construit une **vue** (`ResultsView`) ;
`templates/render/*.md.jinja` décide de la forme. Rust dit *quoi* montrer, le
gabarit dit *comment*.

La forme par défaut vient de la maquette d'origine : `📍` sur sa ligne et
copiable dans `read`, `🔹` la signature, `📝` la documentation, le graphe de
dépendances à part, un décompte par type. `compact` reste fourni, trois fois
moins cher en jetons.

`template=` prend un **nom**, pas un chemin : un graphe peut être écrit par un
modèle, et un chemin quelconque en ferait une lecture de fichier arbitraire.

## 6. Le catalogue de gabarits (`src/template.rs`)

**La fiche dans la base, le contenu sur le disque.** L'entité `Template` porte
nom, famille, catégorie, description, chemin, empreinte.

- `hashsafe` sur **`(famille, nom)`** : un `user` entité et un `user` composant
  sont deux choses, pas une collision.
- **chunké**, délibérément : l'index vectoriel vit sur la table de chunks, donc
  `chunked: false` rendrait la fiche introuvable autrement qu'au mot près — ce
  qui tuerait l'argument. Le coût est d'un chunk par gabarit.
- La **description** dit ce que la chose modélise ; le commentaire de
  conception va dans `note`, **non embarqué**. Mélanger les deux faisait
  arriver `user` dernier sur sa propre question.

`Pattern::apply` fusionne un motif **avant** l'enregistrement : après, ce
serait une migration, puisque la clé d'identité change et donc l'uuid de chaque
ligne écrite. Il n'ajoute que ce qui manque — l'entité a le dernier mot — et il
revalide.

## 7. Ce que burn fait, et ne fait plus

Embedding, rerank, OCR — bientôt TTS et STT. **Plus d'inférence de LLM**
(29 août) : elle vient de llama.cpp ou d'un fournisseur distant.

- `Device::vulkan()` **épingle SPIR-V** ; `Device::wgpu()` passe par un
  `AutoCompiler` qui choisit à l'exécution d'après les features. Ce choix
  invisible faisait passer un travail de 3,1 s à 71 s quand on ajoutait un
  backend.
- `burn-rocm` existe, **éteint** : 2,6× plus lent, et le seul fait de le
  compiler dégrade le chemin wgpu d'un facteur 14.
- `BurnRole` n'a plus que trois rôles.

## 8. Les invariants

1. Toute forme réduite d'un résultat dérive du **contenu entier** mis de côté,
   jamais de ce qui est dans l'historique.
2. Un lot d'embedding est borné **par le texte**, pas par le nombre.
3. Le choix d'une carte se **dit**, il ne se devine pas.
4. Un fil à plus de deux se **déclare**, il ne se dérive pas.
5. Une cible sans vecteurs rend **vide en le disant**, elle n'échoue pas.
6. Un défaut de paramètre est **admissible par définition**.
7. Un gabarit peut exister sans être offert ; le nom vit sur l'arête.
8. Un motif s'applique **avant** l'enregistrement, et n'écrase rien.
9. `cd` ne ment pas : il refuse **avec la liste**.
10. Ce que la liste cache, la lecture ne le rend pas — **pas encore vrai**,
    voir les dettes.

## 9. Les dettes nommées

- **Le chemin vectoriel ne classe pas** — [issue 01](../issues/29-08-2026/01-le-vecteur-du-moteur-ne-classe-pas.md).
- **Le plein texte ne trouve rien en français** — [issue 02](../issues/29-08-2026/02-le-plein-texte-ne-trouve-rien-en-francais.md).
- **`.vault/` est lisible** : `list()` saute les points-répertoires, `read()`
  ne les refuse pas. Un seul prédicat manque, partagé par les trois verbes.
- **`Meter::describe()`** n'a toujours aucun appelant.
- **`Trace` n'a pas de `hashsafe`.**
- **Les docstrings de C, C++, C# et Go** — le helper existe, il n'est branché
  que sur Rust, faute de code de ces langages sous la main pour vérifier.
- **`WorkPalette`** est nommée, pas écrite.
