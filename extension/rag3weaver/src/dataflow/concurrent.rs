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
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;

use super::checkpoint::{
    port_value_from_checkpoint, port_value_to_checkpoint, CheckpointPortValue, CheckpointStore, ExecutionCheckpoint,
    NodeCheckpointStatus,
};

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

/// **Les points de reprise d'un run à flux** (page 05, §3.7). L'unité reste
/// le nœud fini ; un flux se range comme une suite de ports numérotés
/// (`port#0`, `port#1`…), chaque élément passant par le chemin d'une valeur :
/// le magasin et le format ne changent pas, un ancien point se relit.
pub(super) struct Checkpointing<'s> {
    pub store: &'s dyn CheckpointStore,
    pub execution_id: &'s str,
    pub checkpoint: &'s ExecutionCheckpoint,
    /// `Full` : l'état des nœuds ; sinon les entrées seules, une reprise rejoue tout.
    pub keep_state: bool,
}

/// Ce qu'un fil de nœud confie au persisteur (le fil principal).
enum Persist {
    Completed {
        node: String,
        outputs: HashMap<String, CheckpointPortValue>,
        undo: Option<serde_json::Value>,
        duration_ms: u64,
    },
    Failed {
        node: String,
        error: String,
    },
}

/// Le séparateur des éléments d'un flux rangé : aucun nom de port n'en a.
const ELEMENT: char = '#';

/// Les sorties rangées d'un nœud fini, relues : par port, sa valeur ou les
/// éléments de son flux, dans l'ordre.
fn restore_outputs(ports: &HashMap<String, CheckpointPortValue>) -> Result<HashMap<String, Vec<PortValue>>, String> {
    let mut by_port: HashMap<String, Vec<(usize, PortValue)>> = HashMap::new();
    for (key, cpv) in ports {
        let value = port_value_from_checkpoint(cpv.clone())?;
        match key.rsplit_once(ELEMENT).and_then(|(p, i)| i.parse::<usize>().ok().map(|i| (p, i))) {
            Some((port, i)) => by_port.entry(port.to_string()).or_default().push((i, value)),
            None => by_port.entry(key.clone()).or_default().push((0, value)),
        }
    }
    Ok(by_port
        .into_iter()
        .map(|(port, mut elements)| {
            elements.sort_by_key(|(i, _)| *i);
            (port, elements.into_iter().map(|(_, v)| v).collect())
        })
        .collect())
}

/// **Le pont vivant** (page 05, §4.1) : un sous-graphe qui s'exécute sur les
/// canaux de son parent. Ses ouvriers s'inscrivent au hub du parent sous
/// `<préfixe>/<nœud>` ; ses entrées et sorties libres déléguées sont les
/// canaux des arêtes extérieures.
pub(crate) struct Outer {
    pub hub: StreamHub<PortValue>,
    /// L'ouvrier du nœud délégant, qui quitte le registre une fois les
    /// ouvriers intérieurs inscrits.
    pub leave: WorkerId,
    pub prefix: String,
    /// (nœud intérieur, entrée) → les canaux extérieurs, et ce que le parent en
    /// avait déjà lu.
    pub inputs: HashMap<(String, String), (Vec<ChannelId>, VecDeque<PortValue>)>,
    /// (nœud intérieur, sortie) → les canaux extérieurs.
    pub outputs: HashMap<(String, String), Vec<ChannelId>>,
}

/// Ce qu'un fil sait de son nœud avant de partir.
struct Plan {
    name: String,
    /// Son nom dans le hub : préfixé dans un sous-graphe délégué.
    worker_name: String,
    node_type: String,
    worker: WorkerId,
    inputs: Vec<PortDef>,
    required_outputs: Vec<String>,
    per_element: HashSet<String>,
    stream_outputs: HashSet<String>,
    initial: HashMap<String, PortValue>,
    /// Des flux posés d'avance sur une entrée lue élément par élément.
    initial_streams: HashMap<String, Vec<PortValue>>,
    open_inputs: HashSet<String>,
    input_kinds: HashMap<String, super::port::PortKind>,
    /// Entrée → ses canaux, et les producteurs nommés (pour la raison d'un saut).
    in_chans: HashMap<String, Vec<ChannelId>>,
    producers: HashMap<String, Vec<String>>,
    out_chans: HashMap<String, Vec<ChannelId>>,
    taps: HashMap<ChannelId, EdgeTap>,
    /// Fini lors d'un run précédent : ses sorties, à rejouer sans l'exécuter.
    restored: Option<HashMap<String, Vec<PortValue>>>,
    /// La panne injectée des tests (service `fail_node`).
    fail: bool,
    /// Où confier ce qu'il faut persister, sous points de reprise.
    persist: Option<mpsc::Sender<Persist>>,
}

