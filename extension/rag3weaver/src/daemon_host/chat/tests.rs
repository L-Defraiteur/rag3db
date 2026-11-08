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
    banc_avec(model, |_| {})
}

/// Le même banc, le manifeste retouché avant son commit (un rôle d'outil…).
fn banc_avec(model: ChatModel, retouche: impl Fn(&mut Value)) -> (tempfile::TempDir, DaemonHost, PathBuf, String) {
    banc_dans("depot", model, retouche)
}

/// Le banc dans un dépôt nommé : la page nomme un dépôt par son dossier.
fn banc_dans(nom: &str, model: ChatModel, retouche: impl Fn(&mut Value)) -> (tempfile::TempDir, DaemonHost, PathBuf, String) {
    let d = tempfile::tempdir().unwrap();
    let depot = d.path().join(nom);
    let arbre = d.path().join("code");
    std::fs::create_dir_all(&depot).unwrap();
    std::fs::create_dir_all(&arbre).unwrap();
    std::fs::write(arbre.join("main.rs"), "fn main() {}\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &depot).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre.to_string_lossy());
    retouche(&mut m);
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
        // Le journal des runs, actif : les fiches écrites pendant une
        // conversation se trouvent par ses runs.
        .with_runs_root(d.path().join("runs"))
        .with_chat_model(model);
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend s'ouvre");
    let id = h.id.clone();
    (d, host, manifeste.canonicalize().unwrap(), id)
}

/// Un faux modèle qui propose `notes.md` au premier appel, puis répond.
fn qui_propose() -> ChatModel {
    let appels = Arc::new(AtomicUsize::new(0));
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
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
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
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

/// **Un seul message système, le verbe selon le mode** (c7), et une phrase
/// sur le code selon qu'on est, ou non, dans une tâche.
#[test]
fn le_message_systeme_dit_le_verbe_du_mode() {
    let a = system_prompt(ModeDeclarations::Approbation, false);
    let t = system_prompt(ModeDeclarations::Auto, true);
    assert!(a.contains("propose") && !a.contains("declare them"), "{a}");
    assert!(t.contains("declare them") && t.contains("task's branch"), "{t}");
    // Le reste est le même texte.
    assert_eq!(a.split("To change them").next(), t.split("To change them").next());
    // Le code : s'éditer sur la branche dans une tâche, se lire seulement hors d'elle.
    assert!(t.contains("edit with edit_file") && t.contains("on the task's branch only"), "{t}");
    assert!(a.contains("can be read here, not written") && a.contains("open a task") && !a.contains("edit with edit_file"), "{a}");
    // La phrase du code suit la tâche, pas le mode.
    assert_eq!(system_prompt(ModeDeclarations::Auto, false).contains("not written"), true);
}

/// Un faux modèle sans état, piloté par la conversation : après le résultat
/// d'un outil, il répond ; sur un message qui contient « lent », il parle
/// jusqu'à ce qu'on l'arrête (une barrière) ; sinon il propose `notes.md`.
#[cfg(unix)]
fn selon_le_message() -> ChatModel {
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
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
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
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

    // **Les barrières sont sur l'hôte** (25 octobre 2026, le troisième rouge sous la lib
    // entière) : avant de regarder la page, le témoin attend que l'hôte ait fait la chose — le
    // tour en vol, ou fini. La page a ensuite 20 s pour la montrer ; si elle ne le fait pas,
    // c'est la page ou le flux qui a manqué, et le message le dit.
    let tour_en_vol = || -> Option<String> {
        let mut r = crate::daemon::agent()
            .get(format!("{racine}/ide/chat?backend={id}"))
            .header("Cookie", "rag3weaver_ide=j")
            .call()
            .ok()?;
        let v: Value = serde_json::from_str(&r.body_mut().read_to_string().ok()?).ok()?;
        v["result"]["run"].as_str().map(str::to_string)
    };
    let attendre_l_hote = |en_vol: bool, quoi: &str| {
        let borne = Instant::now() + Duration::from_secs(120);
        while tour_en_vol().is_some() != en_vol {
            assert!(Instant::now() < borne, "l'hôte, en 120 s : {quoi} jamais vu (sûreté : le tour ne part pas, ou ne s'arrête pas)");
            std::thread::sleep(Duration::from_millis(20));
        }
    };

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/code/{id}/chat"),
        "document.querySelector('#model')?.textContent === 'modèle : faux-modele' && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );

    // Un tour qui propose.
    n.evaluer("document.querySelector('#message').value = 'écris une note'; document.querySelector('#send').click()");
    n.attendre(
        "document.querySelector('#status')?.textContent === 'fini' \
         && document.querySelector('article.message.assistant .proposition-du-tour a[data-proposition]') \
         && [...document.querySelectorAll('article.message.assistant details.outil')].some(d => d.dataset.outil === 'propose') \
         && [...document.querySelectorAll('article.message.assistant')].at(-1)?.querySelector('.body')?.textContent === \"Proposé : une note de l'agent.\"",
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
        "document.querySelector('#send') && !document.querySelector('#send').disabled \
         && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    n.evaluer("document.querySelector('#message').value = 'parle, lentement'; document.querySelector('#send').click()");
    attendre_l_hote(true, "le tour lent en vol");
    n.attendre(
        "!document.querySelector('#stop').hidden && document.querySelector('article.message.en-cours .body')?.textContent.includes('encore')",
        "l'hôte a le tour lent en vol, et la page ne montre pas Arrêter ni ses jetons dans la bulle",
    );
    n.evaluer("document.querySelector('#stop').click()");
    attendre_l_hote(false, "le tour lent arrêté");
    n.attendre(
        "document.querySelector('#status')?.textContent === \"arrêté dans l'IDE\" && document.querySelector('#stop').hidden",
        "l'hôte a arrêté le tour, et la page ne dit pas « arrêté dans l'IDE »",
    );

    // Un tour envoyé flux fermé : la page n'a jamais son RunStarted. À
    // l'ouverture du flux (son onopen, appelé jusqu'à ce que l'hôte ait le run
    // en vol), la page relit l'état du chat, et Arrêter apparaît quand même.
    n.evaluer("window.__ide.flux.close(); document.querySelector('#message').value = 'parle, lentement, sans flux'; document.querySelector('#send').click()");
    attendre_l_hote(true, "le tour parti flux fermé en vol");
    n.attendre(
        "(window.__ide.flux.onopen(), !document.querySelector('#stop').hidden && document.querySelector('#status')?.textContent === \"l'agent travaille…\")",
        "le tour parti flux fermé, Arrêter rendu par la relecture",
    );
    n.evaluer("document.querySelector('#stop').click()");
    attendre_l_hote(false, "le tour parti flux fermé arrêté");
    n.attendre("document.querySelector('#status')?.textContent === \"arrêté dans l'IDE\" && document.querySelector('#stop').hidden", "le tour parti flux fermé, arrêté dans l'IDE");

    // Un tour qui propose, flux toujours fermé : le lien de sa proposition
    // vient de la réponse du tour (le flux peut la dire après sa fin).
    n.evaluer("document.querySelector('#message').value = 'écris une autre note'; document.querySelector('#send').click()");
    n.attendre(
        "document.querySelector('#status')?.textContent === 'fini' \
         && [...document.querySelectorAll('article.message.assistant')].at(-1)?.querySelector('.proposition-du-tour a[data-proposition]')",
        "le tour qui propose flux fermé, le lien de sa proposition",
    );
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// Un faux modèle qui parle, appelle `list_files`, parle, appelle
/// `read_file`, puis répond : « mots, outil, mots, outil, réponse ».
fn qui_lit_deux_fois() -> ChatModel {
    let appels = Arc::new(AtomicUsize::new(0));
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let appels = appels.clone();
        Ok(Arc::new(CallbackLlm::new("faux-lecteur", 8192, move |turns, opts, sink| {
            let modele = match appels.fetch_add(1, Ordering::SeqCst) {
                0 => MockLlm::new("je liste").with_tool_calls(vec![("list_files", "{}")]),
                1 => MockLlm::new("je lis").with_tool_calls(vec![("read_file", r#"{"path":"main.rs"}"#)]),
                _ => MockLlm::new("Fini."),
            };
            modele.generate(turns, opts, sink)
        })))
    })
}

/// **Le chat tient l'ordre et l'heure de ses appels** (tickets du 7 novembre
/// 2026 : « l'heure d'un appel d'outil est celle de l'insertion du tour »,
/// « un outil déplié ne montre que fait », « l'usage du modèle … jauge du
/// contexte »). Un tour « mots, outil, mots, outil, réponse » par le chemin de
/// l'hôte : le fil relu garde cette suite ; chaque appel porte l'heure où il est
/// parti et chaque résultat l'heure où il est revenu, dans l'ordre des faits et
/// dans la fenêtre du tour ; `ToolCallFinished` porte un extrait du résultat ;
/// l'état du chat rend la jauge (le dernier appel au modèle, le contexte du
/// modèle déclaré).
#[test]
fn un_tour_a_deux_outils_garde_l_ordre_et_l_heure_de_ses_appels() {
    let (_d, host, manifeste, id) = banc(qui_lit_deux_fois());
    let h = host.hosted(&manifeste).unwrap();
    let mut agent = host.with_backend(&h, |b| b.bus_de_publication().subscribe(crate::events::topic::AGENT)).unwrap();
    let avant = crate::events::maintenant_ms();
    let r = host.chat_turn(&id, "regarde le code").unwrap();
    let apres = crate::events::maintenant_ms();
    assert_eq!(r["text"], "Fini.", "{r}");

    let etat = host.chat_state(&id).unwrap();
    let tours = etat["turns"].as_array().unwrap().clone();
    let forme: Vec<String> = tours
        .iter()
        .map(|t| match t["role"].as_str().unwrap_or("") {
            "assistant" => format!(
                "assistant:{}:{}",
                t["content"].as_str().unwrap_or(""),
                t["tool_calls"].as_array().map(|c| c.iter().filter_map(|c| c["name"].as_str()).collect::<Vec<_>>().join("+")).unwrap_or_default()
            ),
            "tool" => format!("tool:{}", t["tool_name"].as_str().unwrap_or("")),
            autre => autre.to_string(),
        })
        .collect();
    assert_eq!(
        forme,
        ["user", "assistant:je liste:list_files", "tool:list_files", "assistant:je lis:read_file", "tool:read_file", "assistant:Fini.:"],
        "{etat}"
    );

    // Les heures : début de chaque appel, fin de chaque résultat, dans l'ordre.
    let heure = |v: &Value, quoi: &str| v["at_ms"].as_u64().unwrap_or_else(|| panic!("{quoi} n'a pas d'heure : {v}"));
    let a1 = heure(&tours[1]["tool_calls"][0], "l'appel de list_files");
    let f1 = heure(&tours[2], "le résultat de list_files");
    let a2 = heure(&tours[3]["tool_calls"][0], "l'appel de read_file");
    let f2 = heure(&tours[4], "le résultat de read_file");
    assert!(avant <= a1 && a1 <= f1 && f1 <= a2 && a2 <= f2 && f2 <= apres, "heures hors d'ordre : {avant} {a1} {f1} {a2} {f2} {apres}");
    assert!(tours[0].get("at_ms").is_none() && tours[5].get("at_ms").is_none(), "seuls les appels et les résultats sont datés : {etat}");

    // L'extrait du résultat dans ToolCallFinished, pour le flux.
    let a = vider(&mut agent);
    let fini = |outil: &str| {
        a.iter()
            .find(|e| e["kind"] == "ToolCallFinished" && e["tool"] == outil)
            .cloned()
            .unwrap_or_else(|| panic!("pas de ToolCallFinished pour {outil} : {a:?}"))
    };
    let lu = fini("read_file");
    assert!(lu["result_excerpt"].as_str().is_some_and(|x| x.contains("fn main")), "l'extrait de read_file : {lu}");
    assert!(lu.get("truncated").is_none(), "un petit résultat n'est pas coupé : {lu}");
    assert!(fini("list_files")["result_excerpt"].as_str().is_some_and(|x| x.contains("main.rs")), "{a:?}");

    // La jauge : le dernier appel au modèle, le contexte du modèle déclaré.
    let c = &etat["context"];
    assert_eq!(c["max"], 8192, "{etat}");
    assert_eq!(c["source"], "run", "{etat}");
    assert!(c["used"].as_u64().is_some_and(|n| n > 0), "{etat}");
    assert!(c["at"].as_u64().is_some_and(|t| avant <= t && t <= apres), "{etat}");
}

/// **Un outil déclare son rôle** (`role` à son attachement, forme de 96 et
/// d'everything declarative) : `describe()` le rend à côté de `policy`, et le
/// run du graphe de l'outil l'annonce dans `RunStarted.target.role` — la page
/// range les runs par rôle sans lire leurs noms. Un outil sans rôle n'en porte
/// pas.
#[test]
fn le_role_d_un_outil_se_lit_dans_describe_et_dans_son_run() {
    let (_d, host, manifeste, id) = banc_avec(qui_lit_deux_fois(), |m| {
        m["tools"]["list_files"]["role"] = json!("archiviste");
    });
    let h = host.hosted(&manifeste).unwrap();
    let outils = host.with_backend(&h, |b| b.describe()).unwrap()["tools"].as_array().cloned().unwrap_or_default();
    let de = |nom: &str| outils.iter().find(|t| t["name"] == nom).cloned().unwrap_or_else(|| panic!("pas d'outil {nom}"));
    assert_eq!(de("list_files")["role"], "archiviste", "{:?}", de("list_files"));
    assert!(de("read_file").get("role").is_none(), "{:?}", de("read_file"));

    let mut graphes = host.with_backend(&h, |b| b.bus_de_publication().subscribe(crate::events::topic::DATAFLOW)).unwrap();
    host.chat_turn(&id, "regarde le code").unwrap();
    let d = vider(&mut graphes);
    let debut = |outil: &str| {
        d.iter()
            .find(|e| e["kind"] == "RunStarted" && e["run_kind"] == "graph" && e["target"]["tool"] == outil)
            .cloned()
            .unwrap_or_else(|| panic!("pas de RunStarted du graphe de {outil} : {d:?}"))
    };
    assert_eq!(debut("list_files")["target"]["role"], "archiviste", "{d:?}");
    assert!(debut("read_file")["target"].get("role").is_none(), "{d:?}");
}

/// **L'usage du modèle et les tâches de l'archiviste, par l'état du chat** (liste
/// de Lucie, 8 novembre 2026, la barre latérale) : l'état rend l'usage du dernier
/// tour et celui de la session, par unité, lus au journal des runs du chat ; la
/// session est la somme de ses tours ; il rend aussi les runs à rôle finis, lus
/// au même journal, pour qu'un rechargement de la page les retrouve.
#[test]
fn l_etat_du_chat_rend_l_usage_du_tour_et_de_la_session_et_les_taches_a_role() {
    let (_d, host, _m, id) = banc_avec(qui_lit_deux_fois(), |m| {
        m["tools"]["list_files"]["role"] = json!("archiviste");
    });
    let attendre_le_journal = |run: &str| {
        let debut = Instant::now();
        while host.journal.lire(&id, run).is_none() {
            assert!(debut.elapsed() < Duration::from_secs(10), "le journal du run {run} n'est pas écrit");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let r1 = host.chat_turn(&id, "regarde le code").unwrap();
    attendre_le_journal(r1["run"].as_str().unwrap());
    let e1 = host.chat_state(&id).unwrap();
    let u = |e: &Value, quand: &str, unite: &str| e["usage"][quand][unite].as_u64().unwrap_or(0);
    assert!(u(&e1, "tour", "input_tokens") > 0, "le tour a un usage d'entrée : {}", e1["usage"]);
    assert_eq!(e1["usage"]["tour"], e1["usage"]["session"], "un seul tour : la session est ce tour");
    assert_eq!(e1["usage"]["tour_run"], r1["run"], "{}", e1["usage"]);
    let r2 = host.chat_turn(&id, "et encore").unwrap();
    attendre_le_journal(r2["run"].as_str().unwrap());
    let e2 = host.chat_state(&id).unwrap();
    for unite in ["input_tokens", "output_tokens"] {
        assert_eq!(u(&e2, "session", unite), u(&e1, "tour", unite) + u(&e2, "tour", unite), "la session est la somme de ses tours ({unite}) : {}", e2["usage"]);
    }
    assert_eq!(e2["usage"]["tours"], 2, "{}", e2["usage"]);
    // Les tâches à rôle finies, lues au journal : l'archiviste a listé les fichiers.
    let roles = e2["role_runs"].as_array().cloned().unwrap_or_default();
    let archiviste = roles.iter().find(|r| r["role"] == "archiviste").unwrap_or_else(|| panic!("pas de run de l'archiviste : {roles:?}"));
    assert_eq!((archiviste["tool"].as_str(), archiviste["state"].as_str()), (Some("list_files"), Some("fini")), "{archiviste}");
    assert!(archiviste["at_ms"].as_u64().is_some_and(|t| t > 0), "{archiviste}");
}

/// Le faux modèle de la page du chat : « deux outils » fait « mots, outil, mots,
/// outil, réponse » ; « long » parle soixante lignes, une toutes les 40 ms (un
/// flux de soixante événements), puis finit.
fn page_du_chat() -> ChatModel {
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-page", 8192, |turns, opts, sink| {
            let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
            let question = turns.get(depuis).map(|t| t.content.clone()).unwrap_or_default();
            let outils = turns[depuis..].iter().filter(|t| t.role == "tool").count();
            if question.contains("long") {
                for n in 0..60 {
                    if sink.on_token(&format!("ligne {n}\n")) == Flow::Stop {
                        sink.on_finish(&Finish::cancelled());
                        return Ok((Finish::cancelled(), Usage::default()));
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
                return MockLlm::new("").generate(turns, opts, sink);
            }
            match outils {
                0 => MockLlm::new("je liste").with_tool_calls(vec![("list_files", "{}")]),
                1 => MockLlm::new("je lis").with_tool_calls(vec![("read_file", r#"{"path":"main.rs"}"#)]),
                _ => MockLlm::new("Fini."),
            }
            .generate(turns, opts, sink)
        })))
    })
}

/// **La page du chat tient l'ordre, le défilement et la place** (les sept tickets
/// du 7 novembre 2026, lot « le chat tient l'ordre », et les mots de Lucie sur sa
/// barre). Dans un vrai navigateur, à 2 560 × 1 440 :
/// - un tour « mots, outil, mots, outil, réponse » se lit dans cet ordre dans sa
///   bulle, pendant le tour comme après un rechargement ;
/// - l'outil déplié montre son heure, sa durée et son résultat ;
/// - la conversation porte sa jauge (le dernier appel, le contexte du modèle
///   déclaré, comme l'état du chat de l'hôte) ; un outil à rôle se range dans la
///   barre des tâches de fond ;
/// - la colonne du chat occupe l'écran, l'aide tient sur une ligne, le chat
///   prend la hauteur de la fenêtre moins l'en-tête (1 440 et 1 080) ;
/// - remontée, la liste ne bouge pas pendant un flux de soixante événements ;
///   collée en bas, elle suit ; déplier un outil ne déplace pas la page.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn la_page_du_chat_tient_l_ordre_le_defilement_et_la_place() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "la_page_du_chat_tient_l_ordre_le_defilement_et_la_place";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, host, manifeste, id) = banc_avec(page_du_chat(), |m| {
        m["tools"]["list_files"]["role"] = json!("archiviste");
    });
    drop(host);
    let (chats, taches) = (d.path().join("chats"), d.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(page_du_chat())
    });
    let racine = format!("http://{adresse}");
    let post = |chemin: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}{chemin}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let etat_du_chat = || -> Value {
        let mut r = crate::daemon::agent().get(format!("{racine}/ide/chat?backend={id}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        serde_json::from_str::<Value>(&r.body_mut().read_to_string().unwrap()).unwrap()["result"].clone()
    };
    assert_eq!(post("/ide/describe", json!({ "manifest": manifeste }))["ok"], true);

    let mut n = Navigateur::lancer(&binaire);
    let fenetre = |n: &mut Navigateur, w: u32, h: u32| {
        n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": w, "height": h, "deviceScaleFactor": 1, "mobile": false }));
    };
    fenetre(&mut n, 2560, 1440);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let page = format!("{racine}/ide/code/{id}/chat");
    let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&page, prete);
    let envoyer = |n: &mut Navigateur, texte: &str| {
        n.evaluer(&format!("document.querySelector('#message').value = {}; document.querySelector('#send').click()", json!(texte)));
    };
    // La suite des parties de la dernière bulle d'agent.
    let suite = "[...[...document.querySelectorAll('article.message.assistant')].at(-1).children].filter(c => c.matches('.body, details.outil')).map(c => c.matches('.body') ? 'texte:' + c.textContent.trim() : 'outil:' + c.dataset.outil).join('|')";
    let attendue = "texte:je liste|outil:list_files|texte:je lis|outil:read_file|texte:Fini.";

    // 1. L'ordre, pendant le tour et après.
    envoyer(&mut n, "deux outils");
    n.attendre(&format!("document.querySelector('#status')?.textContent === 'fini' && {suite} === {}", json!(attendue)), "la bulle dans l'ordre d'arrivée");
    // 2. L'outil déplié : son heure, sa durée, son résultat entier.
    let lu = n.evaluer("(() => { const d = document.querySelector('details.outil[data-outil=\"read_file\"]'); return { resultat: d?.querySelector('.resultat')?.textContent ?? '', heure: d?.querySelector('.heure')?.textContent ?? '' }; })()");
    assert!(lu["resultat"].as_str().unwrap_or("").contains("fn main"), "le résultat déplié : {lu}");
    assert!(lu["heure"].as_str().unwrap_or("").contains(" ms"), "l'heure et la durée : {lu}");
    // 3. La jauge de la conversation, la barre des tâches de fond.
    // La page relit les jauges à la fin du tour, sans attendre : on attend la
    // valeur de l'hôte, pas un instant.
    let contexte = etat_du_chat()["context"].clone();
    n.attendre(
        &format!(
            "(() => {{ const u = document.querySelector('.conversation[data-backend=\"{id}\"] .usage'); return u?.dataset.max === '8192' && u?.dataset.used === {}; }})()",
            json!(contexte["used"].to_string())
        ),
        &format!("la jauge de la conversation égale à l'état de l'hôte {contexte}"),
    );
    n.attendre("document.querySelector('#runs-a-role [data-role=\"archiviste\"] .run-a-role')?.dataset.state === 'fini'", "le run à rôle dans la barre");
    // 4. La place : la colonne occupe l'écran, l'aide tient sur une ligne, la hauteur.
    let place = n.evaluer(
        "(() => { const r = (s) => document.querySelector(s).getBoundingClientRect(); \
         return { colonne: r('#messages').width, aide: r('#aide').height, aideL: r('#aide').width, saisie: r('#message').width, \
                  chat: r('.chat').height, fenetre: innerHeight, entete: r('#entete').height }; })()",
    );
    let f = |k: &str| place[k].as_f64().unwrap_or(0.0);
    assert!(f("colonne") >= 2560.0 - 300.0 - 40.0, "la colonne du chat n'occupe pas l'écran : {place}");
    assert!(f("aide") <= 20.0 && f("aideL") >= 200.0 && f("saisie") >= 200.0, "le pied : {place}");
    assert!((f("chat") - (f("fenetre") - f("entete"))).abs() <= 2.0, "la hauteur à 1 440 : {place}");
    fenetre(&mut n, 1920, 1080);
    n.attendre("Math.abs(document.querySelector('.chat').getBoundingClientRect().height - (innerHeight - document.querySelector('#entete').getBoundingClientRect().height)) <= 2 && innerHeight === 1080", "la hauteur à 1 080");
    // 5. Le rechargement rend la même suite.
    n.aller(&page, prete);
    n.attendre(&format!("{suite} === {}", json!(attendue)), "la même suite après un rechargement");

    // 6. Remontée, la liste ne bouge pas pendant le flux.
    let liste = "document.querySelector('#messages')";
    envoyer(&mut n, "parle long");
    n.attendre(&format!("{liste}.scrollHeight > {liste}.clientHeight + 300"), "le fil déborde");
    n.evaluer(&format!("{liste}.scrollTop = 0; {liste}.dispatchEvent(new Event('scroll'))"));
    let mut vus = Vec::new();
    while n.evaluer("document.querySelector('#status')?.textContent").as_str() != Some("fini") {
        vus.push(n.evaluer(&format!("{liste}.scrollTop")).as_f64().unwrap_or(-1.0));
        std::thread::sleep(Duration::from_millis(30));
        assert!(vus.len() < 2000, "le tour long ne finit pas");
    }
    assert!(vus.iter().all(|&y| y == 0.0), "remontée, la liste a bougé pendant le flux : {vus:?}");
    // 7. Collée en bas, elle suit.
    n.evaluer(&format!("{liste}.scrollTop = {liste}.scrollHeight; {liste}.dispatchEvent(new Event('scroll'))"));
    envoyer(&mut n, "encore long");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le second tour long");
    n.attendre(&format!("{liste}.scrollHeight - {liste}.scrollTop - {liste}.clientHeight < 40"), "collée en bas, la liste suit");
    // 8. Déplier un outil visible ne déplace pas la page.
    n.evaluer(&format!(
        "(() => {{ const d = document.querySelector('details.outil[data-outil=\"read_file\"]'); const m = {liste}; \
         m.scrollTop = d.offsetTop - m.offsetTop - 100; m.dispatchEvent(new Event('scroll')); }})()"
    ));
    let avant = n.evaluer(&format!("{liste}.scrollTop"));
    n.evaluer("document.querySelector('details.outil[data-outil=\"read_file\"] summary').click()");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(n.evaluer(&format!("{liste}.scrollTop")), avant, "déplier un outil a déplacé la page");
    assert!(n.evaluer("document.querySelector('details.outil[data-outil=\"read_file\"]').open").as_bool().unwrap_or(false));

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **La barre parle en dépôts** (Lucie, 7 novembre 2026 : « les gens qui
/// bossent parlent en branches, PR, worktrees, repos attachés ») : deux dépôts
/// attachés à l'hôte paraissent sous « Dépôts », chacun nommé par son dossier et
/// sa branche, chacun avec sa jauge ; un tour dans l'un remplit sa jauge et pas
/// celle de l'autre ; le mot « backend » ne paraît nulle part à l'écran du chat.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn la_barre_du_chat_parle_en_depots_et_chacun_a_sa_jauge() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "la_barre_du_chat_parle_en_depots_et_chacun_a_sa_jauge";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d1, h1, m1, id1) = banc_dans("atelier", page_du_chat(), |_| {});
    let (d2, h2, m2, id2) = banc_dans("outils", page_du_chat(), |_| {});
    drop((h1, h2));
    let (chats, taches) = (d1.path().join("chats"), d1.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(page_du_chat())
    });
    let racine = format!("http://{adresse}");
    for m in [&m1, &m2] {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": m }).to_string())
            .unwrap();
        let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(v["ok"], true, "{v}");
    }
    let mut n = Navigateur::lancer(&binaire);
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/code/{id1}/chat"),
        "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    // Les deux dépôts, nommés par leur dossier et leur branche.
    n.attendre(
        "(() => { const s = document.querySelector('[data-section=\"Sessions\"]'); if (!s) return false; \
         const noms = [...s.querySelectorAll('.conversation a')].map(a => a.textContent.trim()); \
         return noms.some(t => t.startsWith('atelier · ')) && noms.some(t => t.startsWith('outils · ')); })()",
        "deux dépôts sous « Sessions », avec leur branche",
    );
    // Un tour dans le premier : sa jauge se remplit, pas celle de l'autre.
    n.evaluer("document.querySelector('#message').value = 'deux outils'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour dans le premier dépôt");
    let jauge = |n: &mut Navigateur, id: &str| n.evaluer(&format!("document.querySelector('.conversation[data-backend=\"{id}\"] .usage')?.dataset.used ?? null"));
    n.attendre(&format!("Number(document.querySelector('.conversation[data-backend=\"{id1}\"] .usage')?.dataset.used) > 0"), "la jauge du premier dépôt");
    assert_eq!(jauge(&mut n, &id2), json!(""), "la jauge de l'autre dépôt a bougé");
    // Le mot du code ne paraît pas à l'écran.
    let texte = n.evaluer("document.body.innerText.toLowerCase()");
    assert!(!texte.as_str().unwrap_or("").contains("backend"), "« backend » paraît à l'écran : {texte}");

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    drop(d2);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **La barre dit l'usage, le récapitulatif et l'archiviste** (liste de Lucie, 8
/// novembre 2026) : après un tour, la barre montre sous la conversation les jetons
/// du tour et de la session, égaux à l'état de l'hôte ; un récapitulatif dit les
/// tours, les jetons et les fiches, et son bouton déplie les fiches écrites ; une
/// tâche de l'archiviste finie reste dans « Tâches de fond » après un
/// rechargement de la page.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn la_barre_dit_l_usage_le_recapitulatif_et_l_archiviste() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "la_barre_dit_l_usage_le_recapitulatif_et_l_archiviste";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, h, m, id) = banc_dans("atelier", page_du_chat(), |m| {
        m["tools"]["list_files"]["role"] = json!("archiviste");
    });
    drop(h);
    // Un journal des runs à lui : sous les témoins, celui par défaut est éteint,
    // et l'usage comme les tâches à rôle se lisent au journal.
    let (chats, taches, runs) = (d.path().join("chats"), d.path().join("taches"), d.path().join("runs"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_runs_root(runs).with_chat_model(page_du_chat())
    });
    let racine = format!("http://{adresse}");
    let mut r = crate::daemon::agent()
        .post(format!("{racine}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": m }).to_string())
        .unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["ok"], true, "{v}");
    let mut n = Navigateur::lancer(&binaire);
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let page = format!("{racine}/ide/code/{id}/chat");
    let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&page, prete);
    n.evaluer("document.querySelector('#message').value = 'deux outils'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
    // L'usage sous la conversation, égal à l'état de l'hôte.
    let conv = format!(".conversation[data-backend=\"{id}\"]");
    n.attendre(&format!("Number(document.querySelector('{conv} .usage-jetons')?.dataset.sessionInput) > 0"), "les jetons de la session dans la barre");
    let (statut, etat) = crate::daemon_host::tests::demander("GET", &format!("{racine}/ide/chat?backend={id}"), Some("j"));
    assert_eq!(statut, 200, "{etat}");
    let attendu = etat["result"]["usage"]["session"]["input_tokens"].as_u64().or_else(|| etat["usage"]["session"]["input_tokens"].as_u64()).unwrap_or(0);
    let lu = n.evaluer(&format!("Number(document.querySelector('{conv} .usage-jetons').dataset.sessionInput)"));
    assert_eq!(lu.as_u64(), Some(attendu), "la barre dit l'usage de l'hôte : {etat}");
    // Le récapitulatif : son bouton déplie les fiches écrites.
    n.attendre(&format!("!!document.querySelector('{conv} .recap button')"), "le récapitulatif et son bouton");
    // La conversation ouverte part dépliée (ses fiches se voyaient d'emblée) ;
    // le bouton la replie, puis la redéplie.
    assert_eq!(n.evaluer(&format!("document.querySelector('{conv} .recap').dataset.ouvert")), json!("oui"));
    n.evaluer(&format!("document.querySelector('{conv} .recap button').click()"));
    n.attendre(&format!("document.querySelector('{conv} .recap').dataset.ouvert === 'non' && !document.querySelector('{conv} .recap .fiches-ecrites')"), "le bouton replie les fiches");
    n.evaluer(&format!("document.querySelector('{conv} .recap button').click()"));
    n.attendre(&format!("document.querySelector('{conv} .recap').dataset.ouvert === 'oui' && !!document.querySelector('{conv} .recap .fiches-ecrites')"), "le bouton déplie les fiches");
    // L'archiviste, après un rechargement : lu à l'état, pas seulement au flux.
    n.aller(&page, prete);
    n.attendre("[...document.querySelectorAll('#runs-a-role [data-role=\"archiviste\"] .run-a-role')].some(e => e.dataset.state === 'fini')", "la tâche de l'archiviste après un rechargement");

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Les fiches écrites pendant une conversation** (forme accordée avec
/// everything declarative et 96) : l'état du chat les rend par les Sources de la
/// mémoire déclarée dont `session` est un run de la conversation, puis les
/// Apports qui les citent — par `select_sources` et `select_apports`. Une source
/// d'une autre session n'y paraît pas.
#[test]
fn l_etat_du_chat_rend_les_fiches_ecrites_pendant_la_conversation() {
    let m = tempfile::tempdir().unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let memoire = crate::gabarits::jouable::manifeste_jouable(&gabarit, m.path()).unwrap();
    let memoire_txt = memoire.to_string_lossy().to_string();
    let (_d, host, manifeste, id) = banc_avec(qui_lit_deux_fois(), move |c| {
        c["bilans"] = json!({ "memoire": memoire_txt });
    });
    let etat = host.chat_state(&id).unwrap();
    assert_eq!(etat["memory_writes"], json!([]), "sans tour, rien : {etat}");

    let r = host.chat_turn(&id, "regarde le code").unwrap();
    let run = r["run"].as_str().unwrap().to_string();
    // Le journal du run porte `chat` : il fait partie de la conversation.
    let debut = Instant::now();
    while host.journal.lire(&id, &run).is_none() {
        assert!(debut.elapsed() < Duration::from_secs(10), "le journal du run n'est pas écrit");
        std::thread::sleep(Duration::from_millis(20));
    }
    // Ce que le juge écrit après un tour : une source datée par la session (le
    // run), un apport qui la cite ; et une source d'une autre session.
    let mem = host.hosted(&memoire).unwrap();
    let ecrire = |outil: &str, record: Value| {
        let r = host.with_backend(&mem, |b| b.call(outil, json!({ "record": record }))).unwrap();
        assert!(r.is_ok(), "{outil} : {r:?}");
    };
    ecrire("put_fiche", json!({ "name": "bilan : regarder le code", "about": "le bilan du tour", "state": "current" }));
    ecrire("put_source", json!({ "chemin": "sources/2026-11-07-ce-run.md", "titre": "bilan", "contenu": "le tour", "origine": "envoi", "session": run }));
    ecrire("put_apport", json!({ "fiche": "bilan : regarder le code", "source": "sources/2026-11-07-ce-run.md", "section": "bilan", "texte": "bien", "niveau": "L1", "state": "current" }));
    ecrire("put_source", json!({ "chemin": "sources/2026-11-07-autre.md", "titre": "autre", "contenu": "autre", "origine": "envoi", "session": "un-autre-run" }));
    ecrire("put_apport", json!({ "fiche": "bilan : regarder le code", "source": "sources/2026-11-07-autre.md", "section": "ailleurs", "texte": "x", "niveau": "L1", "state": "current" }));

    let etat = host.chat_state(&id).unwrap();
    assert_eq!(etat["memory_writes_why"], Value::Null, "{etat}");
    let ecrites = etat["memory_writes"].as_array().cloned().unwrap_or_default();
    assert_eq!(ecrites.len(), 1, "seul l'apport de la conversation : {etat}");
    let w = &ecrites[0];
    assert_eq!((w["fiche"].clone(), w["kind"].clone(), w["section"].clone(), w["run"].clone()), (json!("bilan : regarder le code"), json!("apport"), json!("bilan"), json!(run)), "{w}");
    assert!(w["at_ms"].as_u64().is_some_and(|t| t > 0), "{w}");
    let _ = manifeste;
}

/// Une mémoire sans les outils de lecture par filtre (un gabarit plus ancien) :
/// la liste ne se fait pas, et l'état dit pourquoi au lieu de rendre une liste vide
/// muette.
#[test]
fn sans_outil_de_lecture_la_memoire_dit_pourquoi_la_liste_est_vide() {
    let m = tempfile::tempdir().unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let memoire = crate::gabarits::jouable::manifeste_jouable(&gabarit, m.path()).unwrap();
    let mut v: Value = serde_json::from_slice(&std::fs::read(&memoire).unwrap()).unwrap();
    v["tools"].as_object_mut().unwrap().remove("select_sources");
    std::fs::write(&memoire, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    let memoire_txt = memoire.to_string_lossy().to_string();
    let (_d, host, _manifeste, id) = banc_avec(qui_lit_deux_fois(), move |c| {
        c["bilans"] = json!({ "memoire": memoire_txt });
    });
    // Tant que la mémoire n'est pas ouverte dans l'hôte, la page ne la fait pas
    // ouvrir : elle le dit.
    let etat = host.chat_state(&id).unwrap();
    assert!(etat["memory_writes_why"].as_str().is_some_and(|w| w.contains("pas ouverte")), "{etat}");
    let mem = host.hosted(&memoire).unwrap();
    host.with_backend(&mem, |_| ()).expect("la mémoire s'ouvre");
    host.chat_turn(&id, "regarde le code").unwrap();
    let debut = Instant::now();
    loop {
        let etat = host.chat_state(&id).unwrap();
        if let Some(pourquoi) = etat["memory_writes_why"].as_str() {
            assert!(pourquoi.contains("select_sources"), "{etat}");
            break;
        }
        assert!(debut.elapsed() < Duration::from_secs(10), "l'état ne dit pas pourquoi : {etat}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Le faux modèle rapide : quarante mots d'un trait, sans pause — le premier
/// jeton part aussitôt le tour commencé.
fn rapide() -> ChatModel {
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-rapide", 8192, |turns, opts, sink| {
            let texte = (0..40).map(|n| format!("mot{n}")).collect::<Vec<_>>().join(" ");
            MockLlm::new(&texte).generate(turns, opts, sink)
        })))
    })
}

