# L'architecture actuelle — ce qui a changé depuis 14h

À lire après [`../6-septembre-2026-11h43/02`](../6-septembre-2026-11h43/02-l-architecture-actuelle.md)
et son §11. Ici seulement ce que la soirée a déplacé.

## Le chemin d'indexation de code

```
FileSource ──analyze_source──▶ CodeAnalysis (files, scopes, relations, pending)
                │  own_texts : un scope = sa déclaration + son corps, enfants remplacés par leur signature
                ▼
Catalog::ingest_code
   ├─ ingest_entities(File, Scope, Symbol)      ChunkStrategy::Lines 30 / 1 500 / 5
   │     └─ EmbedNode : embed_pipeline (2 producteurs, 2 lots d'avance) → écriture des vecteurs
   ├─ link_jusqu_a par relation → drain → LinkRecordNode
   │     └─ ≥ 2 000 arêtes étiquetées : CSV + COPY (dédoublonné, existants écartés, bouts absents comptés)
   │        sinon UNWIND … MERGE par tranches de 5 000
   └─ resolve_across_batches : Symbol, DEFINES / MENTIONS (rendez-vous), arêtes résolues
```

- **Le contenu d'un scope est son texte propre** (`code.rs::own_texts`). Le
  catalogue ne sait toujours pas ce qu'est du code ; `content` reste le champ
  de contenu, `signature` n'en est plus un (c'est la première ligne).
- **Les lots** : `embedder::lot_budget(budget_conseille, défaut)` →
  `LotBudget { max_items, max_chars, max_area, stable }` ;
  `stable_batches` ; l'arrondi aux demi-pas seulement avec un conseil.
- **Le chargement en masse est explicite** : `Catalog::bulk_vector_index`
  (doc 18) autour de `ingest_code`, appelé par la mesure, pas par
  `ingest_code`. Le COPY des liens, lui, se décide au volume dans le nœud.

## La vue par parent

`EntityConfig.group_by { relation, frame_field }` → `GroupFrameNode` dans
`search.mmd` → trois clés réservées dans `data` (`_frame_uuid`,
`_frame_title`, `_frame`) posées par `ComposeNode` → le rendu regroupe par
arête et écrit `┌ \`impl X {\`` sous l'en-tête. `Scope` déclare
`HAS_PARENT` / `signature`. Le parent d'une méthode par cette relation est le
scope nommé comme son impl (l'enum ou la struct), résolu par nom dans
codeparsers.

## Le catalogue de gabarits

`TemplateRef { origin, derived_from, … }`, `TemplateRoot`, `roots(project)`,
`scan_roots`, `read_in`, `write_entity`, `Catalog::sync_templates` →
`TemplateSync { added, updated, removed, unchanged, stale }`.
`agent::mount_agent_services(catalog, project)` et `_on(catalog, source)` :
services de recherche, catalogue, gabarits synchronisés, source, porte
(`Garde::new(Mode::Auto)`).

## Le modèle et le démon

- `_catalog_meta.embedding_model = nom:dim` ; `Catalog::check_embedding_model`
  à l'ingestion, au rattrapage, à la requête.
- Le démon porte l'empreinte de son exécutable et `lot_conseille` dans son
  `Identite` ; `POST /quitter` ; le harnais remplace un démon d'une autre
  construction ; il chauffe seize classes de forme avant d'écouter.
- Régime : `confort` bride seulement si la carte est partagée
  (`Regime::carte_partagee`) ; la carte des modèles est celle **au moins
  d'écrans actifs** (`least_watched_card`).
- Modèles : BGE-M3 (32 × 512), granite-107m (128 × 512, dim 384),
  granite-278m (768) — `RAG3WEAVER_EMBED_MODEL` côté démon.

## Ce qui reste faux ou provisoire

- `ingest_code` n'appelle pas le chargement en masse : une première
  ingestion par un agent paie encore l'index HNSW ligne à ligne.
- Les nœuds (scopes, chunks, symboles) s'insèrent par UNWIND, 8 000
  lignes/s ; les vecteurs se posent par un `SET` après coup.
- L'incrémental re-parse tout et ne saute que le travail dérivé.
- 207 000 rendez-vous MENTIONS pour 1 642 fichiers : c'est la conception
  du rendez-vous, pas un doublon — à revoir si la taille de l'index gêne.
- `Catalog::search` (417 lignes) vit encore, à côté de `Catalog::rechercher`.
