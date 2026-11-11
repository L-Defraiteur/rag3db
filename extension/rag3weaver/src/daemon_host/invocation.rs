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
        if context.view.is_some() {
            return Err("vue contextuelle non disponible : aucun repli implicite".into());
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
