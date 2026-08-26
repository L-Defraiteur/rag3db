//! `read` et `grep` : les deux outils qu'un agent de code utilise le plus,
//! **sur une source de fichiers**, jamais sur un chemin de disque — un dépôt
//! distant n'a pas de disque, il a un instantané, un commit, une API.
//!
//! Le graphe **annote**, il ne cherche pas : un `grep` parcourt la source et
//! rapproche chaque `(fichier, ligne)` du scope le plus étroit qui la
//! contient ; un `read` compare le hash du contenu à celui que `File` porte,
//! et sait quand l'index est périmé.
//!
//! Formats retenus de l'ancienne version (docs de ragforge, relues le 25 août
//! 2026) : préfixe de ligne `00042| ` à largeur fixe, pied de page qui dit
//! l'appel suivant, tableau `| File | Line | Scope | Match |`, markdown par
//! défaut, plafonds côté serveur avec `total_found` **et** `returned`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::catalog::Catalog;
use crate::code::{FILE, SCOPE, SKIPPED_DIRS};
use crate::connection::CypherValue;

/// Clé du service `Arc<dyn FileSource>`.
pub const FILE_SOURCE_SERVICE: &str = "file_source";

pub const DEFAULT_READ_LIMIT: usize = 200;
pub const MAX_READ_LIMIT: usize = 2000;
pub const MAX_LINE_CHARS: usize = 2000;
pub const DEFAULT_GREP_LIMIT: usize = 50;
pub const MAX_GREP_LIMIT: usize = 500;
pub const MAX_CONTEXT_LINES: usize = 5;
pub const MAX_MATCH_CHARS: usize = 200;
/// Au-delà, un fichier n'est pas lu par `grep` (binaire ou généré).
pub const MAX_GREP_FILE_BYTES: usize = 2 * 1024 * 1024;

// ─── FileSource ──────────────────────────────────────────────────────────────

/// D'où viennent les fichiers, et comment on les lit. Les chemins sont
/// **relatifs et virtuels** partout : dans la base, dans les outils, dans les
/// réponses au modèle.
pub trait FileSource: Send + Sync {
    /// Identité de la source : `worktree:<racine>`, `snapshot:<étiquette>`,
    /// demain `git:<sha>`. C'est ce que `File.cursor` porte.
    fn cursor(&self) -> String;
    /// Chemins relatifs, triés.
    fn list(&self) -> Result<Vec<String>, String>;
    /// `None` si le chemin n'existe pas dans la source.
    fn read(&self, path: &str) -> Result<Option<String>, String>;
    /// Écrit (crée ou remplace). Une source en lecture seule rend `Err`.
    fn write(&self, _path: &str, _content: &str) -> Result<(), String> {
        Err(format!("{} is read-only", self.cursor()))
    }
}

fn check_relative(path: &str) -> Result<(), String> {
    if path.is_empty() || Path::new(path).is_absolute() || path.split('/').any(|c| c == "..") {
        return Err(format!("path must be relative to the source and without '..': {path}"));
    }
    Ok(())
}

/// L'arbre de travail : le disque, sous une racine.
pub struct WorkingTree {
    root: PathBuf,
}

impl WorkingTree {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl FileSource for WorkingTree {
    fn cursor(&self) -> String {
        format!("worktree:{}", self.root.display())
    }
    fn list(&self) -> Result<Vec<String>, String> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if path.is_dir() {
                    if SKIPPED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                        continue;
                    }
                    walk(&path, root, out)?;
                } else if path.is_file() {
                    out.push(path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string());
                }
            }
            Ok(())
        }
        let mut out = Vec::new();
        walk(&self.root, &self.root, &mut out).map_err(|e| format!("{}: {e}", self.root.display()))?;
        out.sort();
        Ok(out)
    }
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        check_relative(path)?;
        let full = self.root.join(path);
        match std::fs::read(&full) {
            Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", full.display())),
        }
    }
    /// Écriture atomique : fichier temporaire à côté, puis renommage — un
    /// lecteur concurrent voit l'ancien ou le nouveau, jamais un mélange.
    fn write(&self, path: &str, content: &str) -> Result<(), String> {
        check_relative(path)?;
        let full = self.root.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let tmp = full.with_extension(format!("{}.rag3weaver-tmp", full.extension().and_then(|e| e.to_str()).unwrap_or("")));
        std::fs::write(&tmp, content).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &full).map_err(|e| format!("{}: {e}", full.display()))
    }
}

