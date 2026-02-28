# Improvement: use postings offsets in ContainsScorer instead of re-tokenizing

> Follows `05-offsets-implementation-plan.md` (offsets write/read pipeline) and `06-clarifying-implementation.md` (ContainsScorer design).

## Current state

### What exists

`WithFreqsAndPositionsAndOffsets` is fully implemented: byte offsets are stored in the postings and propagated through the entire read pipeline:

```
SegmentPostings::offsets()       -> Vec<(u32, u32)>    OK
LoadedPostings::append_offsets()                        OK
SimpleUnion::append_offsets()                           OK
BitSetPostingUnion::append_offsets()                    OK
PostingsWithOffset::append_offsets()                    OK
```

### What the ContainsScorer does today

For separator validation, the scorer needs byte offsets of matched tokens to extract the text between them from the stored document. Currently it does this:

```rust
// contains_scorer.rs — validate_separators()
let stored_text = store_reader.get(doc_id)?.get_first(field)?.as_str()?;
let doc_tokens = tokenize_raw(&stored_text);  // <-- re-scans the entire string
let separator = &stored_text[doc_tokens[pos_i].1 .. doc_tokens[pos_j].0];
```

`tokenize_raw()` re-scans the stored text to split it into `(byte_from, byte_to)` pairs. This works but is redundant — the offsets are already in the index.

### Why the offsets aren't used yet

The `append_positions_with_offset()` and `append_offsets()` methods in unions (SimpleUnion, etc.) sort and dedup independently. After that, the correspondence `position[i] <-> offset[i]` is lost:

```
positions:  [3, 5, 7]          (sorted, deduped)
offsets:    [(10,15), (20,25), (30,35)]  (sorted by offset_from, deduped)
  -> no guarantee that positions[0] corresponds to offsets[0]
```

---

## Proposed solution: joint positions+offsets method

### New trait method

```rust
// src/postings/postings.rs
trait Postings {
    // ... existing methods ...

    /// Append (position, offset_from, offset_to) tuples for the current document.
    /// Tuples are sorted by position. This keeps positions and offsets correlated.
    fn append_positions_and_offsets(&mut self, _output: &mut Vec<(u32, u32, u32)>) {}
}
```

### Implementation per layer

#### 1. SegmentPostings

Read positions and offsets together. Both readers are aligned to the same document, and both have `term_freq` entries per doc. Zip them:

```rust
fn append_positions_and_offsets(&mut self, output: &mut Vec<(u32, u32, u32)>) {
    let positions = self.positions();  // Vec<u32> (cumulative)
    let offsets = self.offsets_raw();  // Vec<(u32, u32)> (cumulative)
    // Both have term_freq entries, same order
    for (i, &pos) in positions.iter().enumerate() {
        if let Some(&(from, to)) = offsets.get(i) {
            output.push((pos, from, to));
        } else {
            output.push((pos, 0, 0));  // fallback if no offsets indexed
        }
    }
}
```

Note: need a variant that doesn't clear the output (append semantics). The existing `positions()` returns `Vec<u32>` (clears + fills). We might need an `append_positions()` or call the lower-level reader directly.

Actually, `SegmentPostings` already has:
- `positions()` at line ~220 — reads from position_reader, returns Vec<u32>
- `offsets()` at line ~253 — reads from offsets_reader, returns Vec<(u32, u32)>

Both read `term_freq` values for the current doc. They're naturally aligned. We can read both and zip:

```rust
fn append_positions_and_offsets(&mut self, output: &mut Vec<(u32, u32, u32)>) {
    let mut positions = Vec::new();
    self.append_positions_with_offset(0, &mut positions);  // raw positions

    if let Some(ref mut offsets_reader) = self.offsets_reader {
        let mut raw_offsets = Vec::new();
        self.append_offsets(&mut raw_offsets);
        for (i, &pos) in positions.iter().enumerate() {
            let (from, to) = raw_offsets.get(i).copied().unwrap_or((0, 0));
            output.push((pos, from, to));
        }
    } else {
        for &pos in &positions {
            output.push((pos, 0, 0));
        }
    }
}
```

#### 2. LoadedPostings

Same principle — positions and offsets are stored in parallel arrays. Already loaded in memory during `load()`. Zip and append.

#### 3. SimpleUnion

Iterate all docsets where `doc == self.doc`, collect all `(pos, from, to)` tuples, sort by position, dedup by position (keeping first occurrence):

```rust
fn append_positions_and_offsets(&mut self, output: &mut Vec<(u32, u32, u32)>) {
    let mut combined = Vec::new();
    for docset in &mut self.docsets {
        if docset.doc() == self.doc {
            docset.append_positions_and_offsets(&mut combined);
        }
    }
    combined.sort_by_key(|&(pos, _, _)| pos);
    combined.dedup_by_key(|t| t.0);  // dedup by position
    output.extend(combined);
}
```

This naturally keeps positions and offsets correlated because they travel together as tuples.

#### 4. BitSetPostingUnion

Same pattern as SimpleUnion but with `RefCell` borrow.

