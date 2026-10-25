//! Les témoins du chat de l'hôte (page 11, K1), sur un faux modèle : la
//! tuyauterie sans réseau. Le témoin sur Gemini est K3.
use super::*;
use crate::llm::{CallbackLlm, Finish, Flow, Llm, MockLlm, Usage};
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

/// Un backend du gabarit `code` dans un dépôt git (le manifeste commité, sa
/// base à côté), en mode `approbation` (le défaut), et un hôte dont les
/// sessions et les tâches vivent dans le dossier du témoin.
fn banc(model: ChatModel) -> (tempfile::TempDir, DaemonHost, PathBuf, String) {
    let d = tempfile::tempdir().unwrap();
    let depot = d.path().join("depot");
    let arbre = d.path().join("code");
    std::fs::create_dir_all(&depot).unwrap();
    std::fs::create_dir_all(&arbre).unwrap();
    std::fs::write(arbre.join("main.rs"), "fn main() {}\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &depot).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "backend.json"],
        &["commit", "-q", "-m", "le backend"],
    ] {
        assert!(std::process::Command::new("git").arg("-C").arg(&depot).args(args).status().unwrap().success());
    }
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"))
        .with_chats_root(d.path().join("chats"))
        .with_chat_model(model);
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend s'ouvre");
    let id = h.id.clone();
    (d, host, manifeste.canonicalize().unwrap(), id)
}

/// Un faux modèle qui propose `notes.md` au premier appel, puis répond.
fn qui_propose() -> ChatModel {
    let appels = Arc::new(AtomicUsize::new(0));
    Arc::new(move |_b: &Backend| -> Result<Arc<dyn Llm>, String> {
        let appels = appels.clone();
        let proposition = json!({
            "files": [{ "path": "notes.md", "content": "# Une note de l'agent\n" }],
            "reason": "une note de l'agent",
        })
        .to_string();
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
            let modele = match appels.fetch_add(1, Ordering::SeqCst) {
                0 => MockLlm::new("").with_tool_calls(vec![(crate::declare::PROPOSE, proposition.as_str())]),
                _ => MockLlm::new("Proposé : une note de l'agent."),
            };
            modele.generate(turns, opts, sink)
        })))
    })
}

/// **Un faux modèle qui tient le tour ouvert jusqu'à son arrêt** : une
/// barrière, pas une durée. Il parle par fragments tant que le puits ne
/// demande pas `Flow::Stop` ; il ne finit jamais seul. Une borne de sûreté de
/// 120 s le fait échouer en le disant, pour qu'un arrêt perdu ne fige pas la
/// passe.
fn lent() -> ChatModel {
    Arc::new(|_b: &Backend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-lent", 8192, |_turns, _opts, sink| {
            let borne = Instant::now() + Duration::from_secs(120);
            while Instant::now() < borne {
                if sink.on_token("encore ") == Flow::Stop {
                    sink.on_finish(&Finish::cancelled());
                    return Ok((Finish::cancelled(), Usage::default()));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(crate::llm::LlmError::Model("faux-lent : jamais arrêté en 120 s".into()))
        })))
    })
}

fn vider(rx: &mut async_broadcast::Receiver<crate::events::Event>) -> Vec<Value> {
    let mut v = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(e) => v.push(e.to_json()),
            Err(async_broadcast::TryRecvError::Overflowed(n)) => v.push(json!({ "debordement": n })),
            Err(_) => return v,
        }
    }
}