/// Des contenus déjà récupérés — un dépôt distant après téléchargement, une
/// fixture de test. Aucun disque.
pub struct Snapshot {
    label: String,
    files: std::sync::RwLock<BTreeMap<String, String>>,
}

impl Snapshot {
    pub fn new(label: impl Into<String>, files: impl IntoIterator<Item = (String, String)>) -> Self {
        Self { label: label.into(), files: std::sync::RwLock::new(files.into_iter().collect()) }
    }
    pub fn insert(&self, path: impl Into<String>, content: impl Into<String>) {
        self.files.write().unwrap().insert(path.into(), content.into());
    }
}

impl FileSource for Snapshot {
    fn cursor(&self) -> String {
        format!("snapshot:{}", self.label)
    }
    fn list(&self) -> Result<Vec<String>, String> {
        Ok(self.files.read().unwrap().keys().cloned().collect())
    }
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        Ok(self.files.read().unwrap().get(path).cloned())
    }
    /// Un instantané s'édite en mémoire — c'est ce qu'un dépôt distant
    /// modifié localement avant d'être poussé est.
    fn write(&self, path: &str, content: &str) -> Result<(), String> {
        check_relative(path)?;
        self.files.write().unwrap().insert(path.to_string(), content.to_string());
        Ok(())
    }
}

// ─── Le graphe qui annote ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeRef {
    pub name: String,
    pub scope_type: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl ScopeRef {
    pub fn lines(&self) -> String {
        format!("{}-{}", self.start_line, self.end_line)
    }
}

/// Une colonne d'une ligne rendue par le catalogue, sous ses trois formes :
/// `name`, `n.name` (projection), ou dans le nœud entier `{"n": {…}}` que
/// rend `Catalog::get`.
fn col<'a>(row: &'a BTreeMap<String, CypherValue>, name: &str) -> Option<&'a CypherValue> {
    if let Some(v) = row.get(name) {
        return Some(v);
    }
    if let Some((_, v)) = row.iter().find(|(k, _)| k.rsplit('.').next() == Some(name)) {
        return Some(v);
    }
    row.values().find_map(|v| match v {
        CypherValue::Map(m) => m.get(name),
        _ => None,
    })
}

/// Les scopes d'un fichier, par ligne croissante. Vide sans catalogue.
fn scopes_of(catalog: Option<&Catalog>, path: &str) -> Result<Vec<ScopeRef>, String> {
    let Some(catalog) = catalog else { return Ok(vec![]) };
    let rows = catalog
        .find_by_field(SCOPE, "file_path", CypherValue::String(path.to_string()), &["name", "scope_type", "start_line", "end_line"])
        .map_err(|e| e.to_string())?;
    let mut scopes: Vec<ScopeRef> = rows
        .iter()
        .filter_map(|r| {
            Some(ScopeRef {
                name: col(r, "name")?.as_str()?.to_string(),
                scope_type: col(r, "scope_type")?.as_str()?.to_string(),
                start_line: col(r, "start_line")?.as_i64()? as usize,
                end_line: col(r, "end_line")?.as_i64()? as usize,
            })
        })
        .collect();
    scopes.sort_by_key(|s| (s.start_line, s.end_line));
    Ok(scopes)
}

/// Le scope le plus étroit qui contient la ligne.
fn narrowest<'a>(scopes: &'a [ScopeRef], line: usize) -> Option<&'a ScopeRef> {
    scopes
        .iter()
        .filter(|s| s.start_line <= line && line <= s.end_line)
        .min_by_key(|s| s.end_line - s.start_line)
}

/// `File.content_hash` tel qu'indexé, si le fichier est connu du catalogue.
fn indexed_hash(catalog: Option<&Catalog>, path: &str) -> Result<Option<String>, String> {
    let Some(catalog) = catalog else { return Ok(None) };
    let uuid = catalog
        .entity_uuid(FILE, &BTreeMap::from([("path".to_string(), CypherValue::String(path.to_string()))]))
        .map_err(|e| e.to_string())?;
    let row = catalog.get(FILE, &uuid).map_err(|e| e.to_string())?;
    Ok(row.and_then(|r| col(&r, "content_hash").and_then(|v| v.as_str().map(String::from))))
}

