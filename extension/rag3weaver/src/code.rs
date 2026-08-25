//! Le code comme graphe (doc 02 du 25 août 2026) : `File`, `Scope`,
//! `Library` et leurs relations, extraits par `codeparsers` et persistés par le
//! chemin d'ingestion ordinaire du catalogue.
//!
//! - [`register_code_schema`] déclare les trois entités et les neuf relations
//!   — le `CODE_SCHEMA` de février, avec `hashsafe` pour des identités stables
//!   (`File` par chemin, `Scope` par clé déterministe, `Library` par nom).
//! - [`analyze`] parse un jeu de sources (chemin relatif + contenu) et rend
//!   une [`CodeAnalysis`] : des enregistrements plats, sérialisables, prêts à
//!   être ingérés — ou inspectés.
//! - [`Catalog::ingest_code`] les persiste : `ingest_entities` × 3, `link` par
//!   relation, `drain`.
//!
//! `File` **n'est jamais chunké** au sens du contenu : son seul champ de
//! contenu est son chemin (un chunk de quelques octets), ce qui le rend
//! cherchable par nom sans en faire un article de catalogue. Il porte le
//! `content_hash` et le curseur de source qui font de lui l'index du fichier
//! réel — `read` compare, et sait quand l'index est périmé.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};

use codeparsers::parallel::project_parser::{
    detect_language_from_path, is_code_parser_supported, ParseProjectOptions, ProjectParser,
    ProjectParserOptions,
};
use codeparsers::relationship_resolution::types::RelationshipType;
use codeparsers::scope_extraction::types::ScopeInfoType;

use crate::catalog::{Catalog, CatalogError};
use crate::config::{ChunkStrategy, ChunkingConfig, EntityConfig, FieldType, SimpleFieldDef};
use crate::connection::CypherValue;
use crate::records::RefOrUuid;

pub const FILE: &str = "File";
pub const SCOPE: &str = "Scope";
pub const LIBRARY: &str = "Library";

/// `(relation, from, to)` — les neuf du `CODE_SCHEMA` de février.
pub const RELATIONS: [(&str, &str, &str); 9] = [
    ("DEFINED_IN", SCOPE, FILE),
    ("CONSUMES", SCOPE, SCOPE),
    ("CONSUMED_BY", SCOPE, SCOPE),
    ("INHERITS_FROM", SCOPE, SCOPE),
    ("IMPLEMENTS", SCOPE, SCOPE),
    ("PARENT_OF", SCOPE, SCOPE),
    ("HAS_PARENT", SCOPE, SCOPE),
    ("DECORATES", SCOPE, SCOPE),
    ("USES_LIBRARY", SCOPE, LIBRARY),
];

/// Répertoires qu'on ne parse jamais.
pub const SKIPPED_DIRS: [&str; 8] = ["target", "node_modules", ".git", "dist", "build", ".venv", "venv", "__pycache__"];

// ─── Schéma ──────────────────────────────────────────────────────────────────

fn field(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, ..Default::default() }
}
fn title_and_content(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, is_title: true, is_content: true, ..Default::default() }
}
fn title(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, is_title: true, ..Default::default() }
}
fn content(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, is_content: true, ..Default::default() }
}

/// Chunking des scopes : 1000 / 100 depuis février (« ~250 tokens »). À
/// dériver de la fenêtre du modèle d'embedding quand on saura la lire.
pub fn default_scope_chunking() -> ChunkingConfig {
    ChunkingConfig { max_size: 1000, overlap: 100, strategy: ChunkStrategy::Semantic, ..Default::default() }
}

pub fn file_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("path".into(), title_and_content(FieldType::String));
    fields.insert("absolute_path".into(), field(FieldType::String));
    fields.insert("language".into(), field(FieldType::String));
    fields.insert("lines_of_code".into(), field(FieldType::Integer));
    fields.insert("size_bytes".into(), field(FieldType::Integer));
    fields.insert("content_hash".into(), field(FieldType::String));
    // Curseur de source (commit git, instant de balayage…) — vide tant que
    // `FileSource` n'existe pas ; le champ est là pour ne pas migrer.
    fields.insert("cursor".into(), field(FieldType::String));
    EntityConfig { fields, hashsafe: Some(vec!["path".into()]), ..Default::default() }
}

