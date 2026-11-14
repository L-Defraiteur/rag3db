//! **L'intérieur du nœud lexical sur une base à versions** (2a′-2 ; la mesure par étage du 14 novembre
//! 2026 donne `BM25SearchNode` à 6 928 ms sans filtre, contre 874 ms avant les versions). Page
//! `docs/everything-declarative/14-11-2026/04-le-pas-2a-prime-2-l-ancre-immuable.md`, § 2.
//!
//! Le nœud fait trois choses avant de chercher. La sonde les rejoue une à une, sur une **copie**
//! (`RAG3WEAVER_SONDE_BASE`, `RAG3WEAVER_SONDE_MANIFESTE`), par les mêmes fonctions :
//! 1. **les racines** : sans filtre, tous les `_uuid` de `Scope` (`select_all`, le repli
//!    d'`occurrence_roots_for`) ;
//! 2. **le masque** : `derived_references` par lots de 5 000 racines, les versions admises ;
//! 3. **la résolution du filtre `_uuid IN [versions]`** en offsets lucivy (`resolve_filter_to_ids`
//!    sur la table parente, ce que fait `allowed_ids_for`).
//! Le reste du temps du nœud est la recherche lucivy elle-même.
//!
//! Avec `RAG3WEAVER_SONDE_ROLE=reference`, la sonde dit seulement l'état des index de `Scope` (pour le
//! dense à 0 ms de la référence). `RAG3WEAVER_SONDE_CHAUFFE` (1) et `RAG3WEAVER_SONDE_TOURS` (5).

use rag3weaver::backend::PreparedBackend;
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::filter::{FilterCondition, FilterValue};
use rag3weaver::Rag3dbConnection;
use std::time::Instant;

fn env_n(nom: &str, defaut: usize) -> usize {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

fn rss_mio() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|k| k.parse::<f64>().ok()))
        .map_or(f64::NAN, |k| k / 1024.0)
}

