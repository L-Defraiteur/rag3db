//! Le même `OpenAiLlm` synchrone, **piloté par luciole** : l'appel réseau
//! bloquant vit sur un thread du pool, le demandeur ne bloque pas, et
//! l'annulation remonte du consommateur jusqu'à la socket.
//!
//! Contient aussi la **preuve de la règle d'interblocage** — un puits qui
//! bloque referme le pool sur lui-même, un puits en `try_send` ne le fait pas.
#![cfg(feature = "openai-llm")]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

use common::fake_sse::*;
use luciole::mailbox::mailbox;
use luciole::scheduler::{global_scheduler, is_scheduler_thread};
use luciole::{Actor, ActorContext, ActorRef, ActorStatus, Priority};
use rag3weaver::llm::{Finish, Flow, GenOptions, Llm, StringSink, TokenSink, Turn};
use rag3weaver::openai_llm::OpenAiLlm;

fn hello() -> Vec<Turn> {
    vec![Turn::system("s"), Turn::user("u")]
}

enum TokenMsg {
    Token(String),
    Finish(Finish),
}

/// Le `ChannelSink` de `llm.rs`, mais sur un `ActorRef` : `send` échoue dès
/// que l'acteur consommateur a rendu `ActorStatus::Stop` (mailbox
/// déconnectée). C'est par là que « je ne veux plus rien » remonte — luciole
/// n'a pas d'autre mécanisme d'annulation.
struct ActorSink(ActorRef<TokenMsg>);

impl TokenSink for ActorSink {
    fn on_token(&mut self, delta: &str) -> Flow {
        match self.0.send(TokenMsg::Token(delta.to_string())) {
            Ok(()) => Flow::Continue,
            Err(_) => Flow::Stop,
        }
    }
    fn on_finish(&mut self, reason: &Finish) {
        let _ = self.0.send(TokenMsg::Finish(reason.clone()));
    }
}

struct Consumer {
    seen: Vec<String>,
    stop_after: Option<usize>,
    out: mpsc::Sender<(Vec<String>, Option<Finish>)>,
    done: bool,
}

impl Actor for Consumer {
    type Msg = TokenMsg;
    fn name(&self) -> &'static str { "llm_consumer" }
    fn priority(&self) -> Priority { Priority::High }
    fn handle(&mut self, msg: TokenMsg, _ctx: &ActorContext) -> ActorStatus {
        match msg {
            TokenMsg::Token(t) => {
                self.seen.push(t);
                if self.stop_after.is_some_and(|n| self.seen.len() >= n) {
                    self.done = true;
                    let _ = self.out.send((std::mem::take(&mut self.seen), None));
                    return ActorStatus::Stop;
                }
                ActorStatus::Continue
            }
            TokenMsg::Finish(f) => {
                if !self.done {
                    let _ = self.out.send((std::mem::take(&mut self.seen), Some(f)));
                }
                ActorStatus::Stop
            }
        }
    }
}

type Collected = (Vec<String>, Option<Finish>);

fn spawn_consumer(stop_after: Option<usize>) -> (ActorRef<TokenMsg>, mpsc::Receiver<Collected>) {
    let (tx, rx) = mpsc::channel();
    let (mb, mut aref) = mailbox::<TokenMsg>(256);
    global_scheduler().spawn(
        Consumer { seen: Vec::new(), stop_after, out: tx, done: false },
        mb,
        &mut aref,
        256,
    );
    (aref, rx)
}

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