/// **Le premier jeton arrive dans la page** (ticket du 7 novembre 2026,
/// « l'affichage en direct du chat peut manquer le premier jeton du premier
/// tour ») : huit fois, une page fraîche envoie son premier tour à un modèle
/// rapide ; pour chaque tour, la page relève les rangs (`seq`) reçus du flux et
/// ceux qu'elle a posés dans la bulle. Tous les rangs émis doivent être reçus
/// et posés, le rang 0 compris. Un rang absent des reçus accuse le flux ; reçu
/// mais pas posé, la page.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn le_premier_jeton_d_un_tour_arrive_dans_la_page() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_premier_jeton_d_un_tour_arrive_dans_la_page";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, host, manifeste, id) = banc(rapide());
    drop(host);
    let (chats, taches) = (d.path().join("chats"), d.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(rapide())
    });
    let racine = format!("http://{adresse}");
    let mut r = crate::daemon::agent()
        .post(format!("{racine}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": manifeste }).to_string())
        .unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["ok"], true, "{v}");

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let page = format!("{racine}/ide/code/{id}/chat");
    let mut releves = Vec::new();
    for essai in 0..8 {
        n.aller(&page, "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'");
        n.evaluer(&format!("document.querySelector('#message').value = 'dis vite {essai}'; document.querySelector('#send').click()"));
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour rapide");
        let releve = n.evaluer(
            "(() => { const run = [...document.querySelectorAll('article.message.assistant')].at(-1)?.dataset.run; \
             const t = window.__chatTokens ?? { received: {}, applied: {} }; \
             return { run, recus: t.received[run] ?? [], poses: t.applied[run] ?? [], debordements: t.missed ?? [] }; })()",
        );
        releves.push(releve);
    }
    let complet = |v: &Value| {
        let mut rangs: Vec<u64> = v.as_array().map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
        rangs.sort_unstable();
        !rangs.is_empty() && rangs.iter().enumerate().all(|(i, &r)| r == i as u64)
    };
    let manques: Vec<&Value> = releves.iter().filter(|r| !complet(&r["recus"]) || !complet(&r["poses"])).collect();
    assert!(
        manques.is_empty(),
        "des rangs émis manquent ({} tour(s) sur 8) — absent des reçus : le flux ; reçu mais pas posé : la page : {manques:#?}",
        manques.len()
    );
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **La nature d'un backend, sur la forme réelle d'une configuration** (la pose
/// du 7 novembre 2026 chez Lucie : son code et sa mémoire paraissaient sous
/// « Projets »). Un manifeste de code rangé dans un dossier de configuration,
/// hors de tout dépôt, sans `kind` (une copie d'avant le champ), dont
/// `workspace.root` est un dépôt : l'état le dit `code`, par repli, avec le nom
/// et la branche du dépôt de son espace de travail. Une mémoire instanciée du
/// gabarit, sans `bilans` déclaré nulle part : `memory`, par son manifeste.
#[test]
fn la_nature_d_un_backend_vient_de_son_gabarit_sinon_de_son_espace_de_travail() {
    let d = tempfile::tempdir().unwrap();
    // Le dépôt indexé, sur sa branche.
    let depot = d.path().join("atelier");
    std::fs::create_dir_all(&depot).unwrap();
    let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&depot).args(args).status().unwrap().success(), "git {args:?}");
    git(&["init", "-q", "-b", "principale"]);
    git(&["config", "user.name", "Personne Test"]);
    git(&["config", "user.email", "personne@example.org"]);
    std::fs::write(depot.join("main.rs"), "fn main() {}\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "un"]);
    // La configuration, hors de tout dépôt : le code (sans `kind`) et la mémoire.
    let config = d.path().join("config");
    std::fs::create_dir_all(config.join("code")).unwrap();
    std::fs::create_dir_all(config.join("memoire")).unwrap();
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends");
    let code = crate::gabarits::jouable::manifeste_jouable(&gabarits.join("code"), &config.join("code")).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&code).unwrap()).unwrap();
    m.as_object_mut().unwrap().remove("kind");
    m["workspace"]["root"] = json!(depot.to_string_lossy());
    std::fs::write(&code, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let memoire = crate::gabarits::jouable::manifeste_jouable(&gabarits.join("memory"), &config.join("memoire")).unwrap();

    let host = DaemonHost::new().with_chats_root(d.path().join("chats")).with_chat_model(qui_lit_deux_fois());
    for m in [&code, &memoire] {
        let h = host.hosted(m).unwrap();
        host.with_backend(&h, |_| ()).expect("le backend s'ouvre");
    }
    let etat = host.state();
    let de = |m: &Path| {
        let m = m.canonicalize().unwrap();
        etat["backends"].as_array().unwrap().iter().find(|b| b["manifest"] == json!(m)).cloned().unwrap_or_else(|| panic!("pas de {} : {etat}", m.display()))
    };
    let c = de(&code);
    assert_eq!(
        (c["kind"].clone(), c["kind_source"].clone(), c["repo"].clone(), c["branch"].clone()),
        (json!("code"), json!("fallback"), json!("atelier"), json!("principale")),
        "{c}"
    );
    let me = de(&memoire);
    assert_eq!((me["kind"].clone(), me["kind_source"].clone()), (json!("memory"), json!("manifest")), "{me}");
}

