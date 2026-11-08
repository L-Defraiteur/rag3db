//! **Le terminal de l'agent de code, lot tty-1** (docs/07-novembre-2026-tty) :
//! le client `--plain` par le chemin de l'hôte. Un vrai hôte sur un port, avec
//! son jeton ; un équipier lancé par `POST /ide/teammate/mailbox` ; le flux SSE
//! lu par le client du terminal, coupé au milieu du run, repris par
//! `Last-Event-ID` — sans perte ni doublon, comptés contre une relecture
//! entière du tampon. Le faux modèle est injecté à l'hôte (aucun réseau).
#![cfg(all(feature = "rag3db-native", feature = "daemon", feature = "code"))]

use rag3weaver::daemon_host::{DaemonHost, DaemonServer};
use rag3weaver::llm::{CallbackLlm, Llm, MockLlm};
use rag3weaver_tty::{client, endpoint::Endpoint, flux, plain};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

fn git(dir: &Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

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

/// Un petit dépôt git étranger, inscrit par `code-tree add`.
fn depot_inscrit(d: &Path) -> PathBuf {
    let depot = d.join("petit-depot");
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src/lib.rs"), "pub fn bonjour() -> &'static str {\n    \"bonjour\"\n}\n").unwrap();
    std::fs::write(depot.join("README.md"), "# Petit dépôt\n").unwrap();
    for args in [&["init", "-q"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le petit dépôt"]] {
        git(&depot, args);
    }
    let roots = rag3weaver::code_tree::Roots { config: d.join("config"), data: d.join("data") };
    let gabarit = roots.config.join("code-lourd/templates");
    copier(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarit);
    let source = roots.manifest("code-lourd");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&source).unwrap()).unwrap();
    m["database"] = json!(roots.data.join("code-lourd.rag3db").display().to_string());
    std::fs::write(&source, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    rag3weaver::code_tree::add_files(&roots, &depot, "code-lourd", Some("code-petit")).unwrap().manifest.canonicalize().unwrap()
}

/// L'id d'un backend hébergé, par son manifeste.
fn heberger(host: &DaemonHost, manifeste: &Path) -> String {
    let s = host.open_session("le témoin", None, Vec::new(), None);
    let lu = host.call(Some(&s), manifeste, "list_files", json!({}));
    assert_eq!(lu["ok"], true, "le backend s'héberge : {lu}");
    let canon = manifeste.canonicalize().unwrap();
    host.state()["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"].as_str().map(PathBuf::from).and_then(|p| p.canonicalize().ok()) == Some(canon.clone())))
        .unwrap_or_else(|| panic!("le backend n'est pas dans l'état : {}", host.state()))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Les mots que dit le faux équipier dans son pas : assez pour couper au milieu.
const MOTS: usize = 300;

/// **Le faux équipier** : un pas qui parle (MOTS mots, un token chacun) et
/// déclare `step_done`, puis « Fait. » après le résultat de l'outil.
fn faux_equipier() -> rag3weaver::daemon_host::chat::ChatModel {
    Arc::new(|_b: &rag3weaver::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 1 << 20, |turns, opts, sink| {
            if turns.last().is_some_and(|t| t.role == "tool") {
                return MockLlm::new("Fait.").generate(turns, opts, sink);
            }
            let texte: Vec<String> = (0..MOTS).map(|i| format!("mot{i}")).collect();
            let args = json!({ "item": "", "succeeded": true, "done": "j'ai parlé", "remaining": "" }).to_string();
            MockLlm::new(texte.join(" ")).with_tool_calls(vec![("step_done", args.as_str())]).generate(turns, opts, sink)
        })))
    })
}

/// Lit le flux depuis `depuis` jusqu'à un silence de `silence` : les trames
/// `(id, enveloppe)`.
fn lire_jusqu_au_silence(ep: &Endpoint, depuis: Option<u64>, silence: Duration) -> Vec<(u64, Value)> {
    let arret = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let fil = {
        let (ep, arret) = (ep.clone(), arret.clone());
        std::thread::spawn(move || {
            flux::suivre(&ep, &["agent", "tokens", "mailbox"], depuis, &arret, &mut || {}, |t| {
                if let (Some(id), Ok(v)) = (t.id, serde_json::from_str::<Value>(&t.data)) {
                    let _ = tx.send((id, v));
                }
                flux::Suite::Continuer
            })
        })
    };
    let mut vues = Vec::new();
    while let Ok(x) = rx.recv_timeout(silence) {
        vues.push(x);
    }
    arret.store(true, Ordering::SeqCst);
    let (_, fin) = fil.join().unwrap().expect("le flux s'ouvre");
    assert_eq!(fin, flux::Fin::Arrete, "la lecture s'arrête à la demande");
    vues
}

