//! Réessai avec backoff : classement, `Retry-After`, bornes, annulation.
//! Aucun réseau réel — faux serveur local — et **aucune attente réelle** :
//! l'horloge est injectée.
#![cfg(feature = "openai-llm")]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::fake_sse::*;
use rag3weaver::llm::{
    Finish, Flow, GenOptions, Llm, LlmError, RetryEvent, RetryPhase, StringSink, TokenSink, Turn,
};
use rag3weaver::openai_llm::{Clock, OpenAiLlm, RetryPolicy};

fn hello() -> Vec<Turn> {
    vec![Turn::system("s"), Turn::user("u")]
}

/// Horloge virtuelle : `sleep` avance le temps sans dormir. C'est elle qui
/// rend ces tests instantanés au lieu de durer des minutes.
struct FakeClock {
    now: Mutex<Instant>,
    slept: Mutex<Vec<Duration>>,
}

impl FakeClock {
    fn new() -> Arc<Self> {
        Arc::new(Self { now: Mutex::new(Instant::now()), slept: Mutex::new(Vec::new()) })
    }
    /// Total réellement « attendu », en temps virtuel.
    fn total(&self) -> Duration {
        self.slept.lock().unwrap().iter().sum()
    }
    fn naps(&self) -> usize {
        self.slept.lock().unwrap().len()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap()
    }
    fn sleep(&self, d: Duration) {
        *self.now.lock().unwrap() += d;
        self.slept.lock().unwrap().push(d);
    }
}

/// Puits qui enregistre les annonces de réessai (`Scheduled` seulement).
#[derive(Default)]
struct RetrySink {
    text: String,
    announces: Vec<(u32, Duration, bool)>,
    ticks: usize,
    /// Annule dès que ce nombre de tranches d'attente est atteint.
    cancel_after_ticks: Option<usize>,
}

impl TokenSink for RetrySink {
    fn on_token(&mut self, delta: &str) -> Flow {
        self.text.push_str(delta);
        Flow::Continue
    }
    fn on_retry(&mut self, e: &RetryEvent<'_>) -> Flow {
        match e.phase {
            RetryPhase::Scheduled => {
                self.announces.push((e.attempt, e.wait, e.from_server));
                assert!(!e.reason.is_empty(), "un réessai doit dire pourquoi");
                assert!(e.attempt <= e.max_attempts);
                Flow::Continue
            }
            RetryPhase::Waiting => {
                self.ticks += 1;
                match self.cancel_after_ticks {
                    Some(n) if self.ticks >= n => Flow::Stop,
                    _ => Flow::Continue,
                }
            }
        }
    }
}

fn client(srv: &FakeServer, clock: Arc<FakeClock>, policy: RetryPolicy) -> OpenAiLlm {
    OpenAiLlm::new(&srv.url, "m")
        .with_retry(policy)
        .with_clock(clock)
        .with_jitter_seed(0xC0FFEE)
}

// ─── Classement ─────────────────────────────────────────────────────────────

