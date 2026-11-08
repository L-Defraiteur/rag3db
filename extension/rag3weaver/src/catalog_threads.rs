//! **Les fils de fond d'un catalogue, un seul registre d'arrêt** (8 novembre 2026,
//! ticket « un backend écrit encore dans son dossier après son Drop ») : les
//! indexations de fond et le rebâti du plein texte s'y inscrivent, chacun avec son
//! drapeau d'arrêt, lu entre deux paquets. Un catalogue qui se ferme les arrête et
//! les attend tous ([`stop_and_join`]) : il ne garde aucun fil vivant, et rien
//! n'écrit plus dans son dossier quand il est parti.
//!
//! Le registre tient le catalogue par un `Weak` : un fil fini ne garde pas son
//! catalogue en vie (le registre d'avant, `INDEXINGS`, gardait l'`Arc` de chaque
//! indexation jusqu'à la suivante, et le catalogue d'un backend tombé survivait).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Weak};

use crate::catalog::Catalog;
use crate::dataflow::record_nodes::EmbedWatch;

/// Ce que fait un fil de fond : ce que les comptes distinguent (« un index tourne »
/// ne compte pas un rebâti), l'arrêt non.
#[derive(Clone)]
pub enum ThreadKind {
    /// Une indexation, avec le suivi de sa passe d'embarquement.
    Index(Arc<EmbedWatch>),
    /// Le rebâti du plein texte en fichiers, par lots.
    FtsRebuild,
}

struct BackgroundThread {
    catalog: Weak<Mutex<Catalog>>,
    stop: Arc<AtomicBool>,
    kind: ThreadKind,
    handle: std::thread::JoinHandle<()>,
}

static THREADS: LazyLock<Mutex<Vec<BackgroundThread>>> = LazyLock::new(Default::default);

fn of(t: &BackgroundThread, catalog: &Arc<Mutex<Catalog>>) -> bool {
    std::ptr::eq(t.catalog.as_ptr(), Arc::as_ptr(catalog))
}

/// **Inscrit un fil de fond** de ce catalogue. Les fils finis sortent du registre
/// au passage (leur fil est déjà joignable sans attendre).
pub fn register(catalog: &Arc<Mutex<Catalog>>, stop: Arc<AtomicBool>, kind: ThreadKind, handle: std::thread::JoinHandle<()>) {
    let mut fils = THREADS.lock().unwrap_or_else(|p| p.into_inner());
    fils.retain(|t| !t.handle.is_finished());
    fils.push(BackgroundThread { catalog: Arc::downgrade(catalog), stop, kind, handle });
}

/// Combien de fils de ce catalogue tournent encore, parmi ceux que `quoi` retient.
pub fn running(catalog: &Arc<Mutex<Catalog>>, quoi: impl Fn(&ThreadKind) -> bool) -> usize {
    THREADS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .filter(|t| of(t, catalog) && !t.handle.is_finished() && quoi(&t.kind))
        .count()
}

/// Le genre du dernier fil inscrit de ce catalogue que `quoi` retient (le suivi de
/// la dernière indexation).
pub fn latest(catalog: &Arc<Mutex<Catalog>>, quoi: impl Fn(&ThreadKind) -> bool) -> Option<ThreadKind> {
    THREADS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .rev()
        .find(|t| of(t, catalog) && quoi(&t.kind))
        .map(|t| t.kind.clone())
}

/// **Arrête tous les fils de ce catalogue**, au prochain paquet de chacun, et les
/// attend ; ils sortent du registre. Rend combien tournaient encore. À appeler hors
/// du runtime asynchrone et sans tenir le verrou du catalogue : l'attente bloque le
/// fil appelant, et un fil de fond prend ce verrou paquet par paquet.
pub fn stop_and_join(catalog: &Arc<Mutex<Catalog>>) -> usize {
    let siens: Vec<BackgroundThread> = {
        let mut tous = THREADS.lock().unwrap_or_else(|p| p.into_inner());
        let (siens, autres): (Vec<_>, Vec<_>) = std::mem::take(&mut *tous).into_iter().partition(|t| of(t, catalog));
        *tous = autres;
        siens
    };
    let n = siens.iter().filter(|t| !t.handle.is_finished()).count();
    for t in &siens {
        t.stop.store(true, Ordering::SeqCst);
    }
    for t in siens {
        let _ = t.handle.join();
    }
    n
}