fn tokens(trames: &[(u64, Value)]) -> Vec<(String, u64)> {
    trames
        .iter()
        .filter(|(_, v)| v["evenement"]["kind"] == "Token")
        .map(|(_, v)| (v["evenement"]["run"].as_str().unwrap_or_default().to_string(), v["evenement"]["seq"].as_u64().unwrap_or(u64::MAX)))
        .collect()
}

/// **Le flux coupé au milieu d'un run de l'équipier se reprend sans perte ni
/// doublon**, par le vrai chemin de l'hôte (HTTP, jeton, SSE).
#[test]
#[ignore]
fn le_flux_coupe_au_milieu_d_un_run_de_l_equipier_se_reprend_sans_perte_ni_doublon() {
    let d = tempfile::tempdir().unwrap();
    let manifeste = depot_inscrit(d.path());
    let host = Arc::new(
        DaemonHost::new()
            .with_background(Arc::new(rag3weaver::dataflow::background::Registry::new()))
            .with_racine_des_taches(d.path().join("taches"))
            .with_chats_root(d.path().join("chats"))
            .with_runs_root(d.path().join("runs"))
            .with_chat_model(faux_equipier()),
    );
    let parent = heberger(&host, &manifeste);
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = ecoute.local_addr().unwrap().to_string();
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    {
        let host = host.clone();
        std::thread::spawn(move || DaemonServer::new(server, 4).serve(host, Some("jeton-du-temoin".into())));
    }
    let ep = Endpoint { address, token: "jeton-du-temoin".into() };
    let agent = client::agent();

    // Un mauvais jeton est refusé avant le flux : le client le dit.
    let faux = Endpoint { token: "faux".into(), ..ep.clone() };
    let refus = flux::suivre(&faux, &["tokens"], None, &AtomicBool::new(false), &mut || {}, |_| flux::Suite::Continuer);
    assert!(refus.is_err(), "un faux jeton n'ouvre pas le flux : {refus:?}");

    // 1. Abonné sans curseur, la demande part une fois l'abonnement pris ; on
    //    coupe après 40 tokens, au milieu du pas.
    let mut premiers: Vec<(u64, Value)> = Vec::new();
    let mut lance = None;
    let mut ouvert = || {
        lance = Some(client::poster(&agent, &ep, "/ide/teammate/mailbox", &json!({ "backend": parent, "task": "parle", "kind": "request", "text": "Parle, puis déclare le pas." })));
    };
    let (dernier, fin) = flux::suivre(&ep, &["agent", "tokens", "mailbox"], None, &AtomicBool::new(false), &mut ouvert, |t| {
        if let (Some(id), Ok(v)) = (t.id, serde_json::from_str::<Value>(&t.data)) {
            premiers.push((id, v));
        }
        if tokens(&premiers).len() >= 40 {
            flux::Suite::Couper
        } else {
            flux::Suite::Continuer
        }
    })
    .expect("le flux s'ouvre avec le bon jeton");
    let lance = lance.expect("la demande est partie à l'ouverture").expect("la demande ouvre la tâche");
    assert_eq!(lance["started"], true, "{lance}");
    assert_eq!(fin, flux::Fin::Coupe);
    let dernier = dernier.expect("des trames avant la coupure");

    // 2. L'équipier finit pendant que personne n'écoute.
    let fin_au_plus_tard = Instant::now() + Duration::from_secs(300);
    let chemin = format!("/ide/teammate?backend={}&task=parle", client::encoder(&parent));
    let boite = loop {
        let v = client::lire(&agent, &ep, &chemin).expect("la boîte se lit");
        if v["running"] == false && v["mailbox"].as_array().is_some_and(|b| b.iter().any(|l| l["kind"] == "ended")) {
            break v["mailbox"].as_array().cloned().unwrap();
        }
        assert!(Instant::now() < fin_au_plus_tard, "l'équipier tourne encore après 5 min : {v}");
        std::thread::sleep(Duration::from_millis(200));
    };
    let ended: Vec<&Value> = boite.iter().filter(|l| l["kind"] == "ended").collect();
    assert_eq!(ended.len(), 1, "{boite:?}");

    // 3. La reprise au dernier id, et la relecture entière du même intervalle.
    let reprise = lire_jusqu_au_silence(&ep, Some(dernier), Duration::from_millis(1500));
    let premier = premiers.first().map(|(id, _)| *id).unwrap();
    let tout = lire_jusqu_au_silence(&ep, Some(premier - 1), Duration::from_millis(1500));

    let ids = |v: &[(u64, Value)]| v.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let mut recolle = ids(&premiers);
    recolle.extend(ids(&reprise));
    let sans_doublon: std::collections::BTreeSet<u64> = recolle.iter().copied().collect();
    eprintln!(
        "MESURE tty-1 : {} trames avant la coupure, {} après la reprise, {} dans la relecture ; {} tokens en tout ; fin « {} »",
        premiers.len(),
        reprise.len(),
        tout.len(),
        tokens(&tout).len(),
        ended[0]["text"].as_str().unwrap_or("")
    );
    assert_eq!(sans_doublon.len(), recolle.len(), "aucun doublon entre avant et après la coupure");
    assert!(recolle.windows(2).all(|w| w[0] < w[1]), "les ids montent sans retour en arrière");
    assert_eq!(recolle, ids(&tout), "avant + après = la relecture entière : aucune perte");
    let toks = tokens(&tout);
    assert!(toks.len() >= MOTS, "les {MOTS} mots du pas sont dans le flux : {}", toks.len());
    assert!(!tokens(&reprise).is_empty(), "la coupure tombe au milieu du pas : des tokens restent pour la reprise");
    for run in toks.iter().map(|(r, _)| r.clone()).collect::<std::collections::BTreeSet<_>>() {
        let seqs: Vec<u64> = toks.iter().filter(|(r, _)| *r == run).map(|(_, s)| *s).collect();
        assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1), "les seq du run {run} se suivent sans trou");
    }

    // 4. Le rendu `--plain` du recollé est celui de la relecture, et dit les mots.
    let rendre = |v: &[(u64, Value)]| v.iter().filter_map(|(_, e)| plain::trame(e)).collect::<String>();
    let mut recolle_texte = rendre(&premiers);
    recolle_texte.push_str(&rendre(&reprise));
    assert!(recolle_texte == rendre(&tout), "le texte recollé est le texte entier");
    assert!(recolle_texte.contains("mot0") && recolle_texte.contains(&format!("mot{}", MOTS - 1)), "les mots du pas sont réimprimés");
    assert!(recolle_texte.contains("» step_done"), "l'outil du pas se voit");
}

