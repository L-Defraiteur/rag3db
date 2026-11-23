//! **The host publishes its state without holding its caller** (a measure on 23 November 2026: opening a folder took
//! 7 s, the attachment publishing the host's whole state synchronously, 6.5 s with 84 backends; the terminal's start
//! waited on it). A host that knows itself publishes its state in the background, and a publication asked while one
//! runs is done after it, once.
use super::*;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

#[test]
fn the_state_is_published_in_the_background() {
    let host = Arc::new(DaemonHost::new());
    host.se_connaitre();
    host.etat_lent_ms.store(2000, Ordering::SeqCst);
    let t0 = Instant::now();
    host.publier_l_etat_de_l_hote();
    host.publier_l_etat_de_l_hote();
    let duree = t0.elapsed();
    assert!(duree < Duration::from_millis(500), "the caller waited for the state: {duree:?}");
    // The publication happens, then stops.
    let fin = Instant::now() + Duration::from_secs(10);
    while host.publication_en_cours.load(Ordering::SeqCst) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!host.publication_en_cours.load(Ordering::SeqCst), "the publication ends");
    assert!(host.publications_faites.load(Ordering::SeqCst) >= 1, "the state was published");
}
