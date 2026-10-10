//! **Le runtime tokio du crate** — paresseux : un binaire qui n'exécute
//! aucun graphe ne le paie pas (il naît au premier pont synchrone).
//!
//! Multi-fil parce que `block_in_place` l'exige — c'est le pont des nœuds
//! d'aujourd'hui (`Node::execute` joué depuis `execute_async`). Deux fils de
//! travail suffisent : les nœuds synchrones bloquent sous `block_in_place`
//! (le fil sort du pool le temps de l'appel, tokio en démarre un autre au
//! besoin), et le parallélisme des niveaux tout-synchrones reste celui des
//! fils de portée de `run_level` — le pool ne porte que la coordination et
//! les nœuds vraiment asynchrones. (Chantier C, 10 octobre 2026.)

use std::sync::OnceLock;

pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("rag3weaver-graphes")
            .build()
            .expect("le runtime tokio du crate")
    })
}

/// **Le pont synchrone** : joue un futur sur le runtime du crate, depuis un
/// fil ordinaire. Depuis une tâche tokio, c'est un refus NOMMÉ — jamais la
/// panique « cannot block the current thread from within a runtime » : une
/// erreur qui se lit coûte trois lignes, une panique dans un gestionnaire
/// coûte une soirée (session mémoire, 10 octobre 2026).
pub(crate) fn bloquer<F: std::future::Future>(quoi: &str, futur: F) -> Result<F::Output, String> {
    match tokio::runtime::Handle::try_current() {
        Err(_) => Ok(runtime().block_on(futur)),
        // La réentrance sur un runtime MULTI-FIL est permise par
        // `block_in_place` : c'est le « graphe dans le graphe » de
        // l'ingestion — un nœud synchrone appelle le catalogue, qui exécute
        // un sous-graphe par ce même pont.
        Ok(h) if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            Ok(tokio::task::block_in_place(|| h.block_on(futur)))
        }
        Ok(_) => Err(format!(
            "{quoi} : impossible de bloquer un runtime tokio à fil unique — \
             prenez la variante async, ou un runtime multi-fil"
        )),
    }
}