pub fn scope_config(chunking: ChunkingConfig) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), title(FieldType::String));
    fields.insert("signature".into(), content(FieldType::Text));
    fields.insert("content".into(), content(FieldType::Text));
    fields.insert("docstring".into(), content(FieldType::Text));
    fields.insert("scope_type".into(), field(FieldType::String));
    fields.insert("file_path".into(), field(FieldType::String));
    fields.insert("parent_name".into(), field(FieldType::String));
    fields.insert("language".into(), field(FieldType::String));
    fields.insert("start_line".into(), field(FieldType::Integer));
    fields.insert("end_line".into(), field(FieldType::Integer));
    fields.insert("start_byte".into(), field(FieldType::Integer));
    fields.insert("end_byte".into(), field(FieldType::Integer));
    // Clé déterministe de `codeparsers` : `blake3(file:name:type:signature)`,
    // stable quand les lignes bougent.
    fields.insert("key".into(), field(FieldType::String));
    EntityConfig { fields, chunking, hashsafe: Some(vec!["key".into()]), ..Default::default() }
}

pub fn library_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), title_and_content(FieldType::String));
    fields.insert("import_path".into(), field(FieldType::String));
    EntityConfig { fields, hashsafe: Some(vec!["name".into()]), ..Default::default() }
}

/// Déclare `File`, `Scope`, `Library` et les neuf relations. Idempotent
/// (`register_entity` / `register_relation` le sont).
pub fn register_code_schema(catalog: &mut Catalog, scope_chunking: ChunkingConfig) -> Result<(), CatalogError> {
    catalog.register_entity(FILE, file_config())?;
    catalog.register_entity(SCOPE, scope_config(scope_chunking))?;
    catalog.register_entity(LIBRARY, library_config())?;
    for (rel, from, to) in RELATIONS {
        catalog.register_relation(rel, from, to)?;
    }
    Ok(())
}

// ─── Analyse ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub absolute_path: String,
    pub language: String,
    pub lines_of_code: usize,
    pub size_bytes: usize,
    pub content_hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScopeRecord {
    pub key: String,
    pub name: String,
    pub scope_type: String,
    pub signature: String,
    pub content: String,
    pub docstring: String,
    pub file_path: String,
    pub parent_name: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryRecord {
    pub name: String,
    pub import_path: String,
    pub symbols: Vec<String>,
}

/// Une arête, par entité et clé d'identité (pas par uuid : l'uuid est
/// l'affaire du catalogue, `hashsafe` le dérive de la clé).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRelation {
    pub rel: String,
    pub from_entity: String,
    pub from_key: String,
    pub to_entity: String,
    pub to_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub root: String,
    pub files: Vec<FileRecord>,
    pub scopes: Vec<ScopeRecord>,
    pub libraries: Vec<LibraryRecord>,
    pub relations: Vec<CodeRelation>,
    /// Fichiers écartés ou en échec : `(chemin, raison)`.
    pub skipped: Vec<(String, String)>,
    /// Relations dont une extrémité n'a pas été retrouvée.
    pub relations_dropped: usize,
    pub parse_ms: u128,
    pub relation_ms: u128,
}

