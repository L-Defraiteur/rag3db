//! **Marcher dans le graphe par lots** : la pièce commune du voisinage
//! ([`super::neighborhood_nodes`]) et des liens ([`super::links_nodes`]).
//!
//! Un saut part d'une liste d'uuid, suit une relation dans un sens, et rend
//! les paires (départ, arrivée) : `UNWIND $uuids AS u MATCH (d:T {_uuid: u})
//! -[r:REL]->(m:S)`, une liste de valeurs simples jointe par hachage
//! (journal, §6) — jamais `item.champ`, qui fait un produit cartésien. Le
//! degré se compte de la même façon. Ce que l'appelant fait entre deux sauts
//! (dédoublonner, plafonner, couper au budget) reste à lui : c'est ce qu'un
//! chemin de longueur variable (`*1..4`) ne laisserait pas faire.

use std::collections::HashMap;

use super::usage_nodes::RelInfo;
use crate::catalog::Catalog;
use crate::connection::{CypherValue, QueryParam};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `(d)<-[r]-(m)` : ce qui pointe vers le nœud (qui dépend de lui).
    Incoming,
    /// `(d)-[r]->(m)` : ce vers quoi il pointe (ce dont il dépend).
    Outgoing,
}

impl Direction {
    /// La table du nœud de départ, et celle du nœud atteint.
    pub fn tables(self, rel: &RelInfo) -> (&str, &str) {
        match self {
            Direction::Incoming => (&rel.to, &rel.from),
            Direction::Outgoing => (&rel.from, &rel.to),
        }
    }

    /// Le motif `(d:…)…(m:…)` de ce sens ; `r` nomme l'arête.
    pub fn pattern(self, rel: &RelInfo) -> String {
        match self {
            Direction::Incoming => format!("(d:{} {{_uuid: u}})<-[r:{}]-(m:{})", rel.to, rel.name, rel.from),
            Direction::Outgoing => format!("(d:{} {{_uuid: u}})-[r:{}]->(m:{})", rel.from, rel.name, rel.to),
        }
    }
}

/// Le paramètre `$uuids` d'une requête par lot.
pub fn uuid_param(uuids: &[String]) -> QueryParam {
    QueryParam::new("uuids", CypherValue::List(uuids.iter().map(|u| CypherValue::String(u.clone())).collect()))
}

pub fn text(v: Option<&CypherValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Le degré, dans un sens, d'une liste de nœuds.
pub fn degree_query(rel: &RelInfo, direction: Direction) -> String {
    match direction {
        Direction::Incoming => format!("UNWIND $uuids AS u MATCH (d:{} {{_uuid: u}})<-[r:{}]-() RETURN u, count(r)", rel.to, rel.name),
        Direction::Outgoing => format!("UNWIND $uuids AS u MATCH (d:{} {{_uuid: u}})-[r:{}]->() RETURN u, count(r)", rel.from, rel.name),
    }
}

/// La somme des degrés de chaque nœud, par ces relations, dans ces sens.
pub fn degrees(catalog: &Catalog, rels: &[RelInfo], directions: &[Direction], uuids: &[String]) -> Result<HashMap<String, usize>, String> {
    let mut out: HashMap<String, usize> = HashMap::new();
    if uuids.is_empty() {
        return Ok(out);
    }
    let liste = uuid_param(uuids);
    for rel in rels {
        for &direction in directions {
            let rows = catalog.execute_raw_with_params(&degree_query(rel, direction), std::slice::from_ref(&liste)).map_err(|e| e.to_string())?;
            for r in &rows.rows {
                *out.entry(text(r.first())).or_default() += r.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            }
        }
    }
    Ok(out)
}

/// Un saut nu : les paires (départ, atteint) par une relation, dans un sens.
pub fn neighbors(catalog: &Catalog, rel: &RelInfo, direction: Direction, uuids: &[String]) -> Result<Vec<(String, String)>, String> {
    if uuids.is_empty() {
        return Ok(Vec::new());
    }
    let q = format!("UNWIND $uuids AS u MATCH {} RETURN u, m._uuid", direction.pattern(rel));
    let rows = catalog.execute_raw_with_params(&q, &[uuid_param(uuids)]).map_err(|e| e.to_string())?;
    Ok(rows.rows.iter().map(|r| (text(r.first()), text(r.get(1)))).collect())
}
