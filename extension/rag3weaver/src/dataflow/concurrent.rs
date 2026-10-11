//! **L'exécuteur concurrent** (page
//! `docs/11-octobre-2026-recherche/05-l-executeur-concurrent.md`).
//!
//! Pris par `DataflowRuntime::execute_inner` dès qu'une sortie du graphe est
//! un flux (genres résolus). Tous les nœuds partent ensemble, un fil chacun
//! dans une portée bornée ; chaque arête est un canal borné (`dataflow::stream`).
//! Un nœud attend ses entrées de départ (le tout pour une entrée `Complete`,
//! le premier élément pour une entrée `PerElement` requise), est sauté si une
//! entrée requise finit vide, puis s'exécute pendant que l'amont émet encore.
//! Un échec annule le run ; les autres nœuds finissent `Cancelled`.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::graph::DataflowGraph;
use super::node::{merge_stream_elements, CancelCause, Node, NodeContext};
use super::observe::EdgeTap;
use super::port::{merge_port_values, PortDef, PortValue};
use super::runtime::{DataflowEvent, DataflowOutput, DataflowRuntime, PortSnapshot};
use super::stream::{ChannelId, StreamBinding, StreamHub, WorkerId};

/// **La poignée d'arrêt** d'un runtime (voir `DataflowRuntime::cancel_handle`).
/// Déclenchée, elle arrête les runs à flux en cours et ceux qui partiront
/// ensuite : le runtime est arrêté.
#[derive(Clone, Default)]
pub struct CancelHandle {
    inner: Arc<Mutex<HandleState>>,
}

#[derive(Default)]
struct HandleState {
    cancelled: bool,
    hubs: Vec<StreamHub<PortValue>>,
}

impl CancelHandle {
    pub fn cancel(&self) {
        let hubs = {
            let mut s = self.inner.lock().unwrap_or_else(|p| p.into_inner());
            s.cancelled = true;
            s.hubs.clone()
        };
        for h in hubs {
            h.cancel(CancelCause::External);
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).cancelled
    }

    fn attach(&self, hub: &StreamHub<PortValue>) {
        let mut s = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if s.cancelled {
            hub.cancel(CancelCause::External);
        }
        s.hubs.push(hub.clone());
    }

    fn detach(&self, hub: &StreamHub<PortValue>) {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).hubs.retain(|h| !h.same(hub));
    }
}

/// Ce qu'un fil sait de son nœud avant de partir.
struct Plan {
    name: String,
    node_type: String,
    worker: WorkerId,
    inputs: Vec<PortDef>,
    required_outputs: Vec<String>,
    per_element: HashSet<String>,
    stream_outputs: HashSet<String>,
    initial: HashMap<String, PortValue>,
    /// Entrée → ses canaux, et les producteurs nommés (pour la raison d'un saut).
    in_chans: HashMap<String, Vec<ChannelId>>,
    producers: HashMap<String, Vec<String>>,
    out_chans: HashMap<String, Vec<ChannelId>>,
    taps: HashMap<ChannelId, EdgeTap>,
}

/// Comment un nœud a fini.
enum Ended {
    Completed { unconnected: HashMap<String, PortValue> },
    Skipped(String),
    Failed(String),
    Cancelled,
}

/// Un ouvrier qui finit, même en paniquant, rend sa place au détecteur ; une
/// panique annule le run au lieu de le laisser attendre.
struct WorkerGuard<'a> {
    hub: &'a StreamHub<PortValue>,
    worker: WorkerId,
    node: String,
}

impl Drop for WorkerGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.hub.cancel(CancelCause::Failed { node: self.node.clone() });
        }
        self.hub.worker_done(self.worker);
    }
}

