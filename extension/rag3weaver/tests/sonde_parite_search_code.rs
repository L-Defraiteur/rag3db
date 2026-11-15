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
use rag3weaver::connection::CypherValue;
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
    // **Des versions orphelines, à la demande** (le sens changé de la vue publiée, 14 nov.) : sous
    // `RAG3WEAVER_PARITE_ORPHELINES` (un fichier de `_uuid` de Scope, un par ligne), toutes les
    // occurrences des versions de ces Scope sont effacées de la copie. Leurs ScopeContent restent sans
    // origine : une recherche sans filtre ne peut plus court-circuiter le masque. Les mêmes lignes au
    // parent et au lot, sur la même copie neuve : la parité reste au bit près.
    let orphelines = if let Some(f) = var("RAG3WEAVER_PARITE_ORPHELINES") {
        let graines: Vec<String> = std::fs::read_to_string(&f).expect("les Scope à rendre orphelins").lines()
            .map(str::trim).filter(|l| !l.is_empty()).map(|l| format!("'{l}'")).collect();
        let conn = backend.catalog.lock().unwrap().conn_arc();
        let compter = || conn.execute("MATCH (d:ScopeContent) WHERE NOT EXISTS { MATCH (d)-[:ScopeContent_DERIVED_FROM]->(:Scope) } RETURN count(d)")
            .map(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()).unwrap_or(-1)).unwrap_or_else(|e| panic!("compter les orphelines : {e}"));
        let avant = compter();
        let cibles: Vec<String> = conn.execute(&format!(
            "MATCH (d:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s0:Scope) WHERE s0._uuid IN [{}] \
             MATCH (d)-[:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN DISTINCT s._uuid", graines.join(", ")))
            .expect("les occurrences des versions visées").rows.iter()
            .filter_map(|l| l.first().and_then(|v| v.as_str()).map(|u| format!("'{u}'"))).collect();
        conn.execute(&format!("MATCH (s:Scope) WHERE s._uuid IN [{}] DETACH DELETE s", cibles.join(", ")))
            .expect("effacer les occurrences");
        let apres = compter();
        eprintln!("[parité] orphelines : {} Scope visés, {} occurrences effacées ; versions sans origine {avant} → {apres}", graines.len(), cibles.len());
        assert!(apres > avant, "aucune version orpheline fabriquée ({avant} → {apres})");
        json!({"scope_vises": graines.len(), "occurrences_effacees": cibles.len(), "orphelines_avant": avant, "orphelines_apres": apres})
    } else {
        Value::Null
    };
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
    let releve = releve_du_jeu(&backend, &jeu, k);
    let doc = json!({"base": base.to_string_lossy(), "manifeste": manifeste.to_string_lossy(), "forme_de_la_base": forme, "orphelines": orphelines, "k": k, "jeu": jeu, "releve": releve});
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
    // **Sous `RAG3WEAVER_PARITE_GENERATIONS=1`** (l'écrivain de 2a′-2) : la copie passe aux générations,
    // et chaque index est suivi d'une publication là où le code sait publier.
    let generations = std::env::var("RAG3WEAVER_PARITE_GENERATIONS").as_deref() == Ok("1");
    if generations {
        let mut cat = backend.catalog.lock().unwrap();
        if rag3weaver::code::scope_form(&cat).unwrap() == rag3weaver::code::ScopeForm::Versions {
            rag3weaver::code::migrer_vers_les_generations(&mut cat).expect("la migration vers les générations");
        }
        eprintln!("[orphelins] forme {:?}", rag3weaver::code::scope_form(&cat).unwrap());
    }
    // <publication>
    let publier = |etape: &str| -> Value {
        if !generations {
            return Value::Null;
        }
        let mut cat = backend.catalog.lock().unwrap();
        let r = cat.publier_la_generation(&format!("banc-{etape}"), 0);
        eprintln!("[orphelins] publication après {etape} : {r:?} ; rang publié {:?}, en attente {:?}", cat.rang_publie(), cat.rang_en_attente());
        json!({"resultat": format!("{r:?}"), "rang": r.as_ref().ok()})
    };
    // </publication>
    // <au_rang>
    // **Les Symbol visibles au rang n** (la lecture d'everything declarative, 15 nov.) : un Symbol n'a
    // pas de colonnes de visibilité ; il est visible à n s'il a une relation vers un Scope visible à
    // n. La condition porte sur le Scope, sous `au_rang`, par le catalogue (aucun appel d'outil ne lit
    // un rang aujourd'hui).
    let visibles_au_rang = |n: i64, uuids: &[String]| -> Value {
        let liste = uuids.iter().map(|u| format!("'{u}'")).collect::<Vec<_>>().join(", ");
        let mut cat = backend.catalog.lock().unwrap();
        cat.au_rang(n, |c| {
            let Some(p) = c.dialect_arc().condition_visible_sur("s", rag3weaver::code::COUCHE_DU_BACKEND, n) else {
                return json!("refus : pas de condition de visibilité");
            };
            let q = format!("MATCH (s:Symbol)<-[r]-(x:Scope) WHERE s._uuid IN [{liste}] AND {} RETURN s.name, type(r), x.file_path, x.start_line", p.replace("s.", "x."));
            match c.conn().execute(&q) {
                Ok(r) => json!(r.rows.iter().map(|l| format!("{l:?}")).collect::<Vec<_>>()),
                Err(e) => json!(format!("refus : {e} ; requête {q}")),
            }
        })
    };
    // </au_rang>
    // Les lignes du fichier retiré lui-même, par table : un fichier effacé ne doit plus rien porter.
    let nom_retire = retirer.to_string_lossy().to_string();
    let du_fichier = |etape: &str| -> Value {
        let conn = backend.catalog.lock().unwrap().conn_arc();
        let mut out = serde_json::Map::new();
        for (table, champ) in [("Scope", "file_path"), ("File", "path")] {
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
            let versions = match conn.execute(&format!("MATCH (n:{table}) WHERE n.{champ} ENDS WITH '{nom_retire}' RETURN n._uuid, n._couche, n._depuis, n._jusqu_a LIMIT 20")) {
                Ok(r) => r.rows.iter().map(|l| format!("{l:?}")).collect::<Vec<_>>(),
                Err(e) => vec![format!("refus : {e}")],
            };
            for v in &versions {
                eprintln!("[orphelins] {etape} : {table} version {v}");
            }
            out.insert(table.to_string(), json!({"compte": v, "lignes": lignes, "versions": versions}));
        }
        Value::Object(out)
    };
    // **Les Symbol du fichier, par la relation** : `Symbol` n'a pas de chemin ; il est le rendez-vous
    // des scopes (DEFINES, MENTIONS, LOCKS, QUERIES, REGISTERS). Ceux du fichier sont ceux qu'un de ses
    // Scope relie ; ses exclusifs, ceux qu'aucun Scope d'un autre fichier ne relie. Après le retrait,
    // un exclusif encore là et que plus rien ne relie est un Symbol orphelin.
    let uuids_de = |q: &str| -> Result<Vec<String>, String> {
        let conn = backend.catalog.lock().unwrap().conn_arc();
        conn.execute(q).map_err(|e| format!("refus : {e}")).map(|r| {
            r.rows.iter().filter_map(|l| match l.first() { Some(CypherValue::String(u)) => Some(u.clone()), _ => None }).collect()
        })
    };
    let symboles_avant = || -> Value {
        let tous = uuids_de(&format!(
            "MATCH (s:Scope)-[]->(y:Symbol) WHERE s.file_path ENDS WITH '{nom_retire}' RETURN DISTINCT y._uuid"));
        let exclusifs = uuids_de(&format!(
            "MATCH (s:Scope)-[]->(y:Symbol) WHERE s.file_path ENDS WITH '{nom_retire}' WITH DISTINCT y \
             WHERE NOT EXISTS {{ MATCH (t:Scope)-[]->(y) WHERE NOT t.file_path ENDS WITH '{nom_retire}' }} RETURN y._uuid"));
        eprintln!("[orphelins] après l'index de la source entière : Symbol reliés au fichier — {:?} ; exclusifs — {:?}",
                  tous.as_ref().map(Vec::len), exclusifs.as_ref().map(Vec::len));
        json!({"relies": tous.as_ref().map(Vec::len).map_err(Clone::clone), "exclusifs": exclusifs.clone().unwrap_or_default(),
               "refus": exclusifs.err()})
    };
    let symboles_apres = |exclusifs: &[String]| -> Value {
        let conn = backend.catalog.lock().unwrap().conn_arc();
        let liste = exclusifs.iter().map(|u| format!("'{u}'")).collect::<Vec<_>>().join(", ");
        let q = format!("UNWIND [{liste}] AS u MATCH (y:Symbol {{_uuid: u}}) OPTIONAL MATCH (s:Scope)-[]->(y) \
                         RETURN y._uuid, y.name, count(s)");
        let lignes = match conn.execute(&q) {
            Ok(r) => r.rows.iter().map(|l| format!("{l:?}")).collect::<Vec<_>>(),
            Err(e) => vec![format!("refus : {e}")],
        };
        let orphelins = lignes.iter().filter(|l| l.ends_with("Int(0)]")).count();
        eprintln!("[orphelins] après l'index sans le fichier : Symbol exclusifs encore là — {} sur {} ; sans aucune relation — {orphelins}",
                  lignes.len(), exclusifs.len());
        for l in lignes.iter().take(20) {
            eprintln!("[orphelins] après l'index sans le fichier : Symbol exclusif {l}");
        }
        json!({"exclusifs_avant": exclusifs.len(), "encore_la": lignes, "sans_relation": orphelins})
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
    let pub1 = publier("source entière");
    let entiere = comptes("après l'index de la source entière");
    let fichier_avant = du_fichier("après l'index de la source entière");
    let symb_avant = symboles_avant();
    let exclusifs: Vec<String> = symb_avant["exclusifs"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    std::fs::remove_file(&fichier).expect("retirer le fichier");
    let fin2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| indexer("sans le fichier")));
    std::fs::write(&fichier, &contenu).expect("remettre le fichier");
    let fin2 = fin2.unwrap_or_else(|e| std::panic::resume_unwind(e));
    let pub2 = publier("sans le fichier");
    let sans = comptes("après l'index sans le fichier");
    let fichier_apres = du_fichier("après l'index sans le fichier");
    let symb_apres = symboles_apres(&exclusifs);
    let (remise, visibilites) = if generations {
        let fin3 = indexer("après la remise");
        let pub3 = publier("après la remise");
        let fichier_remis = du_fichier("après la remise");
        let mut vis = serde_json::Map::new();
        for (nom, p) in [("avant le retrait", &pub1), ("au retrait", &pub2), ("après la remise", &pub3)] {
            if let Some(n) = p["rang"].as_i64() {
                let v = visibles_au_rang(n, &exclusifs);
                eprintln!("[orphelins] Symbol exclusifs visibles au rang {n} ({nom}) : {v}");
                vis.insert(nom.to_string(), json!({"rang": n, "relations": v}));
            }
        }
        (json!({"fin_index": fin3, "publication": pub3, "fichier": fichier_remis}), Value::Object(vis))
    } else {
        (Value::Null, Value::Null)
    };
    let doc = json!({"base": base.to_string_lossy(), "racine": racine, "retire": retirer.to_string_lossy(),
                     "ouverture": ouverture, "apres_source_entiere": entiere, "apres_effacement": sans, "fichier_retire_avant": fichier_avant, "fichier_retire_apres": fichier_apres,
                     "symboles_avant": symb_avant, "symboles_apres": symb_apres, "publications": [pub1, pub2], "remise": remise, "visibilites_des_symboles": visibilites,
                     "fin_index_source_entiere": fin1, "fin_index_effacement": fin2});
    std::fs::write(&sortie, serde_json::to_string_pretty(&doc).unwrap()).expect("écrire le relevé");
    eprintln!("[orphelins] relevé écrit : {}", sortie.to_string_lossy());
}

