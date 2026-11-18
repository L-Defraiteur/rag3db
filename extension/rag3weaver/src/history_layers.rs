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
    /// worktree with its content and mtime.
    fn empreinte(root: &Path) -> String {
        let mut e = vec![git(root, &["rev-parse", "HEAD"]), git(root, &["status", "--porcelain=v2"])];
        let index = root.join(".git/index");
        e.push(format!("{:?} {:?}", std::fs::read(&index).unwrap(), std::fs::metadata(&index).unwrap().modified().unwrap()));
        for p in ["f.rs", "g.rs"] {
            let f = root.join(p);
            e.push(format!("{p} {} {:?}", std::fs::read_to_string(&f).unwrap(), std::fs::metadata(&f).unwrap().modified().unwrap()));
        }
        e.join("\n")
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
