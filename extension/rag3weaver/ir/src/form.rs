//! **Les formes de requêtes**, que chaque dialecte traduit. Une forme décrit
//! ce qu'on veut lire ou écrire, jamais comment une base le dit ; une forme
//! de plus s'ajoute quand un nœud en a besoin, et l'optimisation (fondre une
//! suite de sauts en une requête) reste sous le dialecte.
//!
//! Aujourd'hui : [`Hop`], [`Count`], [`Select`] et [`Write`]. `Tx` est un
//! appel sur la connexion, pas une forme.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Le sens d'un saut, vu du nœud de départ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// Une colonne rendue par un saut ou une sélection.
///
/// **En JSON** (un Plan déclaré) : une chaîne nue est un champ du nœud
/// ([`Column::Node`]), `{"edge": "champ"}` un champ de l'arête, `{"collect":
/// {…}}` une liste de lignes liées ([`Collect`]). Les colonnes vides, l'étiquette
/// et la ligne entière ne se déclarent pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ColonneDeclaree", into = "ColonneDeclaree")]
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
    /// **Les lignes liées, en une liste** (dans une sélection) : pour chaque
    /// ligne, celles qu'une relation atteint, réduites à quelques champs.
    Collect(Collect),
}

/// **Une colonne de lignes liées** : pour chaque ligne de la sélection, les
/// lignes de `end` que `relation` atteint dans `direction`, chacune réduite à
/// `fields`, gardées par `filter` (sur la ligne atteinte) ; une liste vide
/// quand il n'y en a pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collect {
    /// Le nom de la colonne rendue.
    #[serde(rename = "as")]
    pub alias: String,
    pub relation: String,
    pub direction: Direction,
    pub end: String,
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Predicate>,
}

/// La forme JSON d'une colonne (voir [`Column`]).
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ColonneDeclaree {
    Champ(String),
    Arete { edge: String },
    Liste { collect: Collect },
}

impl TryFrom<ColonneDeclaree> for Column {
    type Error = String;
    fn try_from(c: ColonneDeclaree) -> Result<Self, String> {
        Ok(match c {
            ColonneDeclaree::Champ(f) => Column::Node(f),
            ColonneDeclaree::Arete { edge } => Column::Edge(edge),
            ColonneDeclaree::Liste { collect } => Column::Collect(collect),
        })
    }
}

impl From<Column> for ColonneDeclaree {
    fn from(c: Column) -> Self {
        match c {
            Column::Node(f) => ColonneDeclaree::Champ(f),
            Column::Edge(f) => ColonneDeclaree::Arete { edge: f },
            Column::Collect(c) => ColonneDeclaree::Liste { collect: c },
            // Ni déclarables ni rendues en JSON : elles naissent du code.
            Column::Null => ColonneDeclaree::Champ("(vide)".into()),
            Column::Label => ColonneDeclaree::Champ("(étiquette)".into()),
            Column::Whole => ColonneDeclaree::Champ("(ligne entière)".into()),
        }
    }
}

/// **Une clé d'ordre** : un champ, ou le premier non nul de plusieurs
/// (`coalesce`), croissant sauf `desc`. En JSON : `{"field": "f"}` ou
/// `{"coalesce": ["a", "b"]}`, avec `"desc": true` au besoin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderKey {
    #[serde(flatten)]
    pub by: OrderBy,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub desc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    Field(String),
    Coalesce(Vec<String>),
}

impl OrderKey {
    /// Un champ, croissant.
    pub fn field(f: impl Into<String>) -> Self {
        Self { by: OrderBy::Field(f.into()), desc: false }
    }

    fn names(&self, out: &mut Vec<String>) {
        match &self.by {
            OrderBy::Field(f) => out.push(f.clone()),
            OrderBy::Coalesce(v) if v.is_empty() => out.push("(coalesce vide)".into()),
            OrderBy::Coalesce(v) => out.extend(v.iter().cloned()),
        }
    }
}

/// **Au plus tant de lignes** : un nombre, ou un paramètre de l'appel avec
/// son plafond (`{"param": "limit", "max": 200}`), que le liage remplace par
/// un nombre. Un paramètre sans plafond est refusé par la validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Limit {
    Literal(usize),
    Param {
        param: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<usize>,
    },
}

