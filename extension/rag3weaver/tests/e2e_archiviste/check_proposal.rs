//! 5.2, une proposition de gabarit vérifiée sur une copie.
//!
//! Un fichier à ce lot (règle de c7, 21 nov. 2026) : un témoin neuf ne s'écrit jamais dans le corps d'un fichier de
//! tests partagé.

use super::*;

/// A copy of the templates tree, as a proposal of the memory template would be written, next to the one in service.
fn copy_of_template(dir: &Path, name: &str) -> std::path::PathBuf {
    fn copier(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let cible = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() { copier(&e.path(), &cible) } else { std::fs::copy(e.path(), &cible).unwrap(); }
        }
    }
    // The whole templates tree: the memory template cites its neighbours (`../notebook/graphs/…`, `../../tools/…`).
    copier(&dir.join("templates"), &dir.join(name));
    dir.join(format!("{name}/backends/memory/backend.json"))
}

/// **5.2, a proposal checked without touching the version in service** (20 November 2026, the page of the agent that
/// edits its memory workflow): `check_proposal` loads a copy of the template and checks it like a reload — what was
/// fixed at the opening must not change — and says its version; the template in service keeps serving.
#[test]
#[ignore]
fn a_proposal_is_checked_on_a_copy_without_touching_the_version_in_service() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let h0 = appel(&b, "list_turns", json!({}))["metadata"]["template"]["hash"].as_str().unwrap().to_string();
    // A valid proposal: a script edited in the copy.
    let manifest = copy_of_template(dir.path(), "proposal");
    let script = manifest.parent().unwrap().join("scripts/turns_list.rhai");
    let mut text = std::fs::read_to_string(&script).unwrap();
    text.push_str("\n// proposed\n");
    std::fs::write(&script, text).unwrap();
    let p = b.check_proposal(&manifest).expect("a valid proposal is accepted");
    assert_eq!(p.in_service, h0);
    assert_ne!(p.template_hash, h0, "another content, another version");
    assert_eq!(appel(&b, "list_turns", json!({}))["metadata"]["template"]["hash"], json!(h0), "the version in service is untouched");
    // A proposal that changes what was fixed at the opening is refused, naming it.
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    // (Turn gained the vector signal in the template on 23 November 2026: removing it is still an entity change.)
    m["entities"]["Turn"]["config"]["signals"] = json!(["bm25"]);
    std::fs::write(&manifest, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let e = b.check_proposal(&manifest).err().expect("an entity change is refused");
    assert!(e.contains("entities") && e.contains("migration"), "{e}");
    // A proposal whose graph does not load is refused, saying why.
    let manifest = copy_of_template(dir.path(), "broken");
    std::fs::write(manifest.parent().unwrap().join("graphs/list_turns.mmd"), "%% tool: list_turns\n%% description: broken\n%% result: x.result\n\ngraph LR\n    x[\"NoSuchNode()\"]\n").unwrap();
    let e = b.check_proposal(&manifest).err().expect("a graph that does not load is refused");
    assert!(e.contains("NoSuchNode"), "the refusal names what does not load: {e}");
}
