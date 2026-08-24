# Doc 14 — Mémoire des ambitions (février → mai 2026)

Écrit le 24 août 2026, en relisant **323 documents** de session : 239 dans
`docs/` (racine, ère fork kuzu / C++, 6 février → 8 mars) et 84 dans
`extension/rag3weaver/docs/` (ère Rust, 8 mars → 17 mai).

Ce doc ne raconte pas ce qui a été *fait* — c'est le rôle du [doc 11](11-etat-des-lieux-24-aout.md).
Il restitue ce qui était **voulu** : les visions énoncées, les raisons des choix, et
surtout **les intentions qui se sont perdues sans jamais être annulées**.

Compagnons : [11 — état des lieux](11-etat-des-lieux-24-aout.md) ·
[12 — ambitions et roadmap](12-ambitions-et-roadmap.md) ·
[13 — knowledge dump](13-knowledge-dump.md)

**Convention de fiabilité.** Les docs anciens mentent parfois : ils décrivent des
choses comme acquises qui ne l'ont jamais été. D'où deux marqueurs :

- ✅ **vérifié dans le code au 24 août 2026**
- 📄 **affirmé par un doc seulement** — non revérifié, ou vérifié faux (dit alors explicitement)

---

## 1. L'énoncé fondateur

La toute première phrase du tout premier état des lieux, le 6 février, répétée
mot pour mot le 8 :

> **Objectif final :** Remplacer Neo4j par Kuzu (embedded) + Lucivy (FTS
> fuzzy/regex) dans un seul module WASM, pour alimenter le framework Rag3Weaver
> puis ragforge-core et community-docs.
>
> — `docs/6-fevrier-2026-22h22/00-etat-des-lieux.md:3`

Et sous chaque diagramme d'architecture cible, la même parenthèse :
**« Remplace Neo4j (zero Docker, embedded) »**.

Le « zéro Docker » n'est pas un confort de développement. C'est **l'énoncé
fondateur**, et c'est le critère qui a tranché tous les arbitrages importants
depuis : le refus de PyTorch, le refus des sidecars Python, le passage de candle
à burn, le choix de garder le fork kuzu. Chaque fois qu'une décision future
semblera difficile, c'est cette phrase qu'il faut relire.

La chaîne de valeur visée allait d'ailleurs bien au-delà de la base :

```
rag3db  →  Rag3Weaver  →  ragforge-core  →  community-docs
```

**Six avantages** étaient revendiqués pour l'approche (même doc) : un seul module
WASM, threads via SharedArrayBuffer, **« FTS supérieur »**, **« contrôle total —
on possède le fork, on peut itérer librement »**, embedded/zéro serveur, et le
filtrage graph→FTS.

Le quatrième est la justification profonde du fork : *posséder* le moteur pour
pouvoir le modifier. Ce qui a effectivement été fait — 48 fichiers modifiés dans
le fork dès le 8 février, puis un nouveau `IndexRecordOption` à la Lucene.

**Dimension business, souvent oubliée** ✅ : licence **LRSL v1.2**, source-available,
seuil de **100 000 € de revenu annuel** (`LICENSE`). Ce n'est pas un projet
open-source sans modèle — c'est un produit avec une clause commerciale.

---

## 2. Le différenciateur technique d'origine : chercher du **code** comme un humain

C'est l'intention la plus originale du corpus, et la plus facile à oublier parce
qu'elle est technique et ancienne. Elle n'est pas « faire du RAG » :

> On veut un `contains` qui fonctionne comme un humain chercherait : on colle un
> bout de texte (code, identifiant, phrase), et ça retrouve le passage même avec
> des typos, des variations de casse, ou des fragments partiels.
>
> — `docs/6-fevrier-2026-22h22/04-contains-query-design.md`

L'exemple canonique, répété partout : `"this.I.My"` doit matcher `"thys.Is.MyQueri"`.
Et les cas de test emblématiques : `c++`, `std::collections`, `os.path.join`.

