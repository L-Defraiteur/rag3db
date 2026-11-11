//! Parcours contextuel réel : mêmes chemins et symboles, deux dépôts distincts.
//! Le faux modèle choisit l'outil déclaré ; ni résultat ni lien ne sont simulés.

use super::atelier_fichiers::tests::{atelier_avec, attendre_l_index, Atelier};
use super::chat::ChatModel;
use super::navigateur::{navigateur_ou_saut, Navigateur};
use crate::llm::{CallbackLlm, Llm, LlmError, MockLlm, Turn};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const A: &str = "CONTENU_A_PREUVE";
const B: &str = "CONTENU_B_PREUVE";
const PRETE: &str = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";

type Cibles = Arc<Mutex<HashMap<String, String>>>;
type Retours = Arc<Mutex<HashMap<(String, String), Value>>>;

fn depot(racine: &Path, nom: &str, marque: &str) -> PathBuf {
    let depot = racine.join(nom);
    std::fs::create_dir_all(depot.join("src")).unwrap();
    let contenu = format!("pub fn repere() -> &'static str {{\n    \"{marque}\"\n}}\n");
    std::fs::write(depot.join("src/lib.rs"), contenu).unwrap();
    for args in [
        &["init", "-q", "-b", "essai"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "-A"],
        &["commit", "-q", "-m", "le dépôt du parcours"],
    ] {
        super::atelier_fichiers::tests::git(&depot, args);
    }
    depot
}

fn attacher(a: &Atelier, depot: &Path) -> String {
    a.appeler("attach_worktree", json!({ "path": depot })).expect("attacher le vrai dépôt");
    attendre_l_index(a, depot);
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let m = a.host.racines_de_code().unwrap().manifest(&nom).canonicalize().unwrap();
    a.host.hosted(&m).unwrap().id.clone()
}

/// Le même transport que les autres témoins ; fermeture même en cas de rouge.
struct Serveur {
    adresse: String,
    fil: Option<std::thread::JoinHandle<()>>,
}

impl Serveur {
    fn lancer(a: &Atelier) -> Self {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
        let host = a.host.clone();
        let fil = std::thread::spawn(move || super::DaemonServer::new(server, 2).serve(host, Some("j".into())));
        Self { adresse, fil: Some(fil) }
    }

    fn racine(&self) -> String {
        format!("http://{}", self.adresse)
    }

    fn arreter(&mut self) {
        let (statut, _) = super::tests::demander("POST", &format!("{}/arret", self.racine()), Some("j"));
        assert_eq!(statut, 200);
        self.fil.take().unwrap().join().unwrap();
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        if let Some(fil) = self.fil.take() {
            let ferme = crate::daemon::agent_with_timeout(Duration::from_secs(5))
                .post(format!("{}/arret", self.racine()))
                .header("Authorization", "Bearer j")
                .header("content-type", "application/json")
                .send("{}");
            if ferme.is_ok() {
                let _ = fil.join();
            }
        }
    }
}

/// Le contexte autoritaire voyage hors du texte borné du modèle. Aucun
/// repli sur une présentation, une citation ou la conversation.
fn invocation(tour: &Turn) -> Value {
    tour.metadata.as_ref().and_then(|m| m.get("invocation")).cloned().unwrap_or(Value::Null)
}

fn modele(cibles: Cibles, exposes: Arc<Mutex<Vec<Value>>>, retours: Retours) -> ChatModel {
    Arc::new(move |_| -> Result<Arc<dyn Llm>, String> {
        let (cibles, exposes, retours) = (cibles.clone(), exposes.clone(), retours.clone());
        Ok(Arc::new(CallbackLlm::new("faux-parcours-contextuel", 8192, move |turns: &[Turn], opts, sink| {
            exposes
                .lock()
                .unwrap()
                .push(json!(opts.tools.iter().map(|t| json!({ "name": t.name, "schema": t.parameters })).collect::<Vec<_>>()));
            for nom in ["search_code", "outline"] {
                if !opts.tools.iter().any(|t| t.name == nom) {
                    return Err(LlmError::Model(format!("outil contextuel non exposé : {nom}")));
                }
            }
            let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
            let demande = &turns[depuis].content;
            let lettre = if demande.contains("dépôt B") {
                "B"
            } else if demande.contains("dépôt A") {
                "A"
            } else {
                return Err(LlmError::Model("la demande ne choisit pas de dépôt".into()));
            };
            let cible = cibles.lock().unwrap().get(lettre).cloned().expect("id observé après l'attachement");
            let faits: Vec<_> = turns[depuis..].iter().filter(|t| t.role == "tool").collect();
            if let Some(dernier) = faits.last() {
                eprintln!(
                    "reçu réel {} : {} ; metadata {:?}",
                    dernier.tool_name.as_deref().unwrap_or("?"),
                    dernier.content,
                    dernier.metadata
                );
                let provenance = invocation(dernier);
                if provenance.is_null() {
                    return Err(LlmError::Model(format!("reçu sans invocation structurée : {}", dernier.content)));
                }
                let nom = dernier.tool_name.clone().expect("nom du véritable outil exécuté");
                retours.lock().unwrap().insert((lettre.to_string(), nom), provenance);
            }
            let contexte = json!({ "target": { "kind": "backend", "id": cible } });
            let prochain = if !faits.iter().any(|t| t.tool_name.as_deref() == Some("search_code")) {
                Some(("search_code", json!({ "context": contexte, "query": "repere", "options": {} })))
            } else if !faits.iter().any(|t| t.tool_name.as_deref() == Some("outline")) {
                Some(("outline", json!({ "context": contexte, "path": "src/lib.rs" })))
            } else {
                None
            };
            match prochain {
                Some((nom, args)) => MockLlm::new("").with_tool_calls(vec![(nom.to_string(), args.to_string())]).generate(turns, opts, sink),
                None => MockLlm::new("Cible consultée.").generate(turns, opts, sink),
            }
        })))
    })
}

