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

/// **La marque d'une arête devinée**, déclarée au gabarit : le champ de
/// l'arête (`edge_field`) et les valeurs qui disent « devinée »
/// (`edge_guessed`, séparées par |) — pour le code, `resolution` et `nom` :
/// une arête que le rendez-vous a posée par le seul nom. Une marche ne suit
/// pas ces arêtes ; `usages` les montre en le disant. Une arête où le champ
/// est nul (une base d'avant la colonne) est **inconnue, traitée comme
/// sûre** ; une relation qui n'a pas le champ n'est pas touchée. Sans
/// déclaration, rien ne change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EdgeMark {
    pub field: String,
    pub guessed: Vec<String>,
}

impl EdgeMark {
    /// Lue de `edge_field` et `edge_guessed`. Champ et valeurs entrent dans
    /// le texte des requêtes : des identifiants seulement.
    pub fn from_config(config: &serde_json::Value, noeud: &str) -> Result<Self, String> {
        let field = config.get("edge_field").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let guessed: Vec<String> = config
            .get("edge_guessed")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split(['|', ','])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let ident = |x: &str| x.chars().all(|c| c.is_alphanumeric() || c == '_');
        if let Some(x) = std::iter::once(&field).chain(&guessed).find(|x| !ident(x)) {
            return Err(format!("{noeud}: « {x} » n'est pas un identifiant"));
        }
        if field.is_empty() != guessed.is_empty() {
            return Err(format!("{noeud}: « edge_field » et « edge_guessed » vont ensemble"));
        }
        Ok(EdgeMark { field, guessed })
    }

    /// La relation porte-t-elle le champ déclaré ?
    fn applies(&self, rel: &RelInfo) -> bool {
        !self.field.is_empty() && rel.props.iter().any(|p| p == &self.field)
    }

    /// ` WHERE …` qui écarte les arêtes `r` devinées, ou rien.
    pub fn clause(&self, rel: &RelInfo) -> String {
        if !self.applies(rel) {
            return String::new();
        }
        let valeurs = self.guessed.iter().map(|v| format!("'{v}'")).collect::<Vec<_>>().join(", ");
        format!(" WHERE r.{f} IS NULL OR NOT r.{f} IN [{valeurs}]", f = self.field)
    }

    /// La colonne à relire sur l'arête `r` (`NULL` si la relation ne la porte pas).
    pub fn column(&self, rel: &RelInfo) -> String {
        if self.applies(rel) { format!("r.{}", self.field) } else { "NULL".into() }
    }

    pub fn is_guessed(&self, valeur: &str) -> bool {
        self.guessed.iter().any(|g| g == valeur)
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
pub fn neighbors(catalog: &Catalog, rel: &RelInfo, direction: Direction, mark: &EdgeMark, uuids: &[String]) -> Result<Vec<(String, String)>, String> {
    if uuids.is_empty() {
        return Ok(Vec::new());
    }
    let q = format!("UNWIND $uuids AS u MATCH {}{} RETURN u, m._uuid", direction.pattern(rel), mark.clause(rel));
    let rows = catalog.execute_raw_with_params(&q, &[uuid_param(uuids)]).map_err(|e| e.to_string())?;
    Ok(rows.rows.iter().map(|r| (text(r.first()), text(r.get(1)))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(props: &[&str]) -> RelInfo {
        RelInfo { name: "CONSUMES".into(), from: "Scope".into(), to: "Scope".into(), props: props.iter().map(|p| p.to_string()).collect() }
    }

    #[test]
    fn une_arete_devinee_n_est_pas_suivie_une_inconnue_l_est() {
        let m = EdgeMark::from_config(&serde_json::json!({"edge_field": "resolution", "edge_guessed": "nom"}), "N").unwrap();
        assert_eq!(m.clause(&rel(&["usage", "resolution"])), " WHERE r.resolution IS NULL OR NOT r.resolution IN ['nom']");
        assert_eq!(m.column(&rel(&["resolution"])), "r.resolution");
        assert!(m.is_guessed("nom") && !m.is_guessed("type") && !m.is_guessed(""));
        // Une relation sans le champ n'est pas touchée.
        assert_eq!(m.clause(&rel(&["usage"])), "");
        assert_eq!(m.column(&rel(&[])), "NULL");
    }

    #[test]
    fn sans_declaration_rien_ne_change() {
        let m = EdgeMark::from_config(&serde_json::json!({}), "N").unwrap();
        assert_eq!(m, EdgeMark::default());
        assert_eq!(m.clause(&rel(&["resolution"])), "");
    }

    #[test]
    fn la_declaration_est_verifiee() {
        let j = |f: &str, g: &str| EdgeMark::from_config(&serde_json::json!({"edge_field": f, "edge_guessed": g}), "N");
        assert!(j("resolution", "nom') OR true //").is_err(), "une valeur entre dans le texte de la requête");
        assert!(j("resolution", "").is_err());
        assert!(j("", "nom").is_err());
        assert_eq!(j("resolution", "nom|import").unwrap().guessed, vec!["nom".to_string(), "import".to_string()]);
    }
}
