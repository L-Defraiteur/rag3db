//! Embedding-rate calibration through the host's service client.
//! The witness was seen red (measurement empty) on 18 Nov. 2026 before `measure` was written.
//! No installed service, GPU, model weights or catalog is used by this target.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::ops::Range;
use std::time::{Duration, Instant};

use rag3weaver::daemon::embeddings::{DaemonEmbedder, EmbedDaemon};
use rag3weaver::burst::{plan, Batches};
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
    first_batch: Duration,
}

impl Measurement {
    fn chars_per_second(&self) -> f64 {
        self.sent_chars as f64 / self.elapsed.as_secs_f64()
    }

    fn texts_per_second(&self) -> f64 {
        self.texts as f64 / self.elapsed.as_secs_f64()
    }

    fn batches_per_second(&self) -> f64 {
        self.batches.len() as f64 / self.elapsed.as_secs_f64()
    }
}

/// The batches the catalog forms for these texts (`EmbedNode`, `record_nodes.rs`): sorted by byte
/// length, then cut by `burst::plan` under the service's advice and `default_items` (32 in the
/// catalog). Returns the sort order and the ranges over it.
fn host_batches(client: &DaemonEmbedder, texts: &[String], default_items: usize) -> Result<(Vec<usize>, Vec<Range<usize>>), String> {
    let mut order: Vec<usize> = (0..texts.len()).collect();
    order.sort_by_key(|&i| texts[i].len());
    let lens: Vec<usize> = order.iter().map(|&i| texts[i].len()).collect();
    match plan(&lens, client.budget_conseille(), default_items, !client.distant()) {
        Batches::Fixed(ranges) => Ok((order, ranges)),
        Batches::Paced { .. } => Err("a paced plan: this client holds a card, which a service client never does".into()),
    }
}

/// Embeds `texts` through the host's service client, in the given ranges over `order`, and times
/// the embedding calls only. `sent_chars` counts what reaches the service: each text cut to its
/// `max_chars_per_text` (in chars, as the client cuts).
fn measure_ranges(client: &DaemonEmbedder, texts: &[String], order: &[usize], ranges: &[Range<usize>]) -> Result<Measurement, String> {
    if texts.is_empty() {
        return Err("no text to measure: a rate needs at least one text".into());
    }
    let cap = client.identite().max_chars_per_text.unwrap_or(usize::MAX);
    let mut elapsed = Duration::ZERO;
    let mut first_batch = None;
    for range in ranges {
        let batch: Vec<String> = order[range.clone()].iter().map(|&i| texts[i].clone()).collect();
        let started = Instant::now();
        let vectors = client.embed(&batch).map_err(|error| format!("embedding a batch of {}: {error}", batch.len()))?;
        let took = started.elapsed();
        first_batch.get_or_insert(took);
        elapsed += took;
        if vectors.len() != batch.len() {
            return Err(format!("the service returned {} vectors for {} texts", vectors.len(), batch.len()));
        }
    }
    let lengths: Vec<usize> = texts.iter().map(|text| text.chars().count()).collect();
    Ok(Measurement {
        input_chars: lengths.iter().sum(),
        sent_chars: lengths.iter().map(|&length| length.min(cap)).sum(),
        texts: texts.len(),
        batches: ranges.iter().map(|range| range.len()).collect(),
        elapsed,
        first_batch: first_batch.unwrap_or_default(),
    })
}

/// The catalog's path: its batches, then the timed calls.
fn measure(client: &DaemonEmbedder, texts: &[String], default_items: usize) -> Result<Measurement, String> {
    if texts.is_empty() {
        return Err("no text to measure: a rate needs at least one text".into());
    }
    let (order, ranges) = host_batches(client, texts, default_items)?;
    measure_ranges(client, texts, &order, &ranges)
}