/// Le faux modèle qui répond avec du code : du code en ligne, un chemin cité, un
/// bloc rust.
fn qui_montre_du_code() -> ChatModel {
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-code", 8192, |turns, opts, sink| {
            MockLlm::new("Voir `src/lib.rs:2` et l'outil `search_code`.\n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n").generate(turns, opts, sink)
        })))
    })
}

/// Le faux modèle qui répond par deux blocs : du rust (les grammaires de l'hôte le
/// colorent) et du yaml (highlight.js le connaît, les grammaires non : le repli).
fn qui_montre_deux_langues() -> ChatModel {
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-deux-langues", 8192, |turns, opts, sink| {
            MockLlm::new("Deux blocs.\n\n```rust\n// é\nfn main() {\n    let s = \"x\";\n}\n```\n\n```yaml\nclef: valeur\n```\n").generate(turns, opts, sink)
        })))
    })
}

/// **Le code du chat est coloré par les grammaires de l'hôte** (coloration B,
/// page 12 de codeparsers) : un bloc rust est recoloré par `POST /ide/highlight`
/// (classes `hl-*`, `data-hl="hote"`) et son texte ne change pas d'un caractère ; un
/// bloc yaml, que les grammaires ne connaissent pas, reste sur highlight.js
/// (`data-hl="repli"`, classes `hljs-*`).
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn le_code_du_chat_est_colore_par_les_grammaires_de_l_hote() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_code_du_chat_est_colore_par_les_grammaires_de_l_hote";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, host, manifeste, id) = banc(qui_montre_deux_langues());
    drop(host);
    let (chats, taches) = (d.path().join("chats"), d.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(qui_montre_deux_langues())
    });
    let racine = format!("http://{adresse}");
    let mut r = crate::daemon::agent()
        .post(format!("{racine}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": manifeste }).to_string())
        .unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["ok"], true, "{v}");
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{racine}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'");
    n.evaluer("document.querySelector('#message').value = 'montre'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
    n.attendre(
        "!![...document.querySelectorAll('article.message.assistant')].at(-1)?.querySelector('pre code.language-rust[data-hl=\"hote\"]')",
        "le bloc rust recoloré par l'hôte",
    );
    let lu = n.evaluer(
        "(() => { const a = [...document.querySelectorAll('article.message.assistant')].at(-1); \
          const rs = a.querySelector('pre code.language-rust'); const ya = a.querySelector('pre code.language-yaml'); \
          return { mot: [...rs.querySelectorAll('span.hl-keyword')].map(s => s.textContent), \
            commentaire: rs.querySelector('span.hl-comment')?.textContent ?? null, chaine: rs.querySelector('span.hl-string')?.textContent ?? null, \
            texte: rs.textContent, yaml: ya?.dataset.hl ?? null, yamlHljs: !!ya?.querySelector('[class^=\"hljs-\"]') }; })()",
    );
    assert!(lu["mot"].as_array().is_some_and(|m| m.contains(&json!("fn")) && m.contains(&json!("let"))), "les mots-clés : {lu}");
    assert_eq!(lu["commentaire"], "// é", "{lu}");
    assert_eq!(lu["chaine"], "\"x\"", "{lu}");
    assert_eq!(lu["texte"], "// é\nfn main() {\n    let s = \"x\";\n}", "le texte du bloc est intact : {lu}");
    assert_eq!((&lu["yaml"], &lu["yamlHljs"]), (&json!("repli"), &json!(true)), "le yaml reste sur highlight.js : {lu}");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Le code dans le chat se lit comme du code** (tickets du 7 novembre 2026,
/// retours de Lucie : « le texte limité en largeur », « `code.truc` pris en
/// compte », la coloration) : à 2 560 px, le texte d'une bulle occupe la
/// colonne ; le code en ligne d'une réponse et celui du message de la personne
/// sont du code, un chemin cité mène au fichier à sa ligne, un nom d'outil reste
/// du code sans lien ; un bloc rust est coloré (la coloration A).
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn le_code_du_chat_se_lit_comme_du_code() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_code_du_chat_se_lit_comme_du_code";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, host, manifeste, id) = banc(qui_montre_du_code());
    drop(host);
    let (chats, taches) = (d.path().join("chats"), d.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(qui_montre_du_code())
    });
    let racine = format!("http://{adresse}");
    let mut r = crate::daemon::agent()
        .post(format!("{racine}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": manifeste }).to_string())
        .unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["ok"], true, "{v}");

    let mut n = Navigateur::lancer(&binaire);
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 2560, "height": 1440, "deviceScaleFactor": 1, "mobile": false }));
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{racine}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'");
    n.evaluer("document.querySelector('#message').value = 'regarde `src/lib.rs:2`'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
    let lu = n.evaluer(&format!(
        "(() => {{ const a = [...document.querySelectorAll('article.message.assistant')].at(-1); const u = [...document.querySelectorAll('article.message.user')].at(-1); \
          const lien = (el) => el?.querySelector('a.code-ref[href$=\"/fichier/src/lib.rs#L2\"] code')?.textContent ?? null; \
          return {{ \
            liens: [lien(a), lien(u)], \
            outil: [...(a?.querySelectorAll('code') ?? [])].some(c => c.textContent === 'search_code' && !c.closest('a')), \
            rust: !!a?.querySelector('pre code.language-rust span.hljs-keyword, pre code.language-rust span.hl-keyword'), \
            largeur: a?.querySelector('.body')?.getBoundingClientRect().width ?? 0, \
            colonne: document.querySelector('#messages').getBoundingClientRect().width, \
          }}; }})()"
    ));
    assert_eq!(lu["liens"], json!(["src/lib.rs:2", "src/lib.rs:2"]), "le chemin cité, en code et en lien, dans la réponse et dans le message : {lu}");
    assert_eq!(lu["outil"], true, "le nom d'outil en code, sans lien : {lu}");
    assert_eq!(lu["rust"], true, "le bloc rust coloré : {lu}");
    let (largeur, colonne) = (lu["largeur"].as_f64().unwrap_or(0.0), lu["colonne"].as_f64().unwrap_or(1.0));
    assert!(largeur >= colonne - 120.0, "le texte de la bulle n'occupe pas la colonne ({largeur} sur {colonne}) : {lu}");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Le retour d'un outil se lit, et le brut reste exact** (ticket du 7 novembre
/// 2026, « les retours d'outils du chat n'ont pas de mise en page humaine ») : le
/// retour de read_file déplié montre ses lignes numérotées et colorées ; le
/// bouton « brut » rend le texte exact que le modèle a reçu (celui de la
/// session) ; les champs autour du résultat restent lisibles (« autres champs »).
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn le_retour_d_un_outil_se_lit_et_le_brut_reste_exact() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_retour_d_un_outil_se_lit_et_le_brut_reste_exact";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d, host, manifeste, id) = banc(qui_lit_deux_fois());
    drop(host);
    let (chats, taches) = (d.path().join("chats"), d.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(qui_lit_deux_fois())
    });
    let racine = format!("http://{adresse}");
    let post = |chemin: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent().post(format!("{racine}{chemin}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    assert_eq!(post("/ide/describe", json!({ "manifest": manifeste }))["ok"], true);
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{racine}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'");
    n.evaluer("document.querySelector('#message').value = 'regarde le code'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour");
    let etat: Value = {
        let mut r = crate::daemon::agent().get(format!("{racine}/ide/chat?backend={id}")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        serde_json::from_str::<Value>(&r.body_mut().read_to_string().unwrap()).unwrap()["result"].clone()
    };
    let recu = etat["turns"].as_array().unwrap().iter().find(|t| t["role"] == "tool" && t["tool_name"] == "read_file").and_then(|t| t["content"].as_str()).unwrap_or_default().to_string();
    assert!(!recu.is_empty(), "le retour de read_file dans la session : {etat}");
    let d = "document.querySelector('details.outil[data-outil=\"read_file\"]')";
    n.evaluer(&format!("{d}.open = true"));
    let rendu = n.evaluer(&format!(
        "(() => {{ const r = {d}.querySelector('.rendu-humain'); return {{ numero: r?.querySelector('pre.lignes-numerotees .numero')?.textContent ?? null, code: r?.textContent.includes('fn main') ?? false, autres: !!{d}.querySelector('.autres-champs') }}; }})()"
    ));
    assert_eq!(rendu["numero"], "1", "les lignes numérotées : {rendu}");
    assert_eq!(rendu["code"], true, "le code du fichier : {rendu}");
    assert_eq!(rendu["autres"], true, "les champs autour du résultat : {rendu}");
    n.evaluer(&format!("{d}.querySelector('button.brut').click()"));
    n.attendre(&format!("{d}.querySelector('.texte-brut')"), "le brut");
    assert_eq!(n.evaluer(&format!("{d}.querySelector('.texte-brut').textContent")), json!(recu), "le brut n'est pas le texte exact reçu par le modèle");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}


/// **L'état de l'hôte range sessions, tâches et espaces** (modèle tranché par c7
/// avec Lucie, forme accordée avec everything declarative) : le backend ouvert
/// est une session, avec son dépôt pour espace ; la tâche ouverte dessus est dans
/// `tasks`, `session` le backend, son espace une branche `agent/<nom>` du même
/// dépôt, sa proposition ouverte ; un backend connu et fermé est « disponible » ;
/// ni la session ni la tâche ne le sont.
#[test]
fn l_etat_de_l_hote_range_sessions_taches_et_espaces() {
    let (d, host, _manifeste, id) = banc_dans("atelier", page_du_chat(), |_| {});
    let (d2, h2, m2, id2) = banc_dans("outils", page_du_chat(), |_| {});
    drop(h2);
    host.hosted(&m2).unwrap();
    let tache = host.ouvrir_une_tache(&id, "relecture").expect("la tâche s'ouvre");
    let etat = host.state();
    let sessions = etat["chat_sessions"].as_array().cloned().unwrap_or_default();
    let session = sessions.iter().find(|s| s["id"] == json!(id)).unwrap_or_else(|| panic!("la session du backend ouvert : {}", etat["chat_sessions"]));
    assert_eq!(session["spaces"][0]["repo"], "atelier", "{session}");
    assert_eq!(session["spaces"][0]["proposal"], Value::Null, "{session}");
    assert!(!sessions.iter().any(|s| s["id"] == json!(tache.id)), "une tâche n'est pas une session : {sessions:?}");
    let tasks = etat["tasks"].as_array().cloned().unwrap_or_default();
    let t = tasks.iter().find(|t| t["id"] == json!(tache.id)).unwrap_or_else(|| panic!("la tâche : {}", etat["tasks"]));
    assert_eq!((&t["session"], &t["name"], &t["parent_task"]), (&json!(id), &json!("relecture"), &Value::Null), "{t}");
    assert_eq!(t["spaces"][0]["repo"], "atelier", "le dépôt de la tâche, pas le dossier de son worktree : {t}");
    assert_eq!(t["spaces"][0]["branch"], "agent/relecture", "{t}");
    assert_eq!(t["spaces"][0]["proposal"]["state"], "open", "{t}");
    let disponibles = etat["available_repos"].as_array().cloned().unwrap_or_default();
    assert!(disponibles.iter().any(|r| r["backend"] == json!(id2) && r["repo"] == "outils" && r["loaded"] == false), "le backend fermé est disponible : {disponibles:?}");
    assert!(!disponibles.iter().any(|r| r["backend"] == json!(id) || r["backend"] == json!(tache.id)), "{disponibles:?}");
    host.fermer_une_tache(&id, "relecture").unwrap();
    drop((d, d2));
}

/// **La barre se lit par session, tâche et espace** (ticket « la barre du chat ne
/// se lit pas par session », forme tranchée avec Lucie) : la session de la page est
/// dépliée, sa tâche repliée ; dépliée, la tâche montre son espace (dépôt ·
/// `agent/<nom>`) et qui le tient ; le menu « par dépôt » inverse l'arbre (le
/// dépôt, ses espaces, la session et la tâche qui les tiennent) et se souvient
/// du choix au rechargement ; « actifs seulement » retire ce qui ne tourne pas
/// (la page reste) ; sur la page de la tâche, le fil d'Ariane dit session ›
/// tâche › dépôt · branche.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn la_barre_se_lit_par_session_tache_et_espace() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "la_barre_se_lit_par_session_tache_et_espace";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d1, h1, m1, id1) = banc_dans("atelier", page_du_chat(), |_| {});
    let (d2, h2, m2, _id2) = banc_dans("outils", page_du_chat(), |_| {});
    drop((h1, h2));
    let (chats, taches) = (d1.path().join("chats"), d1.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(page_du_chat())
    });
    let racine = format!("http://{adresse}");
    let ide = |chemin: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}{chemin}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    for m in [&m1, &m2] {
        let v = ide("/ide/describe", json!({ "manifest": m }));
        assert_eq!(v["ok"], true, "{v}");
    }
    let o = ide("/ide/taches", json!({ "backend": id1, "nom": "relecture" }));
    let t = o["result"]["id"].as_str().map(str::to_string).or_else(|| o["result"]["tache"]["id"].as_str().map(str::to_string)).unwrap_or_else(|| panic!("la tâche s'ouvre : {o}"));

    let mut n = Navigateur::lancer(&binaire);
    n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.evaluer("try { localStorage.removeItem('rag3weaver.ide.barre.vue') } catch {}");
    n.aller(&format!("{racine}/ide/code/{id1}/chat"), &format!("!!document.querySelector('#arbre-sessions .session[data-session=\"{id1}\"]')"));
    let lu = n.evaluer(&format!(
        "(() => {{ const s = document.querySelector('.session[data-session=\"{id1}\"]'); const t = s?.querySelector('.tache[data-task=\"{t}\"]'); \
          const autre = [...document.querySelectorAll('.session')].find(x => x.dataset.session !== '{id1}'); \
          return {{ session: s?.dataset.open ?? null, tache: t?.dataset.open ?? null, autre: autre?.dataset.open ?? null }}; }})()"
    ));
    assert_eq!(lu, json!({ "session": "true", "tache": "false", "autre": "false" }), "la page dépliée, le reste replié : {lu}");
    n.evaluer(&format!("document.querySelector('.tache[data-task=\"{t}\"] > button.repli').click()"));
    n.attendre(&format!("!!document.querySelector('.tache[data-task=\"{t}\"] .espace[data-backend=\"{t}\"]')"), "la tâche dépliée");
    let espace = n.evaluer(&format!("document.querySelector('.tache[data-task=\"{t}\"] .espace[data-backend=\"{t}\"]').textContent"));
    let espace = espace.as_str().unwrap_or_default().to_string();
    assert!(espace.contains("atelier · agent/relecture"), "l'espace de la tâche : {espace}");
    assert!(espace.contains("tenu par relecture (atelier)"), "qui tient l'espace, côté session : {espace}");

    // Le menu : par dépôt, l'arbre s'inverse ; le choix se garde.
    n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'repo'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
    n.attendre("!!document.querySelector('#arbre-depots')", "la vue par dépôt");
    let depot = n.evaluer(&format!(
        "(() => {{ const d = document.querySelector('#arbre-depots .depot[data-depot=\"atelier\"]'); \
          return [...(d?.querySelectorAll('.espace') ?? [])].map(e => [e.dataset.backend, e.querySelector('.tenu-par')?.textContent ?? null]); }})()"
    ));
    assert_eq!(depot, json!([[id1, "tenu par atelier"], [t, "tenu par relecture (atelier)"]]), "le dépôt, ses espaces et qui les tient : {depot}");
    n.aller(&format!("{racine}/ide/code/{id1}/chat"), "!!document.querySelector('#barre')");
    assert_eq!(n.evaluer("document.querySelector('#barre').dataset.view"), json!("repo"), "le choix de vue s'est perdu au rechargement");
    n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'session'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
    n.attendre("!!document.querySelector('#arbre-sessions')", "retour à la vue par session");

    // Actifs seulement : rien ne tourne, la session de la page reste.
    n.evaluer("document.querySelector('#actifs-seulement').click()");
    n.attendre(&format!("!document.querySelector('.tache[data-task=\"{t}\"]')"), "la tâche inactive retirée");
    let restent = n.evaluer("[...document.querySelectorAll('#arbre-sessions .session')].map(s => s.dataset.session)");
    assert_eq!(restent, json!([id1]), "seule la session de la page reste : {restent}");

    // Le fil d'Ariane, sur la page de la tâche.
    n.aller(&format!("{racine}/ide/code/{t}/chat"), "!!document.querySelector('#fil-chat')");
    let fil_ariane = n.evaluer("document.querySelector('#fil-chat').textContent");
    assert_eq!(fil_ariane, json!("atelier › relecture › atelier · agent/relecture"), "{fil_ariane}");
    let texte = n.evaluer("document.querySelector('#barre').innerText.toLowerCase()");
    assert!(!texte.as_str().unwrap_or("").contains("backend"), "« backend » paraît dans la barre : {texte}");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    drop(d2);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Un appel à délai s'arrête à son délai** (ticket du 7 novembre 2026, « une
