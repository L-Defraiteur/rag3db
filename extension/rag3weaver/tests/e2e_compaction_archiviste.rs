//! **La compaction d'un chat est l'archiviste** (page du 15 novembre 2026,
//! `docs/memoire/15-11-2026/03-la-compaction-est-l-archiviste.md`) : le témoin,
//! rouge d'abord, de la forme que Lucie a dite. Le code de l'hôte est à archi ;
//! ce témoin fixe le contrat.
//!
//! Le contrat, tel que le témoin le lit :
//! - **au fil de l'eau** : la fin de chaque tour de chat ferme une section ; l'hôte
//!   la passe en fond à l'archiviste (`archive` de la mémoire de `bilans.memoire`),
//!   et chaque L1 posé porte l'intervalle de tours qu'il couvre (section
//!   `turns <a>-<b>`, source un L0 `sources/…` de la transcription) ;
//! - **seul rédacteur** : le `ChatFile` note `archived_through_turn` (« rangé
//!   jusqu'au tour N ») ; aucun échange n'est jugé deux fois par le Decide ;
//! - **le seuil** : quand `used` du dernier appel dépasse `compaction.ratio` de
//!   `max` (défaut 0.8), la compaction se fait avant l'appel suivant ; la passe est
//!   bornée par `compaction.timeout_s` (défaut 120) ; les deux sont des réglages
//!   du manifeste, lus dans `chat_state()["context"]["compaction"]` ;
//! - **ce qu'elle rend** : un L1 `compaction`, et le bilan (section `bilan`) : une
//!   ligne par pas, dans l'ordre du run, chacune citant `fiche:<nom>` ;
//! - **le contexte d'après** : composé dans un budget, réglage par modèle
//!   (`models.llm.restart_budget_tokens`, défaut 100 000 pour un modèle cloud),
//!   par priorité : l'identité `agent:`, la fiche de tâche, le bilan ordonné, les
//!   L1 de la session (les siens en entier, dans l'ordre où ils ont été pris,
//!   la fiche de chacun en marge), puis ceux des autres sur les mêmes
//!   sujets, les plus récents d'abord ; ce qui ne tient pas reste au rappel. Le
//!   témoin compte les jetons par l'estimation du faux modèle (`llm::fragments`,
//!   ce que `MockLlm` rend en `prompt_tokens`) et refuse au-dessus du budget.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`) ; le
//! Decide de l'archiviste joue sur un faux serveur SSE, l'équipier sur un faux
//! modèle par fermeture.
#![cfg(all(feature = "rag3db-native", feature = "daemon", feature = "code", feature = "openai-llm"))]