// ─── read ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    pub path: String,
    pub cursor: String,
    pub total_lines: usize,
    /// Première ligne rendue, 1-based.
    pub offset: usize,
    pub lines_read: usize,
    pub has_more: bool,
    /// `None` : le fichier n'est pas dans le catalogue (pas indexé).
    pub stale: Option<bool>,
    pub content_hash: String,
    pub indexed_hash: Option<String>,
    /// Les scopes qui intersectent la fenêtre rendue.
    pub scopes: Vec<ScopeRef>,
    /// Lignes préfixées `00042| `.
    pub text: String,
}

impl ReadResult {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("**{}** — lines {}-{} of {}", self.path, self.offset, self.offset + self.lines_read.saturating_sub(1), self.total_lines));
        if let Some(true) = self.stale {
            out.push_str(" — **INDEX STALE**: the file changed since it was indexed; scope lines below may be off");
        }
        if !self.scopes.is_empty() {
            let names: Vec<String> = self.scopes.iter().map(|s| format!("{} `{}` ({})", s.scope_type, s.name, s.lines())).collect();
            out.push_str(&format!("\nScopes: {}", names.join(", ")));
        }
        out.push_str("\n```\n");
        out.push_str(&self.text);
        if !self.text.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n");
        if self.has_more {
            out.push_str(&format!("(File has more lines. Use offset={} to continue. Total: {} lines)\n", self.offset + self.lines_read, self.total_lines));
        } else {
            out.push_str(&format!("(End of file - {} lines)\n", self.total_lines));
        }
        out
    }
}

/// Lit `path` dans la source à partir de la ligne `offset` (1-based), au plus
/// `limit` lignes, et annote depuis le catalogue s'il y en a un.
pub fn read_file(
    source: &dyn FileSource,
    catalog: Option<&Catalog>,
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<ReadResult, String> {
    let content = match source.read(path)? {
        Some(c) => c,
        None => {
            // « Vouliez-vous dire » : même nom de fichier ailleurs, ou un
            // chemin qui finit par ce qu'on a demandé — le cas classique d'un
            // préfixe de répertoire deviné (`src/dataflow/x.rs` pour `x.rs`).
            let wanted = Path::new(path).file_name().and_then(|f| f.to_str()).unwrap_or(path).to_string();
            let mut candidates: Vec<String> = source
                .list()?
                .into_iter()
                .filter(|p| p.ends_with(&format!("/{wanted}")) || *p == wanted || path.ends_with(&format!("/{p}")))
                .take(5)
                .collect();
            candidates.sort();
            return Err(if candidates.is_empty() {
                format!("no such file in {}: {path}", source.cursor())
            } else {
                format!("no such file in {}: {path} — did you mean: {}", source.cursor(), candidates.join(", "))
            });
        }
    };
    let limit = limit.clamp(1, MAX_READ_LIMIT);
    let offset = offset.max(1);
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let start = (offset - 1).min(total_lines);
    let end = (start + limit).min(total_lines);
    let mut text = String::new();
    for (i, line) in lines[start..end].iter().enumerate() {
        let n = start + i + 1;
        let shown: String = if line.chars().count() > MAX_LINE_CHARS {
            let mut s: String = line.chars().take(MAX_LINE_CHARS).collect();
            s.push_str("...");
            s
        } else {
            (*line).to_string()
        };
        text.push_str(&format!("{n:05}| {shown}\n"));
    }
    let content_hash = crate::hash::content_hash(&content);
    let indexed = indexed_hash(catalog, path)?;
    let stale = indexed.as_ref().map(|h| *h != content_hash);
    let scopes: Vec<ScopeRef> = scopes_of(catalog, path)?
        .into_iter()
        .filter(|s| s.start_line <= end && s.end_line >= start + 1)
        .collect();
    Ok(ReadResult {
        path: path.to_string(),
        cursor: source.cursor(),
        total_lines,
        offset,
        lines_read: end - start,
        has_more: end < total_lines,
        stale,
        content_hash,
        indexed_hash: indexed,
        scopes,
        text,
    })
}

// ─── grep ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct GrepOptions {
    /// Ne parcourir que les chemins qui commencent par ce préfixe.
    pub path_prefix: Option<String>,
    /// Ne parcourir que cette extension (sans le point).
    pub extension: Option<String>,
    pub case_insensitive: bool,
    pub max_results: usize,
    pub context_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub path: String,
    pub line: usize,
    pub text: String,
    pub scope: Option<ScopeRef>,
    /// `Some(true)` : le fichier a changé depuis son indexation.
    pub stale: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_after: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    pub pattern: String,
    pub cursor: String,
    pub files_searched: usize,
    pub files_skipped: usize,
    pub total_found: usize,
    pub returned: usize,
    pub matches: Vec<GrepMatch>,
}