/// recherche continue sur l'hôte après l'abandon de son client ») : un `/call`
/// porte `deadline_ms` ; passé ce délai, l'hôte arrête lui-même le run du
/// graphe par le chemin d'arrêt de tout run (`cancelled_by: "deadline"`), et
/// l'appel rend « délai dépassé (N ms) » au lieu de tourner jusqu'au bout pour
/// un client parti. Un outil lent (`wait_output` sur un journal où rien ne
/// viendra, une minute d'attente) sert de graphe lent. Sans `deadline_ms`,
/// rien ne change (le même outil, un délai d'outil court, rend son résultat).
#[test]
fn un_appel_a_delai_s_arrete_a_son_delai() {
    let (_d, host, manifeste, id) = banc(qui_lit_deux_fois());
    let session = host.open_session("claude-code", None, Vec::new(), None);
    let h = host.hosted(&manifeste).unwrap();
    let mut dataflow = host.with_backend(&h, |b| b.bus_de_publication().subscribe(crate::events::topic::DATAFLOW)).unwrap();
    let journal = crate::dataflow::run_nodes::dossier_journaux().join(format!("temoin-delai-{}.log", std::process::id()));
    let corps = |delai: Option<u64>, timeout_s: u64| {
        let mut v = json!({ "manifest": manifeste, "name": "wait_output", "arguments": { "journal": journal.to_string_lossy(), "pattern": "jamais", "timeout_s": timeout_s } });
        if let Some(ms) = delai {
            v["deadline_ms"] = json!(ms);
        }
        serde_json::to_vec(&v).unwrap()
    };

    let debut = Instant::now();
    let (statut, r) = host.handle("POST", "/call", Some(&session), &corps(Some(800), 60));
    let duree = debut.elapsed();
    assert!(duree < Duration::from_secs(10), "l'appel à 800 ms de délai a tourné {duree:?} : l'hôte n'applique pas le délai ({r})");
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error_kind"], "deadline", "{r}");
    assert!(r["error"].as_str().is_some_and(|e| e.contains("délai dépassé (800 ms)")), "{r}");
    assert_eq!(statut, 422, "{r}");
    // Le run du graphe a fini, arrêté au nom du délai.
    let fin = Instant::now() + Duration::from_secs(10);
    let mut vus = Vec::new();
    loop {
        match dataflow.try_recv() {
            Ok(e) => {
                let v = e.to_json();
                if v["kind"] == "RunFinished" && v["cancelled_by"] == "deadline" {
                    break;
                }
                vus.push(v);
            }
            Err(_) => {
                assert!(Instant::now() < fin, "aucun RunFinished arrêté par le délai : {vus:?}");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    // Sans délai d'appel, rien ne change : l'outil rend son résultat (son propre
    // délai, une seconde, passé).
    let (_, r) = host.handle("POST", "/call", Some(&session), &corps(None, 1));
    assert_eq!(r["ok"], true, "{r}");
    assert!(r.get("error_kind").is_none(), "{r}");
    let _ = id;
}

/// Une tâche à rôle sans fin lisible se dit « inconnu », pas « fini »
/// (relecture d'everything declarative) ; sans rôle, ce n'en est pas une.
#[test]
fn une_tache_a_role_sans_fin_lisible_se_dit_inconnue() {
    let debut = json!({ "kind": "RunStarted", "target": { "role": "archiviste", "tool": "list_files" } });
    let t = super::tache_a_role("graph-19a0b1c2d3e-1", &json!({ "evenements": [debut.clone()] })).expect("une tâche à rôle");
    assert_eq!(t["state"], "inconnu");
    assert_eq!(t["at_ms"], json!(u64::from_str_radix("19a0b1c2d3e", 16).unwrap()));
    let finie = super::tache_a_role("graph-1-1", &json!({ "evenements": [debut, { "kind": "RunFinished", "ok": false }] })).unwrap();
    assert_eq!(finie["state"], "échec");
    assert!(super::tache_a_role("graph-1-2", &json!({ "evenements": [{ "kind": "RunStarted", "target": {} }] })).is_none());
}

/// **Le thème daemon dans l'IDE** (Lucie, 7 novembre 2026 : l'IDE « comme un terminal
/// avant-gardiste », le même thème que le terminal ; c7, 8 novembre). Un troisième choix
/// « daemon » à côté de clair et sombre : la page applique les couleurs du fichier commun
/// `templates/themes/daemon.json`, servi par l'hôte (`/ide/theme/daemon.json`), le choix tient
/// au rechargement ; pendant qu'un tour attend le modèle, l'animation « réfléchit » du thème
/// tourne, et elle disparaît à la fin du tour. Hors du thème daemon, la page reste sobre.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn le_theme_daemon_se_choisit_et_l_invocation_tourne_pendant_que_le_modele_reflechit() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_theme_daemon_se_choisit_et_l_invocation_tourne_pendant_que_le_modele_reflechit";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    // Un modèle qui réfléchit deux secondes avant de répondre, sans jeton entre-temps.
    let lent: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-lent", 8192, |turns, opts, sink| {
            std::thread::sleep(Duration::from_millis(2000));
            MockLlm::new("Voilà.").generate(turns, opts, sink)
        })))
    });
    let (d, h, m, id) = banc_dans("atelier", lent.clone(), |_| {});
    drop(h);
    let (chats, taches, runs) = (d.path().join("chats"), d.path().join("taches"), d.path().join("runs"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_runs_root(runs).with_chat_model(lent)
    });
    let racine = format!("http://{adresse}");
    let mut r = crate::daemon::agent()
        .post(format!("{racine}/ide/describe"))
        .header("Cookie", "rag3weaver_ide=j")
        .header("content-type", "application/json")
        .send(json!({ "manifest": m }).to_string())
        .unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["ok"], true, "{v}");
    // Le thème que l'hôte sert est le fichier commun, octet pour octet.
    let commun = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/themes/daemon.json")).unwrap();
    let servi = crate::daemon::agent().get(format!("{racine}/ide/theme/daemon.json")).header("Cookie", "rag3weaver_ide=j").call().unwrap().body_mut().read_to_string().unwrap();
    assert_eq!(servi, commun, "l'hôte sert templates/themes/daemon.json");
    let theme: Value = serde_json::from_str(&commun).unwrap();

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&format!("{racine}/ide/code/{id}/chat"), prete);
    // Le choix « daemon » : la marque sur <html>, les couleurs du fichier commun.
    n.evaluer("document.querySelector('#theme-daemon').click()");
    n.attendre("document.documentElement.dataset.theme === 'daemon' && document.documentElement.classList.contains('daemon')", "le thème daemon choisi");
    let var = |n: &mut Navigateur, v: &str| n.evaluer(&format!("getComputedStyle(document.documentElement).getPropertyValue('{v}').trim()"));
    n.attendre(&format!("getComputedStyle(document.documentElement).getPropertyValue('--daemon-fond').trim() === '{}'", theme["couleurs"]["fond"].as_str().unwrap()), "les couleurs lues dans le fichier commun");
    assert_eq!(var(&mut n, "--daemon-texte"), theme["couleurs"]["texte"]);
    assert_eq!(var(&mut n, "--daemon-code-keyword"), theme["code"]["keyword"]);
    assert_eq!(n.evaluer("getComputedStyle(document.body).backgroundColor"), json!("rgb(5, 5, 5)"), "le fond de la page est celui du thème");
    assert_eq!(n.evaluer("getComputedStyle(document.body).color"), json!("rgb(255, 51, 51)"), "le texte de la page est celui du thème");
    assert_eq!(n.evaluer("localStorage.getItem('ide-theme')"), json!("daemon"));
    n.aller(&format!("{racine}/ide/code/{id}/chat"), prete);
    assert_eq!(n.evaluer("document.documentElement.dataset.theme"), json!("daemon"), "le choix tient au rechargement");

    // Un tour : l'invocation tourne pendant que le modèle réfléchit, puis disparaît.
    assert_eq!(n.evaluer("!!document.querySelector('#invocation')"), json!(false), "pas d'invocation hors d'un tour");
    n.evaluer("document.querySelector('#message').value = 'réfléchis'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#invocation')?.dataset.anim === 'reflechit' && document.querySelector('#invocation').textContent.includes('INVOCATION DU CODE')", "l'invocation pendant que le modèle réfléchit");
    let image = n.evaluer("document.querySelector('#invocation').textContent");
    assert!(image.as_str().unwrap_or_default().contains('⛧'), "le cœur du cercle : {image}");
    n.attendre(&format!("document.querySelector('#invocation') && document.querySelector('#invocation').textContent !== {image}"), "le cercle tourne");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "la fin du tour");
    assert_eq!(n.evaluer("!!document.querySelector('#invocation')"), json!(false), "l'invocation disparaît à la fin du tour");

    // Hors du thème daemon (retour au choix du système), la page reste sobre pendant un tour.
    n.evaluer("document.querySelector('#theme-daemon').click()");
    n.attendre("document.documentElement.dataset.theme !== 'daemon' && !document.documentElement.classList.contains('daemon')", "le thème daemon quitté");
    n.evaluer("document.querySelector('#message').value = 'encore'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === \"l'agent travaille…\"", "le second tour part");
    assert_eq!(n.evaluer("!!document.querySelector('#invocation')"), json!(false), "pas d'invocation hors du thème daemon");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "la fin du second tour");

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    drop(d);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **Un symbole cité par le modèle mène à sa définition** (ticket « le code dans le chat
/// n'est ni coloré ni relié au dépôt », dernier point). La réponse cite `un()`, `double`,
/// `inconnu_xyz` et `Ok` : `un()` devient un lien vers src/lib.rs à sa ligne, `double`
/// (deux définitions) un lien vers la vue du symbole, `inconnu_xyz` reste du code, et `Ok`
/// (un mot du langage) n'est même pas demandé à l'hôte.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn un_symbole_cite_par_le_modele_mene_a_sa_definition() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "un_symbole_cite_par_le_modele_mene_a_sa_definition";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let cite: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-cite", 8192, |turns, opts, sink| {
            MockLlm::new("Voir `un()`, puis `double`, `inconnu_xyz` et `Ok`.").generate(turns, opts, sink)
        })))
    });
    let (d, h, m, id) = banc_dans("atelier", cite.clone(), |_| {});
    drop(h);
    let code = d.path().join("code");
    std::fs::create_dir_all(code.join("src")).unwrap();
    std::fs::write(code.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n").unwrap();
    std::fs::write(code.join("src/a.rs"), "pub fn double() -> u32 {\n    2\n}\n").unwrap();
    std::fs::write(code.join("src/b.rs"), "pub fn double() -> u32 {\n    4\n}\n").unwrap();
    let (chats, taches, runs) = (d.path().join("chats"), d.path().join("taches"), d.path().join("runs"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_runs_root(runs).with_chat_model(cite)
    });
    let racine = format!("http://{adresse}");
    let m = m.to_string_lossy().to_string();
    crate::daemon_host::symboles_cites::tests::indexer_et_attendre(&adresse, &m, "double", 2);

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
    n.aller(&format!("{racine}/ide/code/{id}/chat"), prete);
    n.evaluer("document.querySelector('#message').value = 'où sont-ils ?'; document.querySelector('#send').click()");
    n.attendre("document.querySelector('#status')?.textContent === 'fini'", "la fin du tour");
    n.attendre(
        &format!("!!document.querySelector('.markdown a.code-ref.symbole[data-symbol=\"un\"][href$=\"/ide/code/{id}/fichier/src/lib.rs#L1\"]')"),
        "un() en lien vers sa définition",
    );
    n.attendre(&format!("!!document.querySelector('.markdown a.code-ref.symbole[data-symbol=\"double\"][href$=\"/ide/code/{id}/symbole/double\"]')"), "double en lien vers la vue du symbole");
    assert_eq!(n.evaluer("[...document.querySelectorAll('.markdown a.code-ref')].some(a => a.textContent === 'inconnu_xyz')"), json!(false), "un nom inconnu n'est pas un lien");
    assert_eq!(n.evaluer("[...document.querySelectorAll('.markdown code')].some(c => c.textContent === 'inconnu_xyz')"), json!(true), "il reste du code");
    // Un fichier change sous la montre : le cache des symboles se vide, le lien suit la
    // définition à sa nouvelle ligne (une redemande expliquée, pas un rendu qui redemande).
    std::fs::write(code.join("src/lib.rs"), "\n\npub fn un() -> u32 {\n    1\n}\n").unwrap();
    n.attendre("(window.__ide?.symbolesOublis ?? []).length > 0", "l'oubli des symboles quand un fichier change");
    n.attendre(
        &format!("!!document.querySelector('.markdown a.code-ref.symbole[data-symbol=\"un\"][href$=\"/ide/code/{id}/fichier/src/lib.rs#L3\"]')"),
        "le lien suit la définition à sa nouvelle ligne",
    );
    let demandes = n.evaluer("window.__ide?.symbolesDemandes ?? null");
    let demandes: Vec<String> = serde_json::from_value(demandes.clone()).unwrap_or_else(|_| panic!("la page tient la liste des noms demandés : {demandes}"));
    assert!(demandes.iter().any(|s| s == "un") && !demandes.iter().any(|s| s == "Ok"), "Ok est un mot du langage, jamais demandé : {demandes:?}");
    let releve = n.evaluer("({ lots: window.__ide?.symbolesLots ?? null, oublis: window.__ide?.symbolesOublis ?? [] })");
    // Chaque lot au-delà du premier s'explique : par une réponse sans lecture (`why`, la
    // relance prévue) ou par un oubli (un fichier changé, la fin de sa réindexation) — jamais
    // par un simple rendu (rouge de 96 sous charge, 8 novembre 2026 : deux lots entiers) ;
    // et un lot ne répète aucun nom.
    let lots: Vec<Value> = releve["lots"].as_array().cloned().unwrap_or_default();
    let oublis = releve["oublis"].as_array().map(Vec::len).unwrap_or(0);
    let sans_lecture = lots.iter().filter(|l| !l["why"].is_null()).count();
    for l in &lots {
        let noms: Vec<&str> = l["noms"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let uniques: std::collections::BTreeSet<&str> = noms.iter().copied().collect();
        assert_eq!(noms.len(), uniques.len(), "un lot répète un nom : {releve}");
    }
    let lots_de_un = lots.iter().filter(|l| l["noms"].as_array().is_some_and(|n| n.iter().any(|x| x == "un"))).count();
    assert!(lots_de_un >= 1 && lots_de_un <= 1 + sans_lecture + oublis, "une redemande sans raison (ni réponse sans lecture, ni oubli) : {demandes:?} ; relevé : {releve}");

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    drop(d);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **L'accueil parle en dépôts** (Lucie, 8 novembre 2026, sur le portable posé : l'accueil disait
/// encore « Backends », « backends/code », le chemin du manifeste sous chaque nom). Le titre est
/// « Dépôts » ; chaque carte est nommée par son dépôt et sa branche, comme la barre du chat ; le
/// chemin du manifeste n'est qu'en infobulle ; une tâche se dit par son worktree (sa branche
/// `agent/<nom>`) ; « backend » ne paraît pas à l'écran.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn l_accueil_parle_en_depots_et_en_worktrees() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "l_accueil_parle_en_depots_et_en_worktrees";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let (d1, h1, m1, id1) = banc_dans("atelier", page_du_chat(), |_| {});
    let (d2, h2, m2, id2) = banc_dans("outils", page_du_chat(), |_| {});
    drop((h1, h2));
    let (chats, taches) = (d1.path().join("chats"), d1.path().join("taches"));
    let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(page_du_chat())
    });
    let racine = format!("http://{adresse}");
    let ide = |chemin: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}{chemin}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    for m in [&m1, &m2] {
        let v = ide("/ide/describe", json!({ "manifest": m }));
        assert_eq!(v["ok"], true, "{v}");
    }
    let o = ide("/ide/taches", json!({ "backend": id1, "nom": "relecture" }));
    assert!(o["ok"] == true, "la tâche s'ouvre : {o}");

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{racine}/ide/"), "document.querySelector('#vue h1')?.textContent === 'Dépôts'");
    let nom = |n: &mut Navigateur, id: &str| n.evaluer(&format!("document.querySelector('#vue .depot[data-backend=\"{id}\"] .nom')?.textContent ?? null"));
    assert_eq!(nom(&mut n, &id1), json!("atelier · master"), "la carte nommée par son dépôt et sa branche");
    assert_eq!(nom(&mut n, &id2), json!("outils · master"));
    let infobulle = n.evaluer(&format!("document.querySelector('#vue .depot[data-backend=\"{id1}\"] .nom')?.title ?? null"));
    assert_eq!(infobulle, json!(m1.to_string_lossy()), "le chemin du manifeste en infobulle");
    let texte = n.evaluer("document.querySelector('#vue').innerText");
    let texte = texte.as_str().unwrap_or_default().to_string();
    assert!(!texte.contains(&*m1.to_string_lossy()), "le chemin du manifeste ne s'affiche plus en clair : {texte}");
    assert!(!texte.to_lowercase().contains("backend"), "« backend » paraît à l'accueil : {texte}");
    let worktree = n.evaluer(&format!("document.querySelector('#vue .depot[data-backend=\"{id1}\"] .worktree')?.textContent ?? null"));
    assert!(worktree.as_str().is_some_and(|w| w.contains("agent/relecture")), "la tâche dite par son worktree : {worktree}");
    // Le lien vers l'arbre reste, celui que les autres témoins suivent.
    assert_eq!(n.evaluer(&format!("!!document.querySelector('#vue a[href=\"/ide/code/{id1}/arbre/\"]')")), json!(true));

    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    drop(d2);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
