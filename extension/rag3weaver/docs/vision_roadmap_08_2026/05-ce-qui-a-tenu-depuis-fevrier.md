# 05 — Ce qui a tenu depuis février

Relecture des documents de vision d'origine (30 janvier → 6 février 2026, quand
le projet s'appelait encore `ragforge-core-exp-kuzu` / `kuzu-wasm-exp`). Ce
n'est pas un résumé : c'est une **comparaison**, pour distinguer ce qu'on a
redressé de ce qu'on a laissé tomber sans décider.

*Avertissement sur le corpus* : sur 49 fichiers, **13 ne sont pas de nous** —
`index.md`, tout `guide/`, la configuration VitePress sont la documentation
amont de `kuzu-wasm` (Dylan Shang, `unswdb/kuzu-wasm`), vendorée telle quelle.
**Le nom du dossier vient de là, pas d'une intention** (§5).

## 1. L'intention d'origine

Deux projets superposés.

**Le framework** : *« un chemin incrémental pour transformer rag3weaver en
framework RAG universel, browser-first »*. Le code n'était **pas** le sujet —
*« une abstraction unique pour tout type de catalogue (voitures, immobilier,
produits, jobs, événements…) »*, et le RAG de code n'en était qu'une instance :
*« On ne doit pas créer un système code-only. On doit **instancier** le système
universel pour le code. »* Métrique de succès n° 2 : *« schema-driven : tout
configurable via YAML, zéro code »*.

**La base** : *« un fork de Kuzu optimisé pour le RAG avec fuzzy search natif »*,
parce que *« WASM build abandonné/non maintenu, pas de fuzzy search… contrôle
total sur la roadmap »*. Avec, déjà, la scission de licence : la bibliothèque
fuzzy en MIT « pour maximiser l'adoption », la base en Source Available.

**Le liant** : cinq couches L0→L5, `Catalog` en L3, un `schema.yaml` en entrée,
une démo navigateur en sortie.

## 2. Ce qui a tenu — souvent au mot près

Au point que certains fichiers Rust d'aujourd'hui sont la traduction littérale
d'un design de février.

| février | aujourd'hui |
|---|---|
| `consistency: immediate \| eventual \| strict`, défaut `eventual` | `enum Consistency`, `Default = Eventual` (`search.rs`) |
| `title_for` / `content_for` / `boost` par champ, `hashsafe` | `FieldDef`, `EntityDef` (`config.rs`) |
| `title_boost`, `content_boost`, `keyword_weight`, `rrfK: 60` | mêmes noms, `rrf_k = 60.0` |
| types `choice` et `tags` | `FieldType::{Choice, Tags}` |
| filtres `has-any` / `has-all` / `has-none` / `range` | `FilterOp::{HasAny, HasAll, HasNone, Between}` |
| `flush({upToPriority})` / `flushInsertions()` | `flush_insertions()`, `drain()` |
| stratégies `rrf` / `weighted` | `FusionStrategy::{Rrf, Weighted}` |
| *« Chunk = entité séparée mais **cachée** »* + `fulltext_on_chunks` | `{Entity}_Chunk`, BM25 par chunk, attribution par chunk |
| `searchWithExplore` + relations entrantes/sortantes | `outgoing_relations` / `incoming_relations`, `ExploreGraph` |

Trois intentions plus profondes ont tenu sous d'autres noms :

- **« L3 doit être domain-agnostic »** — 48 pages de plan pour sortir `Scope`,
  `PARENT_OF` et `signature` du cœur. **Atteint, pas abandonné** :
  `register_entity` / `register_kb` prennent la config, rien de « code » n'est
  en dur. Le problème central de février est réglé.
- **Le multi-KB agrégeant plusieurs entités** — *« une KB peut indexer des
  champs de PLUSIEURS entités »* est exactement le `KBUpdateNode` d'aujourd'hui.