/// **Un tour propose, et se voit sur le bus du backend.** En mode
/// `approbation`, l'agent propose (la proposition est gardée, rien n'est
/// écrit) ; son run s'annonce sur `agent` sous l'id que le tour rend, ses
/// appels d'outil aussi, ses jetons sur `tokens` ; la session est écrite dans
/// la racine des chats, pas dans l'arbre du projet.
#[test]
fn un_tour_propose_et_se_voit_sur_le_bus_du_backend() {
    let (_d, host, manifeste, id) = banc(qui_propose());
    let depot = manifeste.parent().unwrap().to_path_buf();
    let h = host.hosted(&manifeste).unwrap();
    let (mut agent, mut jetons) = host
        .with_backend(&h, |b| {
            let bus = b.bus_de_publication();
            (bus.subscribe(crate::events::topic::AGENT), bus.subscribe(crate::events::topic::TOKENS))
        })
        .unwrap();

    let r = host.chat_turn(&id, "écris une note").unwrap();
    assert_eq!(r["text"], "Proposé : une note de l'agent.", "{r}");
    assert_eq!(r["stopped_by"], Value::Null, "{r}");
    let run = r["run"].as_str().unwrap().to_string();

    // Proposée, pas écrite.
    let propositions = host.with_backend(&h, |b| b.propositions()).unwrap();
    assert_eq!(propositions.len(), 1, "{propositions:?}");
    assert_eq!(propositions[0].raison, "une note de l'agent");
    assert!(propositions[0].refus.is_none(), "{:?}", propositions[0].refus);
    assert!(!depot.join("notes.md").exists(), "un tour en approbation a écrit");
    // La réponse du tour la porte, sous la forme de PropositionChanged.
    assert_eq!(r["propositions"], json!([{ "id": propositions[0].id, "paths": ["notes.md"], "accepted": true }]), "{r}");

    // Le bus du backend : le run de l'agent, son appel d'outil, ses jetons.
    let a = vider(&mut agent);
    assert!(a.iter().any(|e| e["kind"] == "RunStarted" && e["run"] == run.as_str() && e["run_kind"] == "agent"), "{a:?}");
    assert!(a.iter().any(|e| e["kind"] == "ToolCallStarted" && e.to_string().contains(crate::declare::PROPOSE)), "{a:?}");
    assert!(a.iter().any(|e| e["kind"] == "RunFinished" && e["run"] == run.as_str() && e["ok"] == true), "{a:?}");
    let t = vider(&mut jetons);
    assert!(t.iter().any(|e| e["kind"] == "Token" && e["run"] == run.as_str()), "{t:?}");

    // La session : dans la racine des chats, le fil sans le message système.
    assert!(chat_file(&host.chats_root, &id).is_file());
    let etat = host.chat_state(&id).unwrap();
    assert_eq!(etat["on"], true, "{etat}");
    assert_eq!(etat["run"], Value::Null, "{etat}");
    let roles: Vec<&str> = etat["turns"].as_array().unwrap().iter().filter_map(|t| t["role"].as_str()).collect();
    assert_eq!(roles.first(), Some(&"user"), "{etat}");
    assert!(roles.contains(&"assistant"), "{etat}");
}

/// **Un tour à la fois, et l'arrêt par le même chemin que tout run.** Un
/// second tour pendant le premier est refusé en le disant ; `stop_run` sur le
/// run du tour l'arrête : le tour rend `stopped_by: ide`, et `RunFinished` dit
/// `requested` par l'IDE. Puis l'arrêt du même run, fini : refus nommé.
#[test]
fn un_tour_a_la_fois_et_l_arret_par_stop_run() {
    let (_d, host, manifeste, id) = banc(lent());
    let host = Arc::new(host);
    let h = host.hosted(&manifeste).unwrap();
    let mut agent = host.with_backend(&h, |b| b.bus_de_publication().subscribe(crate::events::topic::AGENT)).unwrap();

    let tour = {
        let (host, id) = (host.clone(), id.clone());
        std::thread::spawn(move || host.chat_turn(&id, "parle longtemps"))
    };
    let fin = Instant::now() + Duration::from_secs(10);
    let run = loop {
        if let Some(run) = host.chat_state(&id).unwrap()["run"].as_str() {
            break run.to_string();
        }
        assert!(Instant::now() < fin, "le tour n'a pas commencé");
        std::thread::sleep(Duration::from_millis(20));
    };
    let refus = host.chat_turn(&id, "et moi ?").unwrap_err();
    assert!(refus.contains("déjà en cours"), "{refus}");

    host.stop_run(&id, &run, "ide").unwrap();
    let r = tour.join().unwrap().unwrap();
    assert_eq!(r["stopped_by"], "ide", "{r}");
    let a = vider(&mut agent);
    let fini = a.iter().find(|e| e["kind"] == "RunFinished" && e["run"] == run.as_str()).unwrap_or_else(|| panic!("{a:?}"));
    assert_eq!(fini["cause"], "requested", "{fini}");
    assert_eq!(fini["cancelled_by"], "ide", "{fini}");
    let refus = host.stop_run(&id, &run, "ide").unwrap_err();
    assert!(refus.contains("en vol"), "{refus}");
    assert_eq!(host.chat_state(&id).unwrap()["run"], Value::Null);
}