fn chrono<T>(chauffe: usize, tours: usize, mut f: impl FnMut() -> T) -> (f64, f64, T) {
    for _ in 0..chauffe {
        f();
    }
    let mut t = Vec::new();
    let mut dernier = None;
    for _ in 0..tours {
        let d = Instant::now();
        dernier = Some(f());
        t.push(d.elapsed().as_secs_f64() * 1000.0);
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (t[t.len() / 2], t[t.len() - 1], dernier.unwrap())
}

#[test]
#[ignore]
fn l_interieur_du_noeud_lexical() {
    let (Some(base), Some(manifeste)) = (std::env::var_os("RAG3WEAVER_SONDE_BASE"), std::env::var_os("RAG3WEAVER_SONDE_MANIFESTE")) else {
        eprintln!("SAUTÉ l_interieur_du_noeud_lexical : RAG3WEAVER_SONDE_BASE ou RAG3WEAVER_SONDE_MANIFESTE manque (une copie, jamais la base en service)");
        return;
    };
    let role = std::env::var("RAG3WEAVER_SONDE_ROLE").unwrap_or_else(|_| "versions".into());
    let (chauffe, tours) = (env_n("RAG3WEAVER_SONDE_CHAUFFE", 1), env_n("RAG3WEAVER_SONDE_TOURS", 5));
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let backend = {
        // L'embarqueur du manifeste, comme l'hôte : sans lui, le dense et le creux disent « not available ».
        let embarqueur = prepared.connect_embedder().map(|(e, _)| e).map_err(|e| eprintln!("[sonde] AVERTISSEMENT embarqueur absent : {e}")).ok();
        eprintln!("[sonde] embarqueur : {}", if embarqueur.is_some() { "connecté" } else { "ABSENT" });
        prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), embarqueur).expect("ouvrir le backend")
    };
    let mut cat = backend.catalog.lock().unwrap();
    let forme = rag3weaver::code::scope_form(&cat).unwrap();
    eprintln!("[sonde] rôle {role} ; forme {forme:?}");
    for e in ["Scope", "ScopeContent"] {
        match cat.index_state_for(e) {
            Ok(s) => eprintln!("[sonde] état de {e} : {s:?}"),
            Err(err) => eprintln!("[sonde] état de {e} : {err}"),
        }
    }
    if role != "versions" {
        return;
    }
    if forme == rag3weaver::code::ScopeForm::Versions {
        rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration");
    }
    let cible = cat.resolve_search_target("Scope").expect("la cible Scope");
    let projection = cible.occurrence_projection.clone().expect("une base à versions projette ses occurrences");
    let conn = cat.conn_arc();
    let dialecte = cat.dialect_arc();

    // 1. Les racines.
    let tout = dialecte.select_all(&projection.entity, &["_uuid"], None);
    let (m1, x1, racines): (f64, f64, Vec<String>) = chrono(chauffe, tours, || {
        conn.execute(&tout).unwrap().rows.into_iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect()
    });
    eprintln!("[sonde] 1 racines            médiane {m1:>8.1} ms   max {x1:>8.1} ms   {} racines", racines.len());
    // 2. Le masque.
    let masque = dialecte.derived_references(&cible.parent_table, &projection.entity, &projection.relation);
    let mut tri = racines.clone();
    tri.sort();
    let (m2, x2, versions): (f64, f64, Vec<String>) = chrono(chauffe, tours, || {
        let mut v = std::collections::HashSet::new();
        for lot in tri.chunks(5_000) {
            let p = QueryParam::new("uuids", CypherValue::List(lot.iter().cloned().map(CypherValue::String).collect()));
            for l in conn.execute_with_params(&masque, &[p]).unwrap().rows {
                if let Some(id) = l.get(1).and_then(|v| v.as_str()) {
                    v.insert(id.to_string());
                }
            }
        }
        let mut v: Vec<String> = v.into_iter().collect();
        v.sort();
        v
    });
    eprintln!("[sonde] 2 masque             médiane {m2:>8.1} ms   max {x2:>8.1} ms   {} versions", versions.len());
    // 3. La résolution du filtre `_uuid IN [versions]` en offsets, comme `allowed_ids_for`.
    let filtre = FilterCondition::Field {
        key: "_uuid".into(),
        value: FilterValue::List(versions.iter().cloned().map(CypherValue::String).collect()),
    };
    let (m3, x3, offsets) = chrono(chauffe, tours, || cat.resolve_filter_to_ids(&cible.parent_table, &filtre, &cible).unwrap());
    eprintln!(
        "[sonde] 3 filtre en offsets   médiane {m3:>8.1} ms   max {x3:>8.1} ms   {} offsets",
        offsets.map_or(0, |o| o.len())
    );
    // **Pour le ticket moteur** : la requête telle que `resolve_filter_to_ids` l'écrit, son plan, et la
    // même résolution par `UNWIND` et un accès par clé primaire.
    let sans_relations = std::collections::HashMap::new();
    let mut parseur = rag3weaver::filter::FilterParser::new(&sans_relations, dialecte.as_ref());
    let analyse = parseur.parse_condition(&filtre, &cible.parent_table, "n").expect("le filtre");
    let requete = dialecte.filter_resolve_offsets(&cible.parent_table, "n", &analyse.match_clauses, &analyse.combine_where(), None);
    eprintln!("[sonde] requête de resolve_filter_to_ids : {requete}");
    eprintln!("[sonde] paramètres : {}", analyse.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", "));
    match conn.execute_with_params(&format!("EXPLAIN {requete}"), &analyse.params) {
        Ok(r) => {
            for l in &r.rows {
                for v in l {
                    for ligne in format!("{}", v.as_str().unwrap_or(&format!("{v:?}"))).lines() {
                        eprintln!("[sonde] EXPLAIN | {ligne}");
                    }
                }
            }
        }
        Err(e) => eprintln!("[sonde] EXPLAIN refusé : {e}"),
    }
    let par_cle = format!("UNWIND $ids AS u MATCH (n:{} {{_uuid: u}}) RETURN OFFSET(id(n))", cible.parent_table);
    let liste = QueryParam::new("ids", CypherValue::List(versions.iter().cloned().map(CypherValue::String).collect()));
    let (m4, x4, n4) = chrono(chauffe, tours, || conn.execute_with_params(&par_cle, std::slice::from_ref(&liste)).unwrap().rows.len());
    eprintln!("[sonde] 4 même ensemble par UNWIND et clé primaire   médiane {m4:>8.1} ms   max {x4:>8.1} ms   {n4} offsets");
    eprintln!("[sonde] RSS {:.0} Mio", rss_mio());
    // **La mémoire, sans search_code** (`RAG3WEAVER_SONDE_REPETITIONS`, 30 par défaut) : l'étape 3 rejouée,
    // le RSS après chaque tour. Une pente ici, c'est le moteur qui garde, et pas le flux de la recherche.
    let repetitions = env_n("RAG3WEAVER_SONDE_REPETITIONS", 30);
    let depart = rss_mio();
    for i in 1..=repetitions {
        cat.resolve_filter_to_ids(&cible.parent_table, &filtre, &cible).unwrap();
        eprintln!("[sonde] étape 3, répétition {i:>3} : RSS {:.0} Mio ({:+.0} depuis le départ)", rss_mio(), rss_mio() - depart);
    }
    eprintln!("[sonde] RÉSUMÉ 1+2+3 = {:.1} ms (le nœud lexical sans filtre : 6 928 ms en médiane à la mesure par étage)", m1 + m2 + m3);

    // **Les ensembles pour le banc de lucivy** (`RAG3WEAVER_SONDE_OFFSETS=<dossier>`) : les offsets que
    // `allowed_ids_for` passe à lucivy, un u64 par ligne. Deux fichiers : sans filtre, et pour les
    // racines sous `<racine>/src/include/`. Le second est **approché** : il prend les `Scope` dont le
    // chemin commence par ce préfixe, sans repasser par la traduction du paramètre `scope` de l'outil.
    if let Some(dossier) = std::env::var_os("RAG3WEAVER_SONDE_OFFSETS") {
        let dossier = std::path::PathBuf::from(dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let ecrire = |nom: &str, versions: &[String]| {
            let f = FilterCondition::Field {
                key: "_uuid".into(),
                value: FilterValue::List(versions.iter().cloned().map(CypherValue::String).collect()),
            };
            let mut o = cat.resolve_filter_to_ids(&cible.parent_table, &f, &cible).unwrap().unwrap_or_default();
            o.sort_unstable();
            let texte: String = o.iter().map(|i| format!("{i}\n")).collect();
            std::fs::write(dossier.join(nom), texte).unwrap();
            eprintln!("[sonde] écrit {} : {} offsets", dossier.join(nom).display(), o.len());
        };
        ecrire("sans-filtre.ids", &versions);
        let racine = std::env::var("RAG3WEAVER_SONDE_RACINE").unwrap_or_else(|_| "/home/lucied/git_workspaces/rag3db".into());
        let prefixe = format!("{racine}/src/include/");
        let r = conn
            .execute_with_params(
                &format!("MATCH (s:{}) WHERE s.file_path STARTS WITH $p RETURN s._uuid", projection.entity),
                &[QueryParam::new("p", CypherValue::String(prefixe.clone()))],
            )
            .unwrap();
        let mut sous: Vec<String> = r.rows.iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect();
        sous.sort();
        let mut v = std::collections::HashSet::new();
        for lot in sous.chunks(5_000) {
            let p = QueryParam::new("uuids", CypherValue::List(lot.iter().cloned().map(CypherValue::String).collect()));
            for l in conn.execute_with_params(&masque, &[p]).unwrap().rows {
                if let Some(id) = l.get(1).and_then(|v| v.as_str()) {
                    v.insert(id.to_string());
                }
            }
        }
        let v: Vec<String> = v.into_iter().collect();
        eprintln!("[sonde] {} racines sous {prefixe}, {} versions", sous.len(), v.len());
        ecrire("chemin-src-include.ids", &v);
    }
}

