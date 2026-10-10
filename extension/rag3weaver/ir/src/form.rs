//! **Les formes de requêtes**, que chaque dialecte traduit. Une forme décrit
//! ce qu'on veut lire ou écrire, jamais comment une base le dit ; une forme
//! de plus s'ajoute quand un nœud en a besoin, et l'optimisation (fondre une
//! suite de sauts en une requête) reste sous le dialecte.
//!
//! Aujourd'hui : [`Hop`] et [`Count`]. Viendront `Select`, `Write` et `Tx`.

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

/// Une colonne rendue par un saut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Column {
    /// Un champ du nœud atteint.
    Node(String),
    /// Un champ de l'arête suivie.
    Edge(String),
    /// Une colonne vide, pour garder les positions quand un champ n'est pas
    /// déclaré (ou qu'une relation ne le porte pas).
    Null,
    /// La table du nœud atteint (utile quand le saut ne la fixe pas).
    Label,
    /// Le nœud atteint entier, tous ses champs.
    Whole,
}

/// **Un saut** : depuis une liste d'uuids (le paramètre `$uuids`), suivre une
/// relation dans un sens, et rendre une ligne par arête suivie.
///
/// Les colonnes rendues : l'uuid de départ, puis [`Hop::returns`] dans
/// l'ordre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hop {
    /// La table du nœud de départ (celle des uuids donnés) ; `None` : un
    /// nœud de n'importe quelle table, retrouvé par son seul uuid.
    pub start: Option<String>,
    pub relation: String,
    /// La table du nœud atteint ; `None` : n'importe laquelle.
    pub end: Option<String>,
    pub direction: Direction,
    pub returns: Vec<Column>,
    pub exclude: Option<EdgeExclusion>,
}

impl Hop {
    /// Un saut qui ne rend que l'uuid du nœud atteint.
    pub fn new(start: impl Into<String>, relation: impl Into<String>, end: impl Into<String>, direction: Direction) -> Self {
        Self {
            start: Some(start.into()),
            relation: relation.into(),
            end: Some(end.into()),
            direction,
            returns: vec![Column::Node("_uuid".into())],
            exclude: None,
        }
    }

    /// Un saut dont le départ et l'arrivée peuvent être de n'importe quelle
    /// table : les uuids suffisent à les retrouver.
    pub fn untyped(relation: impl Into<String>, direction: Direction) -> Self {
        Self { start: None, end: None, ..Self::new("", relation, "", direction) }
    }

    /// Tout ce qui entre dans le texte d'une requête est un identifiant :
    /// tables, relation, champs, et les valeurs écartées.
    pub fn validate(&self) -> Result<(), TranslateError> {
        let mut noms: Vec<&str> = vec![&self.relation];
        noms.extend(self.start.as_deref());
        noms.extend(self.end.as_deref());
        noms.extend(self.returns.iter().filter_map(|c| match c {
            Column::Node(f) | Column::Edge(f) => Some(f.as_str()),
            Column::Null | Column::Label | Column::Whole => None,
        }));
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

/// **Un compte.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Count {
    /// Les lignes d'une table, en une colonne.
    Rows { table: String },
    /// Le degré : pour chaque uuid de départ (le paramètre `$uuids`), le
    /// nombre d'arêtes d'une relation dans un sens. Deux colonnes : l'uuid,
    /// le compte ; un départ sans arête n'a pas de ligne.
    Edges { start: String, relation: String, direction: Direction },
}

impl Count {
    pub fn validate(&self) -> Result<(), TranslateError> {
        let noms: Vec<&str> = match self {
            Count::Rows { table } => vec![table],
            Count::Edges { start, relation, .. } => vec![start, relation],
        };
        match noms.into_iter().find(|n| !crate::is_valid_identifier(n)) {
            Some(n) => Err(TranslateError::Invalid(format!("compte : « {n} » n'est pas un identifiant"))),
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