fn escape_md(s: &str) -> String {
    s.replace('|', "\\|").replace('`', "\\`")
}

impl GrepResult {
    pub fn to_markdown(&self) -> String {
        let mut out = format!(
            "**Pattern:** `{}` | **Files:** {} searched{} | **Matches:** {}",
            escape_md(&self.pattern),
            self.files_searched,
            if self.files_skipped > 0 { format!(" ({} skipped: too large)", self.files_skipped) } else { String::new() },
            self.total_found
        );
        if self.returned < self.total_found {
            out.push_str(&format!(" — **showing {}**; narrow with path_prefix or extension", self.returned));
        }
        out.push('\n');
        if self.matches.is_empty() {
            out.push_str("\n(no match)\n");
            return out;
        }
        out.push_str("\n| File | Line | Scope | Match |\n|------|------|-------|-------|\n");
        for m in &self.matches {
            let scope = m
                .scope
                .as_ref()
                .map(|s| format!("{} `{}` ({})", s.scope_type, escape_md(&s.name), s.lines()))
                .unwrap_or_else(|| "—".to_string());
            let stale = if m.stale == Some(true) { " ⚠stale" } else { "" };
            out.push_str(&format!("| {}{} | {} | {} | `{}` |\n", escape_md(&m.path), stale, m.line, scope, escape_md(m.text.trim())));
            for c in &m.context_before {
                out.push_str(&format!("|  | ↑ | | `{}` |\n", escape_md(c.trim())));
            }
            for c in &m.context_after {
                out.push_str(&format!("|  | ↓ | | `{}` |\n", escape_md(c.trim())));
            }
        }
        out
    }
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let mut t: String = s.chars().take(max).collect();
        t.push_str("...");
        t
    } else {
        s.to_string()
    }
}

/// Regex sur chaque fichier de la source (filtré par préfixe / extension),
/// annoté depuis le catalogue. Tous les résultats sont **comptés**
/// (`total_found`), au plus `max_results` sont **rendus**.
pub fn grep_files(
    source: &dyn FileSource,
    catalog: Option<&Catalog>,
    pattern: &str,
    opts: &GrepOptions,
) -> Result<GrepResult, String> {
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(opts.case_insensitive)
        .build()
        .map_err(|e| format!("invalid regex: {e}"))?;
    let max_results = if opts.max_results == 0 { DEFAULT_GREP_LIMIT } else { opts.max_results.min(MAX_GREP_LIMIT) };
    let context = opts.context_lines.min(MAX_CONTEXT_LINES);

    let mut files_searched = 0usize;
    let mut files_skipped = 0usize;
    let mut total_found = 0usize;
    let mut matches: Vec<GrepMatch> = Vec::new();
    let mut scope_cache: HashMap<String, Vec<ScopeRef>> = HashMap::new();
    let mut stale_cache: HashMap<String, Option<bool>> = HashMap::new();

    for path in source.list()? {
        if let Some(p) = &opts.path_prefix {
            if !path.starts_with(p.as_str()) {
                continue;
            }
        }
        if let Some(ext) = &opts.extension {
            if Path::new(&path).extension().and_then(|e| e.to_str()) != Some(ext.trim_start_matches('.')) {
                continue;
            }
        }
        let Some(content) = source.read(&path)? else { continue };
        if content.len() > MAX_GREP_FILE_BYTES {
            files_skipped += 1;
            continue;
        }
        files_searched += 1;
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !re.is_match(line) {
                continue;
            }
            total_found += 1;
            if matches.len() >= max_results {
                continue; // on compte, on ne rend plus
            }
            let scopes = match scope_cache.get(&path) {
                Some(s) => s,
                None => {
                    let s = scopes_of(catalog, &path)?;
                    scope_cache.entry(path.clone()).or_insert(s)
                }
            };
            let stale = match stale_cache.get(&path) {
                Some(s) => *s,
                None => {
                    let s = indexed_hash(catalog, &path)?.map(|h| h != crate::hash::content_hash(&content));
                    stale_cache.insert(path.clone(), s);
                    s
                }
            };
            let line_no = i + 1;
            matches.push(GrepMatch {
                path: path.clone(),
                line: line_no,
                text: clip(line, MAX_MATCH_CHARS),
                scope: narrowest(scopes, line_no).cloned(),
                stale,
                context_before: (i.saturating_sub(context)..i).map(|j| clip(lines[j], MAX_MATCH_CHARS)).collect(),
                context_after: (i + 1..(i + 1 + context).min(lines.len())).map(|j| clip(lines[j], MAX_MATCH_CHARS)).collect(),
            });
        }
    }
    Ok(GrepResult {
        pattern: pattern.to_string(),
        cursor: source.cursor(),
        files_searched,
        files_skipped,
        total_found,
        returned: matches.len(),
        matches,
    })
}