use crate::common::fake_sse::{FakeServer, Reply};
use rag3weaver::daemon_host::DaemonHost;
use rag3weaver::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// Une étiquette `create`, un nom et une phrase, sous le schéma strict du Decide.
fn cree(nom: &str) -> Reply {
    Reply::Sse {
        frames: vec![
            format!(r#"{{"id":"a1","choices":[{{"index":0,"delta":{{"role":"assistant","content":"{{\"label\": \"create\", \"name\": \"{nom}\", \"about\": \"Ce que le chat a fait sur {nom}, pour le témoin.\"}}"}},"finish_reason":null}}]}}"#),
            r#"{"id":"a1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#.into(),
        ],
    }
}

const M1: &str = "MARQUEUN la reprise du journal relit chaque enregistrement";
const M2: &str = "MARQUEDEUX le compositeur fige à cent pour cent";
const M3: &str = "MARQUETROIS les vecteurs se rattrapent à la réouverture";

/// Ce que l'équipier a reçu à chaque appel, le texte de l'invite entière.
type Invites = Arc<Mutex<Vec<String>>>;

struct Banc {
    host: DaemonHost,
    memoire: PathBuf,
    id: String,
    chats: PathBuf,
    contexte: Arc<AtomicUsize>,
    invites: Invites,
    serveur: FakeServer,
}

/// Le banc : une mémoire dont le modèle est le faux serveur, un backend de code
/// qui déclare `bilans` vers elle (et, s'il le faut, un bloc `compaction`), un
/// hôte dont l'équipier a un contexte réglable pendant le test.
fn banc(d: &Path, compaction: Option<Value>, budget: Option<u64>) -> Banc {
    banc_ou_refus(d, compaction, budget, true).unwrap_or_else(|e| panic!("le backend de code s'héberge : {e}"))
}

/// Le banc, avec ou sans `bilans` ; rend le refus de l'hôte au premier appel s'il y en a un.
fn banc_ou_refus(d: &Path, compaction: Option<Value>, budget: Option<u64>, bilans: bool) -> Result<Banc, Value> {
    let gabarits = d.join("templates");
    copier(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits);
    let serveur = FakeServer::start_sequence((1..=12).map(|i| cree(&format!("sujet {i} du chat"))).collect());
    let memoire = gabarits.join("backends/memory/backend.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&memoire).unwrap()).unwrap();
    m["database"] = json!(d.join("memoire.rag3db").display().to_string());
    m["vector_extension"] = json!(crate::common::vector_extension());
    m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": serveur.url});
    std::fs::write(&memoire, serde_json::to_string_pretty(&m).unwrap()).unwrap();

    let arbre = d.join("code");
    std::fs::create_dir_all(&arbre).unwrap();
    std::fs::write(arbre.join("main.rs"), "fn main() {}\n").unwrap();
    let code = gabarits.join("backends/code/backend.json");
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&code).unwrap()).unwrap();
    c["database"] = json!(d.join("code.rag3db").display().to_string());
    c["workspace"]["root"] = json!(arbre.display().to_string());
    if bilans {
        c["bilans"] = json!({ "memoire": memoire.display().to_string() });
    }
    if let Some(v) = compaction {
        c["compaction"] = v;
    }
    if let Some(n) = budget {
        c["models"]["llm"]["restart_budget_tokens"] = json!(n);
    }
    std::fs::write(&code, serde_json::to_string_pretty(&c).unwrap()).unwrap();

    let contexte = Arc::new(AtomicUsize::new(10_000_000));
    let invites: Invites = Arc::default();
    let (ctx, vu) = (contexte.clone(), invites.clone());
    let modele: rag3weaver::daemon_host::chat::ChatModel = Arc::new(move |_| {
        let vu = vu.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", ctx.load(Ordering::SeqCst), move |turns, opts, sink| {
            vu.lock().unwrap().push(turns.iter().map(|t| t.content.as_str()).collect::<Vec<_>>().join("\n"));
            MockLlm::new("Compris.").with_context_len(usize::MAX).generate(turns, opts, sink)
        })) as Arc<dyn Llm>)
    });
    let chats = d.join("chats");
    let host = DaemonHost::new()
        .with_runs_root(d.join("runs"))
        .with_chats_root(chats.clone())
        .with_racine_des_taches(d.join("taches"))
        .with_chat_model(modele.clone())
        .with_juge_model(modele);
    let s = host.open_session("le témoin", None, Vec::new(), None);
    let lu = host.call(Some(&s), &code, "read_file", json!({ "path": "main.rs" }));
    if lu["ok"] != true {
        return Err(lu);
    }
    let canon = code.canonicalize().unwrap();
    let id = host.state()["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"].as_str().map(PathBuf::from).and_then(|p| p.canonicalize().ok()) == Some(canon.clone())))
        .unwrap_or_else(|| panic!("le backend de code n'est pas dans l'état : {}", host.state()))["id"]
        .as_str()
        .unwrap()
        .to_string();
    Ok(Banc { host, memoire, id, chats, contexte, invites, serveur })
}

impl Banc {
    fn memoire(&self, outil: &str, args: Value) -> Value {
        let s = self.host.open_session("le témoin", None, Vec::new(), None);
        self.host.call(Some(&s), &self.memoire, outil, args)
    }

    /// Les apports de la mémoire dont la section vaut `section`.
    fn apports(&self, section: &str) -> Vec<Value> {
        let r = self.memoire("recall_apports", json!({"query": "chat tours", "options": {"limit": 50, "consistency": "strict",
            "filter_condition": {"field": {"key": "section", "value": section}}}}));
        r["result"]["result"].as_array().or(r["result"].as_array()).cloned().unwrap_or_default().into_iter()
            .map(|x| if x["data"].is_object() { x["data"].clone() } else { x }).collect()
    }

