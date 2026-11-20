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
