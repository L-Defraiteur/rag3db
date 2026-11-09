//! **Un symbole cité dans le chat mène à sa définition** (ticket « le code dans le chat
//! n'est ni coloré ni relié au dépôt », dernier point ; Lucie, 7 novembre 2026 : « genre
//! `code.truc` pris en compte » ; forme accordée avec everything declarative le 8
//! novembre). `POST /ide/symboles {backend, names}` dit, pour chaque nom cité, combien de
//! définitions l'index lui connaît et où est la première. Lu par `usages`, en lot, sans run
//! au journal ; le rendu ne fait jamais ouvrir un dépôt ni n'attend une indexation : un
//! dépôt fermé ou un catalogue tenu rendent la liste vide et disent pourquoi.

use serde_json::{json, Value};

/// Les noms lus par appel : une réponse du modèle en cite rarement plus.
const NOMS_MAX: usize = 32;

impl super::DaemonHost {
    /// **Les symboles cités, résolus par l'index** : pour chaque nom, `usages` (une
    /// définition au moins, la première rendue avec son chemin et sa ligne) ; un nom
    /// sans définition est absent. Ne fait jamais ouvrir le dépôt, ne fait jamais
    /// attendre une indexation (relecture d'everything declarative) : un dépôt fermé,
    /// un index partiel ou un catalogue tenu rendent `{symbols: {}, why}`. Le rendu
    /// passe par `Backend::call_discret`, pas par `/call` : aucun run au journal.
    pub fn symboles_cites(&self, backend_id: &str, noms: &[String]) -> Result<Value, String> {
        let hosted = self.hote_par_id(backend_id)?;
        let sans = |why: &str| Ok(json!({ "symbols": {}, "why": why }));
        let Ok(slot) = hosted.backend.try_read() else {
            return sans("le dépôt s'ouvre ou se ferme : les liens viendront au rendu suivant");
        };
        let Some(b) = slot.as_ref() else {
            return sans("le dépôt n'est pas ouvert dans l'hôte : les liens viendront quand il le sera");
        };
        // Pas de garde sur `indexings_running` : il compte aussi les vecteurs en fond,
        // quand les définitions sont prêtes depuis longtemps ; l'état partiel que
        // `usages` dit (ci-dessous) est la bonne mesure.
        // Une sonde, sans attendre : une écriture qui tient le catalogue ferait attendre
        // `usages` (la porte de lecture patiente jusqu'à READ_WAIT).
        match b.catalog.try_lock() {
            Ok(g) => drop(g),
            Err(_) => return sans("le catalogue est tenu par une écriture : les liens viendront au rendu suivant"),
        }
        let mut symboles = serde_json::Map::new();
        let mut premiere_erreur: Option<String> = None;
        for nom in noms.iter().take(NOMS_MAX) {
            if symboles.contains_key(nom) || nom.is_empty() {
                continue;
            }
            // Un appel discret : son run ne va ni au journal ni dans la barre.
            let v = match b.call_discret("usages", json!({ "name": nom, "limit": 1, "format": "json" })) {
                Ok(v) => v,
                // Une erreur du dépôt (l'outil absent, un index illisible) et non du nom :
                // la dire plutôt que taire tous les noms (relecture d'everything declarative).
                Err(e) if symboles.is_empty() && premiere_erreur.is_none() => {
                    premiere_erreur = Some(e);
                    continue;
                }
                Err(_) => continue,
            };
            // Un index partiel (`catalog_read::partial_status` : index ou relations en
            // cours, « résultat partiel ») peut ne compter qu'une partie des définitions :
            // aucun lien sur un compte faux. Le `status` de fraîcheur (« indexé à
            // l'instant ») n'en est pas un.
            if let Some(etat) = v["result"]["status"].as_str().filter(|s| s.contains("partiel")) {
                return sans(&format!("index partiel ({etat}) : les liens viendront à la fin de l'index"));
            }
            let definitions = [&v["result"]["result"]["definitions"], &v["result"]["definitions"], &v["definitions"]]
                .into_iter()
                .find_map(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let Some(premiere) = definitions.first() else { continue };
            symboles.insert(nom.clone(), json!({ "definitions": definitions.len(), "path": premiere["path"], "line": premiere["line"] }));
        }
        Ok(match premiere_erreur {
            Some(e) if symboles.is_empty() => json!({ "symbols": symboles, "why": format!("usages : {e}") }),
            _ => json!({ "symbols": symboles }),
        })
    }
}

#[cfg(all(test, feature = "code", feature = "rag3db-native"))]
pub(super) mod tests {
    use serde_json::{json, Value};
    use std::time::{Duration, Instant};