// ─── list ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEntry {
    pub path: String,
    /// `None` : pas lu (`with_state = false`).
    pub lines: Option<usize>,
    /// `Some(true)` : connu du catalogue.
    pub indexed: Option<bool>,
    /// `Some(true)` : connu et modifié depuis.
    pub stale: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub cursor: String,
    pub path_prefix: Option<String>,
    pub total: usize,
    pub returned: usize,
    pub entries: Vec<ListEntry>,
}

pub const DEFAULT_LIST_LIMIT: usize = 200;
pub const MAX_LIST_LIMIT: usize = 2000;

impl ListResult {
    pub fn to_markdown(&self) -> String {
        let mut out = format!(
            "**Files:** {} under `{}`{}",
            self.total,
            self.path_prefix.as_deref().unwrap_or(""),
            if self.returned < self.total { format!(" — **showing {}**; narrow with path_prefix", self.returned) } else { String::new() }
        );
        out.push('\n');
        for e in &self.entries {
            let state = match (e.indexed, e.stale) {
                (Some(true), Some(true)) => " ⚠stale",
                (Some(true), _) => " ✓indexed",
                (Some(false), _) => " (not indexed)",
                _ => "",
            };
            match e.lines {
                Some(n) => out.push_str(&format!("- `{}` ({n} lines){state}\n", e.path)),
                None => out.push_str(&format!("- `{}`{state}\n", e.path)),
            }
        }
        out
    }
}

/// Les fichiers de la source sous un préfixe. Avec `with_state`, chaque
/// fichier est lu pour compter ses lignes et comparer son hash à l'index.
pub fn list_files(
    source: &dyn FileSource,
    catalog: Option<&Catalog>,
    path_prefix: Option<&str>,
    limit: usize,
    with_state: bool,
) -> Result<ListResult, String> {
    let limit = if limit == 0 { DEFAULT_LIST_LIMIT } else { limit.min(MAX_LIST_LIMIT) };
    let all: Vec<String> = source
        .list()?
        .into_iter()
        .filter(|p| path_prefix.map_or(true, |pre| p.starts_with(pre)))
        .collect();
    let total = all.len();
    let mut entries = Vec::new();
    for path in all.into_iter().take(limit) {
        let (lines, indexed, stale) = if with_state {
            let content = source.read(&path)?.unwrap_or_default();
            let indexed = indexed_hash(catalog, &path)?;
            let stale = indexed.as_ref().map(|h| *h != crate::hash::content_hash(&content));
            (Some(content.lines().count()), catalog.map(|_| indexed.is_some()), stale)
        } else {
            (None, None, None)
        };
        entries.push(ListEntry { path, lines, indexed, stale });
    }
    Ok(ListResult { cursor: source.cursor(), path_prefix: path_prefix.map(String::from), total, returned: entries.len(), entries })
}

// ─── edit ────────────────────────────────────────────────────────────────────

