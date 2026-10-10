//! **Le langage intermédiaire de rag3weaver** : ce qui passe d'une couche à
//! l'autre sans dire à quelle base on parle.
//!
//! Aujourd'hui : les valeurs et les paramètres ([`Value`], [`QueryParam`]),
//! le type déclaré d'un champ ([`FieldType`]), l'arbre d'un filtre
//! ([`FilterCondition`]) et la cellule ([`Scope`]). Les formes de requêtes
//! (`Select`, `Count`, `Hop`, `Write`, `Tx`) viendront ici, chacune traduite
//! par un dialecte ; voir
//! `docs/8-octobre-2026-16h29/embarquements/02-le-cypher-au-dessus-des-dialectes.md`.
//!
//! Aucun mot d'une base ici : rag3weaver réexporte ces types sous leurs
//! anciens noms (`connection::CypherValue`, …) le temps que les sites
//! d'usage se renomment, par touches.

mod filter;
mod scope;
mod value;

pub use filter::{FilterCondition, FilterOp, FilterValue};
pub use scope::{Scope, DEFAULT_ID, ORG_COLUMN, PROJECT_COLUMN};
pub use value::{is_valid_identifier, normalize, validate_payload_type, FieldType, QueryParam, Value};