Deux inventions en découlent :

- **le budget de distance cumulatif** — pas une distance par token, un budget
  global sur toute la requête ;
- **la validation des séparateurs** — `strict_separators = true` par défaut.

Avec le coût architectural lucidement anticipé dès le départ : *« on ne peut pas
utiliser des automatons Levenshtein indépendants par position (ils ne partagent
pas de budget). Il faut un scorer custom… C'est un changement plus profond que
juste combiner des automatons existants. »*

**Le refus de l'auto-détection** est une décision de goût, tranchée par Lucie
elle-même dans le seul doc en dialogue direct du corpus
(`docs/14-fevrier-2026-20h36/03-*.md`, annoté « lucie: ») : `"c++"` ne doit
**jamais** être interprété comme un regex, *« car l'user peut vouloir justement
chercher des regex […] faudrait par défaut false »*. Et le vrai objectif de
l'unification, dit par elle : *« oui bien sûr fuzzy sur littéraux regex c'est
exactement ce qui m'intéresse dans cette unification »*.

C'est exactement le trou séparateurs que lucivy v3 rouvre aujourd'hui. **Ce n'est
pas une régression : c'est le problème fondateur du projet, jamais entièrement
résolu.**

---

## 3. Les cinq visions successives

Le projet a changé de nature cinq fois en trois mois. Aucun de ces virages n'est
consigné comme une décision — ils se déduisent des docs.

| # | Période | Ce que le projet était censé être |
|---|---|---|
| 1 | 6–15 fév | Un **fork kuzu + FTS supérieur** en un seul WASM, pour remplacer Neo4j |
| 2 | 20–28 fév | Un **framework RAG universel**, « alternative crédible à Qdrant », pas seulement du code |
| 3 | 1–3 mars | Une **base multi-index où tout se branche dans le `WHERE` Cypher** (INDEX_SCAN) |
| 4 | 6–8 mars | Un **moteur de dataflow générique** — un seul DAG pour search, ingestion et migrations |
| 5 | 13–24 mars | Un **framework RAG multi-backend** dont la cible de production est **Supabase**, pas rag3db |

Le virage n°5 est le plus contre-intuitif aujourd'hui et mérite d'être cité :

> Supabase = auth, storage, realtime, edge functions — stack complète […]
> **Adoption développeurs ≫ rag3db pour le moment**
>
> — `13-mars-2026-18h45/02-priorites-multi-backend.md:16-21`

Avec une matrice de backends priorisée incluant **Neo4j et Qdrant** comme cibles
🟡 moyennes — jamais reprises depuis.

Autrement dit : tout le chantier `SchemaDialect` / `SearchBackend` / `BlobStore`
n'était pas un exercice de propreté. **C'était la préparation d'un déploiement
cloud sur Postgres**, où rag3db jouait le rôle de terrain d'essai. C'est un
renversement complet par rapport à la vision n°1, et il n'a jamais été ni acté
ni annulé.

### Le portage TypeScript → Rust

Le README du 14 février annonçait rag3weaver comme *« wrapper Node.js/TypeScript
qui expose les fonctions Cypher en API haut niveau »*. Le doc
`docs/14-fevrier-2026-22h57/01-rag3weaver-port-to-rag3db.md` argumente le
basculement en Rust : language-agnostic, self-contained (*« un seul binaire = DB
+ FTS + catalog »*), chunking *« 5-10× plus rapide »*, et surtout ce tableau des
wrappers résiduels par langage — Node.js : NAPI glue ; Python : PyO3 ;
**C/C++ : « Rien (appel direct) »**.

~6 500 lignes de TypeScript à porter. C'est devenu 40 000 lignes de Rust.

---

## 4. Ce qui est resté constant

Malgré cinq virages, cinq principes n'ont jamais bougé. Ce sont eux le noyau.