fn language_name(path: &str) -> String {
    detect_language_from_path(path)
        .map(|l| format!("{l:?}").to_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Parse des sources (chemin **relatif** à `root`, contenu) et résout les
/// relations. Aucun accès disque : c'est l'appelant qui lit — un arbre de
/// travail, un commit git, ou une fixture.
pub fn analyze(root: &str, sources: Vec<(String, String)>) -> CodeAnalysis {
    let mut content_map = HashMap::new();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut sizes: HashMap<String, usize> = HashMap::new();
    for (rel, content) in sources {
        if !is_code_parser_supported(&rel) {
            skipped.push((rel, "unsupported extension".to_string()));
            continue;
        }
        let abs = Path::new(root).join(&rel).to_string_lossy().to_string();
        sizes.insert(abs.clone(), content.len());
        content_map.insert(abs.clone(), content);
        files.push(abs);
    }

    let parser = ProjectParser::new(ProjectParserOptions { verbose: false });
    let result = parser.parse_project(ParseProjectOptions {
        root: root.to_string(),
        files,
        content_map: Some(content_map),
        resolve_relationships: Some(true),
    });
    for e in &result.errors {
        skipped.push((relative(root, &e.file), e.error.clone()));
    }

    let mut analysis = CodeAnalysis {
        root: root.to_string(),
        skipped,
        parse_ms: result.stats.parse_time_ms,
        relation_ms: result.stats.relationship_time_ms.unwrap_or(0),
        ..Default::default()
    };

    // Fichiers
    for (abs, fa) in &result.files {
        let rel = relative(root, abs);
        analysis.files.push(FileRecord {
            path: rel,
            absolute_path: abs.clone(),
            language: language_name(abs),
            lines_of_code: fa.total_lines,
            size_bytes: sizes.get(abs).copied().unwrap_or(0),
            content_hash: fa.content_hash.clone().unwrap_or_default(),
        });
    }
    analysis.files.sort_by(|a, b| a.path.cmp(&b.path));

    let Some(rels) = result.relationships else {
        return analysis;
    };

    // uuid codeparsers → (entité, clé). Les scopes portent leur uuid comme clé.
    let mut identity: HashMap<&str, (&str, String)> = HashMap::new();
    for (uuid, entry) in &rels.uuid_mapping {
        identity.insert(uuid.as_str(), (SCOPE, entry.uuid.clone()));
    }
    for (path, info) in &rels.files {
        identity.insert(info.uuid.as_str(), (FILE, path.clone()));
    }
    for (name, lib) in &rels.external_libraries {
        identity.insert(lib.uuid.as_str(), (LIBRARY, name.clone()));
        analysis.libraries.push(LibraryRecord {
            name: name.clone(),
            import_path: name.clone(),
            symbols: lib.symbols.clone(),
        });
    }
    analysis.libraries.sort_by(|a, b| a.name.cmp(&b.name));

    // Scopes : le ScopeInfo complet (contenu, docstring, octets) est dans
    // `result.files` ; son uuid dans `uuid_mapping`. Rapprochement par
    // (fichier relatif, nom, type, ligne de début).
    let mut by_position: HashMap<(String, String, String, usize), String> = HashMap::new();
    for (uuid, entry) in &rels.uuid_mapping {
        by_position.insert((entry.file.clone(), entry.name.clone(), entry.r#type.clone(), entry.start_line), uuid.clone());
    }
    for (abs, fa) in &result.files {
        let rel = relative(root, abs);
        let language = language_name(abs);
        for s in &fa.scopes {
            let type_str = scope_type_name(&s.r#type).to_string();
            let Some(key) = by_position.get(&(rel.clone(), s.name.clone(), type_str.clone(), s.scope_start_line)) else {
                continue;
            };
            let content = if s.content_dedented.is_empty() { s.content.clone() } else { s.content_dedented.clone() };
            analysis.scopes.push(ScopeRecord {
                key: key.clone(),
                name: s.name.clone(),
                scope_type: type_str,
                signature: s.signature.clone(),
                content,
                docstring: s.docstring.clone().unwrap_or_default(),
                file_path: rel.clone(),
                parent_name: s.parent.clone().unwrap_or_default(),
                language: language.clone(),
                start_line: s.scope_start_line,
                end_line: s.scope_end_line,
                start_byte: s.scope_start_byte,
                end_byte: s.scope_end_byte,
            });
        }
    }
    analysis.scopes.sort_by(|a, b| (&a.file_path, a.start_line, &a.name).cmp(&(&b.file_path, b.start_line, &b.name)));

    // Relations
    let kept: std::collections::HashSet<&str> = RELATIONS.iter().map(|(r, _, _)| *r).collect();
    for r in &rels.relationships {
        let name = relation_name(&r.r#type);
        if !kept.contains(name) {
            continue;
        }
        let (Some(from), Some(to)) = (identity.get(r.from_uuid.as_str()), identity.get(r.to_uuid.as_str())) else {
            analysis.relations_dropped += 1;
            continue;
        };
        analysis.relations.push(CodeRelation {
            rel: name.to_string(),
            from_entity: from.0.to_string(),
            from_key: from.1.clone(),
            to_entity: to.0.to_string(),
            to_key: to.1.clone(),
        });
    }
    analysis
}

/// Les noms que le résolveur de `codeparsers` met dans `ScopeMappingEntry.type`
/// (sa fonction privée `scope_type_str`) — pas les noms serde de l'enum.
fn scope_type_name(t: &ScopeInfoType) -> &'static str {
    match t {
        ScopeInfoType::Class => "class",
        ScopeInfoType::Interface => "interface",
        ScopeInfoType::Function => "function",
        ScopeInfoType::Method => "method",
        ScopeInfoType::Enum => "enum",
        ScopeInfoType::TypeAlias => "type_alias",
        ScopeInfoType::Namespace => "namespace",
        ScopeInfoType::Module => "module",
        ScopeInfoType::Variable => "variable",
        ScopeInfoType::Lambda => "lambda",
        ScopeInfoType::Constant => "constant",
        ScopeInfoType::Block => "block",
    }
}

fn relation_name(t: &RelationshipType) -> &'static str {
    match t {
        RelationshipType::CONSUMES => "CONSUMES",
        RelationshipType::CONSUMEDBY => "CONSUMED_BY",
        RelationshipType::INHERITSFROM => "INHERITS_FROM",
        RelationshipType::IMPLEMENTS => "IMPLEMENTS",
        RelationshipType::PARENTOF => "PARENT_OF",
        RelationshipType::HASPARENT => "HAS_PARENT",
        RelationshipType::DECORATES => "DECORATES",
        RelationshipType::DECORATEDBY => "DECORATED_BY",
        RelationshipType::DEFINEDIN => "DEFINED_IN",
        RelationshipType::USESLIBRARY => "USES_LIBRARY",
    }
}

fn relative(root: &str, abs: &str) -> String {
    Path::new(abs)
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| abs.to_string())
}

/// Lit les sources d'un répertoire : extensions supportées, [`SKIPPED_DIRS`]
/// ignorés, fichiers non-UTF-8 écartés. Chemins relatifs à `root`, triés.
pub fn read_sources(root: &str) -> std::io::Result<Vec<(String, String)>> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if SKIPPED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                    continue;
                }
                walk(&path, root, out)?;
            } else if is_code_parser_supported(&path.to_string_lossy()) {
                let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    out.push((rel, content));
                }
            }
        }
        Ok(())
    }
    let root_path = Path::new(root);
    let mut out = Vec::new();
    walk(root_path, root_path, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

// ─── Ingestion ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeIngestReport {
    pub files: usize,
    pub scopes: usize,
    pub libraries: usize,
    pub relations: usize,
    pub failed: usize,
}

