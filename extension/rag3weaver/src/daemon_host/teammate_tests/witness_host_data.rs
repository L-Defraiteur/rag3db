//! **A witness's host keeps its data in its own folder** (23 Nov 2026). The witnesses' host had no code roots, so it
//! fell back to the user's: the root grants of the judge's witnesses were written in the machine's real register
//! (1 226 of them at the second machine), and the witnesses running together lost each other's grants by rewriting
//! that file, which made `judge_all` fail one run in two. A module of its own.

use super::*;

#[test]
fn a_witness_host_keeps_its_data_in_its_own_folder() {
    let d = tempfile::tempdir().unwrap();
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model in this witness".into()) });
    let host = hote(d.path(), model);
    let data = host.racines_de_code().map(|r| r.data).unwrap_or_default();
    assert!(data.starts_with(d.path()), "the witness host's data folder is its own, not the user's: {}", data.display());
}
