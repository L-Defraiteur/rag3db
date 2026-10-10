//! **Les formes de requêtes**, que chaque dialecte traduit. Une forme décrit
//! ce qu'on veut lire ou écrire, jamais comment une base le dit ; une forme
//! de plus s'ajoute quand un nœud en a besoin, et l'optimisation (fondre une
//! suite de sauts en une requête) reste sous le dialecte.
//!
//! Aujourd'hui : [`Hop`], [`Count`], [`Select`] et les premières variantes
//! de [`Write`]. `Tx` est un appel sur la connexion, pas une forme.

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
    /// Une condition sur le nœud atteint.
    pub filter: Option<Predicate>,
    /// L'ordre des lignes, par des champs du nœud atteint.
    pub order_by: Vec<String>,
    /// Au plus tant de lignes en tout (pour un départ unique : par départ).
    pub limit: Option<usize>,
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
            filter: None,
            order_by: Vec::new(),
            limit: None,
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
        noms.extend(self.order_by.iter().map(String::as_str));
        let mut du_filtre = Vec::new();
        if let Some(p) = &self.filter {
            p.names(&mut du_filtre);
        }
        noms.extend(du_filtre.iter().map(String::as_str));
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
    /// Les lignes qu'une condition retient, en une colonne.
    Matching { table: String, filter: Predicate },
}

impl Count {
    pub fn validate(&self) -> Result<(), TranslateError> {
        let noms: Vec<&str> = match self {
            Count::Rows { table } => vec![table],
            Count::Edges { start, relation, .. } => vec![start, relation],
            Count::Matching { table, filter } => {
                let mut v = vec![table.clone()];
                filter.names(&mut v);
                return match v.iter().find(|n| !crate::is_valid_identifier(n)) {
                    Some(n) => Err(TranslateError::Invalid(format!("compte : « {n} » n'est pas un identifiant"))),
                    None => Ok(()),
                };
            }
        };
        match noms.into_iter().find(|n| !crate::is_valid_identifier(n)) {
            Some(n) => Err(TranslateError::Invalid(format!("compte : « {n} » n'est pas un identifiant"))),
            None => Ok(()),
        }
    }
}

/// Une condition sur les champs d'une ligne ; les valeurs viennent des
/// paramètres nommés de la requête.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// Le champ vaut le paramètre.
    Equals { field: String, param: String },
    /// Le champ (un texte) contient le paramètre.
    Contains { field: String, param: String },
    /// Le champ vaut au moins le paramètre.
    AtLeast { field: String, param: String },
    /// L'une au moins des conditions.
    AnyOf(Vec<Predicate>),
    /// Une condition déjà dite dans la langue du dialecte, par son propre
    /// parseur de filtres (`FilterParser`), sur l'alias `m` : le filtre d'un
    /// utilisateur, avec ses paramètres. Elle ne se valide pas ici.
    Compiled(String),
}

impl Predicate {
    fn names(&self, out: &mut Vec<String>) {
        match self {
            Predicate::Equals { field, param } | Predicate::Contains { field, param } | Predicate::AtLeast { field, param } => {
                out.push(field.clone());
                out.push(param.clone());
            }
            Predicate::AnyOf(v) => v.iter().for_each(|p| p.names(out)),
            Predicate::Compiled(_) => {}
        }
    }
}

/// **Une sélection** dans une table : les lignes de uuids donnés (le
/// paramètre `$uuids`, l'uuid donné en première colonne), ou celles qu'une
/// condition retient ; puis des colonnes, un ordre, une limite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select {
    pub table: String,
    pub by_uuids: bool,
    pub filter: Option<Predicate>,
    /// Des champs de la ligne ([`Column::Node`]), des colonnes vides, ou la
    /// ligne entière ; pas de champ d'arête ici.
    pub returns: Vec<Column>,
    pub order_by: Vec<String>,
    pub limit: Option<usize>,
}

impl Select {
    /// Les lignes de uuids donnés.
    pub fn by_uuids(table: impl Into<String>, returns: Vec<Column>) -> Self {
        Self { table: table.into(), by_uuids: true, filter: None, returns, order_by: Vec::new(), limit: None }
    }

