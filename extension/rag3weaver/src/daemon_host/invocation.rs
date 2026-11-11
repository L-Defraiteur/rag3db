//! Résolution d'une référence explicite ; jamais le premier dépôt disponible.
use std::sync::{Arc, Weak};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::dataflow::invocation_nodes::{InvocationAuthority, InvocationHost};
use crate::dataflow::node::CancelToken;

pub(super) struct HostedInvocations { pub host: Weak<super::DaemonHost> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context { target: Target, #[serde(default)] view: Option<Value> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target { kind: String, id: String }
impl InvocationHost for HostedInvocations {
    fn invoke(&self, authority: &InvocationAuthority, context: &Value, operation: &str,
        arguments: Value, parent_run: &str, cancel: CancelToken) -> Result<Value, String> {
        let context: Context = serde_json::from_value(context.clone()).map_err(|e| format!("contexte invalide : {e}"))?;
        if context.view.is_some() { return Err("vue contextuelle non disponible : aucun repli implicite".into()); }
        if context.target.kind != "backend" || context.target.id.trim().is_empty() {
            return Err("référence cible non disponible : une cible backend explicite est requise".into());
        }
        let host: Arc<super::DaemonHost> = self.host.upgrade().ok_or("l'hôte s'arrête")?;
        let target = host.hosted_by_id(&context.target.id)?;
        if target.manifest == authority.manifest { return Err("auto-invocation interdite".into()); }
        if cancel.is_cancelled() { return Err("invocation annulée avant résolution".into()); }
        let mut receipt = host.with_backend(&target, |b| b.invoke_read(authority, operation, arguments, cancel.clone()))??;
        if cancel.is_cancelled() { return Err("invocation annulée".into()); }
        let label = host.with_backend(&target, |b| b.prepared().manifest.name.clone())?;
        let child_run = receipt["run"].clone();
        receipt["metadata"]["invocation"] = json!({"target":{"kind":"backend","id":target.id,"label":label},"parent_run":parent_run,"run":child_run});
        Ok(receipt)
    }
}
