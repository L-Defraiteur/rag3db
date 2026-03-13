# Rapport 02 : Fix deadlock KB delete-only + run_e2e.sh --summary

## Suite du rapport 01

Le rapport 01 identifiait un bug ouvert : deadlock du drain sur delete-only KB. Cette session le corrige.

### 1. Fix : KBGatherNode ne propageait pas kb_content quand vide

**Fichier** : `src/dataflow/record_nodes.rs` (~ligne 2343)

**Problème** : Quand `KBGatherNode.execute()` ne trouvait aucun agrégat à traiter (cas delete-only sur une entité qui n'est pas titre KB), il émettait `done` mais **pas** `kb_content`. Or `KBUpdateNode` déclare `kb_content` comme `required: true`. L'edge `gather_kb.kb_content → update_kb.kb_content` existait mais n'avait jamais de données → deadlock détecté par le runtime.

**Fix** : Émettre un batch `kb_content` vide dans le early-return :

```rust
// Avant
if items.is_empty() {
    ctx.set_output("done", PortValue::Empty);
    return Ok(());
}

// Après
if items.is_empty() {
    ctx.set_output("done", PortValue::Empty);
    ctx.set_output("kb_content", PortValue::Batch(
        BatchPayload::new(PortType::KBContent, Vec::<KBContentRecord>::new()),
    ));
    return Ok(());
}
```

**Pourquoi ça suffit** : Tous les nœuds KB downstream (KBUpdateNode, KBChunkNode, InsertRecordNode, LinkRecordNode, KBEmbedNode, FlushNode) gèrent naturellement les batches vides — ils bouclent sur rien et propagent leurs outputs. Le pipeline se termine proprement.

**Principe** : Auto-régulation des nœuds. Plutôt que `build_ingestion_graph()` décide conditionnellement quels nœuds créer, chaque nœud gère le cas "rien à faire" et propage du vide. Plus robuste car :
- Pas de logique conditionnelle fragile dans le graph builder
- Fonctionne aussi pour les cas dynamiques (DeleteRecordNode peut ou non produire des agrégats selon l'entité supprimée)
- Un seul point de fix au lieu de modifier la construction du graph

### 2. Amélioration : run_e2e.sh --summary

**Fichier** : `run_e2e.sh`

Ajout d'une option `--summary` qui affiche un tableau récapitulatif par suite après l'exécution :

```
═══════════════════════════════════════════════
  SUMMARY
═══════════════════════════════════════════════
  e2e_batch_observe                2 passed
  e2e_checkpoint                   3 passed
  e2e_dataflow_observe             7 passed
  e2e_drain_unified                6 passed
  e2e_generic_search               8 passed
  e2e_highlight_long_text          8 passed
  e2e_native                      11 passed
  e2e_phase0b                     14 passed
  e2e_result_mode                 10 passed
  e2e_search                      37 passed
  e2e_search_queue                 5 passed
  e2e_simple_entity               15 passed
  e2e_undo                         4 passed
───────────────────────────────────────────────
  TOTAL                          130 passed
═══════════════════════════════════════════════
```

Sans `--summary`, un tip est affiché à la fin : `Tip: run with --summary for a per-suite results table.`

### 3. Tous les tests passent

| Suite | Tests | Status |
|-------|------:|--------|
| e2e_batch_observe | 2 | OK |
| e2e_checkpoint | 3 | OK |
| e2e_dataflow_observe | 7 | OK |
| e2e_drain_unified | 6 | OK |
| e2e_generic_search | 8 | OK |
| e2e_highlight_long_text | 8 | OK |
| e2e_native | 11 | OK |
| e2e_phase0b | 14 | OK |
| e2e_result_mode | 10 | OK |
| e2e_search | 37 | OK |
| e2e_search_queue | 5 | OK |
| e2e_simple_entity | 15 | OK |
| e2e_undo | 4 | OK |
| **TOTAL** | **130** | **0 failed** |

Les 4 tests undo passent tous :
- `undo_delete_simple_entity` — BM25 only
- `undo_update_simple_entity` — BM25 only
- `undo_delete_simple_entity_bgem3` — BM25 + Vector + Sparse (vrais embeddings BGE-M3)
- `undo_delete_kb_bgem3` — BM25 + Vector + Sparse sur KB (vrais embeddings BGE-M3) ← **débloqué**

### 4. DualEmbedder + vérification BM25 highlights

**Fichier** : `tests/e2e_undo.rs`

#### 4.1 Passage au DualEmbedder

Les tests BGE-M3 utilisaient `set_embedder()` + `set_sparse_embedder()` (2 forward passes séparés). Remplacé par `set_dual_embedder()` qui fait dense + sparse en un seul forward pass — c'est le chemin de code utilisé en production.

```rust
// Avant : 2 appels séparés
catalog.set_embedder(embedder);
catalog.set_sparse_embedder(sparse);

// Après : 1 appel dual
let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
catalog.set_dual_embedder(dual);
```

#### 4.2 Vérification diagnostics BM25 highlights → chunk resolution

`search_all_signals()` passe maintenant `diagnostics: true`. `assert_all_signals()` vérifie en plus que les highlights BM25 se résolvent en chunks (`chunks_matched > 0`).

#### 4.3 Recherche BM25-only en fin de pipeline

Ajout d'une recherche BM25-only (`signals: Some(SearchSignals::BM25)`) à la fin des 2 tests BGE-M3 pour valider que le FTS index est sain après le cycle complet create → drain → delete → undo → re-ingest → drain.

**Résultat KB** :
```
[BM25-only KB] query='ownership memory safety' → 1 results
  uuid=539d235f, score=2.8040
  bm25_hits=1, chunks_matched=1
  hit[0]: highlights={"_content": [(92, 101), (50, 56), (117, 123)]}
```

**Résultat simple entity** :
```
[BM25-only Product] query='Rust ownership lifetimes concurrency' → 1 results
  uuid=e3f464ca, score=3.1962
  bm25_hits=1, chunks_matched=1
  hit[0]: highlights={"description": [(25, 29), (60, 69), (71, 80), (86, 97)]}
```

Les offsets de highlights sont corrects (correspondent aux mots dans le texte), la résolution highlights→chunks fonctionne, et le score BM25 est cohérent (plus de termes matchés = score plus élevé).

### 5. Résumé des fichiers modifiés (depuis rapport 01)

| Fichier | Changement |
|---------|------------|
| `src/dataflow/record_nodes.rs` | KBGatherNode : émettre kb_content vide quand pas d'agrégats |
| `tests/e2e_undo.rs` | DualEmbedder, diagnostics BM25, recherche BM25-only en fin de test |
| `run_e2e.sh` | Option `--summary` + tip en mode normal |

### 6. Prochaines étapes

1. Commit + push
2. Tests undo pour UpdateRecordNode avec KB + BGE-M3 (undo_update_kb_bgem3)
3. Tests undo sur relations (si applicable)