/// **Sauter tant de lignes** : un nombre, ou un paramètre de l'appel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Offset {
    Literal(usize),
    Param { param: String },
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
            Column::Null | Column::Label | Column::Whole | Column::Collect(_) => None,
        }));
        if self.returns.iter().any(|c| matches!(c, Column::Collect(_))) {
            return Err(TranslateError::Invalid("saut : une liste de lignes liées (collect) ne se rend que dans une sélection".into()));
        }
        if let Some(x) = &self.exclude {
            noms.push(&x.field);
            // Une valeur vide s'exclut aussi (`''`, un champ posé sans
            // valeur) : rien à injecter, elle n'a pas à être un identifiant.
            noms.extend(x.values.iter().map(String::as_str).filter(|v| !v.is_empty()));
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Count {
    /// Les lignes d'une table, en une colonne.
    Rows { table: String },
    /// Le degré : pour chaque uuid de départ (le paramètre `$uuids`), le
    /// nombre d'arêtes d'une relation dans un sens. Deux colonnes : l'uuid,
    /// le compte ; un départ sans arête n'a pas de ligne.
    Edges { start: String, relation: String, direction: Direction },
    /// Les lignes qu'une condition retient (toutes sans condition), en une
    /// colonne.
    Matching {
        table: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<Predicate>,
    },
    /// Les arêtes d'une relation qu'une condition sur **leurs** champs
    /// retient, en une colonne (pour le code : les arêtes `resolution =
    /// ambigu`, que l'état de l'index dit).
    EdgesMatching { relation: String, filter: Predicate },
}

impl Count {
    pub fn validate(&self) -> Result<(), TranslateError> {
        let noms: Vec<&str> = match self {
            Count::Rows { table } | Count::Matching { table, filter: None } => vec![table],
            Count::Edges { start, relation, .. } => vec![start, relation],
            Count::Matching { table: nom, filter: Some(filter) } | Count::EdgesMatching { relation: nom, filter } => {
                let mut v = vec![nom.clone()];
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
///
/// **En JSON** (un Plan déclaré), une clé par condition, en `snake_case` :
/// `{"equals": {"field": "f", "param": "p"}}`, `{"all_of": [...]}`,
/// `{"optional": {...}}`, `{"let": "nom"}`… `Compiled` ne se désérialise
/// jamais : un Plan déclaré ne porte pas de langue de base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    /// Le champ vaut le paramètre.
    Equals { field: String, param: String },
    /// Le champ (un texte) contient le paramètre.
    Contains { field: String, param: String },
    /// Le champ vaut au moins le paramètre.
    AtLeast { field: String, param: String },
    /// L'une au moins des conditions.
    AnyOf(Vec<Predicate>),
    /// Toutes les conditions.
    AllOf(Vec<Predicate>),
    /// Le champ vaut l'une des valeurs du paramètre (une liste).
    In { field: String, param: String },
    /// Le champ (un texte) commence par le paramètre.
    StartsWith { field: String, param: String },
    /// **Il existe une ligne liée** : `relation` mène, dans `direction`, à une
    /// ligne de `end` que `filter` retient (sur la ligne atteinte), ou à une
    /// ligne quelconque sans filtre.
    Through {
        relation: String,
        direction: Direction,
        end: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<Box<Predicate>>,
    },
    /// **Une condition facultative** (un Plan déclaré) : retirée au liage si
    /// l'un de ses paramètres est absent, une liste vide ou une chaîne vide.
    /// Un Plan lié n'en porte plus ; la validation la refuse.
    Optional(Box<Predicate>),
    /// **Une condition nommée** du Plan déclaré (`"let"`), remplacée au liage.
    /// Un Plan lié n'en porte plus ; la validation la refuse.
    Let(String),
    /// Une condition déjà dite dans la langue du dialecte, par son propre
    /// parseur de filtres (`FilterParser`), sur l'alias `m` : le filtre d'un
    /// utilisateur, avec ses paramètres. Elle ne se valide pas ici, et ne se
    /// désérialise jamais.
    #[serde(skip)]
    Compiled(String),
}

impl Predicate {
    /// Une condition du Plan déclaré qui n'a pas été liée (`optional`, `let`) :
    /// son nom, pour le refus.
    pub fn unresolved(&self) -> Option<&'static str> {
        match self {
            Predicate::Optional(_) => Some("optional"),
            Predicate::Let(_) => Some("let"),
            Predicate::AnyOf(v) | Predicate::AllOf(v) => v.iter().find_map(Predicate::unresolved),
            Predicate::Through { filter: Some(f), .. } => f.unresolved(),
            _ => None,
        }
    }

    /// Les **paramètres** que la condition lit (pas ses champs) : ce qu'un appel doit lier.
    pub(crate) fn params(&self, out: &mut std::collections::BTreeSet<String>) {
        match self {
            Predicate::Equals { param, .. }
            | Predicate::Contains { param, .. }
            | Predicate::AtLeast { param, .. }
            | Predicate::In { param, .. }
            | Predicate::StartsWith { param, .. } => {
                out.insert(param.clone());
            }
            Predicate::Through { filter: Some(f), .. } => f.params(out),
            Predicate::Optional(p) => p.params(out),
            Predicate::AnyOf(v) | Predicate::AllOf(v) => v.iter().for_each(|p| p.params(out)),
            Predicate::Through { filter: None, .. } | Predicate::Let(_) | Predicate::Compiled(_) => {}
        }
    }

    pub(crate) fn names(&self, out: &mut Vec<String>) {
        match self {
            Predicate::Equals { field, param }
            | Predicate::Contains { field, param }
            | Predicate::AtLeast { field, param }
            | Predicate::In { field, param }
            | Predicate::StartsWith { field, param } => {
                out.push(field.clone());
                out.push(param.clone());
            }
            Predicate::Through { relation, end, filter, .. } => {
                out.push(relation.clone());
                out.push(end.clone());
                if let Some(f) = filter {
                    f.names(out);
                }
            }
            // Non résolues : la validation les refuse ([`Predicate::unresolved`]).
            Predicate::Optional(p) => p.names(out),
            Predicate::Let(_) => {}
            // Une liste vide ne dit rien (`WHERE ` vide, `()`) : elle se refuse
            // comme un nom qui n'est pas un identifiant, avant le moteur.
            Predicate::AnyOf(v) | Predicate::AllOf(v) if v.is_empty() => out.push("(condition vide)".into()),
            Predicate::AnyOf(v) | Predicate::AllOf(v) => v.iter().for_each(|p| p.names(out)),
            Predicate::Compiled(_) => {}
        }
    }
}

/// **Une sélection** dans une table : les lignes de uuids donnés (le
/// paramètre `$uuids`, l'uuid donné en première colonne), ou celles qu'une
/// condition retient ; puis des colonnes, un ordre, une limite, un décalage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Select {
    pub table: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub by_uuids: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Predicate>,
    /// Des champs de la ligne ([`Column::Node`]), des colonnes vides, la
    /// ligne entière, ou des lignes liées ([`Column::Collect`]) ; pas de champ
    /// d'arête ici.
    pub returns: Vec<Column>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_by: Vec<OrderKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Limit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<Offset>,
}

impl Select {
    /// **L'ordre est total** : il finit par `_uuid`. Exigé d'une sélection
    /// paginée d'un Plan déclaré (voir `DeclaredPlan::check`), pas des sondes
    /// internes (`limit 1` pour savoir s'il en existe une).
    pub fn total_order(&self) -> bool {
        self.order_by.last().map(|k| &k.by) == Some(&OrderBy::Field("_uuid".into()))
    }

    /// Les lignes de uuids donnés.
    pub fn by_uuids(table: impl Into<String>, returns: Vec<Column>) -> Self {
        Self { table: table.into(), by_uuids: true, filter: None, returns, order_by: Vec::new(), limit: None, offset: None }
    }

    /// Toutes les lignes de la table.
    pub fn all(table: impl Into<String>, returns: Vec<Column>) -> Self {
        Self { table: table.into(), by_uuids: false, filter: None, returns, order_by: Vec::new(), limit: None, offset: None }
    }

    /// Les lignes qu'une condition retient.
    pub fn filtered(table: impl Into<String>, filter: Predicate, returns: Vec<Column>) -> Self {
        Self { table: table.into(), by_uuids: false, filter: Some(filter), returns, order_by: Vec::new(), limit: None, offset: None }
    }

    pub fn validate(&self) -> Result<(), TranslateError> {
        let mut noms = vec![self.table.clone()];
        for k in &self.order_by {
            k.names(&mut noms);
        }
        if let Some(p) = &self.filter {
            if let Some(nom) = p.unresolved() {
                return Err(TranslateError::Invalid(format!("sélection : une condition « {nom} » n'est pas liée")));
            }
            p.names(&mut noms);
        }
        for c in &self.returns {
            match c {
                Column::Node(f) => noms.push(f.clone()),
                Column::Null | Column::Whole => {}
                Column::Collect(c) => {
                    noms.extend([c.alias.clone(), c.relation.clone(), c.end.clone()]);
                    if c.fields.is_empty() {
                        return Err(TranslateError::Invalid(format!("sélection : la liste « {} » ne nomme aucun champ", c.alias)));
                    }
                    noms.extend(c.fields.iter().cloned());
                    if let Some(f) = &c.filter {
                        if let Some(nom) = f.unresolved() {
                            return Err(TranslateError::Invalid(format!("sélection : une condition « {nom} » n'est pas liée")));
                        }
                        f.names(&mut noms);
                    }
                }
                Column::Edge(_) | Column::Label => {
                    return Err(TranslateError::Invalid("sélection : pas d'arête ni d'étiquette dans une table".into()));
                }
            }
        }
        match &self.limit {
            Some(Limit::Param { max: None, param }) => {
                return Err(TranslateError::Invalid(format!(
                    "sélection : le limit lié au paramètre « {param} » n'a pas de plafond (max)"
                )))
            }
            Some(Limit::Param { param, .. }) => noms.push(param.clone()),
            _ => {}
        }
        if let Some(Offset::Param { param }) = &self.offset {
            noms.push(param.clone());
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
    /// Charger en masse depuis un fichier écrit par l'appelant (le CSV de
    /// l'ingestion). Un dialecte qui ne déclare pas `bulk_load` le refuse en
    /// le nommant, et l'appelant repasse par `Upsert` / `Link`.
    Load { target: LoadTarget, path: String },
    /// **Défaire les arêtes d'un ensemble de nœuds** : les arêtes de
    /// `relation` dont le nœud du côté `side` (`Outgoing` : la source,
    /// `Incoming` : la cible) est une ligne de `table` que `filter` retient
    /// (sur l'alias `m`). Une instruction par relation.
    Detach { relation: String, table: String, side: Direction, filter: Predicate },
}

/// Ce qu'un [`Write::Load`] remplit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadTarget {
    /// Des lignes, colonnes dans l'ordre du fichier.
    Nodes { table: String, columns: Vec<String> },
    /// Des arêtes : les deux bouts d'abord, puis les propriétés.
    Links { relation: String, ends: (String, String), props: Vec<String> },
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
            // Le chemin du fichier n'entre pas comme un nom : il est cité.
            Write::Load { target: LoadTarget::Nodes { table, columns }, .. } => {
                noms.push(table);
                noms.extend(columns.iter().map(String::as_str));
            }
            Write::Load { target: LoadTarget::Links { relation, ends, props }, .. } => {
                noms.extend([relation.as_str(), ends.0.as_str(), ends.1.as_str()]);
                noms.extend(props.iter().map(String::as_str));
            }
            Write::Detach { relation, table, filter, .. } => {
                let mut du_filtre = Vec::new();
                filter.names(&mut du_filtre);
                let mut tous: Vec<&str> = vec![relation, table];
                tous.extend(du_filtre.iter().map(String::as_str));
                return match tous.into_iter().find(|n| !crate::is_valid_identifier(n)) {
                    Some(n) => Err(TranslateError::Invalid(format!("écriture : « {n} » n'est pas un identifiant"))),
                    None => Ok(()),
                };
            }
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
