# 03 — `codeparsers` intégré, et deux bugs de fond trouvés en chemin

25 août 2026, soir. Le chantier prévu en [02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md)
§6, première étape : le code entre dans le graphe. Et ce qu'on a trouvé en
ingérant **notre propre code** — deux bugs que rien d'autre n'aurait révélés,
parce qu'aucun E2E ne dépassait quelques centaines de lignes.

## 1. Ce qui est livré

**Le crate réparé** (commit `30c0fec67`) : `ScopeInfo.scope_start_byte /
scope_end_byte` (dérivés des lignes après extraction — `File` devient la
source des positions, `read` d'un scope est une tranche) ; `content_hash`
enfin rempli (blake3, le même que les UUID) ; `RelationshipResolutionResult.files`
et `.external_libraries` enfin remplies (un commentaire avouait qu'elles ne
l'étaient jamais) ; extension inconnue ignorée et signalée au lieu d'être
parsée comme du TypeScript ; l'exemple `compare.rs` recompilé.

Puis, révélé par nos fichiers : **`&first_line[..77]` paniquait au milieu
d'un `─`** — nos séparateurs `// ─── … ───` faisaient échouer le parsing de
chaque fichier qui en a. Sept troncatures par index d'octet remplacées par
`utils::text::{truncate_at_char_boundary, ellipsize}`.

**Le module `code`** (feature `code`, `src/code.rs`) :

- `register_code_schema` — `File` / `Scope` / `Library` et les neuf relations
  du `CODE_SCHEMA` de février. **`hashsafe`** ajouté à `EntityConfig` (et
  `Catalog::entity_uuid` exposé) : `File` par `path`, `Scope` par `key` (l'uuid
  déterministe de `codeparsers`), `Library` par `name` — l'identité survit au
  changement de contenu, la ré-ingestion met à jour au lieu de dupliquer.
  `File` a pour seul contenu son chemin : cherchable par nom, jamais chunké
  au sens du texte. `Scope` : `name` titre ; `signature`, `content`,
  `docstring` contenus ; lignes, octets, `file_path`, `parent_name`.
- `analyze(root, sources)` — sans accès disque (c'est l'appelant qui lit :
  arbre, commit, fixture), rend une `CodeAnalysis` plate et sérialisable ;
  `read_sources(root)` pour l'arbre de travail.
- `Catalog::ingest_code` — `ingest_entities` × 3, `link` par relation
  (uuid dérivés des clés, comme le catalogue les dérive), `drain`.
- Nœuds `ParseCodeNode` (`sources` ou `root`) → `CodeIngestNode`, port
  `PortType::Code`. 31 types de nœuds avec la feature.

## 2. Mesuré, sur notre propre `src/dataflow/`

- Le module entier : **25 fichiers, 1 402 scopes, 66 771 relations**, parse
  1,0 s, résolution 0,18 s (test unitaire ignoré `analyze_own_dataflow_dir`).
- `e2e_code`, borné à cinq fichiers (§3) : 252 scopes, 4 639 relations, 1,6 s
  d'ingestion. Le fichier `generic_search_nodes.rs` retrouvé par son nom ; le
  scope `take_results` par sa signature ; **`take_results CONSUMED_BY` →
  `execute`** (celui de `FuseResultsNode`) — l'agent a un graphe à mordre.
  Ré-ingestion : mêmes comptes, **3 → 3** résultats.

**66 771 relations pour 1 402 scopes**, c'est 47 par scope : le repli « par
nom global » du résolveur relie tout `execute` à tous les `execute`. Ce n'est
pas faux au sens du parseur, c'est du bruit — et c'est exactement ce que la
résolution **contre la base** ([02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md) §4)
devra filtrer (par fichier, par import, par type). Dette nommée — **et réglée
pour la précision le soir même** : 9 645 relations, 8 par scope, mêmes cibles
([04](04-attribution-des-references-le-graphe-divise-par-sept.md)).

## 3. Bug 1 — l'UPDATE de l'index HNSW segfaute au-delà de ~512 lignes

> **Corrigé le soir même** — deux défauts dans l'extension (état de
> suppression partageant ses vecteurs avec l'insertion ; vecteur en attente
> relu dans la table avant d'y être écrit) et un hors-bornes dans le cœur.
> Récit et mesures : `docs/25-aout-2026-20h30/01` à la racine du dépôt.
> Après correctif : UPDATE à 4 096, double ré-ingestion, `e2e_code` sur le
> module entier (1 402 scopes). Le texte qui suit est l'état d'avant.

`e2e_code` sur le module entier : SIGSEGV. Trace gdb :
`OnDiskHNSWIndex::update` → `insertInternal` → `insertToLayer` →
`createRels` → **`shrinkForNode`** → `computeDistance` → `simsimd_cos_f32`.

Isolé par la sonde `e2e_hnsw_scale`, sans rien de `codeparsers` :

| chemin | code | 256 | 512 | 768 | 1 024 | 4 096 |
|---|---|---|---|---|---|---|
| `CREATE (:V {emb: […]})` — insertion avec embedding | amont kuzu | ✓ | ✓ | ✓ | ✓ | ✓ |
| `CREATE` puis `SET v.emb = […]` — **UPDATE** | **fork, `98e35566a`** (fév. 2026) | ✓ | ✓ | **SIGSEGV** | SIGSEGV | — |

Écarté : les vecteurs nuls de `MockEmbedder` (même crash avec `HashEmbedder`,
vecteurs unitaires pseudo-aléatoires — ajouté à `embedder.rs`, à utiliser
dès qu'un test ingère plus qu'une poignée de lignes) ; la réservation
d'espace d'adressage du jour (même crash à 8 TiB) ; une extension `.so`
désynchronisée (reconstruite).

**C'est le chemin de toute notre ingestion** : `InsertRecordNode` insère les
chunks sans embedding, `EmbedNode` fait `SET`. Et c'est le seul chemin
possible pour une ré-ingestion (une ligne existante change d'embedding). Donc
réordonner le graphe pour insérer avec l'embedding ne suffirait pas : **le
bug C++ est à corriger.** Pistes : `getEmbeddings` (multiple) ne distingue
pas source `UNCOMMITTED` / `COMMITTED` comme `getEmbedding` (simple) le fait ;
les `EmbeddingHandle` gardés dans `nbrs` pendant `shrinkForNode` pointent
dans un tampon de scan réutilisé. Un build **Debug** de l'extension (les
`KU_ASSERT` de `checkEmbeddingValidity` sont là) est la première chose à
faire. En attendant : `e2e_code` borné à cinq fichiers, sondes derrière
`RAG3DB_PROBE_HNSW=1` (elles tuent le processus de test).

## 4. Bug 2 — ré-ingérer doublait les documents plein-texte

`InsertRecordNode` fait `MERGE` (idempotent en base) puis `index_document`
— et `add_document` de lucivy **n'est pas un merge**. Chaque ré-ingestion
ajoutait un second document par ligne, et chaque recherche rendait la ligne
deux fois (3 → 6 résultats). Personne ne l'avait vu : aucun E2E ne
ré-ingérait la même ligne.

Correctif : `fts_handle::upsert_document` — si le routeur de lucivy connaît
l'identifiant (`shard_for_node_id`), suppression puis ré-ajout ; sinon ajout.
(Un `delete_by_node_id` sur un identifiant inconnu diffuse une suppression à
tous les shards : trop cher pour le cas courant, une ligne neuve.) Les
suites `search` 38, `idempotent_registration` 22, `generic_search` 12,
`simple_entity` 15 : inchangées.

## 5. Ce qui reste

1. ~~**Le bug HNSW UPDATE**~~ — corrigé (`docs/25-aout-2026-20h30/01`).
2. **La résolution contre la base** — la précision est faite
   ([04](04-attribution-des-references-le-graphe-divise-par-sept.md)) ;
   reste l'incrémentalité : le mapping global en mémoire à remplacer par
   la base.
3. `FileSource` (`GitRef`, `WorkingTree`), `CodeSyncNode`, le curseur de
   `File`.
4. Le résumé « conteneur » de février (signature + membres) n'est pas porté ;
   `content_dedented` est pris tel quel.
5. `grep` / `read` sur `File`, avec comparaison du hash.