#[test]
fn task_pipe_to_runs_on_the_pool_and_blocks_no_caller() {
    let srv = FakeServer::start(text_frames(), false);
    let (consumer, rx) = spawn_consumer(None);

    let on_pool = Arc::new(AtomicUsize::new(0));
    let on_pool2 = Arc::clone(&on_pool);
    let url = srv.url.clone();
    let sink_ref = consumer.clone();

    // Le thread de test n'est pas un thread de scheduler : il ne peut pas
    // affamer le pool en attendant.
    assert!(!is_scheduler_thread());

    global_scheduler().task_pipe_to(
        Priority::Idle, // I/O réseau : jamais High, jamais Critical
        move || {
            on_pool2.store(if is_scheduler_thread() { 1 } else { 2 }, Ordering::SeqCst);
            let llm = OpenAiLlm::new(&url, "m");
            let mut sink = ActorSink(sink_ref);
            llm.generate(&hello(), &GenOptions::default(), &mut sink)
        },
        &consumer,
        "llm_generate",
        // Le vrai `Finish` est déjà passé par le puits, au fil de l'eau.
        |_res| TokenMsg::Finish(Finish::Eos),
    );

    let (tokens, finish) = rx.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(tokens.concat(), "Bonjour le monde");
    assert_eq!(finish, Some(Finish::Eos));
    assert_eq!(on_pool.load(Ordering::SeqCst), 1, "la tâche doit tourner sur un thread du pool");
}

#[test]
fn a_consumer_that_stops_closes_the_socket() {
    // Serveur intarissable : sans annulation réelle, ce test ne finit pas.
    let srv = FakeServer::start(text_frames(), true);
    let (consumer, rx) = spawn_consumer(Some(2));

    let url = srv.url.clone();
    let sink_ref = consumer.clone();
    let result = global_scheduler().submit_task(Priority::Idle, move || {
        let llm = OpenAiLlm::new(&url, "m");
        let mut sink = ActorSink(sink_ref);
        llm.generate(&hello(), &GenOptions::default(), &mut sink)
    });
    drop(consumer); // seul l'ActorSink garde une référence

    let (tokens, _) = rx.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(tokens.len(), 2);

    // `wait` depuis un thread externe : attente bloquante, autorisée.
    let (finish, usage) = global_scheduler().wait(result, "llm_cancel").unwrap();
    assert_eq!(finish, Finish::Cancelled);
    assert!(!finish.is_complete());
    assert!(usage.completion_tokens >= 2);

    let written = srv.written.load(Ordering::SeqCst);
    assert!(written < 10_000, "{written} trames écrites : socket pas coupée");
}

#[test]
fn four_concurrent_generations_all_complete() {
    let servers: Vec<_> = (0..4).map(|_| FakeServer::start(text_frames(), false)).collect();
    let rxs: Vec<_> = servers
        .iter()
        .map(|s| {
            let url = s.url.clone();
            global_scheduler().submit_task(Priority::Idle, move || {
                let llm = OpenAiLlm::new(&url, "m");
                let mut sink = StringSink::default();
                llm.generate(&hello(), &GenOptions::default(), &mut sink)
                    .map(|(f, _)| (sink.text, f))
            })
        })
        .collect();

    for rx in rxs {
        let (text, finish) = global_scheduler().wait(rx, "llm_par").unwrap();
        assert_eq!(text, "Bonjour le monde");
        assert_eq!(finish, Finish::Eos);
    }
}

// ─── La règle d'interblocage, prouvée ────────────────────────────────────────
//
// Sur un thread de scheduler, l'I/O bloquante doit être la SEULE chose qui
// bloque : le puits ne doit jamais dormir sur une mailbox pleine.
//
// Le scénario fautif tue définitivement le thread du pool qu'il occupe, donc
// il ne peut pas cohabiter avec les autres tests du binaire — d'où `#[ignore]`
// et la sélection par variable d'environnement. À relancer à la main :
//
//   LUCIVY_SCHEDULER_THREADS=1 SINK=blocking \
//     cargo test --features openai-llm --test openai_llm_luciole -- --ignored
//   LUCIVY_SCHEDULER_THREADS=1 SINK=try_send \
//     cargo test --features openai-llm --test openai_llm_luciole -- --ignored
//
// Mesuré : `blocking` n'aboutit jamais (3 s de garde), `try_send` finit en
// 0,01 s avec ses 3 jetons.

enum SlowMsg { Token(String), Done }

struct SlowConsumer { out: mpsc::Sender<usize>, n: usize }