/// **Une session survit à l'hôte ; une orpheline est dite.** Un second hôte
/// sur la même racine reprend le fil du premier. Une session dont le manifeste
/// a disparu est orpheline : dite par `chat_state`, jamais rouverte, son
/// fichier laissé.
#[test]
fn une_session_survit_a_l_hote_et_une_orpheline_est_dite() {
    let (d, host, manifeste, id) = banc(qui_propose());
    host.chat_turn(&id, "écris une note").unwrap();
    drop(host);
    // Une session d'un backend disparu, posée à côté.
    let racine = d.path().join("chats");
    let disparu = d.path().join("disparu/backend.json");
    std::fs::write(racine.join("disparu0000.json"), json!({ "manifest": disparu, "session": { "turns": [] } }).to_string()).unwrap();

    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"))
        .with_chats_root(racine.clone())
        .with_chat_model(qui_propose());
    let lignes = host.reread_chats();
    assert!(lignes.iter().any(|l| l.contains("reprise")), "{lignes:?}");
    assert!(lignes.iter().any(|l| l.contains("orpheline") && l.contains("disparu")), "{lignes:?}");
    let h = host.hosted(&manifeste).unwrap();
    assert_eq!(h.id, id, "le backend a changé d'id");
    let etat = host.chat_state(&id).unwrap();
    assert!(etat["turns"].as_array().unwrap().iter().any(|t| t["content"] == "écris une note"), "{etat}");
    let orphelines = etat["orphans"].as_array().unwrap();
    assert_eq!(orphelines.len(), 1, "{etat}");
    assert!(orphelines[0]["pourquoi"].as_str().unwrap().contains("a disparu"), "{etat}");
    assert!(!host.chats.lock().unwrap().contains_key("disparu0000"), "l'orpheline a été rouverte");
    assert!(racine.join("disparu0000.json").is_file(), "le fichier de l'orpheline a été effacé");
}

/// **Un seul message système, le verbe selon le mode** (c7).
#[test]
fn le_message_systeme_dit_le_verbe_du_mode() {
    let a = system_prompt(ModeDeclarations::Approbation);
    let t = system_prompt(ModeDeclarations::Auto);
    assert!(a.contains("propose") && !a.contains("declare them"), "{a}");
    assert!(t.contains("declare them") && t.contains("task's branch"), "{t}");
    // Le reste est le même texte.
    assert_eq!(a.split("To change them").next(), t.split("To change them").next());
}

/// Un faux modèle sans état, piloté par la conversation : après le résultat
/// d'un outil, il répond ; sur un message qui contient « lent », il parle
/// jusqu'à ce qu'on l'arrête (une barrière) ; sinon il propose `notes.md`.
#[cfg(unix)]
fn selon_le_message() -> ChatModel {
    Arc::new(|_b: &Backend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, |turns, opts, sink| {
            let dernier = turns.last().map(|t| (t.role.clone(), t.content.clone())).unwrap_or_default();
            if dernier.0 == "tool" {
                return MockLlm::new("Proposé : une note de l'agent.").generate(turns, opts, sink);
            }
            if dernier.1.contains("lent") {
                // Tenu ouvert jusqu'à son arrêt, comme `lent` : une barrière.
                let borne = Instant::now() + Duration::from_secs(120);
                while Instant::now() < borne {
                    if sink.on_token("encore ") == Flow::Stop {
                        sink.on_finish(&Finish::cancelled());
                        return Ok((Finish::cancelled(), Usage::default()));
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                return Err(crate::llm::LlmError::Model("faux modèle : jamais arrêté en 120 s".into()));
            }
            let proposition = json!({
                "files": [{ "path": "notes.md", "content": "# Une note de l'agent\n" }],
                "reason": "une note de l'agent",
            })
            .to_string();
            MockLlm::new("").with_tool_calls(vec![(crate::declare::PROPOSE, proposition.as_str())]).generate(turns, opts, sink)
        })))
    })
}