- **La liste des domaines cibles n'a pas bougé d'un mot en sept mois** :
  « voitures, immobilier, restaurant, jobs, événements, e-commerce » ↔
  « voitures · produits · services · rendez-vous · médicaments · projets de
  loi ». **C'est le même projet.**

## 3. Ce qui a dérivé — choix ou oubli ?

C'est la distinction qui compte : après six mois, les deux se ressemblent, et
seul le second mérite qu'on y revienne.

### Choix assumés, tracés par écrit

| dérive | où c'est décidé |
|---|---|
| `fuzzy-fst` (bibliothèque complète, submodule, FFI C, crates.io) abandonnée | doc 14 : « redondant avec lucivy » |
| Summa/Tantivy en WASM abandonné | « `wasm32-unknown-unknown` n'a pas `std::thread` » |
| hooks `onResultEnrich` / `onBoost` (~150 lignes de design) | « N+1 requêtes, logique opaque ; `ResultMode` + `Explore` couvrent les cas » |
| agrégation des signatures d'enfants dans le `content` du parent | « explose le content, dégrade les chunks » |
| **browser-first** rétrogradé | doc 51 : un des quatre produits, « bloqué par la taille des modèles » |
| le YAML remplacé par du JSON déclaratif | conséquence assumée du portage TypeScript → Rust |

### Oublis — perdus sans décision

- **`special_ops: { grep, read }`.** Le champ **existe encore** dans notre
  config — `pub special_ops: Option<HashMap<String, Value>>` — et **rien ne le
  lit**. Un champ mort dans une struct est la signature d'un oubli. Voir §4.2.
- **La persistance des opérations** (`_Operation`, `recover()`,
  `getFailedOperations()`, `retryOperation()`) — deux documents entiers, marqués
  « implémenté ». *Cas partiellement indécidable* : les checkpoints du dataflow
  avec `undo` et `drain_resume` couvrent le **rejeu**, mais **l'inspection et le
  retry sélectif des opérations en échec** n'ont aucun successeur, et aucun
  document ne dit qu'on y a renoncé.
- **`getFilterOptions()`** — *« retourne `{marque: [...], prix: {min,max}}` »*
  pour construire une UI à facettes. Zéro trace, zéro successeur. **Oubli net**,
  et il redevient pertinent avec les catalogues ([02](02-les-deux-moities.md)).
- **Le routing multi-KB par intention** (`searchAll()`, `QueryRouter`,
  « question sur la fiabilité → ReviewsKB »). Le [04](04-le-catalogue-comme-graphe.md)
  réinvente un routing, mais sur les **outils**, pas sur les KB.
