//! **Les canaux de l'exécuteur concurrent** (page
//! `docs/11-octobre-2026-recherche/05-l-executeur-concurrent.md`, §3.2 et §3.6).
//!
//! Un run à flux tient **un** `StreamHub` : tous ses canaux bornés, tous ses
//! fils (les « ouvriers », un par nœud) et un seul verrou. Ce qui n'est pas
//! dans `std::sync::mpsc` et qui est la raison de ce module :
//!
//! - la **fin** d'un flux (plus d'émetteur) et l'**annulation** du run sont deux
//!   états : un lecteur ne prend jamais un flux interrompu pour un flux complet ;
//! - l'annulation **réveille** toute attente, en lecture comme en écriture ;
//! - l'**interblocage est exact** : chaque attente s'inscrit, et quand tous les
//!   ouvriers vivants attendent, le run est annulé avec une cause qui nomme ce
//!   que chacun attend. Pas de délai arbitraire.
//!
//! Un seul verrou pour tout le run : un graphe d'agent a des dizaines de nœuds,
//! pas des milliers, et la détection exacte demande une vue d'ensemble.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

pub(crate) use super::node::{CancelCause, Cancelled, Stopped};

/// Un canal : une arête du graphe.
pub(crate) type ChannelId = usize;
/// Un ouvrier : le fil d'un nœud.
pub(crate) type WorkerId = usize;

struct Channel<T> {
    /// « A.out → B.in », pour les messages.
    label: String,
    buf: VecDeque<T>,
    capacity: usize,
    senders: usize,
    receivers: usize,
}

enum Wait {
    Recv(Vec<ChannelId>),
    Send(ChannelId),
}

struct Workers {
    names: BTreeMap<WorkerId, String>,
    next_id: WorkerId,
}

type OnCancel = Box<dyn FnOnce() + Send>;

struct State<T> {
    channels: Vec<Channel<T>>,
    workers: Workers,
    waiting: BTreeMap<WorkerId, Wait>,
    cancel: Option<CancelCause>,
    /// Appelés une fois, hors du verrou, à l'annulation.
    on_cancel: Vec<OnCancel>,
}

impl<T> State<T> {
    /// Pose la cause (la première gagne) ; rend les rappels à appeler, une
    /// fois le verrou rendu.
    fn set_cancel(&mut self, cause: CancelCause) -> Vec<OnCancel> {
        if self.cancel.is_some() {
            return Vec::new();
        }
        self.cancel = Some(cause);
        std::mem::take(&mut self.on_cancel)
    }
}

fn call_all(callbacks: Vec<OnCancel>) {
    for f in callbacks {
        f();
    }
}

/// Les canaux d'un run, partagés par ses ouvriers.
pub(crate) struct StreamHub<T> {
    inner: Arc<(Mutex<State<T>>, Condvar)>,
}

