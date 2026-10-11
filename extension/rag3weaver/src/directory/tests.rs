//! Les témoins de l'annuaire et du verbe `send` (lot 3) : c'est l'**agent**
//! qui répond à l'autre par `send`, pas le test.
use super::*;
use crate::agent::{Agent, CallbackToolBox, StopReason, SEND};
use crate::events::{Event, EventBus};
use crate::llm::{CallbackLlm, FinishReason, Llm, MockLlm, StringSink, Turn};
use std::sync::{Arc, Mutex};

// ── L'annuaire seul ────────────────────────────────────────────────────

#[test]
fn a_participant_is_found_by_name_or_by_run() {
    let d = Directory::new();
    d.register("alice", "run-a");
    d.register("bob", "run-b");
    let bob = Recipient { name: "bob".into(), run: "run-b".into() };
    assert_eq!(d.resolve("bob"), Some(bob.clone()));
    assert_eq!(d.resolve("run-b"), Some(bob));
    assert_eq!(d.resolve("carol"), None);
    assert_eq!(d.names(), ["alice", "bob"]);
}

#[test]
fn a_send_resolves_its_recipients_and_says_why_it_cannot() {
    let d = Directory::new();
    d.register("alice", "run-a");
    d.register("bob", "run-b");
    d.register("carol", "run-c");

    let err = d.recipients("alice", "dave", "").unwrap_err();
    assert!(err.contains("dave") && err.contains("alice, bob, carol"), "{err}");
    assert!(d.recipients("alice", "alice", "").unwrap_err().contains("soi-même"));
    assert!(d.recipients("alice", "", "").unwrap_err().contains("ni destinataire"));
    assert!(d.recipients("alice", "", "fil").unwrap_err().contains("personne"));

    // Écrire à bob dans un fil y inscrit les deux ; carol y entre par l'hôte.
    assert_eq!(d.recipients("alice", "bob", "fil").unwrap().len(), 1);
    assert_eq!(d.participants("fil"), ["alice", "bob"]);
    d.join("fil", "carol");
    // À tout le fil : tous sauf l'expéditeur.
    let all: Vec<_> = d.recipients("bob", "", "fil").unwrap().into_iter().map(|r| r.name).collect();
    assert_eq!(all, ["alice", "carol"]);
}

#[test]
fn enrolling_opens_the_inbox_before_the_run() {
    let bus = EventBus::new(16);
    let d = Directory::new();
    d.enroll(&bus, "bob", "run-b");
    bus.send_message("run-a", "alice", "run-b", "tôt");
    let cursor = bus.cursor(&crate::events::inbox_topic("run-b"), crate::agent::AGENT_INBOX_CURSOR);
    let got = cursor.lock().unwrap().try_recv();
    assert!(matches!(got, Ok(Event::Message { content, .. }) if content == "tôt"));
}

// ── Le verbe `send`, joué par des agents ────────────────────────────────

/// Un modèle qui joue une suite de [`MockLlm`], un par appel, rejoue le
/// dernier ensuite, et garde ce qu'il a lu à chaque appel.
fn scripted(steps: Vec<MockLlm>, read: Arc<Mutex<Vec<Vec<Turn>>>>) -> CallbackLlm {
    let steps = Mutex::new((steps, 0usize));
    CallbackLlm::new("scripted", 4096, move |turns, opts, sink| {
        read.lock().unwrap().push(turns.to_vec());
        let step = {
            let mut g = steps.lock().unwrap();
            let i = g.1.min(g.0.len() - 1);
            g.1 += 1;
            g.0[i].clone()
        };
        step.generate(turns, opts, sink)
    })
}

fn send(to: &str, content: &str, conversation: &str) -> MockLlm {
    let args = serde_json::json!({"to": to, "content": content, "conversation": conversation}).to_string();
    MockLlm::new("").with_tool_calls(vec![(SEND, args.as_str())])
}

