//! **Le banc des fiches classées** (page `docs/11-octobre-2026-memoire-longue/04-…`,
//! §5) : le sujet d'une fiche au `put_memory`, décidé par oui / non sur chaque
//! candidat que rend `recall_subjects`, **par le graphe d'une tentative datée**
//! (`templates/experiments/<date>-fiches-classees-<n>/`, un `Decide` par voie) :
//!
//! - **le Decider à un jeton** (`RAG3WEAVER_BANC_DECIDE=http://127.0.0.1:8091`,
//!   un llama-server local) — `decide_subject_decider.mmd`, une probabilité par
//!   étiquette, pour éprouver le protocole, pas pour juger ;
//! - **la sortie structurée** (`RAG3WEAVER_BANC_VERTEX=1` avec
//!   `RAG3WEAVER_LLM=vertex` et les identifiants de `.vault`) —
//!   `decide_subject_llm.mmd`, Gemini, qui juge.
//!
//! `RAG3WEAVER_BANC_TENTATIVE=<dossier>` choisit la tentative (par défaut la
//! première). Le jeu est `tests/data/banc-fiches-classees/v1.json` : des sujets,
//! des fiches, et pour chacune le sujet juste ou `null`. Le banc mesure le rappel
//! des candidats, les ancrages justes, les **faux ancrages**, les manqués, le
//! temps et (voie du Decider) les probabilités. Il n'affirme aucun seuil de
//! qualité ; il affirme le protocole : chaque décision rend une étiquette de la
//! liste, ou le banc tombe en le disant.
//!
//! `RAG3WEAVER_BANC_SORTIE=<fichier.json>` écrit chaque décision et le résumé,
//! pour la note de la tentative.

// Le banc décide par le Decider (feature `daemon`) ou par le LLM (feature
// `openai-llm`, qui ne va pas sans `daemon` dans les jeux joués) : sans `daemon`,
// la suite n'a rien à jouer et ne se compile pas (deny(warnings) sous
// `rag3db-native,code`, master rouge le 11 octobre 2026 à 30ce3aadc).
#![cfg(all(feature = "rag3db-native", feature = "daemon"))]

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// Combien de candidats la recherche rend : assez pour que le bon sujet y soit,
/// assez peu pour que chaque fiche coûte quelques décisions.
const CANDIDATS: usize = 3;


fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn jeu() -> Value {
    let chemin = crate_dir().join("tests/data/banc-fiches-classees/v1.json");
    serde_json::from_str(&std::fs::read_to_string(&chemin).expect("le jeu du banc")).expect("du JSON")
}

/// Le gabarit `memory` dans une copie de `templates/` (ses chemins sont
/// relatifs à son dossier), son extension vector prise au bâti (`RAG3DB_ROOT`)
/// comme le font les autres témoins — le gabarit la cherche relativement à
/// lui-même, défaut ouvert (`2026-10-11-un-gabarit-livre-cherche-l-extension-vector-…`).
fn ouvrir(dir: &Path) -> Backend {
    fn copier(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let cible = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copier(&e.path(), &cible);
            } else {
                std::fs::copy(e.path(), &cible).unwrap();
            }
        }
    }
    copier(&crate_dir().join("templates"), &dir.join("templates"));
    let manifeste = dir.join("templates/backends/memory/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    let racine = std::env::var("RAG3DB_ROOT").map(PathBuf::from).unwrap_or_else(|_| crate_dir().join("../.."));
    m["vector_extension"] = json!(racine.join("extension/vector/build/libvector.rag3db_extension").to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let prepared = PreparedBackend::load(&manifeste).expect("le gabarit memory charge");
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!(
            "le banc embarque (bge-m3) : posez RAG3WEAVER_EMBED_SERVICE — un banc sauté n'est \
             pas un banc vert ({e})"
        )
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("banc.rag3db")).expect("la base")), Some(embedder))
        .expect("l'ouverture")
}

fn question(sujet: &Value, fiche: &Value) -> String {
    format!(
        "Sujet : {} — {}\nFiche : {}\nPourquoi : {}\nCette fiche va-t-elle sous ce sujet ?",
        sujet["name"].as_str().unwrap_or(""),
        sujet["about"].as_str().unwrap_or(""),
        fiche["claim"].as_str().unwrap_or(""),
        fiche["why"].as_str().unwrap_or("")
    )
}

/// Ce qu'une décision rend.
struct Reponse {
    oui: bool,
    proba_verdict: Option<f64>,
}

/// **Une voie** : le graphe de la tentative pour elle, et les services qu'il
/// demande (le Decider, ou le LLM).
struct Voie {
    nom: String,
    graphe: rag3weaver::dataflow::checkpoint::GraphDefinition,
    services: std::sync::Arc<rag3weaver::dataflow::ServiceRegistry>,
}

