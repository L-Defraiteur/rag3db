//! Text chunking with overlap and offset tracking.
//!
//! Uses [`text-splitter`](https://crates.io/crates/text-splitter) for intelligent
//! boundary detection (semantic text + markdown-aware) and adds line number tracking.
//!
//! - `Semantic` / `Sentence` → `TextSplitter` (text semantic boundaries)
//! - `Markdown` → `MarkdownSplitter` (respects headers, code blocks, lists)
//! - `Fixed` → simple fixed-size splitting (no delimiter search)

use crate::config::ChunkStrategy;
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};

// ─── Types ──────────────────────────────────────────────────────────────────

/// A chunk of text with its position in the original document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub text: String,
    /// Sequential index (0, 1, 2, ...).
    pub index: usize,
    /// Byte offset where this chunk starts in the original text.
    pub start_byte: usize,
    /// Byte offset where this chunk ends in the original text.
    pub end_byte: usize,
    /// Line number (0-based) where this chunk starts.
    pub start_line: usize,
    /// Line number (0-based) where this chunk ends.
    pub end_line: usize,
}

/// Configuration for the chunker.
#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    /// Maximum characters per chunk.
    pub max_size: usize,
    /// Number of overlapping characters between consecutive chunks.
    pub overlap: usize,
    /// Chunking strategy.
    pub strategy: ChunkStrategy,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_size: 1500,
            overlap: 200,
            strategy: ChunkStrategy::Semantic,
        }
    }
}

// ─── Chunker ────────────────────────────────────────────────────────────────

/// Split text into overlapping chunks with offset tracking.
pub struct Chunker {
    config: ChunkerConfig,
}

impl Chunker {
    pub fn new(config: ChunkerConfig) -> Self {
        Self { config }
    }

