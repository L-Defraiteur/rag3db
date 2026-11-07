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
    Arc::new(move |_b: &Backend| -> Result<Arc<dyn Llm>, String> {
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

/// Le faux modèle de la page du chat : « deux outils » fait « mots, outil, mots,
/// outil, réponse » ; « long » parle soixante lignes, une toutes les 40 ms (un
/// flux de soixante événements), puis finit.
fn page_du_chat() -> ChatModel {
    Arc::new(|_b: &Backend| -> Result<Arc<dyn Llm>, String> {
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
    let contexte = etat_du_chat()["context"].clone();
    let jauge = n.evaluer(&format!("(() => {{ const u = document.querySelector('.conversation[data-backend=\"{id}\"] .usage'); return {{ used: u?.dataset.used, max: u?.dataset.max }}; }})()"));
    assert_eq!(jauge["max"], "8192", "{jauge} ; l'hôte : {contexte}");
    assert_eq!(jauge["used"], contexte["used"].to_string(), "{jauge} ; l'hôte : {contexte}");
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
