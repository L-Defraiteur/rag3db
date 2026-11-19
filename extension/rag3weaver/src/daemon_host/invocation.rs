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
