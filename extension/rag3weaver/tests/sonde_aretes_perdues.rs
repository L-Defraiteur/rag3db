//! **Les arêtes perdues d'un repassage** (15 novembre 2026, demandée par c7 après le juge étendu de banc :
//! CONSUMES −4 080, MENTIONS −984 sur la base migrée du dépôt). Compare deux **copies** d'une même base de
//! code, le parent (`RAG3WEAVER_SONDE_PARENT`) et le lot (`RAG3WEAVER_SONDE_LOT`), jamais une base en
//! service : les arêtes entre lignes ouvertes (`_jusqu_a` nul aux deux bouts), par la clé des bouts, celles
//! du parent qui manquent au lot, leur répartition, et dix exemples avec l'état des deux bouts au lot.
//! Mesure sans échouer.
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::Rag3dbConnection;
use std::collections::{BTreeMap, BTreeSet};

fn t(v: &CypherValue) -> String {
    match v {
        CypherValue::String(s) => s.clone(),
        CypherValue::Null => "∅".into(),
        autre => format!("{autre:?}").trim_start_matches("Int(").trim_end_matches(')').to_string(),
    }
}

fn lignes(c: &Rag3dbConnection, q: &str) -> Vec<Vec<String>> {
    c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}")).rows.iter().map(|l| l.iter().map(t).collect()).collect()
}

/// Les arêtes ouvertes d'une relation, par (clé du bout a, clé du bout b).
fn aretes(c: &Rag3dbConnection, rel: &str, vers: &str, cle_b: &str) -> BTreeSet<(String, String)> {
    let q = format!(
        "MATCH (a:Scope)-[:{rel}]->(b:{vers}) WHERE a._jusqu_a IS NULL{} RETURN a.key, b.{cle_b}",
        if vers == "Scope" { " AND b._jusqu_a IS NULL" } else { "" }
    );
    lignes(c, &q).into_iter().map(|l| (l[0].clone(), l[1].clone())).collect()
}

/// L'état d'une clé de scope : toutes ses lignes, (fichier, nom, début, `_depuis`, `_jusqu_a`).
fn etat_scope(c: &Rag3dbConnection, cle: &str) -> String {
    let r = lignes(c, &format!("MATCH (s:Scope) WHERE s.key = '{}' RETURN s.file_path, s.name, s.start_line, s._depuis, s._jusqu_a ORDER BY s._depuis", cle.replace('\'', "\\'")));
    r.iter().map(|l| format!("{}|{}|l{}|[{},{})", l[0], l[1], l[2], l[3], l[4])).collect::<Vec<_>>().join(" ; ")
}