fn no_tools() -> CallbackToolBox {
    CallbackToolBox::new(Vec::new(), |call| format!(r#"{{"error":"unknown_tool","detail":"{}"}}"#, call.name))
}

fn users(turns: &[Turn]) -> Vec<String> {
    turns.iter().filter(|t| t.role == "user").map(|t| t.content.clone()).collect()
}

/// **L'agent répond à l'autre par `send`** : alice écrit à bob par son nom ;
/// bob lit le message à son run, et c'est **son modèle** qui choisit de lui
/// répondre par `send` ; alice lit la réponse à son tour suivant. Le test ne
/// relaie rien : il lance les runs, c'est tout.
#[test]
fn an_agent_answers_another_by_send() {
    let bus = EventBus::new(256);
    let mut messages = bus.subscribe(crate::events::topic::MESSAGES);
    let dir = Arc::new(Directory::new());
    let tools = no_tools();

    let alice_read = Arc::default();
    let alice_llm = scripted(
        vec![send("bob", "quelle heure est-il ?", ""), MockLlm::new("j'ai demandé à bob"), MockLlm::new("bob dit midi")],
        Arc::clone(&alice_read),
    );
    let bob_read = Arc::default();
    let bob_llm = scripted(vec![send("alice", "midi", ""), MockLlm::new("répondu")], Arc::clone(&bob_read));
    let alice = Agent::new(&alice_llm, &tools)
        .with_name("alice")
        .with_run_id("run-a")
        .with_events(bus.clone())
        .with_inbox()
        .with_messaging(dir.clone())
        .unwrap();
    let bob = Agent::new(&bob_llm, &tools)
        .with_name("bob")
        .with_run_id("run-b")
        .with_events(bus.clone())
        .with_inbox()
        .with_messaging(dir.clone())
        .unwrap();
    // L'hôte inscrit les participants de la session.
    dir.enroll(&bus, "alice", "run-a");
    dir.enroll(&bus, "bob", "run-b");

    // 1. alice écrit à bob, et s'arrête là.
    let mut alice_turns = vec![Turn::user("demande l'heure à bob")];
    let run = alice.run(&mut alice_turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert!(alice_turns[2].content.contains("bob"), "{:?}", alice_turns[2].content);

    // 2. bob lit, et répond — son modèle a choisi `send`.
    let mut bob_turns = vec![Turn::user("tu es bob ; réponds à qui t'écrit")];
    bob.run(&mut bob_turns, &mut StringSink::default()).unwrap();
    assert_eq!(users(&bob_turns)[1], "[message de alice] quelle heure est-il ?");

    // 3. alice, à son tour suivant, lit la réponse de bob.
    alice_turns.push(Turn::user("alors ?"));
    let run = alice.run(&mut alice_turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.text, "bob dit midi");
    assert!(users(&alice_turns).contains(&"[message de bob] midi".to_string()), "{alice_turns:#?}");
    let last_read = alice_read.lock().unwrap().last().cloned().unwrap();
    assert!(users(&last_read).contains(&"[message de bob] midi".to_string()));

    // Sur le bus : deux messages, chacun vers le run de l'autre.
    let sent: Vec<(String, String, String)> = std::iter::from_fn(|| messages.try_recv().ok())
        .filter_map(|e| match e {
            Event::Message { from, to, content, .. } => Some((from, to, content)),
            _ => None,
        })
        .collect();
    assert_eq!(
        sent,
        [
            ("alice".to_string(), "run-b".to_string(), "quelle heure est-il ?".to_string()),
            ("bob".to_string(), "run-a".to_string(), "midi".to_string()),
        ]
    );
}

/// **À tout un fil** : sans `to`, le message part à chaque participant du
/// fil sauf l'expéditeur, et le tour reçu dit le fil.
#[test]
fn a_send_to_a_thread_reaches_every_other_participant_and_says_the_thread() {
    let bus = EventBus::new(256);
    let dir = Arc::new(Directory::new());
    let tools = no_tools();
    for (name, run) in [("alice", "run-a"), ("bob", "run-b"), ("carol", "run-c")] {
        dir.enroll(&bus, name, run);
        dir.join("revue", name);
    }
    let llm = scripted(vec![send("", "la revue commence", "revue"), MockLlm::new("annoncé")], Arc::default());
    let bob = Agent::new(&llm, &tools)
        .with_name("bob")
        .with_run_id("run-b")
        .with_events(bus.clone())
        .with_inbox()
        .with_messaging(dir.clone())
        .unwrap();
    let mut turns = vec![Turn::user("annonce la revue")];
    bob.run(&mut turns, &mut StringSink::default()).unwrap();
    assert!(turns[2].content.contains("alice") && turns[2].content.contains("carol"), "{}", turns[2].content);

    for (name, run) in [("alice", "run-a"), ("carol", "run-c")] {
        let read = Arc::new(Mutex::new(Vec::new()));
        let llm = scripted(vec![MockLlm::new("vu")], Arc::clone(&read));
        let agent = Agent::new(&llm, &tools).with_name(name).with_run_id(run).with_events(bus.clone()).with_inbox();
        let mut turns = vec![Turn::user("attends")];
        agent.run(&mut turns, &mut StringSink::default()).unwrap();
        assert_eq!(users(&turns)[1], "[message de bob, fil revue] la revue commence", "{name}");
    }
    // bob ne s'est pas écrit à lui-même.
    let cursor = bus.cursor(&crate::events::inbox_topic("run-b"), crate::agent::AGENT_INBOX_CURSOR);
    assert!(cursor.lock().unwrap().try_recv().is_err());
}

/// **Un envoi refusé nourrit le tour suivant** : destinataire inconnu (avec
/// la liste des inscrits), contenu vide — une erreur lisible, la boucle
/// continue.
#[test]
fn a_refused_send_feeds_the_next_turn() {
    let bus = EventBus::new(64);
    let dir = Arc::new(Directory::new());
    dir.enroll(&bus, "alice", "run-a");
    dir.enroll(&bus, "bob", "run-b");
    let tools = no_tools();
    let llm = scripted(vec![send("dave", "bonjour", ""), send("bob", "  ", ""), MockLlm::new("tant pis")], Arc::default());
    let alice = Agent::new(&llm, &tools)
        .with_name("alice")
        .with_run_id("run-a")
        .with_events(bus.clone())
        .with_messaging(dir.clone())
        .unwrap();
    let mut turns = vec![Turn::user("écris à dave")];
    let run = alice.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert!(turns[2].content.contains("unknown_recipient") && turns[2].content.contains("alice, bob"), "{}", turns[2].content);
    assert!(turns[4].content.contains("bad_argument"), "{}", turns[4].content);
    assert_eq!(run.tool_errors, 2);
}

/// **Sans `with_messaging`, rien ne change** : `send` n'est pas annoncé, et un
/// appel de ce nom va à l'outillage comme avant.
#[test]
fn without_messaging_send_is_neither_announced_nor_intercepted() {
    let seen_tools = Arc::new(Mutex::new(Vec::new()));
    let s = seen_tools.clone();
    let steps = Mutex::new(0usize);
    let llm = CallbackLlm::new("l", 4096, move |turns, opts, sink| {
        s.lock().unwrap().push(opts.tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>());
        let mut n = steps.lock().unwrap();
        *n += 1;
        let step = if *n == 1 { send("bob", "x", "") } else { MockLlm::new("fini") };
        step.generate(turns, opts, sink)
    });
    let called = Arc::new(Mutex::new(Vec::new()));
    let c = called.clone();
    let tools = CallbackToolBox::new(Vec::new(), move |call| {
        c.lock().unwrap().push(call.name.clone());
        "{}".into()
    });
    let agent = Agent::new(&llm, &tools);
    agent.run(&mut vec![Turn::user("x")], &mut StringSink::default()).unwrap();
    assert!(seen_tools.lock().unwrap().iter().all(|t| !t.contains(&SEND.to_string())));
    assert_eq!(*called.lock().unwrap(), [SEND]);

    // Avec, il est annoncé.
    let llm = scripted(vec![MockLlm::new("ok")], Arc::default());
    let agent = Agent::new(&llm, &tools).with_messaging(Arc::new(Directory::new())).unwrap();
    assert!(agent.gen_options().tools.iter().any(|t| t.name == SEND));
}


// ── Relecture de la recherche ──────────────────────────────────────────

/// **Deux agents du même nom ne se volent pas leur boîte** : le second agent
/// par défaut (« agent ») ne prend pas le nom du premier ; il ne peut pas
/// écrire sous ce nom, et un message à « agent » arrive au premier.
#[test]
fn two_agents_cannot_hold_the_same_name() {
    let bus = EventBus::new(256);
    let mut warnings = bus.subscribe(crate::events::topic::CATALOG);
    let dir = Arc::new(Directory::new());
    let tools = no_tools();
    let first_llm = scripted(vec![MockLlm::new("présent")], Arc::default());
    let first = Agent::new(&first_llm, &tools)
        .with_run_id("run-1")
        .with_events(bus.clone())
        .with_inbox()
        .with_messaging(dir.clone())
        .unwrap();
    first.run(&mut vec![Turn::user("x")], &mut StringSink::default()).unwrap();

    let second_llm = scripted(vec![send("carol", "c'est moi", ""), MockLlm::new("fini")], Arc::default());
    let second = Agent::new(&second_llm, &tools)
        .with_run_id("run-2")
        .with_events(bus.clone())
        .with_messaging(dir.clone())
        .unwrap();
    dir.enroll(&bus, "carol", "run-c");
    let mut turns = vec![Turn::user("écris à carol")];
    second.run(&mut turns, &mut StringSink::default()).unwrap();
    assert!(turns[2].content.contains("tenu par un autre agent"), "{}", turns[2].content);
    let said = std::iter::from_fn(|| warnings.try_recv().ok())
        .any(|e| matches!(e, Event::Warning { message, .. } if message.contains("déjà tenu")));
    assert!(said, "le refus est dit sur le bus");

    // « agent » est toujours le premier.
    assert_eq!(dir.resolve("agent").unwrap().run, "run-1");
    assert!(dir.claim("agent", "run-1b", 999_999).is_err());
}

/// Un agent reprend **son** nom sous un run neuf, et adopte un nom posé par
/// l'hôte ; un autre agent ne prend ni l'un ni l'autre une fois tenus.
#[test]
fn a_name_is_held_by_its_agent() {
    let d = Directory::new();
    let (a, b) = (new_owner(), new_owner());
    d.claim("alice", "run-1", a).unwrap();
    d.claim("alice", "run-2", a).unwrap();
    assert_eq!(d.resolve("alice").unwrap().run, "run-2");
    assert!(d.claim("alice", "run-3", b).unwrap_err().contains("alice"));
    d.register("bob", "run-b");
    d.claim("bob", "run-b", b).unwrap();
    assert!(d.claim("bob", "run-x", a).is_err());
}

/// **Un outil `send` de l'outillage n'est pas masqué en silence** :
/// `with_messaging` le refuse, en le nommant.
#[test]
fn messaging_refuses_a_toolbox_that_already_has_send() {
    let tools = CallbackToolBox::new(
        vec![crate::tools::ToolDef {
            name: SEND.to_string(),
            description: "un send à soi".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }],
        |_| "{}".into(),
    );
    let llm = scripted(vec![MockLlm::new("ok")], Arc::default());
    let err = Agent::new(&llm, &tools).with_messaging(Arc::new(Directory::new())).err().unwrap();
    assert!(err.contains("send") && err.contains("masquerait"), "{err}");
}

/// **Des arguments illisibles le disent** : le modèle corrige le JSON, pas
/// le contenu.
#[test]
fn unreadable_send_arguments_say_so() {
    let bus = EventBus::new(64);
    let dir = Arc::new(Directory::new());
    dir.enroll(&bus, "bob", "run-b");
    let tools = no_tools();
    let llm = scripted(
        vec![MockLlm::new("").with_tool_calls(vec![(SEND, "{to: bob")]), MockLlm::new("pardon")],
        Arc::default(),
    );
    let alice = Agent::new(&llm, &tools)
        .with_name("alice")
        .with_run_id("run-a")
        .with_events(bus.clone())
        .with_messaging(dir)
        .unwrap();
    let mut turns = vec![Turn::user("x")];
    alice.run(&mut turns, &mut StringSink::default()).unwrap();
    assert!(turns[2].content.contains("ne se lisent pas"), "{}", turns[2].content);
}
