//! **Ce que coûte une recherche, étage par étage, avant et après les versions** (2a′-2 ; mot de Lucie
//! du 14 novembre 2026 : « si on apporte la feature faut faire le max pour qu'elle devienne viable »).
//! Page `docs/everything-declarative/14-11-2026/04-le-pas-2a-prime-2-l-ancre-immuable.md`, § 2.
//!
//! Sur une **copie** d'une base de code (`RAG3WEAVER_SONDE_BASE`) et son manifeste
//! (`RAG3WEAVER_SONDE_MANIFESTE`), jamais la base en service. `RAG3WEAVER_SONDE_ROLE` dit laquelle :
//! - `reference` : une base de la forme d'avant les versions, telle que les utilisateurs l'ont
//!   aujourd'hui. Un état, **R** ;
//! - `versions` : une base à versions, migrée ici par 2a′-1 (`code::migrer_vers_les_generations`,
//!   chronométrée). Trois états :
//!   - **A** : le drapeau `tout_visible` tenu ;
//!   - **B** : le drapeau tombé ;
//!   - **C** : B, et 1 % des `Scope` retirés au rang lu.
//!
//! Chaque requête du jeu (5 noms, les plus appelés, et 5 phrases) se joue sous trois formes :
//! - **sans filtre** ;
//! - **filtrée par chemin** : le sous-dossier qui porte la part de scopes la plus proche de 10 % ;
//! - **filtrée par langage** : le langage dont la part est la plus proche de 10 %.
//! R et A jouent les trois formes ; B et C la forme sans filtre seule, celle que le drapeau touche.
//!
//! **Les étages** : chaque nœud des graphes que `search_code` exécute publie `NodeRun` avec sa durée.
//! On y trouve la source, le lexical, le dense (l'embarquement de la requête compris), le creux, le
//! balayage, la fusion, la relecture, la pagination, le surlignage, le parent et le rendu, puis les
//! nœuds du crochet « après » (links.mmd). La sonde les écoute et en rend la médiane par nœud. Les
//! nœuds tournent en parallèle : leur somme dépasse la durée de l'appel, qui est donnée à part. Pour
//! les versions, la sonde chronomètre aussi, hors de l'appel, (a) les racines des occurrences et (b) le
//! masque des versions.
//!
//! `RAG3WEAVER_SONDE_CHAUFFE` tours de chauffe (1 par défaut), puis `RAG3WEAVER_SONDE_TOURS` tours (5 par
//! défaut). La médiane, et le maximum sur les tours (sous 20 tours, le « p95 » n'en est pas un). Une
//! sonde mesure, elle n'échoue pas.

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::events::{topic, Event};
use rag3weaver::search_backend::SearchBackend;
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