/// **Le chat dans la page de l'IDE** (K2), dans un vrai navigateur, sur un faux
/// modèle. Un message envoyé : la bulle de l'agent montre son appel d'outil,
/// le lien vers la proposition qu'il a faite, puis sa réponse ; la proposition
/// est dans la vue des propositions, rien n'est écrit. Un second tour, lent :
/// Arrêter le coupe par le même chemin que tout run, et la page dit « arrêté
/// dans l'IDE ». Un troisième, envoyé flux fermé : la relecture de l'ouverture
/// du flux rend Arrêter à la page. Un quatrième, qui propose flux fermé : le
/// lien de la proposition vient de la réponse du tour. La console reste vide.
#[cfg(unix)]
#[test]
fn le_chat_se_mene_et_s_arrete_dans_la_page() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_chat_se_mene_et_s_arrete_dans_la_page";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, host, manifeste, id) = banc(selon_le_message());
    drop(host);
    let depot = manifeste.parent().unwrap().to_path_buf();
    let (chats, taches) = (d.path().join("chats"), d.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(selon_le_message())
    });
    let racine = format!("http://{adresse}");
    let decrit: Value = {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste }).to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    assert_eq!(decrit["ok"], true, "{decrit}");

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/code/{id}/chat"),
        "document.querySelector('#chat-note')?.textContent === 'modèle : faux-modele' && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );

    // Un tour qui propose.
    n.evaluer("document.querySelector('#chat-message').value = 'écris une note'; document.querySelector('#chat-envoyer').click()");
    n.attendre(
        "document.querySelector('#chat-note')?.textContent === 'fini' \
         && document.querySelector('.tour.assistant .proposition-du-tour a[data-proposition]') \
         && [...document.querySelectorAll('.tour.assistant .outils li')].some(li => li.textContent.startsWith('propose')) \
         && document.querySelector('.tour.assistant:last-child pre')?.textContent === \"Proposé : une note de l'agent.\"",
        "le tour, son outil, sa proposition et sa réponse",
    );
    assert!(!depot.join("notes.md").exists(), "un tour en approbation a écrit");
    n.evaluer("document.querySelector('.proposition-du-tour a').click()");
    n.attendre("document.querySelector('#vue h1')?.textContent.startsWith('Proposition') || location.pathname.includes('/propositions/')", "la proposition du tour, ouverte");

    // Un tour lent, arrêté depuis la page. La page rechargée rouvre son flux :
    // un tour envoyé avant qu'il soit ouvert perd son RunStarted, et la page ne
    // connaît jamais son run (ni Arrêter, ni jetons).
    n.aller(
        &format!("{racine}/ide/code/{id}/chat"),
        "document.querySelector('#chat-envoyer') && !document.querySelector('#chat-envoyer').disabled \
         && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    n.evaluer("document.querySelector('#chat-message').value = 'parle, lentement'; document.querySelector('#chat-envoyer').click()");
    n.attendre("!document.querySelector('#chat-arreter').hidden && document.querySelector('.tour.en-cours pre')?.textContent.includes('encore')", "le tour lent, en cours, ses jetons dans la bulle");
    n.evaluer("document.querySelector('#chat-arreter').click()");
    n.attendre("document.querySelector('#chat-note')?.textContent === \"arrêté dans l'IDE\" && document.querySelector('#chat-arreter').hidden", "le tour arrêté dans l'IDE");

    // Un tour envoyé flux fermé : la page n'a jamais son RunStarted. À
    // l'ouverture du flux (son onopen, appelé jusqu'à ce que l'hôte ait le run
    // en vol), la page relit l'état du chat, et Arrêter apparaît quand même.
    n.evaluer("window.__ide.flux.close(); document.querySelector('#chat-message').value = 'parle, lentement, sans flux'; document.querySelector('#chat-envoyer').click()");
    n.attendre(
        "(window.__ide.flux.onopen(), !document.querySelector('#chat-arreter').hidden && document.querySelector('#chat-note')?.textContent === \"l'agent travaille…\")",
        "le tour parti flux fermé, Arrêter rendu par la relecture",
    );
    n.evaluer("document.querySelector('#chat-arreter').click()");
    n.attendre("document.querySelector('#chat-note')?.textContent === \"arrêté dans l'IDE\" && document.querySelector('#chat-arreter').hidden", "le tour parti flux fermé, arrêté dans l'IDE");

    // Un tour qui propose, flux toujours fermé : le lien de sa proposition
    // vient de la réponse du tour (le flux peut la dire après sa fin).
    n.evaluer("document.querySelector('#chat-message').value = 'écris une autre note'; document.querySelector('#chat-envoyer').click()");
    n.attendre(
        "document.querySelector('#chat-note')?.textContent === 'fini' \
         && document.querySelector('.tour.assistant:last-child .proposition-du-tour a[data-proposition]')",
        "le tour qui propose flux fermé, le lien de sa proposition",
    );
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
