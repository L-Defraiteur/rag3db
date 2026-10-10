//! **Les formes de requêtes**, que chaque dialecte traduit. Une forme décrit
//! ce qu'on veut lire ou écrire, jamais comment une base le dit ; une forme
//! de plus s'ajoute quand un nœud en a besoin, et l'optimisation (fondre une
//! suite de sauts en une requête) reste sous le dialecte.
//!
//! Aujourd'hui : [`Hop`]. Viendront `Count`, `Select`, `Write` et `Tx`.

use std::fmt;

/// Le sens d'un saut, vu du nœud de départ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Le départ est la source de l'arête : `(d)-[r]->(m)`.
    Outgoing,
    /// Le départ est la cible de l'arête : `(d)<-[r]-(m)`.
    Incoming,
}

/// Écarter des arêtes par la valeur d'un de leurs champs : une arête est
/// gardée si le champ est nul **ou** hors des valeurs écartées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeExclusion {
    pub field: String,
    pub values: Vec<String>,
}

/// **Un saut** : depuis une liste d'uuids (le paramètre `$uuids`), suivre une
/// relation dans un sens, et rendre une ligne par arête suivie.
///
/// Les colonnes rendues, dans cet ordre : l'uuid de départ, puis les champs
/// du nœud atteint ([`Hop::fields`]), puis ceux de l'arête
/// ([`Hop::edge_fields`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hop {
    /// La table du nœud de départ (celle des uuids donnés).
    pub start: String,
    pub relation: String,
    /// La table du nœud atteint.
    pub end: String,
    pub direction: Direction,
    pub fields: Vec<String>,
    pub edge_fields: Vec<String>,
    pub exclude: Option<EdgeExclusion>,
}

impl Hop {
    /// Un saut qui ne rend que l'uuid du nœud atteint.
    pub fn new(start: impl Into<String>, relation: impl Into<String>, end: impl Into<String>, direction: Direction) -> Self {
        Self {
            start: start.into(),
            relation: relation.into(),
            end: end.into(),
            direction,
            fields: vec!["_uuid".into()],
            edge_fields: Vec::new(),
            exclude: None,
        }
    }

    /// Tout ce qui entre dans le texte d'une requête est un identifiant :
    /// tables, relation, champs, et les valeurs écartées.
    pub fn validate(&self) -> Result<(), TranslateError> {
        let mut noms: Vec<&str> = vec![&self.start, &self.relation, &self.end];
        noms.extend(self.fields.iter().map(String::as_str));
        noms.extend(self.edge_fields.iter().map(String::as_str));
        if let Some(x) = &self.exclude {
            noms.push(&x.field);
            noms.extend(x.values.iter().map(String::as_str));
        }
        match noms.into_iter().find(|n| !crate::is_valid_identifier(n)) {
            Some(n) => Err(TranslateError::Invalid(format!("saut : « {n} » n'est pas un identifiant"))),
            None => Ok(()),
        }
    }
}

/// Pourquoi une forme ne se traduit pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslateError {
    /// Le dialecte ne sait pas dire cette forme.
    Untranslated { dialect: String, form: &'static str },
    /// La forme elle-même est fausse (un nom qui n'est pas un identifiant).
    Invalid(String),
}

impl fmt::Display for TranslateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Untranslated { dialect, form } => write!(f, "le dialecte {dialect} ne traduit pas la forme {form}"),
            Self::Invalid(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for TranslateError {}