fn s(v: &str) -> CypherValue {
    CypherValue::String(v.to_string())
}
fn i(v: usize) -> CypherValue {
    CypherValue::Int(v as i64)
}

impl FileRecord {
    pub fn data(&self) -> BTreeMap<String, CypherValue> {
        BTreeMap::from([
            ("path".into(), s(&self.path)),
            ("absolute_path".into(), s(&self.absolute_path)),
            ("language".into(), s(&self.language)),
            ("lines_of_code".into(), i(self.lines_of_code)),
            ("size_bytes".into(), i(self.size_bytes)),
            ("content_hash".into(), s(&self.content_hash)),
            ("cursor".into(), s("")),
        ])
    }
}

impl ScopeRecord {
    pub fn data(&self) -> BTreeMap<String, CypherValue> {
        BTreeMap::from([
            ("key".into(), s(&self.key)),
            ("name".into(), s(&self.name)),
            ("scope_type".into(), s(&self.scope_type)),
            ("signature".into(), s(&self.signature)),
            ("content".into(), s(&self.content)),
            ("docstring".into(), s(&self.docstring)),
            ("file_path".into(), s(&self.file_path)),
            ("parent_name".into(), s(&self.parent_name)),
            ("language".into(), s(&self.language)),
            ("start_line".into(), i(self.start_line)),
            ("end_line".into(), i(self.end_line)),
            ("start_byte".into(), i(self.start_byte)),
            ("end_byte".into(), i(self.end_byte)),
        ])
    }
}

