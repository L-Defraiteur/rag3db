# Doc 10 — Conflict Resolution : exploration des options

Date : 12 mars 2026

## Contexte

Après Phase 4 (API sync, doc 08) et l'observabilité dataflow (doc 09), le prochain chantier est la résolution de conflits dans le queue/drain unifié (doc 02, Phase 5).

Trois types de conflits possibles dans `PendingWork` :

| Conflit | Exemple |
|---------|---------|
| Delete + Update même UUID | `delete("abc")` puis `update("abc", {name: "new"})` |
| Deux updates même UUID, mêmes champs | `update("abc", {name: "A"})` puis `update("abc", {name: "B"})` |
| Deux updates même UUID, champs différents | `update("abc", {name: "A"})` puis `update("abc", {email: "x@y"})` |
| Delete + Create même UUID | `delete("abc")` puis `create({_uuid: "abc", ...})` |

## Comportement actuel (sans résolution)

Le graph exécute : **deletes → updates → inserts → links → KB**.

- **Delete + Update même UUID** : delete supprime l'entité, puis update tente un MATCH sur un UUID qui n'existe plus → no-op + warning `ctx.warn()`. Correct mais inutile.
- **Deux updates même UUID, champs différents** : groupés dans des UNWIND séparés (par sorted field keys), les deux s'appliquent. **Bug actuel** : `changed_uuids` est un `Vec` (pas HashSet) → le même UUID peut y apparaître deux fois → double chunk + double embed (GPU waste). **Fix indépendant** : dédupliquer `changed_uuids` avec un `HashSet` (1 ligne). Après fix, le seul coût résiduel est un double hash check (CPU négligeable).
- **Deux updates même UUID, mêmes champs** : dans le même UNWIND, Cypher MATCH+SET s'applique deux fois, la dernière valeur gagne. Même bug `changed_uuids` Vec. Après fix dedup, **correct sans gaspillage significatif**.
- **Delete + Create même UUID** : delete supprime, puis insert MERGE recrée. **Correct** (le graph topology gère déjà l'ordre).

## Conflit 1 : Delete + Update même UUID

**Consensus : delete gagne, update supprimé.**

Un UUID marqué pour suppression ne devrait pas être mis à jour. L'update est superflu.

### Où résoudre ?

| Niveau | Avantage | Inconvénient |
|--------|----------|--------------|
| **Queue level** (`PendingWork` ou `build_ingestion_graph()`) | Simple, O(n) scan avant construction du graph. Zéro impact sur les nœuds. | Logique métier hors des nœuds |
| **Node level** (inter-nœuds) | Chaque nœud gère ses propres conflits | Nécessite communication entre DeleteRecordNode et UpdateRecordNode (complexe) |

**Option retenue provisoirement** : queue level — construire un `HashSet<(entity_name, uuid)>` des deletes, filtrer les updates.

## Conflit 2 : Deux updates même UUID

C'est le cas le plus subtil.

### Scénario problématique

```
update("abc", {title: "Hello"})   → hash_1 = hash(all content fields with title changed)
update("abc", {email: "x@y.com"}) → hash_2 = hash(all content fields with email changed, title=old)
```

Si on merge les data et qu'on prend hash_2, ce hash a été calculé **sans connaître le changement de title**. Si `title` est un champ de contenu et `email` ne l'est pas, alors hash_2 == old_hash → on rate le re-chunk du title.

### Options explorées

#### Option A — "Last wins" (remplacement complet)

Garder seulement le dernier `UpdateRecord` pour chaque `(entity_name, uuid)`.

- **Problème** : on perd les champs du premier update. Si update 1 change `title` et update 2 change `email`, on perd le changement de `title`.
- **Verdict** : incorrect, écarté.

#### Option B — Merge data au queue level, hash sentinel

Fusionner les `data` BTreeMap (dernier champ gagne en cas de collision). Mettre `new_content_hash = ""` (sentinel) pour forcer `content_changed = true`.

```rust
// Pseudo-code
fn merge_updates(updates: &mut Vec<UpdateRecord>) {
    let mut seen: HashMap<(String, String), usize> = HashMap::new(); // (entity, uuid) → index
    let mut merged = Vec::new();
    for update in updates.drain(..) {
        let key = (update.entity_name.clone(), update.uuid.clone());
        if let Some(&idx) = seen.get(&key) {
            // Merge: extend data (last wins per field)
            merged[idx].data.extend(update.data);
            merged[idx].new_content_hash = String::new(); // force change
        } else {
            seen.insert(key, merged.len());
            merged.push(update);
        }
    }
    *updates = merged;
}
```

- **Avantage** : simple, un seul update par UUID, un seul re-chunk
- **Inconvénient** : force toujours un re-chunk même si aucun champ de contenu n'a changé (rare, acceptable)
- **Où** : queue level ou début de `UpdateRecordNode::execute()`

#### Option C — Merge data au node level, recalcul du hash

Merge dans `UpdateRecordNode::execute()`, puis recalculer `build_content_text()` sur les champs fusionnés.

- **Avantage** : hash précis, pas de re-chunk inutile
- **Inconvénient** : nécessite accès à `build_content_text()` + entity config dans le nœud. Plus complexe. Et `build_content_text()` a besoin des valeurs complètes de l'entité (pas juste les champs mis à jour), ce qui nécessiterait un read DB supplémentaire.
- **Verdict** : over-engineering pour un cas rare

#### Option D — Ne pas dédupliquer les updates (+ fix bug changed_uuids)

Laisser les UpdateRecords tels quels. Deux updates passent dans des UNWIND séparés, les deux s'appliquent. Corriger le bug `changed_uuids` Vec → HashSet (1 ligne) pour éviter le double rechunk/embed.

- **Avantage** : quasi zéro code (1 ligne de fix), comportement déjà correct
- **Inconvénient** : double hash check (CPU négligeable), double UNWIND SET (2 queries Cypher au lieu d'1)
- **Quand c'est problématique** : jamais en pratique. Le coût réel est 1 query Cypher supplémentaire.

#### Option E — Merge au node level, sentinel hash

Comme option B mais dans `UpdateRecordNode::execute()` au lieu du queue level.

- **Avantage** : la logique de merge reste dans le nœud (responsabilité claire), le nœud peut logger le merge via `ctx.info()`
- **Inconvénient** : même compromis sur le hash que option B

## Conflit 3 : Delete + Create même UUID

**Pas de conflit réel.** L'ordre d'exécution du graph (deletes → inserts) gère naturellement ce cas. Delete supprime l'ancien, Insert MERGE recrée le nouveau. Aucune résolution nécessaire.

## Incertitude : ordre UNWIND dans KuzuDB (rag3db)

Quand un UNWIND produit plusieurs rows qui MATCH+SET le **même nœud** (même UUID), on ne sait pas si Kuzu garantit un traitement séquentiel (dernière row gagne) ou s'il batche/parallélise en interne. C'est un fork de Kuzu v0.11.2.2, on n'a touché qu'aux extensions.

**Impact** : concerne uniquement le cas "deux updates, mêmes champs, même UUID" dans le même UNWIND. Pour des champs différents (groupes UNWIND séparés, exécutés séquentiellement par le nœud), pas d'ambiguïté.

**Conséquence** : argument supplémentaire pour merger les updates avant le UNWIND (options B/E). Après merge, un seul update par UUID → la question de l'ordre ne se pose plus.

## Questions ouvertes

1. **Où placer la résolution ?**
   - Queue level (PendingWork) : plus simple, centralisé
   - Node level (UpdateRecordNode) : meilleure séparation des responsabilités, accès à ctx.warn()/info()
   - Hybride : delete vs update au queue level, merge updates au node level

2. **Le sentinel hash (force re-chunk) est-il acceptable ?**
   - Coût : re-chunk + re-embed inutile sur les merges dont aucun champ de contenu n'a changé
   - Fréquence du cas : très rare (qui fait deux updates sur le même UUID dans le même drain ?)
   - Alternative : recalcul de hash (complexe, nécessite read DB)

3. **Faut-il logger les résolutions ?**
   - `ctx.info("merged 2 updates for UUID 'abc' into one")`
   - `ctx.info("update for UUID 'abc' dropped — entity scheduled for deletion")`
   - Si oui, ça pousse vers node level (accès à ctx)

4. **Faut-il exposer les résolutions dans le report ?**
   - Nouveau champ `conflicts_resolved: Vec<ConflictResolution>` dans ExecutionReport ?
   - Ou juste des NodeLog info suffisent ?

## Décisions prises

1. **Delete + Update même UUID** → delete gagne, update supprimé au queue level
2. **Updates dupliqués même UUID** → merge data BTreeMap au node level (option E), sentinel hash `""` pour forcer re-chunk. Couvre KB et simple uniformément. Optimisation content-field-aware possible plus tard.
3. **Delete + Create même UUID** → pas de résolution nécessaire (graph topology gère)
4. **Bug changed_uuids** → fix Vec → HashSet (indépendant du conflict resolution)

## Optimisation future possible

Check content fields au merge : ne forcer le re-chunk que si un champ de contenu a été touché par le merge. Nécessite accès à la config entity (différente KB vs simple). Pas prioritaire — le sentinel couvre tout correctement, le surcoût est négligeable (cas rare).

## Idées futures (pas prio)

- **`deleteKBEntry(title_uuid, cascade_content?)`** : helper qui cascade delete title → KB_Index → chunks, et optionnellement les content entities liées. Les content entities orphelines ne posent pas de problème (pas searchables, peuvent être re-liées à un autre title).
- **Content-field-aware merge** : ne forcer le re-chunk que si un champ de contenu a été touché par le merge (nécessite accès entity config, différent KB vs simple).

## État actuel

Implémenté et commité (`39269e380`). Ce doc capture l'exploration, les options envisagées, et les décisions.
