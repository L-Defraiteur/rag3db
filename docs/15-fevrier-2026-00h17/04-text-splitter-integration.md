# Rag3Weaver — Integration de text-splitter dans le chunker

Date : 15 fevrier 2026
Statut : FAIT

---

## Contexte

Le chunker de l'Etape 1 (L2) etait une implementation maison : recherche de delimiteurs par priorite (`\n\n`, `\n`, `. `, etc.) via `rfind` dans une fenetre glissante. Ca fonctionnait, mais le doc des findings (`03-findings-crates-ecosystem.md`) prevoyait d'integrer `text-splitter` (562 stars, maj 13 fev 2026) pour :

- Decoupe aux frontieres semantiques plus intelligente (Unicode-aware, grapheme clusters)
- Support markdown natif (CommonMark : headers, code blocks, listes)
- Support code natif (tree-sitter, par fonctions/classes) — branchable plus tard
- Sizing par tokens (tiktoken, HuggingFace) — branchable plus tard

**Ce que text-splitter ne faisait PAS a l'origine** (d'apres les findings) :
- Pas d'overlaps → **Faux depuis v0.18+** : `ChunkConfig::with_overlap(n)` existe
- Pas de tracking d'offsets → **Partiellement faux** : `chunk_indices()` retourne `(byte_offset, &str)`
- Pas de tracking de lignes → **Vrai**, on conserve notre `build_line_index`

Resultat : text-splitter couvre plus qu'on ne pensait. L'integration est minimale.

---

## Ce qui a change

### Cargo.toml

```toml
# AVANT
[dependencies]
blake3 = "1"
# ... (pas de text-splitter)

# APRES
[dependencies]
blake3 = "1"
text-splitter = { version = "0.28", features = ["markdown"] }
```

44 nouvelles dependances transitives (dont `icu_segmenter` pour la segmentation Unicode, `pulldown-cmark` pour le parsing Markdown).

### config.rs — Nouvelle variante ChunkStrategy

```rust
pub enum ChunkStrategy {
    Semantic,
    Fixed,
    Sentence,
    Markdown,  // NOUVEAU
}
```

Backward-compatible : les configs existantes avec `"strategy": "semantic"` continuent de fonctionner. `"strategy": "markdown"` est disponible pour le contenu Markdown.

### chunker.rs — Remplacement du coeur de decoupe

**Supprime** (code maison) :
- `SEMANTIC_DELIMITERS` — `&["\n\n", "\n", ". ", "! ", "? ", "; ", ", ", " "]`
- `SENTENCE_DELIMITERS` — `&[". ", "! ", "? ", "\n\n", "\n"]`
- `find_break_point()` — recherche du dernier delimiteur par priorite dans une fenetre
- `chunk_with_delimiters()` — boucle manuelle avec fenetre glissante

**Ajoute** :
- `chunk_with_text_splitter()` — delegation a text-splitter avec tracking de lignes

**Conserve** (inchange) :
- `Chunk` struct (text, index, start_byte, end_byte, start_line, end_line)
- `ChunkerConfig` struct (max_size, overlap, strategy)
- `chunk_fixed()` — strategie Fixed (coupe a max_size sans delimiteurs)
- `build_line_index()` — index cumulatif O(n) pour lookup lignes O(1)
- `snap_to_char_boundary()` — securite UTF-8 (utilise par chunk_fixed)
- `count_newlines()` — compteur simple

---

## Mapping des strategies

| ChunkStrategy | Splitter utilise | Comportement |
|---|---|---|
| `Semantic` | `TextSplitter` | Frontieres semantiques : graphemes → mots → phrases → paragraphes |
| `Sentence` | `TextSplitter` | Idem (text-splitter gere les phrases dans sa hierarchie) |
| `Markdown` | `MarkdownSplitter` | CommonMark-aware : respecte headers, code blocks, listes, emphases |
| `Fixed` | Notre `chunk_fixed` | Coupe a max_size, pas de recherche de frontieres |

---

## Implementation

```rust
fn chunk_with_text_splitter(&self, text: &str) -> Vec<Chunk> {
    let line_at = build_line_index(text);

    // Clamp overlap < max_size (text-splitter le requiert)
    let overlap = self.config.overlap.min(self.config.max_size.saturating_sub(1));

    let chunk_config = ChunkConfig::new(self.config.max_size)
        .with_overlap(overlap)
        .expect("overlap is clamped < max_size")
        .with_trim(false);  // On garde les offsets bruts, on trim nous-memes

    let indices: Vec<(usize, &str)> = match self.config.strategy {
        ChunkStrategy::Markdown => MarkdownSplitter::new(chunk_config)
            .chunk_indices(text).collect(),
        _ => TextSplitter::new(chunk_config)
            .chunk_indices(text).collect(),
    };

    // Enrichir avec index sequentiel + tracking de lignes
    let mut chunks = Vec::new();
    let mut chunk_index = 0;
    for (byte_offset, chunk_text) in indices {
        let trimmed = chunk_text.trim();
        if trimmed.is_empty() { continue; }
        let end_byte = byte_offset + chunk_text.len();
        chunks.push(Chunk {
            text: trimmed.to_string(),
            index: chunk_index,
            start_byte: byte_offset,
            end_byte,
            start_line: line_at[byte_offset],
            end_line: line_at[end_byte],
        });
        chunk_index += 1;
    }
    chunks
}
```

### Points de design