#[test]
#[ignore]
fn les_aretes_perdues_d_un_repassage() {
    let (Some(parent), Some(lot)) = (std::env::var_os("RAG3WEAVER_SONDE_PARENT"), std::env::var_os("RAG3WEAVER_SONDE_LOT")) else {
        eprintln!("SAUTÉ : RAG3WEAVER_SONDE_PARENT et RAG3WEAVER_SONDE_LOT (deux copies) manquent");
        return;
    };
    let p = Rag3dbConnection::new(&parent).expect("ouvrir la copie du parent");
    let l = Rag3dbConnection::new(&lot).expect("ouvrir la copie du lot");
    for (rel, vers, cle_b) in [("CONSUMES", "Scope", "key"), ("MENTIONS", "Symbol", "name")] {
        let (ap, al) = (aretes(&p, rel, vers, cle_b), aretes(&l, rel, vers, cle_b));
        let perdues: Vec<_> = ap.difference(&al).cloned().collect();
        let gagnees = al.difference(&ap).count();
        eprintln!("[sonde] {rel} : parent {}, lot {}, perdues {}, gagnées {gagnees}", ap.len(), al.len(), perdues.len());
        // La répartition : le bout a au lot (ouvert né à 0, ouvert né après, absent ouvert), et b de même.
        let depuis_ouvert = |c: &Rag3dbConnection, table: &str, champ: &str, cle: &str| -> String {
            let r = lignes(c, &format!("MATCH (s:{table}) WHERE s.{champ} = '{}' AND s._jusqu_a IS NULL RETURN s._depuis", cle.replace('\'', "\\'")));
            match r.len() { 0 => "pas d'ouverte".into(), 1 => format!("ouverte née à {}", r[0][0]), n => format!("{n} ouvertes") }
        };
        let mut repartition: BTreeMap<(String, String), usize> = BTreeMap::new();
        for (a, b) in perdues.iter().take(2000) {
            let ea = depuis_ouvert(&l, "Scope", "key", a);
            let eb = if vers == "Scope" { depuis_ouvert(&l, "Scope", "key", b) } else { "symbole".into() };
            *repartition.entry((ea, eb)).or_default() += 1;
        }
        for ((ea, eb), n) in &repartition {
            eprintln!("[sonde]   {rel} perdue : a {ea}, b {eb} : {n}");
        }
        for (a, b) in perdues.iter().take(10) {
            eprintln!("[sonde]   exemple {rel}");
            eprintln!("[sonde]     a parent : {}", etat_scope(&p, a));
            eprintln!("[sonde]     a lot    : {}", etat_scope(&l, a));
            if vers == "Scope" {
                eprintln!("[sonde]     b parent : {}", etat_scope(&p, b));
                eprintln!("[sonde]     b lot    : {}", etat_scope(&l, b));
                // Au lot, les arêtes de a (toutes ses lignes) vers une ligne de b (toutes) : fermées ou ouvertes.
                let r = lignes(&l, &format!(
                    "MATCH (x:Scope)-[:{rel}]->(y:Scope) WHERE x.key = '{}' AND y.key = '{}' RETURN x._depuis, x._jusqu_a, y._depuis, y._jusqu_a",
                    a.replace('\'', "\\'"), b.replace('\'', "\\'")
                ));
                eprintln!("[sonde]     arêtes a→b au lot, toutes lignes : {r:?}");
                // Au lot, b a-t-il un homonyme ouvert (la résolution par le nom a pu changer de cible) ?
                let nom = lignes(&p, &format!("MATCH (s:Scope) WHERE s.key = '{}' RETURN s.name LIMIT 1", b.replace('\'', "\\'")));
                if let Some(n) = nom.first() {
                    let h = lignes(&l, &format!("MATCH (s:Scope) WHERE s.name = '{}' AND s._jusqu_a IS NULL RETURN count(*)", n[0].replace('\'', "\\'")));
                    let vers_homonyme = lignes(&l, &format!(
                        "MATCH (x:Scope)-[:{rel}]->(y:Scope) WHERE x.key = '{}' AND x._jusqu_a IS NULL AND y.name = '{}' RETURN y.file_path, y._depuis, y._jusqu_a",
                        a.replace('\'', "\\'"), n[0].replace('\'', "\\'")
                    ));
                    let mp = lignes(&p, &format!("MATCH (s:Scope)-[:MENTIONS]->(m:Symbol) WHERE s.key = '{}' AND s._jusqu_a IS NULL AND m.name = '{}' RETURN count(*)", a.replace('\'', "\\'"), n[0].replace('\'', "\\'")));
                    let ml = lignes(&l, &format!("MATCH (s:Scope)-[:MENTIONS]->(m:Symbol) WHERE s.key = '{}' AND s._jusqu_a IS NULL AND m.name = '{}' RETURN count(*)", a.replace('\'', "\\'"), n[0].replace('\'', "\\'")));
                    eprintln!("[sonde]     nom « {} » : {} ouvertes de ce nom au lot ; a ouvert → ce nom au lot : {vers_homonyme:?} ; MENTIONS de a vers ce nom : parent {:?}, lot {:?}", n[0], h[0][0], mp, ml);
                }
            } else {
                let r = lignes(&l, &format!("MATCH (s:Symbol) WHERE s.name = '{}' RETURN count(*)", b.replace('\'', "\\'")));
                eprintln!("[sonde]     symbole « {b} » : {} au lot", r[0][0]);
            }
        }
    }
}
