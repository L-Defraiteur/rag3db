//! Text chunking with overlap and offset tracking (core-first approach).
//!
//! Uses [`text-splitter`](https://crates.io/crates/text-splitter) for intelligent
//! boundary detection (semantic text + markdown-aware) and adds line number tracking.
//!
//! Algorithm (inspired by L3 SemanticChunker):
//! 1. Split text into non-overlapping **cores** at semantic boundaries
//! 2. Extend each core by `overlap` chars on both sides
//! 3. Core offsets track the "owned" zone, full offsets include overlap context
//!
//! - `Semantic` / `Sentence` → `TextSplitter` (text semantic boundaries)
//! - `Markdown` → `MarkdownSplitter` (respects headers, code blocks, lists)
//! - `Fixed` → simple fixed-size splitting (no delimiter search)

use crate::config::ChunkStrategy;
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};

// ─── Types ──────────────────────────────────────────────────────────────────

/// A chunk of text with its position in the original document.
///
/// Each chunk has two sets of offsets:
/// - **Full** (`start_byte`..`end_byte`): the chunk text including overlap context
/// - **Core** (`core_start_byte`..`core_end_byte`): the "owned" zone without overlap
///
/// Cores are contiguous and non-overlapping. The frontend can use core offsets
/// to highlight the unique portion of each chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub text: String,
    /// Sequential index (0, 1, 2, ...).
    pub index: usize,
    /// Byte offset where this chunk starts in the original text (with overlap).
    pub start_byte: usize,
    /// Byte offset where this chunk ends in the original text (with overlap).
    pub end_byte: usize,
    /// Line number (0-based) where this chunk starts.
    pub start_line: usize,
    /// Line number (0-based) where this chunk ends.
    pub end_line: usize,
    /// Byte offset where the core zone starts (without overlap).
    pub core_start_byte: usize,
    /// Byte offset where the core zone ends (without overlap).
    pub core_end_byte: usize,
    /// Line number (0-based) where the core zone starts.
    pub core_start_line: usize,
    /// Line number (0-based) where the core zone ends.
    pub core_end_line: usize,
}

/// Configuration for the chunker.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChunkerConfig {
    /// Maximum characters per chunk.
    pub max_size: usize,
    /// Number of overlapping characters between consecutive chunks.
    pub overlap: usize,
    /// Chunking strategy.
    pub strategy: ChunkStrategy,
    /// `Lines` : lignes par chunk, au plus (0 = sans borne).
    pub max_lines: usize,
    /// `Lines` : lignes partagées entre voisins, quand le texte dépasse.
    pub overlap_lines: usize,
    /// `Lines`: **the bound of a whole chunk**, core and context, in bytes (the window of the base's embedder). None:
    /// only `max_size`, which bounds the core and each side of the context.
    pub max_chunk_size: Option<usize>,
    /// `Lines`: **the bound of a whole chunk in tokens**, counted by the tokenizer of the model it names. A chunker
    /// with a bound in tokens is built by [`Chunker::try_new`], which refuses when the tokenizer is missing.
    pub token_bound: Option<crate::tokens::TokenBound>,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_size: 1500,
            overlap: 200,
            strategy: ChunkStrategy::Semantic,
            max_lines: 30,
            overlap_lines: 5,
            max_chunk_size: None,
            token_bound: None,
        }
    }
}

/// **Une seule source** pour passer de la configuration déclarée au découpeur :
/// neuf littéraux faisaient cette copie champ par champ, et chaque champ
/// nouveau les cassait tous (6 septembre 2026).
impl From<&crate::config::ChunkingConfig> for ChunkerConfig {
    fn from(c: &crate::config::ChunkingConfig) -> Self {
        Self {
            max_size: c.max_size,
            overlap: c.overlap,
            strategy: c.strategy.clone(),
            max_lines: c.max_lines,
            overlap_lines: c.overlap_lines,
            max_chunk_size: c.max_chunk_size,
            token_bound: c.token_bound.clone(),
        }
    }
}

// ─── Chunker ────────────────────────────────────────────────────────────────

