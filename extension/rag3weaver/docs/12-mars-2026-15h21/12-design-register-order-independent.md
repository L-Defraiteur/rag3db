# Doc 12 — Design : register_entity / register_kb order-independent

Date : 12 mars 2026

Réf : doc 07, doc 09, doc 11

## Problème

`register_entity()` exige `is_content=true` sur au moins un champ. Une entité
KB-only (champs `content_for`/`title_for` pointant vers un KB name) n'a pas
de `is_content` → rejetée.

De plus, l'ordre d'appel compte :
- `register_entity` avant `register_kb` → OK (register_kb scanne les entities)
- `register_entity` APRÈS `register_kb` (migration, ajout de champ content_for) → la KB ne sait pas qu'un nouveau champ la concerne

## Design retenu

**Principe : peu importe l'ordre, le résultat final est le même.**

### register_entity()

1. Validation relaxée : accepte si l'entité a `is_content` (simple pipeline) **OU** `content_for`/`title_for` (KB participation). Ni l'un ni l'autre → erreur.
2. Crée la table entité (toujours).
3. Crée chunk table + FTS + vector + sparse **seulement** si `has_simple_pipeline()` (= a des `is_content` fields).
4. **Nouveau** : après persist, scanne les `content_for`/`title_for` du config. Pour chaque KB mentionnée qui existe déjà dans `self.kb_metadata` → re-appelle `register_kb(kb_name, existing_config)`. Comme `register_kb` est idempotent, il met à jour les content_refs et reconstruit le FTS KB si nécessaire.

### register_kb()

1. Scanne `self.config.entities` (inclut les entities enregistrées via `register_entity`) pour trouver les `title_for`/`content_for` pointant vers cette KB.
2. Si la KB existe déjà → met à jour `KBMetadata` (content_refs, entities set). Si les content fields ont changé → rebuild FTS sur `{KB}_Index`.
3. Si la KB est nouvelle → `create_kb_tables()` comme avant.

### Conséquence sur search

- Entité avec simple pipeline → `search("EntityName", ...)` fonctionne (chunk table existe)
- Entité KB-only → `search("EntityName", ...)` retourne une erreur claire. Chercher via `search("kb_name", ...)`.
- Entité mixte (simple + KB) → les deux fonctionnent

### migrate_entity() ajustements

- FTS rebuild : seulement si `has_simple_pipeline()` sur la nouvelle config
- Content changed detection : inchangé (vérifie déjà `content_for`/`title_for`)
- Re-trigger des KBs : délégué à `register_entity()` après la migration

## Réflexion future : entité composite (simple + KB)

Une entité pourrait avoir les deux types de champs **sans conflit** :

```
fields:
  name:        { is_title: true }                    # → simple pipeline
  description: { is_content: true }                  # → simple pipeline
  body:        { content_for: ["docs"] }             # → KB "docs"
  heading:     { title_for: "docs" }                 # → KB "docs"
```

Techniquement ça devrait marcher :
- `is_title`/`is_content` sont mutuellement exclusifs avec `title_for`/`content_for` **par champ**, pas par entité
- La table entité a tous les champs
- Le simple pipeline crée `{Entity}_Chunk` avec les champs `is_content`
- La KB crée `{KB}_Index` / `{KB}_Index_Chunk` avec les champs `content_for`
- `search("Entity", ...)` cherche dans le simple pipeline
- `search("docs", ...)` cherche dans la KB
- `ingest_entities("Entity", data)` → UpdateRecordNode enqueue dans les deux pipelines (simple rechunk + KB aggregate)

**Pas de conflit en théorie.** Mais il faudra vérifier :
- Que `ingest_entities` gère bien le dual pipeline (UpdateRecordNode produit rechunk_entities ET AggregateRecords)
- Que `reindex` fonctionne pour les deux pipelines simultanément
- Que les hash de contenu sont indépendants (simple: `_content_hash` sur entity, KB: `_content_hash` sur `{KB}_Index`)

→ À valider avec des tests E2E quand on y arrivera. Pour l'instant on implémente le order-independent sans le cas composite.

## Ce qu'on implémente maintenant

1. `EntityConfig::has_simple_pipeline()` + `has_kb_participation()` (helpers)
2. `register_entity()` : validation relaxée + skip chunk tables si KB-only + re-trigger KBs existantes
3. `register_kb()` : mise à jour KBMetadata quand re-appelée (pas juste no-op)
4. `resolve_search_target()` : erreur claire pour KB-only entities
5. `migrate_entity()` : skip FTS rebuild si pas de simple pipeline