impl DataflowRuntime {
    pub(super) fn execute_concurrent(
        &self,
        graph: &mut DataflowGraph,
        run_id: &str,
        mut initial_inputs: HashMap<String, HashMap<String, PortValue>>,
        stream_ports: &HashMap<String, (HashSet<String>, HashSet<String>)>,
        start: Instant,
    ) -> Result<DataflowOutput, String> {
        let capacity = self.services.get::<usize>("stream_capacity").copied().unwrap_or(64);
        let hub = StreamHub::<PortValue>::new();
        self.cancel.attach(&hub);

        let mut out_chans: HashMap<(String, String), Vec<ChannelId>> = HashMap::new();
        let mut in_chans: HashMap<(String, String), Vec<ChannelId>> = HashMap::new();
        let mut producers: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut taps: HashMap<ChannelId, (String, EdgeTap)> = HashMap::new();
        for e in &graph.edges {
            let label = format!("{}.{} → {}.{}", e.from_node, e.from_port, e.to_node, e.to_port);
            let ch = hub.channel(label, capacity, 1, 1);
            out_chans.entry((e.from_node.clone(), e.from_port.clone())).or_default().push(ch);
            in_chans.entry((e.to_node.clone(), e.to_port.clone())).or_default().push(ch);
            producers
                .entry((e.to_node.clone(), e.to_port.clone()))
                .or_default()
                .push(format!("{}.{}", e.from_node, e.from_port));
            if let Some(tap) = self.taps.tap_of(e) {
                taps.insert(ch, (e.to_node.clone(), tap));
            }
        }

        // Les ouvriers s'inscrivent tous avant le premier départ : le
        // détecteur ne doit jamais voir un graphe à moitié lancé.
        let plans: Vec<Plan> = graph
            .nodes
            .iter()
            .map(|node| {
                let name = node.name().to_string();
                let (per_element, stream_outputs) = stream_ports.get(&name).cloned().unwrap_or_default();
                let inputs = node.inputs();
                let outputs = node.outputs();
                Plan {
                    worker: hub.worker(name.clone()),
                    node_type: node.node_type().to_string(),
                    required_outputs: outputs.iter().filter(|p| p.required).map(|p| p.name.to_string()).collect(),
                    in_chans: inputs
                        .iter()
                        .filter_map(|p| in_chans.get(&(name.clone(), p.name.to_string())).map(|c| (p.name.to_string(), c.clone())))
                        .collect(),
                    producers: inputs
                        .iter()
                        .filter_map(|p| producers.get(&(name.clone(), p.name.to_string())).map(|c| (p.name.to_string(), c.clone())))
                        .collect(),
                    out_chans: outputs
                        .iter()
                        .filter_map(|p| out_chans.get(&(name.clone(), p.name.to_string())).map(|c| (p.name.to_string(), c.clone())))
                        .collect(),
                    taps: taps
                        .iter()
                        .filter(|(_, (to, _))| *to == name)
                        .map(|(c, (_, t))| (*c, t.clone()))
                        .collect(),
                    initial: initial_inputs.remove(&name).unwrap_or_default(),
                    inputs,
                    per_element,
                    stream_outputs,
                    name,
                }
            })
            .collect();

        let ends: Vec<(String, Ended)> = std::thread::scope(|scope| {
            let handles: Vec<_> = graph
                .nodes
                .iter_mut()
                .zip(plans)
                .map(|(node, plan)| {
                    let hub = &hub;
                    scope.spawn(move || {
                        let name = plan.name.clone();
                        let ended = self.run_one(node, plan, hub, run_id);
                        (name, ended)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("un nœud a paniqué dans son fil")).collect()
        });
        self.cancel.detach(&hub);

        let cause = hub.cancelled();
        let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
        let mut skipped = HashMap::new();
        let mut failures = Vec::new();
        for (name, ended) in ends {
            match ended {
                Ended::Completed { unconnected } => {
                    if !unconnected.is_empty() {
                        data.insert(name, unconnected);
                    }
                }
                Ended::Skipped(reason) => {
                    skipped.insert(name, reason);
                }
                Ended::Failed(error) => failures.push((name, error)),
                Ended::Cancelled => {}
            }
        }
        // La cause d'abord : le nœud qui a annulé le run, ou l'interblocage.
        let error = match &cause {
            Some(CancelCause::Failed { node }) => failures
                .iter()
                .find(|(n, _)| n == node)
                .map(|(_, e)| e.clone())
                .or_else(|| Some(format!("le nœud « {node} » a paniqué"))),
            Some(CancelCause::Deadlock(why)) => Some(format!("interblocage des flux : {why}")),
            Some(CancelCause::External) | None => failures.first().map(|(_, e)| e.clone()),
        };
        if let Some(error) = error {
            self.emit(DataflowEvent::Failed { error: error.clone() });
            return Err(error);
        }
        self.emit(DataflowEvent::Completed {
            total_nodes: graph.nodes.len(),
            duration_ms: start.elapsed().as_millis() as u64,
        });
        Ok(DataflowOutput::new(data, skipped, cause))
    }

    /// Le fil d'un nœud : ses entrées de départ, le saut ou l'exécution, ses
    /// sorties, et la fin de ses canaux.
    fn run_one(&self, node: &mut Box<dyn Node>, plan: Plan, hub: &StreamHub<PortValue>, run_id: &str) -> Ended {
        let _guard = WorkerGuard { hub, worker: plan.worker, node: plan.name.clone() };
        let mut binding = StreamBinding {
            hub: hub.clone(),
            worker: plan.worker,
            inputs: HashMap::new(),
            pending: HashMap::new(),
            outputs: plan.out_chans.clone(),
            sent: HashMap::new(),
            taps: plan.taps.clone(),
        };
        let cancelled = |binding: &mut StreamBinding, at: &str| {
            binding.finish();
            self.emit(DataflowEvent::NodeCancelled {
                node: plan.name.clone(),
                node_type: plan.node_type.clone(),
                cause: hub.cancelled().map(|c| c.to_string()).unwrap_or_else(|| at.to_string()),
            });
            Ended::Cancelled
        };

        // ── Les entrées de départ ──────────────────────────────────────────
        let mut complete: HashMap<String, Vec<Vec<PortValue>>> = HashMap::new();
        let mut pending: HashMap<String, VecDeque<PortValue>> = HashMap::new();
        let mut waiting: Vec<(String, ChannelId, usize)> = Vec::new();
        for input in &plan.inputs {
            let port = input.name.to_string();
            let Some(chans) = plan.in_chans.get(&port) else { continue };
            let per_element = plan.per_element.contains(&port);
            if per_element && !input.required {
                continue;
            }
            complete.entry(port.clone()).or_insert_with(|| vec![Vec::new(); chans.len()]);
            for (i, c) in chans.iter().enumerate() {
                waiting.push((port.clone(), *c, i));
            }
        }
        loop {
            // Une entrée PerElement requise est servie dès son premier élément.
            waiting.retain(|(port, c, _)| {
                let first_seen = plan.per_element.contains(port) && pending.get(port).is_some_and(|q| !q.is_empty());
                !first_seen && !hub.ended(*c)
            });
            if waiting.is_empty() {
                break;
            }
            let chans: Vec<ChannelId> = waiting.iter().map(|(_, c, _)| *c).collect();
            match hub.recv_any(plan.worker, &chans) {
                Ok(Some((i, v))) => {
                    let (port, c, slot) = waiting[i].clone();
                    if let Some(tap) = plan.taps.get(&c) {
                        tap.emit(&v);
                    }
                    if plan.per_element.contains(&port) {
                        pending.entry(port).or_default().push_back(v);
                    } else {
                        complete.get_mut(&port).expect("entrée attendue")[slot].push(v);
                    }
                }
                Ok(None) => break,
                Err(_) => return cancelled(&mut binding, "en attendant ses entrées"),
            }
        }

        let mut ctx = NodeContext::with_services(self.services.clone());
        ctx.set_run_id(run_id);
        ctx.set_stream_ports(plan.per_element.clone(), plan.stream_outputs.clone());
        let mut missing: Option<String> = None;
        for input in &plan.inputs {
            let port = input.name.to_string();
            let initial = plan.initial.get(&port).cloned();
            let mut has_data = initial.is_some();
            if plan.per_element.contains(&port) {
                if let Some(chans) = plan.in_chans.get(&port) {
                    binding.inputs.insert(port.clone(), chans.clone());
                }
                let mut queue = pending.remove(&port).unwrap_or_default();
                if let Some(v) = initial {
                    queue.push_front(v);
                }
                has_data |= !queue.is_empty();
                if !queue.is_empty() {
                    binding.pending.insert(port.clone(), queue);
                }
                // Une entrée optionnelle n'attend pas ; une requise a son premier.
                if !input.required || !plan.in_chans.contains_key(&port) {
                    has_data = true;
                }
            } else {
                // Chaque canal est un flux, fusionné en entier ; les canaux d'un
                // même port se fusionnent comme un fan-in d'aujourd'hui.
                let per_channel: Vec<PortValue> = complete
                    .remove(&port)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|elements| {
                        let mut it = elements.into_iter();
                        let first = it.next()?;
                        Some(it.fold(first, |a, b| merge_stream_elements(a, b).unwrap_or(PortValue::Trigger)))
                    })
                    .collect();
                let merged = per_channel
                    .into_iter()
                    .reduce(|a, b| merge_port_values(a, b).unwrap_or(PortValue::Trigger))
                    .or(initial);
                if let Some(v) = merged {
                    has_data = true;
                    ctx.set_input(&port, v);
                }
            }
            if input.required && plan.in_chans.contains_key(&port) && !has_data && missing.is_none() {
                missing = Some(format!(
                    "entrée requise « {port} » sans donnée : {} n'a rien émis (branche non prise)",
                    plan.producers.get(&port).map(|p| p.join(", ")).unwrap_or_default()
                ));
            }
        }
        if let Some(reason) = missing {
            binding.finish();
            self.emit(DataflowEvent::NodeSkipped { node: plan.name.clone(), node_type: plan.node_type.clone(), reason: reason.clone() });
            return Ended::Skipped(reason);
        }

        // ── L'exécution ────────────────────────────────────────────────────
        let input_snapshots: Vec<PortSnapshot> =
            ctx.inputs().iter().map(|(name, value)| PortSnapshot::from_port(name, value)).collect();
        self.emit(DataflowEvent::NodeStarted { node: plan.name.clone(), node_type: plan.node_type.clone(), inputs: input_snapshots });
        ctx.bind_stream(binding);
        let started = Instant::now();
        let result = if node.is_async() {
            super::rt::bloquer("nœud asynchrone d'un graphe à flux", node.execute_async(&mut ctx)).and_then(|r| r)
        } else {
            node.execute(&mut ctx)
        };
        let duration_ms = started.elapsed().as_millis() as u64;
        let mut binding = ctx.take_stream().expect("le lien posé avant l'exécution");

        // Le run annulé pendant ce nœud : il est arrêté, sauf s'il est la cause.
        let own_failure = matches!(hub.cancelled(), Some(CancelCause::Failed { ref node }) if *node == plan.name);
        if hub.cancelled().is_some() && !own_failure {
            return cancelled(&mut binding, "pendant son exécution");
        }
        let result = result.and_then(|()| ctx.settle_streams()).and_then(|()| {
            let missing: Vec<&String> = plan.required_outputs.iter().filter(|p| !ctx_has_output(&ctx, &binding, p)).collect();
            if missing.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "{} : sortie requise non posée ({}) — une sortie requise doit émettre ; seule une sortie \
                     optionnelle peut finir vide",
                    plan.name,
                    missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ))
            }
        });
        self.publish(crate::events::CatalogEvent::NodeRun {
            run: run_id.to_string(),
            node: plan.name.clone(),
            node_type: plan.node_type.clone(),
            ms: duration_ms,
            error: result.as_ref().err().cloned(),
        });
        if let Err(error) = result {
            hub.cancel(CancelCause::Failed { node: plan.name.clone() });
            binding.finish();
            self.emit(DataflowEvent::NodeFailed { node: plan.name.clone(), error: error.clone() });
            return Ended::Failed(error);
        }

