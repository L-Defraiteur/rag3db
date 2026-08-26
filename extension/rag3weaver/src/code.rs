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
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};
use codeparsers::scope_extraction::types::ScopeInfoType;

use crate::catalog::{Catalog, CatalogError};
use crate::config::{ChunkStrategy, ChunkingConfig, EntityConfig, FieldType, SimpleFieldDef};
use crate::connection::CypherValue;
use crate::records::RefOrUuid;

pub const FILE: &str = "File";
pub const SCOPE: &str = "Scope";
pub const LIBRARY: &str = "Library";

/// `(relation, from, to)` — les neuf du `CODE_SCHEMA` de février.
/// Le point de rendez-vous entre celui qui définit un nom et ceux qui
/// l'attendent — voir [doc 17](../../docs/25-aout-2026-18h58/17-relations-a-travers-les-lots.md).
/// Invisible pour l'agent au sens où il n'a rien à y faire, mais parfaitement
/// interrogeable : « qui mentionne `merge_port_values` ? » est une vraie
/// question.
pub const SYMBOL: &str = "Symbol";

pub const RELATIONS: [(&str, &str, &str); 11] = [
    ("DEFINED_IN", SCOPE, FILE),
    ("CONSUMES", SCOPE, SCOPE),
    ("CONSUMED_BY", SCOPE, SCOPE),
    ("INHERITS_FROM", SCOPE, SCOPE),
    ("IMPLEMENTS", SCOPE, SCOPE),
    ("PARENT_OF", SCOPE, SCOPE),
    ("HAS_PARENT", SCOPE, SCOPE),
    ("DECORATES", SCOPE, SCOPE),
    ("USES_LIBRARY", SCOPE, LIBRARY),
    // La couche de rendez-vous : ce qu'un scope offre, ce qu'il attend.
    ("DEFINES", SCOPE, SYMBOL),
    ("MENTIONS", SCOPE, SYMBOL),
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
    // **Absolu dans sa source**, toujours (doc 04 v3). Pas relatif à la
    // racine d'analyse, qui n'est qu'un point de vue.
    fields.insert("path".into(), title_and_content(FieldType::String));
    // D'où viennent les octets : `file` pour le système de fichiers local,
    // `snapshot:…` pour un instantané. Connu à l'ingestion, jamais deviné.
    fields.insert("source".into(), field(FieldType::String));
    // Coordonnées : d'autres façons de nommer le même fichier, produites par
    // les fournisseurs souscrits ([`crate::origin::Coordinates`]). Des champs
    // comme les autres — donc `hashsafe` peut les prendre, et la politique
    // d'identité se change sans une ligne de moteur.
    fields.insert("repo".into(), field(FieldType::String));
    fields.insert("repo_path".into(), field(FieldType::String));
    fields.insert("revision".into(), field(FieldType::String));
    fields.insert("absolute_path".into(), field(FieldType::String));
    fields.insert("language".into(), field(FieldType::String));
    fields.insert("lines_of_code".into(), field(FieldType::Integer));
    fields.insert("size_bytes".into(), field(FieldType::Integer));
    fields.insert("content_hash".into(), field(FieldType::String));
    // Curseur de source (commit git, instant de balayage…) — vide tant que
    // `FileSource` n'existe pas ; le champ est là pour ne pas migrer.
    fields.insert("cursor".into(), field(FieldType::String));
    EntityConfig {
        fields,
        // La politique par défaut : **copie de travail**. Un fichier, un
        // nœud, mis à jour sur place ; la révision est une propriété. Un
        // gestionnaire de commits déclarerait `["repo", "revision",
        // "repo_path"]` et aurait un nœud par révision — même moteur, même
        // schéma, autre configuration (doc 04 §10).
        hashsafe: Some(vec!["source".into(), "path".into()]),
        return_fields: Some(vec!["language".into(), "lines_of_code".into(), "cursor".into()]),
        ..Default::default()
    }
}

pub fn scope_config(chunking: ChunkingConfig) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), title(FieldType::String));
    fields.insert("signature".into(), content(FieldType::Text));
    fields.insert("content".into(), content(FieldType::Text));
    fields.insert("docstring".into(), content(FieldType::Text));
    fields.insert("scope_type".into(), field(FieldType::String));
    fields.insert("file_path".into(), field(FieldType::String));
    // La source du fichier qui le contient — le même nom absolu peut exister
    // dans deux sources, ce sont deux fichiers.
    fields.insert("source".into(), field(FieldType::String));
    // La coordonnée portable, dénormalisée depuis `File` : c'est elle qu'un
    // domaine d'agent filtrera, et on ne veut pas une jointure par scope.
    fields.insert("repo".into(), field(FieldType::String));
    fields.insert("parent_name".into(), field(FieldType::String));
    fields.insert("language".into(), field(FieldType::String));
    fields.insert("start_line".into(), field(FieldType::Integer));
    fields.insert("end_line".into(), field(FieldType::Integer));
    fields.insert("start_byte".into(), field(FieldType::Integer));
    fields.insert("end_byte".into(), field(FieldType::Integer));
    // Clé déterministe de `codeparsers` : `blake3(file:name:type:signature)`,
    // stable quand les lignes bougent.
    fields.insert("key".into(), field(FieldType::String));
    EntityConfig {
        fields,
        chunking,
        hashsafe: Some(vec!["key".into()]),
        // Ce qu'un résultat de recherche doit dire pour qu'on puisse le lire.
        return_fields: Some(vec!["file_path".into(), "start_line".into(), "end_line".into(), "scope_type".into(), "parent_name".into()]),
        ..Default::default()
    }
}