/// Split text into overlapping chunks with offset tracking.
pub struct Chunker {
    config: ChunkerConfig,
    /// The counter of the bound in tokens, when the configuration has one.
    tokens: Option<crate::tokens::TokenCounter>,
}

impl Chunker {
    /// A chunker without a bound in tokens. **With one, use [`Self::try_new`]**: this one would have no counter, and
    /// panics rather than cut by the bytes alone.
    pub fn new(config: ChunkerConfig) -> Self {
        Self::try_new(config).unwrap_or_else(|e| panic!("Chunker::new with a bound in tokens: {e} (use Chunker::try_new)"))
    }

    /// **A chunker, its counter of tokens loaded** when its configuration bounds chunks in tokens. Refuses, naming the
    /// tokenizer, when this build or this machine has none: the chunks for that model are never cut by the bytes
    /// alone.
    pub fn try_new(config: ChunkerConfig) -> Result<Self, String> {
        let tokens = match &config.token_bound {
            Some(b) => Some(crate::tokens::counter(&b.model)?),
            None => None,
        };
        Ok(Self { config, tokens })
    }

    /// A chunker with this counter of tokens (the witnesses, and a caller that holds one already).
    pub fn with_counter(config: ChunkerConfig, tokens: crate::tokens::TokenCounter) -> Self {
        Self { config, tokens: Some(tokens) }
    }

    /// **Whether `text[a..b]` fits a whole chunk**: under the bound in bytes, and, when the chunk could pass the
    /// window, under it in tokens. A token covers at least one byte, so a text of at most `window − 2` bytes (the two
    /// special tokens) is never counted.
    fn fits(&self, text: &str, a: usize, b: usize) -> bool {
        let octets = b - a;
        if octets > self.config.max_chunk_size.unwrap_or(usize::MAX) {
            return false;
        }
        match (&self.config.token_bound, &self.tokens) {
            (Some(borne), Some(compte)) if octets + 2 > borne.window => compte(text[a..b].trim_end()) <= borne.window,
            _ => true,
        }
    }