1. **Zéro Docker, zéro Python, zéro serveur.** Le critère d'arbitrage permanent.
2. **Le graphe est le produit**, pas un accessoire. *« Le graphe de dépendances
   EST le contexte »* (`docs/20-fevrier/07`). *« Contexte croisé impossible avec
   un vector store »* (`docs/2-mars/03`).
3. **Le `contains` cross-token de lucivy est le différenciateur FTS**, et il faut
   pousser les utilisateurs dessus : *« Utiliser `contains` pour tout »*
   (`docs/6-mars/12`).
4. **L'API est conçue pour les LLM autant que pour les humains.** Dès le 6
   février : les modes Cypher sont *« destinés aux utilisateurs et aux LLMs »*,
   `exact` existe *« pour un agent/LLM qui connaît le terme exact »*.
5. **Le Catalog est la seule surface.** *« L'utilisateur pense en termes de
   Catalog. Le Catalog pense en termes de sous-graphe. Les nœuds pensent en
   termes de backend. »* (`12-mars-2026-15h21/04`). Corollaire : pas de Cypher
   exposé, pas de `.mmd` utilisateur, **pas de MigrationRunner public**. Le DAG
   est un détail d'implémentation, jamais un produit.

---

## 5. Les ambitions perdues

Classées par valeur décroissante. **Statut vérifié dans le code aujourd'hui.**

### 5.1 Le reranking cross-encoder — le meilleur ratio qualité/effort jamais identifié

✅ **Absent du code** (aucune occurrence de `rerank`).

> Le hybrid search fait du RRF […] **sans reranking. Le RRF mélange des scores
> hétérogènes** (BM25 log-freq vs cosine similarity vs sparse dot product) — le
> classement final est approximatif.
>
> C'est **le plus gros gain de qualité retrieval pour le moins d'effort**.
>
> — `13-mars-2026-18h45/03-directions-futures-reranking-eval-multitenancy.md:9-28`

Design déjà écrit : un `RerankNode` après le top-K du RRF, un
`trait Reranker` *« même pattern que Embedder »*, implémentations API (Cohere,
Jina) ou locale (BGE-reranker-v2-m3 via candle/ONNX).

**Le point à retenir** : le RRF actuel n'est pas « bien » — il est documenté
comme **approximatif par construction**. Ce n'est pas une amélioration
hypothétique, c'est un défaut connu depuis mars.

### 5.2 L'évaluation type RAGAS — « on ne peut pas optimiser ce qu'on ne mesure pas »

✅ **Absent du code**.

> Quand on change la stratégie de chunking, les poids hybrid, ou qu'on ajoute le
> reranking — comment savoir si c'est mieux ?
>
> — `13-mars/03:30-62`

API cible déjà rédigée : `catalog.evaluate(&[EvalCase { query, expected_ids, expected_answer }])`
→ `recall_at_k(5)`, `mrr`, `faithfulness`, `context_precision`. Avec deux
intentions d'intégration précieuses : **tourner en CI** pour détecter les
régressions de qualité, et **être compatible BEIR/MTEB**.

Aujourd'hui, rien ne dit si un changement de fusion améliore ou dégrade la
qualité. Le CI créé hier teste la compilation, pas la pertinence.

### 5.3 Le multi-tenancy — un avertissement explicite, non suivi

✅ **`_tenant_id`, `TenantScope`, `project_id` : tous absents du code.**

⚠️ Correction d'un doc de mars qui affirmait *« projectId existe déjà comme
filtre sur search »* : **c'est faux aujourd'hui**, aucune occurrence.

La distinction conceptuelle mérite d'être conservée :

- **Niveau 1 — notre cloud** : `orgId`, une DB par org, quotas, billing.
- **Niveau 2 — le multi-tenant *de nos utilisateurs*** : `projectId`. *« Un
  développeur déploie une seule instance rag3weaver, mais sert N clients. »*

Et l'avertissement, écrit deux fois :