    /// Split `text` into chunks.
    ///
    /// Returns an empty vec for empty input. Returns a single chunk if the
    /// text fits within `max_size`.
    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return vec![];
        }

        if text.len() <= self.config.max_size {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return vec![];
            }
            return vec![Chunk {
                text: trimmed.to_string(),
                index: 0,
                start_byte: 0,
                end_byte: text.len(),
                start_line: 0,
                end_line: count_newlines(text),
            }];
        }

        match self.config.strategy {
            ChunkStrategy::Fixed => self.chunk_fixed(text),
            _ => self.chunk_with_text_splitter(text),
        }
    }

    /// Fixed-size chunking: split at max_size boundaries, no delimiter search.
    fn chunk_fixed(&self, text: &str) -> Vec<Chunk> {
        let line_at = build_line_index(text);
        let mut chunks = Vec::new();
        let mut pos = 0;
        let mut chunk_index = 0;

        while pos < text.len() {
            let raw_end = (pos + self.config.max_size).min(text.len());
            let end = snap_to_char_boundary(text, raw_end);
            let end = end.max(pos + 1); // always advance

            let chunk_text = text[pos..end].trim();
            if !chunk_text.is_empty() {
                chunks.push(Chunk {
                    text: chunk_text.to_string(),
                    index: chunk_index,
                    start_byte: pos,
                    end_byte: end,
                    start_line: line_at[pos],
                    end_line: line_at[end],
                });
                chunk_index += 1;
            }

            // Advance with overlap
            let next_pos = end.saturating_sub(self.config.overlap);
            let next_pos = snap_to_char_boundary(text, next_pos);
            pos = next_pos.max(pos + 1);
        }

        chunks
    }

    /// Use text-splitter for intelligent boundary detection.
    ///
    /// `Semantic` / `Sentence` → `TextSplitter` (semantic text boundaries).
    /// `Markdown` → `MarkdownSplitter` (CommonMark-aware).
    fn chunk_with_text_splitter(&self, text: &str) -> Vec<Chunk> {
        let line_at = build_line_index(text);

        // Clamp overlap to be strictly less than max_size
        let overlap = self.config.overlap.min(self.config.max_size.saturating_sub(1));

        let chunk_config = ChunkConfig::new(self.config.max_size)
            .with_overlap(overlap)
            .expect("overlap is clamped < max_size")
            .with_trim(false);

        let indices: Vec<(usize, &str)> = match self.config.strategy {
            ChunkStrategy::Markdown => MarkdownSplitter::new(chunk_config)
                .chunk_indices(text)
                .collect(),
            _ => TextSplitter::new(chunk_config)
                .chunk_indices(text)
                .collect(),
        };

        let mut chunks = Vec::new();
        let mut chunk_index = 0;

        for (byte_offset, chunk_text) in indices {
            let trimmed = chunk_text.trim();
            if trimmed.is_empty() {
                continue;
            }
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
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Precompute cumulative newline counts: `line_at[i]` = number of `\n` in `text[..i]`.
///
/// O(n) to build, O(1) per lookup. Handles overlapping chunks correctly.
fn build_line_index(text: &str) -> Vec<usize> {
    let mut idx = Vec::with_capacity(text.len() + 1);
    let mut count = 0;
    idx.push(0); // line_at[0] = 0
    for b in text.bytes() {
        if b == b'\n' {
            count += 1;
        }
        idx.push(count);
    }
    idx
}

/// Snap a byte position backward to the nearest UTF-8 char boundary.
fn snap_to_char_boundary(text: &str, pos: usize) -> usize {
    let mut p = pos.min(text.len());
    while p > 0 && !text.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Count newline characters in a string slice.
fn count_newlines(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn semantic_chunker(max_size: usize, overlap: usize) -> Chunker {
        Chunker::new(ChunkerConfig {
            max_size,
            overlap,
            strategy: ChunkStrategy::Semantic,
        })
    }

    fn fixed_chunker(max_size: usize, overlap: usize) -> Chunker {
        Chunker::new(ChunkerConfig {
            max_size,
            overlap,
            strategy: ChunkStrategy::Fixed,
        })
    }

    fn markdown_chunker(max_size: usize, overlap: usize) -> Chunker {
        Chunker::new(ChunkerConfig {
            max_size,
            overlap,
            strategy: ChunkStrategy::Markdown,
        })
    }

    // ── empty / short input ──────────────────────────────────────────────

    #[test]
    fn empty_text() {
        let chunks = semantic_chunker(100, 20).chunk("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn whitespace_only() {
        let chunks = semantic_chunker(100, 20).chunk("   \n\n  ");
        assert!(chunks.is_empty());
    }

    #[test]
    fn short_text_single_chunk() {
        let text = "Hello, world!";
        let chunks = semantic_chunker(100, 20).chunk(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello, world!");
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].start_byte, 0);
        assert_eq!(chunks[0].end_byte, text.len());
        assert_eq!(chunks[0].start_line, 0);
        assert_eq!(chunks[0].end_line, 0);
    }

    // ── semantic chunking (text-splitter) ─────────────────────────────────

    #[test]
    fn splits_at_paragraph_boundary() {
        let text = "First paragraph with enough text to fill a chunk.\n\n\
                     Second paragraph also has enough text to be its own.";
        let chunks = semantic_chunker(55, 10).chunk(text);
        assert!(chunks.len() >= 2, "should split into at least 2 chunks");
        assert!(
            chunks[0].text.contains("First paragraph"),
            "first chunk has first paragraph"
        );
    }

    #[test]
    fn splits_at_semantic_boundary() {
        let text = "First sentence is here. Second sentence follows. Third sentence arrives. Fourth and final.";
        let chunks = semantic_chunker(50, 10).chunk(text);
        assert!(chunks.len() >= 2, "should produce at least 2 chunks");
        // text-splitter should not split mid-word
        for chunk in &chunks {
            let first_char = chunk.text.chars().next().unwrap();
            assert!(
                first_char.is_alphabetic() || first_char.is_ascii_digit(),
                "chunk should not start with a delimiter: {:?}",
                chunk.text
            );
        }
    }

    #[test]
    fn overlap_produces_shared_content() {
        let text = "AAAA. BBBB. CCCC. DDDD. EEEE. FFFF. GGGG. HHHH.";
        let chunks = semantic_chunker(25, 10).chunk(text);
        assert!(chunks.len() >= 2);

        // Check that consecutive chunks share some content (overlap)
        if chunks.len() >= 2 {
            let c0_end = chunks[0].end_byte;
            let c1_start = chunks[1].start_byte;
            assert!(
                c1_start < c0_end,
                "chunk 1 should start before chunk 0 ends (overlap). c0_end={c0_end}, c1_start={c1_start}"
            );
        }
    }

    #[test]
    fn sequential_indices() {
        let text = "A. ".repeat(100);
        let chunks = semantic_chunker(30, 5).chunk(&text);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    // ── fixed chunking ───────────────────────────────────────────────────

    #[test]
    fn fixed_splits_at_size() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let chunks = fixed_chunker(10, 0).chunk(text);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(
                chunk.end_byte - chunk.start_byte <= 10,
                "chunk too large: {}..{}",
                chunk.start_byte,
                chunk.end_byte
            );
        }
    }

    #[test]
    fn fixed_with_overlap() {
        let text = "0123456789abcdefghij";
        let chunks = fixed_chunker(10, 4).chunk(text);
        assert!(chunks.len() >= 2);
        if chunks.len() >= 2 {
            assert!(
                chunks[1].start_byte < chunks[0].end_byte,
                "overlap expected"
            );
        }
    }

    // ── markdown chunking ────────────────────────────────────────────────

    #[test]
    fn markdown_respects_headers() {
        let text = "\
# Section One

Content for section one with enough text to fill a chunk.

# Section Two

Content for section two with enough text to be its own chunk.";
        let chunks = markdown_chunker(80, 10).chunk(text);
        assert!(chunks.len() >= 2, "should split into at least 2 chunks");
        // First chunk should contain section one
        assert!(
            chunks[0].text.contains("Section One"),
            "first chunk should contain first header: {:?}",
            chunks[0].text
        );
    }

    #[test]
    fn markdown_preserves_code_blocks() {
        let text = "\
Some intro text before the code block.

```rust
fn main() {
    println!(\"Hello, world!\");
}
```

Some text after the code block that is long enough to chunk.";
        let chunks = markdown_chunker(120, 10).chunk(text);
        // The code block should not be split across chunks
        let code_chunk = chunks.iter().find(|c| c.text.contains("fn main"));
        assert!(code_chunk.is_some(), "should have a chunk with the code");
        let code = code_chunk.unwrap();
        assert!(
            code.text.contains("println!"),
            "code block should be kept together: {:?}",
            code.text
        );
    }

    // ── offset tracking ──────────────────────────────────────────────────

    #[test]
    fn offsets_cover_full_text() {
        let text = "Hello world. This is a longer text. It should be chunked properly.";
        let chunks = semantic_chunker(30, 5).chunk(text);
        // First chunk starts at 0
        assert_eq!(chunks[0].start_byte, 0);
        // Last chunk ends at text length
        assert_eq!(chunks.last().unwrap().end_byte, text.len());
    }

    #[test]
    fn line_tracking_basic() {
        let text = "line 0\nline 1\nline 2\nline 3\nline 4";
        let chunks = semantic_chunker(15, 3).chunk(text);
        // First chunk starts at line 0
        assert_eq!(chunks[0].start_line, 0);
        // Last chunk's end_line should account for all newlines in its range
        let total_newlines = text.matches('\n').count();
        let last = chunks.last().unwrap();
        assert!(last.end_line <= total_newlines, "end_line should not exceed total lines");
    }

    #[test]
    fn line_tracking_no_newlines() {
        let text = "no newlines at all in this text which is long enough to chunk";
        let chunks = semantic_chunker(25, 5).chunk(text);
        for chunk in &chunks {
            assert_eq!(chunk.start_line, 0);
            assert_eq!(chunk.end_line, 0);
        }
    }

    // ── UTF-8 safety ─────────────────────────────────────────────────────

    #[test]
    fn utf8_multibyte_chars() {
        let text = "Café résumé. Ça marche très bien. 🎉 Unicode est supporté.";
        let chunks = semantic_chunker(30, 5).chunk(text);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
        }
    }

    // ── edge cases ───────────────────────────────────────────────────────

    #[test]
    fn overlap_larger_than_chunk() {
        // overlap > max_size should not panic or loop (clamped internally)
        let text = "a b c d e f g h i j k l m n o p";
        let chunks = semantic_chunker(10, 50).chunk(text);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn single_long_word() {
        // No break points — forced split
        let text = "a".repeat(200);
        let chunks = semantic_chunker(50, 10).chunk(&text);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn default_config() {
        let chunker = Chunker::new(ChunkerConfig::default());
        assert_eq!(chunker.config.max_size, 1500);
        assert_eq!(chunker.config.overlap, 200);
    }

    // ── helpers ──────────────────────────────────────────────────────────

    #[test]
    fn count_newlines_helper() {
        assert_eq!(count_newlines(""), 0);
        assert_eq!(count_newlines("no newlines"), 0);
        assert_eq!(count_newlines("one\nnewline"), 1);
        assert_eq!(count_newlines("\n\n\n"), 3);
    }

    #[test]
    fn snap_to_char_boundary_ascii() {
        let text = "hello";
        assert_eq!(snap_to_char_boundary(text, 3), 3);
        assert_eq!(snap_to_char_boundary(text, 100), 5); // clamped
    }

    #[test]
    fn snap_to_char_boundary_utf8() {
        let text = "café"; // 'é' is 2 bytes: byte positions 0,1,2,3,4 where é is at 3..5
        let byte_len = text.len(); // 5 bytes
        assert_eq!(byte_len, 5);
        // Position 4 is mid-'é', should snap back to 3
        let snapped = snap_to_char_boundary(text, 4);
        assert!(text.is_char_boundary(snapped));
    }
}