/// Ce qu'on fait au fichier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOp {
    /// Remplace `old` (qui doit apparaître **exactement une fois**) par `new`.
    Replace { old: String, new: String },
    /// Remplace tout le contenu (crée le fichier s'il n'existe pas).
    Write { content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    pub path: String,
    pub cursor: String,
    pub created: bool,
    pub lines_before: usize,
    pub lines_after: usize,
    /// Première ligne touchée (1-based), pour relire autour.
    pub first_changed_line: Option<usize>,
    pub content_hash: String,
    /// Ce que la ré-ingestion du fichier a fait, s'il y avait un catalogue.
    pub reingest: Option<ReingestReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReingestReport {
    pub scopes_upserted: usize,
    pub scopes_deleted: usize,
    pub relations: usize,
    pub failed: usize,
}

impl EditResult {
    pub fn to_markdown(&self) -> String {
        let mut out = format!(
            "**{}** {} — {} → {} lines, hash `{}`",
            self.path,
            if self.created { "created" } else { "edited" },
            self.lines_before,
            self.lines_after,
            &self.content_hash[..12]
        );
        if let Some(l) = self.first_changed_line {
            out.push_str(&format!(" — first change at line {l} (read with offset={} to check)", l.saturating_sub(3).max(1)));
        }
        match &self.reingest {
            Some(r) => out.push_str(&format!(
                "\nIndex updated: {} scopes upserted, {} removed, {} relations{}",
                r.scopes_upserted,
                r.scopes_deleted,
                r.relations,
                if r.failed > 0 { format!(", {} failed", r.failed) } else { String::new() }
            )),
            None => out.push_str("\n(no catalogue: index not updated)"),
        }
        out.push('\n');
        out
    }
}

/// Retire les préfixes `00042| ` d'un texte recopié depuis `read` — le
/// détail d'ergonomie le plus payant de l'ancienne version : le modèle peut
/// coller ce qu'il a lu.
pub fn strip_line_prefixes(text: &str) -> String {
    let re = regex::Regex::new(r"(?m)^\d{5}\| ").unwrap();
    // Seulement si TOUTES les lignes non vides portent le préfixe : sinon un
    // code qui contiendrait la forme `00001| ` serait abîmé.
    let all = text.lines().filter(|l| !l.trim().is_empty()).all(|l| re.is_match(l));
    if all && text.lines().any(|l| !l.trim().is_empty()) {
        re.replace_all(text, "").into_owned()
    } else {
        text.to_string()
    }
}

/// Applique `op` à `path` dans la source, puis ré-ingère le fichier si un
/// catalogue est là : ses scopes sont réécrits (identités `hashsafe` stables),
/// ceux qui ont disparu sont supprimés. Les relations inter-fichiers vers ce
/// fichier ne sont pas recalculées — dette nommée.
pub fn edit_file(
    source: &dyn FileSource,
    catalog: Option<&mut Catalog>,
    path: &str,
    op: &EditOp,
) -> Result<EditResult, String> {
    let before = source.read(path)?;
    let created = before.is_none();
    let before_text = before.unwrap_or_default();
    let after_text = match op {
        EditOp::Replace { old, new } => {
            if created {
                return Err(format!("no such file in {}: {path} (use a full write to create it)", source.cursor()));
            }
            let old = strip_line_prefixes(old);
            let new = strip_line_prefixes(new);
            let n = before_text.matches(old.as_str()).count();
            if n == 0 {
                return Err(format!("old text not found in {path} (copy it exactly from read; line-number prefixes are stripped)"));
            }
            if n > 1 {
                return Err(format!("old text appears {n} times in {path}; include more context to make it unique"));
            }
            before_text.replacen(old.as_str(), &new, 1)
        }
        EditOp::Write { content } => strip_line_prefixes(content),
    };
    let first_changed_line = before_text
        .lines()
        .zip(after_text.lines())
        .position(|(a, b)| a != b)
        .map(|i| i + 1)
        .or_else(|| if before_text != after_text { Some(before_text.lines().count().min(after_text.lines().count()) + 1) } else { None });
    source.write(path, &after_text)?;
    let content_hash = crate::hash::content_hash(&after_text);
    let reingest = match catalog {
        Some(catalog) => Some(reingest_file(catalog, source, path, &after_text)?),
        None => None,
    };
    Ok(EditResult {
        path: path.to_string(),
        cursor: source.cursor(),
        created,
        lines_before: before_text.lines().count(),
        lines_after: after_text.lines().count(),
        first_changed_line,
        content_hash,
        reingest,
    })
}

/// Ré-ingère un seul fichier : analyse seule (références locales et
/// `DEFINED_IN` ; l'inter-fichiers attend la résolution contre la base),
/// suppression des scopes disparus, upsert du reste.
pub fn reingest_file(catalog: &mut Catalog, source: &dyn FileSource, path: &str, content: &str) -> Result<ReingestReport, String> {
    use crate::code::{analyze, FILE, SCOPE};
    let cursor = source.cursor();
    let (root, virtual_source) = match cursor.strip_prefix("worktree:") {
        Some(root) => (root.to_string(), false),
        None => ("/".to_string(), true),
    };
    let mut analysis = analyze(&root, vec![(path.to_string(), content.to_string())]);
    for f in &mut analysis.files {
        f.cursor = cursor.clone();
        if virtual_source {
            f.absolute_path.clear();
        }
    }
    // Scopes connus du fichier, moins ceux que l'analyse produit encore.
    let known = catalog
        .find_by_field(SCOPE, "file_path", CypherValue::String(path.to_string()), &["key"])
        .map_err(|e| e.to_string())?;
    let new_keys: std::collections::HashSet<&str> = analysis.scopes.iter().map(|s| s.key.as_str()).collect();
    let mut deleted = 0usize;
    for row in &known {
        let Some(key) = col(row, "key").and_then(|v| v.as_str()) else { continue };
        if !new_keys.contains(key) {
            let uuid = catalog
                .entity_uuid(SCOPE, &BTreeMap::from([("key".to_string(), CypherValue::String(key.to_string()))]))
                .map_err(|e| e.to_string())?;
            catalog.delete(SCOPE, &uuid).map_err(|e| e.to_string())?;
            deleted += 1;
        }
    }
    let _ = FILE;
    let report = catalog.ingest_code(&analysis).map_err(|e| e.to_string())?;
    Ok(ReingestReport { scopes_upserted: report.scopes, scopes_deleted: deleted, relations: report.relations, failed: report.failed })
}

/// Ce que les nœuds mettent sur leur port : JSON structuré, ou markdown
/// (rendu tel quel au modèle — `render_port_value` rend une chaîne JSON nue).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolFormat {
    #[default]
    Markdown,
    Json,
}