/// **Le relevé du jeu** : pour chaque requête et chaque forme (sans filtre, chemin, langage), les k
/// premiers de search_code et le temps de l'appel.
fn releve_du_jeu(backend: &rag3weaver::backend::Backend, jeu: &Value, k: usize) -> Vec<Value> {
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
    releve
}

/// **Les usages d'un nom, en liste triée** : (fichier, nom de l'appelant) de chaque usage rendu par
/// l'outil `usages` (format json), quel que soit son genre. Rend la liste et la réponse brute.
fn usages_de(backend: &rag3weaver::backend::Backend, nom: &str) -> (Vec<(String, String)>, Value) {
    let brut = backend.call("usages", json!({"name": nom, "format": "json", "limit": 200})).unwrap_or_else(|e| json!({"refus": e}));
    fn cueillir(v: &Value, sous_usages: bool, out: &mut Vec<(String, String)>) {
        match v {
            Value::Object(o) => {
                if sous_usages {
                    let chemin = ["path", "file_path", "repo_path"].iter().find_map(|k| o.get(*k).and_then(Value::as_str));
                    // Un usage est {user: {uuid, title, kind, path, line}, usage, …} (UsagesReport).
                    let qui = ["title", "name", "caller", "qualified_name"].iter().find_map(|k| o.get(*k).and_then(Value::as_str));
                    if let (Some(c), Some(n)) = (chemin, qui) {
                        out.push((c.to_string(), n.to_string()));
                    }
                }
                for (k, x) in o {
                    cueillir(x, sous_usages || k == "usages", out);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| cueillir(x, sous_usages, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    cueillir(&brut, false, &mut out);
    out.sort();
    out.dedup();
    (out, brut)
}

/// Les hash de contenu de tous les Scope, par (fichier, nom) : de quoi dire quels scopes un index a
/// changés.
fn hash_des_scopes(backend: &rag3weaver::backend::Backend) -> std::collections::BTreeMap<(String, String), std::collections::BTreeSet<String>> {
    let conn = backend.catalog.lock().unwrap().conn_arc();
    let r = conn.execute("MATCH (s:Scope) RETURN s.file_path, s.name, s._content_hash").expect("les hash des Scope");
    let mut m: std::collections::BTreeMap<(String, String), std::collections::BTreeSet<String>> = Default::default();
    for l in &r.rows {
        let t = |i: usize| l.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
        m.entry((t(0), t(1))).or_default().insert(t(2));
    }
    m
}

/// **Un index de la source, attendu jusqu'à sa dernière ligne** : « indexation terminée » dans le
/// journal que l'appel désigne. Rend les dernières lignes du journal.
fn indexer_et_attendre(backend: &rag3weaver::backend::Backend, etape: &str) -> String {
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
            eprintln!("[parité] index {etape} fini en {:.0} s", debut.elapsed().as_secs_f64());
            return lu.lines().rev().take(6).collect::<Vec<_>>().join("\n");
        }
        assert!(!lu.contains("indexation échouée"), "index {etape} : {lu}");
        assert!(debut.elapsed().as_secs() < 3600, "index {etape} : pas fini en une heure");
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// **La parité de l'écrivain des générations** (2a′-2, demande d'everything declarative, 15 nov.) :
/// une copie à générations, réindexée sur une source (le manifeste repointé sur un worktree à soi),
/// puis publiée là où le code sait publier, rend-elle les mêmes réponses qu'avant le lot ? Deux
/// relevés : avant l'index (`<sortie>.avant.json`) et après la publication (`<sortie>`), au rang
/// courant ; et les lignes `_generation`, dont `aretes_recopiees`. Les `_uuid` changent d'une
/// génération à l'autre : le comparateur juge les réponses (régime `reponses`).
#[test]
#[ignore]
fn releve_les_reponses_apres_une_reindexation_publiee() {
    let var = |n: &str| std::env::var_os(n);
    let (Some(base), Some(manifeste), Some(requetes), Some(sortie)) =
        (var("RAG3WEAVER_PARITE_BASE"), var("RAG3WEAVER_PARITE_MANIFESTE"), var("RAG3WEAVER_PARITE_REQUETES"), var("RAG3WEAVER_PARITE_SORTIE"))
    else {
        eprintln!("SAUTÉ releve_les_reponses_apres_une_reindexation_publiee : RAG3WEAVER_PARITE_BASE, _MANIFESTE, _REQUETES ou _SORTIE manque");
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
    let ecrire = |chemin: &str, releve: &[Value], extra: Value| {
        let mut doc = json!({"base": base.to_string_lossy(), "manifeste": manifeste.to_string_lossy(), "forme_de_la_base": forme, "k": k, "jeu": jeu, "releve": releve});
        if let (Some(d), Some(e)) = (doc.as_object_mut(), extra.as_object()) {
            d.extend(e.clone());
        }
        std::fs::write(chemin, serde_json::to_string_pretty(&doc).unwrap()).expect("écrire le relevé");
        eprintln!("[parité] relevé écrit : {chemin}");
    };
    let avant = releve_du_jeu(&backend, &jeu, k);
    ecrire(&format!("{}.avant.json", sortie.to_string_lossy()), &avant, json!({}));
    let hash_avant = hash_des_scopes(&backend);
    let fin_index = indexer_et_attendre(&backend, "source entière");
    // <publication>
    let publication = {
        let mut cat = backend.catalog.lock().unwrap();
        let r = cat.publier_la_generation("banc-835d5298f", 0);
        eprintln!("[parité] publication : {r:?} ; rang publié {:?}, en attente {:?}", cat.rang_publie(), cat.rang_en_attente());
        json!(format!("{r:?}"))
    };
    // </publication>
    let generations: Vec<String> = {
        let conn = backend.catalog.lock().unwrap().conn_arc();
        match conn.execute("MATCH (g:_generation) RETURN g ORDER BY g.rang") {
            Ok(r) => r.rows.iter().map(|l| format!("{:?}", l.first())).collect(),
            Err(e) => vec![format!("refus : {e}")],
        }
    };
    for g in &generations {
        eprintln!("[parité] génération {g}");
    }
    let apres = releve_du_jeu(&backend, &jeu, k);
    // **Les usages de noms dont le scope a changé** (au mot d'ED, 15 nov.) : cinq noms dont un scope
    // porte après l'index un contenu absent d'avant, et qui ont un appelant hors de leur fichier, ceux
    // d'`arbre.rs` d'abord. Le premier relevé (le parent) les choisit et les écrit dans
    // `RAG3WEAVER_PARITE_USAGES_NOMS` ; les suivants relisent ce fichier : les mêmes noms partout.
    let fichier_noms = std::env::var("RAG3WEAVER_PARITE_USAGES_NOMS").unwrap_or_else(|_| format!("{}.noms", sortie.to_string_lossy()));
    let noms: Vec<String> = match std::fs::read_to_string(&fichier_noms) {
        Ok(t) => t.lines().map(str::to_string).filter(|l| !l.is_empty()).collect(),
        Err(_) => {
            let hash_apres = hash_des_scopes(&backend);
            let mut changes: Vec<(String, String)> = hash_apres.iter()
                .filter(|(cle, h)| hash_avant.get(*cle).is_some_and(|a| h.iter().any(|x| !a.contains(x))))
                .map(|(cle, _)| cle.clone()).collect();
            changes.sort_by_key(|(f, n)| (!f.ends_with("arbre.rs"), f.clone(), n.clone()));
            eprintln!("[parité] scopes changés par l'index : {}", changes.len());
            let mut pris = Vec::new();
            for (f, n) in changes {
                if pris.len() == 5 || n.is_empty() || pris.contains(&n) {
                    continue;
                }
                let (u, _) = usages_de(&backend, &n);
                if u.iter().any(|(c, _)| !c.ends_with(&f) && !f.ends_with(c.as_str())) {
                    pris.push(n);
                }
            }
            std::fs::write(&fichier_noms, pris.join("\n") + "\n").expect("écrire les noms");
            pris
        }
    };
    let mut usages = serde_json::Map::new();
    for n in &noms {
        let (liste, brut) = usages_de(&backend, n);
        eprintln!("[parité] usages de {n} : {} appelants", liste.len());
        usages.insert(n.clone(), json!({"liste": liste, "brut": brut}));
    }
    ecrire(&sortie.to_string_lossy(), &apres, json!({"publication": publication, "generations": generations, "fin_index": fin_index, "usages": usages}));
}