        // ── Les sorties ────────────────────────────────────────────────────
        let outputs = ctx.drain_outputs();
        let metrics = ctx.drain_metrics();
        for log in ctx.drain_logs() {
            self.emit(DataflowEvent::NodeLog { node: plan.name.clone(), node_type: plan.node_type.clone(), level: log.level, text: log.text });
        }
        let output_snapshots: Vec<PortSnapshot> = outputs.iter().map(|(n, v)| PortSnapshot::from_port(n, v)).collect();
        let mut unconnected = HashMap::new();
        for (port, value) in outputs {
            if binding.outputs.contains_key(&port) {
                // Plus personne ne lit : la valeur n'a pas d'aval, ce n'est pas une faute.
                if let Err(super::node::Stopped::Cancelled(_)) = binding.send(&port, value) {
                    return cancelled(&mut binding, "en rendant ses sorties");
                }
            } else {
                unconnected.insert(port, value);
            }
        }
        binding.finish();
        self.emit(DataflowEvent::NodeCompleted { node: plan.name.clone(), duration_ms, outputs: output_snapshots, metrics });
        Ended::Completed { unconnected }
    }
}

/// Une sortie posée : par valeur, par élément fusionné, ou envoyée sur ses canaux.
fn ctx_has_output(ctx: &NodeContext, binding: &StreamBinding, port: &str) -> bool {
    ctx.has_output(port) || binding.sent.get(port).is_some_and(|n| *n > 0)
}

#[cfg(test)]
mod tests;
