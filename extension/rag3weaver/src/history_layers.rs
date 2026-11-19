//! **The history layers** (2a′-3, lot 3, `docs/arbre-principal/18-11-2026/03-2a-prime-3-en-sept-lots.md`): a
//! generation per commit of a tracked branch, written from the git objects — `git ls-tree`, `git diff-tree`,
//! `git cat-file` — **without any checkout**: no worktree, no index, no file of the repository changes.
//!
//! A generation of a branch's layer holds the files changed since the layer's last generation, or, for its first,
//! since the commit of its pinned parent's generation; the rest it reads from its parent through the stack.

use std::path::{Path, PathBuf};

use crate::catalog::{Catalog, LayerWrite};
use crate::code_tools::FileSource;

/// **A commit read through the git objects**, as a file source: its tree by `git ls-tree`, a file by
/// `git cat-file`. Its cursor is the worktree's (`worktree:<root>`), so its rows keep the source of the
/// backend's layer (`file`), and a layer's view masks its parent's rows of the same files.
pub struct GitCommitSource {
    root: PathBuf,
    commit: String,
}

impl GitCommitSource {
    pub fn new(root: impl Into<PathBuf>, commit: impl Into<String>) -> Self {
        GitCommitSource { root: root.into(), commit: commit.into() }
    }
}