**`with_trim(false)`** : on desactive le trim de text-splitter pour garder les byte offsets fideles au texte original. Le trim est fait sur `Chunk.text` uniquement (pour l'affichage), mais `start_byte`/`end_byte` couvrent la plage brute. C'est coherent avec l'ancien comportement.

**Clamp de l'overlap** : text-splitter requiert `overlap < max_size` (retourne `Err` sinon). On clamp a `max_size - 1` pour eviter les panics. Le test `overlap_larger_than_chunk` verifie ce cas.

**`chunk_indices()` zero-copy** : retourne des `&str` slices dans le texte original. On copie dans `String` pour le `Chunk.text` car les chunks doivent outlive le texte source.

---

## Tests

### Existants adaptes (17 → 19)

| Test | Statut | Notes |
|------|--------|-------|
| `empty_text` | Inchange | |
| `whitespace_only` | Inchange | |
| `short_text_single_chunk` | Inchange | |
| `splits_at_paragraph_boundary` | Inchange | text-splitter respecte `\n\n` |
| `splits_at_sentence_boundary` | **Renomme** `splits_at_semantic_boundary` | Verifie que les chunks ne commencent pas par un delimiteur |
| `overlap_produces_shared_content` | Inchange | `with_overlap` produit le meme effet |
| `sequential_indices` | Inchange | |
| `fixed_splits_at_size` | Inchange | chunk_fixed non modifie |
| `fixed_with_overlap` | Inchange | |
| `offsets_cover_full_text` | Inchange | |
| `line_tracking_basic` | Inchange | build_line_index non modifie |
| `line_tracking_no_newlines` | Inchange | |
| `utf8_multibyte_chars` | Inchange | text-splitter est nativement Unicode-safe |
| `overlap_larger_than_chunk` | Inchange | Clamp verifie |
| `single_long_word` | Inchange | text-splitter force le split au niveau grapheme |
| `default_config` | Inchange | |
| `count_newlines_helper` | Inchange | |
| `snap_to_char_boundary_ascii` | Inchange | |
| `snap_to_char_boundary_utf8` | Inchange | |

### Nouveaux tests (2)

| Test | Strategie | Verifie |
|------|-----------|---------|
| `markdown_respects_headers` | Markdown | Split entre `# Section One` et `# Section Two` |
| `markdown_preserves_code_blocks` | Markdown | Le code block (` ```rust ... ``` `) reste dans un seul chunk |

### Bilan

```bash
cd packages/rag3db/extension/rag3weaver && cargo test
# 120 passed, 0 failed, 0 warnings
```

| Module | Tests |
|--------|:-----:|
| events.rs | 5 |
| config.rs | 11 |
| embedder.rs | 5 |
| connection.rs | 14 |
| schema.rs | 22 |
| query.rs | 17 |
| hash.rs | 4 |
| uuid.rs | 10 |
| chunker.rs | **21** (etait 19) |
| fusion.rs | 11 |
| **Total** | **120** |

---

## Amelioration vs l'ancien chunker

| Aspect | Avant (maison) | Apres (text-splitter) |
|--------|:-:|:-:|
| Frontieres semantiques | 8 delimiteurs string (`rfind`) | Unicode segmentation (graphemes, mots, phrases) |
| Markdown | Non | Oui (CommonMark : headers, code blocks, listes) |
| Code-aware | Non | Branchable (feature `code` + tree-sitter) |
| Overlap | Implementation maison (fenetre glissante) | Natif (`with_overlap`) |
| Byte offsets | Implementation maison | Natif (`chunk_indices`) |
| Line tracking | `build_line_index` O(n) | Toujours le notre (text-splitter ne le fait pas) |
| Sizing par tokens | Non | Branchable (tiktoken, HuggingFace) |
| UTF-8 safety | `snap_to_char_boundary` maison | Natif (Unicode segmentation) |

---

## Ce qui peut etre branche plus tard

### Feature `code` (tree-sitter)

```toml
text-splitter = { version = "0.28", features = ["markdown", "code"] }
```

```rust
use text_splitter::CodeSplitter;
let splitter = CodeSplitter::new(tree_sitter_rust::LANGUAGE, max_size)?;
```

Decoupe par fonctions/classes/blocs. Necessite les grammaires tree-sitter par langage.

### Sizing par tokens

```toml
text-splitter = { version = "0.28", features = ["markdown", "tiktoken-rs"] }
```

```rust
use text_splitter::ChunkConfig;
let config = ChunkConfig::new(512).with_sizer(tiktoken_tokenizer);
```

Utile quand max_size doit correspondre a la fenetre du modele d'embedding. Note : tiktoken-rs a des problemes WASM — utiliser le sizing par caracteres en WASM.

### Nouvelle strategie `Code` dans ChunkStrategy

```rust
pub enum ChunkStrategy {
    Semantic,
    Fixed,
    Sentence,
    Markdown,
    Code,  // futur
}
```

Necessiterait de passer la grammaire tree-sitter au `Chunker`. A designer avec le `CodeParser` de l'Etape 4.

---

## Dependances ajoutees

```
text-splitter v0.28.0
├── auto_enums (dispatch des iterateurs)
├── itertools
├── pulldown-cmark (parsing CommonMark)
├── icu_segmenter (segmentation Unicode : graphemes, mots, phrases)
│   ├── icu_collections
│   ├── icu_locale
│   ├── icu_provider
│   └── zerovec / zerotrie / yoke (structures ICU4X)
├── strum (derive enum)
└── unicase
```

Impact build : +44 crates, ~4s de compilation incrementale supplementaire.

---

## Prochaines etapes

L3 — Catalog CRUD + pipeline async :
- `catalog.rs` — Catalog::create/open, create(entity, data), relate(), drain()
- `pipeline.rs` — 4 phases (prepare → embed → store → link)
- `refs.rs` — EntityRef, RelationRef (resolution lazy des UUIDs)
- `queue.rs` — Queue configurable (drain explicite + auto-flush)
- `persistence.rs` — Tables systeme (_catalog_meta, _catalog_queue)
