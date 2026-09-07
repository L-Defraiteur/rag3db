# Replier les bases de connaissances en entités dérivées par gabarit

**7 septembre 2026, 21h30.** Doc de conception, avant le code, au go de
Lucie (« oui replier KB en entités dérivées par gabarit », premier chantier
de son ordre du jour). Repose sur une cartographie complète du pipeline KB
(agent, 7 septembre) et sur le constat de la veille : une KB est un second
pipeline (`KBGather`, `KBUpdate`, `KBChunk`, `KBEmbed`), jumeau du chemin
générique, qui prend chaque amélioration en retard ou jamais — pas de
court-circuit de l'inchangé, pas de chemin de masse, une dette d'agrégat
hors base (C5 bis du [bilan](../6-septembre-2026-11h43/05-bilan-de-la-passe-fondations.md)).

## 1. Ce qu'une KB est aujourd'hui, exactement

- **Pas un objet de config.** `KBConfig` ne porte que des réglages de
  recherche et de découpe (`signals`, `fusion_strategy`, `keyword_weight`,
  `sparse_weight`, `rrf_k`, `chunking`) et trois champs morts
  (`title_boost`, `content_boost`, `special_ops`). La structure — quelle
  entité donne le titre, quelles entités et quels champs donnent le contenu —
  est **inférée** en balayant `title_for` / `content_for` sur les champs des
  entités (`resolve_entity_kbs`, `resolve_kb_title_entities`, `KBMetadata`).
- **Un texte composé sans gabarit.** Le document indexé est
  `titre + "\n" + sources jointes par "\n"`, sources triées par
  `(entité, uuid, champ)`, écrit à deux endroits qui doivent rester
  d'accord (`mettre_en_file_la_creation` et `KBGatherNode::gather_batch`).
- **Une matérialisation à part** : `{KB}_Index` (`_title`, `_content`,
  `_content_hash`, `_source_entity`, `_source_uuid`, `{kb}_embedding`),
  `{KB}_Index_Chunk` (avec `_kb_name`, `_source_field`, `_source_entity`,
  `_source_uuid`, `_content_offset`), `{KB}_Index_HAS_CHUNK`,
  `{Titre}_IN_{KB}`, un `{Entité}_SOURCED_{KB}` par entité contributrice, un
  HNSW `{KB}_Index_Chunk_vec`.
- **Un pipeline à part** : `gather_kb → update_kb → chunk_kb → agg_inserts →
  agg_links → agg_embeds → flush_fts`, alimenté par `pending.aggregates`
  (`AggregateRecord`) que poussent `create`, `link`, `UpdateRecordNode`,
  `DeleteRecordNode`, et par un second drain dans `ingest_entities_jusqu_a`
  qui fabrique des `UpdateRecord` à hash vide pour forcer le recalcul. Le
  rattrapage d'embarquement a sa propre branche (`KBEmbedNode("rattrapage")`).
- **Ce qu'elle apporte de vrai** : un document composé de plusieurs entités
  liées, embarqué comme un tout ; la recherche qui rend la source
  (`SourceResolved`, `_source_uuid`) et dédoublonne ; le filtre indirect par
  l'entité titre ; des poids de fusion par KB.

## 2. Le concept cible : l'entité dérivée

Une entité comme les autres, déclarée dans le schéma, dont les lignes ne
sont pas ingérées mais **rendues** depuis une entité racine et ses voisines
par un gabarit Jinja. Tout ce qui suit la ligne — insertion, découpe,
embarquement, plein texte, chemin de masse, court-circuit de l'inchangé,
checkpoints, vue par parent — est le chemin générique, sans un nœud de plus.

```rust
pub struct EntityConfig {
    …
    /// Cette entité est rendue depuis une autre, pas ingérée.
    pub derived: Option<DerivedConfig>,
    /// Les poids de fusion par défaut pour une recherche sur cette entité
    /// (ce que `KBConfig` portait). Précédence : appelant > entité > défaut.
    pub fusion: Option<FusionConfig>,
}

pub struct DerivedConfig {
    /// L'entité racine : une ligne dérivée par ligne racine.
    pub from: String,
    /// Les voisines rassemblées, par relation — la même forme que `GroupBy`.
    pub gather: Vec<GatherRule>,
    /// Un gabarit Jinja par champ rendu : `title`, `content`, …
    pub render: BTreeMap<String, String>,
}

pub struct GatherRule {
    /// Le nom sous lequel les lignes rassemblées arrivent au gabarit.
    pub name: String,
    /// La relation qui mène de la racine aux voisines (ou l'inverse).
    pub relation: String,
    pub direction: Direction,
    /// Les champs relus sur chaque voisine.
    pub fields: Vec<String>,
}
```

Contexte du gabarit : `root` (les champs de la racine) et une liste par
`GatherRule.name`, triée de façon stable `(uuid)`. Exemple, un ticket et
son fil :

