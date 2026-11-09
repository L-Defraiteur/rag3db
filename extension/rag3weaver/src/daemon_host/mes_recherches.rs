//! **Mes recherches, avec les clics** (8 novembre 2026, décision de Lucie du
//! 11 octobre : « j'aimerais éviter qu'on n'ait pas de logs de ce que j'ai
//! cherché ») : les recherches faites dans l'IDE et par les agents, lues dans le
//! journal des runs, avec les résultats cliqués.

use super::navigateur::{navigateur_ou_saut, Navigateur};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn git(dir: &Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// **Une recherche de l'IDE, un clic, une recherche d'agent : « mes recherches »
/// les rend, par l'hôte et dans un vrai navigateur.**
///
/// 1. Un petit dépôt inscrit par `code-tree add`, indexé.
/// 2. Dans la vue recherche, « bonjour », puis un clic sur le premier résultat.
/// 3. Un agent (une session à lui) cherche « salut » par `/call`.
/// 4. `GET /ide/recherches` rend les deux, la plus récente d'abord : celle de
///    l'agent sous son nom, sans clic ; celle de l'IDE (`client: ide`) avec son
///    clic, sur le premier résultat. Le filtre `qui` (ide, agents) borne.
/// 5. Un clic sur un run inconnu, ou sur un run qui n'est pas une recherche
///    (l'index), est refusé en disant pourquoi : jamais avalé.
/// 6. La vue « mes recherches » montre les deux lignes, « ma recherche » et le
///    nom de l'agent, le résultat cliqué marqué ; le filtre « agents » n'en
///    laisse qu'une.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn mes_recherches_rendent_celles_de_l_ide_et_des_agents_avec_les_clics() {
    const TEMOIN: &str = "mes_recherches_rendent_celles_de_l_ide_et_des_agents_avec_les_clics";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let depot = d.path().join("petit-depot");
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src/lib.rs"), "pub fn bonjour() -> &'static str {\n    \"bonjour\"\n}\n\npub fn salut() -> &'static str {\n    \"salut\"\n}\n").unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "-A"],
        &["commit", "-q", "-m", "le petit dépôt"],
    ] {
        git(&depot, args);
    }
    let roots = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let gabarit = roots.config.join("code-lourd/templates");
    std::fs::create_dir_all(gabarit.parent().unwrap()).unwrap();
    crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarit, false).unwrap();
    let source = roots.manifest("code-lourd");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&source).unwrap()).unwrap();
    m["database"] = json!(roots.data.join("code-lourd.rag3db").display().to_string());
    std::fs::write(&source, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let manifeste = crate::code_tree::add_files(&roots, &depot, "code-lourd", Some("code-petit")).unwrap().manifest.canonicalize().unwrap();

    let runs = d.path().join("runs");
    let (adresse, _fil) = super::tests::hote_garde_avec("j", |h| h.with_runs_root(runs));
    let racine = format!("http://{adresse}");
    // Une réponse qui n'est pas du JSON (une route absente rend la page) se dit.
    let lire = |mut r: ureq::http::Response<ureq::Body>| -> Value {
        let texte = r.body_mut().read_to_string().unwrap();
        serde_json::from_str(&texte).unwrap_or_else(|_| json!({ "pas_du_json": texte.chars().take(200).collect::<String>() }))
    };
    let ide = |route: &str, corps: Option<Value>| -> Value {
        let a = crate::daemon::agent_de_temoin();
        let r = match corps {
            Some(c) => a
                .post(format!("{racine}{route}"))
                .header("Cookie", "rag3weaver_ide=j")
                .header("content-type", "application/json")
                .send(c.to_string())
                .unwrap(),
            None => a.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap(),
        };
        lire(r)
    };
    let appeler = |nom: &str, args: Value| -> Value { ide("/ide/call", Some(json!({ "manifest": manifeste, "name": nom, "arguments": args }))) };
    let session_agent = {
        let r = crate::daemon::agent_de_temoin()
            .post(format!("{racine}/session"))
            .header("Authorization", "Bearer j")
            .header("content-type", "application/json")
            .send(json!({ "client": "l'agent du témoin" }).to_string())
            .unwrap();
        lire(r)["session"].as_str().unwrap().to_string()
    };

    let index = appeler("index", json!({ "confirm": true }));
    assert_eq!(index["ok"], true, "{index}");
    let run_de_l_index = index["result"]["run"].as_str().unwrap_or_default().to_string();
    let debut = Instant::now();
    loop {
        let u = appeler("usages", json!({ "name": "bonjour", "format": "json" }));
        if u["ok"] == true && u["result"]["result"]["result"]["definitions"].as_array().is_some_and(|d| !d.is_empty()) {
            break;
        }
        assert!(debut.elapsed() < Duration::from_secs(60), "bonjour n'est pas indexé en 60 s : {u}");
        std::thread::sleep(Duration::from_millis(200));
    }
    let id = ide("/ide/etat", None)["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.to_string_lossy().as_ref()))
        .unwrap_or_else(|| panic!("le code tree n'est pas dans l'état"))["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. La recherche de l'IDE, et un clic sur son premier résultat.
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{racine}/ide/code/{id}/recherche?q=bonjour"), "document.querySelector('.resultat a')");
    n.evaluer("document.querySelector('.resultat a').click()");
    n.attendre("location.pathname.includes('/fichier/')", "le fichier du résultat cliqué");

    // 3. La recherche d'un agent, par sa session.
    let agent = {
        let r = crate::daemon::agent_de_temoin()
            .post(format!("{racine}/call"))
            .header("Authorization", "Bearer j")
            .header(super::SESSION_HEADER, &session_agent)
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste, "name": "search_code", "arguments": { "query": "salut", "options": { "limit": 10 } } }).to_string())
            .unwrap();
        lire(r)
    };
    assert_eq!(agent["ok"], true, "{agent}");

    // 4. Les deux, lues par l'hôte (le journal s'écrit après la fin du run).
    let fin = Instant::now() + Duration::from_secs(20);
    let lignes = loop {
        let r = ide(&format!("/ide/recherches?backend={id}"), None);
        let l = r["result"]["recherches"].as_array().cloned().unwrap_or_default();
        let ide_a_son_clic = l.iter().any(|x| x["client"] == "ide" && x["clics"].as_array().is_some_and(|c| !c.is_empty()));
        if l.len() == 2 && ide_a_son_clic {
            break l;
        }
        assert!(Instant::now() < fin, "GET /ide/recherches ne rend pas les deux recherches, celle de l'IDE avec son clic : {r}");
        std::thread::sleep(Duration::from_millis(200));
    };
    let (de_l_agent, de_l_ide) = (&lignes[0], &lignes[1]);
    assert_eq!(de_l_agent["client"], "l'agent du témoin", "la plus récente d'abord : {lignes:?}");
    assert_eq!(de_l_agent["query"], "salut");
    assert_eq!(de_l_agent["outil"], "search_code");
    assert!(de_l_agent["clics"].as_array().is_none_or(|c| c.is_empty()), "{de_l_agent}");
    assert_eq!(de_l_ide["client"], "ide");
    assert_eq!(de_l_ide["query"], "bonjour");
    let premier = de_l_ide["resultats"][0].clone();
    assert!(premier["uuid"].is_string() && premier["chemin"] == "src/lib.rs", "le premier résultat, avec son chemin : {de_l_ide}");
    let clic = &de_l_ide["clics"][0];
    assert_eq!((&clic["uuid"], &clic["rang"]), (&premier["uuid"], &json!(1)), "le clic désigne le premier résultat : {de_l_ide}");
    for (qui, attendu) in [("ide", "bonjour"), ("agents", "salut")] {
        let r = ide(&format!("/ide/recherches?backend={id}&qui={qui}"), None);
        let q: Vec<&str> = r["result"]["recherches"].as_array().into_iter().flatten().filter_map(|x| x["query"].as_str()).collect();
        assert_eq!(q, vec![attendu], "le filtre {qui} : {r}");
    }

    // 5. Un clic qui ne désigne pas une recherche est refusé, et dit pourquoi.
    let inconnu = ide("/ide/recherches/clic", Some(json!({ "backend": id, "run": "run-inconnu", "uuid": premier["uuid"], "rang": 1 })));
    assert_eq!(inconnu["ok"], false, "{inconnu}");
    assert!(inconnu["error"].as_str().is_some_and(|e| e.contains("run-inconnu")), "{inconnu}");
    if !run_de_l_index.is_empty() {
        let pas_une_recherche = ide("/ide/recherches/clic", Some(json!({ "backend": id, "run": run_de_l_index, "uuid": premier["uuid"], "rang": 1 })));
        assert_eq!(pas_une_recherche["ok"], false, "{pas_une_recherche}");
        assert!(pas_une_recherche["error"].as_str().is_some_and(|e| e.contains("pas une recherche")), "{pas_une_recherche}");
    }

    // 6. La vue « mes recherches ».
    n.aller(
        &format!("{racine}/ide/code/{id}/recherches/"),
        "document.querySelectorAll('#recherches .recherche').length === 2",
    );
    let texte = n.evaluer("document.querySelector('#recherches').innerText").as_str().unwrap_or_default().to_string();
    assert!(texte.contains("ma recherche") && texte.contains("l'agent du témoin"), "la vue dit qui a cherché : {texte}");
    assert!(texte.contains("bonjour") && texte.contains("salut"), "{texte}");
    assert_eq!(
        n.evaluer("document.querySelectorAll('#recherches .recherche[data-client=\"ide\"] .clique').length"),
        json!(1),
        "le résultat cliqué est marqué"
    );
    n.evaluer("document.querySelector('#filtre-agents').click()");
    n.attendre("document.querySelectorAll('#recherches .recherche').length === 1 && document.querySelector('#recherches .recherche').dataset.client !== 'ide'", "le filtre agents");
}