    /// Toutes les lignes de la table.
    pub fn all(table: impl Into<String>, returns: Vec<Column>) -> Self {
        Self { table: table.into(), by_uuids: false, filter: None, returns, order_by: Vec::new(), limit: None }
    }

    /// Les lignes qu'une condition retient.
    pub fn filtered(table: impl Into<String>, filter: Predicate, returns: Vec<Column>) -> Self {
        Self { table: table.into(), by_uuids: false, filter: Some(filter), returns, order_by: Vec::new(), limit: None }
    }

    pub fn validate(&self) -> Result<(), TranslateError> {
        let mut noms = vec![self.table.clone()];
        noms.extend(self.order_by.iter().cloned());
        if let Some(p) = &self.filter {
            p.names(&mut noms);
        }
        for c in &self.returns {
            match c {
                Column::Node(f) => noms.push(f.clone()),
                Column::Null | Column::Whole => {}
                Column::Edge(_) | Column::Label => {
                    return Err(TranslateError::Invalid("sélection : pas d'arête ni d'étiquette dans une table".into()));
                }
            }
        }
        match noms.iter().find(|n| !crate::is_valid_identifier(n)) {
            Some(n) => Err(TranslateError::Invalid(format!("sélection : « {n} » n'est pas un identifiant"))),
            None => Ok(()),
        }
    }
}

/// **Une écriture**, née de l'ingestion. Les lignes arrivent dans le
/// paramètre `$items` (une carte par ligne, ses clés sont les colonnes).
/// Les variantes suivantes (`Update`, `Mark`, `Delete`, `Unlink`, `Load`)
/// viendront à leur tour ; voir
/// `docs/8-octobre-2026-16h29/embarquements/05-write-et-tx.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Write {
    /// Créer ou remplacer des lignes par leur `_uuid`, en rendant
    /// `(identifiant interne, _uuid)` pour chacune.
    Upsert { table: String, columns: Vec<String> },
    /// Poser des arêtes entre des lignes données par leurs uuids
    /// (`from_uuid`, `to_uuid`, puis les propriétés). Les tables des deux
    /// bouts, quand on les connaît, évitent de chercher dans toutes.
    Link { relation: String, ends: Option<(String, String)>, props: Vec<String> },
    /// Mettre à jour des champs, **une valeur par ligne** : `$items` porte
    /// `_uuid` et les champs.
    Update { table: String, columns: Vec<String> },
    /// Marquer une liste d'uuids (`$uuids`) d'**une seule valeur** par champ :
    /// `None` pour vider le champ, `Some(param)` pour le paramètre nommé.
    Mark { table: String, set: Vec<(String, Option<String>)> },
    /// Supprimer les lignes dont le champ `by` (souvent `_uuid`) est dans
    /// `$uuids`, avec ou sans leurs arêtes ; `count` : rendre, par valeur de
    /// `by`, le nombre de lignes supprimées.
    Delete { table: String, by: String, cascade: bool, count: bool },
    /// Supprimer des arêtes données par leurs bouts : `$items` porte `from`
    /// et `to` (les uuids), une carte par arête.
    Unlink { relation: String },
}

impl Write {
    pub fn validate(&self) -> Result<(), TranslateError> {
        let mut noms: Vec<&str> = Vec::new();
        match self {
            Write::Upsert { table, columns } => {
                noms.push(table);
                noms.extend(columns.iter().map(String::as_str));
            }
            Write::Link { relation, ends, props } => {
                noms.push(relation);
                if let Some((f, t)) = ends {
                    noms.extend([f.as_str(), t.as_str()]);
                }
                noms.extend(props.iter().map(String::as_str));
            }
            Write::Update { table, columns } => {
                noms.push(table);
                noms.extend(columns.iter().map(String::as_str));
            }
            Write::Mark { table, set } => {
                noms.push(table);
                for (f, p) in set {
                    noms.push(f);
                    noms.extend(p.as_deref());
                }
            }
            Write::Delete { table, by, .. } => noms.extend([table.as_str(), by.as_str()]),
            Write::Unlink { relation } => noms.push(relation),
        }
        match noms.into_iter().find(|n| !crate::is_valid_identifier(n)) {
            Some(n) => Err(TranslateError::Invalid(format!("écriture : « {n} » n'est pas un identifiant"))),
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