```json
"TicketView": {
  "fields": { "title": { "type": "string", "is_title": true },
              "content": { "type": "text", "is_content": true } },
  "signals": "hybrid",
  "derived": {
    "from": "Ticket",
    "gather": [ { "name": "comments", "relation": "HAS_COMMENT", "direction": "out", "fields": ["author", "body"] } ],
    "render": {
      "title": "{{ root.subject }}",
      "content": "{{ root.body }}\n{% for c in comments %}{{ c.author }} : {{ c.body }}\n{% endfor %}"
    }
  }
}
```

Colonnes posées d'office sur une entité dérivée, en plus de ses champs :
`_source_entity`, `_source_uuid` (ce que la recherche rend), et
`_render_hash` (le hash des entrées au dernier rendu ; `''` veut dire « à
re-rendre », c'est la dette d'agrégat en base, C5 bis fait dès le départ).
Identité : `hashsafe = ["_source_uuid"]` — une ligne dérivée par racine, uuid
déterministe. Relation générée comme `_CHUNKED_FROM` : `{Derived}_DERIVED_FROM`
(dérivée → racine). Les voisines n'ont pas de relation stockée : on les
retrouve par les règles `gather`, au rendu comme à l'invalidation.

Généricité : rien ici ne sait ce qu'est un titre ou un commentaire ; c'est
la consigne du 6 septembre — une organisation nouvelle se décrit dans
`EntityConfig`, la découpe et les relations, jamais en dur pour un cas.

## 3. Le pipeline

Un seul nœud nouveau, **`DeriveNode`** : entrée `derivations`
(`BatchPayload<Derivation { entity, root_uuid }>`), sortie `entities`
(`EntityRecord` de l'entité dérivée, champs rendus, `_render_hash` posé). Il
lit la racine et les voisines (deux requêtes par règle, par UNWIND, comme
`fetch_related`), rend chaque champ avec `render_nodes::rendre`, et **saute
les lignes dont le hash des entrées n'a pas bougé** (relecture de
`_render_hash`, une requête). Puis la chaîne de toujours :

```
derive → insert → chunk → chunk_insert → chunk_link → marquer → embed → flush_fts
```

c'est exactement le graphe d'`ingest_entities_jusqu_a`, chemin de masse
compris quand la table dérivée est vide. Dans `build_ingestion_graph`, la
branche KB (`gather_kb … agg_embeds`) devient `derive → <chaîne simple>` sur
`pending.derivations`, et `templates/kb_pipeline.mmd` disparaît.

**Ce qui pousse une dérivation** (`Derivation`, remplace `AggregateRecord`),
par une seule fonction `Catalog::derivations_touchees(entity, uuid) ->
Vec<Derivation>` qui regarde la config : les entités dérivées dont `from`
est cette entité (racine changée), et celles dont une règle `gather` vise
cette entité (voisine changée : la racine se retrouve par la relation, une
requête). Appelée par `create`, `update`, `delete`, `link`, et par
`ingest_entities_jusqu_a` — qui n'a plus besoin de fabriquer des
`UpdateRecord` à hash vide ni d'un second drain.

