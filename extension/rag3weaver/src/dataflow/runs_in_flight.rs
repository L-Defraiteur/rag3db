//! **Les runs en vol d'un backend, et leur arrêt** (C4 de l'atelier, page 05
//! de everything declarative) : un seul chemin d'arrêt pour tout run, quel
//! que soit qui l'a lancé — un appel d'outil, une réaction, un sous-graphe.
//!
//! Un runtime dont les services portent ce registre (`RUNS_IN_FLIGHT_SERVICE`)
//! y inscrit sa poignée d'arrêt sous l'id de son run, le temps du run
//! ([`DataflowRuntime::execute_as`](super::runtime::DataflowRuntime::execute_as)),
//! et l'en retire à la fin, même sur panique. [`RunsInFlight::stop`] la
//! déclenche : le run finit `requested`, avec qui l'a demandé
//! (`RunFinished.cancelled_by`), comme l'arrêt d'un run en fond
//! (`background::cause::REQUESTED`). La page de l'IDE n'en est qu'un client.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::concurrent::CancelHandle;

/// Le nom du service qui porte le registre.
pub const RUNS_IN_FLIGHT_SERVICE: &str = "runs_in_flight";

/// Ce qui arrête un run, au nom de qui le demande : la poignée d'un runtime
/// ([`RunsInFlight::register`]), ou l'arrêt d'une boucle d'agent (un tour de
/// chat, [`RunsInFlight::register_stop`]).
pub type Stop = Arc<dyn Fn(&str) + Send + Sync>;

/// Les runs en vol : id de run → ce qui l'arrête.
#[derive(Clone, Default)]
pub struct RunsInFlight {
    runs: Arc<Mutex<HashMap<String, Stop>>>,
}

/// Un run inscrit ; il sort du registre à la chute de la garde.
pub struct Registered {
    runs: Arc<Mutex<HashMap<String, Stop>>>,
    run: String,
}

impl Drop for Registered {
    fn drop(&mut self) {
        self.runs.lock().unwrap_or_else(|p| p.into_inner()).remove(&self.run);
    }
}

impl RunsInFlight {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inscrit le run d'un runtime, par sa poignée, le temps de la garde rendue.
    pub fn register(&self, run: &str, handle: CancelHandle) -> Registered {
        self.register_stop(run, Arc::new(move |by: &str| handle.cancel_by(by)))
    }

    /// Inscrit un run par ce qui l'arrête (une boucle d'agent n'a pas de
    /// poignée de runtime), le temps de la garde rendue.
    pub fn register_stop(&self, run: &str, stop: Stop) -> Registered {
        self.runs.lock().unwrap_or_else(|p| p.into_inner()).insert(run.to_string(), stop);
        Registered { runs: self.runs.clone(), run: run.to_string() }
    }

    /// Le run est-il en vol ?
    pub fn contains(&self, run: &str) -> bool {
        self.runs.lock().unwrap_or_else(|p| p.into_inner()).contains_key(run)
    }

    /// Les runs en vol, triés.
    pub fn runs(&self) -> Vec<String> {
        let mut v: Vec<String> = self.runs.lock().unwrap_or_else(|p| p.into_inner()).keys().cloned().collect();
        v.sort();
        v
    }

    /// **Arrête `run`**, au nom de `by`. Un run qui n'est pas (ou plus) en vol
    /// est un refus nommé, et rien ne bouge.
    pub fn stop(&self, run: &str, by: &str) -> Result<(), String> {
        let stop = self.runs.lock().unwrap_or_else(|p| p.into_inner()).get(run).cloned();
        match stop {
            Some(stop) => {
                stop(by);
                Ok(())
            }
            None => Err(format!("aucun run « {run} » en vol : il a fini, ou il n'est pas de ce backend ; rien n'est arrêté")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_run_inscrit_s_arrete_et_sort_du_registre_a_la_chute_de_sa_garde() {
        let r = RunsInFlight::new();
        let h = CancelHandle::default();
        let garde = r.register("g-1", h.clone());
        assert!(r.contains("g-1"));
        r.stop("g-1", "ide").unwrap();
        assert!(h.is_cancelled());
        assert_eq!(h.requested_by().as_deref(), Some("ide"));
        drop(garde);
        assert!(!r.contains("g-1"));
        let refus = r.stop("g-1", "ide").unwrap_err();
        assert!(refus.contains("aucun run « g-1 » en vol"), "{refus}");
    }
}