- **L'enrichissement automatique** — `photos: { type: images, analyze: true,
  content_for: CarKB }` : la vision transformait une image en contenu
  cherchable. Oubli, et **il pique maintenant que l'OCR est livré**.
- **Le vocabulaire L0–L5** a disparu sans une ligne, ce qui rend les vieux
  documents illisibles (§6).

## 4. Six idées qui dormaient et valent d'être reprises

### 4.1 Le protocole de détection de l'identifiant unique

Écrit en entier, en quatre étapes :

> 1. ID explicite dans le schéma ? 2. Colonne évidente — motifs `id`, `sku`,
> `reference`, `ref`, `code`, `uuid`, `ean`, `numero` → trouvé **et** valeurs
> uniques → proposer. 3. Demander au modèle : « quelle colonne ou combinaison
> serait unique ? » → proposer au client, **confirmation requise**. 4. Repli :
> hachage du contenu ⚠️ « incrémental impossible sans ID stable ».

Avec un contrat de retour `needs_confirmation` portant le type détecté, les
colonnes, un score et des exemples.

**Pourquoi ça vaut encore** : `hashsafe` existe dans le code, mais il faut le
*fournir*. Un tableur n'en a pas. C'est le chaînon manquant entre « un xlsx
tombe » et « le mode KB ingère » — et l'étape 3 est un `LlmNode` d'aujourd'hui.
À lire **avec** le [03](03-normaliser-des-tableurs.md) §6, qui montre ce qui
arrive quand on ne le fait pas.

### 4.2 `grep` et `read` : `File` n'est pas un article de catalogue

> *« File n'est **pas** un item de catalogue normal. C'est : le **conteneur
> physique** des autres entités, la **source de vérité** pour les offsets, et
> l'unité de **grep** et **read**. »*

Avec l'API : `grep('TODO:', { paths, context: 2 })`,
`readFile(path, { offset: 42, lines: 50 })`, et une KB `search: fulltext`,
`chunking: { enabled: false }` — *« JAMAIS chunké »*.

**Pourquoi ça vaut encore** : le produit n° 1 est un **agent de code embarqué**,
et les deux outils qu'un agent de code utilise le plus sont grep et
read-with-offset. On a lucivy (`Regex`, `Symbol`, highlights par offsets
d'octets) : **les briques sont là, l'exposition ne l'est pas.** Et `special_ops`
attend toujours dans la config.

### 4.3 Dériver la taille de chunk des limites du modèle

> *« Le `max_size` du chunking devrait être dérivé de `max_input_tokens` du
> provider. »* — avec `max_size: auto` et un registre par modèle.

**Pourquoi ça vaut plus qu'alors** : on a maintenant **six modèles burn** aux
fenêtres différentes, et le doc 42 note déjà pour le MiniLM multilingue une
« **troncature 128 héritée** ». Un chunker qui ignore quel modèle l'attend
produit du contenu tronqué **en silence**. Le trait `Embedder` peut porter un
`max_input_tokens()`.

### 4.4 Le rapport de validation à l'ingestion

Erreurs **bloquantes** (champ `price` non numérique, type déclaré qui ne
correspond pas) contre **avertissements** (valeur `choice` non déclarée, image
référencée mais absente, champ déclaré jamais utilisé) — avec une
**suggestion** : *« ajouter 'GPL' aux valeurs possibles ? »*

**Pourquoi ça vaut encore** : c'est la culture `meta.warnings` du doc 42
(« toujours peuplé et honnête ») appliquée à **l'ingestion**, où elle n'existe
pas. Et un avertissement porteur de suggestion est **directement lisible par un
agent qui doit se corriger**. À croiser avec le [03](03-normaliser-des-tableurs.md)
§7, qui montre la forme aboutie : localisé, illustré, dédupliqué, plafonné.

### 4.5 Le boost par champ, devenu bon marché

> *« Title/Content Boost — workaround pour field boosting (**limitation
> Kuzu FTS**). Idée : créer des index FTS séparés par champ, pondérer les
> scores. »*

**Pourquoi ça vaut encore** : la raison de le repousser était « limitation
Kuzu ». **Cette limitation n'existe plus** — on possède le moteur FTS, et
lucivy a des *fast fields* et des filtres natifs par champ. Mieux : les poids
historiques `signature ×2.5 / content ×1.0 / docstring ×0.5` viennent d'un
document de février qui contient **un exemple travaillé complet** (un scope de
3 000 lignes contre un `parseJSON`, `boost = 1 + log(matchCount) × 0.1`,
résultat 2,10 contre 1,02) — **c'est un cas d'éval prêt à l'emploi**, et l'éval
est justement ce qui manque.

### 4.6 Profondeur d'exploration par type de relation

Trois niveaux étaient prévus : options directes, **preset `ExploreStrategy`**,
et **hook `onGetRelations(node, ctx)`**. Seul le premier existe. Le plus
intéressant est au milieu :

> `maxDepthPerRelation: { 'CITES': 2, 'CITED_BY': 3 }`

**Pourquoi ça vaut encore** : le [04](04-le-catalogue-comme-graphe.md) fait des
`Tag` un **graphe de concepts** avec `RELATES_TO`. Explorer un graphe de tags
sans profondeur différenciée par type d'arête, **c'est exploser**. Et un preset
nommé est, dans le vocabulaire du 04, **un graphe-outil** — donc une entité
cherchable, pas une constante.

*Plus brièvement* : la détection **dynamique** de conteneur
(`hasChildren = edges.some(e => e.type === 'PARENT_OF')`, qui permet à une
variable ou une fonction ayant des enfants d'être traitée comme un conteneur) —
utile le jour où `codeparsers` est branché ; et le format « CSV enrichi » à
métadonnées en en-tête (`#schema:kilometrage=number:range:sort`), rejeté à
l'époque comme « peu intuitif », **qui redevient intéressant si c'est un modèle
qui écrit l'en-tête**.

