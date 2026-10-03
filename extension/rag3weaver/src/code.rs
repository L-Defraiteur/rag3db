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
use codeparsers::scope_extraction::types::{ScopeInfoType, UsageKind, UsageSite};

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

/// **Extensions qu'on n'ingère jamais** — artefacts de build, données, binaires.
///
/// La couche la moins chère des trois : elle se décide sur le **nom**, donc
/// avant d'ouvrir le fichier. Mesurée sur ce dépôt le 30 août 2026 : 494
/// fichiers, 365 Mo, dont 260 Mo de `.csv` de test.
///
/// Ce qui n'est pas ici n'est pas innocent pour autant : `.json` est parfois
/// une configuration écrite à la main et parfois un corpus de 21 Mo. C'est
/// [`TEXT_MAX_BYTES`] qui tranche ce que l'extension ne peut pas trahir.
pub const SKIPPED_EXTENSIONS: [&str; 28] = [
    // verrous et sorties de compilation
    "lock", "d", "o", "a", "so", "dylib", "rlib", "rmeta", "pyc", "class", "map",
    // journaux
    "log",
    // données
    "csv", "parquet", "npy", "db", "sqlite", "arrow",
    // binaires et médias
    "wasm", "node", "png", "jpg", "jpeg", "gif", "pdf", "gz", "zip", "bin",
];

/// Motifs de nom qu'on n'ingère jamais : minifiés et verrous de dépendances,
/// que l'extension seule ne distingue pas de leur source.
pub const SKIPPED_NAME_PATTERNS: [&str; 4] = [".min.js", ".min.css", "-lock.json", ".generated."];

/// **Au-delà, un fichier texte n'est plus de la prose.**
///
/// Un seuil est un proxy, et il sera faux un jour — une spécification écrite à
/// la main de 300 Kio serait écartée. Il est retenu parce que la mesure du
/// 30 août 2026 le rend franc : sur ce dépôt, les 26 fichiers texte au-dessus
/// sont des corpus de banc, des dictionnaires, des `consoledump` et du Cypher
/// généré, **sans exception**, tandis que le plus gros document écrit par
/// quelqu'un fait 72 Kio.
///
/// Ce qui rend le proxy acceptable n'est pas sa justesse : c'est que ce qu'il
/// écarte se **compte** dans `CodeAnalysis::skipped`, au lieu de disparaître.
pub const TEXT_MAX_BYTES: usize = 128 * 1024;

/// Ce qu'on fait d'un fichier — le fait, pas encore la conséquence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Une grammaire existe : scopes, imports, relations.
    Code,
    /// Aucune grammaire, mais quelqu'un l'a écrit : **un** scope
    /// `texte_brut` couvrant tout le fichier.
    Texte,
    /// Hors index, avec la raison — jamais en silence.
    Ecarte(&'static str),
}

/// **La politique, en un seul endroit.** codeparsers rend des faits ; c'est
/// ici qu'on décide, et la décision se dit.
///
/// L'ordre compte : la liste noire d'abord parce qu'elle est gratuite, le
/// seuil ensuite parce qu'il demande la taille.
pub fn verdict(path: &str, size: usize) -> Verdict {
    let nom = Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if SKIPPED_NAME_PATTERNS.iter().any(|m| nom.contains(m)) {
        return Verdict::Ecarte("généré ou minifié");
    }
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if SKIPPED_EXTENSIONS.contains(&ext.as_str()) {
        return Verdict::Ecarte("artefact de build ou données");
    }
    if is_code_parser_supported(path) {
        return Verdict::Code;
    }
    // Un fichier sans extension est presque toujours un outil ou un artefact
    // (`Makefile` mis à part), et sans extension on ne peut rien en dire.
    if ext.is_empty() {
        return Verdict::Ecarte("sans extension");
    }
    if size > TEXT_MAX_BYTES {
        return Verdict::Ecarte("texte trop gros pour être de la prose");
    }
    Verdict::Texte
}

// ─── Schéma ──────────────────────────────────────────────────────────────────

fn field(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, ..Default::default() }
}
/// Un champ indexé **et** son vocabulaire : sans lui, le filtre existe sans
/// que personne puisse le nommer.
fn field_values(t: FieldType, values: &[&str]) -> SimpleFieldDef {
    SimpleFieldDef {
        field_type: t,
        values: Some(values.iter().map(|v| v.to_string()).collect()),
        ..Default::default()
    }
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
    // **Par lignes, façon ragforge** (6 septembre 2026) : 30 lignes ou 1 500
    // caractères, jamais au milieu d'une ligne, 5 lignes de recouvrement
    // seulement au-delà. Avant : sémantique 1 000 / 100, une découpe de prose
    // qui ne connaît ni bloc ni accolade.
    ChunkingConfig {
        max_size: 1500,
        overlap: 0,
        strategy: ChunkStrategy::Lines,
        max_lines: 30,
        overlap_lines: 5,
        ..Default::default()
    }
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
        // Un fichier supprimé de sa source part à la fin de la synchronisation
        // de celle-ci (`code_sync::sync_source`) ; une édition n'en supprime
        // pas, le grain fin n'a donc pas lieu d'être ici.
        snapshot: Some(crate::config::SnapshotConfig {
            scope: vec!["source".into()],
            fine_scope: vec![],
            max_missing_ratio: 0.5,
            on_missing: crate::config::OnMissing::Delete,
            keep_for: None,
        }),
        ..Default::default()
    }
}

pub fn scope_config(chunking: ChunkingConfig) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), title(FieldType::String));
    // **La signature n'est plus un champ de contenu** (6 septembre 2026) :
    // c'est la première ligne du texte propre, déjà dans `content`. L'embarquer
    // à part faisait un vecteur de plus par scope — 1 633 sur 4 240 — pour un
    // texte que le premier chunk porte déjà. Elle reste rendue et filtrable.
    fields.insert("signature".into(), field(FieldType::Text));
    fields.insert("content".into(), content(FieldType::Text));
    // La docstring, elle, précède la déclaration : hors de l'empan, donc hors
    // de `content`. Elle reste embarquée à part, comme chez ragforge.
    fields.insert("docstring".into(), content(FieldType::Text));
    // **Le vocabulaire, déclaré.** Les douze premiers viennent de
    // `ScopeInfoType` ; `texte_brut` est ce qu'on n'a pas essayé de parser.
    // C'est la réponse à « code ou texte ? » : `scope_type != 'texte_brut'`.
    fields.insert(
        "scope_type".into(),
        field_values(
            FieldType::String,
            &[
                "class", "interface", "function", "method", "enum", "type_alias", "namespace",
                "module", "variable", "lambda", "constant", "block", "texte_brut",
            ],
        ),
    );
    fields.insert("file_path".into(), field(FieldType::String));
    // La source du fichier qui le contient — le même nom absolu peut exister
    // dans deux sources, ce sont deux fichiers.
    fields.insert("source".into(), field(FieldType::String));
    // Les coordonnées portables, dénormalisées depuis `File` : `repo` est ce
    // qu'un domaine d'agent filtrera, `repo_path` ce que le rendu affiche.
    // Une jointure par scope à l'affichage coûterait plus que ces deux
    // colonnes.
    fields.insert("repo".into(), field(FieldType::String));
    fields.insert("repo_path".into(), field(FieldType::String));
    fields.insert("parent_name".into(), field(FieldType::String));
    fields.insert("language".into(), field(FieldType::String));
    fields.insert("start_line".into(), field(FieldType::Integer));
    fields.insert("end_line".into(), field(FieldType::Integer));
    // Les enfants repliés dans `content` : de quoi rendre un extrait avec
    // les lignes du fichier, pas celles du texte propre.
    fields.insert("folds".into(), field(FieldType::String));
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
        // **La documentation fait partie de ce qu'un résultat doit dire.** Elle
        // manquait : la fiche rendue n'avait donc jamais sa ligne `📝`, celle
        // qui, dans la maquette d'origine, dit en une phrase *ce que fait* le
        // scope — la seule ligne qu'on lit avant de décider d'ouvrir le
        // fichier. Elle est bornée au rendu (`max_chars`), pas ici.
        return_fields: Some(vec![
            "file_path".into(), "start_line".into(), "end_line".into(),
            "scope_type".into(), "parent_name".into(),
            "docstring".into(), "signature".into(), "folds".into(),
        ]),
        source_lines: Some(crate::config::SourceLines {
            field: "content".into(),
            start_line: Some("start_line".into()),
            folds: Some("folds".into()),
        }),
        // **La pondération par genre, tranchée par Lucie le 3 octobre 2026**
        // (doc des mesures du même jour) : les fonctions et méthodes à 1,0,
        // tout le reste à 0,85 — l'« ombre pondérée » du filtre G du banc,
        // qu'elle rejoint à sa variance près (0,407 contre 0,409) en gardant
        // tout : un poids, jamais un filtre. 0,85 est la plus douce des
        // valeurs qui y arrivent. **Ce que le banc ne voit pas** : aucune de
        // ses questions n'a un fichier entier, un module ou un espace de noms
        // pour bonne réponse — il mesure ce que la dévaluation rend aux
        // fonctions, pas ce qu'elle coûte à « quel fichier gère X » ; c'est
        // pour ces questions-là que la valeur reste douce. Un graphe
        // (`weights` du nœud) ou un appelant la surchargent par l'échelle du
        // pas C.
        field_weights: vec![crate::search::FieldWeight {
            field: "scope_type".into(),
            weights: [("function".to_string(), 1.0), ("method".to_string(), 1.0)]
                .into_iter()
                .collect(),
            default: 0.85,
        }],
        // La vue par parent : les méthodes d'un même impl rendues ensemble,
        // sous sa signature, que l'impl soit ou non un résultat.
        group_by: Some(crate::config::GroupBy { relation: "HAS_PARENT".into(), frame_field: "signature".into() }),
        // **La synchronisation déclarée** : la source entière est le grain
        // large, le fichier le grain fin. Une édition finit son fichier
        // (`code_sync::reingest_file`) ; une synchronisation de la source
        // retire ce qui a disparu, fichiers supprimés compris.
        snapshot: Some(crate::config::SnapshotConfig {
            scope: vec!["source".into()],
            fine_scope: vec!["file_path".into()],
            max_missing_ratio: 0.5,
            on_missing: crate::config::OnMissing::Delete,
            keep_for: None,
        }),
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