impl Actor for SlowConsumer {
    type Msg = SlowMsg;
    fn name(&self) -> &'static str { "slow" }
    fn priority(&self) -> Priority { Priority::Low }
    fn handle(&mut self, m: SlowMsg, _c: &ActorContext) -> ActorStatus {
        match m {
            SlowMsg::Token(t) => { self.n += t.chars().count().min(1); ActorStatus::Continue }
            SlowMsg::Done => { let _ = self.out.send(self.n); ActorStatus::Stop }
        }
    }
}

/// LE MOTIF INTERDIT : `send` bloquant sur une mailbox bornée.
struct BlockingSink(ActorRef<SlowMsg>);
impl TokenSink for BlockingSink {
    fn on_token(&mut self, d: &str) -> Flow {
        match self.0.send(SlowMsg::Token(d.into())) {
            Ok(()) => Flow::Continue,
            Err(_) => Flow::Stop,
        }
    }
    fn on_finish(&mut self, _: &Finish) { let _ = self.0.send(SlowMsg::Done); }
}

/// LE MOTIF CORRECT. Tant que la mailbox est pleine, on exécute du travail
/// prêt au lieu de dormir — exactement ce que fait `merge_permits::acquire`.
/// `on_finish` DOIT passer par là aussi : y laisser un `send` bloquant
/// reproduit le même interblocage (erreur faite, et corrigée, en écrivant ce
/// test).
fn push(r: &ActorRef<SlowMsg>, mut msg: SlowMsg) -> Flow {
    loop {
        match r.try_send(msg) {
            Ok(()) => return Flow::Continue,
            Err(flume::TrySendError::Disconnected(_)) => return Flow::Stop,
            Err(flume::TrySendError::Full(m)) => {
                msg = m;
                if !global_scheduler().run_one_step() {
                    std::thread::yield_now();
                }
            }
        }
    }
}

struct NonBlockingSink(ActorRef<SlowMsg>);
impl TokenSink for NonBlockingSink {
    fn on_token(&mut self, d: &str) -> Flow { push(&self.0, SlowMsg::Token(d.into())) }
    fn on_finish(&mut self, _: &Finish) { push(&self.0, SlowMsg::Done); }
}

fn run_sink(blocking: bool) -> Option<usize> {
    let srv = FakeServer::start(text_frames(), false);
    let (tx, rx) = mpsc::channel();
    let (mb, mut aref) = mailbox::<SlowMsg>(1); // bornée à 1 : le pire cas
    global_scheduler().spawn(SlowConsumer { out: tx, n: 0 }, mb, &mut aref, 1);

    let url = srv.url.clone();
    let sink_ref = aref.clone();
    global_scheduler().submit_task(Priority::Idle, move || {
        let llm = OpenAiLlm::new(&url, "m");
        let opts = GenOptions::default();
        if blocking {
            let _ = llm.generate(&[Turn::user("u")], &opts, &mut BlockingSink(sink_ref));
        } else {
            let _ = llm.generate(&[Turn::user("u")], &opts, &mut NonBlockingSink(sink_ref));
        }
    });
    drop(aref);
    rx.recv_timeout(std::time::Duration::from_secs(3)).ok()
}

#[test]
#[ignore = "scénario manuel : LUCIVY_SCHEDULER_THREADS=1 SINK=blocking|try_send"]
fn deadlock_rule() {
    let threads: usize = std::env::var("LUCIVY_SCHEDULER_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    assert_eq!(threads, 1, "relancer avec LUCIVY_SCHEDULER_THREADS=1");
    match std::env::var("SINK").as_deref() {
        // Le seul thread du pool exécute `generate`, dont le puits dort sur la
        // mailbox pleine que plus personne ne peut dépiler.
        Ok("blocking") => assert_eq!(run_sink(true), None, "attendu : interblocage"),
        // Même pool d'un thread, même mailbox de 1 — et ça passe.
        Ok("try_send") => assert_eq!(run_sink(false), Some(3), "attendu : 3 jetons"),
        _ => panic!("préciser SINK=blocking ou SINK=try_send"),
    }
}