## 5. Ce que l'expérience a invalidé — ne pas y revenir par nostalgie

1. **`fuzzy-fst` comme produit.** Deux semaines de feuille de route, un dépôt,
   un submodule, une licence, un plan de publication — et cinq jours plus tard
   un document constate « intégration FTS pas commencée ». lucivy a mangé le
   besoin.
2. **Modifier le FTS C++ de Kuzu.** Tout un plan y visait. Le doc 41 acte
   l'inverse : **plus aucune extension C++ n'embarque de Rust**.
3. **Summa/Tantivy en WASM** — 6,7 Mo, puis tué par `std::thread`.
4. **Le cadrage « limitation Kuzu ».** `fuzzyDistance` et `title_boost` étaient
   classés impossibles pour cette raison. **Toute décision fondée sur cette
   phrase est périmée.**
5. **L4 comme couche.** Un document recommandait « L4 simplifié (events +
   stats) ». C'est allé plus loin : Accumulator, EmbedderPool et EventEmitter
   sont **dissous** dans la queue, le dataflow et la diffusion. La prédiction
   était juste, la couche n'a pas survécu.
6. **Pthreads / SharedArrayBuffer / COOP-COEP** comme prérequis structurant.
   Plus jamais mentionné en sept mois.
7. **Le calendrier.** *« Total MVP : 5 semaines »*. Cinq jours plus tard, un
   tableau « roadmap originale contre réalité » avec trois lignes sur quatre en
   partiel. **La leçon est plus utile que le plan.**

## 6. Vocabulaire — table de conversion

| février | aujourd'hui |
|---|---|
| **ragforge-core** | disparu ; rag3weaver est devenu le produit |
| **kuzu-wasm-exp** | `extension/rag3weaver` |
| L0 `KuzuConnection` | `DbConnection` + `SchemaDialect` |
| L1 `SchemaBuilder` (fluent) | `register_entity` / `register_relation` |
| L2 `DocumentStore` / `Chunker` | nœuds de chunking, `{Entity}_Chunk`, `hashsafe_uuid` |
| **L3 `Catalog`** | `Catalog` — **le seul nom de la pile à avoir survécu** |
| L4 `Orchestrator` / `Accumulator` | `OperationQueue` puis DAG dataflow + `drain()` |
| L5 `code-rag` | `codeparsers/` (**dormant**) |
| `hybridStrategy: 'boost'` | `SignalRole::Boost` — **sémantique différente** : boost par *signal*, pas par prédicat sur le résultat |
| `boostIf` / `onBoost` / `onResultEnrich` | pas d'équivalent (abandonnés) |
| `fuzzy-fst` | lucivy (paramètre `distance`) |
| `schema.yaml` | JSON déclaratif (`config.rs`) |

**Deux pièges de lecture, à connaître avant d'ouvrir un vieux fichier :**

- **`Scope`.** En février, c'est **l'entité de code** (une fonction, une classe :
  `scopeType`, `signature`, `startLine`). Aujourd'hui, `Scope { org, project }`
  est **le cloisonnement multi-tenant**. Collision totale. Chaque occurrence de
  « Scope » dans le vieux corpus parle de code, jamais de locataire.
