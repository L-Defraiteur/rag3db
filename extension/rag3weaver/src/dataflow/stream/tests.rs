//! Les témoins du canal de l'exécuteur concurrent : la fin, l'annulation, le
//! réveil, le reflux, le détecteur d'interblocage et ses limites.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// **Un flux se lit dans l'ordre, et sa fin est `None`** quand le dernier
/// émetteur est rendu.
#[test]
fn a_stream_is_read_in_order_then_ends() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("a.out → b.in", 2, 1, 1);
    let (a, b) = (hub.worker("a"), hub.worker("b"));
    std::thread::scope(|s| {
        s.spawn(|| {
            for i in 1..=5 {
                hub.send(a, &[ch], i).unwrap();
            }
            hub.release_sender(ch);
            hub.worker_done(a);
        });
        let mut seen = Vec::new();
        while let Some((_, v)) = hub.recv_any(b, &[ch]).unwrap() {
            seen.push(v);
        }
        hub.worker_done(b);
        assert_eq!(seen, [1, 2, 3, 4, 5]);
    });
    assert_eq!(hub.cancelled(), None);
}

/// **La fin et l'annulation sont deux états** : un canal fini mais un run
/// annulé rend `Cancelled`, jamais `None` — un flux interrompu ne passe pas
/// pour complet.
#[test]
fn a_cancelled_run_is_not_an_ended_stream() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("a.out → b.in", 4, 1, 1);
    let b = hub.worker("b");
    hub.send(b, &[ch], 1).unwrap();
    hub.release_sender(ch);
    hub.cancel(CancelCause::Failed { node: "a".into() });
    assert_eq!(hub.recv_any(b, &[ch]), Err(Cancelled(CancelCause::Failed { node: "a".into() })));
}

/// **L'annulation réveille un lecteur bloqué**, et la première cause gagne.
#[test]
fn cancel_wakes_a_blocked_reader() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("a.out → b.in", 4, 1, 1);
    let (main, b) = (hub.worker("hôte"), hub.worker("b"));
    std::thread::scope(|s| {
        let reader = s.spawn(|| hub.recv_any(b, &[ch]));
        // L'hôte n'attend pas nos canaux : il est vivant, rien n'est déclaré.
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(hub.cancelled(), None);
        hub.cancel(CancelCause::External);
        hub.cancel(CancelCause::Failed { node: "x".into() });
        assert_eq!(reader.join().unwrap(), Err(Cancelled(CancelCause::External)));
    });
    hub.worker_done(main);
}

/// **Un producteur dont plus personne ne lit voit `NoReader`**, et ce qui
/// restait en attente est jeté.
#[test]
fn a_producer_without_reader_is_stopped() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("a.out → b.in", 4, 1, 1);
    let a = hub.worker("a");
    hub.send(a, &[ch], 1).unwrap();
    hub.release_receiver(ch);
    assert_eq!(hub.send(a, &[ch], 2), Err(Stopped::NoReader));
}

/// **Un port sur plusieurs arêtes** : chaque canal lu reçoit l'élément ; un
/// canal lâché est sauté, et `NoReader` ne tombe que si tous le sont.
#[test]
fn fan_out_reaches_every_live_reader() {
    let hub = StreamHub::<String>::new();
    let (c1, c2) = (hub.channel("a.out → b.in", 4, 1, 1), hub.channel("a.out → c.in", 4, 1, 1));
    let w = hub.worker("t");
    hub.send(w, &[c1, c2], "x".to_string()).unwrap();
    hub.release_receiver(c2);
    hub.send(w, &[c1, c2], "y".to_string()).unwrap();
    hub.release_sender(c1);
    let read: Vec<String> = std::iter::from_fn(|| hub.recv_any(w, &[c1]).unwrap().map(|(_, v)| v)).collect();
    assert_eq!(read, ["x", "y"]);
    hub.release_receiver(c1);
    assert_eq!(hub.send(w, &[c1, c2], "z".to_string()), Err(Stopped::NoReader));
}

/// **Le reflux, et le détecteur sur un seul ouvrier** : un canal plein que
/// personne ne lit bloque l'écriture ; seul ouvrier vivant, il est
/// interbloqué, et la cause le nomme.
#[test]
fn a_full_channel_with_no_live_reader_is_a_named_deadlock() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("a.out → b.in", 1, 1, 1);
    let a = hub.worker("a");
    hub.send(a, &[ch], 1).unwrap();
    let err = hub.send(a, &[ch], 2).unwrap_err();
    match err {
        Stopped::Cancelled(CancelCause::Deadlock(why)) => {
            assert!(why.contains("a attend d'écrire sur a.out → b.in, plein"), "{why}");
        }
        other => panic!("un interblocage nommé attendu : {other:?}"),
    }
}

/// **Deux ouvriers qui s'attendent l'un l'autre** : l'interblocage est
/// déclaré quand le second s'endort, et les deux attentes sont nommées.
#[test]
fn two_workers_waiting_on_each_other_are_named() {
    let hub = StreamHub::<u32>::new();
    let ab = hub.channel("a.out → b.in", 1, 1, 1);
    let ba = hub.channel("b.out → a.in", 1, 1, 1);
    let (a, b) = (hub.worker("a"), hub.worker("b"));
    std::thread::scope(|s| {
        let ta = s.spawn(|| hub.recv_any(a, &[ba]));
        let tb = s.spawn(|| hub.recv_any(b, &[ab]));
        let (ra, rb) = (ta.join().unwrap(), tb.join().unwrap());
        for r in [ra, rb] {
            match r {
                Err(Cancelled(CancelCause::Deadlock(why))) => {
                    assert!(why.contains("a attend une lecture sur b.out → a.in"), "{why}");
                    assert!(why.contains("b attend une lecture sur a.out → b.in"), "{why}");
                }
                other => panic!("un interblocage attendu : {other:?}"),
            }
        }
    });
}

