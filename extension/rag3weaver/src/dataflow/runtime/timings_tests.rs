//! **A run says how long each of its nodes took** (23 November 2026): the output keeps the duration of every node, by
//! name, whatever way the run went (live, with checkpoints, under a run id). The bench and the IDE read it from the
//! metadata of any tool without instrumenting it. A file of its own.

use super::*;
use crate::dataflow::checkpoint_store::MockCheckpointStore;
use crate::dataflow::port::PortDef;

struct Sleepy {
    name: String,
    ms: u64,
}

impl Node for Sleepy {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "Sleepy"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![]
    }
    fn execute(&mut self, _: &mut NodeContext) -> Result<(), String> {
        std::thread::sleep(std::time::Duration::from_millis(self.ms));
        Ok(())
    }
}

fn graph() -> DataflowGraph {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Sleepy { name: "slow".into(), ms: 40 })).unwrap();
    g.add_node(Box::new(Sleepy { name: "quick".into(), ms: 0 })).unwrap();
    g
}

fn check(output: &DataflowOutput, how: &str) {
    let t = output.timings();
    assert!(t.get("slow").is_some_and(|ms| *ms >= 35), "{how}: {t:?}");
    assert!(t.get("quick").is_some_and(|ms| *ms < 35), "{how}: {t:?}");
}

#[test]
fn a_live_run_keeps_the_duration_of_each_node() {
    check(&DataflowRuntime::new(10).execute(&mut graph()).unwrap(), "execute");
}

#[test]
fn a_run_with_checkpoints_keeps_the_duration_of_each_node() {
    let store = MockCheckpointStore::new();
    check(&DataflowRuntime::new(10).execute_with_checkpoint(&mut graph(), &store, "timings").unwrap(), "checkpoint");
}

#[test]
fn a_run_under_its_id_keeps_the_duration_of_each_node() {
    check(&DataflowRuntime::new(10).execute_as(&mut graph(), "graph:timings").unwrap(), "execute_as");
}
