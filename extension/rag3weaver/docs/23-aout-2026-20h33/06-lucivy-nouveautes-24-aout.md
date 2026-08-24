# lucivy : nouveautés du 24 août à prendre en compte

Suite de la passation `04-migration-fts-lucivy-v3-rust.md` (déjà annotée).
Trois évolutions côté lucivy (branche `v3-recovery`) changent ce que vous avez
à écrire — toutes dans le sens « moins de code chez vous ».

## 1. `_node_id` : le piège n'existe plus

`add_document(doc, offset)` (et `add_documents`, `add_document_with_hashes`)
**estampille lui-même** le champ `_node_id` avec l'offset passé. Un document
qui porte déjà un id différent est **refusé** avec un message explicite (c'est
un bug appelant, plus une corruption silencieuse). Ne mettez plus
`doc.add_u64(nid_f, offset)` dans votre code — c'est toléré si la valeur est
la même, mais inutile.

## 2. `add_document_json` : champs par nom, erreurs qui parlent

```rust
handle.add_document_json(offset, &serde_json::json!({
    "_title": title, "_content": content, "_source_entity": src
}))?;
```
Types vérifiés par le schéma (`text`→string, `u64`→nombre, …). Nom inconnu →
l'erreur **liste les champs du schéma**. C'est le mapping que chaque binding
réimplémentait ; utilisez-le plutôt que de faire les lookups `Field` à la main.

## 3. Config de schéma : les typos échouent fort, la réouverture survit

- `SchemaConfig`/`FieldDef` refusent les **clés inconnues** à la
  désérialisation : `"shard": 4` (au lieu de `"shards"`) échoue en nommant les
  clés valides — avant, c'était ignoré et l'index partait sur 1 shard sans un
  mot. Idem `"storde"`, `"sfx_versoin"`, etc.
- Valeurs vérifiées à la création : type de champ inconnu (liste les cinq
  valides : text, string, u64, i64, f64), nom `_node_id` réservé, doublons,
  `fields` vide, `shards: 0`, `balance_weight` hors [0,1], `sfx_version` ∉ {2,3}.
- Un `_config.json` **stocké** par une autre version de la struct reste
  rouvrable (`SchemaConfig::from_stored_json`, utilisé par les chemins open) —
  la strictesse ne s'applique qu'aux configs fournies par l'appelant.

## 4. Rappel des nouveautés déjà annotées dans le 04

- **Lazy loading optionnel** : `.with_load_mode(BlobLoadMode::Lazy)` sur
  `BlobShardStorage`. Pour le plein effet, implémentez sur vos deux stores
  `blob_len` (`LENGTH(_data)` / `SIZE(b._data)`) et `load_range`
  (`SUBSTRING(_data FROM $1+1 FOR $2)`) — méthodes à défaut du trait, rien ne
  casse sans elles. Benchmarkez Eager (défaut) vs Lazy sur vos vrais index.
- Les imports morts de `lucistore` qui cassaient votre `-D warnings` sont
  corrigés (`b09667e`).
- Nuance importante sur votre lecture de lucistore : LUCIDS ne « répare » pas
  le téléchargement complet de `BlobDirectory` — ce sont **deux topologies** :
  (a) blob store source de vérité + cache jetable (c'est `BlobShardStorage`,
  maintenant avec l'option Lazy), (b) copie locale durable synchronisée par
  deltas LUCIDS (`shard_versions` → `export_sharded_delta` →
  `apply_sharded_delta`) — la bonne base pour le WASM offline. Le choix entre
  les deux est une décision d'archi chez vous. Et LUCID/LUCIDS ont été
  **réparés le 23 août** (chaque delta renvoyait l'index entier : uuid avec
  tirets vs sans ; fichiers `.del` jamais envoyés ; writer périmé après
  apply) — validés spans-exactes de bout en bout depuis.

## 5. Réponses à votre journal (05)

- **`DynBlobStore` peut disparaître** : `lucistore` implémente maintenant
  `BlobStore` pour `Arc<T>` (dont `Arc<dyn BlobStore>`), toutes méthodes
  transmises y compris `blob_len`/`load_range`. `BlobShardStorage<Arc<dyn
  BlobStore>>` marche tel quel (`f7dd5c2`).
- **`build_document` : le `_node_id` manuel est superflu** depuis
  l'estampillage automatique (§1). Le garder est toléré (même valeur =
  accepté), mais autant le retirer — et surtout ne pas le compter comme
  invariant à maintenir.
- Votre `[patch.crates-io]` est la bonne approche en attendant la publication
  2.1.0 (chantier côté lucivy).

## Commits concernés (repo lucivy, branche v3-recovery)

`4fa729e` lazy loading + load_range/blob_len · `ce03ac6` node_id estampillé +
add_document_json · `32ca1dc` erreurs de config · `f7dd5c2` Arc<dyn BlobStore> · `b09667e` warnings lucistore ·
`5a05e4e` test ACID v3 MemBlobStore (le modèle à copier pour vos tests).