- **`rag3weaver` en février désigne du TypeScript** — un `l3.js` de 6 224
  lignes. Les métriques de ces documents sont des lignes JS et ne se comparent
  pas aux 45 000 lignes Rust d'aujourd'hui.

## 7. Le navigateur : un retournement que personne n'a acté

**Ce qu'on en disait** — *« Principes directeurs : … 2. **Browser-first — tout
doit tourner dans le navigateur.** »*, avec une « vision finale » dont la couche
du bas est « L0 : Kuzu WASM (graph DB in browser!) », une métrique de succès
« démo complète dans le navigateur », et une cible chiffrée : **« WASM < 3 Mo
gzippé »**.

Et une inversion qui frappe rétrospectivement :

> *« **Phase 5 – Native Kuzu (future)** : porter Level 1 en extension C++ …
> **benchmark browser vs native.** »*

Le natif était l'horizon lointain. **Aujourd'hui le natif *est* le projet, et le
navigateur est l'horizon lointain.** L'axe s'est exactement retourné, et on ne
trouve **nulle part** de document qui acte ce retournement.

**Ce qu'on en dit aujourd'hui**, en trois régimes qui ne disent pas la même
chose :

1. **Le build est en dette, honnêtement nommée** — « non revalidé depuis mai ».
   Nuance : ce n'est pas mort, la bibliothèque compile et passe **606 tests
   sous `wasm-emscripten`**. C'est **l'artefact complet** qui n'est pas
   revalidé.
2. **Le critère de sélection est intact, et c'est la continuité la plus
   forte.** Chaque dépendance nouvelle est encore jugée sur le WASM, en août
   2026 : `llguidance` « wasm OK », `minijinja` « wasm sans flag »,
   `hf-chat-template` « compile en wasm32 », `apistos` **rejeté** parce qu'il ne
   compile pas ; burn/wgpu choisi parce que c'est « un code pour
   AMD/NVIDIA/Apple/**navigateur** ». **Le navigateur ne guide plus la feuille
   de route mais il guide encore chaque choix technique.** Personne ne l'a
   écrit ; ça ne se voit qu'en agrégeant.
3. **La thèse est promue, pas abandonnée** — le doc 36 en fait un **nœud de
   calcul** (« le shard est l'unité de distribution »), tout en posant la
   borne : « burn-wgpu dans un onglet est plausible, **pas prouvé**. Il faut une
   passe Playwright avant de dire compute unit. »

**Et le blocage a changé de nature.** En février, c'était la base : threads,
COOP/COEP, taille du `.wasm`. Aujourd'hui c'est **la taille des modèles** — le
plus petit embedder utile fait 90 Mo, le multilingue 470 Mo, BGE-M3 2,2 Go. La
cible « WASM < 3 Mo » était juste, et **complètement hors sujet** : ce n'est
plus le binaire qui pèse.

**Tombé sans un mot** : le prérequis COOP/COEP (configurations Netlify, Vercel,
Express, `coi-serviceworker`) et les cibles Node/Deno du pipeline de build. Ce
sont des contraintes de *déploiement* réelles pour la passe Playwright que le
doc 36 réclame — il faudra les redécouvrir.

## 8. Ce qui n'a pas pu être établi

- **Le retournement navigateur → natif n'est acté nulle part**, il est déduit de
  la comparaison des textes. S'il existe une décision écrite, elle est dans les
  239 documents de `docs/` racine (6 février → 8 mars), non lus.
- **Le sort de la persistance des opérations** : impossible de trancher entre
  « remplacé par les checkpoints » et « oublié ».
- **`title_boost` / `content_boost`** sont désérialisés — **non vérifié** s'ils
  influencent réellement le scoring ou s'ils sont, comme `special_ops`, des
  survivances inertes. Dix minutes de vérification avant de reprendre §4.5.