impl ToolFormat {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "markdown" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            other => Err(format!("format '{other}': expected markdown | json")),
        }
    }
}

/// `Arc<dyn FileSource>` depuis le registre de services.
pub fn source_service(ctx: &crate::dataflow::NodeContext) -> Option<Arc<dyn FileSource>> {
    ctx.service::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_format_has_no_alias() {
        assert!(ToolFormat::parse("markdown").is_ok() && ToolFormat::parse("json").is_ok());
        assert!(ToolFormat::parse("md").is_err(), "l'enum de la fiche dit markdown | json, le parseur aussi");
    }


    fn snapshot() -> Snapshot {
        Snapshot::new(
            "t",
            [
                ("src/a.rs".to_string(), "fn alpha() {\n    beta();\n}\n\nfn beta() {}\n".to_string()),
                ("src/b.rs".to_string(), (0..300).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n")),
                ("README.md".to_string(), "# Alpha\nbeta is called by alpha\n".to_string()),
            ],
        )
    }

    #[test]
    fn read_numbers_lines_and_paginates() {
        let s = snapshot();
        let r = read_file(&s, None, "src/b.rs", 1, 10).unwrap();
        assert_eq!(r.total_lines, 300);
        assert_eq!(r.lines_read, 10);
        assert!(r.has_more);
        assert!(r.text.starts_with("00001| line 0\n"));
        assert!(r.stale.is_none(), "no catalogue → no staleness verdict");
        let md = r.to_markdown();
        assert!(md.contains("Use offset=11 to continue. Total: 300 lines"), "{md}");
        let last = read_file(&s, None, "src/b.rs", 295, 50).unwrap();
        assert_eq!(last.lines_read, 6);
        assert!(!last.has_more);
        assert!(last.to_markdown().contains("(End of file - 300 lines)"));
        assert!(read_file(&s, None, "nope.rs", 1, 10).is_err());
        let err = read_file(&s, None, "src/dataflow/b.rs", 1, 10).unwrap_err();
        assert!(err.contains("did you mean: src/b.rs"), "{err}");
        let err = read_file(&s, None, "b.rs", 1, 10).unwrap_err();
        assert!(err.contains("did you mean: src/b.rs"), "{err}");
    }

    #[test]
    fn grep_counts_everything_returns_bounded() {
        let s = snapshot();
        let r = grep_files(&s, None, "beta", &GrepOptions::default()).unwrap();
        assert_eq!(r.files_searched, 3);
        assert_eq!(r.total_found, 3, "{:?}", r.matches);
        assert_eq!(r.returned, 3);
        assert_eq!(r.matches[0].path, "README.md");
        assert!(r.matches.iter().all(|m| m.scope.is_none() && m.stale.is_none()));

        let limited = grep_files(&s, None, "line", &GrepOptions { max_results: 5, ..Default::default() }).unwrap();
        assert_eq!(limited.total_found, 300);
        assert_eq!(limited.returned, 5);
        assert!(limited.to_markdown().contains("**showing 5**"));

        let only_rs = grep_files(&s, None, "beta", &GrepOptions { extension: Some("rs".into()), ..Default::default() }).unwrap();
        assert_eq!(only_rs.files_searched, 2);
        assert_eq!(only_rs.total_found, 2);
        let prefixed = grep_files(&s, None, "alpha", &GrepOptions { path_prefix: Some("src/".into()), case_insensitive: true, ..Default::default() }).unwrap();
        assert_eq!(prefixed.total_found, 1);
        assert!(grep_files(&s, None, "(", &GrepOptions::default()).is_err(), "invalid regex is an error, not a silent empty");
    }

    #[test]
    fn grep_context_is_capped_and_markdown_escapes_pipes() {
        let s = snapshot();
        s.insert("c.txt", "a|b\nneedle `x`\nafter\n");
        let r = grep_files(&s, None, "needle", &GrepOptions { context_lines: 99, ..Default::default() }).unwrap();
        let m = &r.matches[0];
        assert_eq!(m.context_before, vec!["a|b"]);
        assert_eq!(m.context_after, vec!["after"]);
        let md = r.to_markdown();
        assert!(md.contains("a\\|b"), "{md}");
        assert!(md.contains("\\`x\\`"), "{md}");
    }

    #[test]
    fn list_counts_and_filters() {
        let s = snapshot();
        let r = list_files(&s, None, Some("src/"), 0, true).unwrap();
        assert_eq!(r.total, 2);
        assert_eq!(r.entries[0].path, "src/a.rs");
        assert_eq!(r.entries[1].lines, Some(300));
        assert!(r.entries.iter().all(|e| e.indexed.is_none()), "no catalogue → no verdict");
        let bounded = list_files(&s, None, None, 1, false).unwrap();
        assert_eq!((bounded.total, bounded.returned), (3, 1));
        assert!(bounded.to_markdown().contains("**showing 1**"));
    }

    #[test]
    fn edit_replace_is_unique_and_strips_read_prefixes() {
        let s = snapshot();
        // Texte recopié depuis `read`, préfixes compris.
        let op = EditOp::Replace { old: "00002|     beta();".into(), new: "00002|     beta(); // twice\n00003|     beta();".into() };
        let r = edit_file(&s, None, "src/a.rs", &op).unwrap();
        assert!(!r.created);
        assert_eq!((r.lines_before, r.lines_after), (5, 6));
        assert_eq!(r.first_changed_line, Some(2));
        assert!(r.reingest.is_none());
        let after = s.read("src/a.rs").unwrap().unwrap();
        assert!(after.contains("    beta(); // twice\n    beta();\n"), "{after}");
        // Ambigu : `beta();` apparaît maintenant deux fois.
        let err = edit_file(&s, None, "src/a.rs", &EditOp::Replace { old: "beta();".into(), new: "x".into() }).unwrap_err();
        assert!(err.contains("appears 2 times"), "{err}");
        let err = edit_file(&s, None, "src/a.rs", &EditOp::Replace { old: "gamma();".into(), new: "x".into() }).unwrap_err();
        assert!(err.contains("not found"), "{err}");
        let err = edit_file(&s, None, "new.rs", &EditOp::Replace { old: "a".into(), new: "b".into() }).unwrap_err();
        assert!(err.contains("full write"), "{err}");
        let w = edit_file(&s, None, "new.rs", &EditOp::Write { content: "fn fresh() {}\n".into() }).unwrap();
        assert!(w.created && w.lines_after == 1);
        assert_eq!(s.read("new.rs").unwrap().unwrap(), "fn fresh() {}\n");
    }

    #[test]
    fn prefixes_are_stripped_only_when_every_line_has_one() {
        assert_eq!(strip_line_prefixes("00001| a\n00002| b\n"), "a\nb\n");
        assert_eq!(strip_line_prefixes("00001| a\nb\n"), "00001| a\nb\n");
        assert_eq!(strip_line_prefixes("plain"), "plain");
    }

    #[test]
    fn working_tree_refuses_escaping_paths() {
        let wt = WorkingTree::new(env!("CARGO_MANIFEST_DIR"));
        assert!(wt.read("../Cargo.toml").is_err());
        assert!(wt.read("/etc/passwd").is_err());
        assert!(wt.read("Cargo.toml").unwrap().is_some());
        assert!(wt.read("does-not-exist.rs").unwrap().is_none());
        assert!(wt.cursor().starts_with("worktree:"));
    }
}