#[test]
fn a_429_is_retried_and_then_succeeds() {
    let srv = FakeServer::start_sequence(vec![Reply::status(429), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = RetrySink::default();

    let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
    assert_eq!(finish, Finish::eos());
    assert_eq!(sink.text, "Bonjour le monde");
    assert_eq!(usage.retries, 1, "le compteur doit rendre le réessai visible");
    assert_eq!(srv.connections.load(Ordering::SeqCst), 2);

    // Annoncé une seule fois, avec une attente proche de la base de 60 s.
    assert_eq!(sink.announces.len(), 1);
    let (attempt, wait, from_server) = sink.announces[0];
    assert_eq!(attempt, 1);
    assert!(!from_server, "pas de Retry-After ici");
    assert!(
        wait >= Duration::from_secs(48) && wait <= Duration::from_secs(72),
        "60 s ± 20 % de gigue, eu {wait:?}"
    );
    // Et l'attente a bien eu lieu — en temps virtuel.
    assert!(clock.total() >= Duration::from_secs(48));
}

#[test]
fn a_5xx_uses_a_much_shorter_base_than_a_429() {
    let srv = FakeServer::start_sequence(vec![Reply::status(503), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = RetrySink::default();
    llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();

    let (_, wait, _) = sink.announces[0];
    assert!(wait <= Duration::from_secs(2), "base 1 s pour un 5xx, eu {wait:?}");
}

#[test]
fn a_400_is_never_retried() {
    // Un schéma invalide, un jeton périmé, un droit manquant : rien de tout
    // cela ne guérit en attendant. Réessayer ne ferait que retarder de
    // plusieurs minutes un message d'erreur déjà juste.
    for code in [400u16, 401, 403, 404, 422] {
        let srv = FakeServer::start_sequence(vec![Reply::status(code), Reply::sse()]);
        let clock = FakeClock::new();
        let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
        let mut sink = RetrySink::default();
        let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();
        assert!(matches!(err, LlmError::Model(_)), "{code} → {err:?}");
        assert!(err.to_string().contains(&code.to_string()), "{err}");
        assert!(sink.announces.is_empty(), "{code} ne doit rien annoncer");
        assert_eq!(srv.connections.load(Ordering::SeqCst), 1, "{code} : une seule tentative");
        assert_eq!(clock.naps(), 0, "{code} : aucune attente");
    }
}

// ─── Retry-After ────────────────────────────────────────────────────────────

#[test]
fn retry_after_in_seconds_wins_over_the_computed_backoff() {
    let srv = FakeServer::start_sequence(vec![Reply::status_after(429, "7"), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = RetrySink::default();
    llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();

    let (_, wait, from_server) = sink.announces[0];
    assert!(from_server, "et le puits doit savoir d'où vient l'attente");
    // Le délai serveur est un **plancher** : la gigue ne peut qu'ajouter,
    // jamais retrancher — réessayer avant l'heure dite reprendrait un 429.
    assert!(
        wait >= Duration::from_secs(7) && wait <= Duration::from_millis(8_400),
        "7 s + 0 à 20 %, eu {wait:?}"
    );
    assert!(clock.total() >= Duration::from_secs(7));
}

#[test]
fn google_puts_its_delay_in_the_body_not_in_a_header() {
    // Forme `google.rpc.RetryInfo` : pas d'en-tête, un `retryDelay` enfoui
    // dans `error.details[]`.
    let body = r#"{"error":{"code":429,"status":"RESOURCE_EXHAUSTED","details":[
        {"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"27s"}]}}"#;
    let srv = FakeServer::start_sequence(vec![Reply::body(429, body), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = RetrySink::default();
    llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();

    let (_, wait, from_server) = sink.announces[0];
    assert!(wait >= Duration::from_secs(27) && wait <= Duration::from_millis(32_400), "{wait:?}");
    assert!(from_server);
}

#[test]
fn an_absurd_retry_after_is_still_capped() {
    // `Retry-After: 3600` ne doit pas faire attendre une heure.
    let srv = FakeServer::start_sequence(vec![Reply::status_after(429, "3600"), Reply::sse()]);
    let clock = FakeClock::new();
    let policy = RetryPolicy { max_total: Duration::from_secs(3600), ..Default::default() };
    let llm = client(&srv, Arc::clone(&clock), policy);
    let mut sink = RetrySink::default();
    llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
    let (_, wait, _) = sink.announces[0];
    assert!(
        wait >= Duration::from_secs(120) && wait <= Duration::from_secs(144),
        "plafonné à max_backoff (plus la gigue additive), eu {wait:?}"
    );
}

#[test]
fn a_quota_that_will_not_heal_is_not_retried() {
    // Tous les 429 ne se valent pas. Un quota **journalier** ou une limite de
    // dépense ne se dissipent pas : attendre quatre fois soixante secondes ne
    // ferait que retarder de quatre minutes le même message. Les SDK
    // officiels ne font pas cette distinction.
    let cases = [
        // Gemini : le `quotaId` nomme la fenêtre.
        r#"{"error":{"code":429,"status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.QuotaFailure","violations":[{"quotaId":"GenerateRequestsPerDayPerProjectPerModel-FreeTier"}]}]}}"#,
        // OpenAI : ce n'est pas une limite de débit, c'est une limite d'argent.
        r#"{"error":{"message":"You exceeded your current credit balance.","code":"insufficient_quota"}}"#,
        r#"{"error":{"message":"Project spend limit reached."}}"#,
    ];
    for body in cases {
        let srv = FakeServer::start_sequence(vec![Reply::body(429, body), Reply::sse()]);
        let clock = FakeClock::new();
        let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
        let mut sink = RetrySink::default();
        let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();
        assert!(err.to_string().contains("429"), "{err}");
        assert!(sink.announces.is_empty(), "aucun réessai attendu pour : {body}");
        assert_eq!(srv.connections.load(Ordering::SeqCst), 1);
        assert_eq!(clock.naps(), 0);
    }

    // Le contre-exemple : un quota **par minute** est bien réessayé.
    let per_minute = r#"{"error":{"code":429,"details":[{"@type":"type.googleapis.com/google.rpc.QuotaFailure","violations":[{"quotaId":"GenerateRequestsPerMinutePerProjectPerModel-FreeTier"}]},{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"31s"}]}}"#;
    let srv = FakeServer::start_sequence(vec![Reply::body(429, per_minute), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = RetrySink::default();
    llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
    assert_eq!(sink.announces.len(), 1);
    assert!(sink.announces[0].1 >= Duration::from_secs(31));
}

#[test]
fn a_408_and_a_409_are_retried_like_a_429() {
    for code in [408u16, 409] {
        let srv = FakeServer::start_sequence(vec![Reply::status(code), Reply::sse()]);
        let clock = FakeClock::new();
        let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
        let mut sink = RetrySink::default();
        llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
        assert_eq!(sink.announces.len(), 1, "{code} doit être réessayé");
    }
}

// ─── Bornes ─────────────────────────────────────────────────────────────────

#[test]
fn the_attempt_cap_bites() {
    let srv = FakeServer::start_sequence(vec![
        Reply::status(429),
        Reply::status(429),
        Reply::status(429),
        Reply::status(429),
        Reply::sse(),
    ]);
    let clock = FakeClock::new();
    let policy = RetryPolicy { max_attempts: 3, ..Default::default() };
    let llm = client(&srv, Arc::clone(&clock), policy);
    let mut sink = RetrySink::default();
    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();

    assert!(err.to_string().contains("429"), "{err}");
    assert_eq!(srv.connections.load(Ordering::SeqCst), 3, "3 tentatives, pas plus");
    assert_eq!(sink.announces.len(), 2, "donc 2 attentes");
}

#[test]
fn the_total_time_cap_bites_before_the_attempt_cap() {
    // Un agent ne doit jamais attendre une heure en silence : c'est le
    // plafond de temps, pas celui de tentatives, qui l'en empêche.
    let srv = FakeServer::start_sequence(vec![
        Reply::status(429),
        Reply::status(429),
        Reply::status(429),
        Reply::status(429),
    ]);
    let clock = FakeClock::new();
    let policy = RetryPolicy {
        max_attempts: 10,
        max_total: Duration::from_secs(90),
        ..Default::default()
    };
    let llm = client(&srv, Arc::clone(&clock), policy);
    let mut sink = RetrySink::default();
    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();

    assert!(err.to_string().contains("429"));
    assert!(clock.total() <= Duration::from_secs(90), "eu {:?}", clock.total());
    assert!(sink.announces.len() < 9, "le plafond de temps a mordu avant celui de tentatives");
}

// ─── La frontière : une fois le flux commencé, on ne rejoue pas ─────────────

#[test]
fn a_stream_cut_after_the_first_token_is_never_replayed() {
    // Le point le plus important du chantier. La coupure survient APRÈS que
    // des jetons ont été poussés : rejouer les dupliquerait, et le
    // consommateur n'aurait aucun moyen de le savoir.
    let srv = FakeServer::start_sequence(vec![
        Reply::CutMidStream {
            frames: vec![
                r#"{"choices":[{"index":0,"delta":{"content":"début"},"finish_reason":null}]}"#
                    .into(),
                r#"{"choices":[{"index":0,"delta":{"content":" de réponse"},"finish_reason":null}]}"#
                    .into(),
            ],
        },
        Reply::sse(),
    ]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = RetrySink::default();
    let (finish, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();

    // Le flux s'est terminé sans `[DONE]` : on rend ce qu'on a, sans rejouer.
    assert_eq!(sink.text, "début de réponse", "aucune duplication");
    assert_eq!(finish, Finish::eos());
    assert_eq!(usage.retries, 0);
    assert!(sink.announces.is_empty(), "aucun réessai après la frontière");
    assert_eq!(srv.connections.load(Ordering::SeqCst), 1, "une seule connexion");
    assert_eq!(clock.naps(), 0);
}

#[test]
fn a_transport_error_before_the_first_frame_is_retried() {
    // L'autre côté de la frontière : la socket n'a jamais rendu de 200, donc
    // rien n'a été poussé, donc rejouer est sûr.
    let srv = FakeServer::start_sequence(vec![Reply::sse()]);
    let dead = format!("http://127.0.0.1:{}/v1", 1);
    let clock = FakeClock::new();
    let policy = RetryPolicy { max_attempts: 3, ..Default::default() };
    let llm = OpenAiLlm::new(&dead, "m")
        .with_retry(policy)
        .with_clock(Arc::clone(&clock) as Arc<dyn Clock>)
        .with_jitter_seed(1);
    let mut sink = RetrySink::default();
    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();

    assert!(matches!(err, LlmError::Model(_)));
    assert_eq!(sink.announces.len(), 2, "deux réessais avant d'abandonner");
    // Base courte, comme un 5xx : une panne de socket se dissipe en secondes.
    assert!(sink.announces[0].1 <= Duration::from_secs(2));
    let _ = srv;
}

// ─── Annulation ─────────────────────────────────────────────────────────────

#[test]
fn the_wait_is_cancellable_from_the_sink() {
    let srv = FakeServer::start_sequence(vec![Reply::status(429), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    // Annule après 3 tranches d'attente, soit bien avant les 60 s.
    let mut sink = RetrySink { cancel_after_ticks: Some(3), ..Default::default() };

    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();
    assert!(err.to_string().contains("429"), "{err}");
    assert_eq!(srv.connections.load(Ordering::SeqCst), 1, "pas de seconde tentative");

    // L'annulation a été vue en moins d'une seconde de temps simulé : les
    // tranches font 200 ms, donc 3 tranches = 600 ms au plus.
    assert!(
        clock.total() < Duration::from_secs(1),
        "annulation vue après {:?}, il faut moins d'une seconde",
        clock.total()
    );
    assert!(sink.ticks >= 1);
}

#[test]
fn refusing_at_the_announcement_skips_the_wait_entirely() {
    struct Refuse(usize);
    impl TokenSink for Refuse {
        fn on_token(&mut self, _: &str) -> Flow {
            Flow::Continue
        }
        fn on_retry(&mut self, e: &RetryEvent<'_>) -> Flow {
            if e.phase == RetryPhase::Scheduled {
                self.0 += 1;
            }
            Flow::Stop
        }
    }
    let srv = FakeServer::start_sequence(vec![Reply::status(429), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = Refuse(0);
    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();
    assert!(err.to_string().contains("429"));
    assert_eq!(sink.0, 1);
    assert_eq!(clock.naps(), 0, "refusé avant la première tranche");
}

// ─── Réglages ───────────────────────────────────────────────────────────────

#[test]
fn without_retry_gives_up_immediately() {
    let srv = FakeServer::start_sequence(vec![Reply::status(429), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = OpenAiLlm::new(&srv.url, "m")
        .without_retry()
        .with_clock(Arc::clone(&clock) as Arc<dyn Clock>);
    let mut sink = RetrySink::default();
    let err = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap_err();
    assert!(err.to_string().contains("429"));
    assert_eq!(srv.connections.load(Ordering::SeqCst), 1);
    assert_eq!(clock.naps(), 0);
}

#[test]
fn backoff_grows_and_jitter_keeps_callers_apart() {
    let mk = |seed: u64| {
        let srv = FakeServer::start_sequence(vec![
            Reply::status(429),
            Reply::status(429),
            Reply::status(429),
        ]);
        let clock = FakeClock::new();
        let policy = RetryPolicy { max_attempts: 3, max_total: Duration::from_secs(3600), ..Default::default() };
        let llm = OpenAiLlm::new(&srv.url, "m")
            .with_retry(policy)
            .with_clock(Arc::clone(&clock) as Arc<dyn Clock>)
            .with_jitter_seed(seed);
        let mut sink = RetrySink::default();
        let _ = llm.generate(&hello(), &GenOptions::default(), &mut sink);
        sink.announces.iter().map(|a| a.1).collect::<Vec<_>>()
    };

    let a = mk(0xAAAA);
    assert_eq!(a.len(), 2);
    assert!(a[1] > a[0], "l'attente doit croître : {a:?}");

    // Deux appelants distincts n'attendent pas la même chose — c'est tout
    // l'objet de la gigue : sans elle, N appels refusés ensemble
    // réessaieraient ensemble et reproduiraient la rafale.
    let b = mk(0x5555);
    assert_ne!(a, b, "la gigue doit séparer les appelants");

    // Mais une graine fixée redonne la même suite : les tests sont
    // reproductibles.
    assert_eq!(mk(0xAAAA), a);
}

static PARALLEL_GUARD: AtomicUsize = AtomicUsize::new(0);

#[test]
fn a_sink_that_ignores_retries_still_sees_them_in_usage() {
    // Le rappel sert à agir ; le compteur à ne jamais être muet. Un appelant
    // qui n'a rien implémenté doit quand même pouvoir constater, après coup,
    // qu'un appel a passé l'essentiel de son temps à attendre.
    PARALLEL_GUARD.fetch_add(1, Ordering::SeqCst);
    let srv = FakeServer::start_sequence(vec![Reply::status(429), Reply::status(503), Reply::sse()]);
    let clock = FakeClock::new();
    let llm = client(&srv, Arc::clone(&clock), RetryPolicy::default());
    let mut sink = StringSink::default(); // n'implémente pas `on_retry`
    let (_, usage) = llm.generate(&hello(), &GenOptions::default(), &mut sink).unwrap();
    assert_eq!(usage.retries, 2);
    assert!(usage.ms > 0, "les attentes comptent dans la durée totale");
}
