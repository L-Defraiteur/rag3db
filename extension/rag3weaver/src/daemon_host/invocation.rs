//! Résolution d'une référence explicite ; jamais le premier dépôt disponible.
use crate::dataflow::invocation_nodes::{InvocationAuthority, InvocationHost};
use crate::dataflow::node::CancelToken;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, Weak};

pub(super) struct HostedInvocations {
    pub host: Weak<super::DaemonHost>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    target: Target,
    #[serde(default)]
    view: Option<Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    kind: String,
    id: String,
}
impl InvocationHost for HostedInvocations {
    fn invoke(
        &self,
        authority: &InvocationAuthority,
        context: &Value,
        operation: &str,
        arguments: Value,
        parent_run: &str,
        cancel: CancelToken,
    ) -> Result<Value, String> {
        let context: Context = serde_json::from_value(context.clone())
            .map_err(|e| format!("contexte invalide : {e}"))?;
        // **The current view by the context** (2b-C): the context's view goes into the arguments, where the call
        // reads, checks and refuses it as `view`; an explicit `view` in the arguments wins. No implicit view without a
        // context.
        let mut arguments = arguments;
        if let Some(view) = context.view.clone() {
            let args = arguments.as_object_mut().ok_or("arguments cibles : objet attendu")?;
            args.entry("view").or_insert(view);
        }
        if context.target.kind != "backend" || context.target.id.trim().is_empty() {
            return Err(
                "référence cible non disponible : une cible backend explicite est requise".into(),
            );
        }
        let host: Arc<super::DaemonHost> = self.host.upgrade().ok_or("l'hôte s'arrête")?;
        let targets: Vec<_> = host
            .backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .filter(|h| h.id == context.target.id)
            .cloned()
            .collect();
        let target = match targets.as_slice() {
            [target] => target.clone(),
            [] => return Err(format!("contexte cible inconnu : {}", context.target.id)),
            _ => return Err(format!("contexte cible ambigu : {}", context.target.id)),
        };
        if target.manifest == authority.manifest {
            return Err("auto-invocation interdite".into());
        }
        if cancel.is_cancelled() {
            return Err("invocation annulée avant résolution".into());
        }
        let (mut receipt, label) = host.with_backend(&target, |b| {
            let label = b.prepared().manifest.name.clone();
            b.invoke_read(authority, operation, arguments, cancel.clone())
                .map(|r| (r, label))
        })??;
        if cancel.is_cancelled() {
            return Err("invocation annulée".into());
        }
        let child_run = receipt["run"].clone();
        receipt["metadata"]["invocation"] = json!({"target":{"kind":"backend","id":target.id,"label":label},"parent_run":parent_run,"run":child_run});
        Ok(receipt)
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;
    use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};
    use crate::dataflow::node::NodeContext;
    #[test]
    fn contextual_host_refuses_unknown_views_self_calls_and_missing_targets() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let authority = InvocationAuthority {
            manifest: a.h.manifest.clone(),
            policy: crate::backend_code::ToolPolicy {
                read_files: true,
                ..Default::default()
            },
            scope: Default::default(),
        };
        let relay = HostedInvocations {
            host: Arc::downgrade(&a.host),
        };
        let cancel = NodeContext::new().cancellation();
        for context in [
            json!({"target":{"kind":"backend","id":a.h.id}}),
            json!({"target":{"kind":"backend","id":"inconnu"}}),
            json!({"target":{"kind":"space","id":"inconnu"}}),
            json!({"target":{"kind":"backend","id":a.h.id},"view":{"branch":"main"}}),
            json!({"target":{"kind":"backend"}}),
        ] {
            assert!(
                relay
                    .invoke(
                        &authority,
                        &context,
                        "schema",
                        json!({}),
                        "parent",
                        cancel.clone()
                    )
                    .is_err(),
                "{context}"
            );
        }
    }
    /// A host with a space of worktrees and a table of folders in `d`: `arbre` is « le témoin »'s, `archi` another
    /// session's, `sans-proprietaire` a folder on the disk the table does not know; Lucie's tree and a sairen folder exist.
    fn hote_d_espace(d: &std::path::Path) -> (crate::daemon_host::DaemonHost, std::path::PathBuf, String) {
        let espace = d.join("git_workspaces");
        for dossier in ["rag3db", "sairen", "archi", "sans-proprietaire", "arbre"] {
            std::fs::create_dir_all(espace.join(dossier)).unwrap();
        }
        let table = d.join("dossiers-des-sessions.json");
        std::fs::write(&table, json!({"_": "un commentaire", "arbre": "le témoin", "archi": "rag3weaver archi"}).to_string()).unwrap();
        let host = crate::daemon_host::DaemonHost::new();
        host.set_espace(espace.clone(), table);
        host.set_vues_dir(d.join("vues"));
        let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
        let (_, r) = host.handle("POST", "/session", None, corps.as_bytes());
        (host, espace, r["session"].as_str().unwrap().to_string())
    }

    fn refus_d_ouverture(lot: &str, dossier: Option<&str>) -> (String, std::path::PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let (host, espace, s) = hote_d_espace(d.path());
        let r = host.open_view(Some(&s), std::path::Path::new("/nulle-part/backend.json"), lot, dossier);
        assert_eq!(r["ok"], json!(false), "the opening is not refused: {r}");
        let reste: Vec<String> = std::fs::read_dir(&espace).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        (r["error"].as_str().unwrap_or_default().to_string(), espace.join(reste.join(",")))
    }

    /// **Guard 1 of `open_view`: never Lucie's tree** (page 03 of 20 November 2026), even by a lot that climbs into it.
    #[test]
    fn open_view_refuses_lucies_tree() {
        let (e, _) = refus_d_ouverture("x", Some("rag3db"));
        assert!(e.contains("l'arbre de Lucie"), "{e}");
        // The lot follows the date (`<dd-mm-yyyy>-<lot>`): three `..` climb from the session's folder into the space.
        let (e, _) = refus_d_ouverture("../../../rag3db/x", Some("arbre"));
        assert!(e.contains("l'arbre de Lucie"), "a lot that climbs into Lucie's tree is not refused as such: {e}");
    }

    /// **Guard 2: never sairen.**
    #[test]
    fn open_view_refuses_sairen() {
        let (e, _) = refus_d_ouverture("x", Some("sairen"));
        assert!(e.contains("sairen"), "{e}");
    }

    /// **Guard 3: never another session's folder**, nor a folder on the disk the table does not know.
    #[test]
    fn open_view_refuses_another_sessions_folder() {
        let (e, _) = refus_d_ouverture("x", Some("archi"));
        assert!(e.contains("rag3weaver archi"), "the refusal does not say whose folder it is: {e}");
        let (e, _) = refus_d_ouverture("x", Some("sans-proprietaire"));
        assert!(e.contains("sans propriétaire"), "a folder the table does not know is guessed: {e}");
    }

    /// **Guard 4: never outside the session's folder**: a lot that leaves it, or one with characters a name does not take.
    #[test]
    fn open_view_refuses_a_path_outside_the_sessions_folder() {
        let (e, _) = refus_d_ouverture("../banc/x", Some("arbre"));
        assert!(e.contains("hors du dossier"), "{e}");
        let (e, _) = refus_d_ouverture("a b", Some("arbre"));
        assert!(e.contains("hors du dossier"), "{e}");
    }

    /// **Guard 5: never a path already taken.**
    #[test]
    fn open_view_refuses_a_path_already_taken() {
        let d = tempfile::tempdir().unwrap();
        let (host, espace, s) = hote_d_espace(d.path());
        let date = String::from_utf8(std::process::Command::new("date").arg("+%d-%m-%Y").output().unwrap().stdout).unwrap();
        std::fs::create_dir_all(espace.join("arbre").join(format!("{}-pris", date.trim()))).unwrap();
        let r = host.open_view(Some(&s), std::path::Path::new("/nulle-part/backend.json"), "pris", None);
        assert!(r["ok"] == json!(false) && r["error"].as_str().unwrap_or_default().contains("déjà pris"), "{r}");
    }

    /// **`open_view` opens** (O1): under the session's folder, from the table (no `session_dir`), on a new branch
    /// `<folder>/<lot>`, noted in the session's state, without touching anything else of the space.
    #[test]
    fn open_view_opens_a_worktree_under_the_sessions_folder() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let manifeste = roots.manifest(&name);
        let (_h, espace, _) = hote_d_espace(&d.path().join("e"));
        a.host.set_espace(espace.clone(), d.path().join("e/dossiers-des-sessions.json"));
        a.host.set_vues_dir(d.path().join("e/vues"));
        let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
        let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
        let s = r["session"].as_str().unwrap().to_string();
        let r = a.host.open_view(Some(&s), &manifeste, "essai", None);
        assert_eq!(r["ok"], json!(true), "{r}");
        let chemin = std::path::PathBuf::from(r["path"].as_str().unwrap());
        assert!(chemin.starts_with(espace.join("arbre")) && chemin.join(".git").exists(), "the worktree is not under the session's folder: {r}");
        assert_eq!(r["branch"], json!("arbre/essai"), "{r}");
        assert!(r["name"].as_str().is_some_and(|n| n.starts_with("arbre/") && n.ends_with("-essai")), "{r}");
        let branche = std::process::Command::new("git").arg("-C").arg(&chemin).args(["rev-parse", "--abbrev-ref", "HEAD"]).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&branche.stdout).trim(), "arbre/essai");
        let etat = std::fs::read_to_string(d.path().join("e/vues/S1.json")).expect("the opened view is not noted");
        assert!(etat.contains(r["name"].as_str().unwrap()), "{etat}");
    }

    /// **`open_view` makes its view active on a base without `tracked`** (page 03 of 20 November 2026, O2, Lucie's rule of
    /// 18 h 47): no ref is followed; the view opened is a work layer on its fork, among the session's views, active, and a
    /// call without `view` reads under it.
    #[test]
    fn open_view_makes_its_view_active_without_a_followed_ref() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let manifeste = roots.manifest(&name);
        let (_h, espace, _) = hote_d_espace(&d.path().join("e"));
        a.host.set_espace(espace, d.path().join("e/dossiers-des-sessions.json"));
        a.host.set_vues_dir(d.path().join("e/vues"));
        let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
        let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
        let s = r["session"].as_str().unwrap().to_string();
        let r = a.host.open_view(Some(&s), &manifeste, "fourche", None);
        assert_eq!(r["ok"], json!(true), "{r}");
        let chemin = std::path::PathBuf::from(r["path"].as_str().unwrap()).canonicalize().unwrap();
        let couche = format!("worktree:{}", chemin.display());
        assert_eq!(r["layer"], json!(couche), "the view opened has no layer on a base without tracked: {r}");
        let v = a.host.views(Some(&s), &manifeste);
        assert!(v["views"].as_array().is_some_and(|l| l.iter().any(|x| x["name"] == json!(couche))), "the view opened is not among the views: {v}");
        assert_eq!(v["active"], json!({"name": couche, "source": "session"}), "the view opened is not active: {v}");
        let lu = a.host.call(Some(&s), &manifeste, "usages", json!({"name": "un"}));
        assert_eq!((&lu["ok"], &lu["result"]["view"]["source"]), (&json!(true), &json!("session")), "a call without view does not read under the view opened: {lu}");
    }

    /// **`open_view` answers at once, and its view says « index pas prêt » until its layer is written** (page 03 of
    /// 20 November 2026, O2 in the background): the writing held by the witness's gate, `open_view` answers
    /// `indexing: true`, and a read under the view answers not ready at once; the gate opened and the writing done, the
    /// read reads under the view.
    #[test]
    fn open_view_answers_at_once_and_its_view_is_not_ready_until_written() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let manifeste = roots.manifest(&name);
        let (_h, espace, _) = hote_d_espace(&d.path().join("e"));
        a.host.set_espace(espace, d.path().join("e/dossiers-des-sessions.json"));
        a.host.set_vues_dir(d.path().join("e/vues"));
        let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
        let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
        let s = r["session"].as_str().unwrap().to_string();
        let porte = a.host.retenir_les_ecritures_de_vues();
        let r = a.host.open_view(Some(&s), &manifeste, "fond", None);
        assert_eq!((&r["ok"], &r["indexing"]), (&json!(true), &json!(true)), "open_view waits for its layer: {r}");
        let lu = a.host.call(Some(&s), &manifeste, "usages", json!({"name": "un"}));
        assert_eq!(lu["result"]["index_state"]["ready"], json!(false), "a read under a view being written does not say it is not ready: {lu}");
        assert!(lu["result"]["result"].as_str().is_some_and(|t| t.contains("index pas prêt")), "{lu}");
        porte.relacher();
        assert!(a.host.attendre_l_ecriture_des_vues(std::time::Duration::from_secs(120)), "the view's layer is not written");
        let lu = a.host.call(Some(&s), &manifeste, "usages", json!({"name": "un"}));
        assert_eq!((&lu["ok"], &lu["result"]["view"]["source"]), (&json!(true), &json!("session")), "the view written is not read: {lu}");
    }

    /// A view opened for « le témoin » (session S1) on the atelier's backend, its layer written: the atelier, the
    /// manifest, the session, the view's name and path, and the folder of the test.
    struct VueOuverte {
        a: crate::daemon_host::atelier_fichiers::tests::Atelier,
        manifeste: std::path::PathBuf,
        s: String,
        nom: String,
        chemin: std::path::PathBuf,
        couche: String,
        _d: tempfile::TempDir,
    }

    fn une_vue_ouverte(lot: &str) -> VueOuverte {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let manifeste = roots.manifest(&name);
        let (_h, espace, _) = hote_d_espace(&d.path().join("e"));
        a.host.set_espace(espace, d.path().join("e/dossiers-des-sessions.json"));
        a.host.set_vues_dir(d.path().join("e/vues"));
        let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
        let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
        let s = r["session"].as_str().unwrap().to_string();
        let r = a.host.open_view(Some(&s), &manifeste, lot, None);
        assert_eq!(r["ok"], json!(true), "{r}");
        assert!(a.host.attendre_l_ecriture_des_vues(std::time::Duration::from_secs(120)), "the view's layer is not written");
        let (nom, chemin) = (r["name"].as_str().unwrap().to_string(), std::path::PathBuf::from(r["path"].as_str().unwrap()));
        let couche = r["layer"].as_str().expect("the view's layer").to_string();
        VueOuverte { a, manifeste, s, nom, chemin, couche, _d: d }
    }

    /// **`close_view` refuses a view the host did not open for the session** (O3, page 03 of 20 November 2026), Lucie's
    /// tree among them.
    #[test]
    fn close_view_refuses_a_view_it_did_not_open() {
        let v = une_vue_ouverte("a");
        for autre in ["arbre/20-11-2026-jamais", "rag3db"] {
            let r = v.a.host.close_view(Some(&v.s), &v.manifeste, autre, false);
            assert!(r["ok"] == json!(false) && r["error"].as_str().unwrap_or_default().contains("pas ouverte"), "{autre}: {r}");
        }
        assert!(v.chemin.exists());
    }

    /// **`close_view` refuses a dirty tree.**
    #[test]
    fn close_view_refuses_a_dirty_tree() {
        let v = une_vue_ouverte("b");
        std::fs::write(v.chemin.join("sale.rs"), "pub fn sale() {}\n").unwrap();
        let r = v.a.host.close_view(Some(&v.s), &v.manifeste, &v.nom, false);
        assert!(r["ok"] == json!(false) && r["error"].as_str().unwrap_or_default().contains("sale"), "{r}");
        assert!(v.chemin.exists());
    }

    /// **`close_view` refuses commits no remote branch holds.**
    #[test]
    fn close_view_refuses_commits_not_pushed() {
        let v = une_vue_ouverte("c");
        std::fs::write(v.chemin.join("neuf.rs"), "pub fn neuf() {}\n").unwrap();
        let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&v.chemin).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args).status().unwrap().success());
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "local"]);
        let r = v.a.host.close_view(Some(&v.s), &v.manifeste, &v.nom, false);
        assert!(r["ok"] == json!(false) && r["error"].as_str().unwrap_or_default().contains("non poussé"), "{r}");
        assert!(v.chemin.exists());
    }

    /// **`close_view` refuses a tree a process works in.**
    #[test]
    fn close_view_refuses_a_tree_a_process_works_in() {
        let v = une_vue_ouverte("d");
        let mut dedans = std::process::Command::new("sleep").arg("60").current_dir(&v.chemin).spawn().unwrap();
        let r = v.a.host.close_view(Some(&v.s), &v.manifeste, &v.nom, false);
        let _ = dedans.kill();
        let _ = dedans.wait();
        assert!(r["ok"] == json!(false) && r["error"].as_str().unwrap_or_default().contains("processus"), "{r}");
        assert!(v.chemin.exists());
    }

    /// **`close_view` closes a clean view**: the worktree and its target removed, the view forgotten, the session's active
    /// view back to the backend.
    #[test]
    fn close_view_removes_a_clean_tree_and_its_target() {
        let v = une_vue_ouverte("e");
        let etat: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(v._d.path().join("e/vues/S1.json")).unwrap()).unwrap();
        let target = std::path::PathBuf::from(etat["ouvertes"][&v.nom]["target"].as_str().unwrap());
        std::fs::create_dir_all(target.join("debug")).unwrap();
        let r = v.a.host.close_view(Some(&v.s), &v.manifeste, &v.nom, false);
        assert_eq!(r["ok"], json!(true), "{r}");
        assert!(!v.chemin.exists(), "the worktree is still there");
        assert!(!target.exists(), "the target is still there");
        assert_eq!(v.a.host.views(Some(&v.s), &v.manifeste)["active"], json!({"name": "backend", "source": "défaut"}), "the session still reads the closed view");
        let etat = std::fs::read_to_string(v._d.path().join("e/vues/S1.json")).unwrap();
        assert!(!etat.contains(&v.nom), "the closed view is still noted: {etat}");
        // Closed by hand, not landed: the layer ends, and a read under it is refused saying so (c7, 20 November).
        let lu = v.a.host.call(Some(&v.s), &v.manifeste, "usages", json!({"name": "un", "view": {"layer": v.couche}}));
        assert!(lu["ok"] == json!(false) && lu.to_string().contains("couche finie"), "the closed view's layer is not ended: {lu}");
    }

    /// The generations of a view's layer, by its name, on the atelier's backend.
    fn generations_de(v: &VueOuverte) -> i64 {
        let hosted = v.a.host.hosted(&v.manifeste).unwrap();
        v.a.host
            .with_backend(&hosted, |b| {
                let racine = b.prepared().workspace_root().unwrap();
                let repo = crate::suivies::repo_id(&racine, b.prepared().manifest.tracked.as_ref()).unwrap().unwrap();
                let cat = b.catalog.lock().unwrap();
                let lire = |k: &str| cat.read_meta_key(k).map_err(|e| e.to_string());
                let prefixe = format!("couche_nom:{repo}:{}", v.couche);
                let id = lire(&prefixe).unwrap().expect("the layer's id");
                let lu = cat
                    .execute_raw_with_params(
                        &format!("MATCH (g:{}) WHERE g.couche = $c RETURN count(g)", crate::generations::TABLE),
                        &[crate::connection::QueryParam::new("c", crate::connection::CypherValue::String(id))],
                    )
                    .unwrap();
                lu.rows[0][0].as_i64().unwrap_or(-1)
            })
            .unwrap()
    }

    /// **The views of the landed lots close on their machine** (O4, page 03 of 20 November 2026): one machine, its
    /// `landed.tsv` read as a file; the view named in the 6th column is closed, its generations purged.
    #[test]
    fn landed_views_close_on_their_machine() {
        let v = une_vue_ouverte("f");
        assert!(generations_de(&v) > 0, "the view opened has no generation");
        let journal = v._d.path().join("landed.tsv");
        std::fs::write(&journal, format!("autre\th\th\t-\t-\t-\nle-lot\th1\th2\ttemoin\tlib\tposte-a:{}\n", v.nom)).unwrap();
        v.a.host.set_atterrissages("poste-a", crate::daemon_host::VoieDesAtterrissages::Fichier(journal));
        let r = v.a.host.fermer_les_vues_atterries();
        assert_eq!(r["closed"], json!([v.nom]), "the landed view is not closed: {r}");
        assert!(!v.chemin.exists(), "the landed view's worktree is still there");
        assert_eq!(generations_de(&v), 0, "the landed view's generations are not purged");
    }

    /// **Each machine closes its own views only** (O4, N machines): the landings read by a declared command; a host of
    /// another machine closes nothing, the host of the view's machine closes it.
    #[test]
    fn landed_views_close_only_on_their_own_machine() {
        let v = une_vue_ouverte("g");
        let journal = v._d.path().join("landed.tsv");
        std::fs::write(&journal, format!("le-lot\th1\th2\ttemoin\tlib\tposte-a:{}\n", v.nom)).unwrap();
        let voie = crate::daemon_host::VoieDesAtterrissages::Commande(vec!["cat".into(), journal.to_string_lossy().to_string()]);
        v.a.host.set_atterrissages("poste-b", voie.clone());
        let r = v.a.host.fermer_les_vues_atterries();
        assert_eq!(r["closed"], json!([]), "a host closes another machine's view: {r}");
        assert!(v.chemin.exists());
        v.a.host.set_atterrissages("poste-a", voie);
        let r = v.a.host.fermer_les_vues_atterries();
        assert_eq!(r["closed"], json!([v.nom]), "the view's machine does not close it: {r}");
    }

    /// **`close_view` purges on demand** (O4): by hand, a view's generations stay; with `purge`, they go.
    #[test]
    fn close_view_purges_on_demand() {
        let v = une_vue_ouverte("h");
        let r = v.a.host.close_view(Some(&v.s), &v.manifeste, &v.nom, true);
        assert_eq!(r["ok"], json!(true), "{r}");
        assert_eq!(generations_de(&v), 0, "close_view with purge leaves the generations");
    }

    /// **The IDE shows the machine at a glance** (Lucie, 20 November 2026: « gestion de tous les agents actifs ou non,
    /// savoir tous les dossiers sur le PC ouverts dans rag3weaver »; scenario 1 of the night, tickets
    /// `2026-11-21-l-ide-ne-montre-pas-les-agents-du-poste` and `…-les-dossiers-du-poste`). From the welcome page, the
    /// page « Poste »: the agents (a live session by its Claude name; a session of the table of the folders without a live
    /// session, inactive), the folders of the machine with their session and worktrees, the views opened (the active one
    /// said), the repositories attached.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn the_ide_shows_the_machine_at_a_glance() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        let Some(binaire) = navigateur_ou_saut("the_ide_shows_the_machine_at_a_glance") else {
            return;
        };
        let v = une_vue_ouverte("p");
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let racine = format!("http://{}", ecoute.local_addr().unwrap());
        let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
        let hote = v.a.host.clone();
        let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(hote, Some("j".to_string())));

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.attendre("document.querySelector('a[href=\"/ide/poste\"]')", "a link to the page « Poste » on the welcome page");
        n.cliquer_pour_naviguer("document.querySelector('a[href=\"/ide/poste\"]')");
        n.attendre("document.querySelector('#agents') && document.querySelector('#dossiers-du-poste')", "the page « Poste »");
        let texte = |n: &mut Navigateur, id: &str| -> String {
            n.evaluer(&format!("document.querySelector('#{id}')?.innerText ?? ''")).as_str().unwrap_or_default().to_string()
        };
        n.attendre("(document.querySelector('#agents')?.innerText ?? '').includes('le témoin')", "the live agent by its Claude name");
        let agents = texte(&mut n, "agents");
        let archi = agents.lines().find(|l| l.contains("rag3weaver archi")).unwrap_or_default().to_string();
        assert!(archi.contains("inactif"), "an agent of the table without a live session, said inactive: {agents}");
        let dossiers = texte(&mut n, "dossiers-du-poste");
        let nom_du_worktree = v.chemin.file_name().unwrap().to_string_lossy().to_string();
        assert!(dossiers.contains("arbre") && dossiers.contains("le témoin") && dossiers.contains(&nom_du_worktree), "the folder, its session, its worktree: {dossiers}");
        let vues = texte(&mut n, "vues-ouvertes");
        let ligne = vues.lines().find(|l| l.contains(&v.nom)).unwrap_or_default().to_string();
        assert!(ligne.contains("active"), "the view opened, said active: {vues}");
        let depots = texte(&mut n, "depots-du-poste");
        assert!(depots.contains("rag3db-essai"), "the repository attached: {depots}");

        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **The machine at a glance** (21 November 2026, recherche's scenario 1; keys in English, as recherche asked): after a
    /// view is opened, `GET /poste` says
    /// the session with its Claude name, the table of the folders with the worktree found under the session's, the view
    /// opened and active, and the repository attached.
    #[test]
    fn the_machine_at_a_glance_says_its_sessions_folders_views_and_repositories() {
        let v = une_vue_ouverte("p");
        let (code, r) = v.a.host.handle("GET", "/poste", None, &[]);
        assert_eq!(code, 200, "{r}");
        let sessions = r["sessions"].as_array().cloned().unwrap_or_default();
        assert!(sessions.iter().any(|x| x["claude"]["name"] == json!("le témoin")), "the session and its Claude name are not said: {r}");
        let folders = r["folders"].as_array().cloned().unwrap_or_default();
        let arbre = folders.iter().find(|x| x["folder"] == json!("arbre")).cloned().unwrap_or_default();
        assert_eq!(arbre["session"], json!("le témoin"), "the table of the folders is not said: {r}");
        let reel = v.chemin.canonicalize().unwrap();
        assert!(arbre["worktrees"].as_array().is_some_and(|l| l.iter().any(|w| w.as_str().map(std::path::PathBuf::from).and_then(|p| p.canonicalize().ok()) == Some(reel.clone()))), "the worktree under the session's folder is not said: {arbre}");
        let vues = r["open_views"].as_array().cloned().unwrap_or_default();
        assert!(r["machine"].as_str().is_some_and(|m| !m.is_empty()), "the machine is not named: {r}");
        assert!(vues.iter().any(|x| x["name"] == json!(v.nom) && x["active"] == json!(true)), "the view opened and active is not said: {r}");
        let depots = r["repositories"].as_array().cloned().unwrap_or_default();
        assert!(depots.iter().any(|x| x["repo"] == json!("rag3db-essai")), "the repository attached is not said: {r}");
    }

    /// **`views` and `use_view` follow the Claude Code session** (page 02 of 20 November 2026, lot 2): two thin clients
    /// of one session (one `sessionId`) see the same active view, another session does not; an unknown name is refused,
    /// listing the known views; a host restarted on the same state finds the view again.
    #[test]
    fn views_and_use_view_follow_the_claude_session() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let manifeste = roots.manifest(&name);
        let etat = d.path().join("vues");
        a.host.set_vues_dir(etat.clone());
        let ouvrir = |host: &crate::daemon_host::DaemonHost, sid: &str| -> String {
            let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": sid, "name": "le témoin"}}).to_string();
            let (code, r) = host.handle("POST", "/session", None, corps.as_bytes());
            assert_eq!(code, 200, "{r}");
            r["session"].as_str().unwrap().to_string()
        };
        let (s1, s2, s3) = (ouvrir(&a.host, "S1"), ouvrir(&a.host, "S1"), ouvrir(&a.host, "S2"));
        let v = a.host.views(Some(&s1), &manifeste);
        assert!(v["views"].as_array().is_some_and(|l| l.iter().any(|x| x["name"] == "backend")), "the backend is not among the views: {v}");
        assert_eq!(v["active"], json!({"name": "backend", "source": "défaut"}), "{v}");
        let u = a.host.use_view(Some(&s1), &manifeste, "backend");
        assert_eq!(u["ok"], json!(true), "use_view refuses a known view: {u}");
        assert_eq!(a.host.views(Some(&s2), &manifeste)["active"], json!({"name": "backend", "source": "session"}), "a second thin client of the same session does not see its view");
        assert_eq!(a.host.views(Some(&s3), &manifeste)["active"]["source"], json!("défaut"), "another session takes this one's view");
        let refus = a.host.use_view(Some(&s1), &manifeste, "inconnue");
        assert!(refus["ok"] == json!(false) && refus.to_string().contains("inconnue") && refus.to_string().contains("backend"), "an unknown view is not refused listing the known ones: {refus}");
        let relance = crate::daemon_host::DaemonHost::new();
        relance.set_vues_dir(etat);
        let s4 = ouvrir(&relance, "S1");
        assert_eq!(relance.session_default_view(&s4), Some(json!({"layer": "backend"})), "a restarted host does not find the session's view");
    }

    /// **A call without `view` reads the view of its session** (V1 of the page
    /// `docs/arbre-principal/20-11-2026/01-cadrage-la-vue-par-defaut-de-la-session.md`): the view read is the call's,
    /// then the session's, then the backend's, and every answer of a reading tool says it and its source. A session's
    /// view unknown to the backend is refused at the call, naming it; a malformed one at its declaration.
    #[test]
    fn a_call_without_view_reads_the_view_of_its_session() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let manifeste = roots.manifest(&name);
        let avec = a.host.open_session("session avec vue", None, Vec::new(), None);
        let sans = a.host.open_session("session sans vue", None, Vec::new(), None);
        a.host.set_session_view(&avec, Some(json!({"layer": "backend"}))).expect("a session's view is refused");
        // The host wraps the backend's answer: `{ok, result: {…, view}}`.
        let lu = a.host.call(Some(&avec), &manifeste, "usages", json!({"name": "un"}));
        assert_eq!(lu["ok"], json!(true), "{lu}");
        let vue = &lu["result"]["view"];
        assert_eq!((&vue["layer"], &vue["source"]), (&json!("backend"), &json!("session")), "the view of the session is not read, or not said: {lu}");
        assert!(vue["rank"].is_i64(), "the rank read under the session's view is not said: {lu}");
        let explicite = a.host.call(Some(&avec), &manifeste, "usages", json!({"name": "un", "view": {"layer": "backend"}}));
        assert_eq!(explicite["result"]["view"]["source"], json!("appel"), "a call's view does not win over the session's: {explicite}");
        let defaut = a.host.call(Some(&sans), &manifeste, "usages", json!({"name": "un"}));
        assert_eq!(defaut["result"]["view"], json!({"layer": "backend", "source": "défaut"}), "a call without view nor session's view does not say it reads the backend: {defaut}");
        a.host.set_session_view(&avec, Some(json!({"layer": "inconnue"}))).unwrap();
        let refus = a.host.call(Some(&avec), &manifeste, "usages", json!({"name": "un"}));
        assert!(refus["ok"] == json!(false) && refus.to_string().contains("inconnue"), "a session's view unknown to the backend is read: {refus}");
        let malformee = a.host.set_session_view(&avec, Some(json!({"branch": "main"}))).expect_err("a malformed session's view is posed");
        assert!(malformee.contains("vue par défaut"), "{malformee}");
    }

    /// **The current view by the context** (2b-C, ticket « les vues des worktrees des sessions », point 2): an
    /// invocation whose context carries `view`, without `view` in its arguments, reads under that view, and its answer
    /// says the view read (layer, rank). A view the target does not know is refused naming it. An explicit `view` in
    /// the arguments wins over the context's.
    #[test]
    fn an_invocation_reads_under_the_view_of_its_context() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let target = a.host.hosted(&roots.manifest(&name)).unwrap();
        let authority = InvocationAuthority {
            manifest: a.h.manifest.clone(),
            policy: crate::backend_code::ToolPolicy { read_files: true, ..Default::default() },
            scope: Default::default(),
        };
        let relay = HostedInvocations { host: Arc::downgrade(&a.host) };
        let invoke = |context: Value, arguments: Value| {
            relay.invoke(&authority, &context, "usages", arguments, "parent", NodeContext::new().cancellation())
        };
        let lu = invoke(json!({"target":{"kind":"backend","id":target.id},"view":{"layer":"backend"}}), json!({"name":"un"}))
            .expect("an invocation under the view of its context is refused");
        assert_eq!(lu["view"]["layer"], "backend", "the answer does not say the view read: {lu}");
        assert!(lu["view"]["rank"].is_i64(), "the answer does not say the rank read: {lu}");
        let refus = invoke(json!({"target":{"kind":"backend","id":target.id},"view":{"layer":"inconnue"}}), json!({"name":"un"}))
            .expect_err("a view unknown to the target is read");
        assert!(refus.contains("inconnue"), "the refusal does not name the view: {refus}");
        let explicite = invoke(
            json!({"target":{"kind":"backend","id":target.id},"view":{"layer":"inconnue"}}),
            json!({"name":"un","view":{"layer":"backend"}}),
        )
        .expect("an explicit view does not win over the context's");
        assert_eq!(explicite["view"]["layer"], "backend", "{explicite}");
    }

    #[test]
    fn contextual_host_reads_the_selected_depot_and_refuses_an_elevated_operation() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&d.path().join("B"));
        a.appeler("attach_worktree", json!({"path":depot})).unwrap();
        attendre_l_index(&a, &depot);
        let roots = a.host.racines_de_code().unwrap();
        let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let target = a.host.hosted(&roots.manifest(&name)).unwrap();
        let context = json!({"target":{"kind":"backend","id":target.id}});
        let inventory = a.appeler("list_worktrees", json!({})).unwrap();
        assert!(inventory["result"]
            .as_str()
            .unwrap_or("")
            .contains(&target.id));
        let result = a
            .appeler("outline", json!({"context":context,"path":"src/lib.rs"}))
            .unwrap();
        assert_eq!(result["metadata"]["invocation"]["target"]["id"], target.id);
        assert!(
            result["presentation"].as_str().unwrap_or("").contains("un"),
            "{result}"
        );
        let authority = InvocationAuthority {
            manifest: a.h.manifest.clone(),
            policy: crate::backend_code::ToolPolicy {
                read_files: true,
                ..Default::default()
            },
            scope: Default::default(),
        };
        let relay = HostedInvocations {
            host: Arc::downgrade(&a.host),
        };
        let error = relay
            .invoke(
                &authority,
                &context,
                "edit_file",
                json!({"path":"src/lib.rs","content":"effacé"}),
                "parent",
                NodeContext::new().cancellation(),
            )
            .unwrap_err();
        assert!(error.contains("capacités"), "{error}");
        assert!(std::fs::read_to_string(depot.join("src/lib.rs"))
            .unwrap()
            .contains("pub fn un"));
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&target.manifest).unwrap()).unwrap();
        manifest["tools"]["bounce"] = json!({"graph":"bounce.mmd","policy":{"read_files":true}});
        std::fs::write(target.manifest.parent().unwrap().join("bounce.mmd"),
            format!("%% tool: bounce\n%% description: Un relais imbriqué refusé.\n%% result: invoke.results\ngraph LR\n invoke[\"InvokeToolNode(operation='schema', context={{\"target\":{{\"kind\":\"backend\",\"id\":\"{}\"}}}}, arguments={{}})\"]\n", a.h.id)).unwrap();
        std::fs::write(
            &target.manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let session =
            a.host
                .open_session("recharger la déclaration du témoin", None, Vec::new(), None);
        let opened = a.host.open(Some(&session), &target.manifest);
        assert_eq!(opened["ok"], true, "{opened}");
        let error = relay
            .invoke(
                &authority,
                &context,
                "bounce",
                json!({}),
                "parent",
                NodeContext::new().cancellation(),
            )
            .unwrap_err();
        assert!(error.contains("boucles"), "{error}");
        let mut database_only = authority.clone();
        database_only.policy = Default::default();
        let receipt = relay
            .invoke(
                &database_only,
                &context,
                "schema",
                json!({}),
                "parent",
                NodeContext::new().cancellation(),
            )
            .unwrap();
        assert_eq!(receipt["metadata"]["invocation"]["target"]["id"], target.id);
        let error = relay
            .invoke(
                &database_only,
                &context,
                "read_file",
                json!({"path":"src/lib.rs"}),
                "parent",
                NodeContext::new().cancellation(),
            )
            .unwrap_err();
        assert!(error.contains("capacités"), "{error}");
        let mut other_scope = authority.clone();
        other_scope.scope.org = "autre".into();
        assert!(relay
            .invoke(
                &other_scope,
                &context,
                "schema",
                json!({}),
                "parent",
                NodeContext::new().cancellation()
            )
            .is_err());
    }
}