/// Les relations qui sont des usages : elles portent, sur l'arête, comment la
/// source se sert de la cible (`usage`, `usages`) et à quelle ligne (`line`).
/// Les relations de structure (`DEFINED_IN`, `PARENT_OF`, `HAS_PARENT`) et
/// le rendez-vous `DEFINES` n'en portent pas.
const USAGE_RELATIONS: [&str; 6] = ["CONSUMES", "CONSUMED_BY", "INHERITS_FROM", "IMPLEMENTS", "DECORATES", "USES_LIBRARY"];

fn field_def(field_type: FieldType) -> crate::config::FieldDef {
    crate::config::FieldDef { field_type, title_for: None, content_for: None, boost: None, default_value: None }
}

fn usage_property_defs() -> HashMap<String, crate::config::FieldDef> {
    HashMap::from([
        ("usage".to_string(), field_def(FieldType::String)),
        ("usages".to_string(), field_def(FieldType::String)),
        ("line".to_string(), field_def(FieldType::Int64)),
    ])
}

fn usage_name(u: &UsageKind) -> &'static str {
    match u {
        UsageKind::Call => "call",
        UsageKind::Type => "type",
        UsageKind::Import => "import",
        UsageKind::Inheritance => "inheritance",
        UsageKind::Other => "other",
    }
}

/// Les propriétés d'une arête d'usage, tirées de ses sites :
/// - `usage`, le genre le plus fort présent (héritage, puis appel, type,
///   import, autre) : « tous les appels de X » ne doit pas manquer un appel
///   parce que X sert aussi de type au même endroit ;
/// - `usages`, tous les genres présents, triés, séparés par des virgules
///   (`call,type`) ;
/// - `line`, la première ligne d'un site.
///
/// Les trois clés sont toujours là (`line` peut être nulle) : le chemin COPY
/// écrit une colonne par propriété déclarée, un groupe à qui il en manque
/// une n'y entrerait pas.
fn usage_properties(sites: &[UsageSite]) -> BTreeMap<String, CypherValue> {
    const ORDRE: [UsageKind; 5] = [UsageKind::Inheritance, UsageKind::Call, UsageKind::Type, UsageKind::Import, UsageKind::Other];
    let Some(retenu) = ORDRE.iter().find(|k| sites.iter().any(|s| &s.usage == *k)) else {
        return BTreeMap::new();
    };
    let mut tous: Vec<&str> = sites.iter().map(|s| usage_name(&s.usage)).collect();
    tous.sort_unstable();
    tous.dedup();
    let line = sites.iter().filter_map(|s| s.line).min().map_or(CypherValue::Null, |l| CypherValue::Int(l as i64));
    BTreeMap::from([
        ("usage".to_string(), s(usage_name(retenu))),
        ("usages".to_string(), s(&tous.join(","))),
        ("line".to_string(), line),
    ])
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
            // l'ordre d'ingestion changerait le graphe (doc 17 §10). Il porte
            // aussi l'usage, que l'arête matérialisée recopiera.
            let mut props = usage_property_defs();
            props.insert("kind".to_string(), field_def(FieldType::String));
            props.insert("qualifier_types".to_string(), field_def(FieldType::String));
            catalog.register_relation_with(rel, from, to, props)?;
            continue;
        }
        if USAGE_RELATIONS.contains(&rel) {
            catalog.register_relation_with(rel, from, to, usage_property_defs())?;
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
    /// Le chemin du fichier **dans ce dépôt** — ce que le rendu affiche.
    #[serde(default)]
    pub repo_path: String,
    pub name: String,
    pub scope_type: String,
    pub signature: String,
    pub content: String,
    /// Les lignes repliées de `content` (voir [`crate::config::SourceLines`]),
    /// ou `?` quand ses lignes ne sont pas celles du fichier.
    #[serde(default)]
    pub folds: String,
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
    /// Les usages qui font cette arête (genre, ligne) ; vide pour une
    /// relation de structure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<UsageSite>,
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
    /// Les usages derrière chaque rendez-vous (clé du scope, nom) : toutes
    /// les références du scope à ce nom, pas seulement la première.
    #[serde(default)]
    pub pending_sites: Vec<(String, String, Vec<UsageSite>)>,
    /// Le type des variables par lesquelles le scope atteint le nom
    /// (`n.run()` avec `n: Node` donne `Node`), quand **toutes** ses
    /// références à ce nom en ont un ; absent sinon. Il départage les
    /// définisseurs homonymes à la matérialisation.
    #[serde(default)]
    pub pending_qualifier_types: Vec<(String, String, Vec<String>)>,
    pub parse_ms: u128,
    pub relation_ms: u128,
}