pub fn library_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), title_and_content(FieldType::String));
    fields.insert("import_path".into(), field(FieldType::String));
    EntityConfig { fields, hashsafe: Some(vec!["name".into()]), ..Default::default() }
}
/// Un nom, et rien d'autre. `hashsafe` sur le nom : l'uuid se calcule sans
/// requête, ce qui rend le rendez-vous gratuit.
pub fn symbol_config() -> EntityConfig {
    let mut fields = HashMap::new();
    // `title_and_content` et pas `title` seul : le catalogue refuse une entité
    // sans champ de contenu (« toute entité est cherchable »). Un `Symbol`
    // paie donc le pipeline complet — découpage, index plein texte — pour un
    // nom de vingt caractères. C'est 12,5 s sur 3 275 symboles, et c'est le
    // premier levier si l'ingestion devient gênante.
    fields.insert("name".into(), title_and_content(FieldType::String));
    EntityConfig {
        fields,
        hashsafe: Some(vec!["name".into()]),
        // BM25 seul, et sans chunks. Un nom de symbole n'a rien à gagner
        // d'un vecteur — et le défaut `HYBRID` faisait calculer et stocker
        // un embedding pour chacun des 3 275 symboles de `src/dataflow`,
        // ce que personne n'avait voulu. Le plein texte, lui, reste entier :
        // son index vit sur la table parente, et le mode BM25 `Symbol` est
        // fait pour les identifiants.
        signals: crate::search::SearchSignals::BM25,
        chunked: Some(false),
        ..Default::default()
    }
}


/// Déclare `File`, `Scope`, `Library` et les neuf relations. Idempotent
/// (`register_entity` / `register_relation` le sont).
pub fn register_code_schema(catalog: &mut Catalog, scope_chunking: ChunkingConfig) -> Result<(), CatalogError> {
    catalog.register_entity(FILE, file_config())?;
    catalog.register_entity(SCOPE, scope_config(scope_chunking))?;
    catalog.register_entity(LIBRARY, library_config())?;
    catalog.register_entity(SYMBOL, symbol_config())?;
    for (rel, from, to) in RELATIONS {
        if rel == "MENTIONS" {
            // Le rendez-vous porte le **genre** de l'arête à poser quand la
            // cible arrivera : sans lui, un `IMPLEMENTS` dont l'interface est
            // ingérée au lot suivant se matérialiserait en `CONSUMES`, et
            // l'ordre d'ingestion changerait le graphe (doc 17 §10).
            let mut props = HashMap::new();
            props.insert(
                "kind".to_string(),
                crate::config::FieldDef {
                    field_type: FieldType::String,
                    title_for: None,
                    content_for: None,
                    boost: None,
                    default_value: None,
                },
            );
            catalog.register_relation_with(rel, from, to, props)?;
            continue;
        }
        catalog.register_relation(rel, from, to)?;
    }
    Ok(())
}

// ─── Analyse ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileRecord {
    /// **Absolu dans sa source** — pas relatif à la racine d'analyse, qui
    /// n'est qu'un point de vue (doc 04).
    pub path: String,
    /// D'où viennent les octets : `file`, `snapshot:…`.
    #[serde(default)]
    pub source: String,
    /// Les autres façons de nommer ce fichier, par fournisseur souscrit :
    /// `repo`, `repo_path`, `revision`. Vide quand personne ne sait.
    #[serde(default)]
    pub coordinates: BTreeMap<String, String>,
    /// Vide pour une source virtuelle (instantané, dépôt distant).
    pub absolute_path: String,
    pub language: String,
    pub lines_of_code: usize,
    pub size_bytes: usize,
    pub content_hash: String,
    /// Identité de la source (`worktree:…`, `snapshot:…`) — voir
    /// [`crate::code_tools::FileSource::cursor`]. Vide par [`analyze`],
    /// rempli par [`analyze_source`].
    #[serde(default)]
    pub cursor: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScopeRecord {
    pub key: String,
    /// La source du fichier qui le contient.
    #[serde(default)]
    pub source: String,
    /// La coordonnée portable du dépôt, dénormalisée depuis `File`.
    #[serde(default)]
    pub repo: String,
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
    /// Toute référence externe d'un scope, par nom : `(clé du scope, nom
    /// cherché, **genre** de la relation à faire)`. Ce ne sont pas des
    /// échecs, ce sont des **rendez-vous** — le symbole sera peut-être défini
    /// par une ingestion ultérieure, et celui qui est déjà défini peut être
    /// détruit puis recréé
    /// ([doc 17](../../docs/25-aout-2026-18h58/17-relations-a-travers-les-lots.md)).
    ///
    /// Le genre est porté ici parce qu'il ne dépend **que de la source et du
    /// nom** : sans lui, un `IMPLEMENTS` dont l'interface arrive au lot
    /// suivant se matérialiserait en `CONSUMES`, et l'ordre d'ingestion
    /// changerait le graphe.
    pub pending: Vec<(String, String, String)>,
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
    analyze_with(root, sources, "")
}

