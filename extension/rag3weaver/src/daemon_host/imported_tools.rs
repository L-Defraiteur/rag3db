//! **Tools of another backend, offered to a backend's agent** (23 Nov 2026): the manifest's `imports` names another
//! backend (its manifest) and the tools of it offered here; the agent sees their declarations beside its own, and a call
//! goes to that backend, made for the agent's key so that a write is signed by it (`writes.author`). Nothing here names a
//! tool: the list is the manifest's.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use super::chat::OutilsImportes;
use super::{DaemonHost, Hosted};
use crate::agent::ToolBox;

impl DaemonHost {
    /// The tools that the backend `b` (hosted as `backend_id`) imports, ready for its chat's toolbox. An import whose
    /// backend does not open is said on the host's error output and left out; the others stay.
    pub(super) fn outils_importes<'a>(&'a self, b: &crate::backend::Backend, backend_id: &str) -> OutilsImportes<'a> {
        let p = b.prepared();
        let mut defs = Vec::new();
        let mut routes: HashMap<String, Arc<Hosted>> = HashMap::new();
        for import in &p.manifest.imports {
            let chemin = if import.backend.is_absolute() { import.backend.clone() } else { p.directory.join(&import.backend) };
            let leurs = self.hosted(&chemin).and_then(|h| self.with_backend(&h, |o| crate::backend_tools::BackendTools { backend: o }.tool_defs()).map(|d| (h, d)));
            match leurs {
                Ok((h, leurs)) => {
                    for d in leurs {
                        if import.tools.contains(&d.name) && !routes.contains_key(&d.name) {
                            routes.insert(d.name.clone(), h.clone());
                            defs.push(d);
                        }
                    }
                }
                Err(e) => eprintln!("[démon] {} : les outils importés de {} ne sont pas offerts : {e}", p.directory.display(), chemin.display()),
            }
        }
        let agent = self.agent_key_of_backend(backend_id);
        OutilsImportes::new(defs, move |call| {
            let h = routes.get(&call.name)?;
            let tour = crate::backend::as_agent(agent.as_deref(), || self.with_backend(h, |o| crate::backend_tools::BackendTools { backend: o }.call(call)));
            Some(tour.unwrap_or_else(|e| crate::llm::Turn::tool_result(&call.id, &call.name, json!({ "ok": false, "error": e }).to_string())))
        })
    }
}
