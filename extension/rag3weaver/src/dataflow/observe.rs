//! Tap system for edge-level debugging.
//!
//! A [`TapSpec`] identifies an edge to intercept. When data flows through
//! a tapped edge, a [`TapEvent`] is emitted with a clone of the value.
//! Zero cost when no taps are registered.

use serde::Serialize;

use super::graph::Edge;
use super::port::PortValue;

// ─── TapSpec ────────────────────────────────────────────────────────────────

/// Identifies an edge to tap.
#[derive(Debug, Clone)]
pub struct TapSpec {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

impl TapSpec {
    pub fn new(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> Self {
        Self {
            from_node: from_node.into(),
            from_port: from_port.into(),
            to_node: to_node.into(),
            to_port: to_port.into(),
        }
    }

    fn matches(&self, edge: &Edge) -> bool {
        self.from_node == edge.from_node
            && self.from_port == edge.from_port
            && self.to_node == edge.to_node
            && self.to_port == edge.to_port
    }
}

// ─── TapEvent ───────────────────────────────────────────────────────────────

/// Emitted when data flows through a tapped edge.
#[derive(Debug, Clone, Serialize)]
pub struct TapEvent {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
    #[serde(skip)]
    pub value: PortValue,
}

// ─── TapRegistry ────────────────────────────────────────────────────────────

/// Collects tap specs and broadcasts TapEvents when edges are matched.
pub(crate) struct TapRegistry {
    specs: Vec<TapSpec>,
    all: bool,
    tx: async_broadcast::Sender<TapEvent>,
    _rx: async_broadcast::InactiveReceiver<TapEvent>,
}

impl TapRegistry {
    pub fn new() -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(256);
        tx.set_overflow(true);
        Self {
            specs: Vec::new(),
            all: false,
            tx,
            _rx: rx.deactivate(),
        }
    }

    /// Subscribe to tap events.
    pub fn subscribe(&self) -> async_broadcast::Receiver<TapEvent> {
        self._rx.activate_cloned()
    }

    /// Tap a specific edge.
    pub fn add(&mut self, spec: TapSpec) {
        self.specs.push(spec);
    }

    /// Tap all edges.
    pub fn set_all(&mut self) {
        self.all = true;
    }

    /// Whether any taps are active.
    pub fn is_active(&self) -> bool {
        self.all || !self.specs.is_empty()
    }

    /// Check if an edge is tapped and emit a TapEvent if so.
    pub fn check_and_emit(&self, edge: &Edge, value: &PortValue) {
        if !self.is_active() {
            return;
        }

        let should_emit = self.all || self.specs.iter().any(|s| s.matches(edge));
        if !should_emit {
            return;
        }

        let _ = self.tx.try_broadcast(TapEvent {
            from_node: edge.from_node.clone(),
            from_port: edge.from_port.clone(),
            to_node: edge.to_node.clone(),
            to_port: edge.to_port.clone(),
            value: value.clone(),
        });
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_spec_matches_edge() {
        let spec = TapSpec::new("a", "out", "b", "in");
        let edge = Edge {
            from_node: "a".into(),
            from_port: "out".into(),
            to_node: "b".into(),
            to_port: "in".into(),
        };
        assert!(spec.matches(&edge));

        let other = Edge {
            from_node: "a".into(),
            from_port: "out".into(),
            to_node: "c".into(),
            to_port: "in".into(),
        };
        assert!(!spec.matches(&other));
    }

    #[test]
    fn tap_registry_inactive_by_default() {
        let registry = TapRegistry::new();
        assert!(!registry.is_active());
    }

    #[test]
    fn tap_registry_active_after_add() {
        let mut registry = TapRegistry::new();
        registry.add(TapSpec::new("a", "out", "b", "in"));
        assert!(registry.is_active());
    }

    #[test]
    fn tap_registry_emits_on_match() {
        let mut registry = TapRegistry::new();
        registry.add(TapSpec::new("a", "out", "b", "in"));
        let mut rx = registry.subscribe();

        let edge = Edge {
            from_node: "a".into(),
            from_port: "out".into(),
            to_node: "b".into(),
            to_port: "in".into(),
        };
        registry.check_and_emit(&edge, &PortValue::Trigger);

        let event = rx.try_recv().unwrap();
        assert_eq!(event.from_node, "a");
        assert_eq!(event.to_node, "b");
    }

    #[test]
    fn tap_registry_silent_on_no_match() {
        let mut registry = TapRegistry::new();
        registry.add(TapSpec::new("a", "out", "b", "in"));
        let mut rx = registry.subscribe();

        let edge = Edge {
            from_node: "x".into(),
            from_port: "out".into(),
            to_node: "y".into(),
            to_port: "in".into(),
        };
        registry.check_and_emit(&edge, &PortValue::Trigger);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn tap_all_emits_for_every_edge() {
        let mut registry = TapRegistry::new();
        registry.set_all();
        let mut rx = registry.subscribe();

        let edge1 = Edge {
            from_node: "a".into(),
            from_port: "out".into(),
            to_node: "b".into(),
            to_port: "in".into(),
        };
        let edge2 = Edge {
            from_node: "x".into(),
            from_port: "out".into(),
            to_node: "y".into(),
            to_port: "in".into(),
        };
        registry.check_and_emit(&edge1, &PortValue::Trigger);
        registry.check_and_emit(&edge2, &PortValue::Trigger);

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
    }
}