/// La source des octets d'un poste : **le système de fichiers**, pas la
/// racine de l'arbre de travail. C'est ce qui fait qu'ouvrir une source sur
/// `/projet` ou sur `/projet/src` donne la même identité — il n'y a rien à
/// faire converger, c'est la même chose.
pub const LOCAL_SOURCE: &str = "file";

/// D'où viennent les octets, à partir du curseur d'une [`crate::code_tools::FileSource`].
pub fn source_id(cursor: &str) -> String {
    if cursor.is_empty() || cursor.starts_with("worktree:") {
        LOCAL_SOURCE.to_string()
    } else {
        cursor.to_string()
    }
}

/// [`analyze`], en disant d'où viennent les fichiers.
///
/// `cursor` sert à une seule chose, mais elle est décisive : une source sans
/// système de fichiers — un instantané, un dépôt distant — **est sa propre
/// origine**. Sans le curseur, on irait chercher une ancre sur un disque qui
/// ne contient pas ces fichiers.
pub fn analyze_with(root: &str, sources: Vec<(String, String)>, cursor: &str) -> CodeAnalysis {
    let mut content_map = HashMap::new();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut sizes: HashMap<String, usize> = HashMap::new();
    // L'identité de chaque fichier : `chemin d'analyse → (source, chemin
    // absolu dans cette source, coordonnées)`. Aucune découverte, aucune
    // heuristique — la source est **connue** à l'ingestion (doc 04 v3).
    let source = source_id(cursor);
    let virtual_source = source != LOCAL_SOURCE;
    let registry = crate::origin::CoordinateRegistry::default();
    let mut named: HashMap<String, (String, BTreeMap<String, String>)> = HashMap::new();
    for (rel, content) in sources {
        if !is_code_parser_supported(&rel) {
            skipped.push((rel, "unsupported extension".to_string()));
            continue;
        }
        let abs = Path::new(root).join(&rel).to_string_lossy().to_string();
        // Une source sans disque n'a pas de chemin absolu : son nom dans la
        // source *est* son identité, et personne ne peut la coordonner.
        let (name, coords) = if virtual_source {
            (rel.clone(), BTreeMap::new())
        } else {
            (abs.clone(), registry.of(Path::new(&abs)))
        };
        named.insert(rel.clone(), (name, coords));
        sizes.insert(abs.clone(), content.len());
        content_map.insert(abs.clone(), content);
        files.push(abs);
    }
    let identity_of = |rel: &str| -> (String, BTreeMap<String, String>) {
        named.get(rel).cloned().unwrap_or_else(|| (rel.to_string(), BTreeMap::new()))
    };

    let parser = ProjectParser::new(ProjectParserOptions { verbose: false });
    let result = parser.parse_project(ParseProjectOptions {
        root: root.to_string(),
        files,
        content_map: Some(content_map),
        resolve_relationships: Some(true),
        // Une référence n'est portée que par le scope le plus interne qui la
        // contient : ni les références de niveau fichier attribuées à chaque
        // scope, ni celles des méthodes remontées à la classe. Sans ça, sur
        // notre propre code, 47 relations par scope — `PortType` « consommé »
        // par 1 347 scopes (doc 04 du 25 août).
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            ..Default::default()
        }),
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
        let (name, coordinates) = identity_of(&relative(root, abs));
        analysis.files.push(FileRecord {
            path: name,
            source: source.clone(),
            coordinates,
            absolute_path: abs.clone(),
            language: language_name(abs),
            lines_of_code: fa.total_lines,
            size_bytes: sizes.get(abs).copied().unwrap_or(0),
            content_hash: fa.content_hash.clone().unwrap_or_default(),
            cursor: String::new(),
        });
    }
    analysis.files.sort_by(|a, b| a.path.cmp(&b.path));

    let Some(rels) = result.relationships else {
        return analysis;
    };

    // L'identité d'un scope, la nôtre (voir `stable_scope_keys`).
    let stable = stable_scope_keys(&rels.uuid_mapping, &named, &source);

    // uuid codeparsers → (entité, clé).
    let mut identity: HashMap<&str, (&str, String)> = HashMap::new();
    for (uuid, _entry) in &rels.uuid_mapping {
        let Some(key) = stable.get(uuid.as_str()) else { continue };
        identity.insert(uuid.as_str(), (SCOPE, key.clone()));
    }
    for (path, info) in &rels.files {
        identity.insert(info.uuid.as_str(), (FILE, identity_of(path).0));
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
        let Some(key) = stable.get(uuid.as_str()) else { continue };
        by_position.insert((entry.file.clone(), entry.name.clone(), entry.r#type.clone(), entry.start_line), key.clone());
    }
    for (abs, fa) in &result.files {
        let rel = relative(root, abs);
        let (indexed_name, coords) = identity_of(&rel);
        let repo = coords.get("repo").cloned().unwrap_or_default();
        let language = language_name(abs);
        for s in &fa.scopes {
            let type_str = scope_type_name(&s.r#type).to_string();
            let Some(key) = by_position.get(&(rel.clone(), s.name.clone(), type_str.clone(), s.scope_start_line)) else {
                continue;
            };
            let content = if s.content_dedented.is_empty() { s.content.clone() } else { s.content_dedented.clone() };
            analysis.scopes.push(ScopeRecord {
                key: key.clone(),
                source: source.clone(),
                repo: repo.clone(),
                name: s.name.clone(),
                scope_type: type_str,
                signature: s.signature.clone(),
                content,
                docstring: s.docstring.clone().unwrap_or_default(),
                file_path: indexed_name.clone(),
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
    // ── Ce que le lot référence, par nom ────────────────────────────────
    //
    // **Toutes** les références externes, y compris celles que le résolveur
    // du lot a su relier lui-même. Ne garder que ses abandons rendait la
    // couche de rendez-vous incomplète : le graphe savait *qu'*une arête
    // existe, pas *pourquoi*. Quand un scope est détruit puis recréé — un
    // `edit` qui change une signature change sa clé — les arêtes entrantes
    // meurent avec lui, et sans trace de la référence rien ne peut les
    // refaire sans relire les fichiers appelants (doc 17 §2 bis).
    //
    // Les `Builtin` et les `LocalScope` restent écartés : résolus, ou hors
    // projet. Les bibliothèques aussi — elles ont leur propre entité.
    let libraries: std::collections::HashSet<&str> =
        analysis.libraries.iter().map(|l| l.name.as_str()).collect();
    for (abs, fa) in &result.files {
        let rel = relative(root, abs);
        for sc in &fa.scopes {
            let Some(key) = by_position.get(&(
                rel.clone(),
                sc.name.clone(),
                scope_type_name(&sc.r#type).to_string(),
                sc.scope_start_line,
            )) else {
                continue;
            };
            let mut seen = std::collections::HashSet::new();
            for r in &sc.identifier_references {
                use codeparsers::scope_extraction::types::IdentifierReferenceKind as K;
                if matches!(r.kind, Some(K::Builtin) | Some(K::LocalScope)) {
                    continue;
                }
                let id = r.identifier.as_str();
                if id.is_empty()
                    || id == sc.name
                    || libraries.contains(id)
                    || !seen.insert(id.to_string())
                {
                    continue;
                }
                let kind = relation_name(&codeparsers::relationship_resolution::relationship_resolver::detect_relationship_type_by_name(
                    sc, id, "", r.context.as_deref(),
                ));
                analysis.pending.push((key.clone(), id.to_string(), kind.to_string()));
            }
            // Les clauses d'héritage ne passent pas toujours par les
            // références d'identifiants — `codeparsers` les résout à part.
            // Sans ça, `class X implements Y` dans un fichier ingéré seul ne
            // laisse aucune trace.
            for clause in sc.heritage_clauses.iter().flatten() {
                let kind = match clause.clause {
                    codeparsers::scope_extraction::types::HeritageClauseClause::Implements => "IMPLEMENTS",
                    _ => "INHERITS_FROM",
                };
                for t in &clause.types {
                    if t.is_empty() || t == &sc.name || libraries.contains(t.as_str()) {
                        continue;
                    }
                    analysis.pending.push((key.clone(), t.clone(), kind.to_string()));
                }
            }
        }
    }
    analysis.pending.sort();
    analysis.pending.dedup();

    fold_lambdas(&mut analysis);
    dedupe_relations(&mut analysis);
    analysis
}

/// L'identité d'un scope — la nôtre, pas celle de `codeparsers`.
///
/// `codeparsers` dérive son uuid de la **signature**, et quand il n'y en a
/// pas, du **contenu**. Deux conséquences, toutes deux vérifiées :
///
/// - changer une signature détruit le scope, et **toutes ses arêtes
///   entrantes** meurent avec lui ;
/// - toucher au corps d'un scope sans signature — un module, un fichier —
///   fait exactement la même chose, à chaque édition.
///
/// La couche `Symbol` sait refaire les `CONSUMES` après coup ; elle ne sait
/// pas refaire un `IMPLEMENTS`. Mieux vaut donc ne rien détruire.
///
/// Notre identité ne dépend ni de la signature ni du contenu :
/// `fichier#parent.nom:type`, plus un **rang** qui ne départage que des
/// homonymes de même parent, de même type, dans le même fichier — les
/// surcharges. Dans un langage qui n'en a pas, il n'apparaît jamais.
///
/// Ce qui change encore une identité : renommer, changer de parent, changer
/// de fichier. C'est-à-dire exactement ce qui *est* un autre symbole.
fn stable_scope_keys(
    mapping: &codeparsers::relationship_resolution::types::UuidToScopeMapping,
    named: &HashMap<String, (String, BTreeMap<String, String>)>,
    source: &str,
) -> HashMap<String, String> {
    // Les homonymes de même parent et de même type, par fichier.
    let mut groups: HashMap<(&str, &str, &str, &str), Vec<(usize, &str)>> = HashMap::new();
    for (uuid, e) in mapping {
        groups
            .entry((
                e.file.as_str(),
                e.parent.as_deref().unwrap_or(""),
                e.name.as_str(),
                e.r#type.as_str(),
            ))
            .or_default()
            .push((e.start_line, uuid.as_str()));
    }

    let mut keys = HashMap::with_capacity(mapping.len());
    for ((file, parent, name, typ), mut members) in groups {
        // Le fichier se nomme dans sa source, pas dans la racine d'analyse.
        let file = named.get(file).map(|(n, _)| n.clone()).unwrap_or_else(|| file.to_string());
        let origin = source;
        // Par ligne, puis par uuid : deux surcharges sur la même ligne
        // restent départagées de façon déterministe.
        members.sort_unstable();
        let qualified = if parent.is_empty() { name.to_string() } else { format!("{parent}.{name}") };
        for (rank, (_, uuid)) in members.into_iter().enumerate() {
            let key = if rank == 0 {
                format!("{origin}#{file}#{qualified}:{typ}")
            } else {
                format!("{origin}#{file}#{qualified}:{typ}#{rank}")
            };
            keys.insert(uuid.to_string(), key);
        }
    }
    keys
}

/// Une fermeture n'est pas une entité qu'on cherche par son nom : ses
/// références sont attribuées au scope nommé qui l'englobe, et elle disparaît
/// des entités. Sur notre propre code : 244 « Closure » sur 1 402 scopes, et
/// 10 648 relations CONSUMES portées par elles.
fn fold_lambdas(a: &mut CodeAnalysis) {
    let mut to_parent: HashMap<String, String> = HashMap::new();
    for l in a.scopes.iter().filter(|s| s.scope_type == "lambda") {
        // Le scope nommé le plus étroit du même fichier qui contient la fermeture.
        let parent = a
            .scopes
            .iter()
            .filter(|p| {
                p.scope_type != "lambda"
                    && p.file_path == l.file_path
                    && p.start_line <= l.start_line
                    && p.end_line >= l.end_line
            })
            .min_by_key(|p| p.end_line - p.start_line);
        if let Some(p) = parent {
            to_parent.insert(l.key.clone(), p.key.clone());
        }
    }
    if to_parent.is_empty() {
        return;
    }
    // Résolution transitive (fermeture dans une fermeture).
    let resolve = |k: &str| -> String {
        let mut cur = k.to_string();
        for _ in 0..8 {
            match to_parent.get(&cur) {
                Some(p) => cur = p.clone(),
                None => break,
            }
        }
        cur
    };
    a.scopes.retain(|s| !to_parent.contains_key(&s.key));
    let mut kept = Vec::with_capacity(a.relations.len());
    for mut r in a.relations.drain(..) {
        let from_is_lambda = to_parent.contains_key(&r.from_key);
        let to_is_lambda = r.to_entity == SCOPE && to_parent.contains_key(&r.to_key);
        if (from_is_lambda || to_is_lambda)
            && matches!(r.rel.as_str(), "PARENT_OF" | "HAS_PARENT" | "DEFINED_IN")
        {
            continue; // la hiérarchie de la fermeture n'a plus de sens
        }
        if from_is_lambda {
            r.from_key = resolve(&r.from_key);
        }
        if to_is_lambda {
            r.to_key = resolve(&r.to_key);
        }
        if r.from_entity == r.to_entity && r.from_key == r.to_key {
            continue;
        }
        kept.push(r);
    }
    a.relations = kept;
}

fn dedupe_relations(a: &mut CodeAnalysis) {
    let mut seen: std::collections::HashSet<(String, String, String, String, String)> = Default::default();
    a.relations.retain(|r| {
        seen.insert((r.rel.clone(), r.from_entity.clone(), r.from_key.clone(), r.to_entity.clone(), r.to_key.clone()))
    });
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

/// [`analyze`] sur tout ce qu'une [`crate::code_tools::FileSource`] contient
/// de parsable, avec `File.cursor` = l'identité de la source et
/// `absolute_path` vide pour une source virtuelle.
pub fn analyze_source(source: &dyn crate::code_tools::FileSource) -> Result<CodeAnalysis, String> {
    let (root, virtual_source) = match source.cursor().strip_prefix("worktree:") {
        Some(root) => (root.to_string(), false),
        None => ("/".to_string(), true),
    };
    let mut sources = Vec::new();
    for path in source.list()? {
        if !is_code_parser_supported(&path) {
            continue;
        }
        if let Some(content) = source.read(&path)? {
            sources.push((path, content));
        }
    }
    let cursor = source.cursor();
    let mut analysis = analyze_with(&root, sources, &cursor);
    for f in &mut analysis.files {
        f.cursor = cursor.clone();
        if virtual_source {
            f.absolute_path.clear();
        }
    }
    Ok(analysis)
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
    /// Symboles touchés par ce lot (définis ou attendus).
    pub symbols: usize,
    /// Relations `CONSUMES` créées **après coup**, en reliant ce que le lot
    /// attendait à ce que la base connaissait, et l'inverse. C'est la mesure
    /// de ce qu'une résolution intra-lot laissait tomber.
    pub linked_across_batches: usize,
    /// Rendez-vous restés en attente : personne ne définit encore ce nom.
    /// Une incomplétude qui se **compte** plutôt que de se taire.
    pub still_pending: usize,
    /// Noms écartés parce que plusieurs scopes les définissent : une
    /// relation manquante vaut mieux qu'une relation fausse.
    pub ambiguous: usize,
    /// Millisecondes par phase — sans ça, « l'ingestion est lente » n'a pas
    /// de suite possible.
    pub entities_ms: u128,
    pub relations_ms: u128,
    pub symbols_ms: u128,
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
            ("source".into(), s(&self.source)),
            ("repo".into(), s(self.coordinates.get("repo").map(String::as_str).unwrap_or(""))),
            ("repo_path".into(), s(self.coordinates.get("repo_path").map(String::as_str).unwrap_or(""))),
            ("revision".into(), s(self.coordinates.get("revision").map(String::as_str).unwrap_or(""))),
            ("absolute_path".into(), s(&self.absolute_path)),
            ("language".into(), s(&self.language)),
            ("lines_of_code".into(), i(self.lines_of_code)),
            ("size_bytes".into(), i(self.size_bytes)),
            ("content_hash".into(), s(&self.content_hash)),
            ("cursor".into(), s(&self.cursor)),
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
            ("source".into(), s(&self.source)),
            ("repo".into(), s(&self.repo)),
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
        let phase = std::time::Instant::now();

        let files = self.ingest_entities(FILE, analysis.files.iter().map(FileRecord::data).collect())?;
        report.files = files.processed;
        report.failed += files.failed;
        let scopes = self.ingest_entities(SCOPE, analysis.scopes.iter().map(ScopeRecord::data).collect())?;
        report.scopes = scopes.processed;
        report.failed += scopes.failed;
        let libs = self.ingest_entities(LIBRARY, analysis.libraries.iter().map(LibraryRecord::data).collect())?;
        report.libraries = libs.processed;
        report.failed += libs.failed;

        report.entities_ms = phase.elapsed().as_millis();
        let phase = std::time::Instant::now();

        for r in &analysis.relations {
            let from = self.entity_uuid(&r.from_entity, &key_data(&r.from_entity, &r.from_key))?;
            let to = self.entity_uuid(&r.to_entity, &key_data(&r.to_entity, &r.to_key))?;
            self.link(&r.rel, RefOrUuid::Uuid(from), RefOrUuid::Uuid(to), BTreeMap::new())?;
        }
        let linked = self.drain();
        report.relations = linked.processed;
        report.failed += linked.failed;
        report.relations_ms = phase.elapsed().as_millis();

        let phase = std::time::Instant::now();
        self.resolve_across_batches(analysis, &mut report)?;
        report.symbols_ms = phase.elapsed().as_millis();
        Ok(report)
    }

    /// La couche de rendez-vous : ce que le lot **offre**, ce qu'il
    /// **attend**, et la matérialisation dans les deux sens.
    ///
    /// C'est ce qui rend l'ingestion indépendante de l'ordre : un fichier
    /// ajouté seul retrouve ce qui existait, et l'existant retrouve ce que
    /// le fichier apporte — sans ré-analyser le dossier
    /// ([doc 17](../../docs/25-aout-2026-18h58/17-relations-a-travers-les-lots.md)).
    fn resolve_across_batches(
        &mut self,
        analysis: &CodeAnalysis,
        report: &mut CodeIngestReport,
    ) -> Result<(), CatalogError> {
        use std::collections::{BTreeMap as Map, BTreeSet};

        // Les noms en jeu : ceux que le lot définit, ceux qu'il attend.
        let offered: BTreeSet<&str> = analysis.scopes.iter().map(|s| s.name.as_str()).collect();
        let expected: BTreeSet<&str> = analysis.pending.iter().map(|(_, n, _)| n.as_str()).collect();
        let names: BTreeSet<&str> = offered.union(&expected).copied().collect();
        if names.is_empty() {
            return Ok(());
        }

        // Un symbole par nom — `hashsafe`, donc idempotent.
        let records: Vec<Map<String, CypherValue>> = names
            .iter()
            .map(|n| {
                let mut d = Map::new();
                d.insert("name".into(), s(n));
                d
            })
            .collect();
        report.symbols = records.len();
        let ingested = self.ingest_entities(SYMBOL, records)?;
        report.failed += ingested.failed;

        let symbol_uuid = |cat: &Self, name: &str| -> Result<String, CatalogError> {
            let mut d = Map::new();
            d.insert("name".into(), s(name));
            cat.entity_uuid(SYMBOL, &d)
        };

        // Ce que le lot offre, et ce qu'il attend.
        for sc in &analysis.scopes {
            let from = self.entity_uuid(SCOPE, &key_data(SCOPE, &sc.key))?;
            let to = symbol_uuid(self, &sc.name)?;
            self.link("DEFINES", RefOrUuid::Uuid(from), RefOrUuid::Uuid(to), BTreeMap::new())?;
        }
        for (scope_key, name, kind) in &analysis.pending {
            let from = self.entity_uuid(SCOPE, &key_data(SCOPE, scope_key))?;
            let to = symbol_uuid(self, name)?;
            // Le genre voyage avec le rendez-vous : c'est lui qui décide de
            // l'arête à poser quand la cible arrivera.
            let props = BTreeMap::from([("kind".to_string(), s(kind))]);
            self.link("MENTIONS", RefOrUuid::Uuid(from), RefOrUuid::Uuid(to), props)?;
        }
        let drained = self.drain();
        report.failed += drained.failed;

        // Matérialisation, dans les deux sens. **Deux requêtes en tout** :
        // une par relation, en `UNWIND` sur tous les symboles du lot. Une
        // requête par symbole coûtait 2,5 fois le temps d'ingestion.
        let uuids: Vec<String> = names.iter().map(|n| symbol_uuid(self, n)).collect::<Result<_, _>>()?;
        let definers_by_symbol = self.linked_from_many("DEFINES", &uuids)?;
        let mentioners_by_symbol = self.linked_from_many_with_kind("MENTIONS", &uuids, true)?;
        for sym in &uuids {
            let no_definer: Vec<String> = Vec::new();
            let empty: Vec<(String, String)> = Vec::new();
            let definers = definers_by_symbol.get(sym).unwrap_or(&no_definer);
            if definers.len() > 1 {
                // Plusieurs définisseurs : on s'abstient. Une relation
                // manquante vaut mieux qu'une relation fausse — c'est
                // exactement la sur-connexion que RAGForge a payée.
                report.ambiguous += 1;
                continue;
            }
            let Some(target) = definers.first() else {
                report.still_pending += 1;
                continue;
            };
            for (mentioner, kind) in mentioners_by_symbol.get(sym).unwrap_or(&empty).iter().cloned() {
                if &mentioner == target {
                    continue;
                }
                // L'arête est du genre inscrit au rendez-vous. Seul `CONSUMES`
                // a une réciproque déclarée ; `IMPLEMENTS` et `INHERITS_FROM`
                // n'en ont pas, et on n'en invente pas.
                let rel = if RELATIONS.iter().any(|(r, _, _)| *r == kind) { kind.as_str() } else { "CONSUMES" };
                self.link(rel, RefOrUuid::Uuid(mentioner.clone()), RefOrUuid::Uuid(target.clone()), BTreeMap::new())?;
                if rel == "CONSUMES" {
                    self.link("CONSUMED_BY", RefOrUuid::Uuid(target.clone()), RefOrUuid::Uuid(mentioner), BTreeMap::new())?;
                }
                report.linked_across_batches += 1;
            }
        }
        let linked = self.drain();
        report.failed += linked.failed;
        Ok(())
    }

    /// Pour chaque uuid donné, ceux qui le pointent par `rel` — en une seule
    /// requête (`UNWIND`), le même idiome que l'expansion de recherche.
    fn linked_from_many(
        &self,
        rel: &str,
        to_uuids: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<String>>, CatalogError> {
        Ok(self
            .linked_from_many_with_kind(rel, to_uuids, false)?
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|(uuid, _)| uuid).collect()))
            .collect())
    }

    /// Comme [`Self::linked_from_many`], mais rend aussi la propriété `kind`
    /// de l'arête quand `with_kind` — le genre inscrit au rendez-vous.
    fn linked_from_many_with_kind(
        &self,
        rel: &str,
        to_uuids: &[String],
        with_kind: bool,
    ) -> Result<std::collections::HashMap<String, Vec<(String, String)>>, CatalogError> {
        let mut out: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();
        if to_uuids.is_empty() {
            return Ok(out);
        }
        let kind_expr = if with_kind { ", r.kind" } else { "" };
        let cypher = format!(
            "UNWIND $uuids AS uid MATCH (n {{_uuid: uid}})<-[r:{rel}]-(m) RETURN uid, m._uuid{kind_expr}"
        );
        let param = CypherValue::List(to_uuids.iter().map(|u| CypherValue::String(u.clone())).collect());
        let result = self
            .conn()
            .execute_with_params(&cypher, &[crate::connection::QueryParam::new("uuids", param)])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        for row in &result.rows {
            if let (Some(CypherValue::String(to)), Some(CypherValue::String(from))) = (row.first(), row.get(1)) {
                let kind = row.get(2).and_then(|v| v.as_str()).unwrap_or("CONSUMES").to_string();
                out.entry(to.clone()).or_default().push((from.clone(), kind));
            }
        }
        Ok(out)
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
        // L'identité d'un fichier est son chemin **absolu dans sa source**,
        // pas son chemin relatif à la racine d'analyse (doc 04 v3).
        assert_eq!(a.files[0].path, "/virtual/a.rs");
        assert_eq!(a.files[0].source, LOCAL_SOURCE);
        assert_eq!(a.files[0].language, "rust");
        assert!(!a.files[0].content_hash.is_empty());
        assert_eq!(a.skipped.len(), 1, "{:?}", a.skipped);
        let names: Vec<&str> = a.scopes.iter().map(|s| s.name.as_str()).collect();
        let norm = a.scopes.iter().find(|s| s.name == "norm").unwrap_or_else(|| panic!("norm not in {names:?}"));
        assert_eq!(norm.scope_type, "method");
        assert!(norm.end_byte > norm.start_byte);
        assert!(RUST_SRC[norm.start_byte..norm.end_byte].contains("fn norm"));
        assert!(a.relations.iter().any(|r| r.rel == "DEFINED_IN" && r.from_key == norm.key && r.to_entity == FILE && r.to_key == "/virtual/a.rs"),
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
        // Histogramme : par type, puis les cibles les plus reliées.
        let mut by_type: std::collections::BTreeMap<&str, usize> = Default::default();
        let mut by_target: std::collections::HashMap<String, usize> = Default::default();
        let name_of = |e: &str, k: &str| -> String {
            if e == super::SCOPE { a.scopes.iter().find(|s| s.key == k).map(|s| format!("{}:{}", s.scope_type, s.name)).unwrap_or_else(|| k.to_string()) } else { format!("{e}:{k}") }
        };
        for r in &a.relations {
            *by_type.entry(r.rel.as_str()).or_default() += 1;
            if r.rel == "CONSUMES" { *by_target.entry(name_of(&r.to_entity, &r.to_key)).or_default() += 1; }
        }
        eprintln!("by type: {by_type:?}");
        let mut top: Vec<_> = by_target.into_iter().collect();
        top.sort_by(|x, y| y.1.cmp(&x.1));
        eprintln!("top CONSUMES targets: {:?}", &top[..top.len().min(25)]);
        let distinct_targets = a.relations.iter().filter(|r| r.rel == "CONSUMES").map(|r| &r.to_key).collect::<std::collections::HashSet<_>>().len();
        eprintln!("CONSUMES: {} edges → {} distinct targets", by_type.get("CONSUMES").copied().unwrap_or(0), distinct_targets);
        let mut by_source_type: std::collections::BTreeMap<String, usize> = Default::default();
        let mut dup_through_nesting = 0usize;
        let scope_of = |k: &str| a.scopes.iter().find(|s| s.key == k);
        for r in a.relations.iter().filter(|r| r.rel == "CONSUMES") {
            if let Some(src) = scope_of(&r.from_key) {
                *by_source_type.entry(src.scope_type.clone()).or_default() += 1;
                // même cible depuis un scope enfant du même fichier ?
                if a.relations.iter().any(|o| o.rel == "CONSUMES" && o.to_key == r.to_key && o.from_key != r.from_key
                    && scope_of(&o.from_key).map_or(false, |c| c.file_path == src.file_path && c.start_line >= src.start_line && c.end_line <= src.end_line && (c.start_line, c.end_line) != (src.start_line, src.end_line))) {
                    dup_through_nesting += 1;
                }
            }
        }
        let mut by_scope_type: std::collections::BTreeMap<String, usize> = Default::default();
        for sc in &a.scopes { *by_scope_type.entry(sc.scope_type.clone()).or_default() += 1; }
        eprintln!("scopes by type: {by_scope_type:?}");
        let lambdas: Vec<String> = a.scopes.iter().filter(|s| s.scope_type == "lambda").take(6).map(|s| format!("{}@{}:{} parent={}", s.name, s.file_path, s.start_line, s.parent_name)).collect();
        eprintln!("lambda samples: {lambdas:?}");
        eprintln!("CONSUMES by source type: {by_source_type:?}");
        eprintln!("CONSUMES also emitted by an enclosed child scope (nesting duplicates): {dup_through_nesting}");
    }
}
