//! Embedding-rate calibration through the host's service client.
//! WIP: the measurement is deliberately empty until the witness has been seen red.
//! No installed service, GPU, model weights or catalog is used by this target.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rag3weaver::daemon::embeddings::{DaemonEmbedder, EmbedDaemon};
use rag3weaver::embedder::{lot_budget, stable_batches};
use rag3weaver::{CallbackEmbedder, Embedder};

const KNOWN_CHARS_PER_SECOND: f64 = 1_000.0;
const TEXT_CAP: usize = 200;
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);

type BatchTrace = Arc<Mutex<Vec<Vec<usize>>>>;

struct FakeService {
    address: String,
    batches: BatchTrace,
}

impl FakeService {
    fn start() -> Self {
        let batches: BatchTrace = Arc::default();
        let observed = batches.clone();
        let embedder = CallbackEmbedder::new(3, move |texts| {
            let lengths: Vec<_> = texts.iter().map(|text| text.chars().count()).collect();
            let chars: usize = lengths.iter().sum();
            observed.lock().expect("batch trace").push(lengths);
            std::thread::sleep(Duration::from_secs_f64(chars as f64 / KNOWN_CHARS_PER_SECOND));
            Ok(texts.iter().map(|_| vec![1.0, 0.0, 0.0]).collect())
        });
        let listener = TcpListener::bind("127.0.0.1:0").expect("private listener");
        let address = listener.local_addr().expect("private address").to_string();
        let service = EmbedDaemon::new(Arc::new(embedder)).max_chars_per_text(TEXT_CAP);
        // This target owns the service threads for its process lifetime. No child
        // process or installed service is created; the test process closes the port.
        std::thread::spawn(move || service.servir_lie(listener).expect("fake service"));
        Self { address, batches }
    }

    fn client(&self) -> DaemonEmbedder {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match DaemonEmbedder::joindre_dans(&self.address, Duration::from_millis(200)) {
                Ok(client) => return client.with_timeout(CLIENT_TIMEOUT),
                Err(error) if Instant::now() >= deadline => panic!("fake service unavailable: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    }
}

#[derive(Debug)]
struct Measurement {
    input_chars: usize,
    sent_chars: usize,
    texts: usize,
    batches: Vec<usize>,
    elapsed: Duration,
}

impl Measurement {
    fn chars_per_second(&self) -> f64 {
        self.sent_chars as f64 / self.elapsed.as_secs_f64()
    }
}

fn measure(_client: &DaemonEmbedder, _texts: &[String], _max_items: usize) -> Result<Measurement, String> {
    // RED stage: do not replace before observing the witness fail here.
    Err("embedding-rate measurement is not implemented".into())
}

#[test]
fn known_service_rate_is_recovered_and_batches_and_cuts_are_observed() {
    let service = FakeService::start();
    let client = service.client();
    assert_eq!(client.identite().modele, "callback");
    assert_eq!(client.identite().dim, 3);
    assert_eq!(client.identite().max_chars_per_text, Some(TEXT_CAP));

    // Warm-up is explicitly outside the measurement and its trace.
    assert_eq!(client.embed(&["warm".into()]).expect("warm-up").len(), 1);
    service.batches.lock().expect("batch trace").clear();

    // Non-ASCII input catches accidental byte-counting at the service cap.
    let texts: Vec<_> = (0..16).map(|_| "é".repeat(TEXT_CAP + 50)).collect();
    let lengths: Vec<_> = texts.iter().map(|text| text.chars().count()).collect();
    let expected_batches: Vec<_> = stable_batches(&lengths, lot_budget(client.budget_conseille(), 8))
        .iter().map(|range| range.len()).collect();
    let measured = measure(&client, &texts, 8).expect("measurement must exist");
    let observed = service.batches.lock().expect("batch trace").clone();

    assert_eq!(measured.texts, texts.len());
    assert_eq!(measured.input_chars, 16 * (TEXT_CAP + 50));
    assert_eq!(measured.sent_chars, 16 * TEXT_CAP);
    assert_eq!(measured.batches, expected_batches);
    assert_eq!(observed.iter().map(Vec::len).collect::<Vec<_>>(), measured.batches);
    assert!(observed.iter().flatten().all(|&length| length == TEXT_CAP));
    assert!(client.char_truncations().is_some_and(|(count, cap)| count == 16 && cap == TEXT_CAP));
    let relative_error = (measured.chars_per_second() / KNOWN_CHARS_PER_SECOND - 1.0).abs();
    assert!(relative_error <= 0.05, "rate={}; relative error={relative_error}", measured.chars_per_second());
}

#[test]
fn empty_input_is_refused_instead_of_reporting_a_rate() {
    let service = FakeService::start();
    let client = service.client();
    assert!(measure(&client, &[], 8).is_err());
    assert!(service.batches.lock().expect("batch trace").is_empty());
}
