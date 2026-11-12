Voici les constats issus de la revue statique du code de la tranche A (`facts.rs`, `backend_schema.rs`, `plan.rs`, `transfer.rs` et `migrations.rs`).

---

### Constat 1 — `canonical_json` ne canonicalise pas les clés des sous-objets inclus dans les valeurs dynamiques (`serde_json::Value`)

* **Classification :** Défaut démontré.
* **Localisation :** `extension/rag3weaver/src/dataflow/migrations/facts.rs`, lignes 224–234.
* **Description :**
  La fonction `canonical_json` exécute :
  ```rust
  serde_json::to_value(value).map(canonical).and_then(|v| serde_json::to_string(&v))
  ```
  Or, lorsque `canonical` traite un `serde_json::Value::Object(m)`, elle trie bien l'objet courant via `BTreeMap`, mais `to_value(value)` a déjà converti les structures sérialisées. Si un type possède un champ typé `serde_json::Value` (notamment `RelationDef.properties[k].default_value`, ou les définitions de champs non typées), ce