/// **Chaque outil dit ce qu'il a le droit de faire** : `describe` porte la
/// `policy` du manifeste, sur laquelle le terminal choisit son animation.
#[test]
#[ignore]
fn describe_porte_la_politique_de_chaque_outil() {
    let d = tempfile::tempdir().unwrap();
    let manifeste = depot_inscrit(d.path());
    let host = DaemonHost::new().with_background(Arc::new(rag3weaver::dataflow::background::Registry::new()));
    let s = host.open_session("le témoin", None, Vec::new(), None);
    let v = host.describe(Some(&s), &manifeste);
    let outils = v["result"]["tools"].as_array().or_else(|| v["tools"].as_array()).unwrap_or_else(|| panic!("describe sans outils : {v}"));
    let politique = |nom: &str| outils.iter().find(|t| t["name"] == nom).map(|t| t["policy"].clone()).unwrap_or_else(|| panic!("pas d'outil {nom}"));
    assert_eq!(politique("read_file")["write_files"], false, "{}", politique("read_file"));
    assert_eq!(politique("read_file")["read_files"], true, "{}", politique("read_file"));
    assert_eq!(politique("edit_file")["write_files"], true, "{}", politique("edit_file"));
}

/// **Le code se colore par le chemin de l'hôte** (le lot de coloration du
/// terminal) : `POST /highlight` sur un vrai port, avec son jeton, rend des spans
/// que le client du terminal lit ; une langue inconnue des grammaires ne rend
/// rien, et le bloc reste sans couleur.
#[test]
#[ignore]
fn le_code_d_un_bloc_se_colore_par_les_spans_de_l_hote() {
    let host = Arc::new(DaemonHost::new());
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = ecoute.local_addr().unwrap().to_string();
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    std::thread::spawn(move || DaemonServer::new(server, 2).serve(host, Some("jeton-du-temoin".into())));
    let ep = Endpoint { address, token: "jeton-du-temoin".into() };
    let agent = client::agent();
    let code = "fn main() {\n    // é\n    let x = \"chaîne\";\n}";
    let spans = client::surligner(&agent, &ep, code, "rust").expect("rust est connu des grammaires");
    let sorte = |mot: &str| {
        let a = code.find(mot).unwrap();
        spans.iter().find(|(d, f, _)| *d <= a && a + mot.len() <= *f).map(|(_, _, s)| s.clone())
    };
    eprintln!("MESURE coloration : {} spans pour {} octets", spans.len(), code.len());
    assert_eq!(sorte("fn").as_deref(), Some("keyword"), "{spans:?}");
    assert_eq!(sorte("// é").as_deref(), Some("comment"), "un commentaire après un caractère de deux octets : {spans:?}");
    assert_eq!(sorte("\"chaîne\"").as_deref(), Some("string"), "{spans:?}");
    assert!(spans.windows(2).all(|w| w[0].1 <= w[1].0), "triés et disjoints");
    assert_eq!(client::surligner(&agent, &ep, "x", "brainfuck"), None, "une langue inconnue : pas de couleur");
    let faux = Endpoint { token: "faux".into(), ..ep.clone() };
    assert_eq!(client::surligner(&agent, &faux, code, "rust"), None, "un faux jeton : rien, sans panique");
}
