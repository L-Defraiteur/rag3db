//! Le moteur de script générique : un script se **prépare** (lu, vérifié,
//! compilé s'il y a lieu) puis s'**appelle** avec une valeur JSON et rend une
//! valeur JSON. Le langage est un point de branchement : `rhai` (l'historique,
//! une expression qui lit la constante `input`), `typescript` (le langage du
//! produit) et `javascript` (du TypeScript sans types). En TypeScript et en
//! JavaScript le script définit `function run(input)`, qui rend la sortie.
//!
//! Ce que le script a le droit d'appeler, c'est ce qu'on lui donne : rien par
//! défaut, ni fichiers ni réseau. Chaque appel est borné par les
//! [`ScriptLimits`] de l'hôte (temps, mémoire, tailles), qu'un script ne peut
//! pas relever.
//!
//! TypeScript n'est jamais vérifié à la `tsc` : les types sont effacés au
//! chargement, et ce sont les ports du nœud qui vérifient entrées et sorties à
//! la frontière.
use serde_json::Value;
use std::sync::Arc;

mod quickjs;
mod rhai;
#[cfg(test)]
mod tests;
mod typescript;

/// Les langages connus, sous le nom qu'une déclaration leur donne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rhai,
    TypeScript,
    JavaScript,
}

impl Language {
    pub const ALL: [Language; 3] = [Language::Rhai, Language::TypeScript, Language::JavaScript];

    pub fn name(self) -> &'static str {
        match self {
            Language::Rhai => "rhai",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
        }
    }

    pub fn parse(name: &str) -> Result<Self, ScriptError> {
        Self::ALL
            .into_iter()
            .find(|l| l.name() == name)
            .ok_or_else(|| {
                let known: Vec<_> = Self::ALL.iter().map(|l| l.name()).collect();
                ScriptError::new(
                    ScriptErrorKind::UnknownLanguage,
                    format!(
                        "unknown script language '{name}' (known: {})",
                        known.join(", ")
                    ),
                )
            })
    }
}

/// Bornes de l'hôte, par appel. `operations` ne vaut que pour rhai ;
/// `memory_bytes` ne vaut que pour TypeScript et JavaScript (rhai borne ses
/// tailles de chaînes et de collections).
#[derive(Clone, Debug)]
pub struct ScriptLimits {
    pub operations: u64,
    pub source_bytes: usize,
    pub json_bytes: usize,
    pub collection_items: usize,
    pub timeout_ms: u64,
    pub memory_bytes: usize,
}

impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            operations: 1_000_000,
            source_bytes: 65536,
            json_bytes: 16 * 1024 * 1024,
            collection_items: 131072,
            timeout_ms: 1000,
            memory_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptErrorKind {
    UnknownLanguage,
    /// Une borne de l'hôte invalide ou dépassée (taille, opérations, pile).
    Limit,
    Timeout,
    Memory,
    Syntax,
    /// Du TypeScript qui ne s'efface pas (enum, namespace…).
    Unsupported,
    /// Le script a demandé ce qu'on ne lui donne pas ; porte le nom demandé.
    Forbidden(String),
    /// Pas de `function run(input)`.
    MissingEntry,
    Runtime,
    /// La sortie n'est pas une valeur JSON acceptable.
    Output,
}

/// Une erreur de script, avec la position dans **le source tel qu'écrit**
/// quand le moteur la connaît (lignes à partir de 1).
#[derive(Debug, Clone)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl ScriptError {
    pub fn new(kind: ScriptErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub fn at(mut self, line: Option<u32>, column: Option<u32>) -> Self {
        self.line = line;
        self.column = column;
        self
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)?;
        match (self.line, self.column) {
            (Some(l), Some(c)) => write!(f, " (line {l}, column {c})"),
            (Some(l), None) => write!(f, " (line {l})"),
            _ => Ok(()),
        }
    }
}

impl std::error::Error for ScriptError {}

/// Un script prêt à être appelé, autant de fois qu'on veut.
pub trait PreparedScript: Send + Sync {
    fn language(&self) -> Language;
    fn call(&self, input: &Value, limits: &ScriptLimits) -> Result<Value, ScriptError>;
}

/// Ce que chaque langage remplit.
pub trait ScriptEngine: Send + Sync {
    fn language(&self) -> Language;
    fn prepare(
        &self,
        source: &str,
        limits: &ScriptLimits,
    ) -> Result<Arc<dyn PreparedScript>, ScriptError>;
}

pub fn engine(language: Language) -> &'static dyn ScriptEngine {
    match language {
        Language::Rhai => &rhai::RhaiEngine,
        Language::TypeScript => &typescript::TypeScriptEngine,
        Language::JavaScript => &quickjs::QuickJsEngine,
    }
}

/// Prépare un script dans le langage nommé.
pub fn prepare(
    language: &str,
    source: &str,
    limits: &ScriptLimits,
) -> Result<Arc<dyn PreparedScript>, ScriptError> {
    engine(Language::parse(language)?).prepare(source, limits)
}

/// Prépare et appelle une fois.
pub fn evaluate(
    language: &str,
    source: &str,
    input: &Value,
    limits: &ScriptLimits,
) -> Result<Value, ScriptError> {
    prepare(language, source, limits)?.call(input, limits)
}

/// Le nom que portent les messages de bornes : « Rhai » garde les messages
/// d'avant le moteur générique, mot pour mot.
fn label(language: Language) -> &'static str {
    match language {
        Language::Rhai => "Rhai",
        Language::TypeScript | Language::JavaScript => "Script",
    }
}

/// Contrôles communs à tous les langages, avant de toucher au moteur.
fn check_limits(
    language: Language,
    limits: &ScriptLimits,
    source: &str,
) -> Result<(), ScriptError> {
    if limits.operations == 0
        || limits.timeout_ms == 0
        || limits.collection_items == 0
        || limits.memory_bytes == 0
        || source.len() > limits.source_bytes
    {
        return Err(ScriptError::new(
            ScriptErrorKind::Limit,
            format!("{} input or host limit invalid/exceeded", label(language)),
        ));
    }
    Ok(())
}

fn check_input(
    language: Language,
    limits: &ScriptLimits,
    input: &Value,
) -> Result<(), ScriptError> {
    if input.to_string().len() > limits.json_bytes {
        return Err(ScriptError::new(
            ScriptErrorKind::Limit,
            format!("{} input or host limit invalid/exceeded", label(language)),
        ));
    }
    Ok(())
}

fn check_output(
    language: Language,
    limits: &ScriptLimits,
    output: Value,
) -> Result<Value, ScriptError> {
    if output.to_string().len() > limits.json_bytes {
        return Err(ScriptError::new(
            ScriptErrorKind::Output,
            format!("{} output too large", label(language)),
        ));
    }
    Ok(output)
}