impl LibraryRecord {
    pub fn data(&self) -> BTreeMap<String, CypherValue> {
        BTreeMap::from([
            ("name".into(), s(&self.name)),
            ("import_path".into(), s(&self.import_path)),
        ])
    }
}

/// Champ `hashsafe` d'une entité de code → sa clé dans une [`CodeRelation`].
fn key_data(entity: &str, key: &str) -> BTreeMap<String, CypherValue> {
    let field = match entity {
        FILE => "path",
        SCOPE => "key",
        _ => "name",
    };
    BTreeMap::from([(field.to_string(), s(key))])
}

impl Catalog {
    /// Persiste une [`CodeAnalysis`] : entités par `ingest_entities`, relations
    /// par `link` (uuid dérivés des clés, comme le catalogue les dérivera),
    /// puis `drain`. Le schéma doit être déclaré ([`register_code_schema`]).
    pub fn ingest_code(&mut self, analysis: &CodeAnalysis) -> Result<CodeIngestReport, CatalogError> {
        let mut report = CodeIngestReport::default();

        let files = self.ingest_entities(FILE, analysis.files.iter().map(FileRecord::data).collect())?;
        report.files = files.processed;
        report.failed += files.failed;
        let scopes = self.ingest_entities(SCOPE, analysis.scopes.iter().map(ScopeRecord::data).collect())?;
        report.scopes = scopes.processed;
        report.failed += scopes.failed;
        let libs = self.ingest_entities(LIBRARY, analysis.libraries.iter().map(LibraryRecord::data).collect())?;
        report.libraries = libs.processed;
        report.failed += libs.failed;

        for r in &analysis.relations {
            let from = self.entity_uuid(&r.from_entity, &key_data(&r.from_entity, &r.from_key))?;
            let to = self.entity_uuid(&r.to_entity, &key_data(&r.to_entity, &r.to_key))?;
            self.link(&r.rel, RefOrUuid::Uuid(from), RefOrUuid::Uuid(to), BTreeMap::new())?;
        }
        let linked = self.drain();
        report.relations = linked.processed;
        report.failed += linked.failed;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_SRC: &str = "use serde::Serialize;\n\npub struct Point {\n    x: i32,\n}\n\nimpl Point {\n    pub fn norm(&self) -> i32 {\n        self.x.abs()\n    }\n}\n\npub fn twice(p: &Point) -> i32 {\n    p.norm() * 2\n}\n";

    #[test]
    fn analyze_yields_files_scopes_and_relations_by_key() {
        let a = analyze("/virtual", vec![("a.rs".into(), RUST_SRC.into()), ("README.md".into(), "# no".into())]);
        assert_eq!(a.files.len(), 1);
        assert_eq!(a.files[0].path, "a.rs");
        assert_eq!(a.files[0].language, "rust");
        assert!(!a.files[0].content_hash.is_empty());
        assert_eq!(a.skipped.len(), 1, "{:?}", a.skipped);
        let names: Vec<&str> = a.scopes.iter().map(|s| s.name.as_str()).collect();
        let norm = a.scopes.iter().find(|s| s.name == "norm").unwrap_or_else(|| panic!("norm not in {names:?}"));
        assert_eq!(norm.scope_type, "method");
        assert!(norm.end_byte > norm.start_byte);
        assert!(RUST_SRC[norm.start_byte..norm.end_byte].contains("fn norm"));
        assert!(a.relations.iter().any(|r| r.rel == "DEFINED_IN" && r.from_key == norm.key && r.to_entity == FILE && r.to_key == "a.rs"),
            "{:?}", a.relations.iter().map(|r| (&r.rel, &r.from_entity, &r.to_entity)).collect::<Vec<_>>());
        let twice = a.scopes.iter().find(|s| s.name == "twice").expect("twice");
        let named = |k: &str| a.scopes.iter().find(|s| s.key == k).map(|s| s.name.clone()).unwrap_or_else(|| k.to_string());
        let edges: Vec<String> = a.relations.iter().map(|r| format!("{} {} {}", named(&r.from_key), r.rel, if r.to_entity == SCOPE { named(&r.to_key) } else { r.to_key.clone() })).collect();
        // `p.norm()` sur un paramètre n'est pas résolu (pas d'inférence de
        // type) ; l'usage du type `Point` par `twice` l'est, et la hiérarchie.
        let point_struct = a.scopes.iter().find(|s| s.name == "Point" && s.scope_type == "class").expect("struct Point");
        assert!(a.relations.iter().any(|r| r.rel == "CONSUMES" && r.from_key == twice.key && r.to_key == point_struct.key),
            "twice CONSUMES Point expected; edges: {edges:#?}");
        assert!(a.relations.iter().any(|r| r.rel == "PARENT_OF" && r.to_key == norm.key), "Point PARENT_OF norm; edges: {edges:#?}");
        assert!(a.relations.iter().all(|r| a.scopes.iter().any(|s| s.key == r.from_key)), "every from is a known scope");
    }

    #[test]
    fn schema_is_consistent_with_records() {
        for (cfg, data) in [
            (file_config(), FileRecord::default().data()),
            (scope_config(default_scope_chunking()), ScopeRecord::default().data()),
            (library_config(), LibraryRecord::default().data()),
        ] {
            cfg.validate().unwrap();
            for k in data.keys() {
                assert!(cfg.fields.contains_key(k), "record field '{k}' missing from schema");
            }
            for h in cfg.hashsafe.as_ref().unwrap() {
                assert!(data.contains_key(h), "hashsafe field '{h}' missing from record");
            }
        }
    }
}

#[cfg(test)]
mod own_source_tests {
    /// Parse notre propre `src/dataflow/` — sans base, sans nœud : isole le
    /// parseur. Ignoré par défaut (quelques secondes), lancé explicitement.
    #[test]
    #[ignore]
    fn analyze_own_dataflow_dir_does_not_crash() {
        let root = format!("{}/src/dataflow", env!("CARGO_MANIFEST_DIR"));
        let sources = super::read_sources(&root).unwrap();
        eprintln!("{} sources", sources.len());
        for (path, content) in &sources {
            eprintln!("  parsing {path} ({} bytes)", content.len());
            let a = super::analyze(&root, vec![(path.clone(), content.clone())]);
            eprintln!("    {} scopes, {} skipped", a.scopes.len(), a.skipped.len());
        }
        eprintln!("all together:");
        let a = super::analyze(&root, sources);
        eprintln!("  {} files, {} scopes, {} relations ({} dropped), {} skipped, parse {} ms, relations {} ms",
            a.files.len(), a.scopes.len(), a.relations.len(), a.relations_dropped, a.skipped.len(), a.parse_ms, a.relation_ms);
        assert!(a.relations.len() > 200);
    }
}