/// **Un seul vocabulaire dans `language`.**
///
/// Les fichiers de code y portent le nom du langage — `rust`, `cpp`,
/// `typescript` — parce que [`language_name`] le dérive de
/// `SupportedLanguage`. Un fichier texte qui y porterait `md` mettrait deux
/// vocabulaires dans la même colonne indexée : un agent qui a appris
/// `language = 'rust'` essaierait `language = 'markdown'` et n'aurait rien.
///
/// Un langage qu'on ne sait pas parser garde quand même son nom — un `.java`
/// est du Java, `scope_type = 'texte_brut'` dit qu'on n'a pas su le lire. Les
/// deux champs ensemble répondent à « quel langage servons-nous mal ? ».
fn texte_language_name(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "mdx" | "markdown" => "markdown".to_string(),
        "txt" | "text" => "texte".to_string(),
        "rst" => "restructuredtext".to_string(),
        "toml" => "toml".to_string(),
        "yml" | "yaml" => "yaml".to_string(),
        "json" | "jsonc" => "json".to_string(),
        "xml" => "xml".to_string(),
        "ini" | "cfg" | "conf" => "config".to_string(),
        "sh" | "bash" | "zsh" | "fish" => "shell".to_string(),
        "sql" => "sql".to_string(),
        "cypher" | "cyp" => "cypher".to_string(),
        "java" => "java".to_string(),
        "kt" | "kts" => "kotlin".to_string(),
        "rb" => "ruby".to_string(),
        "php" => "php".to_string(),
        "swift" => "swift".to_string(),
        "lua" => "lua".to_string(),
        "r" => "r".to_string(),
        "pl" | "pm" => "perl".to_string(),
        "scala" => "scala".to_string(),
        "hs" => "haskell".to_string(),
        "ex" | "exs" => "elixir".to_string(),
        "dart" => "dart".to_string(),
        "zig" => "zig".to_string(),
        "vue" => "vue".to_string(),
        "svelte" => "svelte".to_string(),
        "css" => "css".to_string(),
        "scss" | "sass" => "scss".to_string(),
        "html" | "htm" => "html".to_string(),
        "test" => "test".to_string(),
        // Une extension inconnue se rend telle quelle plutôt que « unknown » :
        // c'est la seule chose qu'on sait du fichier, l'effacer ne sert rien.
        // (Le cas vide n'arrive pas : `verdict` écarte les fichiers sans
        // extension avant d'en venir ici.)
        autre => autre.to_string(),
    }
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
    // Les fichiers sans grammaire, gardés de côté : ils ne passent pas par
    // `parse_project` — il n'y a rien à résoudre — mais ils entrent dans
    // l'index. `(chemin relatif, chemin absolu, contenu)`.
    let mut textes: Vec<(String, String, String)> = Vec::new();
    for (rel, content) in sources {
        let genre = verdict(&rel, content.len());
        if let Verdict::Ecarte(raison) = genre {
            skipped.push((rel, raison.to_string()));
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
        match genre {
            Verdict::Code => {
                content_map.insert(abs.clone(), content);
                files.push(abs);
            }
            Verdict::Texte => textes.push((rel, abs, content)),
            Verdict::Ecarte(_) => unreachable!("écarté plus haut"),
        }
    }
    let identity_of = |rel: &str| -> (String, BTreeMap<String, String>) {
        named.get(rel).cloned().unwrap_or_else(|| (rel.to_string(), BTreeMap::new()))
    };

    // Le texte brut de chaque fichier, gardé pour le texte propre des scopes
    // (`own_texts`) : l'analyseur ne le rend pas.
    let raw: HashMap<String, String> = content_map.clone();
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
    // ── Les fichiers sans grammaire ─────────────────────────────────────
    //
    // Un `File`, **un** `Scope` `texte_brut` qui couvre tout, et l'arête qui
    // les relie. Rien d'autre : pas d'import à résoudre, pas de référence à
    // rapprocher, pas de symbole à définir. Ils ne passent donc pas par le
    // résolveur, et leur clé est forgée ici — au même format que
    // [`stable_scope_keys`] pour un scope sans parent et sans homonyme.
    for (rel, abs, content) in &textes {
        let fa = codeparsers::parallel::parser_worker::analyser_texte_brut(rel, content);
        let (name, coordinates) = identity_of(rel);
        let repo = coordinates.get("repo").cloned().unwrap_or_default();
        let repo_path = coordinates.get("repo_path").cloned().unwrap_or_default();
        let language = texte_language_name(rel);
        analysis.files.push(FileRecord {
            path: name.clone(),
            source: source.clone(),
            coordinates,
            absolute_path: abs.clone(),
            language: language.clone(),
            lines_of_code: fa.total_lines,
            size_bytes: fa.octets,
            content_hash: fa.content_hash.clone().unwrap_or_default(),
            cursor: String::new(),
        });
        let sc = &fa.scopes[0];
        let key = format!("{source}#{name}#{}:texte_brut", sc.name);
        analysis.scopes.push(ScopeRecord {
            key: key.clone(),
            source: source.clone(),
            repo,
            repo_path,
            name: sc.name.clone(),
            scope_type: "texte_brut".to_string(),
            signature: String::new(),
            content: sc.content.clone(),
            folds: String::new(),
            docstring: String::new(),
            file_path: name.clone(),
            parent_name: String::new(),
            language,
            start_line: sc.scope_start_line,
            end_line: sc.scope_end_line,
            start_byte: sc.scope_start_byte,
            end_byte: sc.scope_end_byte,
        });
        analysis.relations.push(CodeRelation {
            rel: "DEFINED_IN".to_string(),
            from_entity: SCOPE.to_string(),
            from_key: key,
            to_entity: FILE.to_string(),
            to_key: name,
            sites: Vec::new(),
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
        let repo_path = coords.get("repo_path").cloned().unwrap_or_default();
        let language = language_name(abs);
        // **Le texte propre de chaque scope**, calculé sur le fichier entier :
        // sa déclaration et son corps, chaque enfant direct remplacé par sa
        // ligne de signature. Décidé le 6 septembre 2026 (doc 19h57) : avant,
        // le `content` d'un `impl` était le corps entier, méthodes comprises,
        // et une méthode était embarquée deux ou trois fois.
        let owned = raw.get(abs).map(|texte| own_texts(texte, &fa.scopes)).unwrap_or_default();
        for (i, s) in fa.scopes.iter().enumerate() {
            let type_str = scope_type_name(&s.r#type).to_string();
            let Some(key) = by_position.get(&(rel.clone(), s.name.clone(), type_str.clone(), s.scope_start_line)) else {
                continue;
            };
            // Sans texte propre, le repli de l'analyseur peut n'être que le
            // corps : sa ligne 0 n'est pas `start_line`, d'où `?` — pas de
            // numéros plutôt que des numéros faux.
            let (content, folds) = match owned.get(i) {
                Some(Some((propre, replis))) => (propre.clone(), replis.clone()),
                _ if s.content_dedented.is_empty() => (s.content.clone(), "?".to_string()),
                _ => (s.content_dedented.clone(), "?".to_string()),
            };
            analysis.scopes.push(ScopeRecord {
                key: key.clone(),
                source: source.clone(),
                repo: repo.clone(),
                repo_path: repo_path.clone(),
                name: s.name.clone(),
                scope_type: type_str,
                signature: s.signature.clone(),
                content,
                folds,
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
            sites: r.metadata.as_ref().map(|m| m.sites.clone()).unwrap_or_default(),
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
            let mut sites: BTreeMap<String, Vec<UsageSite>> = BTreeMap::new();
            // Par nom : les types lus, ou `None` dès qu'une référence n'en a pas.
            let mut types_lus: BTreeMap<String, Option<Vec<String>>> = BTreeMap::new();
            for r in &sc.identifier_references {
                use codeparsers::scope_extraction::types::IdentifierReferenceKind as K;
                if matches!(r.kind, Some(K::Builtin) | Some(K::LocalScope)) {
                    continue;
                }
                let id = r.identifier.as_str();
                if id.is_empty() || id == sc.name || libraries.contains(id) {
                    continue;
                }
                let site = UsageSite { usage: r.usage.clone().unwrap_or(UsageKind::Other), line: Some(r.line) };
                let liste = sites.entry(id.to_string()).or_default();
                if !liste.contains(&site) {
                    liste.push(site);
                }
                let types = types_lus.entry(id.to_string()).or_insert_with(|| Some(Vec::new()));
                match (types.as_mut(), r.qualifier_type.as_ref()) {
                    (Some(v), Some(t)) => {
                        if !v.contains(t) {
                            v.push(t.clone());
                        }
                    }
                    _ => *types = None,
                }
                if !seen.insert(id.to_string()) {
                    continue;
                }
                // Le genre lu sur l'AST décide si la référence peut être un
                // héritage ; la devinette sur le texte ne choisit plus que
                // l'espèce (codeparsers f0faa82).
                let kind = relation_name(&codeparsers::relationship_resolution::relationship_resolver::detect_relationship_type_for_reference(
                    sc, id, "", r,
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
                    let site = UsageSite { usage: UsageKind::Inheritance, line: Some(sc.signature_start_line) };
                    let liste = sites.entry(t.clone()).or_default();
                    if !liste.contains(&site) {
                        liste.push(site);
                    }
                }
            }
            for (name, liste) in sites {
                analysis.pending_sites.push((key.clone(), name, liste));
            }
            for (name, types) in types_lus {
                if let Some(mut v) = types.filter(|v| !v.is_empty()) {
                    v.sort();
                    analysis.pending_qualifier_types.push((key.clone(), name, v));
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
    // Une arête par (relation, source, cible) ; les sites des doublons — une
    // fermeture repliée sur son scope, par exemple — rejoignent la première.
    let mut index: HashMap<(String, String, String, String, String), usize> = HashMap::new();
    let mut kept: Vec<CodeRelation> = Vec::with_capacity(a.relations.len());
    for r in a.relations.drain(..) {
        let cle = (r.rel.clone(), r.from_entity.clone(), r.from_key.clone(), r.to_entity.clone(), r.to_key.clone());
        if let Some(&i) = index.get(&cle) {
            for site in r.sites {
                if !kept[i].sites.contains(&site) {
                    kept[i].sites.push(site);
                }
            }
            continue;
        }
        index.insert(cle, kept.len());
        kept.push(r);
    }
    a.relations = kept;
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
        ScopeInfoType::TexteBrut => "texte_brut",
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
        // Le tri fin est celui d'`analyze_with`, qui a la taille et qui **dit**
        // ce qu'il écarte. Ici on ne retire que ce qui se juge au nom seul,
        // pour ne pas lire un `.parquet` de 50 Mo afin de le jeter ensuite.
        if matches!(verdict(&path, 0), Verdict::Ecarte(_)) {
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

/// [`read_sources_report`], sans le compte rendu.
pub fn read_sources(root: &str) -> std::io::Result<Vec<(String, String)>> {
    Ok(read_sources_report(root)?.0)
}

/// Lit les sources d'un répertoire — code **et** texte — en rendant compte de
/// ce qui a été écarté : `(sources, (chemin, raison))`.
///
/// [`SKIPPED_DIRS`] est ignoré, et le [`verdict`] est appliqué **sur la
/// taille du fichier lue au passage**, avant d'ouvrir quoi que ce soit : un
/// `.parquet` de 50 Mo n'a pas à être lu pour être jeté.
///
/// Le compte rendu existe parce que sans lui, remplacer « on jette les
/// fichiers sans parseur » par « on jette les gros fichiers » serait le même
/// mensonge avec un autre critère.
pub fn read_sources_report(root: &str) -> std::io::Result<(Vec<(String, String)>, Vec<(String, String)>)> {
    fn walk(
        dir: &Path,
        root: &Path,
        out: &mut Vec<(String, String)>,
        skipped: &mut Vec<(String, String)>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let genre = entry.file_type()?;
            if genre.is_symlink() {
                // Un lien peut désigner la racine du dépôt ; le suivre boucle.
                continue;
            }
            if genre.is_dir() {
                if SKIPPED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                    continue;
                }
                walk(&path, root, out, skipped)?;
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
            let taille = entry.metadata().map(|m| m.len() as usize).unwrap_or(0);
            match verdict(&rel, taille) {
                Verdict::Ecarte(raison) => skipped.push((rel, raison.to_string())),
                _ => match std::fs::read_to_string(&path) {
                    Ok(content) => out.push((rel, content)),
                    // Un fichier non-UTF-8 est refusé franchement : des
                    // offsets d'octets sur un encodage inconnu seraient faux.
                    Err(_) => skipped.push((rel, "illisible ou non-UTF-8".to_string())),
                },
            }
        }
        Ok(())
    }
    let root_path = Path::new(root);
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    walk(root_path, root_path, &mut out, &mut skipped)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    skipped.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((out, skipped))
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
            ("folds".into(), s(&self.folds)),
            ("docstring".into(), s(&self.docstring)),
            ("file_path".into(), s(&self.file_path)),
            ("source".into(), s(&self.source)),
            ("repo".into(), s(&self.repo)),
            ("repo_path".into(), s(&self.repo_path)),
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
/// **La clé d'identité d'une entité, entière.**
///
/// Entière, et c'est tout le sujet : `File` s'identifie par
/// `["source", "path"]` depuis le doc 04 v3 — « le même nom absolu dans deux
/// sources, ce sont deux fichiers ». Cette fonction ne posait que `path`, donc
/// l'uuid dérivé de la clé partielle ne tombait jamais sur celui du nœud
/// inséré, et **toutes les arêtes `DEFINED_IN` étaient silencieusement
/// perdues** — `link` posait une arête vers un uuid qui n'existait pas.
///
/// Trouvé le 28 août 2026 en cherchant pourquoi l'arbre de dépendances était
/// vide sur notre propre code : 36 scopes, 37 `CONSUMES`, 15 `HAS_PARENT`, et
/// zéro `DEFINED_IN`. `Scope` et `Library` ont une clé à un champ, donc eux
/// n'ont jamais rien perdu — l'ajout de `source` à l'identité d'un fichier
/// avait cassé la seule relation qui vise un fichier, sans faire échouer un
/// seul test.
fn key_data(entity: &str, key: &str, source: &str) -> BTreeMap<String, CypherValue> {
    match entity {
        FILE => BTreeMap::from([
            ("source".to_string(), s(source)),
            ("path".to_string(), s(key)),
        ]),
        SCOPE => BTreeMap::from([("key".to_string(), s(key))]),
        _ => BTreeMap::from([("name".to_string(), s(key))]),
    }
}

impl Catalog {
    /// Persiste une [`CodeAnalysis`] : entités par `ingest_entities`, relations
    /// par `link` (uuid dérivés des clés, comme le catalogue les dérivera),
    /// puis `drain`. Le schéma doit être déclaré ([`register_code_schema`]).
    pub fn ingest_code(&mut self, analysis: &CodeAnalysis) -> Result<CodeIngestReport, CatalogError> {
        self.ingest_code_jusqu_a(analysis, crate::disponibilite::Disponibilites::TOUT)
    }

    /// [`ingest_code`](Self::ingest_code), en disant **ce qui doit être prêt**
    /// quand il rend : `RECHERCHE_TEXTE` écrit les lignes et le plein texte,
    /// et laisse la dette de vecteurs en base, que
    /// [`embarquer_le_retard`](Self::embarquer_le_retard) solde.
    pub fn ingest_code_jusqu_a(
        &mut self,
        analysis: &CodeAnalysis,
        exige: crate::disponibilite::Disponibilites,
    ) -> Result<CodeIngestReport, CatalogError> {
        self.ingest_code_interne(analysis, exige, None)
    }

    /// **Les nœuds maintenant, les relations plus tard** : les lignes (File,
    /// Scope, Library, Symbol) et leur plein texte sont posés ; les relations
    /// de l'analyse et les rendez-vous `DEFINES` / `MENTIONS` restent **en
    /// file**, et les noms touchés s'ajoutent à `noms`.
    /// [`finir_les_relations_differees`](Self::finir_les_relations_differees)
    /// les pose ensuite en une fois — chaque relation dépasse alors largement
    /// le seuil du chemin par COPY — et résout tous les noms d'un coup.
    /// C'est le chemin de masse d'une première indexation : une suite de
    /// petits lots de liens par MERGE coûtait de plus en plus cher à mesure
    /// que la base grossissait.
    pub fn ingest_code_differe(
        &mut self,
        analysis: &CodeAnalysis,
        exige: crate::disponibilite::Disponibilites,
        noms: &mut std::collections::BTreeSet<String>,
    ) -> Result<CodeIngestReport, CatalogError> {
        self.ingest_code_interne(analysis, exige, Some(noms))
    }

    /// La fin du chemin de masse : le drain des relations et des rendez-vous
    /// en file, puis la résolution de tous les noms touchés.
    pub fn finir_les_relations_differees(
        &mut self,
        noms: &std::collections::BTreeSet<String>,
        exige: crate::disponibilite::Disponibilites,
    ) -> Result<CodeIngestReport, CatalogError> {
        let mut report = CodeIngestReport::default();
        let phase = std::time::Instant::now();
        let linked = self.drain_jusqu_a(exige);
        report.relations = linked.processed;
        report.failed += linked.failed;
        report.relations_ms = phase.elapsed().as_millis();
        let phase = std::time::Instant::now();
        let noms: Vec<String> = noms.iter().cloned().collect();
        let resolu = self.resoudre_les_symboles(&noms, exige)?;
        report.linked_across_batches = resolu.linked_across_batches;
        report.ambiguous = resolu.ambiguous;
        report.still_pending = resolu.still_pending;
        report.failed += resolu.failed;
        report.symbols = noms.len();
        report.symbols_ms = phase.elapsed().as_millis();
        Ok(report)
    }

    fn ingest_code_interne(
        &mut self,
        analysis: &CodeAnalysis,
        exige: crate::disponibilite::Disponibilites,
        mut differes: Option<&mut std::collections::BTreeSet<String>>,
    ) -> Result<CodeIngestReport, CatalogError> {
        let mut report = CodeIngestReport::default();
        let phase = std::time::Instant::now();

        let files = self.ingest_entities_jusqu_a(FILE, analysis.files.iter().map(FileRecord::data).collect(), exige)?;
        report.files = files.processed;
        report.failed += files.failed;
        let scopes = self.ingest_entities_jusqu_a(SCOPE, analysis.scopes.iter().map(ScopeRecord::data).collect(), exige)?;
        report.scopes = scopes.processed;
        report.failed += scopes.failed;
        let libs = self.ingest_entities_jusqu_a(LIBRARY, analysis.libraries.iter().map(LibraryRecord::data).collect(), exige)?;
        report.libraries = libs.processed;
        report.failed += libs.failed;

        report.entities_ms = phase.elapsed().as_millis();
        let phase = std::time::Instant::now();

        // La source des fichiers de ce lot : elle fait partie de leur identité,
        // et une relation ne la porte pas — elle nomme un fichier par son
        // chemin. On la retrouve donc ici, par chemin, avec un repli sur la
        // source commune du lot pour un chemin qu'on n'aurait pas ingéré.
        let source_of: std::collections::HashMap<&str, &str> = analysis
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.source.as_str()))
            .collect();
        let source_commune = analysis.files.first().map(|f| f.source.as_str()).unwrap_or("");

        let mut par_relation: std::collections::BTreeMap<&str, Vec<(String, String, BTreeMap<String, CypherValue>)>> = Default::default();
        for r in &analysis.relations {
            let from_src = source_of.get(r.from_key.as_str()).copied().unwrap_or(source_commune);
            let to_src = source_of.get(r.to_key.as_str()).copied().unwrap_or(source_commune);
            let from = self.entity_uuid(&r.from_entity, &key_data(&r.from_entity, &r.from_key, from_src))?;
            let to = self.entity_uuid(&r.to_entity, &key_data(&r.to_entity, &r.to_key, to_src))?;
            let props = if USAGE_RELATIONS.contains(&r.rel.as_str()) { usage_properties(&r.sites) } else { BTreeMap::new() };
            par_relation.entry(r.rel.as_str()).or_default().push((from, to, props));
        }
        let en_file: usize = par_relation.values().map(Vec::len).sum();
        for (rel, liens) in par_relation {
            self.mettre_en_file_les_liens(rel, liens)?;
        }
        if differes.is_some() {
            // En file : posées à la fin, en masse.
            report.relations = en_file;
        } else {
            let linked = self.drain_jusqu_a(exige);
            report.relations = linked.processed;
            report.failed += linked.failed;
        }
        report.relations_ms = phase.elapsed().as_millis();

        let phase = std::time::Instant::now();
        self.resolve_across_batches(analysis, &mut report, exige, differes.as_deref_mut())?;
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
    /// `exige` : ce que l'ingestion qui l'appelle demande d'être prêt. Ses
    /// drains le respectent — un drain complet solderait au passage la dette
    /// d'embarquement de toute la base (le rattrapage opportuniste), ce
    /// qu'une ingestion en plein texte seul ne veut pas payer.
    fn resolve_across_batches(
        &mut self,
        analysis: &CodeAnalysis,
        report: &mut CodeIngestReport,
        exige: crate::disponibilite::Disponibilites,
        differes: Option<&mut std::collections::BTreeSet<String>>,
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
        let profil = std::env::var("RAG3WEAVER_INGEST_PROFILE").is_ok();
        let mut t = std::time::Instant::now();
        let etape = |nom: &str, t: &mut std::time::Instant| {
            if profil {
                eprintln!("[ingest-profile] {:>6} ms  symboles/{nom}", t.elapsed().as_millis());
            }
            *t = std::time::Instant::now();
        };
        let ingested = self.ingest_entities_jusqu_a(SYMBOL, records, exige)?;
        etape("ingestion", &mut t);
        report.failed += ingested.failed;

        let symbol_uuid = |cat: &Self, name: &str| -> Result<String, CatalogError> {
            let mut d = Map::new();
            d.insert("name".into(), s(name));
            cat.entity_uuid(SYMBOL, &d)
        };

        // Ce que le lot offre, et ce qu'il attend — en file par relation.
        let mut definitions: Vec<(String, String, BTreeMap<String, CypherValue>)> = Vec::with_capacity(analysis.scopes.len());
        let mut mentions: Vec<(String, String, BTreeMap<String, CypherValue>)> = Vec::with_capacity(analysis.pending.len());
        for sc in &analysis.scopes {
            let from = self.entity_uuid(SCOPE, &key_data(SCOPE, &sc.key, ""))?;
            let to = symbol_uuid(self, &sc.name)?;
            definitions.push((from, to, BTreeMap::new()));
        }
        // Ce que le lot apporte : ses scopes (définisseurs possibles) et ses
        // mentionneurs — pour ne reposer que les arêtes neuves.
        let scopes_du_lot: std::collections::HashSet<String> = definitions.iter().map(|(f, _, _)| f.clone()).collect();
        self.mettre_en_file_les_liens("DEFINES", definitions)?;
        let sites_of: HashMap<(&str, &str), &Vec<UsageSite>> =
            analysis.pending_sites.iter().map(|(k, n, v)| ((k.as_str(), n.as_str()), v)).collect();
        let types_of: HashMap<(&str, &str), &Vec<String>> =
            analysis.pending_qualifier_types.iter().map(|(k, n, v)| ((k.as_str(), n.as_str()), v)).collect();
        for (scope_key, name, kind) in &analysis.pending {
            let from = self.entity_uuid(SCOPE, &key_data(SCOPE, scope_key, ""))?;
            let to = symbol_uuid(self, name)?;
            // Le genre voyage avec le rendez-vous : c'est lui qui décide de
            // l'arête à poser quand la cible arrivera ; l'usage aussi, que
            // l'arête recopiera.
            let mut props = sites_of.get(&(scope_key.as_str(), name.as_str())).map_or_else(BTreeMap::new, |v| usage_properties(v));
            if props.is_empty() {
                props = usage_properties(&[UsageSite { usage: UsageKind::Other, line: None }]);
            }
            props.insert("kind".to_string(), s(kind));
            // Vide, pas nul, quand le type ne se lit pas partout.
            let types = types_of.get(&(scope_key.as_str(), name.as_str())).map(|v| v.join(",")).unwrap_or_default();
            props.insert("qualifier_types".to_string(), s(&types));
            mentions.push((from, to, props));
        }
        let mentionneurs_du_lot: std::collections::HashSet<String> = mentions.iter().map(|(f, _, _)| f.clone()).collect();
        self.mettre_en_file_les_liens("MENTIONS", mentions)?;
        etape("mise en file DEFINES/MENTIONS", &mut t);
        if let Some(noms) = differes {
            // Le chemin de masse : les rendez-vous restent en file, la
            // résolution attend que toute la source soit là.
            noms.extend(names.iter().map(|n| n.to_string()));
            return Ok(());
        }
        let drained = self.drain_jusqu_a(exige);
        report.failed += drained.failed;
        etape("drain des rendez-vous", &mut t);

        // Matérialisation, dans les deux sens. **Deux requêtes en tout** :
        // une par relation, en `UNWIND` sur tous les symboles du lot. Une
        // requête par symbole coûtait 2,5 fois le temps d'ingestion.
        let uuids: Vec<String> = names.iter().map(|n| symbol_uuid(self, n)).collect::<Result<_, _>>()?;
        self.materialiser_les_symboles(&uuids, Some((&scopes_du_lot, &mentionneurs_du_lot)), report)?;
        etape("relecture et mise en file des arêtes", &mut t);
        let linked = self.drain_jusqu_a(exige);
        etape("drain des arêtes résolues", &mut t);
        report.failed += linked.failed;
        Ok(())
    }

    /// **Poser les arêtes résolues** de ces symboles : un seul définisseur →
    /// chaque mentionneur gagne l'arête du genre inscrit au rendez-vous ;
    /// plusieurs → on s'abstient ; aucun → le nom reste en attente.
    ///
    /// `lot` : ce qu'un lot apporte (ses scopes, ses mentionneurs). Avec lui,
    /// on ne pose que les arêtes **neuves** — toutes celles d'un définisseur
    /// du lot (il vient d'apparaître ou de changer), et celles des seuls
    /// mentionneurs du lot vers un définisseur qui était déjà là. Sans lui,
    /// toutes. Reposer à chaque lot toutes les arêtes des noms courants
    /// (`new`, `len`, `get`…) rendait l'ingestion d'une source quadratique :
    /// de 2,0 à 5,7 s par paquet de 64 fichiers d'un tiers à l'autre, sur
    /// `src/` du moteur (3 octobre 2026).
    fn materialiser_les_symboles(
        &mut self,
        uuids: &[String],
        lot: Option<(&std::collections::HashSet<String>, &std::collections::HashSet<String>)>,
        report: &mut CodeIngestReport,
    ) -> Result<(), CatalogError> {
        let definers_by_symbol = self.linked_from_many("DEFINES", uuids)?;
        let mentioners_by_symbol = self.linked_from_many_with_kind("MENTIONS", uuids, true)?;
        // Le parent des définisseurs, pour les noms qu'une mention typée
        // atteint (`n.run()` avec `n: Node` vise le `run` de `Node`).
        let a_departager: Vec<String> = uuids
            .iter()
            .filter(|sym| mentioners_by_symbol.get(*sym).is_some_and(|ms| ms.iter().any(|m| !m.qualifier_types.is_empty())))
            .flat_map(|sym| definers_by_symbol.get(sym).cloned().unwrap_or_default())
            .collect();
        let parents = self.parent_names(&a_departager)?;
        for sym in uuids {
            let no_definer: Vec<String> = Vec::new();
            let empty: Vec<Mention> = Vec::new();
            let definers = definers_by_symbol.get(sym).unwrap_or(&no_definer);
            if definers.is_empty() {
                report.still_pending += 1;
                continue;
            }
            if definers.len() > 1 {
                // Plusieurs définisseurs : on s'abstient, sauf pour une
                // mention dont le type désigne l'un d'eux. Une relation
                // manquante vaut mieux qu'une relation fausse — c'est
                // exactement la sur-connexion que RAGForge a payée.
                report.ambiguous += 1;
            }
            for m in mentioners_by_symbol.get(sym).unwrap_or(&empty).iter().cloned() {
                let target = if !m.qualifier_types.is_empty() {
                    // Le type lu choisit, et il est seul juge : un
                    // définisseur d'un autre type n'est pas la cible, même
                    // s'il est le seul.
                    let du_type: Vec<&String> = definers
                        .iter()
                        .filter(|d| parents.get(*d).is_some_and(|p| m.qualifier_types.contains(p)))
                        .collect();
                    match du_type.as_slice() {
                        [un] => (*un).clone(),
                        _ => continue,
                    }
                } else if definers.len() == 1 {
                    definers[0].clone()
                } else {
                    continue;
                };
                let tous = lot.is_none_or(|(scopes, _)| scopes.contains(&target));
                if m.from == target {
                    continue;
                }
                if !tous && !lot.is_some_and(|(_, mentionneurs)| mentionneurs.contains(&m.from)) {
                    continue;
                }
                // L'arête est du genre inscrit au rendez-vous. Seul `CONSUMES`
                // a une réciproque déclarée ; `IMPLEMENTS` et `INHERITS_FROM`
                // n'en ont pas, et on n'en invente pas.
                let rel = if RELATIONS.iter().any(|(r, _, _)| *r == m.kind) { m.kind.as_str() } else { "CONSUMES" };
                self.link_jusqu_a(rel, RefOrUuid::Uuid(m.from.clone()), RefOrUuid::Uuid(target.clone()), m.usage.clone(), crate::disponibilite::Disponibilites::AUCUNE)?;
                if rel == "CONSUMES" {
                    self.link_jusqu_a("CONSUMED_BY", RefOrUuid::Uuid(target.clone()), RefOrUuid::Uuid(m.from), m.usage, crate::disponibilite::Disponibilites::AUCUNE)?;
                }
                report.linked_across_batches += 1;
            }
        }
        Ok(())
    }

    /// **Résoudre de nouveau des noms**, toutes leurs arêtes : après le
    /// retrait de scopes, un nom ambigu peut être redevenu unique, et ses
    /// mentionneurs — d'autres fichiers, qu'aucun lot ne repasse — doivent
    /// gagner leur arête. Les synchronisations du code
    /// (`code_sync::reingest_file`, `code_sync::sync_source`) l'appellent
    /// avec les noms des scopes qu'elles viennent de retirer.
    pub fn resoudre_les_symboles(
        &mut self,
        names: &[String],
        exige: crate::disponibilite::Disponibilites,
    ) -> Result<CodeIngestReport, CatalogError> {
        let mut report = CodeIngestReport::default();
        if names.is_empty() {
            return Ok(report);
        }
        let uuids: Vec<String> = names
            .iter()
            .map(|n| self.entity_uuid(SYMBOL, &BTreeMap::from([("name".to_string(), s(n))])))
            .collect::<Result<_, _>>()?;
        self.materialiser_les_symboles(&uuids, None, &mut report)?;
        let linked = self.drain_jusqu_a(exige);
        report.failed += linked.failed;
        Ok(report)
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
            .map(|(k, v)| (k, v.into_iter().map(|m| m.from).collect()))
            .collect())
    }

    /// Le `parent_name` de scopes, par uuid.
    fn parent_names(&self, uuids: &[String]) -> Result<std::collections::HashMap<String, String>, CatalogError> {
        let mut out = std::collections::HashMap::new();
        if uuids.is_empty() {
            return Ok(out);
        }
        let param = CypherValue::List(uuids.iter().map(|u| CypherValue::String(u.clone())).collect());
        let result = self
            .conn()
            .execute_with_params(
                &format!("UNWIND $uuids AS uid MATCH (s:{SCOPE} {{_uuid: uid}}) RETURN uid, s.parent_name"),
                &[crate::connection::QueryParam::new("uuids", param)],
            )
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        for row in &result.rows {
            if let (Some(CypherValue::String(u)), Some(CypherValue::String(p))) = (row.first(), row.get(1)) {
                out.insert(u.clone(), p.clone());
            }
        }
        Ok(out)
    }

    /// Comme [`Self::linked_from_many`], mais rend aussi la propriété `kind`
    /// et l'usage (`usage`, `usages`, `line`)
    /// de l'arête quand `with_kind` — le genre inscrit au rendez-vous.
    fn linked_from_many_with_kind(
        &self,
        rel: &str,
        to_uuids: &[String],
        with_kind: bool,
    ) -> Result<std::collections::HashMap<String, Vec<Mention>>, CatalogError> {
        let mut out: std::collections::HashMap<String, Vec<Mention>> = std::collections::HashMap::new();
        if to_uuids.is_empty() {
            return Ok(out);
        }
        let kind_expr = if with_kind { ", r.kind, r.usage, r.usages, r.line, r.qualifier_types" } else { "" };
        let cypher = format!(
            // Étiqueté : sans `:Symbol`, le moteur cherchait le nœud dans
            // toutes les tables, à chaque symbole de chaque lot.
            "UNWIND $uuids AS uid MATCH (n:{SYMBOL} {{_uuid: uid}})<-[r:{rel}]-(m) RETURN uid, m._uuid{kind_expr}"
        );
        let param = CypherValue::List(to_uuids.iter().map(|u| CypherValue::String(u.clone())).collect());
        let result = self
            .conn()
            .execute_with_params(&cypher, &[crate::connection::QueryParam::new("uuids", param)])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;
        for row in &result.rows {
            if let (Some(CypherValue::String(to)), Some(CypherValue::String(from))) = (row.first(), row.get(1)) {
                let kind = row.get(2).and_then(|v| v.as_str()).unwrap_or("CONSUMES").to_string();
                // L'usage du rendez-vous, que l'arête matérialisée recopie ;
                // vide pour un rendez-vous posé avant qu'il existe.
                let mut usage = BTreeMap::new();
                if let Some(u) = row.get(3).and_then(|v| v.as_str()) {
                    usage.insert("usage".to_string(), s(u));
                    usage.insert("usages".to_string(), row.get(4).cloned().unwrap_or(CypherValue::Null));
                    usage.insert("line".to_string(), row.get(5).cloned().unwrap_or(CypherValue::Null));
                }
                let types: Vec<String> = row
                    .get(6)
                    .and_then(|v| v.as_str())
                    .map(|t| t.split(',').filter(|x| !x.is_empty()).map(String::from).collect())
                    .unwrap_or_default();
                out.entry(to.clone()).or_default().push(Mention { from: from.clone(), kind, usage, qualifier_types: types });
            }
        }
        Ok(out)
    }
}

/// Un rendez-vous relu : qui mentionne, le genre d'arête inscrit, l'usage à
/// recopier, et les types par lesquels le nom est atteint.
#[derive(Debug, Clone)]
struct Mention {
    from: String,
    kind: String,
    usage: BTreeMap<String, CypherValue>,
    qualifier_types: Vec<String>,
}

/// **Le texte propre de chaque scope d'un fichier.**
///
/// Pour un scope, c'est la tranche du fichier de sa déclaration à sa fin,
/// où chaque **enfant direct** (le scope le plus large strictement contenu)
/// est remplacé par sa ligne de signature suivie de `…`. Une méthode garde
/// tout son texte ; un `impl` garde ce qu'il déclare et la liste de ses
/// méthodes ; un module, ses déclarations. Puis dé-indenté comme l'analyseur
/// le fait, en sautant la première ligne.
///
/// `None` pour un scope sans empan (les `file_scope_NN`, dont le texte est
/// déjà celui d'un trou entre déclarations) : on garde ce que l'analyseur a
/// donné. Les fermetures et les blocs ne sont pas des enfants à remplacer :
/// ils font partie du corps qui les contient.
///
/// C'est ce que ragforge fait dans son analyseur (une classe = sa ligne de
/// déclaration, la hiérarchie en relations). Ici comme là, c'est
/// l'**analyseur** qui décide ce qu'est le contenu d'un scope ; le catalogue
/// ne sait pas ce qu'est du code, et `content` reste son champ de contenu
/// (doc du 6 septembre 2026, 19h57).
fn own_texts(texte: &str, scopes: &[codeparsers::scope_extraction::types::ScopeInfo]) -> Vec<Option<(String, String)>> {
    let n = scopes.len();
    let empan_valide = |i: usize| {
        let s = &scopes[i];
        s.scope_end_byte > s.scope_start_byte
            && s.scope_end_byte <= texte.len()
            && texte.is_char_boundary(s.scope_start_byte)
            && texte.is_char_boundary(s.scope_end_byte)
    };
    // Les empans réels, du plus large au plus étroit à même début.
    let mut ordre: Vec<usize> = (0..n).filter(|&i| empan_valide(i)).collect();
    ordre.sort_by_key(|&i| (scopes[i].scope_start_byte, std::cmp::Reverse(scopes[i].scope_end_byte)));
    // Le parent direct de chaque scope, par imbrication d'empans.
    let mut enfants: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut pile: Vec<usize> = Vec::new();
    for &i in &ordre {
        let (debut, fin) = (scopes[i].scope_start_byte, scopes[i].scope_end_byte);
        while let Some(&haut) = pile.last() {
            if debut >= scopes[haut].scope_start_byte && fin <= scopes[haut].scope_end_byte {
                break;
            }
            pile.pop();
        }
        let remplacable = !matches!(scope_type_name(&scopes[i].r#type), "lambda" | "block");
        if let Some(&parent) = pile.last() {
            if remplacable {
                enfants[parent].push(i);
            }
        }
        pile.push(i);
    }
    (0..n)
        .map(|i| {
            if !empan_valide(i) {
                return None;
            }
            let s = &scopes[i];
            let mut propre = String::new();
            // Les replis, au format de `SourceLines::folds` : la ligne du
            // texte propre qui tient lieu d'un enfant de plusieurs lignes, et
            // l'empan de cet enfant dans le fichier.
            let mut replis: Vec<String> = Vec::new();
            let mut curseur = s.scope_start_byte;
            for &c in &enfants[i] {
                let (cd, cf) = (scopes[c].scope_start_byte, scopes[c].scope_end_byte);
                if cd < curseur || cf > s.scope_end_byte {
                    continue;
                }
                propre.push_str(&texte[curseur..cd]);
                let tranche = &texte[cd..cf];
                propre.push_str(tranche.lines().next().unwrap_or("").trim_end());
                if tranche.contains('\n') {
                    propre.push_str(" …");
                    let ligne = propre.matches('\n').count();
                    replis.push(format!("{ligne}:{}-{}", scopes[c].scope_start_line, scopes[c].scope_end_line));
                }
                curseur = cf;
            }
            propre.push_str(&texte[curseur..s.scope_end_byte]);
            Some((dedent_after_first_line(&propre), replis.join(",")))
        })
        .collect()
}

/// Retire l'indentation commune des lignes **après la première** — la
/// première est livrée sans indentation par tree-sitter, comme dans
/// `codeparsers::dedent_content`.
fn dedent_after_first_line(texte: &str) -> String {
    let lignes: Vec<&str> = texte.lines().collect();
    let commun = lignes
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = String::with_capacity(texte.len());
    for (i, l) in lignes.iter().enumerate() {
        if i == 0 {
            // L'empan commence au début de la ligne, indentation comprise.
            out.push_str(l.trim_start());
            continue;
        }
        {
            out.push('\n');
            if !l.trim().is_empty() {
                let coupe = l.char_indices().nth(commun).map(|(b, _)| b).unwrap_or(l.len());
                out.push_str(&l[coupe..]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_SRC: &str = "use serde::Serialize;\n\npub struct Point {\n    x: i32,\n}\n\nimpl Point {\n    pub fn norm(&self) -> i32 {\n        self.x.abs()\n    }\n}\n\npub fn twice(p: &Point) -> i32 {\n    p.norm() * 2\n}\n";

    /// **Le rendez-vous porte le type par lequel le nom est atteint**, quand
    /// toutes les références en ont un.
    #[test]
    fn le_rendez_vous_porte_le_type_du_qualificatif() {
        let src = "pub fn go(n: &Node) {\n    n.run();\n}\n\npub fn mixte(n: &Node) {\n    n.stop();\n    stop();\n}\n";
        let a = analyze("/virtual", vec![("go.rs".into(), src.into())]);
        let types = |scope: &str, nom: &str| {
            let key = &a.scopes.iter().find(|s| s.name == scope).unwrap().key;
            a.pending_qualifier_types.iter().find(|(k, n, _)| k == key && n == nom).map(|(_, _, v)| v.clone())
        };
        assert_eq!(types("go", "run"), Some(vec!["Node".to_string()]));
        assert_eq!(types("mixte", "stop"), None, "une référence sans type : pas de type sur le rendez-vous");
    }

    /// **Une arête d'usage porte comment on se sert de la cible, et où.**
    /// `twice` prend un `&Point` (ligne 13) : son `CONSUMES` vers `Point` dit
    /// « type » et la ligne ; une relation de structure ne porte rien.
    #[test]
    fn une_arete_d_usage_porte_son_genre_et_sa_ligne() {
        let a = analyze("/virtual", vec![("a.rs".into(), RUST_SRC.into())]);
        let twice = a.scopes.iter().find(|s| s.name == "twice").expect("twice");
        let points: Vec<&str> = a.scopes.iter().filter(|s| s.name == "Point").map(|s| s.key.as_str()).collect();
        let r = a.relations.iter()
            .find(|r| r.rel == "CONSUMES" && r.from_key == twice.key && points.contains(&r.to_key.as_str()))
            .expect("twice CONSUMES Point");
        let props = usage_properties(&r.sites);
        assert_eq!(props.get("usage"), Some(&s("type")), "sites : {:?}", r.sites);
        assert_eq!(props.get("line"), Some(&CypherValue::Int(13)), "sites : {:?}", r.sites);
        let structure = a.relations.iter().find(|r| r.rel == "DEFINED_IN").expect("un DEFINED_IN");
        assert!(structure.sites.is_empty() && usage_properties(&structure.sites).is_empty());
    }

    /// **L'usage retenu est le plus fort présent**, pas le plus fréquent :
    /// une arête qui appelle une fois et nomme le type trois fois est un appel.
    #[test]
    fn l_usage_retenu_est_le_plus_fort_present() {
        let site = |usage, line| UsageSite { usage, line: Some(line) };
        let props = usage_properties(&[
            site(UsageKind::Type, 4),
            site(UsageKind::Type, 5),
            site(UsageKind::Call, 7),
            site(UsageKind::Other, 2),
        ]);
        assert_eq!(props.get("usage"), Some(&s("call")));
        assert_eq!(props.get("usages"), Some(&s("call,other,type")));
        assert_eq!(props.get("line"), Some(&CypherValue::Int(2)));
        let sans_ligne = usage_properties(&[UsageSite { usage: UsageKind::Import, line: None }]);
        assert_eq!(sans_ligne.get("line"), Some(&CypherValue::Null), "la clé reste, pour le chemin COPY");
    }

    /// **Un scope embarque son texte propre, pas celui de ses enfants.** Un
    /// `impl` à deux méthodes garde ses deux signatures et aucun corps ; une
    /// méthode garde tout son texte, signature comprise ; le module qui
    /// contient l'impl ne voit que la ligne `impl`.
    #[test]
    fn un_scope_embarque_son_texte_propre_pas_celui_de_ses_enfants() {
        let src = "mod calcul {\n    pub struct Compteur { n: u32 }\n\n    impl Compteur {\n        pub fn inc(&mut self) {\n            self.n += 1;\n        }\n\n        pub fn total(&self) -> u32 {\n            let f = |x: u32| x * 2;\n            f(self.n)\n        }\n    }\n}\n";
        let a = analyze("/virtual", vec![("c.rs".into(), src.into())]);
        let par_nom = |nom: &str| a.scopes.iter().find(|s| s.name == nom).unwrap_or_else(|| panic!("scope {nom}"));
        let imp = a.scopes.iter().find(|s| s.content.starts_with("impl Compteur")).expect("l'impl");
        assert!(imp.content.contains("pub fn inc(&mut self) {"), "{}", imp.content);
        assert!(imp.content.contains("pub fn total(&self) -> u32 {"), "{}", imp.content);
        assert!(!imp.content.contains("self.n += 1"), "le corps d'une méthode n'est pas dans l'impl : {}", imp.content);
        assert!(imp.content.contains("…"), "{}", imp.content);
        let inc = par_nom("inc");
        assert!(inc.content.starts_with("pub fn inc"), "la méthode garde sa signature : {}", inc.content);
        assert!(inc.content.contains("self.n += 1"), "{}", inc.content);
        let total = par_nom("total");
        assert!(total.content.contains("|x: u32| x * 2"), "une fermeture fait partie du corps : {}", total.content);
        let m = par_nom("calcul");
        assert!(m.content.contains("impl Compteur {"), "{}", m.content);
        assert!(!m.content.contains("pub fn inc"), "le module ne voit que la ligne impl : {}", m.content);
    }

    /// **Les replis rendent les lignes du fichier.** Dans l'`impl`, chaque
    /// méthode tient en une ligne : `folds` dit laquelle et l'empan qu'elle
    /// remplace. Pour tout scope aux lignes connues, chaque ligne non repliée
    /// de `content` est bien celle du fichier à la position calculée.
    #[test]
    fn les_replis_rendent_les_lignes_du_fichier() {
        let src = "mod calcul {\n    pub struct Compteur { n: u32 }\n\n    impl Compteur {\n        pub fn inc(&mut self) {\n            self.n += 1;\n        }\n\n        pub fn total(&self) -> u32 {\n            let f = |x: u32| x * 2;\n            f(self.n)\n        }\n    }\n}\n";
        let a = analyze("/virtual", vec![("c.rs".into(), src.into()), ("a.rs".into(), RUST_SRC.into())]);
        let imp = a.scopes.iter().find(|s| s.content.starts_with("impl Compteur")).expect("l'impl");
        assert_eq!(imp.start_line, 4);
        assert_eq!(imp.folds, "1:5-7,3:9-12");
        for (fichier, texte) in [("/virtual/c.rs", src), ("/virtual/a.rs", RUST_SRC)] {
            let lignes: Vec<&str> = texte.lines().collect();
            for s in a.scopes.iter().filter(|s| s.file_path == fichier && s.folds != "?") {
                let replis: Vec<(usize, usize, usize)> = s.folds.split(',').filter(|m| !m.is_empty()).map(|m| {
                    let (l, e) = m.split_once(':').unwrap();
                    let (d, f) = e.split_once('-').unwrap();
                    (l.parse().unwrap(), d.parse().unwrap(), f.parse().unwrap())
                }).collect();
                for (k, ligne) in s.content.lines().enumerate() {
                    let n = s.start_line + k + replis.iter().filter(|(l, _, _)| *l < k).map(|(_, d, f)| f - d).sum::<usize>();
                    let attendu = lignes[n - 1].trim();
                    if replis.iter().any(|(l, _, _)| *l == k) {
                        assert!(ligne.trim().starts_with(attendu.trim_end_matches('{').trim()), "{} ligne {k} : {ligne:?} / {attendu:?}", s.name);
                    } else {
                        assert_eq!(ligne.trim(), attendu, "{} ligne {k} → fichier {n}", s.name);
                    }
                }
            }
        }
    }

    #[test]
    fn analyze_yields_files_scopes_and_relations_by_key() {
        let a = analyze(
            "/virtual",
            vec![
                ("a.rs".into(), RUST_SRC.into()),
                // Depuis le 30 août 2026, un fichier sans grammaire **entre**.
                ("README.md".into(), "# un titre\n".into()),
                // Ce qui sort encore, et pourquoi : un artefact de build.
                ("Cargo.lock".into(), "[[package]]\n".into()),
            ],
        );
        assert_eq!(a.files.len(), 2, "le code et le texte entrent, l'artefact non");
        let rs = a.files.iter().find(|f| f.path.ends_with("a.rs")).expect("a.rs");
        // L'identité d'un fichier est son chemin **absolu dans sa source**,
        // pas son chemin relatif à la racine d'analyse (doc 04 v3).
        assert_eq!(rs.path, "/virtual/a.rs");
        assert_eq!(rs.source, LOCAL_SOURCE);
        assert_eq!(rs.language, "rust");
        assert!(!rs.content_hash.is_empty());

        // ── Le fichier sans grammaire : un File, un Scope, une arête ──────
        let md = a.files.iter().find(|f| f.path.ends_with("README.md")).expect("README.md entre dans l'index");
        // Le même vocabulaire que le code : `markdown`, pas `md`.
        assert_eq!(md.language, "markdown");
        let texte = a.scopes.iter().find(|s| s.scope_type == "texte_brut").expect("un scope texte_brut");
        assert_eq!(texte.file_path, "/virtual/README.md");
        assert_eq!(texte.language, "markdown");
        assert_eq!(texte.start_byte, 0);
        assert_eq!(texte.end_byte, "# un titre\n".len(), "le scope couvre tout le fichier");
        assert!(texte.signature.is_empty(), "pas de signature inventée");
        assert!(
            a.relations.iter().any(|r| r.rel == "DEFINED_IN" && r.from_key == texte.key && r.to_key == "/virtual/README.md"),
            "le scope texte est relié à son fichier"
        );
        // Rien d'autre : pas de rendez-vous, il n'y a aucun nom à résoudre.
        assert!(!a.pending.iter().any(|(k, _, _)| *k == texte.key), "un texte ne prend pas de rendez-vous");

        // ── Ce qui est écarté le dit ──────────────────────────────────────
        assert_eq!(a.skipped.len(), 1, "{:?}", a.skipped);
        // Retrouvé par son nom, pas par son rang : ce qu'on veut dire est
        // « le verrou a été écarté », pas « le premier écarté est le verrou ».
        // Le jour où un second fichier sort, une assertion accrochée au rang
        // lirait la ligne d'à côté au lieu d'échouer.
        let (chemin, raison) =
            a.skipped.iter().find(|(c, _)| c.ends_with("Cargo.lock")).expect("le verrou est écarté");
        assert!(chemin.ends_with("Cargo.lock"));
        assert!(!raison.is_empty(), "un refus se nomme");

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

#[cfg(test)]
mod tests_vocabulaire {
    use super::*;

    /// **Un genre de scope qui n'est pas dans la carte n'est pas filtrable.**
    ///
    /// `scope_type` est indexé, donc `search(filter="scope_type != 'texte_brut'")`
    /// marche — mais seulement pour qui sait que `texte_brut` existe. Ce test
    /// tient la promesse : ajouter une variante à `ScopeInfoType` sans
    /// l'annoncer casse ici, pas chez un agent qui filtre dans le vide.
    #[test]
    fn le_vocabulaire_de_scope_type_couvre_tous_les_genres() {
        let cfg = scope_config(default_scope_chunking());
        let declares = cfg.fields["scope_type"].values.clone().expect("scope_type déclare ses valeurs");

        use codeparsers::scope_extraction::types::ScopeInfoType as T;
        let tous = [
            T::Class, T::Interface, T::Function, T::Method, T::Enum, T::TypeAlias,
            T::Namespace, T::Module, T::Variable, T::Lambda, T::Constant, T::Block,
            T::TexteBrut,
        ];
        for genre in &tous {
            let nom = scope_type_name(genre);
            assert!(declares.iter().any(|v| v == nom), "le genre '{nom}' n'est pas dans le vocabulaire déclaré");
        }
        assert_eq!(declares.len(), tous.len(), "le vocabulaire annonce un genre qui n'existe pas");
    }

    /// Les deux vocabulaires de `language` avaient divergé : le code portait des
    /// noms (`rust`, `cpp`), le texte des extensions brutes (`md`, `sh`). Un
    /// agent qui a appris l'un échouait silencieusement sur l'autre.
    #[test]
    fn language_ne_porte_qu_un_vocabulaire() {
        assert_eq!(texte_language_name("notes/README.md"), "markdown");
        assert_eq!(texte_language_name("deploy.sh"), "shell");
        assert_eq!(texte_language_name("ci.yml"), "yaml");
        assert_eq!(texte_language_name("q.cypher"), "cypher");
        // Un langage qu'on ne sait pas parser garde son nom : c'est
        // `scope_type` qui dit qu'on ne l'a pas lu, pas `language`.
        assert_eq!(texte_language_name("Value.java"), "java");
        // Une extension inconnue se rend telle quelle plutôt que « unknown ».
        assert_eq!(texte_language_name("truc.zzz"), "zzz");
    }

    /// La politique se lit dans un seul sens, et chaque refus se nomme.
    #[test]
    fn le_verdict_dit_toujours_pourquoi() {
        assert!(matches!(verdict("src/main.rs", 100), Verdict::Code));
        assert!(matches!(verdict("README.md", 100), Verdict::Texte));
        assert!(matches!(verdict("Cargo.lock", 100), Verdict::Ecarte(_)));
        assert!(matches!(verdict("data/big.csv", 100), Verdict::Ecarte(_)));
        assert!(matches!(verdict("app.min.js", 100), Verdict::Ecarte(_)));
        assert!(matches!(verdict("Makefile", 100), Verdict::Ecarte(_)));
        // Le seuil ne s'applique qu'au texte : un fichier de code de 200 Kio
        // reste du code, on sait le lire.
        assert!(matches!(verdict("gros.rs", 200 * 1024), Verdict::Code));
        assert!(matches!(verdict("dump.md", 200 * 1024), Verdict::Ecarte(_)));
    }
}