    /// Split `text` into chunks.
    ///
    /// Returns an empty vec for empty input. Returns a single chunk if the
    /// text fits within `max_size`.
    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return vec![];
        }

        // Par lignes, la borne en lignes compte autant que celle en octets.
        let tient_en_lignes = self.config.strategy != ChunkStrategy::Lines
            || self.config.max_lines == 0
            || text.lines().count() <= self.config.max_lines;
        // A text whole is one chunk only under the bound of a whole chunk too.
        let tient_en_octets = text.len() <= self.config.max_size.min(self.config.max_chunk_size.unwrap_or(usize::MAX));
        if tient_en_octets && tient_en_lignes && self.fits(text, 0, text.len()) {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return vec![];
            }
            let end_line = count_newlines(text);
            return vec![Chunk {
                text: trimmed.to_string(),
                index: 0,
                start_byte: 0,
                end_byte: text.len(),
                start_line: 0,
                end_line,
                core_start_byte: 0,
                core_end_byte: text.len(),
                core_start_line: 0,
                core_end_line: end_line,
            }];
        }

        match self.config.strategy {
            ChunkStrategy::Fixed => self.chunk_fixed(text),
            ChunkStrategy::Lines => self.chunk_lines(text),
            _ => self.chunk_with_text_splitter(text),
        }
    }

    /// **Par lignes** : des cœurs de lignes entières, jusqu'à `max_lines`
    /// lignes ou `max_size` octets, puis `overlap_lines` lignes de contexte
    /// de chaque côté. Un dernier cœur trop court (moins de 50 octets) rejoint
    /// le précédent plutôt que de faire un chunk qui ne dit rien. **A line longer than `max_size` is cut into
    /// cores of at most `max_size` bytes, on character boundaries, with no context** (embarquement's ticket of
    /// 16 November 2026: a data file of one line made a single chunk of 53 294 characters). **The context on
    /// each side stops before it would exceed `max_size` bytes**: five ordinary lines are untouched, a huge
    /// line is never context.
    fn chunk_lines(&self, text: &str) -> Vec<Chunk> {
        let line_at = build_line_index(text);
        // Les débuts de ligne, plus la fin du texte.
        let mut debuts: Vec<usize> = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' && i + 1 < text.len() {
                debuts.push(i + 1);
            }
        }
        let n = debuts.len();
        let fin_de = |l: usize| if l + 1 < n { debuts[l + 1] } else { text.len() };
        let max_lines = if self.config.max_lines == 0 { usize::MAX } else { self.config.max_lines };

        // Phase 1 : les cœurs, en lignes. A line longer than `max_size` is cut in place: its cores are byte
        // ranges inside it (`in_one_line`), and they take no context.
        // **The bound of a whole chunk** (ticket of embarquement, 20 November 2026): the core keeps under it, and so do
        // its context and a merged tail below. Without it, `max_size` bounds the core and each side of the context.
        let borne = self.config.max_chunk_size.unwrap_or(usize::MAX).max(1);
        let cote_max = self.config.max_size.max(1);
        let max_size = cote_max.min(borne);
        // (first line, last line excluded, byte range of the core, inside one long line)
        let mut cores: Vec<(usize, usize, usize, usize, bool)> = Vec::new();
        let mut l = 0;
        while l < n {
            if fin_de(l) - debuts[l] > max_size || !self.fits(text, debuts[l], fin_de(l)) {
                let mut start = debuts[l];
                while start < fin_de(l) {
                    let un_caractere = |s: usize| s + text[s..].chars().next().map_or(1, char::len_utf8);
                    let mut end = snap_to_char_boundary(text, (start + max_size).min(fin_de(l)));
                    // A line too dense for the window in tokens: its piece halves until it fits.
                    while end > start && !self.fits(text, start, end) {
                        end = snap_to_char_boundary(text, start + (end - start) / 2);
                    }
                    if end <= start {
                        // Never an empty core: at least one whole character.
                        end = un_caractere(start);
                    }
                    cores.push((l, l + 1, start, end, true));
                    start = end;
                }
                l += 1;
                continue;
            }
            let mut fin = l + 1;
            while fin < n && fin - l < max_lines && fin_de(fin) - debuts[l] <= max_size && self.fits(text, debuts[l], fin_de(fin)) {
                fin += 1;
            }
            cores.push((l, fin, debuts[l], fin_de(fin - 1), false));
            l = fin;
        }
        if cores.len() >= 2 {
            let (d, f, _, last_byte, in_one_line) = cores[cores.len() - 1];
            let previous_in_one_line = cores[cores.len() - 2].4;
            let merged_start = debuts[cores[cores.len() - 2].0];
            if !in_one_line && !previous_in_one_line && last_byte - debuts[d] < 50 && self.fits(text, merged_start, last_byte) {
                cores.pop();
                let last = cores.last_mut().expect("au moins un cœur");
                last.1 = f;
                last.3 = last_byte;
            }
        }

        // Phase 2 : le contexte, en lignes, at most `max_size` bytes on each side.
        let mut chunks = Vec::new();
        for (index, &(d, f, core_start, core_end, in_one_line)) in cores.iter().enumerate() {
            let (start_byte, end_byte) = if in_one_line {
                (core_start, core_end)
            } else {
                // A line of context at a time, before then after, while each side keeps under `max_size` and the
                // whole chunk under its bound: the context gives way first, the core never does.
                let (premiere, derniere) = (d.saturating_sub(self.config.overlap_lines), (f + self.config.overlap_lines).min(n));
                let (mut start_line, mut end_line) = (d, f);
                loop {
                    let mut grew = false;
                    if start_line > premiere
                        && debuts[d] - debuts[start_line - 1] <= cote_max
                        && self.fits(text, debuts[start_line - 1], fin_de(end_line - 1))
                    {
                        start_line -= 1;
                        grew = true;
                    }
                    if end_line < derniere
                        && fin_de(end_line) - fin_de(f - 1) <= cote_max
                        && self.fits(text, debuts[start_line], fin_de(end_line))
                    {
                        end_line += 1;
                        grew = true;
                    }
                    if !grew {
                        break;
                    }
                }
                (debuts[start_line], fin_de(end_line - 1))
            };
            let chunk_text = text[start_byte..end_byte].trim_end();
            if chunk_text.trim().is_empty() {
                continue;
            }
            chunks.push(Chunk {
                text: chunk_text.to_string(),
                index,
                start_byte,
                end_byte,
                start_line: line_at[start_byte],
                end_line: line_at[end_byte],
                core_start_byte: core_start,
                core_end_byte: core_end,
                core_start_line: line_at[core_start],
                core_end_line: line_at[core_end],
            });
        }
        chunks
    }

    /// Fixed-size chunking: core-first, then extend with overlap.
    fn chunk_fixed(&self, text: &str) -> Vec<Chunk> {
        let line_at = build_line_index(text);
        let core_size = self.config.max_size.saturating_sub(self.config.overlap).max(1);
        let overlap = self.config.overlap;

        // Phase 1: Build non-overlapping cores
        let mut cores: Vec<(usize, usize)> = Vec::new();
        let mut pos = 0;
        while pos < text.len() {
            let raw_end = (pos + core_size).min(text.len());
            let end = snap_to_char_boundary(text, raw_end);
            let end = end.max(pos + 1);
            cores.push((pos, end));
            pos = end;
        }

        // Phase 2: Extend cores with overlap
        extend_cores_to_chunks(text, &cores, overlap, &line_at)
    }

    /// Use text-splitter for intelligent boundary detection (core-first).
    ///
    /// `Semantic` / `Sentence` → `TextSplitter` (semantic text boundaries).
    /// `Markdown` → `MarkdownSplitter` (CommonMark-aware).
    fn chunk_with_text_splitter(&self, text: &str) -> Vec<Chunk> {
        let line_at = build_line_index(text);

        // Phase 1: Build cores via text-splitter (no overlap, smaller max)
        let core_size = self.config.max_size.saturating_sub(self.config.overlap).max(1);
        let chunk_config = ChunkConfig::new(core_size).with_trim(false);

        let indices: Vec<(usize, &str)> = match self.config.strategy {
            ChunkStrategy::Markdown => MarkdownSplitter::new(chunk_config)
                .chunk_indices(text)
                .collect(),
            _ => TextSplitter::new(chunk_config)
                .chunk_indices(text)
                .collect(),
        };

        let cores: Vec<(usize, usize)> = indices
            .iter()
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(offset, s)| (*offset, offset + s.len()))
            .collect();

        // Phase 2: Extend cores with overlap
        extend_cores_to_chunks(text, &cores, self.config.overlap, &line_at)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Extend non-overlapping cores into overlapping chunks.
