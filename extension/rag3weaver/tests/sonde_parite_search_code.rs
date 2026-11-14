//! **Le juge de parité de `search_code`** (2a′-2 ; banc, 14 novembre 2026, à la demande de
//! l'orchestration) : les réponses ne doivent pas changer quand on optimise la recherche sur une
//! base à versions. Ce test ne juge rien lui-même : il **relève** les k premiers résultats de
//! chaque requête du jeu commun, sur une base, dans un JSON. Le comparateur
//! `scripts/parite_search_code.py` juge ensuite deux relevés, et nomme les écarts.
//!
//! Sur une **copie** d'une base de code (`RAG3WEAVER_PARITE_BASE`) et son manifeste
//! (`RAG3WEAVER_PARITE_MANIFESTE`), jamais la base en service. Le jeu est un fichier commun avec la
//! sonde de la recherche (`RAG3WEAVER_PARITE_REQUETES`) : `{"requetes": [...], "chemin": "...",
//! "langage": "..."}`. Il est tiré une fois sur la base d'avant les versions, et figé, pour que les
//! noms ne dérivent pas d'une base à l'autre. Chaque requête se joue sous trois formes : sans filtre,
//! filtrée par chemin et filtrée par langage. Le relevé s'écrit dans `RAG3WEAVER_PARITE_SORTIE`.
//!
//! Une base à versions est d'abord passée aux générations, comme la sonde de la recherche le fait
//! (`code::migrer_vers_les_generations`) : c'est la forme que l'optimisation touche. La copie est
//! modifiée, d'où la règle de n'ouvrir que des copies.

use rag3weaver::backend::PreparedBackend;
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};

#[test]
#[ignore]
fn releve_les_reponses_de_search_code() {
    let var = |n: &str| std::env::var_os(n);
    let (Some(base), Some(manifeste), Some(requetes), Some(sortie)) =
        (var("RAG3WEAVER_PARITE_BASE"), var("RAG3WEAVER_PARITE_MANIFESTE"), var("RAG3WEAVER_PARITE_REQUETES"), var("RAG3WEAVER_PARITE_SORTIE"))
    else {
        eprintln!("SAUTÉ releve_les_reponses_de_search_code : RAG3WEAVER_PARITE_BASE, _MANIFESTE, _REQUETES ou _SORTIE manque (une copie, jamais la base en service)");
        return;
    };
    let k: usize = std::env::var("RAG3WEAVER_PARITE_K").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let jeu: Value = serde_json::from_str(&std::fs::read_to_string(&requetes).expect("le jeu")).expect("le jeu en JSON");
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), None).expect("ouvrir le backend");
    let forme = {
        let mut cat = backend.catalog.lock().unwrap();
        let avant = rag3weaver::code::scope_form(&cat).unwrap();
        if avant == rag3weaver::code::ScopeForm::Versions {
            rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration vers les générations");
        }
        format!("{avant:?} → {:?}", rag3weaver::code::scope_form(&cat).unwrap())
    };
    eprintln!("[parité] base {} ; forme {forme} ; k = {k}", base.to_string_lossy());
    // Tous les `_uuid` de Scope, à côté du relevé : la clé ne vaut que si R et la base migrée en
    // partagent l'essentiel (la migration 1b a recopié les lignes).
    {
        let conn = backend.catalog.lock().unwrap().conn_arc();
        let r = conn.execute("MATCH (s:Scope) RETURN s._uuid").expect("les _uuid de Scope");
        let mut uuids: Vec<String> = r.rows.iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect();
        uuids.sort();
        let chemin = format!("{}.uuids", sortie.to_string_lossy());
        std::fs::write(&chemin, uuids.join("\n")).expect("écrire les _uuid");
        eprintln!("[parité] {} _uuid de Scope écrits dans {chemin}", uuids.len());
    }
    let formes = [
        ("sans filtre".to_string(), json!({"options": {}})),
        (format!("chemin {}", jeu["chemin"].as_str().unwrap_or("")), json!({"options": {}, "scope": jeu["chemin"]})),
        (format!("langage {}", jeu["langage"].as_str().unwrap_or("")), json!({"options": {}, "language": jeu["langage"]})),
    ];
    let mut releve = Vec::new();
    for q in jeu["requetes"].as_array().expect("requetes") {
        for (nom, extra) in &formes {
            let mut args = extra.clone();
            args["query"] = q.clone();
            // Le temps d'un seul appel, à titre indicatif : pas de chauffe ni de répétition, ce n'est
            // pas une mesure (la sonde de la recherche en est une).
            let debut = std::time::Instant::now();
            let r = backend.call("search_code", args).unwrap_or_else(|e| panic!("search_code {q} ({nom}) : {e}"));
            let ms = debut.elapsed().as_secs_f64() * 1000.0;
            let lignes: Vec<Value> = r["result"].as_array().map(|a| a.iter().take(k).cloned().collect()).unwrap_or_default();
            let resultats: Vec<Value> = lignes
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    // Une ligne de search_code : `uuid`, `score`, `signal` et `bestSignal` au premier niveau,
                    // la fiche du Scope sous `data`.
                    let d = &l["data"];
                    let pris = |cles: &[&str]| cles.iter().map(|c| &d[*c]).find(|v| !v.is_null()).cloned().unwrap_or(Value::Null);
                    json!({
                        "rang": i + 1,
                        "_uuid": if l["uuid"].is_null() { d["_uuid"].clone() } else { l["uuid"].clone() },
                        "file_path": pris(&["file_path", "path"]),
                        "name": pris(&["name", "qualified_name"]),
                        "start_line": pris(&["start_line", "line"]),
                        // Le contenu partagé : deux occurrences d'un même contenu (un fichier recopié) ont
                        // le même hash ; le comparateur sépare ainsi « même contenu, autre occurrence ».
                        "content_hash": d["_content_hash"],
                        "score": l["score"],
                        "signal": l["signal"],
                        "best_signal": l["bestSignal"],
                    })
                })
                .collect();
            if let Some(l) = lignes.first() {
                // Les champs de `data`, pour vérifier que le repli (file_path, name, start_line) y est.
                let champs: Vec<&String> = l["data"].as_object().map(|o| o.keys().collect()).unwrap_or_default();
                eprintln!("[parité] {q} · {nom} : {} résultats ; premier {} ; champs de data {champs:?}", resultats.len(), resultats[0]["_uuid"]);
            } else {
                eprintln!("[parité] {q} · {nom} : aucun résultat");
            }
            releve.push(json!({"requete": q, "forme": nom, "ms": ms, "resultats": resultats}));
        }
    }
    let doc = json!({"base": base.to_string_lossy(), "manifeste": manifeste.to_string_lossy(), "forme_de_la_base": forme, "k": k, "jeu": jeu, "releve": releve});
    std::fs::write(&sortie, serde_json::to_string_pretty(&doc).unwrap()).expect("écrire le relevé");
    eprintln!("[parité] relevé écrit : {}", sortie.to_string_lossy());
}