/// Fixed batches of `size` texts, in the catalog's sort order: the rate at one batch size.
fn measure_fixed(client: &DaemonEmbedder, texts: &[String], size: usize) -> Result<Measurement, String> {
    let mut order: Vec<usize> = (0..texts.len()).collect();
    order.sort_by_key(|&i| texts[i].len());
    let ranges: Vec<Range<usize>> = (0..texts.len()).step_by(size.max(1)).map(|start| start..(start + size).min(texts.len())).collect();
    measure_ranges(client, texts, &order, &ranges)
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
    let Batches::Fixed(ranges) = plan(&texts.iter().map(String::len).collect::<Vec<_>>(), client.budget_conseille(), 8, !client.distant())
    else { panic!("a service client plans its batches ahead") };
    let expected_batches: Vec<_> = ranges.iter().map(|range| range.len()).collect();
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

fn percentile(values: &[f64], q: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// One point of the table: `reps` repetitions of one way of batching.
fn point(name: &str, reps: usize, median_chunks_per_file: f64, mut run: impl FnMut() -> Result<Measurement, String>) -> serde_json::Value {
    let runs: Vec<Measurement> = (0..reps).map(|_| run().unwrap_or_else(|error| panic!("{name}: {error}"))).collect();
    let chars: Vec<f64> = runs.iter().map(Measurement::chars_per_second).collect();
    let texts: Vec<f64> = runs.iter().map(Measurement::texts_per_second).collect();
    let batches: Vec<f64> = runs.iter().map(Measurement::batches_per_second).collect();
    let chunks_in_6s = percentile(&texts, 0.5) * 6.0;
    eprintln!("[bench] {name}: {:.0} chars/s, {:.1} chunks/s (p50) ; {:.0} chunks and {:.0} files in 6 s", percentile(&chars, 0.5), percentile(&texts, 0.5), chunks_in_6s, chunks_in_6s / median_chunks_per_file);
    serde_json::json!({
        "name": name,
        "texts": runs[0].texts, "input_chars": runs[0].input_chars, "sent_chars": runs[0].sent_chars,
        "batches": runs[0].batches.len(), "largest_batch": runs[0].batches.iter().max(),
        "chars_per_second": {"p50": percentile(&chars, 0.5), "p90": percentile(&chars, 0.9), "runs": chars},
        "chunks_per_second": {"p50": percentile(&texts, 0.5), "p90": percentile(&texts, 0.9)},
        "batches_per_second": {"p50": percentile(&batches, 0.5)},
        "first_batch_ms": runs.iter().map(|m| m.first_batch.as_secs_f64() * 1000.0).collect::<Vec<_>>(),
        "chunks_in_6s": chunks_in_6s,
        "repository_files_in_6s": chunks_in_6s / median_chunks_per_file,
    })
}

/// **The calibration bench** (docs/banc/16-11-2026/03-le-banc-d-etalonnage.md): the frozen set through
/// each service, by the host's client, in the catalog's batches and at fixed batch sizes.
/// `RAG3WEAVER_BENCH_SET` (the frozen set JSON), `RAG3WEAVER_BENCH_SERVICES` (comma-separated
/// addresses), `RAG3WEAVER_BENCH_HOST` (this machine's name), `RAG3WEAVER_BENCH_OUT` (the JSON
/// written), `RAG3WEAVER_BENCH_REPS` (3 by default). It refuses when one is missing.
#[test]
#[ignore]
fn bench_embedding_rate_on_services() {
    let var = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is required by the calibration bench"));
    let set: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(var("RAG3WEAVER_BENCH_SET")).expect("the frozen set")).expect("the frozen set as JSON");
    let texts_of = |key: &str| -> Vec<String> {
        set[key].as_array().unwrap_or_else(|| panic!("the set has no {key}")).iter().map(|x| x["text"].as_str().expect("a text").to_string()).collect()
    };
    let (real, full) = (texts_of("real"), texts_of("full"));
    let median = set["chunks_per_file"]["median"].as_f64().expect("the median chunks per file");
    let reps: usize = std::env::var("RAG3WEAVER_BENCH_REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    let host = var("RAG3WEAVER_BENCH_HOST");
    let mut services = Vec::new();
    for address in var("RAG3WEAVER_BENCH_SERVICES").split(',').map(str::trim).filter(|a| !a.is_empty()) {
        let client = DaemonEmbedder::joindre(address).unwrap_or_else(|error| panic!("{address}: {error}")).with_timeout(Duration::from_secs(600));
        let identity = serde_json::to_value(client.identite()).expect("the identity as JSON");
        assert!(!client.is_mock(), "{address} is a mock service: its rate means nothing");
        eprintln!("[bench] {address}@{host}: {} ({}), advice {:?}, cap {:?}", client.identite().modele, client.identite().dim, client.budget_conseille(), client.identite().max_chars_per_text);
        // The warm-up is outside every measurement.
        measure(&client, &real[..64.min(real.len())], 32).expect("warm-up");
        let largest = client.budget_conseille().map_or(512, |(items, _)| items.max(1));
        let mut points = vec![
            point("real, catalog batches", reps, median, || measure(&client, &real, 32)),
            point("full, catalog batches", reps, median, || measure(&client, &full, 32)),
        ];
        for size in [1usize, 8, 32, 128, 512].into_iter().filter(|&n| n <= largest) {
            let sample = &real[..(size * 8).clamp(64, real.len())];
            points.push(point(&format!("real, batches of {size}"), reps, median, || measure_fixed(&client, sample, size)));
        }
        services.push(serde_json::json!({"address": address, "key": format!("{}:{address}@{host}", client.identite().modele), "identity": identity, "points": points}));
    }
    let out = serde_json::json!({"host": host, "set": var("RAG3WEAVER_BENCH_SET"), "median_chunks_per_file": median, "reps": reps, "services": services});
    std::fs::write(var("RAG3WEAVER_BENCH_OUT"), serde_json::to_string_pretty(&out).unwrap()).expect("writing the bench");
    eprintln!("[bench] written: {}", var("RAG3WEAVER_BENCH_OUT"));
}
