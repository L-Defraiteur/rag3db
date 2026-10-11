//! **`ReindexFileNode`** : un fichier changé sous une montre est réindexé,
//! un fichier supprimé est retiré de l'index (W2,
//! `docs/11-octobre-2026-everything-declarative/04-surveiller-les-fichiers.md`).
//!
//! Il lit les `FileChanged` que `EventSourceNode` a tirés du sujet `files`, et
//! passe par les deux chemins qui existent déjà : `code_sync::reingest_file`
//! (le fichier est là) et `code_sync::remove_file` (il n'y est plus).
//! **C'est le disque qui décide, pas l'événement** : un « removed » arrivé
//! après la recréation du fichier (un `git checkout`, un balayage en retard),
//! ou forgé par autre chose que la montre, ne retire pas un fichier présent. Pendant une
//! indexation entière de la source, il ne la double pas : le fichier est inscrit
//! au registre de reprise (`code_sync::note_change`), comme une édition.
//!
//! **Réservé aux réactions** (`backend_code::REACTION_NODES`) : c'est une
//! capacité nommée — écrire l'index du code — et non la porte de fichiers ou de
//! commandes. Il ne lit que ce que la source donne, sous sa racine.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::code_tools::{FileSource, FILE_SOURCE_SERVICE};

pub struct ReindexFileNode {
    name: String,
    /// Seulement les événements de cette montre ; vide : toutes.
    watch: String,
}

/// Ce qui est arrivé à un chemin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Reindexed,
    Removed,
    /// Une indexation entière tient la source : repris à sa fin.
    Deferred,
}

impl ReindexFileNode {
    /// Les chemins désignés, dédoublonnés, dans l'ordre. L'événement ne dit
    /// que **quel** fichier regarder ; ce qu'on en fait se lit sur le disque.
    fn todo(&self, events: &Value) -> BTreeSet<String> {
        events
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] == "FileChanged" && (self.watch.is_empty() || e["watch"] == self.watch.as_str()))
            .filter_map(|e| e["path"].as_str().map(str::to_string))
            .collect()
    }
}

impl Node for ReindexFileNode {
    fn node_type(&self) -> &str {
        "ReindexFileNode"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        ReindexFileNodeFactory.schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        ReindexFileNodeFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(json!({"watch": self.watch})))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let events = ctx
            .take_input("events")
            .and_then(|v| v.downcast::<Value>().map(|v| (*v).clone()))
            .unwrap_or(Value::Array(Vec::new()));
        let todo = self.todo(&events);
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("ReindexFileNode : pas de service 'catalog'")?;
        let source = ctx
            .service::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE)
            .cloned()
            .ok_or("ReindexFileNode : pas de source de fichiers (le backend n'a pas de workspace)")?;

        // Le registre du retard, et la cellule : celle de l'appel (le service
        // `scope`), sinon celle du catalogue — comme la lecture
        // (`read_catalog_in`), pour effacer là où la montre a noté.
        let call_cell = ctx.service::<crate::scope::Scope>("scope").cloned();
        let (lag, cell) = {
            let cat = catalog.lock().map_err(|e| format!("catalogue empoisonné : {e}"))?;
            (cat.file_lag(), call_cell.unwrap_or_else(|| cat.scope().clone()))
        };
        let (mut reindexed, mut removed, mut deferred) = (Vec::new(), Vec::new(), Vec::new());
        let mut errors = Vec::new();
        for path in todo {
            let (source_id, _) = crate::code_tools::indexed_name(source.as_ref(), &path);
            // Une indexation entière tient la source : le fichier est lu à son
            // paquet, ou repris à la fin — on ne la double pas. Si elle démarre
            // juste après ce regard, le fichier peut passer deux fois (ici puis à
            // son paquet) : sans dommage, la réingestion d'un même contenu est
            // idempotente (l'upsert inchangé est court-circuité).
            if crate::code_sync::indexing_in_progress(&source_id) {
                // En reprise, **sans effacer le retard** : rien n'est relu ici.
                // C'est la synchronisation qui l'effacera en relisant le fichier
                // (à son paquet ou à la reprise).
                crate::code_sync::note_change(&source_id, &path);
                deferred.push(path);
                continue;
            }
            let mut cat = catalog.lock().map_err(|e| format!("catalogue empoisonné : {e}"))?;
            // L'instant de la lecture : un changement venu après garde son retard.
            let read_at = crate::catalog::retard::now_ms();
            let outcome = match source.read(&path) {
                Ok(Some(content)) => crate::code_tools::reingest_file(&mut cat, source.as_ref(), &path, &content)
                    .map(|r| if r.deferred_to.is_some() { Outcome::Deferred } else { Outcome::Reindexed }),
                Ok(None) => crate::code_sync::remove_file(
                    &mut cat,
                    source.as_ref(),
                    &path,
                    crate::disponibilite::Disponibilites::TOUT,
                )
                .map(|_| Outcome::Removed),
                Err(e) => Err(e),
            };
            drop(cat);
            if matches!(outcome, Ok(Outcome::Reindexed | Outcome::Removed)) {
                lag.done(&cell, &path, read_at);
            }
            match outcome {
                Ok(Outcome::Reindexed) => reindexed.push(path),
                Ok(Outcome::Removed) => removed.push(path),
                Ok(Outcome::Deferred) => {
                    crate::code_sync::note_change(&source_id, &path);
                    deferred.push(path);
                }
                Err(e) => {
                    ctx.warn(&format!("ReindexFileNode : {path} : {e}"));
                    errors.push(json!({"path": path, "error": e}));
                }
            }
        }
        ctx.info(&format!(
            "ReindexFileNode : {} réindexé(s), {} retiré(s), {} repris plus tard, {} en échec",
            reindexed.len(),
            removed.len(),
            deferred.len(),
            errors.len()
        ));
        ctx.set_output(
            "report",
            PortValue::new(json!({"reindexed": reindexed, "removed": removed, "deferred": deferred, "errors": errors})),
        );
        Ok(())
    }
}

pub struct ReindexFileNodeFactory;

impl NodeFactory for ReindexFileNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let watch = match config.get("watch") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("'watch' : une chaîne, pas {other}")),
        };
        Ok(Box::new(ReindexFileNode { name: name.to_string(), watch }))
    }
    fn node_type(&self) -> &str {
        "ReindexFileNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ReindexFileNode".into(),
            description: "Réindexe les fichiers changés sous une montre (FileChanged du sujet 'files') et \
                          retire les supprimés ; pendant une indexation entière, les met en reprise. \
                          Réservé aux réactions."
                .into(),
            inputs: vec![PortDef::value("events", PortType::Map, false)],
            outputs: vec![PortDef::value("report", PortType::Map, true)],
            config_params: vec![ConfigParam {
                name: "watch".into(),
                param_type: ConfigParamType::String,
                required: false,
                default: Some(json!("")),
                description: "seulement les événements de cette montre ; vide : toutes".into(),
                choices: None,
                json_schema: None,
            }],
        }
    }
}
