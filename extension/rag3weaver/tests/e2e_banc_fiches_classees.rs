//! **Le banc des fiches classées** (page `docs/11-octobre-2026-memoire-longue/04-…`,
//! §5) : le sujet d'une fiche au `put_memory`, décidé par oui / non sur chaque
//! candidat que rend `recall_subjects`, par les deux voies du nœud `decide` à
//! venir :
//!
//! - **le Decider à un jeton** (`RAG3WEAVER_BANC_DECIDE=http://127.0.0.1:8091`,
//!   un llama-server local) — une probabilité par option, pour éprouver le
//!   protocole, pas pour juger ;
//! - **la sortie structurée** (`RAG3WEAVER_BANC_VERTEX=1` avec
//!   `RAG3WEAVER_LLM=vertex` et les identifiants de `.vault`) — Gemini, qui juge.
//!
//! Le jeu est `tests/data/banc-fiches-classees/v1.json` : des sujets, des fiches,
//! et pour chacune le sujet juste ou `null` quand aucun ne lui va. Le banc mesure
//! le rappel des candidats, les ancrages justes, les **faux ancrages** (un oui sur
//! un mauvais sujet, la faute qui défait un classeur), les ancrages manqués, le
//! temps et les jetons. Il n'affirme aucun seuil de qualité : il n'y en a pas
//! encore, et un seuil ne se pose pas sans mesure. Il affirme le protocole : chaque
//! décision rend une étiquette de la liste, ou le banc tombe en le disant.
//!
//! `RAG3WEAVER_BANC_SORTIE=<fichier.json>` écrit chaque décision et le résumé,
//! pour la note de la tentative.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// Combien de candidats la recherche rend : assez pour que le bon sujet y soit,
/// assez peu pour que chaque fiche coûte quelques décisions.
const CANDIDATS: usize = 3;

const CONSIGNE: &str = "Tu ranges des fiches de mémoire sous des sujets. Une fiche va sous un \
                        sujet seulement si elle parle de ce que le sujet décrit ; partager un mot \
                        ne suffit pas. Réponds yes ou no.";

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn jeu() -> Value {
    let chemin = crate_dir().join("tests/data/banc-fiches-classees/v1.json");
    serde_json::from_str(&std::fs::read_to_string(&chemin).expect("le jeu du banc")).expect("du JSON")
}