> **Trait `CatalogBackend` doit intégrer orgId + projectId dès le design, pas
> après.** — `13-mars/03:110` et `:161`

`SchemaDialect` (50 méthodes) et `SearchBackend` (6 méthodes) ont été écrits sans.
**Rattraper ça touchera les 50 méthodes.** C'est le coût direct d'un avertissement
ignoré.

### 5.4 L'ingestion de documents réels — la seule ambition tournée vers l'adoption

📄 Jamais commencée.

> L'objectif : un développeur branche son parser favori en **5 lignes** et ingère
> des documents réels — `13-mars/02:85`

Avec **Docling (IBM)** et **Microsoft MarkItDown** nommés comme cibles. Priorité
**7-8 sur 12**, donc *avant* le reranking et l'éval.

### 5.5 Le schéma YAML universel et le mode zero-config

📄 Jamais implémenté. C'est **le seul endroit de tout le corpus où un
positionnement produit hors-développeur est énoncé.**

```yaml
fields:
  prix:    { type: number, filter: range, sort: true }
  marque:  { type: choice, values: auto, filter: multi-select }
  options: { type: tags, filter: has-any, content_for: AnnoncesKB }
```

Avec auto-détection des types depuis un CSV : *« < 20 valeurs uniques → choice ;
contient "|" → tags ; format date reconnu → date »*. Six domaines documentés :
voitures, immobilier, restaurant, jobs, événements, e-commerce.

Verdict de l'époque : *« Priorité basse — le code-first est plus pragmatique pour
l'instant, mais **le schéma YAML reste la vision cible pour l'adoption
non-développeur** »* (`docs/2-mars/02:434`).

### 5.6 Les six abstractions cross-domain

✅ **Aucune n'existe dans le code.** `docs/2-mars-2026-04h34/03-idees-avancees-abstractions.md` §8 —
un plan de conception complet, à zéro ligne de code :

| Abstraction | Ce qu'elle apportait |
|---|---|
| `SourceInfo` | provenance universelle (`source_url`, `content_hash`, `ingested_at`) → GC des entités dont la source a disparu |
| `HierarchyTrait` | `parent()/children()/ancestors()` → explore BFS **sans connaître le nom de la relation** |
| `MentionDetector` | détection auto d'URLs/emails/identifiants → **crée les relations pendant le drain** |
| `VersionedEntity` | `PREVIOUS_VERSION` / `SUPERSEDED_BY` → chercher dans l'historique |
| `EnrichmentPipeline` | enrichissements gratuits vs coûteux (Vision AI, LLM) **opt-in** |
| `TenantScope` | cf. §5.3 |

### 5.7 Les agents LLM — un plan en 3 phases écrit en mai, jamais démarré

📄 `10-mai-2026-04h30/01-piste-agent-llm-normalisation.md`, le doc le plus
« produit » du corpus, et le plus récent avant août.

- **Phase 1** : `LlmNormalizeNode` + `trait LlmProvider`, basé sur **Rig**.
- **Phase 2** : crate `rag3weaver-agent` — `LlmClassifyNode`, `LlmExtractNode` (NER), `LlmValidateNode`.
- **Phase 3** : fork d'**AutoAgents**, en remplaçant Ractor par le trait `Node`.

Le raisonnement architectural central :

> On utilise Rig comme couche LLM […] **le dataflow rag3weaver reste
> l'orchestrateur. Pas de conflit de runtime.**

Détail à ne pas perdre : la normalisation devait être une **propriété de schéma**,
pas du code applicatif — un bloc `"normalize": { provider, model, rules: [...] }`
par entité dans le JSON déclaratif.

### 5.8 Le streaming / watch mode

📄 Jamais fait. *« L'ingestion est batch. **En production, les documents arrivent
en continu.** »* API esquissée : `catalog.watch(WatchSource::S3 { … })
.with_parser(…).on_new(…).start()`. Sources visées : S3/GCS, inotify, webhook,
SQS/RabbitMQ/Redis streams. Note d'implémentation qui reste valable :
**l'EventBus existant peut servir de backbone.**