    /// Un petit dépôt de code : `un` défini une fois, `double` deux fois.
    pub(crate) fn petit_depot(d: &std::path::Path) -> (String, std::path::PathBuf) {
        let (base, arbre) = (d.join("base"), d.join("arbre"));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(arbre.join("src")).unwrap();
        std::fs::write(arbre.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n").unwrap();
        std::fs::write(arbre.join("src/a.rs"), "pub fn double() -> u32 {\n    2\n}\n").unwrap();
        std::fs::write(arbre.join("src/b.rs"), "pub fn double() -> u32 {\n    4\n}\n").unwrap();
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(arbre.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        (manifeste.to_string_lossy().to_string(), arbre)
    }

    fn poster(adresse: &str, route: &str, corps: Value) -> Value {
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("http://{adresse}{route}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    }

    /// L'id d'un backend dans l'état de l'hôte, par son manifeste.
    fn id_de(adresse: &str, manifeste: &str) -> (String, bool) {
        let mut r = crate::daemon::agent_de_temoin().get(format!("http://{adresse}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        let b = etat["backends"].as_array().and_then(|b| b.iter().find(|b| b["manifest"] == manifeste)).unwrap_or_else(|| panic!("{manifeste} n'est pas dans l'état : {etat}")).clone();
        (b["id"].as_str().unwrap().to_string(), b["open"] == true)
    }

    /// Indexer par l'IDE et attendre que `nom` soit défini.
    pub(crate) fn indexer_et_attendre(adresse: &str, manifeste: &str, nom: &str, definitions: usize) {
        let index = poster(adresse, "/ide/call", json!({ "manifest": manifeste, "name": "index", "arguments": { "confirm": true } }));
        assert_eq!(index["ok"], true, "{index}");
        let debut = Instant::now();
        loop {
            let u = poster(adresse, "/ide/call", json!({ "manifest": manifeste, "name": "usages", "arguments": { "name": nom, "format": "json" } }));
            if u["ok"] == true && u["result"]["result"]["result"]["definitions"].as_array().is_some_and(|d| d.len() >= definitions) && !u["result"]["result"]["status"].as_str().is_some_and(|s| s.contains("partiel")) {
                return;
            }
            assert!(debut.elapsed() < Duration::from_secs(60), "{nom} n'est pas indexé en 60 s : {u}");
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// **Par le chemin de l'hôte** : un nom défini une fois rend sa ligne, un nom défini
    /// deux fois en rend le compte, un nom inconnu est absent ; un dépôt que l'hôte
    /// n'a pas ouvert ne s'ouvre pas pour un rendu, et le dit.
    #[test]
    fn un_symbole_cite_se_resout_par_l_index_sans_ouvrir_de_depot() {
        let d = tempfile::tempdir().unwrap();
        let (manifeste, _) = petit_depot(&d.path().join("ouvert"));
        let (ferme, _) = petit_depot(&d.path().join("ferme"));
        let (adresse, fil) = crate::daemon_host::tests::hote_garde("j");
        indexer_et_attendre(&adresse, &manifeste, "double", 2);
        let (id, _) = id_de(&adresse, &manifeste);

        // Une écriture de fond (vecteurs en dette, relations) peut tenir le catalogue juste
        // après l'index, ou le laisser noté partiel : le rendu le dit et promet les liens au
        // rendu suivant, ou à la fin de l'index. La page
        // relit ; le témoin aussi, 10 s au plus (passe du 8 novembre 2026, une fois sur
        // deux seul sur luciepc).
        let debut = Instant::now();
        let r = loop {
            let r = poster(&adresse, "/ide/symboles", json!({ "backend": id, "names": ["un", "double", "inconnu_xyz"] }));
            if !r["result"]["why"].as_str().is_some_and(|w| w.contains("catalogue est tenu") || w.contains("index partiel")) || debut.elapsed() > Duration::from_secs(10) {
                break r;
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        assert_eq!(r["ok"], true, "{r}");
        let s = &r["result"]["symbols"];
        assert_eq!(s["un"], json!({ "definitions": 1, "path": "src/lib.rs", "line": 1 }), "{r}");
        assert_eq!(s["double"]["definitions"], 2, "{r}");
        assert!(s.get("inconnu_xyz").is_none(), "un nom sans définition est absent : {r}");

        // Un dépôt connu de l'hôte mais fermé : rien ne s'ouvre, la raison est dite.
        let decrit = poster(&adresse, "/ide/describe", json!({ "manifest": ferme }));
        assert_eq!(decrit["ok"], true, "{decrit}");
        let mut s = crate::daemon::agent_de_temoin().post(format!("http://{adresse}/session")).header("Authorization", "Bearer j").header("content-type", "application/json").send(json!({ "client": "le témoin" }).to_string()).unwrap();
        let session: Value = serde_json::from_str(&s.body_mut().read_to_string().unwrap()).unwrap();
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("http://{adresse}/backend/close"))
            .header("Authorization", "Bearer j")
            .header("X-Rag3weaver-Session", session["session"].as_str().unwrap_or_default())
            .header("content-type", "application/json")
            .send(json!({ "manifest": ferme }).to_string())
            .unwrap();
        let clos: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(clos["ok"], true, "{clos}");
        let (id_ferme, ouvert) = id_de(&adresse, &ferme);
        assert!(!ouvert, "le dépôt est fermé");
        let r = poster(&adresse, "/ide/symboles", json!({ "backend": id_ferme, "names": ["un"] }));
        assert_eq!(r["result"]["symbols"], json!({}), "{r}");
        assert!(r["result"]["why"].as_str().unwrap_or_default().contains("pas ouvert"), "{r}");
        assert!(!id_de(&adresse, &ferme).1, "le rendu a fait ouvrir le dépôt");

        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("http://{adresse}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **Un catalogue tenu ne fait pas attendre le rendu** (relecture d'everything
    /// declarative) : pendant qu'une écriture tient le catalogue, la résolution rend tout
    /// de suite, sans nom, et dit pourquoi.
    #[test]
    fn un_catalogue_tenu_rend_tout_de_suite_sans_attendre() {
        let d = tempfile::tempdir().unwrap();
        let (manifeste, _) = petit_depot(d.path());
        let host = std::sync::Arc::new(crate::daemon_host::tests::hote_isole());
        let h = host.hosted(std::path::Path::new(&manifeste)).unwrap();
        host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
        let debut = Instant::now();
        loop {
            let r = host.symboles_cites(&h.id, &["un".to_string()]).unwrap();
            if !r["symbols"]["un"].is_null() {
                break;
            }
            // Ce que le rendu disait en dernier : un catalogue tenu, un index partiel, rien.
            assert!(debut.elapsed() < Duration::from_secs(60), "un n'est pas indexé en 60 s ; le dernier rendu : {r}");
            std::thread::sleep(Duration::from_millis(200));
        }
        let catalogue = host.with_backend(&h, |b| b.catalog.clone()).unwrap();
        let tenu = catalogue.lock().unwrap();
        let debut = Instant::now();
        let r = host.symboles_cites(&h.id, &["un".to_string()]).unwrap();
        assert!(debut.elapsed() < Duration::from_millis(500), "le rendu a attendu {:?}", debut.elapsed());
        assert_eq!(r["symbols"], json!({}), "{r}");
        assert!(r["why"].as_str().unwrap_or_default().contains("catalogue"), "{r}");
        drop(tenu);
    }

    /// **Un rendu n'écrit aucun run au journal** (relecture d'everything declarative :
    /// `Backend::call` publie RunStarted et RunFinished sur le bus du dépôt, que le
    /// journal suit) : résoudre trois noms ne laisse aucun run neuf dans le journal du
    /// dépôt, donc rien dans la barre ni dans ses compteurs.
    #[test]
    fn un_rendu_n_ecrit_aucun_run_au_journal() {
        let d = tempfile::tempdir().unwrap();
        let (manifeste, _) = petit_depot(&d.path().join("depot"));
        let host = std::sync::Arc::new(crate::daemon_host::tests::hote_isole().with_runs_root(d.path().join("runs")));
        let h = host.hosted(std::path::Path::new(&manifeste)).unwrap();
        host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
        let debut = Instant::now();
        while host.symboles_cites(&h.id, &["double".to_string()]).unwrap()["symbols"]["double"]["definitions"] != 2 {
            assert!(debut.elapsed() < Duration::from_secs(60), "double n'est pas indexé en 60 s");
            std::thread::sleep(Duration::from_millis(200));
        }
        // Le journal écrit un run à sa fin : on laisse passer ce qui courait.
        std::thread::sleep(Duration::from_millis(500));
        let avant: std::collections::BTreeSet<String> = host.journal.runs_de(&h.id).into_iter().collect();
        // Le catalogue tenu par une écriture de fond, ou l'index noté partiel : relire, comme
        // la page (10 s au plus).
        let debut = Instant::now();
        let r = loop {
            let r = host.symboles_cites(&h.id, &["un".to_string(), "double".to_string(), "inconnu_xyz".to_string()]).unwrap();
            if !r["why"].as_str().is_some_and(|w| w.contains("catalogue est tenu") || w.contains("index partiel")) || debut.elapsed() > Duration::from_secs(10) {
                break r;
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        assert_eq!(r["symbols"]["un"]["definitions"], 1, "{r}");
        std::thread::sleep(Duration::from_millis(500));
        let neufs: Vec<String> = host.journal.runs_de(&h.id).into_iter().filter(|x| !avant.contains(x)).collect();
        let usages: Vec<&String> = neufs
            .iter()
            .filter(|x| host.journal.lire(&h.id, x).is_some_and(|v| v.to_string().contains("\"usages\"")))
            .collect();
        assert!(usages.is_empty(), "le rendu a écrit {} run(s) usages au journal : {usages:?}", usages.len());
    }
}
