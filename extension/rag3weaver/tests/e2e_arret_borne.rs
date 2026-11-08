//! E2E : **l'arrêt borné d'un réacteur revient dans sa borne même quand le runtime du crate est
//! tenu** (le ticket des figements de la lib entière, 8 novembre 2026). Une suite à part, et non un
//! test de la lib : le témoin tient volontairement les deux fils de travail du runtime partagé, et
//! dans la lib entière il fait rougir les tests voisins qui attendent une réaction en quelques
//! secondes (`dataflow::reactor::tests::a_batch_collapses_a_burst_into_one_run`, 3 rouges sur 4).
//! `run_e2e.sh` joue une suite seule dans son processus : le runtime qu'elle tient n'est qu'à elle.
//!
//! Run with: ./run_e2e.sh --test e2e_arret_borne

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rag3weaver::dataflow::{NodeRegistry, ReactPolicy, Reactor, ServiceRegistry};
use rag3weaver::EventBus;

/// **L'arrêt borné revient dans sa borne même quand le runtime est tenu** (le ticket des
/// figements de la lib entière, 8 novembre 2026) : deux réactions figées occupent les deux
/// fils de travail du runtime du crate — comme deux réactions qui attendent un service
/// muet ; l'arrêt borné de l'une rend « réacteur figé » dans sa borne, pas quand le runtime
/// se libère. Avant, son délai était un minuteur de ce runtime, qui ne sonnait plus.
#[test]
#[ignore]
fn a_bounded_stop_returns_within_its_bound_while_the_runtime_is_held() {
    // La garde se lâche dès que l'arrêt a rendu la main : le runtime du crate est partagé
    // par toute la lib, le tenir plus longtemps ferait rougir les tests voisins.
    let (garde, attente) = std::sync::mpsc::channel::<()>();
    let attente = Arc::new(Mutex::new(attente));
    let figer = |bus: &EventBus, entree: &Arc<AtomicUsize>| {
        let (e, a) = (entree.clone(), attente.clone());
        Reactor::new(bus.clone(), Arc::new(NodeRegistry::new()), Arc::new(ServiceRegistry::new()))
            .on("figee", ["messages"], ReactPolicy::Each, move |_| {
                e.fetch_add(1, Ordering::SeqCst);
                // Jamais servi : `garde` vit jusqu'au retour de l'arrêt. Un seul fil tient
                // le verrou ; l'autre attend le verrou, sur son fil de travail aussi.
                let _ = a.lock().unwrap().recv();
            })
            .spawn()
    };
    let (bus_a, bus_b) = (EventBus::new(64), EventBus::new(64));
    let (entree_a, entree_b) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let a = figer(&bus_a, &entree_a);
    let b = figer(&bus_b, &entree_b);
    bus_a.send_message("r", "a", "b", "fige-toi");
    bus_b.send_message("r", "a", "b", "fige-toi");
    // Dans la lib entière, un fil peut déjà être tenu par un autre test : on arrête la
    // première réaction entrée, l'autre tient le second fil si elle en trouve un.
    let debut = Instant::now();
    let (a, _b) = loop {
        if entree_a.load(Ordering::SeqCst) > 0 {
            break (a, b);
        }
        if entree_b.load(Ordering::SeqCst) > 0 {
            break (b, a);
        }
        assert!(debut.elapsed() < Duration::from_secs(5), "aucune réaction n'est entrée");
        std::thread::sleep(Duration::from_millis(2));
    };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let t0 = Instant::now();
        let rendu = a.stop_within(Duration::from_millis(300)).err();
        let _ = tx.send((rendu, t0.elapsed()));
    });
    let attendu = rx.recv_timeout(Duration::from_secs(5));
    drop(garde);
    let (rendu, duree) = attendu.expect("l'arrêt borné n'est pas revenu en 5 s : son délai attend le runtime qu'il devait borner");
    let erreur = rendu.expect("un réacteur figé n'est pas rendu");
    assert!(erreur.contains("réacteur figé"), "{erreur}");
    assert!(duree < Duration::from_secs(3), "l'arrêt borné a attendu {duree:?}");
}