### 5.9 `S3BlobStore`

✅ **Absent.** Le trait `BlobStore` a été conçu générique **précisément pour**
qu'un backend objet cloud s'y branche (`12-mars/20:133`). Le trait existe et est
utilisé par lucivy et sparse ; l'implémentation S3 n'a jamais été écrite. C'est
sans doute la plus petite des ambitions perdues — et la plus facile à rattraper.

### 5.10 Le niveau 4 d'INDEX_SCAN — la « vision finale » du 1er mars

📄 Le mécanisme s'est arrêté au niveau 3 (un signal = un prédicat = un scan). Le
niveau 4 prévoyait :

```cypher
MATCH (d:Document)
WHERE SEARCH(d, 'rust safety', kb := 'main')
RETURN d.title,
       SEARCH_SCORE(d), SEARCH_BM25_SCORE(d), SEARCH_VECTOR_SCORE(d),
       SEARCH_HIGHLIGHTS(d), SEARCH_CHUNKS(d)
```

Avec **la fusion multi-signaux dans l'opérateur physique** et surtout *« le
planner optimise automatiquement l'ordre (search first, then graph join) »*.
Chiffré `10+ sessions`. Le niveau 2 (`return_fields`, `filter :=` SemiMask) a été
**sauté sans décision écrite**.

---

## 6. Les portes fermées sans qu'on le dise

Ce sont les pertes les plus insidieuses : un refactor de simplification qui ferme
une ambition, sans que le doc de refactor le mentionne.

### 6.1 `DynamicNode` — la capacité agentique du DAG

✅ **`DynamicNode` et `GraphEmitter` sont absents du code.**

Le design du dataflow (6 mars) listait six motivations, dont deux qui ne sont
jamais devenues des phases : **« Étapes LLM »** et **« Agentic »**. Elles étaient
pourtant dessinées en détail :

> `LLMDecideNode` est un nœud agentic : il appelle le LLM avec les résultats
> intermédiaires et **le LLM décide quels nœuds spawner**. C'est un `DynamicNode`
> dont la logique d'émission vient du LLM.
>
> — `docs/6-mars-2026-00h01/09-dataflow-graph-design.md:412`

Le 7 mars, `DynamicNode` est supprimé (~300 lignes) au motif qu'il n'a qu'un seul
usage en production, rendable statique. **Le doc de suppression ne mentionne
nulle part qu'il ferme la porte à l'expansion de graphe pilotée par LLM.**
Personne n'a re-mentionné `LLMDecideNode` depuis.

C'est réversible, mais il faut le savoir : la brique agentique a été retirée par
un refactor de propreté.

### 6.2 La roadmap 1→12 qui s'arrête à 6

La feuille de route la plus complète jamais écrite (`13-mars/03:149-159`) :

```
1-3.  IndexBlobStore                      ← fait (mars)
4-6.  CatalogBackend + Supabase           ← fait (mars)
7-8.  Ingestion documents réels           ← jamais abordé
9.    Reranking                           ← jamais abordé
10.   Évaluation                          ← jamais abordé
11.   Multi-tenancy                       ← jamais abordé
12.   Streaming / watch mode              ← jamais abordé
```

Les étapes 1 à 6 sont de l'**infrastructure**. Les étapes 7 à 12 sont **le
produit** — DX, qualité, cloud. Le projet s'est ensuite déplacé latéralement vers
luciole (mai) puis le FTS Rust (août), deux chantiers d'infrastructure **absents
de cette roadmap**. Aucun doc ne consigne cette réorientation.

**C'est le constat le plus important de ce document.** Depuis mars, tout l'effort
est allé à la plomberie, et rien à ce qui rendrait le produit meilleur pour
quelqu'un d'autre que nous.

### 6.3 Le Rhai / ScriptNode

