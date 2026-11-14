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
            let r = backend.call("search_code", args).unwrap_or_else(|e| panic!("search_code {q} ({nom}) : {e}"));
            let lignes: Vec<Value> = r["result"].as_array().map(|a| a.iter().take(k).cloned().collect()).unwrap_or_default();
            let resultats: Vec<Value> = lignes
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    json!({
                        "rang": i + 1,
                        "_uuid": l["_uuid"],
                        "file_path": l["file_path"],
                        "name": l["name"],
                        "start_line": l["start_line"],
                        // Le score s'appelle `score` ou `_score` selon le rendu.
                        "score": if l["score"].is_null() { l["_score"].clone() } else { l["score"].clone() },
                    })
                })
                .collect();
            if let Some(l) = lignes.first() {
                // Les champs d'une ligne, pour vérifier que la clé (_uuid, ou le repli) y est.
                let champs: Vec<&String> = l.as_object().map(|o| o.keys().collect()).unwrap_or_default();
                eprintln!("[parité] {q} · {nom} : {} résultats ; champs {champs:?}", resultats.len());
            } else {
                eprintln!("[parité] {q} · {nom} : aucun résultat");
            }
            releve.push(json!({"requete": q, "forme": nom, "resultats": resultats}));
        }
    }
    let doc = json!({"base": base.to_string_lossy(), "manifeste": manifeste.to_string_lossy(), "forme_de_la_base": forme, "k": k, "jeu": jeu, "releve": releve});
    std::fs::write(&sortie, serde_json::to_string_pretty(&doc).unwrap()).expect("écrire le relevé");
    eprintln!("[parité] relevé écrit : {}", sortie.to_string_lossy());
}