/// **Les orphelins du plein texte après un effacement** (le lot des orphelins de recherche, 14 nov.
/// 2026). Sur une copie et sa source (le manifeste repointé sur un worktree à soi), dans un processus
/// neuf : les comptes du plein texte à l'ouverture, puis après un index de la source entière (qui ne
/// doit rien changer), puis après un index de la même source sans `RAG3WEAVER_PARITE_A_RETIRER`.
/// Pour chaque entité, (documents de l'index plein texte, lignes de la table) : au code d'avant,
/// l'écart documents − lignes grandit du nombre de Scope effacés ; au lot, il ne bouge pas. Le fichier
/// retiré est remis à la fin. Sortie : `RAG3WEAVER_PARITE_SORTIE`.
#[test]
#[ignore]
fn releve_les_comptes_du_plein_texte_apres_un_effacement() {
    let var = |n: &str| std::env::var_os(n);
    let (Some(base), Some(manifeste), Some(retirer), Some(sortie)) =
        (var("RAG3WEAVER_PARITE_BASE"), var("RAG3WEAVER_PARITE_MANIFESTE"), var("RAG3WEAVER_PARITE_A_RETIRER"), var("RAG3WEAVER_PARITE_SORTIE"))
    else {
        eprintln!("SAUTÉ releve_les_comptes_du_plein_texte_apres_un_effacement : RAG3WEAVER_PARITE_BASE, _MANIFESTE, _A_RETIRER ou _SORTIE manque");
        return;
    };
    let prepared = PreparedBackend::load(std::path::Path::new(&manifeste)).expect("le manifeste");
    let racine = prepared.manifest.workspace.as_ref().map(|w| w.root.clone()).expect("la racine du manifeste");
    let fichier = racine.join(&retirer);
    let contenu = std::fs::read(&fichier).unwrap_or_else(|e| panic!("le fichier à retirer {} : {e}", fichier.display()));
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("ouvrir la copie")), None).expect("ouvrir le backend");
    // Les lignes du fichier retiré lui-même, par table : un fichier effacé ne doit plus rien porter.
    let nom_retire = retirer.to_string_lossy().to_string();
    let du_fichier = |etape: &str| -> Value {
        let conn = backend.catalog.lock().unwrap().conn_arc();
        let mut out = serde_json::Map::new();
        for (table, champ) in [("Scope", "file_path"), ("File", "path"), ("Symbol", "file_path")] {
            let q = format!("MATCH (n:{table}) WHERE n.{champ} ENDS WITH '{nom_retire}' RETURN count(*)");
            let v = match conn.execute(&q) {
                Ok(r) => r.rows.first().and_then(|l| l.first()).map(|v| format!("{v:?}")).unwrap_or_default(),
                Err(e) => format!("refus : {e}"),
            };
            eprintln!("[orphelins] {etape} : {table} du fichier retiré — {v}");
            // Les lignes elles-mêmes, toutes colonnes : sur une base à versions, une ligne retirée se
            // ferme (colonnes de version) au lieu de disparaître ; c'est elles qu'il faut lire.
            let lignes = match conn.execute(&format!("MATCH (n:{table}) WHERE n.{champ} ENDS WITH '{nom_retire}' RETURN n LIMIT 20")) {
                Ok(r) => r.rows.iter().map(|l| format!("{:?}", l.first())).collect::<Vec<_>>(),
                Err(e) => vec![format!("refus : {e}")],
            };
            for l in &lignes {
                eprintln!("[orphelins] {etape} : {table} ligne {}", &l[..l.len().min(600)]);
            }
            out.insert(table.to_string(), json!({"compte": v, "lignes": lignes}));
        }
        Value::Object(out)
    };
    let comptes = |etape: &str| -> Value {
        let c = backend.catalog.lock().unwrap().comptes_du_plein_texte();
        for (e, d, l) in &c {
            eprintln!("[orphelins] {etape} : {e} — {d} documents, {l} lignes, écart {}", *d as i64 - *l as i64);
        }
        json!(c.iter().map(|(e, d, l)| json!({"entite": e, "documents": d, "lignes": l, "ecart": *d as i64 - *l as i64})).collect::<Vec<_>>())
    };
    let indexer = |etape: &str| {
        let lance = backend.call("index", json!({"confirm": true})).unwrap_or_else(|e| panic!("index ({etape}) : {e}"));
        let mut r = &lance;
        while let Some(i) = r.get("result") {
            r = i;
        }
        let journal = r.get("journal").and_then(Value::as_str).map(str::to_string)
            .or_else(|| r.as_str().and_then(|t| t.lines().find_map(|l| l.strip_prefix("journal : ").map(str::to_string))))
            .unwrap_or_else(|| panic!("le journal de l'index ({etape}) : {lance}"));
        let debut = std::time::Instant::now();
        loop {
            let lu = std::fs::read_to_string(&journal).unwrap_or_default();
            if lu.contains("indexation terminée") {
                eprintln!("[orphelins] index {etape} fini en {:.0} s", debut.elapsed().as_secs_f64());
                return lu.lines().rev().take(6).collect::<Vec<_>>().join("\n");
            }
            assert!(!lu.contains("indexation échouée"), "index {etape} : {lu}");
            assert!(debut.elapsed().as_secs() < 3600, "index {etape} : pas fini en une heure");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    };
    let ouverture = comptes("ouverture");
    let fin1 = indexer("source entière");
    let entiere = comptes("après l'index de la source entière");
    let fichier_avant = du_fichier("après l'index de la source entière");
    std::fs::remove_file(&fichier).expect("retirer le fichier");
    let fin2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| indexer("sans le fichier")));
    std::fs::write(&fichier, &contenu).expect("remettre le fichier");
    let fin2 = fin2.unwrap_or_else(|e| std::panic::resume_unwind(e));
    let sans = comptes("après l'index sans le fichier");
    let fichier_apres = du_fichier("après l'index sans le fichier");
    let doc = json!({"base": base.to_string_lossy(), "racine": racine, "retire": retirer.to_string_lossy(),
                     "ouverture": ouverture, "apres_source_entiere": entiere, "apres_effacement": sans, "fichier_retire_avant": fichier_avant, "fichier_retire_apres": fichier_apres,
                     "fin_index_source_entiere": fin1, "fin_index_effacement": fin2});
    std::fs::write(&sortie, serde_json::to_string_pretty(&doc).unwrap()).expect("écrire le relevé");
    eprintln!("[orphelins] relevé écrit : {}", sortie.to_string_lossy());
}