✅ **Absent du code.** Deux sessions entières de design (3 et 7 mars) : modèle de
sécurité *« additif, pas soustractif »* à la `redis.call()`, sandbox chiffrée
(100 000 opérations ≈ 10 ms CPU, strings 1 Mo, arrays 10 k), garantie dure
*« un script ne peut jamais écrire dans le graph ni accéder au filesystem »*, et
le raisonnement décisif : **« Rhai tourne en WASM, ProcessNode non »**.

Bloqué depuis mars par une brique de 50 lignes : `Deserialize` sur les types
search (`UnifiedResult`, `ChunkInfo`, `SearchMeta`…). La même brique bloque aussi
le checkpoint des pipelines search.

### 6.4 Chaque `.mmd` déposé devient un type de nœud

📄 Mécanisme d'extensibilité **sans code**, plus ambitieux et plus simple que
Rhai, jamais listé dans les phases :

> Un fichier `.mmd` dans un dossier `templates/` est automatiquement enregistré
> comme un type de nœud disponible. — `docs/6-mars/09:986`

Avec, dans le même esprit, l'analogie assumée : *« le pattern exact de Blender
(shader nodes) et Unreal (Blueprints) »*, et des blocs nommés pour non-experts —
*Ingest Docs*, *Make Searchable*, *Add Semantic Search*.

---

## 7. Les abandons justifiés — à ne pas rouvrir

Utile pour ne pas refaire deux fois la même analyse.

| Abandonné | Raison |
|---|---|
| **fuzzy-fst** (lib standalone complète) | redondant avec lucivy |
| **Summa / wasm-pack** | `wasm32-unknown-unknown` n'a pas `std::thread` |
| **Séparateurs comme tokens dans l'index** | +30-50 % de taille ; validation post-hoc par byte offsets plus propre |
| **DFA Levenshtein sur les n-grams** | *« fausse bonne idée »*, 3 variantes évaluées — DFA et n-grams sont **complémentaires, pas interchangeables** |
| **Table registre `_lucivy_indexes`** | metadata sérialisée avec la NodeTable = atomique avec le checkpoint |
| **RocksDB / LMDB pour le sparse** | ne compilent pas en WASM |
| **pgvector `sparsevec`** | *« pas d'index inversé dessus. C'est pire que notre HashMap »* |
| **Apache AGE / SQL/PGQ** | pas dispo sur Supabase managed / prototype PG18 |
| **Trait `SearchIndex` commun aux 3 index** | les 3 sont à des niveaux différents de la stack ; **la fusion est le bon niveau d'abstraction** |
| **Hooks `onResultEnrich`** | N+1 queries, logique opaque ; ResultMode + Explore couvrent les cas sans requête supplémentaire |
| **Agrégation auto des enfants dans `_content`** | explose le content, dégrade les chunks |
| **candle en WASM** | BLAS/LAPACK, pas de SIMD emscripten |
| **PyTorch / sidecar Python** | viole l'énoncé fondateur (§1) |

Et une leçon d'ingénierie qui vaut d'être retenue : la corruption
`LIST<STRUCT>` de février venait du `HashMap` Rust (ordre d'itération non
déterministe entre instances). Fix structurel `HashMap → BTreeMap` :
*« ordre alphabétique garanti, **problème éliminé structurellement** »*.

---

## 8. Les chiffres qui donnent l'échelle visée

### L'horizon de scale

Modèle de charge sparse (`12-mars/15`), 50 tokens non-nuls/doc :

| Docs | Taille | open() | RAM |
|---|---|---|---|
| 100k | ~150 Mo | ~500 ms | ~150 Mo |
| 1M | ~1,5 Go | ~5 s | ~1,5 Go |
| 10M | ~15 Go | ~50 s+ | **« Inviable »** |

**La cible implicite est donc entre 1M et 10M documents.** C'est cohérent avec
l'obsession « sources kernel Linux ».

