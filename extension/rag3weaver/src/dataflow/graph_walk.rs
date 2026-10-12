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

    /// Ce sens dans le langage intermédiaire.
    pub fn ir(self) -> rag3weaver_ir::Direction {
        match self {
            Direction::Incoming => rag3weaver_ir::Direction::Incoming,
            Direction::Outgoing => rag3weaver_ir::Direction::Outgoing,
        }
    }

    /// Le motif `(d:…)…(m:…)` de ce sens ; `r` nomme l'arête. Les noms sont
    /// cités comme le dialecte rag3db les cite ([`crate::dialect::cypher_quote`]),
    /// pour que la parité compare le même texte.
    pub fn pattern(self, rel: &RelInfo) -> String {
        let q = crate::dialect::cypher_quote;
        match self {
            Direction::Incoming => format!("(d:{} {{_uuid: u}})<-[r:{}]-(m:{})", q(&rel.to), q(&rel.name), q(&rel.from)),
            Direction::Outgoing => format!("(d:{} {{_uuid: u}})-[r:{}]->(m:{})", q(&rel.from), q(&rel.name), q(&rel.to)),
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
        format!(" WHERE r.{f} IS NULL OR NOT r.{f} IN [{valeurs}]", f = crate::dialect::cypher_quote(&self.field))
    }

    /// La colonne à relire sur l'arête `r` (`NULL` si la relation ne la porte pas).
    pub fn column(&self, rel: &RelInfo) -> String {
        if self.applies(rel) { format!("r.{}", crate::dialect::cypher_quote(&self.field)) } else { "NULL".into() }
    }

    /// [`Self::column`] dans le langage intermédiaire.
    pub fn column_ir(&self, rel: &RelInfo) -> rag3weaver_ir::Column {
        if self.applies(rel) { rag3weaver_ir::Column::Edge(self.field.clone()) } else { rag3weaver_ir::Column::Null }
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

/// Le degré, dans un sens, d'une liste de nœuds, en forme du langage
/// intermédiaire.
pub fn degree_count(rel: &RelInfo, direction: Direction) -> rag3weaver_ir::Count {
    let (start, _) = direction.tables(rel);
    rag3weaver_ir::Count::Edges { start: start.into(), relation: rel.name.clone(), direction: direction.ir() }
}

/// La somme des degrés de chaque nœud, par ces relations, dans ces sens.
pub fn degrees(catalog: &Catalog, rels: &[RelInfo], directions: &[Direction], uuids: &[String]) -> Result<HashMap<String, usize>, String> {
    let mut out: HashMap<String, usize> = HashMap::new();
    if uuids.is_empty() {
        return Ok(out);
    }
    // Un éventail de degrés sur la même liste : une instruction quand le
    // dialecte sait le fondre (`crate::plan`), une par degré sinon.
    let liste = uuid_param(uuids);
    let mut plan = rag3weaver_ir::Plan::new();
    let mut k = 0usize;
    for rel in rels {
        for &direction in directions {
            let id = format!("d{k}");
            plan = plan.node(&id, rag3weaver_ir::Form::Count(degree_count(rel, direction)), vec![liste.clone()]).output(&id);
            k += 1;
        }
    }
    let options = crate::plan::PlanOptions { no_transaction: true, ..Default::default() };
    let rapport = catalog.run_read_plan(&plan, options).map_err(|e| e.to_string())?;
    for rows in rapport.outputs.values() {
        for r in &rows.rows {
            *out.entry(text(r.first())).or_default() += r.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        }
    }
    Ok(out)
}

/// **Un saut, dit par le dialecte, en une instruction ou en deux.** Quand le
/// dialecte le déclare (`DialectCapabilities::two_step_hops`) et que le saut
/// lit des champs du nœud atteint d'une table connue, sans filtre, ordre ni
/// limite : d'abord le saut, qui ne rend que l'uuid du voisin (les champs à
/// leur place, vides), puis ces champs par clé (`Select::by_uuids`), remis
/// à leur place ligne par ligne. Les lignes sont celles du saut en une
/// instruction, dans le même ordre de colonnes ; leur ordre entre elles
/// n'est promis ni par l'un ni par l'autre. Sinon, le saut tel quel.
pub fn run_hop(catalog: &Catalog, hop: &rag3weaver_ir::Hop, params: &[crate::connection::QueryParam]) -> Result<crate::connection::QueryResult, String> {
    use rag3weaver_ir::Column;
    let dialect = catalog.dialect_arc();
    let champ = |c: &Column| match c {
        Column::Node(f) if f != "_uuid" => Some(f.clone()),
        _ => None,
    };
    let deux_temps = catalog.two_step_hops()
        && hop.end.is_some()
        && hop.filter.is_none()
        && hop.order_by.is_empty()
        && hop.limit.is_none()
        && hop.returns.iter().any(|c| champ(c).is_some())
        && !hop.returns.iter().any(|c| matches!(c, Column::Whole));
    if !deux_temps {
        let q = dialect.hop(hop).map_err(|e| e.to_string())?;
        return catalog.execute_read_with_params(&q, params).map_err(|e| e.to_string());
    }
    // 1. Le saut : l'uuid du voisin en plus, ses champs vides à leur place.
    let mut premier = hop.clone();
    premier.returns = std::iter::once(Column::Node("_uuid".into()))
        .chain(hop.returns.iter().map(|c| if champ(c).is_some() { Column::Null } else { c.clone() }))
        .collect();
    let q = dialect.hop(&premier).map_err(|e| e.to_string())?;
    let sauts = catalog.execute_read_with_params(&q, params).map_err(|e| e.to_string())?;
    // 2. Les champs, par clé.
    let mut champs: Vec<String> = Vec::new();
    for c in &hop.returns {
        if let Some(f) = champ(c) {
            if !champs.contains(&f) {
                champs.push(f);
            }
        }
    }
    let mut vus = std::collections::BTreeSet::new();
    let voisins: Vec<CypherValue> = sauts
        .rows
        .iter()
        .filter_map(|r| r.get(1).filter(|v| !v.is_null()).cloned())
        .filter(|v| vus.insert(format!("{v:?}")))
        .collect();
    let mut lus: HashMap<String, Vec<CypherValue>> = HashMap::new();
    // Les noms des champs, tels que le dialecte les nomme (même alias `m` que
    // le saut) : les deux formes ont le même en-tête.
    let mut noms_lus: Vec<String> = champs.clone();
    if !voisins.is_empty() {
        let table = hop.end.clone().expect("vérifié plus haut");
        let lire = rag3weaver_ir::Select::by_uuids(table, champs.iter().map(|f| Column::Node(f.clone())).collect());
        let q = dialect.select(&lire).map_err(|e| e.to_string())?;
        let r = catalog
            .execute_read_with_params(&q, &[crate::connection::QueryParam::new("uuids", CypherValue::List(voisins))])
            .map_err(|e| e.to_string())?;
        if r.columns.len() == champs.len() + 1 {
            noms_lus = r.columns[1..].to_vec();
        }
        for l in r.rows {
            let u = text(l.first());
            lus.insert(u, l.into_iter().skip(1).collect());
        }
    }
    // 3. Les lignes du saut en une instruction : le départ, puis chaque
    // colonne à sa place.
    let mut colonnes = vec![sauts.columns.first().cloned().unwrap_or_default()];
    for (i, c) in hop.returns.iter().enumerate() {
        colonnes.push(match champ(c) {
            Some(f) => noms_lus[champs.iter().position(|x| *x == f).expect("champ relevé")].clone(),
            None => sauts.columns.get(i + 2).cloned().unwrap_or_default(),
        });
    }
    // Un voisin rendu par le saut puis supprimé avant la lecture par clé
    // (une réingestion concurrente, en mode multi-écrivains) : sa ligne est
    // écartée, comme si le saut avait lu après la suppression ; la forme en un
    // temps ne l'aurait jamais rendue à moitié. Le compte se dit.
    let disparus = sauts.rows.iter().filter(|r| !lus.contains_key(&text(r.get(1)))).count();
    if disparus > 0 {
        catalog.emit_event(crate::events::CatalogEvent::Warning {
            context: "saut en deux temps".to_string(),
            message: format!("{disparus} ligne(s) écartée(s), voisin supprimé entre le saut et la lecture par clé"),
        });
    }
    let rows = sauts
        .rows
        .iter()
        .filter_map(|r| {
            let voisin = text(r.get(1));
            let ligne_lue = lus.get(&voisin)?;
            let mut ligne = vec![r.first().cloned().unwrap_or(CypherValue::Null)];
            for (i, c) in hop.returns.iter().enumerate() {
                ligne.push(match champ(c) {
                    Some(f) => {
                        let k = champs.iter().position(|x| *x == f).expect("champ relevé");
                        ligne_lue.get(k).cloned().unwrap_or(CypherValue::Null)
                    }
                    None => r.get(i + 2).cloned().unwrap_or(CypherValue::Null),
                });
            }
            Some(ligne)
        })
        .collect();
    Ok(crate::connection::QueryResult { columns: colonnes, rows })
}

/// Le saut nu de [`neighbors`], en forme du langage intermédiaire : les
/// arêtes devinées sont écartées quand la relation porte le champ déclaré.
pub fn hop_of(rel: &RelInfo, direction: Direction, mark: &EdgeMark) -> rag3weaver_ir::Hop {
    let (start, end) = direction.tables(rel);
    let mut hop = rag3weaver_ir::Hop::new(start, &rel.name, end, direction.ir());
    if mark.applies(rel) {
        hop.exclude = Some(rag3weaver_ir::EdgeExclusion { field: mark.field.clone(), values: mark.guessed.clone() });
    }
    hop
}

/// Un saut nu : les paires (départ, atteint) par une relation, dans un sens.
/// La requête vient du dialecte ([`rag3weaver_ir::Hop`]).
pub fn neighbors(catalog: &Catalog, rel: &RelInfo, direction: Direction, mark: &EdgeMark, uuids: &[String]) -> Result<Vec<(String, String)>, String> {
    if uuids.is_empty() {
        return Ok(Vec::new());
    }
    let q = catalog.dialect_arc().hop(&hop_of(rel, direction, mark)).map_err(|e| e.to_string())?;
    let rows = catalog.execute_read_with_params(&q, &[uuid_param(uuids)]).map_err(|e| e.to_string())?;
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
        assert_eq!(m.clause(&rel(&["usage", "resolution"])), " WHERE r.`resolution` IS NULL OR NOT r.`resolution` IN ['nom']");
        assert_eq!(m.column(&rel(&["resolution"])), "r.`resolution`");
        assert!(m.is_guessed("nom") && !m.is_guessed("type") && !m.is_guessed(""));
        // Une relation sans le champ n'est pas touchée.
        assert_eq!(m.clause(&rel(&["usage"])), "");
        assert_eq!(m.column(&rel(&[])), "NULL");
    }

    /// Parité : le saut traduit par le dialecte rag3db est, au caractère
    /// près, la requête que `neighbors` écrivait à la main.
    #[test]
    fn le_saut_du_dialecte_est_la_requete_d_avant() {
        use crate::dialect::{Rag3dbDialect, SchemaDialect};
        let m = EdgeMark::from_config(&serde_json::json!({"edge_field": "resolution", "edge_guessed": "nom|import"}), "N").unwrap();
        let distincte = RelInfo { name: "DEFINES".into(), from: "File".into(), to: "Symbol".into(), props: vec!["resolution".into()] };
        for r in [rel(&["usage", "resolution"]), rel(&["usage"]), distincte] {
            for d in [Direction::Incoming, Direction::Outgoing] {
                let avant = format!("UNWIND $uuids AS u MATCH {}{} RETURN u, m.`_uuid`", d.pattern(&r), m.clause(&r));
                assert_eq!(Rag3dbDialect.hop(&hop_of(&r, d, &m)).unwrap(), avant);
            }
        }
    }

    /// Parité : le degré traduit par rag3db est la requête d'avant.
    #[test]
    fn le_degre_du_dialecte_est_la_requete_d_avant() {
        use crate::dialect::{Rag3dbDialect, SchemaDialect};
        let r = RelInfo { name: "DEFINES".into(), from: "File".into(), to: "Symbol".into(), props: vec![] };
        assert_eq!(
            Rag3dbDialect.count(&degree_count(&r, Direction::Incoming)).unwrap(),
            "UNWIND $uuids AS u MATCH (d:`Symbol` {_uuid: u})<-[r:`DEFINES`]-() RETURN u, count(r)"
        );
        assert_eq!(
            Rag3dbDialect.count(&degree_count(&r, Direction::Outgoing)).unwrap(),
            "UNWIND $uuids AS u MATCH (d:`File` {_uuid: u})-[r:`DEFINES`]->() RETURN u, count(r)"
        );
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