**La dette en base.** Quand une source change, on pose `_render_hash = ''`
sur les lignes dérivées touchées **au niveau donnée** (une requête par
entité dérivée, comme `_chunked_hash`), et on met la dérivation en file. Si
le processus meurt avant le drain, le rattrapage
`Catalog::rendre_le_retard` sélectionne `WHERE _render_hash = ''` et
relance `DeriveNode` dessus — le patron de C4/C5, `reclamer_*`. La marque
d'écriture déclare la dette sur la table dérivée (`PLEIN_TEXTE | DENSE |
SPARSE`) comme aujourd'hui sur `{kb}_Index`.

**Suppression de la racine** : la ligne dérivée et ses chunks partent avec
elle, par `_DERIVED_FROM`, dans `DeleteRecordNode` — générique, comme les
chunks.

## 4. La recherche

`resolve_search_target` pour une entité dérivée = la cible d'une entité
simple, plus : `has_source_refs: true`, `filter_indirection: Some((from,
"{Derived}_DERIVED_FROM"))`, `default_fusion` depuis `EntityConfig.fusion`.
`resolve_to_source_entities` ne change pas (il lit `_source_entity` /
`_source_uuid`) ; `SourceResolved` et le dédoublonnage par source tiennent.
Le lanceur `Catalog::rechercher` et `search_base.mmd` ne bougent pas — et le
`FuseResultsNode` du gabarit d'outil, qui fige `bm25:0.6,vector:0.4`, prend
enfin les poids de l'entité (B4 finit ici).

## 5. Ce qu'on perd, et pourquoi c'est accepté

- **L'attribution des chunks à leur source** (`_source_field`,
  `_SOURCED_`, l'arithmétique `_content_offset` par source). Le contenu
  rendu est un texte ; les chunks et les surlignages portent sur lui, comme
  pour toute entité ; le contrat offset → chunk tient contre le champ
  `content`. Savoir de quel commentaire vient un chunk reviendra, si on le
  veut, par une carte des positions que le rendu peut produire (le gabarit
  sait où il a mis chaque voisine). Pas dans ce chantier.
- **Les noms de tables `{KB}_Index` / `{KB}_Index_Chunk`.** Une entité
  dérivée `k` a les tables `k` et `k_Chunk`. Les bases existantes qui ont des
  KB sont **migrées** (schéma v6) : l'ancienne KB devient une entité dérivée
  traduite (§6), ses tables sont recréées et ses lignes re-rendues puis
  ré-embarquées au premier drain. C'est un ré-embarquement complet des KB,
  une fois. Sur nos bases c'est acceptable ; c'est dit dans la migration.

## 6. La compatibilité : `knowledge_bases` devient une traduction

`CatalogConfig.knowledge_bases` reste lisible. Au chargement,
`traduire_les_kb(config) -> config` fabrique pour chaque KB une entité
dérivée : `from` = l'entité titre (celle qui porte `title_for`), une règle
`gather` par entité contributrice avec la relation trouvée comme aujourd'hui
(`find_relation_to_entity`), `render.title = "{{ root.<title_field> }}"`,
`render.content` = les champs de contenu joints par `"\n"` dans l'ordre
`(entité, uuid, champ)` — le même texte qu'aujourd'hui, à la lettre ; les
poids de fusion vont dans `fusion`. `title_for` / `content_for` sur les
champs restent comme sucre et produisent la même traduction. Ce qui est
retiré : `KBConfig` comme type public (déprécié, traduit), `KBMetadata`, les
quatre nœuds KB et `KBChunkRecordNode` (jamais câblé), `AggregateRecord`,
`kb_pipeline.mmd`, les DDL `_Index*`, `_IN_`, `_SOURCED_`, la branche
`KBEmbedNode` du rattrapage.

## 7. Les gabarits, où ils vivent

`render` accepte ce que `resolve_template` accepte déjà : un nom
(`templates/render/<nom>.jinja`, racine `RAG3WEAVER_RENDER_TEMPLATES`) ou la
source Jinja en ligne. Une entité dérivée livrée avec le produit (un
« gabarit d'entité » du catalogue, `templates/entities/*.json`) porte son
`derived` dans son JSON : c'est la couche utilisateur du catalogue de
gabarits ([doc du 6 septembre](../6-septembre-2026-14h43/01-la-couche-utilisateur-du-catalogue-de-gabarits.md))
qui rend les KB d'un projet visibles et adoptables. C'est là que « ça aurait
dû être un gabarit » devient vrai.

## 8. Les tests

Le filet : `e2e_phase0b` (14 tests, tous KB), `e2e_idempotent_registration`
(10 tests KB : ordre d'enregistrement, migration, multi-entités, cascade de
suppression, sessions successives), `e2e_result_mode` (10), la moitié KB
d'`e2e_highlight_long_text`, `e2e_search` (39, sur `make_kb_config`),
`e2e_undo::undo_delete_kb_bgem3`. Ils passent par la traduction §6 et
doivent rester verts **à l'identique** sauf : les noms de tables (`k` au lieu
de `k_Index`), et les assertions d'attribution par source (`_source_field`,
`_SOURCED_`), qui sont réécrites ou retirées en le disant. Nouveaux tests :
une entité dérivée déclarée directement (`derived`), le rendu par gabarit
nommé, la dette `_render_hash` et son rattrapage après un processus tué,
la première ingestion en masse d'une entité dérivée.

## 9. Les pas, et l'estimation

| pas | contenu | durée |
|---|---|---|
| A | `DerivedConfig`, `fusion` sur `EntityConfig`, schéma (colonnes, `_DERIVED_FROM`, v6), `DeriveNode`, `pending.derivations` et `derivations_touchees`, la branche dans `build_ingestion_graph`, `ingest_entities_jusqu_a` sans second drain, cible de recherche, traduction des KB, retrait des nœuds KB, tests migrés | deux jours |
| B | la dette `_render_hash` au niveau donnée et `rendre_le_retard` | une demi-journée |
| C | `FuseResultsNode` sur les poids de l'entité ; les gabarits d'entités dérivées dans le catalogue ; doc de fin | une demi-journée |

## 10. Trois décisions prises faute d'autre, à contredire si besoin

1. **Un gabarit par champ rendu** (`render: {title, content}`) plutôt qu'un
   seul gabarit qui rend tout — parce que le titre et le contenu sont des
   champs ordinaires ensuite (`is_title`, `is_content`), et que le plein
   texte les indexe séparément.
2. **Les voisines sans relation stockée** : retrouvées par les règles
   `gather` à chaque rendu et à chaque invalidation, pas de `_SOURCED_`. Une
   requête de plus à l'invalidation d'une voisine, contre un schéma sans
   relations dérivées.
3. **Le ré-embarquement des KB existantes à la migration**, plutôt qu'une
   conversion de tables qui garderait les vecteurs : la conversion coûterait
   plus de code que le chantier, pour des bases qui sont les nôtres.
