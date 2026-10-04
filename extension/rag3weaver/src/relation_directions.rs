//! **Les deux sens d'une relation** (4 octobre 2026).
//!
//! Le moteur range chaque relation deux fois : depuis son nœud de départ
//! (sens direct) et depuis son nœud d'arrivée (sens inverse). Un défaut du
//! moteur, trouvé par la session du banc, effaçait au point de reprise les
//! relations d'autres régions d'un groupe de nœuds **dans le sens direct
//! seulement**, après la suppression de quelques-unes. Une base atteinte lit
//! donc moins d'arêtes depuis le départ que depuis l'arrivée.
//!
//! Cette sonde compte, par table de relation et par couple de tables reliées,
//! les arêtes lues dans chaque sens. Deux comptes différents disent qu'une base
//! a perdu des arêtes ; égaux, ils ne prouvent rien de plus que leur égalité.

use crate::connection::{CypherValue, DbConnection};

/// Les comptes d'une table de relation, pour un couple (départ, arrivée).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectionCount {
    pub table: String,
    pub from: String,
    pub to: String,
    pub forward: i64,
    pub backward: i64,
}

impl DirectionCount {
    pub fn is_symmetric(&self) -> bool {
        self.forward == self.backward
    }
}

fn texte(v: Option<&CypherValue>) -> Option<String> {
    v.and_then(|v| v.as_str()).map(str::to_string)
}

fn compte(conn: &dyn DbConnection, requete: &str) -> Result<i64, String> {
    conn.execute(requete)
        .map_err(|e| format!("{requete} : {e}"))?
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("{requete} : aucun compte"))
}

/// **Compter chaque table de relation dans les deux sens.** Le moteur lit le
/// sens qui part d'un nœud **déjà lié** : chaque nœud de départ est lié par un
/// `WITH`, puis on étend depuis lui (sens direct, rangé par nœud d'origine) ;
/// de même pour chaque nœud d'arrivée (sens inverse). Écrire la flèche ne
/// suffit pas : sans nœud lié, le planificateur choisit le sens, et la
/// première version de cette sonde lisait le même sens deux fois sans le
/// savoir (session cœur C++, 4 octobre 2026).
///
/// **Étalonnée sur la recette du banc, avant le correctif du moteur** : elle
/// voit la perte, (2, 2) → (1, 0). Mais l'étalon par clé place la perte dans
/// le sens direct, quand la sonde la range du côté inverse : même avec le
/// `WITH`, le planificateur peut choisir son sens. **La dissymétrie est sûre,
/// l'étiquette du sens ne l'est pas** ; pour savoir quel sens est touché,
/// lier un nœud par sa clé primaire.
pub fn count_both_directions(conn: &dyn DbConnection) -> Result<Vec<DirectionCount>, String> {
    let tables = conn.execute("CALL show_tables() RETURN *").map_err(|e| format!("show_tables : {e}"))?;
    let mut noms: Vec<String> = tables
        .rows
        .iter()
        .filter(|r| r.iter().any(|v| v.as_str() == Some("REL")))
        .filter_map(|r| texte(r.get(1)))
        .collect();
    noms.sort();
    let mut out = Vec::new();
    for table in noms {
        let couples = conn
            .execute(&format!("CALL show_connection('{table}') RETURN *"))
            .map_err(|e| format!("show_connection({table}) : {e}"))?;
        for r in &couples.rows {
            let (Some(from), Some(to)) = (texte(r.first()), texte(r.get(1))) else { continue };
            let forward = compte(conn, &format!("MATCH (a:{from}) WITH a MATCH (a)-[r:{table}]->(b:{to}) RETURN count(r)"))?;
            let backward = compte(conn, &format!("MATCH (b:{to}) WITH b MATCH (b)<-[r:{table}]-(a:{from}) RETURN count(r)"))?;
            out.push(DirectionCount { table: table.clone(), from, to, forward, backward });
        }
    }
    Ok(out)
}