/// Le gabarit `memory` chargé **sur place** (ses chemins sont relatifs à son
/// dossier), sa base dans un dossier temporaire.
fn ouvrir(dir: &Path) -> Backend {
    let prepared = PreparedBackend::load(&crate_dir().join("templates/backends/memory/backend.json"))
        .expect("le gabarit memory charge");
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

/// Une voie : prend la question, rend « yes » ou « no », sa probabilité quand elle
/// la connaît, et ses jetons.
struct Reponse {
    oui: bool,
    proba_oui: Option<f64>,
    jetons: (usize, usize),
}

trait Voie {
    fn nom(&self) -> String;
    fn decider(&self, question: &str) -> Result<Reponse, String>;
}

#[cfg(feature = "daemon")]
struct VoieDecider {
    decider: rag3weaver::decider::LlamaServerDecider,
}

#[cfg(feature = "daemon")]
impl Voie for VoieDecider {
    fn nom(&self) -> String {
        "decider-un-jeton".into()
    }
    fn decider(&self, question: &str) -> Result<Reponse, String> {
        use rag3weaver::decider::Decider;
        // Le Decider complète un texte brut : le gabarit de conversation du
        // modèle s'écrit ici (ChatML, celui de Qwen), réflexion fermée.
        let prompt = format!(
            "<|im_start|>system\n{CONSIGNE}<|im_end|>\n<|im_start|>user\n{question}<|im_end|>\n\
             <|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        let p = self
            .decider
            .decide_at(&prompt, &["yes".to_string(), "no".to_string()], 1.0)
            .map_err(|e| format!("{e:?}"))?;
        Ok(Reponse { oui: p[0] > p[1], proba_oui: Some(p[0]), jetons: (0, 1) })
    }
}

#[cfg(feature = "openai-llm")]
struct VoieStructuree {
    llm: rag3weaver::openai_llm::OpenAiLlm,
    modele: String,
}

#[cfg(feature = "openai-llm")]
impl Voie for VoieStructuree {
    fn nom(&self) -> String {
        format!("sortie-structuree ({})", self.modele)
    }
    fn decider(&self, question: &str) -> Result<Reponse, String> {
        use rag3weaver::llm::{generate_to_string, GenOptions, ResponseFormat, Turn};
        let schema = json!({
            "type": "object",
            "properties": {"answer": {"type": "string", "enum": ["yes", "no"]}},
            "required": ["answer"],
            "additionalProperties": false
        });
        let opts = GenOptions::default()
            .with_max_tokens(1024)
            .with_response_format(ResponseFormat::strict_schema("decision", schema));
        let sortie = generate_to_string(&self.llm, &[Turn::system(CONSIGNE), Turn::user(question)], &opts)
            .map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(sortie.text.trim())
            .map_err(|e| format!("pas du JSON ({e}) : {}", sortie.text))?;
        let oui = match v["answer"].as_str() {
            Some("yes") => true,
            Some("no") => false,
            autre => return Err(format!("étiquette hors liste : {autre:?}")),
        };
        Ok(Reponse {
            oui,
            proba_oui: None,
            jetons: (sortie.usage.prompt_tokens, sortie.usage.completion_tokens),
        })
    }
}

fn voies() -> Vec<Box<dyn Voie>> {
    let mut v: Vec<Box<dyn Voie>> = Vec::new();
    #[cfg(feature = "daemon")]
    if let Ok(adresse) = std::env::var("RAG3WEAVER_BANC_DECIDE") {
        let decider = rag3weaver::decider::LlamaServerDecider::new(&adresse, "banc")
            .unwrap_or_else(|e| panic!("le Decider à {adresse} : {e}"));
        v.push(Box::new(VoieDecider { decider }));
    }
    #[cfg(feature = "openai-llm")]
    if std::env::var("RAG3WEAVER_BANC_VERTEX").is_ok() {
        let (llm, modele) = rag3weaver::regime::modele_agentique_nomme("banc-fiches-classees")
            .expect("RAG3WEAVER_BANC_VERTEX posé, mais pas de modèle : identifiants Vertex ?");
        v.push(Box::new(VoieStructuree { llm, modele }));
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
    jetons_entree: usize,
    jetons_sortie: usize,
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
                    .unwrap_or_else(|e| panic!("{} : la décision n'a pas rendu d'étiquette — {e}", voie.nom()));
                c.ms += t.elapsed().as_millis();
                c.decisions += 1;
                c.jetons_entree += r.jetons.0;
                c.jetons_sortie += r.jetons.1;
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
                lignes.push(json!({"claim": f["claim"], "candidat": nom, "gold": gold, "oui": r.oui, "proba_oui": r.proba_oui}));
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
        eprintln!("\n  ── {} ──", voie.nom());
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
            "  {} décisions, {} ms ({} ms par décision), jetons {} entrée / {} sortie",
            c.decisions,
            c.ms,
            c.ms / c.decisions.max(1) as u128,
            c.jetons_entree,
            c.jetons_sortie
        );
        sortie["voies"].as_array_mut().unwrap().push(json!({
            "voie": voie.nom(), "fiches": fiches.len(), "fiches_justes": c.fiches_justes,
            "abstentions_justes": c.abstentions_justes, "sans_sujet": sans_sujet,
            "ancrages_justes": c.ancrages_justes, "faux_ancrages": c.faux_ancrages, "manques": c.manques,
            "decisions": c.decisions, "ms": c.ms as u64,
            "jetons_entree": c.jetons_entree, "jetons_sortie": c.jetons_sortie,
            "lignes": lignes,
        }));
    }
    if let Ok(chemin) = std::env::var("RAG3WEAVER_BANC_SORTIE") {
        let chemin = PathBuf::from(chemin);
        std::fs::write(&chemin, serde_json::to_vec_pretty(&sortie).unwrap()).expect("la sortie du banc");
        eprintln!("\n  écrit : {}", chemin.display());
    }
}