#[test]
#[cfg_attr(
    not(feature = "navigateur"),
    ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)"
)]
fn le_relais_du_chat_ouvre_la_cible_effective_et_la_garde_apres_rechargement() {
    let Some(binaire) = navigateur_ou_saut("le_relais_du_chat_ouvre_la_cible_effective_et_la_garde_apres_rechargement") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let cibles: Cibles = Arc::new(Mutex::new(HashMap::new()));
    let exposes = Arc::new(Mutex::new(Vec::new()));
    let retours: Retours = Arc::new(Mutex::new(HashMap::new()));
    let a = atelier_avec(d.path(), modele(cibles.clone(), exposes.clone(), retours.clone()));
    let da = depot(&a.racine, "depot-a", A);
    let db = depot(&a.racine, "depot-b", B);
    let ida = attacher(&a, &da);
    let idb = attacher(&a, &db);
    assert_ne!(ida, idb, "deux occurrences distinctes, même chemin et même symbole");
    cibles.lock().unwrap().extend([("A".into(), ida.clone()), ("B".into(), idb.clone())]);
    let mut serveur = Serveur::lancer(&a);
    for m in [&da, &db] {
        let nom = crate::code_tree::name_of_tree(&m.canonicalize().unwrap()).unwrap();
        let manifeste = a.host.racines_de_code().unwrap().manifest(&nom);
        super::symboles_cites::tests::indexer_et_attendre(&serveur.adresse, manifeste.to_str().unwrap(), "repere", 1);
        attendre_l_index(&a, m);
    }
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{}/ide/entrer?jeton=j", serveur.racine()), "location.pathname.startsWith('/ide/')");
    let chat = format!("{}/ide/code/{}/chat", serveur.racine(), a.h.id);
    // C'est le lien produit par le vrai search_code, jamais un href ajouté par le témoin.
    let lien = "[...document.querySelectorAll('.outil[data-outil=search_code]')].at(-1)?.querySelector('a.code-ref[data-path=\"src/lib.rs\"]')";
    for (lettre, id, marque, autre) in [("B", &idb, B, A), ("A", &ida, A, B)] {
        n.aller(&chat, PRETE);
        let demande = serde_json::to_string(&format!("cherche repere puis son outline dans le dépôt {lettre}")).unwrap();
        n.evaluer(&format!("document.querySelector('#message').value = {demande}; document.querySelector('#send').click()"));
        n.attendre(
            "['fini', 'Erreur'].includes(document.querySelector('#status')?.textContent)",
            "le tour atteint une issue observable",
        );
        let schemas = exposes.lock().unwrap().last().cloned().expect("un appel modèle depuis le navigateur");
        for outil in ["search_code", "outline"] {
            let def = schemas.as_array().unwrap().iter().find(|t| t["name"] == outil);
            assert!(def.is_some(), "rouge d'exposition : {outil} n'atteint pas le modèle : {schemas}");
            assert!(def.unwrap()["schema"]["properties"]["context"].is_object(), "contexte structuré déclaré pour {outil}");
            let provenance = retours
                .lock()
                .unwrap()
                .get(&(lettre.to_string(), outil.to_string()))
                .cloned()
                .expect("un véritable reçu d'exécution");
            assert_eq!(provenance["target"]["kind"], "backend");
            assert_eq!(provenance["target"]["id"], id.as_str());
            assert!(provenance["target"]["label"].as_str().is_some_and(|s| !s.is_empty()));
            assert!(provenance["parent_run"].as_str().is_some_and(|s| !s.is_empty()));
            assert!(provenance["run"].as_str().is_some_and(|s| !s.is_empty()));
        }
        eprintln!("rendu réel : {}", n.evaluer("[...document.querySelectorAll('.outil')].map(e => e.outerHTML).join('\n')"));
        n.attendre(&format!("!!({lien})"), "un lien de fichier issu du résultat réel");
        let attendu = format!("/ide/code/{id}/fichier/src/lib.rs");
        let href = n.evaluer(&format!("({lien}).getAttribute('href')"));
        assert!(
            href.as_str().is_some_and(|h| h.starts_with(&attendu)),
            "le rendu doit utiliser la cible {id}, pas la conversation : {href}"
        );
        n.evaluer(&format!("({lien}).click()"));
        n.attendre(
            &format!(
                "location.pathname === {} && document.body.innerText.includes({}) && !document.body.innerText.includes({})",
                json!(attendu),
                json!(marque),
                json!(autre)
            ),
            "le clic ouvre le contenu du dépôt demandé",
        );
        let appels = exposes.lock().unwrap().len();
        n.aller(&chat, PRETE);
        n.attendre(&format!("!!({lien})"), "le résultat conservé après rechargement");
        assert_eq!(exposes.lock().unwrap().len(), appels, "recharger n'exécute pas à nouveau les outils");
        let apres = n.evaluer(&format!("({lien}).getAttribute('href')"));
        assert_eq!(apres, href, "le lien conserve son contexte effectif après relecture de session");
        n.evaluer(&format!("({lien}).click()"));
        n.attendre(
            &format!("location.pathname === {} && document.body.innerText.includes({})", json!(attendu), json!(marque)),
            "le clic reste correct après reprise",
        );
    }
    assert!(n.erreurs.is_empty(), "console : {:?}", n.erreurs);
    drop(n);
    serveur.arreter();
}