#### 5. PostingsWithOffset (in phrase_scorer.rs)

Delegates to inner postings, adjusts position by the offset:

```rust
fn append_positions_and_offsets(&mut self, output: &mut Vec<(u32, u32, u32)>) {
    let mut inner = Vec::new();
    self.postings.append_positions_and_offsets(&mut inner);
    for (pos, from, to) in inner {
        output.push((pos + self.offset as u32, from, to));
    }
}
```

Byte offsets (from, to) are absolute in the document — they don't need adjustment. Only the position is shifted.

---

## ContainsScorer changes

### validate_separators() — multi-token

Current flow:
```
1. load stored text
2. tokenize_raw(stored_text) -> all doc tokens with byte offsets
3. for each phrase occurrence:
   a. map matched positions -> doc token indices
   b. extract separators from stored_text using doc token offsets
   c. compare with query separators
```

New flow:
```
1. for each posting list, call append_positions_and_offsets()
   -> now we have (position, byte_from, byte_to) per query token
2. do position intersection (find consecutive positions)
3. for each phrase occurrence:
   a. matched_offsets = [(from0, to0), (from1, to1), ...]
      directly from the intersection
   b. load stored text (only now, only if needed)
   c. separator = stored_text[to_i .. from_{i+1}]
   d. prefix = stored_text[..from_0]  (if query has prefix)
   e. suffix = stored_text[to_n..]    (if query has suffix)
   f. compare with query separators
```

Benefits:
- No `tokenize_raw()` call at all
- Stored text is only loaded if a position intersection succeeds (lazy)
- Byte offsets come directly from the index — zero redundant work

### validate_current() — single token

Current: loads stored text, re-tokenizes, finds matching token, checks prefix/suffix.

New: the `ContainsSingleScorer` doesn't use posting lists with positions (it uses a BitSet). For single tokens, we'd need to also store the matched term's offsets. This is trickier since `single_token_scorer()` in `automaton_phrase_weight.rs` builds a BitSet from block postings (Basic level, no positions/offsets).

Options:
1. Keep `tokenize_raw()` for single-token only (simple, single tokens are cheap)
2. Switch to a posting-based scorer instead of BitSet (more complex, but consistent)
3. Read postings with offsets when building the BitSet (store a side map doc_id -> offsets)

Recommendation: keep `tokenize_raw()` for single-token for now. The multi-token case is where the real benefit is (multiple posting lists, position intersection, more complex documents). Single-token re-tokenization is negligible.

---

## Files to modify

| File | Change | Lines est. |
|------|--------|-----------|
| `src/postings/postings.rs` | Add `append_positions_and_offsets()` to trait + Box delegation | +10 |
| `src/postings/segment_postings.rs` | Impl: read positions + offsets together | +20 |
| `src/postings/loaded_postings.rs` | Impl: zip from loaded arrays | +15 |
| `src/query/union/simple_union.rs` | Impl: collect + sort + dedup tuples | +15 |
| `src/query/union/bitset_union.rs` | Impl: same pattern | +15 |
| `src/query/phrase_query/phrase_scorer.rs` | PostingsWithOffset: proxy + adjust position | +10 |
| `src/query/phrase_query/contains_scorer.rs` | Use offsets from postings, remove tokenize_raw for multi-token | +30, -40 |

**Total: ~115 lines added, ~40 removed. 7 files.**

---

## Verification plan

1. `cargo test --lib` in ld-tantivy (997+ tests) — no regression
2. `cargo build --release` + FFI tests (129 tests) — separator validation still works
3. Specific attention to:
   - `c++` query (single token — still uses tokenize_raw)
   - `std::collections` query (multi-token — now uses postings offsets)
   - `option<result<(i32` query (multi-token with complex separators)
   - Fuzzy matches with separators (offsets should still be correct)

---

## Prerequisite: field must use WithFreqsAndPositionsAndOffsets

The postings offsets are only available if the field was indexed with `WithFreqsAndPositionsAndOffsets`. If the field uses `WithFreqsAndPositions` (no offsets), the `append_positions_and_offsets()` method returns `(pos, 0, 0)` tuples.

The ContainsScorer should detect this and fall back to `tokenize_raw()` when offsets are all zeros. This keeps backward compatibility with fields that don't have offsets indexed.

```rust
// In validate_separators():
let has_real_offsets = matched_offsets.iter().any(|&(_, from, _)| from > 0);
if !has_real_offsets {
    // Fallback: re-tokenize (field doesn't have offsets indexed)
    return self.validate_separators_via_retokenize(...);
}
```

Currently in tantivy_fts, the raw field is indexed with `WithFreqsAndPositions` (not offsets). To benefit from this improvement, we'd need to switch the raw field to `WithFreqsAndPositionsAndOffsets` in `handle.rs`. This increases index size slightly (2x more data in `.offsets` file) but eliminates re-tokenization entirely.

---

## Impact on README

Current README says:
> Re-tokenizes to obtain byte offsets of each token

After this change:
> Uses byte offsets stored in postings to locate separators directly (no re-tokenization needed for multi-token queries)