impl<T> Clone for StreamHub<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + Send> StreamHub<T> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new((
                Mutex::new(State {
                    channels: Vec::new(),
                    workers: Workers { names: BTreeMap::new(), next_id: 0 },
                    waiting: BTreeMap::new(),
                    cancel: None,
                    on_cancel: Vec::new(),
                }),
                Condvar::new(),
            )),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State<T>> {
        // Un ouvrier qui panique sous le verrou ne doit pas rendre le run
        // muet : l'état reste lisible.
        self.inner.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Un canal borné, avec ses comptes d'émetteurs et de lecteurs, posés une
    /// fois pour toutes au montage (chaque `release_*` en retire un).
    pub(crate) fn channel(&self, label: impl Into<String>, capacity: usize, senders: usize, receivers: usize) -> ChannelId {
        let mut s = self.lock();
        s.channels.push(Channel {
            label: label.into(),
            buf: VecDeque::new(),
            capacity: capacity.max(1),
            senders,
            receivers,
        });
        s.channels.len() - 1
    }

    /// Un ouvrier vivant. Il doit finir par `worker_done`, sans quoi le
    /// détecteur le compte vivant pour toujours.
    pub(crate) fn worker(&self, name: impl Into<String>) -> WorkerId {
        let mut s = self.lock();
        let id = s.workers.next_id;
        s.workers.next_id += 1;
        s.workers.names.insert(id, name.into());
        id
    }

    /// Un ouvrier a fini : s'il était le seul à ne pas attendre, les autres
    /// sont interbloqués.
    pub(crate) fn worker_done(&self, worker: WorkerId) {
        let mut s = self.lock();
        s.workers.names.remove(&worker);
        s.waiting.remove(&worker);
        let callbacks = Self::detect_deadlock(&mut s);
        self.inner.1.notify_all();
        drop(s);
        call_all(callbacks);
    }

    /// Un émetteur de moins sur le canal : à zéro, le flux est fini.
    pub(crate) fn release_sender(&self, ch: ChannelId) {
        let mut s = self.lock();
        s.channels[ch].senders = s.channels[ch].senders.saturating_sub(1);
        self.inner.1.notify_all();
    }

    /// Un lecteur de moins : à zéro, ce qui reste est jeté et l'émetteur voit
    /// `Stopped::NoReader`.
    pub(crate) fn release_receiver(&self, ch: ChannelId) {
        let mut s = self.lock();
        let c = &mut s.channels[ch];
        c.receivers = c.receivers.saturating_sub(1);
        if c.receivers == 0 {
            c.buf.clear();
        }
        self.inner.1.notify_all();
    }

    /// Annule le run : toute attente se réveille sur `Cancelled`. La
    /// première cause est gardée.
    pub(crate) fn cancel(&self, cause: CancelCause) {
        let mut s = self.lock();
        let callbacks = s.set_cancel(cause);
        self.inner.1.notify_all();
        drop(s);
        call_all(callbacks);
    }

    /// `f` sera appelé une fois à l'annulation (tout de suite si le run l'est
    /// déjà), hors du verrou : il peut réveiller une attente extérieure.
    pub(crate) fn on_cancel(&self, f: impl FnOnce() + Send + 'static) {
        let mut s = self.lock();
        if s.cancel.is_some() {
            drop(s);
            f();
        } else {
            s.on_cancel.push(Box::new(f));
        }
    }

    /// Dort au plus `timeout` ; `true` si le run est annulé.
    pub(crate) fn wait_cancel_timeout(&self, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        let mut s = self.lock();
        while s.cancel.is_none() {
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            s = self.inner.1.wait_timeout(s, deadline - now).unwrap_or_else(|p| p.into_inner()).0;
        }
        true
    }

    pub(crate) fn cancelled(&self) -> Option<CancelCause> {
        self.lock().cancel.clone()
    }

    /// Le canal est fini : plus d'émetteur, plus rien en attente.
    pub(crate) fn ended(&self, ch: ChannelId) -> bool {
        let s = self.lock();
        s.channels[ch].senders == 0 && s.channels[ch].buf.is_empty()
    }

    /// Le même hub (deux poignées du même run).
    pub(crate) fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Pose un élément sur chaque canal encore lu de `chans` (un clone par
    /// canal, la valeur elle-même pour le dernier). Attend tant qu'un de ces
    /// canaux est plein. `NoReader` si plus aucun n'est lu.
    pub(crate) fn send(&self, worker: WorkerId, chans: &[ChannelId], value: T) -> Result<(), Stopped> {
        let mut s = self.lock();
        loop {
            if let Some(c) = &s.cancel {
                return Err(Stopped::Cancelled(c.clone()));
            }
            let live: Vec<ChannelId> = chans.iter().copied().filter(|&c| s.channels[c].receivers > 0).collect();
            if live.is_empty() {
                return Err(Stopped::NoReader);
            }
            match live.iter().copied().find(|&c| s.channels[c].buf.len() >= s.channels[c].capacity) {
                Some(full) => {
                    s = self.wait(s, worker, Wait::Send(full));
                }
                None => {
                    let (last, rest) = live.split_last().expect("au moins un canal lu");
                    for &c in rest {
                        s.channels[c].buf.push_back(value.clone());
                    }
                    s.channels[*last].buf.push_back(value);
                    self.inner.1.notify_all();
                    return Ok(());
                }
            }
        }
    }

    /// L'élément suivant de l'un des canaux de `chans`, avec la position du
    /// canal dans `chans` ; `None` quand tous sont finis (plus d'émetteur, plus
    /// rien en attente). Un canal est lu dans l'ordre de ses éléments ; entre
    /// canaux, le premier non vide dans l'ordre de `chans`.
    pub(crate) fn recv_any(&self, worker: WorkerId, chans: &[ChannelId]) -> Result<Option<(usize, T)>, Cancelled> {
        let mut s = self.lock();
        loop {
            if let Some(c) = &s.cancel {
                return Err(Cancelled(c.clone()));
            }
            if let Some(i) = chans.iter().position(|&c| !s.channels[c].buf.is_empty()) {
                let v = s.channels[chans[i]].buf.pop_front().expect("non vide");
                self.inner.1.notify_all();
                return Ok(Some((i, v)));
            }
            if chans.iter().all(|&c| s.channels[c].senders == 0) {
                return Ok(None);
            }
            s = self.wait(s, worker, Wait::Recv(chans.to_vec()));
        }
    }

    /// S'inscrit, vérifie l'interblocage, dort, se désinscrit.
    fn wait<'a>(&'a self, mut s: MutexGuard<'a, State<T>>, worker: WorkerId, what: Wait) -> MutexGuard<'a, State<T>> {
        s.waiting.insert(worker, what);
        let callbacks = Self::detect_deadlock(&mut s);
        if s.cancel.is_some() {
            self.inner.1.notify_all();
            s.waiting.remove(&worker);
            if !callbacks.is_empty() {
                drop(s);
                call_all(callbacks);
                return self.lock();
            }
            return s;
        }
        let mut s = self.inner.1.wait(s).unwrap_or_else(|p| p.into_inner());
        s.waiting.remove(&worker);
        s
    }

    /// Tous les ouvriers vivants attendent, et aucune attente ne peut être
    /// servie : c'est un interblocage, nommé.
    fn detect_deadlock(s: &mut State<T>) -> Vec<OnCancel> {
        if s.cancel.is_some() || s.workers.names.is_empty() {
            return Vec::new();
        }
        // Un ouvrier hors registre (bloqué sur une requête, sur le bus) est
        // vivant et n'attend pas nos canaux : rien n'est déclaré.
        if s.workers.names.keys().any(|w| !s.waiting.contains_key(w)) {
            return Vec::new();
        }
        // Une attente servable n'est pas bloquée : le réveil est en route.
        let servable = s.waiting.values().any(|w| match w {
            Wait::Recv(chans) => chans
                .iter()
                .any(|&c| !s.channels[c].buf.is_empty() || s.channels[c].senders == 0),
            Wait::Send(c) => {
                let ch = &s.channels[*c];
                ch.buf.len() < ch.capacity || ch.receivers == 0
            }
        });
        if servable {
            return Vec::new();
        }
        let describe = |w: &Wait| match w {
            Wait::Recv(chans) => format!(
                "attend une lecture sur {}",
                chans.iter().map(|&c| s.channels[c].label.as_str()).collect::<Vec<_>>().join(", ")
            ),
            Wait::Send(c) => format!("attend d'écrire sur {}, plein", s.channels[*c].label),
        };
        let lines: Vec<String> = s
            .waiting
            .iter()
            .map(|(w, what)| format!("{} {}", s.workers.names.get(w).map_or("?", String::as_str), describe(what)))
            .collect();
        s.set_cancel(CancelCause::Deadlock(lines.join(" ; ")))
    }
}

/// **Ce qu'un nœud voit de ses canaux** sous l'exécuteur concurrent : posé
/// dans son `NodeContext`, lu par `next`, `emit`, `close_input`.
pub(crate) struct StreamBinding {
    pub hub: StreamHub<super::port::PortValue>,
    pub worker: WorkerId,
    /// Une entrée lue élément par élément → ses canaux (un par arête).
    pub inputs: std::collections::HashMap<String, Vec<ChannelId>>,
    /// Ce que le runtime a déjà lu d'une entrée avant `execute` (son premier
    /// élément, et ce qui est arrivé pendant l'attente de départ).
    pub pending: std::collections::HashMap<String, VecDeque<super::port::PortValue>>,
    /// Une sortie branchée → ses canaux.
    pub outputs: std::collections::HashMap<String, Vec<ChannelId>>,
    /// Les éléments envoyés, par port de sortie.
    pub sent: std::collections::HashMap<String, usize>,
    /// Les robinets ouverts, par canal.
    pub taps: std::collections::HashMap<ChannelId, super::observe::EdgeTap>,
    /// Sous points de reprise : ce qui est parti sur chaque sortie, dans
    /// l'ordre, pour que la reprise le rejoue sans rejouer le nœud.
    pub recorded: Option<std::collections::HashMap<String, Vec<super::port::PortValue>>>,
}

impl StreamBinding {
    /// L'élément suivant d'une entrée lue élément par élément : ce qui attend
    /// déjà, puis ses canaux.
    pub(crate) fn next(&mut self, port: &str) -> Result<Option<super::port::PortValue>, Cancelled> {
        if let Some(v) = self.pending.get_mut(port).and_then(VecDeque::pop_front) {
            return Ok(Some(v));
        }
        let Some(chans) = self.inputs.get(port) else { return Ok(None) };
        match self.hub.recv_any(self.worker, chans)? {
            Some((i, v)) => {
                if let Some(tap) = self.taps.get(&chans[i]) {
                    tap.emit(&v);
                }
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    /// L'élément suivant de n'importe quelle entrée lue élément par élément,
    /// avec le nom de son port.
    pub(crate) fn next_any(&mut self) -> Result<Option<(String, super::port::PortValue)>, Cancelled> {
        let mut ports: Vec<&String> = self.pending.iter().filter(|(_, q)| !q.is_empty()).map(|(p, _)| p).collect();
        ports.sort();
        if let Some(port) = ports.first().map(|p| (*p).clone()) {
            let v = self.pending.get_mut(&port).and_then(VecDeque::pop_front).expect("non vide");
            return Ok(Some((port, v)));
        }
        let mut names: Vec<&String> = self.inputs.keys().collect();
        names.sort();
        let mut all = Vec::new();
        let mut owner = Vec::new();
        for name in names {
            for &c in &self.inputs[name] {
                all.push(c);
                owner.push(name.clone());
            }
        }
        if all.is_empty() {
            return Ok(None);
        }
        match self.hub.recv_any(self.worker, &all)? {
            Some((i, v)) => {
                if let Some(tap) = self.taps.get(&all[i]) {
                    tap.emit(&v);
                }
                Ok(Some((owner[i].clone(), v)))
            }
            None => Ok(None),
        }
    }

    /// N'en lira plus : ses canaux perdent ce lecteur.
    pub(crate) fn close_input(&mut self, port: &str) {
        self.pending.remove(port);
        if let Some(chans) = self.inputs.remove(port) {
            for c in chans {
                self.hub.release_receiver(c);
            }
        }
    }

    /// Envoie un élément sur une sortie branchée.
    pub(crate) fn send(&mut self, port: &str, value: super::port::PortValue) -> Result<(), Stopped> {
        let chans = self.outputs.get(port).cloned().unwrap_or_default();
        if let Some(recorded) = self.recorded.as_mut() {
            recorded.entry(port.to_string()).or_default().push(value.clone());
        }
        self.hub.send(self.worker, &chans, value)?;
        *self.sent.entry(port.to_string()).or_default() += 1;
        Ok(())
    }

    /// La fin du nœud : ses sorties finissent, ses entrées perdent leur lecteur.
    pub(crate) fn finish(&mut self) {
        for (_, chans) in self.outputs.drain() {
            for c in chans {
                self.hub.release_sender(c);
            }
        }
        for (_, chans) in self.inputs.drain() {
            for c in chans {
                self.hub.release_receiver(c);
            }
        }
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests;