///
/// Each core is extended by `overlap` chars on both sides (clamped to text bounds).
/// Core offsets are preserved as-is.
fn extend_cores_to_chunks(
    text: &str,
    cores: &[(usize, usize)],
    overlap: usize,
    line_at: &[usize],
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut chunk_index = 0;

    for &(core_start, core_end) in cores {
        // Extend with overlap on both sides
        let start_byte = if core_start >= overlap {
            snap_to_char_boundary(text, core_start - overlap)
        } else {
            0
        };
        let end_byte = snap_to_char_boundary(text, (core_end + overlap).min(text.len()));
        let end_byte = end_byte.max(start_byte + 1);

        let chunk_text = text[start_byte..end_byte].trim();
        if chunk_text.is_empty() {
            continue;
        }

        chunks.push(Chunk {
            text: chunk_text.to_string(),
            index: chunk_index,
            start_byte,
            end_byte,
            start_line: line_at[start_byte],
            end_line: line_at[end_byte],
            core_start_byte: core_start,
            core_end_byte: core_end,
            core_start_line: line_at[core_start],
            core_end_line: line_at[core_end],
        });
        chunk_index += 1;
    }

    chunks
}

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

    fn par_lignes(max_lines: usize, max_size: usize, overlap_lines: usize) -> Chunker {
        Chunker::new(ChunkerConfig { strategy: ChunkStrategy::Lines, max_lines, max_size, overlap_lines, overlap: 0, max_chunk_size: None, token_bound: None })
    }

    /// **A whole chunk keeps under its bound, context and merged tail included** (ticket of embarquement, 20 November
    /// 2026: 72,5 % of rag3weaver's chunks passed granite's window, because the bound held the core and each side of
    /// the context, never the whole). Long lines, a context of five lines on each side, and a short last core that
    /// the chunker merges with the previous one.
    #[test]
    fn a_whole_chunk_keeps_under_its_bound() {
        let ligne = |i: usize| format!("    let value_{i:03} = compute_something_long(argument_{i}, other_{i});");
        let mut texte = (0..120).map(ligne).collect::<Vec<_>>().join("\n");
        texte.push_str("\n}");
        let borne = 900;
        let c = Chunker::new(ChunkerConfig {
            strategy: ChunkStrategy::Lines,
            max_lines: 30,
            max_size: 1500,
            overlap_lines: 5,
            overlap: 0,
            max_chunk_size: Some(borne),
            token_bound: None,
        })
        .chunk(&texte);
        assert!(c.len() > 1, "the text is cut");
        let trop: Vec<(usize, usize)> = c.iter().filter(|k| k.text.len() > borne).map(|k| (k.index, k.text.len())).collect();
        assert!(trop.is_empty(), "chunks over the bound of {borne} bytes (index, length): {trop:?}");
        // Every line is in a core: the bound cuts the context first, never a line out of the index.
        let mut couvert = 0;
        for k in &c {
            assert_eq!(k.core_start_byte, couvert, "the cores follow each other");
            couvert = k.core_end_byte;
        }
        assert!(couvert >= texte.trim_end().len(), "the cores cover the text");
        // A text that fits `max_size` but not the bound is cut too, not kept whole.
        let court: String = (0..14).map(ligne).collect::<Vec<_>>().join("\n");
        assert!(court.len() > borne && court.len() < 1500);
        let c = Chunker::new(ChunkerConfig {
            strategy: ChunkStrategy::Lines,
            max_lines: 30,
            max_size: 1500,
            overlap_lines: 5,
            overlap: 0,
            max_chunk_size: Some(borne),
            token_bound: None,
        })
        .chunk(&court);
        assert!(c.iter().all(|k| k.text.len() <= borne), "a short text over the bound: {:?}", c.iter().map(|k| k.text.len()).collect::<Vec<_>>());
    }

    /// **Un texte qui tient est un seul chunk, tel quel.** Pas de
    /// recouvrement, pas de coupe : la découpe ne touche qu'à ce qui dépasse.
    #[test]
    fn par_lignes_un_texte_qui_tient_reste_entier() {
        let texte = (0..20).map(|i| format!("ligne {i}")).collect::<Vec<_>>().join("\n");
        let c = par_lignes(30, 1500, 5).chunk(&texte);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, texte);
    }

    /// **Au-delà, des cœurs de lignes entières et cinq lignes partagées.**
    /// 70 lignes en cœurs de 30 : 30 + 30 + 10. Le second chunk commence 5
    /// lignes avant son cœur et finit 5 lignes après ; jamais au milieu d'une
    /// ligne ; les cœurs sont disjoints et couvrent tout.
    #[test]
    fn par_lignes_les_coeurs_sont_disjoints_et_le_contexte_partage() {
        let texte = (0..70).map(|i| format!("ligne {i:02}")).collect::<Vec<_>>().join("\n");
        let c = par_lignes(30, 100_000, 5).chunk(&texte);
        assert_eq!(c.len(), 3, "{:?}", c.iter().map(|k| (k.core_start_line, k.core_end_line)).collect::<Vec<_>>());
        assert_eq!((c[0].core_start_line, c[1].core_start_line, c[2].core_start_line), (0, 30, 60));
        assert_eq!(c[0].core_end_byte, c[1].core_start_byte, "les cœurs se touchent");
        assert_eq!(c[2].core_end_byte, texte.len(), "et couvrent tout");
        assert_eq!(c[1].start_line, 25, "cinq lignes avant le cœur");
        assert!(c[1].text.starts_with("ligne 25"), "{}", c[1].text);
        assert!(c[1].text.ends_with("ligne 64"), "cinq lignes après : {}", c[1].text);
        assert!(c[0].text.starts_with("ligne 00") && c[0].text.ends_with("ligne 34"));
        for k in &c {
            assert!(!k.text.ends_with('\n') && k.text.lines().all(|l| l.starts_with("ligne")), "jamais au milieu d'une ligne");
        }
    }

    /// **La borne en caractères vaut autant que la borne en lignes**, et un
    /// reste trop court rejoint le chunk précédent.
    #[test]
    fn par_lignes_la_borne_en_caracteres_et_le_reste_court() {
        let texte = (0..12).map(|i| format!("{i:02} {}", "x".repeat(40))).collect::<Vec<_>>().join("\n");
        // 44 octets par ligne : 3 lignes par cœur à 150, 12 lignes → 4 cœurs.
        let c = par_lignes(30, 150, 1).chunk(&texte);
        assert_eq!(c.len(), 4, "{:?}", c.iter().map(|k| (k.core_start_line, k.core_end_line)).collect::<Vec<_>>());
        // 10 lignes : le dernier cœur d'une ligne (43 octets < 50) rejoint le précédent.
        let texte = (0..10).map(|i| format!("{i:02} {}", "x".repeat(40))).collect::<Vec<_>>().join("\n");
        let c = par_lignes(30, 150, 1).chunk(&texte);
        assert_eq!(c.len(), 3, "{:?}", c.iter().map(|k| (k.core_start_line, k.core_end_line)).collect::<Vec<_>>());
        assert_eq!(c[2].core_end_byte, texte.len());
    }

    /// **A line longer than the chunk size is cut too** (embarquement's ticket of 16 November 2026: 31 chunks
    /// above 12 000 characters on the bench's fresh index, up to 53 294, all from data files of one or two
    /// lines). With the code's chunking, every chunk of the bench's files stays within three chunk sizes
    /// (a core within `max_size`, at most `max_size` of context on each side), a long line's cores within
    /// `max_size`, and the cores put end to end give back the text.
    #[cfg(feature = "code")]
    #[test]
    fn a_line_longer_than_the_chunk_size_is_cut_and_the_cores_give_back_the_text() {
        let config = crate::code::default_scope_chunking();
        let chunker = par_lignes(config.max_lines, config.max_size, config.overlap_lines);
        let mut fichiers: Vec<(String, String)> = [
            "test/test_files/copy/copy_edge_case.test",
            "extension/vector/test/test_files/demo.test",
            "test/test_files/exceptions/copy/duplicated.test",
            "benchmark/queries/click/click/q24.benchmark",
            "test/answers/large_list_of_struct_answer.txt",
        ]
        .iter()
        .map(|p| (p.to_string(), std::fs::read_to_string(format!("{}/../../{p}", env!("CARGO_MANIFEST_DIR"))).expect(p)))
        .collect();
        fichiers.push(("une ligne de 53 000 caractères".into(), "é".repeat(26_500)));
        for (nom, texte) in &fichiers {
            let chunks = chunker.chunk(texte);
            let plus_long = chunks.iter().map(|c| c.text.len()).max().unwrap_or(0);
            assert!(plus_long <= 3 * config.max_size, "{nom}: a chunk of {plus_long} bytes");
            let cores: String = chunks.iter().map(|c| &texte[c.core_start_byte..c.core_end_byte]).collect();
            assert_eq!(cores, *texte, "{nom}: the cores give back the text");
            for c in &chunks {
                if !texte[c.core_start_byte..c.core_end_byte].contains('\n') && c.core_end_byte - c.core_start_byte > config.max_size {
                    panic!("{nom}: a core of {} bytes inside one line", c.core_end_byte - c.core_start_byte);
                }
            }
        }
    }

    fn semantic_chunker(max_size: usize, overlap: usize) -> Chunker {
        Chunker::new(ChunkerConfig {
            max_size,
            overlap,
            strategy: ChunkStrategy::Semantic,
            ..ChunkerConfig::default()
        })
    }

    fn fixed_chunker(max_size: usize, overlap: usize) -> Chunker {
        Chunker::new(ChunkerConfig {
            max_size,
            overlap,
            strategy: ChunkStrategy::Fixed,
            ..ChunkerConfig::default()
        })
    }

    fn markdown_chunker(max_size: usize, overlap: usize) -> Chunker {
        Chunker::new(ChunkerConfig {
            max_size,
            overlap,
            strategy: ChunkStrategy::Markdown,
            ..ChunkerConfig::default()
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
        // Single chunk: core = full range
        assert_eq!(chunks[0].core_start_byte, 0);
        assert_eq!(chunks[0].core_end_byte, text.len());
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

    // ── core offsets ─────────────────────────────────────────────────────

    #[test]
    fn cores_are_contiguous() {
        let text = "A. ".repeat(200);
        let chunks = semantic_chunker(30, 10).chunk(&text);
        assert!(chunks.len() >= 3, "need at least 3 chunks");

        // Cores should be contiguous: core_end[i] == core_start[i+1]
        for i in 0..chunks.len() - 1 {
            assert_eq!(
                chunks[i].core_end_byte, chunks[i + 1].core_start_byte,
                "cores should be contiguous at chunk {i}: end={} vs next start={}",
                chunks[i].core_end_byte, chunks[i + 1].core_start_byte,
            );
        }
    }

    #[test]
    fn cores_cover_full_text() {
        let text = "Hello world. This is a longer text. It should be chunked properly.";
        let chunks = semantic_chunker(30, 5).chunk(text);

        // First core starts at 0
        assert_eq!(chunks[0].core_start_byte, 0);
        // Last core ends at text length
        assert_eq!(chunks.last().unwrap().core_end_byte, text.len());
    }

    #[test]
    fn core_within_chunk_bounds() {
        let text = "A. ".repeat(200);
        let chunks = semantic_chunker(30, 10).chunk(&text);

        for chunk in &chunks {
            assert!(
                chunk.core_start_byte >= chunk.start_byte,
                "core_start should be >= start: core_start={}, start={}",
                chunk.core_start_byte, chunk.start_byte
            );
            assert!(
                chunk.core_end_byte <= chunk.end_byte,
                "core_end should be <= end: core_end={}, end={}",
                chunk.core_end_byte, chunk.end_byte
            );
        }
    }

    #[test]
    fn single_chunk_core_equals_full() {
        let text = "Short text";
        let chunks = semantic_chunker(100, 20).chunk(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].core_start_byte, chunks[0].start_byte);
        assert_eq!(chunks[0].core_end_byte, chunks[0].end_byte);
    }

    #[test]
    fn core_lines_within_chunk_lines() {
        let text = "line 0\nline 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7";
        let chunks = semantic_chunker(20, 8).chunk(text);

        for chunk in &chunks {
            assert!(
                chunk.core_start_line >= chunk.start_line,
                "core_start_line >= start_line"
            );
            assert!(
                chunk.core_end_line <= chunk.end_line,
                "core_end_line <= end_line"
            );
        }
    }

    // ── fixed chunking ───────────────────────────────────────────────────

    #[test]
    fn fixed_splits_at_size() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let chunks = fixed_chunker(10, 0).chunk(text);
        assert!(chunks.len() >= 2);
        // With overlap=0, core = full chunk, size <= max_size
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

    #[test]
    fn fixed_cores_contiguous() {
        let text = "abcdefghijklmnopqrstuvwxyz0123456789";
        let chunks = fixed_chunker(15, 5).chunk(text);
        assert!(chunks.len() >= 2);
        for i in 0..chunks.len() - 1 {
            assert_eq!(
                chunks[i].core_end_byte, chunks[i + 1].core_start_byte,
                "fixed cores should be contiguous"
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