/// Comment un nœud a fini.
enum Ended {
    Completed {
        unconnected: HashMap<String, PortValue>,
        streams: HashMap<String, Vec<PortValue>>,
        sent: HashMap<String, usize>,
    },
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
        stream_ports: &HashMap<String, super::runtime::StreamPorts>,
        start: Instant,
        checkpointing: Option<Checkpointing<'_>>,
        mut outer: Option<Outer>,
    ) -> Result<DataflowOutput, String> {
        // Les nœuds finis d'un run précédent, dont les sorties se relisent ; un
        // lot qui ne se relit pas n'est pas un nœud fait : il est rejoué.
        let mut restored: HashMap<String, HashMap<String, Vec<PortValue>>> = HashMap::new();
        if let Some(c) = &checkpointing {
            for (node, cp) in &c.checkpoint.nodes {
                if cp.status != NodeCheckpointStatus::Completed {
                    continue;
                }
                match restore_outputs(&cp.output_ports) {
                    Ok(outputs) => {
                        restored.insert(node.clone(), outputs);
                    }
                    Err(e) => self.emit(DataflowEvent::NodeLog {
                        node: node.clone(),
                        node_type: String::new(),
                        level: super::node::NodeLogLevel::Warn,
                        text: format!("checkpoint : sortie non relue, le nœud sera rejoué ({e})"),
                    }),
                }
            }
        }
        let fail_node = self.services.get::<String>("fail_node").cloned();
        let mut initial_streams = std::mem::take(&mut graph.initial_streams);
        let (persist_tx, persist_rx) = mpsc::channel::<Persist>();
        let keep_state = checkpointing.as_ref().is_some_and(|c| c.keep_state);
        let capacity = self.services.get::<usize>("stream_capacity").copied().unwrap_or(64);
        let hub = outer.as_ref().map(|o| o.hub.clone()).unwrap_or_else(StreamHub::<PortValue>::new);
        if outer.is_none() {
            self.cancel.attach(&hub);
        }
        let prefix = outer.as_ref().map(|o| format!("{}/", o.prefix)).unwrap_or_default();
        let mut outer_inputs = outer.as_mut().map(|o| std::mem::take(&mut o.inputs)).unwrap_or_default();
        let outer_outputs = outer.as_mut().map(|o| std::mem::take(&mut o.outputs)).unwrap_or_default();
        for ((node, port), (_, pending)) in outer_inputs.iter_mut() {
            if !pending.is_empty() {
                let queue = initial_streams.entry(node.clone()).or_default().entry(port.clone()).or_default();
                let mut first: Vec<PortValue> = pending.drain(..).collect();
                first.append(queue);
                *queue = first;
            }
        }

        let mut out_chans: HashMap<(String, String), Vec<ChannelId>> = HashMap::new();
        let mut in_chans: HashMap<(String, String), Vec<ChannelId>> = HashMap::new();
        let mut producers: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut taps: HashMap<ChannelId, (String, EdgeTap)> = HashMap::new();
        for e in &graph.edges {
            let label = format!("{prefix}{}.{} → {prefix}{}.{}", e.from_node, e.from_port, e.to_node, e.to_port);
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
                let ports = stream_ports.get(&name).cloned().unwrap_or_default();
                let (per_element, stream_outputs, open_inputs, input_kinds) =
                    (ports.per_element, ports.stream_outputs, ports.open_inputs, ports.input_kinds);
                let inputs = node.inputs();
                let outputs = node.outputs();
                let mut in_c: HashMap<String, Vec<ChannelId>> = inputs
                    .iter()
                    .filter_map(|p| in_chans.get(&(name.clone(), p.name.to_string())).map(|c| (p.name.to_string(), c.clone())))
                    .collect();
                let mut out_c: HashMap<String, Vec<ChannelId>> = outputs
                    .iter()
                    .filter_map(|p| out_chans.get(&(name.clone(), p.name.to_string())).map(|c| (p.name.to_string(), c.clone())))
                    .collect();
                for ((n, p), (chans, _)) in &outer_inputs {
                    if *n == name {
                        in_c.entry(p.clone()).or_default().extend(chans.iter().copied());
                    }
                }
                for ((n, p), chans) in &outer_outputs {
                    if *n == name {
                        out_c.entry(p.clone()).or_default().extend(chans.iter().copied());
                    }
                }
                let worker_name = format!("{prefix}{name}");
                Plan {
                    worker: hub.worker(worker_name.clone()),
                    worker_name,
                    node_type: node.node_type().to_string(),
                    required_outputs: outputs.iter().filter(|p| p.required).map(|p| p.name.to_string()).collect(),
                    in_chans: in_c,
                    producers: inputs
                        .iter()
                        .filter_map(|p| producers.get(&(name.clone(), p.name.to_string())).map(|c| (p.name.to_string(), c.clone())))
                        .collect(),
                    out_chans: out_c,
                    taps: taps
                        .iter()
                        .filter(|(_, (to, _))| *to == name)
                        .map(|(c, (_, t))| (*c, t.clone()))
                        .collect(),
                    initial: initial_inputs.remove(&name).unwrap_or_default(),
                    initial_streams: initial_streams.remove(&name).unwrap_or_default(),
                    open_inputs,
                    input_kinds,
                    restored: restored.remove(&name),
                    fail: fail_node.as_deref() == Some(name.as_str()),
                    persist: checkpointing.is_some().then(|| persist_tx.clone()),
                    inputs,
                    per_element,
                    stream_outputs,
                    name,
                }
            })
            .collect();
        // Les ouvriers intérieurs sont inscrits : le nœud délégant quitte le
        // registre (il ne fait plus qu'attendre la portée ; le compter vivant
        // empêcherait le détecteur de conclure).
        if let Some(o) = &outer {
            hub.worker_done(o.leave);
        }

        let mut store_error: Option<String> = None;
        let ends: Vec<((String, String), Ended)> = std::thread::scope(|scope| {
            let handles: Vec<_> = graph
                .nodes
                .iter_mut()
                .zip(plans)
                .map(|(node, plan)| {
                    let hub = &hub;
                    scope.spawn(move || {
                        let names = (plan.name.clone(), plan.worker_name.clone());
                        let ended = self.run_one(node, plan, hub, run_id);
                        (names, ended)
                    })
                })
                .collect();
            // **Le persisteur** : le fil principal écrit au magasin ce que les
            // nœuds lui confient, dans l'ordre d'arrivée, jusqu'au dernier fil.
            drop(persist_tx);
            if let Some(c) = &checkpointing {
                for msg in persist_rx {
                    let written = match msg {
                        Persist::Completed { node, outputs, undo, duration_ms } if keep_state => c
                            .store
                            .save_node_completed(c.execution_id, &node, &outputs, undo.as_ref(), duration_ms)
                            .map_err(|e| (node, e)),
                        Persist::Completed { .. } => Ok(()),
                        Persist::Failed { node, error } => {
                            let _ = c.store.save_node_failed(c.execution_id, &node, &error);
                            Ok(())
                        }
                    };
                    if let Err((node, e)) = written {
                        if store_error.is_none() {
                            store_error = Some(format!("point de reprise de « {node} » non écrit : {e}"));
                            hub.cancel(CancelCause::Failed { node });
                        }
                    }
                }
            }
            handles.into_iter().map(|h| h.join().expect("un nœud a paniqué dans son fil")).collect()
        });
        if outer.is_none() {
            self.cancel.detach(&hub);
        }
        if let Some(error) = store_error {
            self.emit(DataflowEvent::Failed { error: error.clone() });
            return Err(error);
        }

        let cause = hub.cancelled();
        let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
        let mut skipped = HashMap::new();
        let mut streams = HashMap::new();
        let mut sent_by = HashMap::new();
        let mut failures = Vec::new();
        for ((name, worker_name), ended) in ends {
            match ended {
                Ended::Completed { unconnected, streams: elements, sent } => {
                    for (port, list) in elements {
                        streams.insert((name.clone(), port), list);
                    }
                    for (port, n) in sent {
                        sent_by.insert((name.clone(), port), n);
                    }
                    if !unconnected.is_empty() {
                        data.insert(name, unconnected);
                    }
                }
                Ended::Skipped(reason) => {
                    skipped.insert(name, reason);
                }
                Ended::Failed(error) => failures.push((worker_name, error)),
                Ended::Cancelled => {}
            }
        }
        // La cause d'abord : le nœud qui a annulé le run, ou l'interblocage.
        let error = match &cause {
            // Le nœud en cause, ou le sous-graphe qui le contient (`sous/x`).
            Some(CancelCause::Failed { node }) => failures
                .iter()
                .find(|(n, _)| n == node || node.starts_with(&format!("{n}/")))
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
        Ok(DataflowOutput::new(data, skipped, cause, streams).with_sent(sent_by))
    }

    /// **Le pont vivant** : exécute un sous-graphe sur les canaux délégués de
    /// son parent (voir [`Outer`]).
    pub(crate) fn execute_delegated(&self, graph: &mut DataflowGraph, outer: Outer, free_inputs: HashMap<(String, String), super::port::PortKind>) -> Result<DataflowOutput, String> {
        let run_id = crate::events::new_run_id("graph");
        let start = Instant::now();
        self.emit_run_started(&run_id, graph);
        let result = (|| {
            graph.validate()?;
            let stream_ports = super::runtime::stream_ports_of_with(graph, &free_inputs)?;
            let initial = std::mem::take(&mut graph.initial_inputs);
            self.execute_concurrent(graph, &run_id, initial, &stream_ports, start, None, Some(outer))
        })();
        self.emit_run_finished(&run_id, start.elapsed().as_millis() as u64, result.is_ok());
        result
    }

    /// Le fil d'un nœud : ses entrées de départ, le saut ou l'exécution, ses
    /// sorties, et la fin de ses canaux.
    fn run_one(&self, node: &mut Box<dyn Node>, plan: Plan, hub: &StreamHub<PortValue>, run_id: &str) -> Ended {
        let _guard = WorkerGuard { hub, worker: plan.worker, node: plan.worker_name.clone() };
        let mut binding = StreamBinding {
            hub: hub.clone(),
            worker: plan.worker,
            worker_name: plan.worker_name.clone(),
            inputs: HashMap::new(),
            pending: HashMap::new(),
            outputs: plan.out_chans.clone(),
            sent: HashMap::new(),
            taps: plan.taps.clone(),
            recorded: plan.persist.is_some().then(HashMap::new),
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

        // ── Fini lors d'un run précédent : ses sorties, rejouées ──────────
        if let Some(outputs) = &plan.restored {
            for chans in plan.in_chans.values() {
                for c in chans {
                    hub.release_receiver(*c);
                }
            }
            let mut ports: Vec<&String> = outputs.keys().collect();
            ports.sort();
            for port in ports {
                if !binding.outputs.contains_key(port) {
                    continue;
                }
                for value in &outputs[port] {
                    match binding.send(port, value.clone()) {
                        Ok(()) => {}
                        Err(super::node::Stopped::NoReader) => break,
                        Err(super::node::Stopped::Cancelled(_)) => return cancelled(&mut binding, "en rejouant ses sorties"),
                    }
                }
            }
            binding.finish();
            self.emit(DataflowEvent::CheckpointResumed { node: plan.name.clone(), output_ports: outputs.keys().cloned().collect() });
            return Ended::Completed { unconnected: HashMap::new(), streams: HashMap::new(), sent: binding.sent.clone() };
        }

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
                let first_seen = plan.per_element.contains(port)
                    && (pending.get(port).is_some_and(|q| !q.is_empty())
                        || plan.initial_streams.get(port).is_some_and(|q| !q.is_empty()));
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
        ctx.set_open_inputs(plan.open_inputs.clone());
        ctx.set_input_kinds(plan.input_kinds.clone());
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
                if let Some(elements) = plan.initial_streams.get(&port) {
                    for v in elements.iter().rev() {
                        queue.push_front(v.clone());
                    }
                }
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
                    plan.producers.get(&port).map(|p| p.join(", ")).unwrap_or_else(|| "l'entrée du sous-graphe".into())
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
        let result = if plan.fail {
            // Panne injectée (tests) : le nœud ne s'exécute pas.
            Err(format!("injected failure for node '{}'", plan.name))
        } else if node.is_async() {
            super::rt::bloquer("nœud asynchrone d'un graphe à flux", node.execute_async(&mut ctx)).and_then(|r| r)
        } else {
            node.execute(&mut ctx)
        };
        let duration_ms = started.elapsed().as_millis() as u64;
        let mut binding = ctx.take_stream().expect("le lien posé avant l'exécution");

        // Le run annulé pendant ce nœud : il est arrêté, sauf s'il est la cause.
        // La cause, c'est lui, ou un nœud de son sous-graphe délégué.
        let own_failure = matches!(hub.cancelled(), Some(CancelCause::Failed { ref node })
            if *node == plan.worker_name || node.starts_with(&format!("{}/", plan.worker_name)));
        if hub.cancelled().is_some() && !own_failure {
            return cancelled(&mut binding, "pendant son exécution");
        }
        let stream_elements = ctx.emitted_elements();
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
            hub.cancel(CancelCause::Failed { node: plan.worker_name.clone() });
            if let Some(tx) = &plan.persist {
                let _ = tx.send(Persist::Failed { node: plan.name.clone(), error: error.clone() });
            }
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
        if let Some(tx) = &plan.persist {
            // Ce qui est parti sur chaque sortie branchée : une valeur sous son
            // port, un flux en ports numérotés.
            let mut checkpoint_outputs = HashMap::new();
            for (port, elements) in binding.recorded.take().unwrap_or_default() {
                let stream = plan.stream_outputs.contains(&port);
                for (i, value) in elements.iter().enumerate() {
                    let key = if stream { format!("{port}{ELEMENT}{i}") } else { port.clone() };
                    match port_value_to_checkpoint(value) {
                        Ok(cpv) => {
                            checkpoint_outputs.insert(key, cpv);
                        }
                        Err(e) => {
                            let error = format!("{} : la sortie « {port} » ne se range pas au point de reprise : {e}", plan.name);
                            hub.cancel(CancelCause::Failed { node: plan.worker_name.clone() });
                            let _ = tx.send(Persist::Failed { node: plan.name.clone(), error: error.clone() });
                            binding.finish();
                            self.emit(DataflowEvent::NodeFailed { node: plan.name.clone(), error: error.clone() });
                            return Ended::Failed(error);
                        }
                    }
                }
            }
            let undo = node.undo_context().and_then(|b| b.downcast::<serde_json::Value>().ok().map(|v| *v));
            let _ = tx.send(Persist::Completed { node: plan.name.clone(), outputs: checkpoint_outputs, undo, duration_ms });
        }
        binding.finish();
        self.emit(DataflowEvent::NodeCompleted { node: plan.name.clone(), duration_ms, outputs: output_snapshots, metrics });
        Ended::Completed { unconnected, streams: stream_elements, sent: binding.sent.clone() }
    }
}

/// Une sortie posée : par valeur, par élément fusionné, ou envoyée sur ses canaux.
fn ctx_has_output(ctx: &NodeContext, binding: &StreamBinding, port: &str) -> bool {
    ctx.has_output(port) || binding.sent.get(port).is_some_and(|n| *n > 0)
}

#[cfg(test)]
mod tests;