/// **Une attente hors de nos canaux n'est pas comptée** : un ouvrier bloqué
/// ailleurs (une requête, le bus) est vivant, et le lecteur qui l'attend
/// n'est pas déclaré interbloqué ; il reçoit ce que l'autre finit par émettre.
#[test]
fn a_worker_blocked_elsewhere_is_alive() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("a.out → b.in", 1, 1, 1);
    let (a, b) = (hub.worker("a"), hub.worker("b"));
    let (tx, rx) = std::sync::mpsc::channel::<u32>();
    std::thread::scope(|s| {
        let h = &hub;
        s.spawn(move || {
            let hub = h;
            let v = rx.recv().unwrap();
            hub.send(a, &[ch], v).unwrap();
            hub.release_sender(ch);
            hub.worker_done(a);
        });
        let reader = s.spawn(|| hub.recv_any(b, &[ch]));
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(hub.cancelled(), None, "a attend hors registre : rien n'est déclaré");
        tx.send(7).unwrap();
        assert_eq!(reader.join().unwrap().unwrap().map(|(_, v)| v), Some(7));
    });
}

/// **Un ouvrier qui finit peut laisser les autres interbloqués** : le
/// détecteur se refait à sa sortie.
#[test]
fn a_worker_leaving_can_reveal_a_deadlock() {
    let hub = StreamHub::<u32>::new();
    let ch = hub.channel("x.out → b.in", 1, 1, 1);
    let (x, b) = (hub.worker("x"), hub.worker("b"));
    std::thread::scope(|s| {
        let reader = s.spawn(|| hub.recv_any(b, &[ch]));
        std::thread::sleep(Duration::from_millis(50));
        // x part sans rendre son émetteur : b attendra pour toujours.
        hub.worker_done(x);
        match reader.join().unwrap() {
            Err(Cancelled(CancelCause::Deadlock(why))) => assert!(why.contains("b attend une lecture sur x.out → b.in"), "{why}"),
            other => panic!("un interblocage attendu : {other:?}"),
        }
    });
}

/// **Les rappels d'annulation** : appelés une fois, hors du verrou (le rappel
/// relit le hub), et tout de suite pour un rappel posé après coup — y
/// compris quand c'est le détecteur qui annule.
#[test]
fn on_cancel_runs_once_outside_the_lock() {
    let hub = StreamHub::<u32>::new();
    let calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..2 {
        let (h, c) = (hub.clone(), calls.clone());
        hub.on_cancel(move || {
            assert!(h.cancelled().is_some(), "le rappel relit le hub sans s'interbloquer");
            c.fetch_add(1, Ordering::SeqCst);
        });
    }
    hub.cancel(CancelCause::External);
    hub.cancel(CancelCause::External);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let c = calls.clone();
    hub.on_cancel(move || {
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    let hub = StreamHub::<u32>::new();
    let fired = Arc::new(AtomicUsize::new(0));
    let f = fired.clone();
    hub.on_cancel(move || {
        f.fetch_add(1, Ordering::SeqCst);
    });
    let ch = hub.channel("a.out → b.in", 1, 1, 1);
    let a = hub.worker("a");
    hub.send(a, &[ch], 1).unwrap();
    assert!(hub.send(a, &[ch], 2).is_err());
    assert_eq!(fired.load(Ordering::SeqCst), 1, "le détecteur appelle aussi les rappels");
}

/// **L'attente bornée du jeton** : rend `false` au bout du délai, `true` dès
/// que le run est annulé d'ailleurs.
#[test]
fn wait_cancel_timeout_wakes_on_cancel() {
    let hub = StreamHub::<u32>::new();
    assert!(!hub.wait_cancel_timeout(Duration::from_millis(10)));
    std::thread::scope(|s| {
        let waiter = s.spawn(|| hub.wait_cancel_timeout(Duration::from_secs(30)));
        std::thread::sleep(Duration::from_millis(20));
        hub.cancel(CancelCause::External);
        assert!(waiter.join().unwrap());
    });
}

/// **Plusieurs canaux en lecture** (`next_any`, fan-in) : la fin ne tombe que
/// quand tous sont finis, et chaque élément dit de quel canal il vient.
#[test]
fn recv_any_ends_when_every_channel_has_ended() {
    let hub = StreamHub::<u32>::new();
    let (c1, c2) = (hub.channel("a.out → d.in", 4, 1, 1), hub.channel("b.out → d.in", 4, 1, 1));
    let w = hub.worker("d");
    hub.send(w, &[c2], 20).unwrap();
    hub.release_sender(c2);
    assert_eq!(hub.recv_any(w, &[c1, c2]).unwrap(), Some((1, 20)));
    hub.send(w, &[c1], 10).unwrap();
    hub.release_sender(c1);
    assert_eq!(hub.recv_any(w, &[c1, c2]).unwrap(), Some((0, 10)));
    assert_eq!(hub.recv_any(w, &[c1, c2]).unwrap(), None);
}