fn tentative() -> PathBuf {
    std::env::var("RAG3WEAVER_BANC_TENTATIVE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate_dir().join("templates/experiments/2026-10-11-fiches-classees-1"))
}

impl Voie {
    fn nouvelle(nom: String, fichier: &str, services: rag3weaver::dataflow::ServiceRegistry) -> Self {
        let chemin = tentative().join(fichier);
        // Un graphe `.json` (une définition, où un retour à la ligne en est un)
        // passe avant le `.mmd` du même nom : retour de la tentative 1.
        let json = chemin.with_extension("json");
        let graphe = if json.exists() {
            let texte = std::fs::read_to_string(&json).unwrap_or_else(|e| panic!("{} : {e}", json.display()));
            serde_json::from_str(&texte).unwrap_or_else(|e| panic!("{} : {e}", json.display()))
        } else {
            let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
            rag3weaver::dataflow::parse_mermaid(&texte).unwrap_or_else(|e| panic!("{} : {e:?}", chemin.display()))
        };
        Self { nom, graphe, services: std::sync::Arc::new(services) }
    }

    /// Le graphe de la tentative, l'élément `{text}` posé devant son `decide`.
    fn decider(&self, question: &str) -> Result<Reponse, String> {
        use rag3weaver::dataflow::checkpoint::{EdgeDef, NodeDef};
        let mut def = self.graphe.clone();
        def.nodes.push(NodeDef {
            name: "source".into(),
            node_type: "RhaiNode".into(),
            config: json!({"script": "input", "value": {"text": question}}),
        });
        def.edges.push(EdgeDef {
            from_node: "source".into(),
            from_port: "result".into(),
            to_node: "decide".into(),
            to_port: "element".into(),
        });
        let mut registre = rag3weaver::dataflow::NodeRegistry::new();
        rag3weaver::dataflow::register_builtins(&mut registre);
        let mut g = rag3weaver::dataflow::DataflowGraph::from_definition(&def, &registre)?;
        let (sortie, _) = rag3weaver::dataflow::DataflowRuntime::with_services_arc(100, self.services.clone())
            .execute_with_report(&mut g)?;
        let element = sortie
            .get("decide", "element")
            .and_then(|v| v.downcast::<Value>())
            .cloned()
            .ok_or("le decide n'a rien rendu")?;
        let oui = match element["verdict"].as_str() {
            Some("yes") => true,
            Some("no") => false,
            autre => return Err(format!("étiquette hors liste : {autre:?} — {element}")),
        };
        Ok(Reponse { oui, proba_verdict: element["verdict_probability"].as_f64() })
    }
}

fn voies() -> Vec<Voie> {
    let mut v = Vec::new();
    #[cfg(feature = "daemon")]
    if let Ok(adresse) = std::env::var("RAG3WEAVER_BANC_DECIDE") {
        let decider = rag3weaver::decider::LlamaServerDecider::new(&adresse, "banc")
            .unwrap_or_else(|e| panic!("le Decider à {adresse} : {e}"));
        let mut s = rag3weaver::dataflow::ServiceRegistry::new();
        s.register(
            rag3weaver::decider::DECIDER_SERVICE,
            std::sync::Arc::new(decider) as std::sync::Arc<dyn rag3weaver::decider::Decider>,
        );
        v.push(Voie::nouvelle(format!("decider ({adresse})"), "decide_subject_decider.mmd", s));
    }
    #[cfg(feature = "openai-llm")]
    if std::env::var("RAG3WEAVER_BANC_VERTEX").is_ok() {
        let (llm, modele) = rag3weaver::regime::modele_agentique_nomme("banc-fiches-classees")
            .expect("RAG3WEAVER_BANC_VERTEX posé, mais pas de modèle : identifiants Vertex ?");
        let mut s = rag3weaver::dataflow::ServiceRegistry::new();
        s.register(
            rag3weaver::dataflow::LLM_SERVICE,
            std::sync::Arc::new(llm) as std::sync::Arc<dyn rag3weaver::llm::Llm>,
        );
        v.push(Voie::nouvelle(format!("llm ({modele})"), "decide_subject_llm.mmd", s));
    }
    v
}

#[derive(Default, Debug)]
struct Compte {
    decisions: usize,
    ancrages_justes: usize,
    faux_ancrages: usize,
    manques: usize,
    fiches_justes: usize,
    abstentions_justes: usize,
    ms: u128,
}

#[test]
#[ignore]
fn le_sujet_d_une_fiche_au_put_memory() {
    let voies = voies();
    assert!(
        !voies.is_empty(),
        "aucune voie : posez RAG3WEAVER_BANC_DECIDE (llama-server) et / ou RAG3WEAVER_BANC_VERTEX — \
         un banc sauté n'est pas un banc vert"
    );
    let jeu = jeu();
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(dir.path());
    let sujets = jeu["subjects"].as_array().expect("des sujets");
    for s in sujets {
        backend
            .call("put_subject", json!({"record": {"name": s["name"], "about": s["about"], "state": "current"}}))
            .expect("le sujet s'écrit");
    }
    let par_nom = |nom: &str| sujets.iter().find(|s| s["name"] == nom).cloned();

    // Les candidats, une fois pour toutes les voies : c'est la même recherche.
    let fiches = jeu["fiches"].as_array().expect("des fiches");
    let mut rappel = 0usize;
    let mut avec_sujet = 0usize;
    let mut candidats_par_fiche = Vec::new();
    let t = Instant::now();
    for f in fiches {
        let r = backend
            .call("recall_subjects", json!({"query": f["claim"], "options": {"limit": CANDIDATS}}))
            .expect("la recherche des sujets");
        let noms: Vec<String> = r["result"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["data"]["name"].as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        if let Some(gold) = f["gold"].as_str() {
            avec_sujet += 1;
            if noms.iter().any(|n| n == gold) {
                rappel += 1;
            }
        }
        candidats_par_fiche.push(noms);
    }
    let ms_recherche = t.elapsed().as_millis();
    eprintln!(
        "\n═══ banc des fiches classées : {} fiches, {} sujets, {CANDIDATS} candidats ═══",
        fiches.len(),
        sujets.len()
    );
    eprintln!(
        "  rappel des candidats : {rappel}/{avec_sujet} fiches à sujet ont leur sujet parmi les candidats \
         ({ms_recherche} ms pour {} recherches)",
        fiches.len()
    );

    let mut sortie = json!({"jeu": "v1", "candidats": CANDIDATS, "rappel": [rappel, avec_sujet], "voies": []});
    for voie in &voies {
        let mut c = Compte::default();
        let mut lignes = Vec::new();
        for (f, noms) in fiches.iter().zip(&candidats_par_fiche) {
            let gold = f["gold"].as_str();
            let mut oui = Vec::new();
            for nom in noms {
                let sujet = par_nom(nom).expect("un candidat est un sujet du jeu");
                let t = Instant::now();
                let r = voie
                    .decider(&question(&sujet, f))
                    .unwrap_or_else(|e| panic!("{} : la décision n'a pas rendu d'étiquette — {e}", voie.nom));
                c.ms += t.elapsed().as_millis();
                c.decisions += 1;
                let juste = Some(nom.as_str()) == gold;
                match (r.oui, juste) {
                    (true, true) => c.ancrages_justes += 1,
                    (true, false) => c.faux_ancrages += 1,
                    (false, true) => c.manques += 1,
                    (false, false) => {}
                }
                if r.oui {
                    oui.push(nom.clone());
                }
                lignes.push(json!({"claim": f["claim"], "candidat": nom, "gold": gold, "oui": r.oui, "proba_verdict": r.proba_verdict}));
            }
            let fiche_juste = match gold {
                Some(g) => oui == [g.to_string()],
                None => oui.is_empty(),
            };
            if fiche_juste {
                c.fiches_justes += 1;
                if gold.is_none() {
                    c.abstentions_justes += 1;
                }
            }
        }
        let sans_sujet = fiches.len() - avec_sujet;
        eprintln!("\n  ── {} ──", voie.nom);
        eprintln!(
            "  fiches justes {}/{} (abstentions justes {}/{sans_sujet}) ; ancrages justes {}, \
             FAUX ancrages {}, manqués {}",
            c.fiches_justes,
            fiches.len(),
            c.abstentions_justes,
            c.ancrages_justes,
            c.faux_ancrages,
            c.manques
        );
        eprintln!(
            "  {} décisions, {} ms ({} ms par décision)",
            c.decisions,
            c.ms,
            c.ms / c.decisions.max(1) as u128
        );
        sortie["voies"].as_array_mut().unwrap().push(json!({
            "voie": voie.nom, "tentative": tentative().file_name().map(|n| n.to_string_lossy().to_string()), "fiches": fiches.len(), "fiches_justes": c.fiches_justes,
            "abstentions_justes": c.abstentions_justes, "sans_sujet": sans_sujet,
            "ancrages_justes": c.ancrages_justes, "faux_ancrages": c.faux_ancrages, "manques": c.manques,
            "decisions": c.decisions, "ms": c.ms as u64,
            "lignes": lignes,
        }));
    }
    if let Ok(chemin) = std::env::var("RAG3WEAVER_BANC_SORTIE") {
        let chemin = PathBuf::from(chemin);
        std::fs::write(&chemin, serde_json::to_vec_pretty(&sortie).unwrap()).expect("la sortie du banc");
        eprintln!("\n  écrit : {}", chemin.display());
    }
}
