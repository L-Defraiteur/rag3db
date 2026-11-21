//! **The gate of the auto mode, v1** (page 07 of rag3weaver-archi, 21 Nov 2026): the hard guards
//! in Rust, the rules in `rules.json`, one judge reading the pair and the human turns only, and the
//! card when the judge does not authorize.

use std::path::PathBuf;

use super::{DaemonHost, GateJudge};

impl DaemonHost {
    /// The folder of the gate's configuration (`rules.json`, the judge's prompt).
    pub fn with_gate_dir(mut self, dir: PathBuf) -> Self {
        self.gate_dir = Some(dir);
        self
    }

    /// The gate's judge, given here instead of the model the configuration names.
    pub fn with_gate_judge(mut self, judge: GateJudge) -> Self {
        self.gate_judge = Some(judge);
        self
    }
}