Et la campagne de benchmarks prévue dès février, **jamais lancée** : petit corpus
(lucivy elle-même, ~500 fichiers Rust), **moyen (headers du kernel Linux,
~10k fichiers)**, grand (top 100 packages npm, ~100k fichiers).

### Latences par signal

`bm25_only` **22 ms** · `dense_only` 210 ms · `sparse_only` 314 ms ·
`bm25+vector` 371 ms · **`all_three` 555 ms** (1er mars, GPU CUDA).

Search DAG parallèle : **3× potentiel** en lançant les trois signaux en parallèle.

### Round-trips

Le gain visé du niveau 1 d'INDEX_SCAN était **~0.3×** de latence en éliminant les
round-trips (4-5 → 1). Le batching UNWIND visait **2700 → 13 requêtes** pour
500 entités, avec ce constat au passage : *« vérifié via git history, les INSERT
et LINK n'ont **jamais** été batchés »* — 1500 allers-retours au lieu de 10.

### Poids de scoring hérités du prototype JS

`signature` **×2.5**, `content` ×1.0, `docstring` **×0.5** ; classe ×1.1 ;
`boostMinScore` 0.7 ; stratégie multi-champs `max_with_boost`.
**Ces valeurs n'apparaissent nulle part dans le code Rust.**

---

## 9. Préférences de travail (à respecter par toute reprise)

Consignées le 24 mars, toujours valables :

- **Docs en français, code en anglais.**
- **« lucivy est sa propre lib — ne jamais dire "fork de Tantivy" »**
- **« Pas de concessions — corriger les bugs, pas les rationaliser »**
- **Pas de mention de Claude dans les commits.**

À quoi s'ajoutent, tirées des docs de février : *« pas de nouveau fichier »*
quand une variante suffit, et **« API impossible à mal utiliser »** comme critère
de design (le handle opaque `i64` de `create()` en est l'exemple).

---

## 10. Ce que ça change pour la suite

Trois conclusions, par ordre d'importance.

**1. Le déséquilibre infrastructure / produit est le vrai sujet.** Depuis le 13
mars, six chantiers d'infrastructure ont été menés (dialect, SearchBackend,
BlobStore, luciole, FTS Rust, burn) et **zéro** des six chantiers produit de la
roadmap (documents réels, reranking, éval, multi-tenancy, streaming, YAML). Ce
n'est pas un reproche — l'infrastructure était nécessaire et elle est saine.
Mais si la question est « qu'est-ce qui rendrait rag3weaver meilleur pour
quelqu'un d'autre que nous », la réponse est écrite depuis mars et n'a jamais été
touchée.

**2. Deux dettes ont un coût qui croît avec le temps.** Le multi-tenancy aurait
dû entrer dans le trait backend « dès le design » ; il touchera maintenant les
50 méthodes du dialect. Et l'absence d'éval fait qu'aucun des chantiers qualité
(reranking, chunking, poids de fusion) ne peut être validé — il faudrait donc
l'éval **avant** le reranking, contrairement à l'ordre de la roadmap de mars.

**3. Le problème fondateur est résolu — reste à en tirer parti.** Chercher `->`,
`};`, `foo->bar` séparateurs inclus était l'obsession de février et la
justification de posséder le moteur plutôt que d'utiliser tantivy ou GIN.
**lucivy v3 le couvre.** La piste « sidecar byte-n-gram » évoquée en doc 12 §7
est donc caduque.

Ce qui reste n'est plus un trou moteur mais un travail d'exposition côté
rag3weaver : quels modes de requête publier, lesquels retirer (`BM25Mode::Parse`
notamment, cf. doc 07), et comment documenter la capacité pour qu'elle soit
utilisée. Un différenciateur que l'API n'expose pas n'existe pas pour
l'utilisateur.

*(Corrigé le 24 août : la première version de ce paragraphe décrivait le trou
séparateurs comme encore ouvert. Il l'était en v2, plus en v3.)*