/// `git -C <root> <args>`, read-only: its stdout, or the refusal with git's stderr.
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("git").arg("-C").arg(root).args(args).output().map_err(|e| format!("git {args:?} : {e}"))?;
    if !out.status.success() {
        return Err(format!("git {args:?} : {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

/// The paths of a `-z` output, in order.
fn paths_z(out: &[u8]) -> Vec<String> {
    out.split(|b| *b == 0).filter(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).into_owned()).collect()
}

impl FileSource for GitCommitSource {
    fn cursor(&self) -> String {
        format!("worktree:{}", self.root.display())
    }
    fn list(&self) -> Result<Vec<String>, String> {
        let mut paths = paths_z(&git(&self.root, &["ls-tree", "-r", "--name-only", "-z", &self.commit])?);
        paths.sort();
        Ok(paths)
    }
    /// The blob of `path` at the commit; `None` when the commit has no such file, or when it is not text.
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        match git(&self.root, &["cat-file", "blob", &format!("{}:{path}", self.commit)]) {
            Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
            Err(_) => Ok(None),
        }
    }
}

/// **Write the generation of `commit` in a history layer**, from the git objects of the repository at `root`, and
/// publish it: its rank. The files written are those changed since the layer's last generation, or since its
/// parent's generation for a first one, or the whole tree for a root layer.
pub fn write_history_generation(catalog: &mut Catalog, root: &Path, layer: &LayerWrite, commit: &str) -> Result<i64, String> {
    let source = GitCommitSource::new(root, commit);
    let base = match generation_commit(catalog, &layer.layer, None)? {
        Some(c) => Some(c),
        None => match &layer.parent {
            Some((couche, rang)) => generation_commit(catalog, couche, Some(*rang))?,
            None => None,
        },
    };
    let paths = match &base {
        Some(b) => paths_z(&git(root, &["diff-tree", "-r", "--name-only", "-z", "--no-renames", b, commit])?),
        None => source.list()?,
    };
    let mut files = Vec::with_capacity(paths.len());
    for p in paths {
        let content = source.read(&p)?;
        files.push((p, content));
    }
    catalog
        .write_in_layer(layer, |c| {
            crate::code_sync::reingest_files(c, &source, &files, crate::disponibilite::Disponibilites::TOUT)?;
            c.publier_la_generation(commit, 0).map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
}

/// The commit of a published generation of `layer`: at `rank`, or its last one; `None` when there is none, or when
/// it was published outside git (an empty commit).
fn generation_commit(catalog: &Catalog, layer: &str, rank: Option<i64>) -> Result<Option<String>, String> {
    use crate::connection::{CypherValue, QueryParam};
    let (filtre, mut params) = (
        if rank.is_some() { "AND g.rang = $rang" } else { "" },
        vec![QueryParam::new("couche", CypherValue::String(layer.to_string()))],
    );
    if let Some(r) = rank {
        params.push(QueryParam::new("rang", CypherValue::Int(r)));
    }
    let lu = catalog
        .execute_raw_with_params(
            &format!("MATCH (g:{}) WHERE g.couche = $couche {filtre} RETURN g.commit ORDER BY g.rang DESC LIMIT 1", crate::generations::TABLE),
            &params,
        )
        .map_err(|e| e.to_string())?;
    Ok(lu.rows.first().and_then(|r| r.first()).and_then(|c| c.as_str()).filter(|c| !c.is_empty()).map(str::to_string))
}

/// **Write the generation of a worktree's work layer** (2a′-3, lot 4): its uncommitted files, as `git status
/// --porcelain=v2` says them, read from its disk, pinned on the generation of `branch_layer` whose commit is the
/// worktree's HEAD. Without that generation, the work layer waits for `tip` and is not written. The files that
/// were dirty in the layer's last generation and are clean now leave it, and the parent's rows show again.
pub fn write_work_generation(catalog: &mut Catalog, worktree: &Path, layer_id: &str, branch_layer: &str) -> Result<i64, String> {
    use crate::connection::{CypherValue, QueryParam};
    let head = String::from_utf8_lossy(&git(worktree, &["rev-parse", "HEAD"])?).trim().to_string();
    let lu = catalog
        .execute_raw_with_params(
            &format!("MATCH (g:{}) WHERE g.couche = $couche AND g.commit = $commit RETURN max(g.rang)", crate::generations::TABLE),
            &[QueryParam::new("couche", CypherValue::String(branch_layer.to_string())), QueryParam::new("commit", CypherValue::String(head.clone()))],
        )
        .map_err(|e| e.to_string())?;
    let Some(parent_rank) = lu.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()) else {
        return Err(format!(
            "couche de travail {layer_id} : la couche {branch_layer} n'a pas de génération au HEAD {head} ; elle attend tip, et ne s'écrit pas sur une autre parente"
        ));
    };
    let dirty = dirty_paths(worktree)?;
    let before = last_tombstoned_files(catalog, layer_id)?;
    let source = crate::code_tools::WorkingTree::new(worktree);
    let mut files = Vec::new();
    for p in &dirty {
        files.push((p.clone(), source.read(p)?));
    }
    // Clean again: its rows leave the layer, and the parent's show.
    for p in before.difference(&dirty) {
        files.push((p.clone(), None));
    }
    let layer = LayerWrite { layer: layer_id.to_string(), parent: Some((branch_layer.to_string(), parent_rank)) };
    catalog
        .write_in_layer(&layer, |c| {
            crate::code_sync::reingest_files(c, &source, &files, crate::disponibilite::Disponibilites::TOUT)?;
            c.set_layer_tombstones(dirty.clone()).map_err(|e| e.to_string())?;
            c.publier_la_generation(&head, dirty.len()).map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
}

/// **The dirty paths of a worktree**: changed, added, deleted, renamed (both names), unmerged and untracked, as
/// `git status --porcelain=v2 -z --untracked-files=all` says them, without refreshing its index.
fn dirty_paths(worktree: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let out = git(worktree, &["--no-optional-locks", "status", "--porcelain=v2", "-z", "--untracked-files=all"])?;
    let mut paths = std::collections::BTreeSet::new();
    let mut entries = out.split(|b| *b == 0).filter(|e| !e.is_empty()).map(|e| String::from_utf8_lossy(e).into_owned());
    while let Some(e) = entries.next() {
        let field = |n: usize| e.splitn(n, ' ').nth(n - 1).map(str::to_string);
        match e.chars().next() {
            Some('1') => paths.extend(field(9)),
            Some('2') => {
                paths.extend(field(10));
                // The original path of a rename follows, as its own entry.
                paths.extend(entries.next());
            }
            Some('u') => paths.extend(field(11)),
            Some('?') => paths.extend(e.get(2..).map(str::to_string)),
            _ => {}
        }
    }
    Ok(paths)
}

/// The files the last generation of `layer` tombstones (empty without one).
fn last_tombstoned_files(catalog: &Catalog, layer: &str) -> Result<std::collections::BTreeSet<String>, String> {
    use crate::connection::{CypherValue, QueryParam};
    let lu = catalog
        .execute_raw_with_params(
            &format!("MATCH (g:{}) WHERE g.couche = $couche RETURN g.rang, g.tombstones ORDER BY g.rang DESC LIMIT 1", crate::generations::TABLE),
            &[QueryParam::new("couche", CypherValue::String(layer.to_string()))],
        )
        .map_err(|e| e.to_string())?;
    let Some(row) = lu.rows.first() else { return Ok(Default::default()) };
    let (rang, texte) = (row.first().and_then(|v| v.as_i64()).unwrap_or(0), row.get(1).and_then(|v| v.as_str()).unwrap_or("[]"));
    Ok(crate::views::read_tombstones(layer, rang, texte)?.1)
}

/// A worktree whose work layer is to be written: its layer id (the stable id of the tracked sources), its path,
/// and the history layer of its branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkLayerToWrite {
    pub layer: String,
    pub worktree: PathBuf,
    pub branch_layer: String,
}

/// **Write the work layers of `worktrees`**, under the cap `tracked.writes.max_work_layers`: a layer already written
/// is written again; a new one beyond the cap is refused, naming the setting. Each worktree's outcome, in order.
pub fn refresh_work_layers(
    catalog: &mut Catalog,
    worktrees: &[WorkLayerToWrite],
    settings: &crate::views::WriteSettings,
) -> Vec<(String, Result<i64, String>)> {
    let mut written: usize = worktrees.iter().filter(|w| has_generation(catalog, &w.layer).unwrap_or(false)).count();
    let mut issues = Vec::with_capacity(worktrees.len());
    for w in worktrees {
        let known = has_generation(catalog, &w.layer);
        let issue = match known {
            Err(e) => Err(e),
            Ok(false) if written >= settings.max_work_layers => Err(format!(
                "plafond des couches de travail atteint : {}, tracked.writes.max_work_layers ; la couche {} ({}) n'est pas écrite",
                settings.max_work_layers,
                w.layer,
                w.worktree.display()
            )),
            Ok(known) => {
                let r = write_work_generation(catalog, &w.worktree, &w.layer, &w.branch_layer);
                if r.is_ok() && !known {
                    written += 1;
                }
                r
            }
        };
        issues.push((w.layer.clone(), issue));
    }
    issues
}

/// Whether `layer` has a published generation.
fn has_generation(catalog: &Catalog, layer: &str) -> Result<bool, String> {
    use crate::connection::{CypherValue, QueryParam};
    let lu = catalog
        .execute_raw_with_params(
            &format!("MATCH (g:{}) WHERE g.couche = $couche RETURN count(*)", crate::generations::TABLE),
            &[QueryParam::new("couche", CypherValue::String(layer.to_string()))],
        )
        .map_err(|e| e.to_string())?;
    Ok(lu.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(0) > 0)
}

/// **Bring the tracked layers up to date** (2a′-3, lot 5c): for each tracked ref whose tip has no generation yet in
/// its layer, the generation of its tip (on its tracked parent's last generation, or on the backend's published rank
/// for a ref without a tracked parent), then the work layer of each tracked worktree other than the backend's, on its
/// branch's generation at its HEAD, under the cap. Each layer's outcome: (name, id, rank or refusal).
pub fn sync_tracked_layers(
    catalog: &mut Catalog,
    workspace_root: &Path,
    tracked: Option<&serde_json::Value>,
    settings: &crate::views::WriteSettings,
) -> Result<Vec<(String, String, Result<i64, String>)>, String> {
    use crate::suivies::ParenteResolue;
    let Some(t) = crate::suivies::tracked_layers(catalog, workspace_root, tracked)? else { return Ok(Vec::new()) };
    let backend = crate::generations::COUCHE_DU_BACKEND.to_string();
    let backend_rank = catalog.rang_publie().ok_or("les couches suivies : cette base n'a pas de générations")?;
    let full = |b: &str| if b.starts_with("refs/") { b.to_string() } else { format!("refs/heads/{b}") };
    let mut issues = Vec::new();
    // The history layers, parents first: a layer waits for its tracked parent's generation.
    let tracked_names: std::collections::BTreeSet<&str> = t.resolution.pile.couches.iter().map(|c| c.nom.as_str()).collect();
    let mut pending: Vec<&crate::suivies::CoucheResolue> = t.resolution.pile.couches.iter().collect();
    let mut done: std::collections::BTreeSet<String> = Default::default();
    while !pending.is_empty() {
        let before = pending.len();
        let mut rest = Vec::new();
        for couche in pending {
            let parent_name = match &couche.parente {
                ParenteResolue::Declaree(p) | ParenteResolue::Inferee { parente: p, .. } if tracked_names.contains(p.as_str()) => Some(p.clone()),
                _ => None,
            };
            if parent_name.as_ref().is_some_and(|p| !done.contains(p)) {
                rest.push(couche);
                continue;
            }
            done.insert(couche.nom.clone());
            let Some(id) = t.ids.get(&couche.nom).cloned() else { continue };
            let issue = if generation_commit(catalog, &id, None)?.as_deref() == Some(couche.pointe.as_str()) {
                last_rank(catalog, &id)
            } else {
                let parent = match parent_name.and_then(|p| t.ids.get(&p).cloned()) {
                    Some(pid) => match last_rank(catalog, &pid) {
                        Ok(r) => (pid, r),
                        Err(_) => (backend.clone(), backend_rank),
                    },
                    None => (backend.clone(), backend_rank),
                };
                write_history_generation(catalog, &t.repo, &LayerWrite { layer: id.clone(), parent: Some(parent) }, &couche.pointe)
            };
            issues.push((couche.nom.clone(), id, issue));
        }
        if rest.len() == before {
            // A cycle of parents (a flow): those layers are said, not written.
            for couche in rest {
                issues.push((couche.nom.clone(), t.ids.get(&couche.nom).cloned().unwrap_or_default(), Err(format!("couche {} : sa parente suivie ne s'écrit pas avant elle (un flux)", couche.nom))));
            }
            break;
        }
        pending = rest;
    }
    // The work layers of the tracked worktrees, the backend's own excepted, on their branch's layer.
    let mut work = Vec::new();
    let mut names = std::collections::BTreeMap::new();
    for tr in t.resolution.sources.iter().flat_map(|s| s.travail.iter()) {
        let name = format!("worktree:{}", tr.chemin.display());
        let Some(id) = t.ids.get(&name).cloned() else { continue };
        if id == crate::suivies::ID_DE_LA_COUCHE_DU_BACKEND || names.contains_key(&id) {
            continue;
        }
        let branch_layer = tr.branche.as_deref().and_then(|b| t.ids.get(&full(b)).cloned()).unwrap_or_else(|| backend.clone());
        names.insert(id.clone(), name);
        work.push(WorkLayerToWrite { layer: id, worktree: tr.chemin.clone(), branch_layer });
    }
    for (id, issue) in refresh_work_layers(catalog, &work, settings) {
        issues.push((names.get(&id).cloned().unwrap_or_default(), id, issue));
    }
    Ok(issues)
}

/// The last published rank of `layer`, or a refusal when it has none.
fn last_rank(catalog: &Catalog, layer: &str) -> Result<i64, String> {
    use crate::connection::{CypherValue, QueryParam};
    let lu = catalog
        .execute_raw_with_params(
            &format!("MATCH (g:{}) WHERE g.couche = $couche RETURN max(g.rang)", crate::generations::TABLE),
            &[QueryParam::new("couche", CypherValue::String(layer.to_string()))],
        )
        .map_err(|e| e.to_string())?;
    lu.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).ok_or_else(|| format!("la couche {layer} n'a pas de génération"))
}

/// **The backend's sync of its tracked layers, in service** (2a′-3, lot 5b-2): a thread that runs
/// [`sync_tracked_layers`] every `every`, under the catalog's lock, until stopped. A refusal is said on stderr once,
/// when it changes; a layer refused (the cap, a HEAD that waits for tip) does not stop the others.
pub struct LayerSyncHandle {
    stop: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl LayerSyncHandle {
    /// Stops the thread and waits for it (woken: no wait for the period).
    pub fn stop(mut self) {
        let (flag, cv) = &*self.stop;
        *flag.lock().unwrap_or_else(|e| e.into_inner()) = true;
        cv.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Starts the sync of the tracked layers of `workspace_root`. See [`LayerSyncHandle`].
pub fn spawn_layer_sync(
    catalog: std::sync::Arc<std::sync::Mutex<Catalog>>,
    workspace_root: PathBuf,
    tracked: Option<serde_json::Value>,
    settings: crate::views::WriteSettings,
    every: std::time::Duration,
) -> LayerSyncHandle {
    let stop: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)> = Default::default();
    let s = stop.clone();
    let thread = std::thread::Builder::new()
        .name("couches-suivies".into())
        .spawn(move || {
            let mut dit: Option<String> = None;
            loop {
                {
                    let (flag, cv) = &*s;
                    let g = flag.lock().unwrap_or_else(|e| e.into_inner());
                    if *g {
                        return;
                    }
                    let (g, _) = cv.wait_timeout(g, every).unwrap_or_else(|e| e.into_inner());
                    if *g {
                        return;
                    }
                }
                let issues = match catalog.lock() {
                    Ok(mut cat) => sync_tracked_layers(&mut cat, &workspace_root, tracked.as_ref(), &settings),
                    Err(_) => return,
                };
                let refus: Vec<String> = match issues {
                    Ok(issues) => issues.into_iter().filter_map(|(n, _, r)| r.err().map(|e| format!("{n} : {e}"))).collect(),
                    Err(e) => vec![e],
                };
                let texte = (!refus.is_empty()).then(|| refus.join(" ; "));
                if texte != dit {
                    if let Some(t) = &texte {
                        eprintln!("[rag3weaver] couches suivies : {t}");
                    }
                    dit = texte;
                }
            }
        })
        .ok();
    LayerSyncHandle { stop, thread }
}

/// **Catch up an older commit of a history layer** (2a′-3, lot 8): its generation is written at its commit's position
/// on the ref's first parent, in the gap under the generations already written, its rows retired at the next written
/// rank above. Its rank.
pub fn write_history_catch_up(catalog: &mut Catalog, root: &Path, layer: &LayerWrite, commit: &str) -> Result<i64, String> {
    // RED STUB.
    let _ = (catalog, root, layer, commit);
    Err("RED STUB: no catch-up".into())
}

/// **The duplicates at the upper edge of the gaps of a layer** (2a′-3, lot 8): rows born at a caught-up rank and retired
/// at the next written rank, whose content that rank holds identical. Counted in the state, never collapsed.
pub fn layer_duplicates(catalog: &Catalog, layer: &str) -> Result<usize, String> {
    // RED STUB.
    let _ = (catalog, layer);
    Ok(0)
}

/// **Whether a history layer still matches its ref** (2a′-3, lot 8): every written generation's commit must stand at
/// its rank on the ref's first parent. A rewritten ref (a rebase, a forced push) makes the layer « à rebâtir », naming
/// the ref and the lost commit.
pub fn check_layer_history(catalog: &Catalog, root: &Path, layer: &str, refname: &str) -> Result<(), String> {
    // RED STUB.
    let _ = (catalog, root, layer, refname);
    Ok(())
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;
    use crate::connection::{CypherValue, QueryParam};

    const F1: &str = "pub fn f() -> u32 {\n    1\n}\n";
    const F2: &str = "pub fn f() -> u32 {\n    let x = 2;\n    x\n}\n";
    const F3: &str = "pub fn f() -> u32 {\n    let x = 3;\n    let y = x;\n    y\n}\n";
    const G: &str = "pub fn g() -> u32 {\n    7\n}\n";

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
            .args(args)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// A repository on `main` (f = F1, g = G, commit c1), a branch `b` with two commits that change f (c2: F2,
    /// c3: F3) and are never checked out: the worktree stays on `main`.
    fn depot() -> (tempfile::TempDir, String, String, String) {
        let d = tempfile::tempdir().unwrap();
        git(d.path(), &["init", "-q", "-b", "main"]);
        std::fs::write(d.path().join("f.rs"), F1).unwrap();
        std::fs::write(d.path().join("g.rs"), G).unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-q", "-m", "c1"]);
        let c1 = git(d.path(), &["rev-parse", "HEAD"]);
        git(d.path(), &["checkout", "-q", "-b", "b"]);
        std::fs::write(d.path().join("f.rs"), F2).unwrap();
        git(d.path(), &["commit", "-q", "-am", "c2"]);
        let c2 = git(d.path(), &["rev-parse", "HEAD"]);
        std::fs::write(d.path().join("f.rs"), F3).unwrap();
        git(d.path(), &["commit", "-q", "-am", "c3"]);
        let c3 = git(d.path(), &["rev-parse", "HEAD"]);
        git(d.path(), &["checkout", "-q", "main"]);
        (d, c1, c2, c3)
    }

    /// The backend's layer indexed from the worktree (on `main`) and published at c1.
    fn base(root: &Path, c1: &str) -> (Catalog, i64) {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = Catalog::new(Box::new(conn), Box::new(crate::embedder::HashEmbedder::new(8)), crate::CatalogConfig {
            allow_mock_embedder: true,
            embedding_dim: 8,
            ..Default::default()
        });
        cat.initialize().unwrap();
        crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
        let src = crate::code_tools::WorkingTree::new(root);
        let lot: Vec<(String, Option<String>)> = ["f.rs", "g.rs"].iter().map(|p| (p.to_string(), Some(std::fs::read_to_string(root.join(p)).unwrap()))).collect();
        crate::code_sync::reingest_files(&mut cat, &src, &lot, crate::disponibilite::Disponibilites::TOUT).expect("l'index du backend");
        let n = cat.publier_la_generation(c1, 0).expect("publier");
        (cat, n)
    }

    /// The end lines of the visible `Scope` rows named `nom`, under the view of `couche` (its last rank).
    fn fins_dans_la_vue(cat: &mut Catalog, couche: &str, nom: &str) -> Vec<i64> {
        let pile = crate::views::load_stack(cat, &crate::views::ViewRequest { layer: couche.into(), rank: None }, &Default::default())
            .unwrap_or_else(|e| panic!("la vue {couche} : {e}"));
        let v = cat.generations.clone().expect("générations").with_stack(&pile);
        let mut fins = Vec::new();
        for etage in std::iter::once((v.couche.clone(), v.rang, std::collections::BTreeSet::<String>::new()))
            .chain(v.parents.iter().map(|p| (p.layer.clone(), p.rank, p.masked_files.clone())))
        {
            let (couche, rang, masques) = etage;
            let visible = cat.dialect_arc().condition_visible_sur("s", &couche, rang).unwrap();
            let lu = cat
                .conn()
                .execute_with_params(
                    &format!("MATCH (s:Scope) WHERE s.name = $n AND {visible} RETURN s.end_line, s.file_path"),
                    &[QueryParam::new("n", CypherValue::String(nom.into()))],
                )
                .unwrap();
            for l in lu.rows {
                let chemin = l[1].as_str().unwrap_or("").to_string();
                if !masques.contains(&chemin) {
                    fins.push(l[0].as_i64().unwrap_or(-1));
                }
            }
            if !fins.is_empty() {
                break;
            }
        }
        fins
    }

    /// A fingerprint of everything a checkout would touch: HEAD, the index file, the status, and every file of the
    /// worktree with its content and mtime. The status is read with `--no-optional-locks`: a plain `git status`
    /// refreshes the index itself when files share its second (racy git), and the fingerprint then changed with no
    /// write of ours (red once in the whole lib, 18 November 2026, alone it passed).
    fn empreinte(root: &Path) -> String {
        let mut e = vec![git(root, &["rev-parse", "HEAD"]), git(root, &["--no-optional-locks", "status", "--porcelain=v2"])];
        let index = root.join(".git/index");
        e.push(format!("{:?} {:?}", std::fs::read(&index).unwrap(), std::fs::metadata(&index).unwrap().modified().unwrap()));
        for p in ["f.rs", "g.rs"] {
            let f = root.join(p);
            e.push(format!("{p} {} {:?}", std::fs::read_to_string(&f).unwrap(), std::fs::metadata(&f).unwrap().modified().unwrap()));
        }
        e.join("\n")
    }

    /// **Témoin 1 : a view on another worktree sees that worktree's uncommitted file** (c7's witness, 2a′-3, lot 4).
    /// A worktree `wt` on branch `b` changes f and adds h, uncommitted; the backend's own worktree changes f too,
    /// uncommitted and never indexed. The view of `wt`'s work layer reads its f and its h, and g from the backend;
    /// the backend's view reads neither. A file that becomes clean again leaves the work layer; a HEAD without a
    /// generation of its branch waits for `tip`.
    #[test]
    fn a_view_on_another_worktree_sees_its_uncommitted_file() {
        let (d, c1, _, _) = depot();
        let (mut cat, n) = base(d.path(), &c1);
        let wt_dir = tempfile::tempdir().unwrap();
        let wt = wt_dir.path().join("wt");
        git(d.path(), &["worktree", "add", "-q", "-b", "travail", wt.to_str().unwrap(), &c1]);
        let branche = LayerWrite { layer: "refs/heads/travail".into(), parent: Some((crate::generations::COUCHE_DU_BACKEND.into(), n)) };
        write_history_generation(&mut cat, d.path(), &branche, &c1).expect("la génération de travail@c1");
        std::fs::write(d.path().join("f.rs"), F2).unwrap();
        std::fs::write(wt.join("f.rs"), F3).unwrap();
        std::fs::write(wt.join("h.rs"), "pub fn h() -> u32 {\n    9\n}\n").unwrap();
        let couche = format!("worktree:{}", wt.display());
        write_work_generation(&mut cat, &wt, &couche, "refs/heads/travail").expect("la génération de travail du worktree");
        assert_eq!(fins_dans_la_vue(&mut cat, &couche, "f"), vec![5], "the view of wt does not read its uncommitted f");
        assert_eq!(fins_dans_la_vue(&mut cat, &couche, "h"), vec![3], "the view of wt does not read its new h");
        assert_eq!(fins_dans_la_vue(&mut cat, &couche, "g"), vec![3], "the view of wt does not read g from the backend");
        assert_eq!(fins_dans_la_vue(&mut cat, crate::generations::COUCHE_DU_BACKEND, "f"), vec![3], "the backend's view reads wt's f");
        assert!(fins_dans_la_vue(&mut cat, crate::generations::COUCHE_DU_BACKEND, "h").is_empty(), "the backend's view reads wt's h");
        git(&wt, &["checkout", "-q", "--", "f.rs"]);
        write_work_generation(&mut cat, &wt, &couche, "refs/heads/travail").expect("la deuxième génération du worktree");
        assert_eq!(fins_dans_la_vue(&mut cat, &couche, "f"), vec![3], "f, clean again, does not come back from the parent");
        assert_eq!(fins_dans_la_vue(&mut cat, &couche, "h"), vec![3], "h, still uncommitted, left the view of wt");
        git(&wt, &["add", "h.rs"]);
        git(&wt, &["commit", "-q", "-m", "h"]);
        let refus = write_work_generation(&mut cat, &wt, &couche, "refs/heads/travail").expect_err("a HEAD without generation of its branch is written");
        assert!(refus.contains("tip"), "the refusal does not say the work layer waits for tip: {refus}");
    }

    /// **Témoin 4 : the cap of the work layers.** Under `tracked.writes.max_work_layers: 2`, three worktrees: the
    /// first two are written, the third is refused naming the setting, and written again later neither; the two
    /// written ones are written again. The manifest accepts the key, and refuses an unknown one under `writes`.
    #[test]
    fn the_cap_of_the_work_layers_refuses_the_one_beyond_naming_the_setting() {
        let tracked = serde_json::json!({"sources": [], "writes": {"max_work_layers": 2}});
        crate::suivies::declaration_du_manifeste(Some(&tracked)).expect("tracked refuses the key writes");
        let settings = crate::views::write_settings(Some(&tracked)).expect("the settings of the writes");
        assert_eq!(settings.max_work_layers, 2, "the cap is not read from tracked.writes");
        let mauvais = serde_json::json!({"writes": {"plafond": 2}});
        let refus = crate::views::write_settings(Some(&mauvais)).expect_err("an unknown key under writes is accepted");
        assert!(refus.contains("plafond"), "{refus}");
        let (d, c1, _, _) = depot();
        let (mut cat, n) = base(d.path(), &c1);
        let branche = LayerWrite { layer: "refs/heads/main".into(), parent: Some((crate::generations::COUCHE_DU_BACKEND.into(), n)) };
        write_history_generation(&mut cat, d.path(), &branche, &c1).expect("la génération de main@c1");
        let dossiers = tempfile::tempdir().unwrap();
        let worktrees: Vec<WorkLayerToWrite> = (0..3)
            .map(|i| {
                let wt = dossiers.path().join(format!("wt{i}"));
                git(d.path(), &["worktree", "add", "-q", "--detach", wt.to_str().unwrap(), &c1]);
                std::fs::write(wt.join(format!("n{i}.rs")), format!("pub fn n{i}() -> u32 {{\n    {i}\n}}\n")).unwrap();
                WorkLayerToWrite { layer: format!("travail-{i}"), worktree: wt, branch_layer: "refs/heads/main".into() }
            })
            .collect();
        for tour in 0..2 {
            let issues = refresh_work_layers(&mut cat, &worktrees, &settings);
            assert!(issues[0].1.is_ok() && issues[1].1.is_ok(), "tour {tour}: the first two are not written: {issues:?}");
            let refus = issues[2].1.as_ref().expect_err(&format!("tour {tour}: the third work layer is written beyond the cap"));
            assert!(refus.contains("tracked.writes.max_work_layers") && refus.contains('2'), "tour {tour}: the refusal does not name the setting: {refus}");
        }
        assert!(fins_dans_la_vue(&mut cat, "travail-1", "n1") == vec![3], "the second work layer is not read");
    }

    /// **The tracked layers are brought up to date** (2a′-3, lot 5c). Tracked: branch `b` and every worktree; a
    /// worktree `wt` on `b`, with an uncommitted `h`. A sync writes `b`'s tip on the backend and `wt`'s work layer on
    /// it: the view of `b` reads f as at its tip, the view of `wt` reads f and h. A second sync writes no new
    /// generation of `b` (its tip did not move); a commit on `b` gives one, and `wt` follows its HEAD.
    #[test]
    fn the_tracked_layers_are_brought_up_to_date() {
        let (d, c1, _, _) = depot();
        let (mut cat, _) = base(d.path(), &c1);
        let dossiers = tempfile::tempdir().unwrap();
        let wt = dossiers.path().join("wt");
        git(d.path(), &["worktree", "add", "-q", wt.to_str().unwrap(), "b"]);
        std::fs::write(wt.join("h.rs"), "pub fn h() -> u32 {\n    9\n}\n").unwrap();
        let tracked = serde_json::json!({"sources": [{"select": "branch:b"}, {"select": "worktree:*"}]});
        let settings = crate::views::WriteSettings::default();
        let issues = sync_tracked_layers(&mut cat, d.path(), Some(&tracked), &settings).expect("la synchro");
        let id_de = |fin: &str| -> String {
            issues.iter().find(|(n, _, _)| n.ends_with(fin)).map(|(_, id, _)| id.clone()).unwrap_or_else(|| panic!("no layer named …{fin}: {issues:?}"))
        };
        let (b, w) = (id_de("refs/heads/b"), id_de("/wt"));
        assert!(issues.iter().all(|(_, _, r)| r.is_ok()), "{issues:?}");
        assert_eq!(fins_dans_la_vue(&mut cat, &b, "f"), vec![5], "the view of b does not read f as at its tip");
        assert_eq!(fins_dans_la_vue(&mut cat, &w, "f"), vec![5], "the view of wt does not read f from b");
        assert_eq!(fins_dans_la_vue(&mut cat, &w, "h"), vec![3], "the view of wt does not read its uncommitted h");
        let generations = |cat: &Catalog, couche: &str| -> i64 {
            let lu = cat
                .execute_raw_with_params(
                    &format!("MATCH (g:{}) WHERE g.couche = $c RETURN count(*)", crate::generations::TABLE),
                    &[crate::connection::QueryParam::new("c", CypherValue::String(couche.into()))],
                )
                .unwrap();
            lu.rows[0][0].as_i64().unwrap()
        };
        sync_tracked_layers(&mut cat, d.path(), Some(&tracked), &settings).expect("la deuxième synchro");
        assert_eq!(generations(&cat, &b), 1, "a tip that did not move gave a new generation of b");
        std::fs::write(wt.join("f.rs"), F1).unwrap();
        git(&wt, &["commit", "-q", "-am", "c4"]);
        let issues = sync_tracked_layers(&mut cat, d.path(), Some(&tracked), &settings).expect("la troisième synchro");
        assert!(issues.iter().all(|(_, _, r)| r.is_ok()), "{issues:?}");
        assert_eq!(generations(&cat, &b), 2, "a commit on b gave no generation");
        assert_eq!(fins_dans_la_vue(&mut cat, &w, "f"), vec![3], "wt does not follow its HEAD on b");
    }

    /// **The sync runs in service** (2a′-3, lot 5b-2): started on a catalog, it writes a tracked worktree's new file
    /// into that worktree's view without any call, and stops when asked.
    #[test]
    fn the_sync_in_service_writes_a_worktree_s_new_file_into_its_view() {
        let (d, c1, _, _) = depot();
        let (cat, _) = base(d.path(), &c1);
        let dossiers = tempfile::tempdir().unwrap();
        let wt = dossiers.path().join("wt");
        git(d.path(), &["worktree", "add", "-q", wt.to_str().unwrap(), "b"]);
        let tracked = serde_json::json!({"sources": [{"select": "branch:b"}, {"select": "worktree:*"}], "writes": {"every_ms": 30}});
        let settings = crate::views::write_settings(Some(&tracked)).unwrap();
        assert_eq!(settings.every_ms, Some(30), "every_ms is not read from tracked.writes");
        let cat = std::sync::Arc::new(std::sync::Mutex::new(cat));
        let sync = spawn_layer_sync(cat.clone(), d.path().to_path_buf(), Some(tracked.clone()), settings, std::time::Duration::from_millis(30));
        std::fs::write(wt.join("h.rs"), "pub fn h() -> u32 {\n    9\n}\n").unwrap();
        let id = {
            let c = cat.lock().unwrap();
            let t = crate::suivies::tracked_layers(&c, d.path(), Some(&tracked)).unwrap().unwrap();
            t.ids.iter().find(|(n, _)| n.ends_with("/wt")).map(|(_, id)| id.clone()).expect("the layer of wt")
        };
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let vu = {
                let mut c = cat.lock().unwrap();
                crate::views::load_stack(&c, &crate::views::ViewRequest { layer: id.clone(), rank: None }, &Default::default()).is_ok()
                    && fins_dans_la_vue(&mut c, &id, "h") == vec![3]
            };
            if vu {
                break;
            }
            assert!(std::time::Instant::now() < fin, "the sync in service did not write wt's h in 20 s");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        sync.stop();
    }

    /// The end lines of `nom` under the view of `couche` at `rang`.
    fn fins_au_rang(cat: &mut Catalog, couche: &str, rang: i64, nom: &str) -> Vec<i64> {
        let pile = crate::views::load_stack(cat, &crate::views::ViewRequest { layer: couche.into(), rank: Some(rang) }, &Default::default())
            .unwrap_or_else(|e| panic!("la vue {couche}:{rang} : {e}"));
        let v = cat.generations.clone().expect("générations").with_stack(&pile);
        let mut fins = Vec::new();
        for (c, r, masques) in std::iter::once((v.couche.clone(), v.rang, std::collections::BTreeSet::<String>::new()))
            .chain(v.parents.iter().map(|p| (p.layer.clone(), p.rank, p.masked_files.clone())))
        {
            let visible = cat.dialect_arc().condition_visible_sur("s", &c, r).unwrap();
            let lu = cat
                .conn()
                .execute_with_params(
                    &format!("MATCH (s:Scope) WHERE s.name = $n AND {visible} RETURN s.end_line, s.file_path"),
                    &[QueryParam::new("n", CypherValue::String(nom.into()))],
                )
                .unwrap();
            for l in lu.rows {
                if !masques.contains(l[1].as_str().unwrap_or("")) {
                    fins.push(l[0].as_i64().unwrap_or(-1));
                }
            }
            if !fins.is_empty() {
                break;
            }
        }
        fins
    }

    /// **Témoin 7 : the catch-up of a history layer, and the duplicate at the upper edge of a gap** (2a′-3, lot 8).
    /// Branch b has two commits after main's c1: c2 changes f (F2) and k, c3 changes f again (F3) and leaves k. The tip
    /// is written first, at its position on the first parent (2); then c2 is caught up, in the gap, at rank 1. The view
    /// at 2 is unchanged, the view at 1 reads f as at c2, the view without rank still reads the tip; k, identical at 1
    /// and 2, is one duplicate at the upper edge, counted. A rewritten ref makes the layer « à rebâtir », named.
    #[test]
    fn the_catch_up_of_a_history_layer_and_the_duplicate_at_the_upper_edge() {
        const K1: &str = "pub fn k() -> u32 {\n    1\n}\n";
        const K2: &str = "pub fn k() -> u32 {\n    let a = 2;\n    a\n}\n";
        let d = tempfile::tempdir().unwrap();
        git(d.path(), &["init", "-q", "-b", "main"]);
        std::fs::write(d.path().join("f.rs"), F1).unwrap();
        std::fs::write(d.path().join("g.rs"), G).unwrap();
        std::fs::write(d.path().join("k.rs"), K1).unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-q", "-m", "c1"]);
        let c1 = git(d.path(), &["rev-parse", "HEAD"]);
        git(d.path(), &["checkout", "-q", "-b", "b"]);
        std::fs::write(d.path().join("f.rs"), F2).unwrap();
        std::fs::write(d.path().join("k.rs"), K2).unwrap();
        git(d.path(), &["commit", "-q", "-am", "c2"]);
        let c2 = git(d.path(), &["rev-parse", "HEAD"]);
        std::fs::write(d.path().join("f.rs"), F3).unwrap();
        git(d.path(), &["commit", "-q", "-am", "c3"]);
        let c3 = git(d.path(), &["rev-parse", "HEAD"]);
        git(d.path(), &["checkout", "-q", "main"]);
        // The backend's layer at c1 (g, f F1, k K1).
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = Catalog::new(Box::new(conn), Box::new(crate::embedder::HashEmbedder::new(8)), crate::CatalogConfig {
            allow_mock_embedder: true,
            embedding_dim: 8,
            ..Default::default()
        });
        cat.initialize().unwrap();
        crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking()).unwrap();
        let src = crate::code_tools::WorkingTree::new(d.path());
        let lot: Vec<(String, Option<String>)> = ["f.rs", "g.rs", "k.rs"].iter().map(|p| (p.to_string(), Some(std::fs::read_to_string(d.path().join(p)).unwrap()))).collect();
        crate::code_sync::reingest_files(&mut cat, &src, &lot, crate::disponibilite::Disponibilites::TOUT).unwrap();
        let n = cat.publier_la_generation(&c1, 0).unwrap();
        let b = LayerWrite { layer: "refs/heads/b".into(), parent: Some((crate::generations::COUCHE_DU_BACKEND.into(), n)) };
        // 1. The tip first, at its position on the first parent.
        let pointe = write_history_generation(&mut cat, d.path(), &b, &c3).expect("la pointe");
        assert_eq!(pointe, 2, "the tip is not at its position on the first parent (2 commits after c1)");
        let avant = (fins_au_rang(&mut cat, "refs/heads/b", 2, "f"), fins_au_rang(&mut cat, "refs/heads/b", 2, "k"));
        // 2. The catch-up of c2, in the gap.
        let rattrape = write_history_catch_up(&mut cat, d.path(), &b, &c2).expect("le rattrapage de c2");
        assert_eq!(rattrape, 1, "the caught-up commit is not at its position (1)");
        assert_eq!((fins_au_rang(&mut cat, "refs/heads/b", 2, "f"), fins_au_rang(&mut cat, "refs/heads/b", 2, "k")), avant, "the view at the tip changed");
        // 3. The view at 1 reads f as at c2; without rank, the tip.
        assert_eq!(fins_au_rang(&mut cat, "refs/heads/b", 1, "f"), vec![4], "the view at 1 does not read f as at c2");
        assert_eq!(fins_dans_la_vue(&mut cat, "refs/heads/b", "f"), vec![5], "the view without rank does not read the tip");
        // 4. k, identical at 1 and 2: one duplicate at the upper edge, counted.
        assert_eq!(layer_duplicates(&cat, "refs/heads/b").unwrap(), 1, "the duplicate at the upper edge is not counted");
        // 5. A rewritten ref: the layer is « à rebâtir », named.
        check_layer_history(&cat, d.path(), "refs/heads/b", "refs/heads/b").expect("an intact ref is said rewritten");
        git(d.path(), &["branch", "-f", "b", &c1]);
        git(d.path(), &["checkout", "-q", "b"]);
        std::fs::write(d.path().join("f.rs"), F3).unwrap();
        git(d.path(), &["commit", "-q", "-am", "c2-réécrit"]);
        git(d.path(), &["checkout", "-q", "main"]);
        let refus = check_layer_history(&cat, d.path(), "refs/heads/b", "refs/heads/b").expect_err("a rewritten ref is not seen");
        assert!(refus.contains("à rebâtir") && refus.contains("refs/heads/b"), "the refusal does not name the layer to rebuild: {refus}");
    }

    /// **Témoin 2 : a history layer is written without checkout.** Writing the generations of branch `b` (never
    /// checked out) changes neither HEAD, nor the index, nor the status, nor any file of the worktree.
    #[test]
    fn a_history_layer_is_written_without_checkout() {
        let (d, c1, c2, c3) = depot();
        let (mut cat, n) = base(d.path(), &c1);
        let avant = empreinte(d.path());
        let b = LayerWrite { layer: "refs/heads/b".into(), parent: Some((crate::generations::COUCHE_DU_BACKEND.into(), n)) };
        write_history_generation(&mut cat, d.path(), &b, &c2).expect("la génération de c2");
        write_history_generation(&mut cat, d.path(), &b, &c3).expect("la génération de c3");
        assert_eq!(empreinte(d.path()), avant, "writing a history layer touched the repository");
    }

    /// **Témoin 3 : a history layer reads its commit, on its pinned parent.** The view of `b` reads f as at each of
    /// its commits, and g, which `b` never changes, from its parent; the backend's view still reads the worktree's
    /// f. Each generation names its commit and its parent; the second holds only what changed since the first,
    /// and still tombstones f.
    #[test]
    fn a_history_layer_reads_its_commit_on_its_pinned_parent() {
        let (d, c1, c2, c3) = depot();
        let (mut cat, n) = base(d.path(), &c1);
        let b = LayerWrite { layer: "refs/heads/b".into(), parent: Some((crate::generations::COUCHE_DU_BACKEND.into(), n)) };
        let r2 = write_history_generation(&mut cat, d.path(), &b, &c2).expect("la génération de c2");
        assert_eq!(fins_dans_la_vue(&mut cat, "refs/heads/b", "f"), vec![4], "the view of b at c2 does not read f as at c2");
        assert_eq!(fins_dans_la_vue(&mut cat, "refs/heads/b", "g"), vec![3], "the view of b does not read g from its parent");
        let r3 = write_history_generation(&mut cat, d.path(), &b, &c3).expect("la génération de c3");
        assert!(r3 > r2, "a new rank: {r2} then {r3}");
        assert_eq!(fins_dans_la_vue(&mut cat, "refs/heads/b", "f"), vec![5], "the view of b at c3 does not read f as at c3");
        assert_eq!(fins_dans_la_vue(&mut cat, crate::generations::COUCHE_DU_BACKEND, "f"), vec![3], "the backend's view reads b's f");
        let g = cat
            .conn()
            .execute(&format!(
                "MATCH (g:{}) WHERE g.couche = 'refs/heads/b' RETURN g.rang, g.commit, g.parente_couche, g.parente_rang, g.tombstones ORDER BY g.rang",
                crate::generations::TABLE
            ))
            .unwrap();
        let lignes: Vec<(i64, String, String, i64, String)> = g
            .rows
            .iter()
            .map(|l| (l[0].as_i64().unwrap(), l[1].as_str().unwrap_or("").into(), l[2].as_str().unwrap_or("").into(), l[3].as_i64().unwrap(), l[4].as_str().unwrap_or("").into()))
            .collect();
        assert_eq!(lignes.len(), 2, "{lignes:?}");
        assert_eq!((lignes[0].1.as_str(), lignes[1].1.as_str()), (c2.as_str(), c3.as_str()), "each generation names its commit");
        assert!(lignes.iter().all(|l| l.2 == crate::generations::COUCHE_DU_BACKEND && l.3 == n), "each generation is pinned on backend:{n}: {lignes:?}");
        assert!(lignes.iter().all(|l| l.4.contains("f.rs") && !l.4.contains("g.rs")), "the tombstones are f.rs alone: {lignes:?}");
    }
}