    /// `archived_through_turn` du `ChatFile`, attendu jusqu'à `n` (le rangement est en fond).
    fn range_jusqu_a(&self, n: u64) -> Value {
        let chemin = self.chats.join(format!("{}.json", self.id));
        let fin = Instant::now() + Duration::from_secs(90);
        loop {
            let f: Value = std::fs::read(&chemin).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
            if f["archived_through_turn"].as_u64().is_some_and(|k| k >= n) {
                return f;
            }
            assert!(Instant::now() < fin, "le ChatFile ne dit jamais « rangé jusqu'au tour {n} » (archived_through_turn) : {}", f["archived_through_turn"]);
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Les corps reçus par le faux serveur du Decide depuis le dernier relevé.
    fn decides(&self, tous: &mut Vec<String>) {
        while let Ok(r) = self.serveur.request.try_recv() {
            tous.push(r);
        }
    }
}

/// **Le rangement au fil de l'eau, puis la compaction au seuil** : chaque tour
/// est rangé une fois, avec son intervalle ; au seuil, seul le non couvert passe
/// par le Decide ; le contexte d'après repart de l'identité, du bilan et des L1.
#[test]
#[ignore]
fn le_chat_se_range_au_fil_de_l_eau_et_se_compacte_par_l_archiviste_au_seuil() {
    let d = tempfile::tempdir().unwrap();
    // Le budget du contexte d'après : il tient l'identité, le bilan et les deux
    // premiers L1, pas le long tour 3 (qui reste au rappel). Réglé après le tour 2.
    const BUDGET: u64 = 8_000;
    const REMPLISSAGE: usize = 20_000;
    let b = banc(d.path(), None, Some(BUDGET));
    let mut decides = Vec::new();

    // Deux tours sous le seuil : chacun rangé, en fond, une fois.
    b.host.chat_turn(&b.id, &format!("Note ceci : {M1}.")).expect("le tour 1");
    b.range_jusqu_a(1);
    b.host.chat_turn(&b.id, &format!("Et ceci : {M2}.")).expect("le tour 2");
    b.range_jusqu_a(2);
    b.decides(&mut decides);
    let couverts: Vec<Value> = b.apports("turns 1-1").into_iter().chain(b.apports("turns 2-2")).collect();
    assert_eq!(couverts.len(), 2, "un L1 par tour rangé, sa section l'intervalle de tours : {couverts:?}");
    for a in &couverts {
        assert!(a["source"].as_str().is_some_and(|s| s.starts_with("sources/")), "le L1 cite le L0 de la transcription : {a}");
    }

    // Le tour 3 remplit le contexte au-delà de 80 % (le défaut) : la jauge le dit.
    let used2 = b.host.chat_state(&b.id).unwrap()["context"]["used"].as_u64().expect("la jauge après le tour 2") as usize;
    assert!((used2 as u64) + 1_500 < BUDGET, "le banc : les deux premiers tours tiennent dans le budget avec de la marge ({used2}/{BUDGET})");
    b.contexte.store((used2 + REMPLISSAGE) * 11 / 10, Ordering::SeqCst);
    let long = format!("Lis ce journal. {M3}. {}", "le journal continue ".repeat(REMPLISSAGE / 3));
    b.host.chat_turn(&b.id, &long).expect("le tour 3");
    let etat = b.host.chat_state(&b.id).unwrap();
    let (used3, max) = (etat["context"]["used"].as_u64().unwrap(), etat["context"]["max"].as_u64().unwrap());
    assert!(used3 * 10 > max * 8, "le banc place le tour 3 au-dessus du seuil : {used3}/{max}");

    // Le tour 4 : la compaction se fait avant son appel au modèle.
    let avant = b.invites.lock().unwrap().len();
    b.host.chat_turn(&b.id, "Et maintenant, où en est-on ?").expect("le tour 4");
    let f = b.range_jusqu_a(3);
    b.decides(&mut decides);

    // Aucun échange jugé deux fois : chaque marque dans un seul Decide.
    for m in [M1, M2, M3] {
        let mot = m.split(' ').next().unwrap();
        let n = decides.iter().filter(|r| r.contains(mot)).count();
        assert_eq!(n, 1, "« {mot} » passe une fois et une seule par le Decide (déjà rangé, pas rangé deux fois) ; {} Decide reçus", decides.len());
    }

    // Ce que la compaction rend : le L1 « compaction », le bilan ordonné.
    assert!(!b.apports("compaction").is_empty(), "un L1 « compaction » ; ChatFile : {}", f["archived_through_turn"]);
    let bilan = b.apports("bilan");
    let texte = bilan.iter().filter_map(|a| a["texte"].as_str()).collect::<Vec<_>>().join("\n");
    let pas: Vec<&str> = texte.lines().filter(|l| l.contains("fiche:")).collect();
    assert!(pas.len() >= 3, "le bilan : une ligne par pas, chacune citant fiche:<nom> : {texte:?}");
    let ou = |n: &str| texte.find(n).unwrap_or(usize::MAX);
    assert!(ou("sujet 1 du chat") < ou("sujet 2 du chat") && ou("sujet 2 du chat") < ou("sujet 3 du chat") && ou("sujet 3 du chat") < usize::MAX,
        "le bilan suit l'ordre du run : {texte:?}");

    // Le contexte d'après : identité, bilan, L1 des tours 1 et 2 ; le long tour 3
    // ne tient pas dans le budget et reste au rappel ; sous le budget et le seuil.
    let invites = b.invites.lock().unwrap();
    let apres = invites.get(avant).expect("le tour 4 a appelé l'équipier");
    assert!(apres.contains("agent:"), "l'identité de l'agent est dans l'invite d'après");
    assert!(apres.contains("sujet 1 du chat") && apres.contains("sujet 2 du chat"), "les L1 qui tiennent sont dans l'invite d'après");
    assert!(apres.contains("MARQUEUN") && apres.contains("MARQUEDEUX"), "les L1 de la session y sont en entier");
    // Les L1 dans l'ordre où ils ont été pris (l'ordre des intervalles de tours),
    // pas groupés par fiche ; la fiche de chaque L1 se dit en marge (sur sa ligne
    // ou la ligne d'avant).
    let (u, deux) = (apres.find("MARQUEUN").unwrap(), apres.find("MARQUEDEUX").unwrap());
    assert!(u < deux, "les L1 suivent l'ordre de la session : le tour 1 avant le tour 2");
    let lignes: Vec<&str> = apres.lines().collect();
    for (marque, fiche) in [("MARQUEUN", "sujet 1 du chat"), ("MARQUEDEUX", "sujet 2 du chat")] {
        let i = lignes.iter().position(|l| l.contains(marque)).unwrap();
        let marge = lignes[i.saturating_sub(1)..=i].join("\n");
        assert!(marge.contains(fiche), "le L1 de « {marque} » dit sa fiche « {fiche} » en marge : {marge:?}");
    }
    assert!(apres.contains("fiche:"), "le bilan est dans l'invite d'après");
    assert!(!apres.contains("le journal continue le journal continue"), "ce qui ne tient pas dans le budget reste au rappel, pas dans l'invite d'après");
    let jetons = rag3weaver::llm::fragments(apres).len() as u64;
    assert!(jetons <= BUDGET, "l'invite d'après tient dans le budget déclaré (estimation par fragments) : {jetons}/{BUDGET}");
    assert!(jetons * 10 <= max * 8, "l'invite d'après est sous le seuil : {jetons}/{max}");
}

/// **Le seuil, la borne et le budget sont des réglages du manifeste**, avec leurs défauts.
#[test]
#[ignore]
fn le_seuil_la_borne_et_le_budget_de_la_compaction_sont_des_reglages_du_manifeste() {
    let d = tempfile::tempdir().unwrap();
    let b = banc(&d.path().join("defauts"), None, None);
    let c = &b.host.chat_state(&b.id).unwrap()["context"]["compaction"];
    assert_eq!(c["ratio"], json!(0.8), "le seuil par défaut : {c}");
    assert_eq!(c["timeout_s"], json!(120), "la borne par défaut : {c}");
    assert!(c["budget_tokens"].as_u64().is_some_and(|n| n > 0 && n <= 100_000), "le budget du contexte d'après, 100 000 au plus par défaut : {c}");
    let b = banc(&d.path().join("declares"), Some(json!({"ratio": 0.5, "timeout_s": 30})), Some(40_000));
    let c = &b.host.chat_state(&b.id).unwrap()["context"]["compaction"];
    assert_eq!(c, &json!({"ratio": 0.5, "timeout_s": 30, "budget_tokens": 40_000}), "les réglages déclarés dans le manifeste");
}

/// **Sans `bilans`, pas de compaction par l'archiviste** : le backend garde la
/// réduction aux renvois seule, et `chat_state` le dit, avec la raison.
#[test]
#[ignore]
fn sans_bilans_la_compaction_par_l_archiviste_est_absente_et_dite() {
    let d = tempfile::tempdir().unwrap();
    let b = banc_ou_refus(d.path(), None, None, false).unwrap_or_else(|e| panic!("le backend sans bilans s'héberge : {e}"));
    let ctx = &b.host.chat_state(&b.id).unwrap()["context"];
    assert!(ctx["compaction"].is_null(), "sans bilans, context.compaction est null : {ctx}");
    assert!(ctx["compaction_unavailable"].as_str().is_some_and(|r| r.contains("bilans")), "et la raison est dite (compaction_unavailable, qui nomme bilans) : {ctx}");
}

/// **Un réglage hors bornes est refusé à la lecture** : un ratio hors de ]0, 1[,
/// un `timeout_s` nul ; le refus nomme le champ (`compaction.ratio`, `compaction.timeout_s`).
#[test]
#[ignore]
fn un_reglage_de_compaction_hors_bornes_est_refuse_a_la_lecture() {
    let d = tempfile::tempdir().unwrap();
    for (i, (bloc, champ)) in [
        (json!({"ratio": 1.5, "timeout_s": 120}), "ratio"),
        (json!({"ratio": 0.0, "timeout_s": 120}), "ratio"),
        (json!({"ratio": 0.8, "timeout_s": 0}), "timeout_s"),
    ].into_iter().enumerate() {
        match banc_ou_refus(&d.path().join(format!("cas-{i}")), Some(bloc.clone()), None, true) {
            Ok(_) => panic!("le bloc {bloc} est accepté ; il doit être refusé à la lecture"),
            Err(refus) => {
                let texte = refus.to_string();
                assert!(texte.contains(&format!("compaction.{champ}")) && !texte.contains("unknown field"),
                    "le refus de {bloc} nomme compaction.{champ}, le bloc étant connu : {refus}");
            }
        }
    }
}