fn env_n(nom: &str, defaut: usize) -> usize {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

fn mediane(mut t: Vec<f64>) -> (f64, f64) {
    if t.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (t[t.len() / 2], t[t.len() - 1])
}

fn chrono<T>(chauffe: usize, tours: usize, mut f: impl FnMut() -> T) -> (f64, f64, T) {
    for _ in 0..chauffe {
        f();
    }
    let mut t = Vec::with_capacity(tours);
    let mut dernier = None;
    for _ in 0..tours {
        let d = Instant::now();
        dernier = Some(f());
        t.push(d.elapsed().as_secs_f64() * 1000.0);
    }
    let (m, x) = mediane(t);
    (m, x, dernier.unwrap())
}

/// L'écoute des `NodeRun`, sur les deux sujets où un run d'outil peut les publier.
struct Ecoute {
    rx: Vec<async_broadcast::Receiver<Event>>,
}

impl Ecoute {
    fn sur(backend: &Backend) -> Self {
        let bus = backend.catalog.lock().unwrap().event_bus();
        Ecoute { rx: vec![bus.subscribe(topic::DATAFLOW), bus.subscribe(topic::CATALOG)] }
    }
    /// Les durées par nœud depuis le dernier appel, sans doublon (un même run sur deux sujets).
    fn vider(&mut self) -> Vec<(String, f64)> {
        let mut vus = HashSet::new();
        let mut out = Vec::new();
        for rx in &mut self.rx {
            while let Ok(e) = rx.try_recv() {
                if let Event::NodeRun { run, node, node_type, ms, .. } = e {
                    if vus.insert((run.clone(), node.clone())) {
                        out.push((format!("{node} ({node_type})"), ms as f64));
                    }
                }
            }
        }
        out
    }
}

/// Une forme de requête : son nom et ses arguments en plus de `query`.
fn formes(chemin: &str, langage: &str, toutes: bool) -> Vec<(String, Value)> {
    let mut f = vec![("sans filtre".to_string(), json!({"options": {}}))];
    if toutes {
        f.push((format!("chemin {chemin}"), json!({"options": {}, "scope": chemin})));
        f.push((format!("langage {langage}"), json!({"options": {}, "language": langage})));
    }
    f
}

/// Joue chaque requête sous une forme ; rend la médiane des appels et les étages.
fn jouer(backend: &Backend, ecoute: &mut Ecoute, etat: &str, forme: &str, extra: &Value, requetes: &[String], chauffe: usize, tours: usize) -> f64 {
    let mut appels = Vec::new();
    let mut etages: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut resultats = Vec::new();
    for q in requetes {
        let mut args = extra.clone();
        args["query"] = json!(q);
        for _ in 0..chauffe {
            backend.call("search_code", args.clone()).unwrap_or_else(|e| panic!("search_code {q} : {e}"));
        }
        ecoute.vider();
        for _ in 0..tours {
            let d = Instant::now();
            let r = backend.call("search_code", args.clone()).unwrap_or_else(|e| panic!("search_code {q} : {e}"));
            appels.push(d.elapsed().as_secs_f64() * 1000.0);
            for (n, ms) in ecoute.vider() {
                etages.entry(n).or_default().push(ms);
            }
            resultats.push(r["result"].as_array().map_or(0, |a| a.len()));
        }
    }
    let (m, x) = mediane(appels.clone());
    let (mr, _) = mediane(resultats.iter().map(|n| *n as f64).collect());
    eprintln!("[sonde] {etat} {forme:<28} appel : médiane {m:>8.1} ms   max {x:>8.1} ms   ({} appels, {mr:.0} résultats en médiane)", appels.len());
    let mut lignes: Vec<(String, f64, f64, usize)> =
        etages.into_iter().map(|(n, t)| { let k = t.len(); let (m, x) = mediane(t); (n, m, x, k) }).collect();
    lignes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (n, m, x, k) in lignes {
        eprintln!("[sonde] {etat} {forme:<28}   étage {n:<44} médiane {m:>8.1} ms   max {x:>8.1} ms   ({k} passages)");
    }
    m
}

#[test]
#[ignore]
fn le_cout_d_une_recherche_par_etage() {
    let (Some(base), Some(manifeste)) = (std::env::var_os("RAG3WEAVER_SONDE_BASE"), std::env::var_os("RAG3WEAVER_SONDE_MANIFESTE")) else {
        eprintln!("SAUTÉ le_cout_d_une_recherche_par_etage : RAG3WEAVER_SONDE_BASE ou RAG3WEAVER_SONDE_MANIFESTE manque (une copie, jamais la base en service)");
        return;
    };
    let role = std::env::var("RAG3WEAVER_SONDE_ROLE").unwrap_or_else(|_| "versions".into());
    let (chauffe, tours) = (env_n("RAG3WEAVER_SONDE_CHAUFFE", 1), env_n("RAG3WEAVER_SONDE_TOURS", 5));
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let racine = prepared.manifest.workspace.as_ref().map(|w| w.root.to_string_lossy().trim_end_matches('/').to_string()).unwrap_or_default();
    let t0 = Instant::now();
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), None).expect("ouvrir le backend");
    eprintln!("[sonde] rôle {role} ; ouverture {:.0} ms ; {chauffe} chauffe(s), {tours} tours ; racine {racine}", t0.elapsed().as_secs_f64() * 1000.0);

    let versions = role == "versions";
    let (conn, dialecte, cible) = {
        let mut cat = backend.catalog.lock().unwrap();
        let forme = rag3weaver::code::scope_form(&cat).unwrap();
        eprintln!("[sonde] forme à l'ouverture : {forme:?}");
        if versions && forme == rag3weaver::code::ScopeForm::Versions {
            let t = Instant::now();
            rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration");
            eprintln!("[sonde] migration 2a′-1 : {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
        }
        if versions {
            assert_eq!(rag3weaver::code::scope_form(&cat).unwrap(), rag3weaver::code::ScopeForm::Generations, "la copie doit être à générations");
        }
        (cat.conn_arc(), cat.dialect_arc(), cat.resolve_search_target("Scope").expect("la cible Scope"))
    };
    let compter = |q: &str| match conn.execute(q).unwrap_or_else(|e| panic!("{q} : {e}")).rows.first().and_then(|l| l.first()) {
        Some(CypherValue::Int(n)) => *n,
        v => panic!("{q} : {v:?}"),
    };
    let total = compter("MATCH (s:Scope) RETURN count(*)");
    eprintln!("[sonde] Scope : {total} lignes ; parent {} ; projection {:?}", cible.parent_table, cible.occurrence_projection.as_ref().map(|p| (&p.entity, &p.relation)));

    // Les filtres : le sous-dossier et le langage dont la part est la plus proche de 10 %.
    let r = conn.execute("MATCH (s:Scope) RETURN s.file_path, s.language").unwrap();
    let mut dossiers: BTreeMap<String, i64> = BTreeMap::new();
    let mut langages: BTreeMap<String, i64> = BTreeMap::new();
    for l in &r.rows {
        if let Some(p) = l.first().and_then(|v| v.as_str()) {
            let rel = p.strip_prefix(&racine).unwrap_or(p).trim_start_matches('/');
            let morceaux: Vec<&str> = rel.split('/').collect();
            for k in 1..morceaux.len().min(4) {
                *dossiers.entry(format!("{}/", morceaux[..k].join("/"))).or_default() += 1;
            }
        }
        if let Some(g) = l.get(1).and_then(|v| v.as_str()) {
            *langages.entry(g.to_string()).or_default() += 1;
        }
    }
    let proche = |m: &BTreeMap<String, i64>| -> (String, i64) {
        m.iter().min_by_key(|(_, n)| ((**n as f64 / total as f64 - 0.10).abs() * 1e6) as i64).map(|(k, n)| (k.clone(), *n)).unwrap_or_default()
    };
    let (chemin, nc) = proche(&dossiers);
    let (langage, nl) = proche(&langages);
    eprintln!("[sonde] filtre par chemin « {chemin} » : {nc} scopes ({:.1} %) ; par langage « {langage} » : {nl} ({:.1} %)", 100.0 * nc as f64 / total as f64, 100.0 * nl as f64 / total as f64);

    // Le jeu : les 5 noms les plus appelés, et 5 phrases.
    let r = conn.execute("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN b.name AS n, count(*) AS k ORDER BY k DESC LIMIT 5").unwrap();
    let mut requetes: Vec<String> = r.rows.iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect();
    requetes.extend(["ouvrir une base et enregistrer le schéma", "parse a json manifest", "rebuild the vector index", "transaction rollback", "translate a hop into a query"].map(String::from));
    eprintln!("[sonde] requêtes : {requetes:?}");

    let mut ecoute = Ecoute::sur(&backend);
    if !versions {
        for (forme, extra) in formes(&chemin, &langage, true) {
            jouer(&backend, &mut ecoute, "R", &forme, &extra, &requetes, chauffe, tours);
        }
        return;
    }

    let projection = cible.occurrence_projection.clone().expect("une base à versions projette ses occurrences");
    let moteur = rag3weaver::rag3db_search_backend::Rag3dbSearchBackend::new(conn.clone());
    let tout = dialecte.select_all(&projection.entity, &["_uuid"], None);
    let masque = dialecte.derived_references(&cible.parent_table, &projection.entity, &projection.relation);
    let clause = dialecte.condition_visible_sur("n", rag3weaver::code::COUCHE_DU_BACKEND, 0).expect("rag3db sait les générations");
    let visibles = dialecte.filter_resolve_offsets(&projection.entity, "n", &[], &clause, None);
    let hors_appel = |etat: &str, drapeau: bool| {
        let (ma, xa, racines): (f64, f64, Vec<String>) = if drapeau {
            chrono(chauffe, tours, || conn.execute(&tout).unwrap().rows.into_iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect())
        } else {
            chrono(chauffe, tours, || {
                let mut ids: Vec<u64> = conn.execute(&visibles).unwrap().rows.iter().filter_map(|l| l.first().and_then(|v| v.as_i64()).map(|i| i as u64)).collect();
                ids.sort_unstable();
                ids.dedup();
                moteur.resolve_offsets(&projection.entity, &ids, &[]).unwrap().into_iter().map(|r| r.uuid).collect()
            })
        };
        eprintln!("[sonde] {etat} hors appel (a) racines   médiane {ma:>8.1} ms   max {xa:>8.1} ms   {} racines", racines.len());
        let mut tri = racines;
        tri.sort();
        let (mb, xb, n) = chrono(chauffe, tours, || {
            tri.chunks(5_000).map(|lot| {
                let p = QueryParam::new("uuids", CypherValue::List(lot.iter().cloned().map(CypherValue::String).collect()));
                conn.execute_with_params(&masque, &[p]).unwrap().rows.len()
            }).sum::<usize>()
        });
        eprintln!("[sonde] {etat} hors appel (b) masque    médiane {mb:>8.1} ms   max {xb:>8.1} ms   {n} références");
    };

    hors_appel("A", true);
    let mut a = 0.0;
    for (forme, extra) in formes(&chemin, &langage, true) {
        let m = jouer(&backend, &mut ecoute, "A", &forme, &extra, &requetes, chauffe, tours);
        if forme == "sans filtre" {
            a = m;
        }
    }
    backend.catalog.lock().unwrap().marquer_des_lignes_invisibles();
    hors_appel("B", false);
    let (_, sans) = formes(&chemin, &langage, false).remove(0);
    let b = jouer(&backend, &mut ecoute, "B", "sans filtre", &sans, &requetes, chauffe, tours);
    let retirees = compter(&format!("MATCH (s:{}) WHERE offset(id(s)) % 100 = 0 SET s._jusqu_a = 0 RETURN count(*)", projection.entity));
    eprintln!("[sonde] C : {retirees} lignes retirées au rang 0");
    hors_appel("C", false);
    let c = jouer(&backend, &mut ecoute, "C", "sans filtre", &sans, &requetes, chauffe, tours);
    eprintln!("[sonde] RÉSUMÉ sans filtre, médiane d'un appel : A {a:.1} ms, B {b:.1} ms ({:+.0} %), C {c:.1} ms ({:+.0} %)", (b / a - 1.0) * 100.0, (c / a - 1.0) * 100.0);
}
