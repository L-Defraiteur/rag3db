//! Politique déclarative initiale : activation locale et maintien explicite.
//! Les identités de profil/périphérique qualifient la recette ; elles ne prouvent
//! pas le placement physique des allocations (à qualifier sur le fournisseur).
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub programme: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    pub profil: String,
    pub peripherique: String,
    pub coordination: std::path::PathBuf,
    pub journal: std::path::PathBuf,
    pub budget_activation_secs: u64,
    pub budget_attente_secs: u64,
    pub residence: Residence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Residence { Maintenir }

#[cfg(feature = "daemon")]
pub fn demander(source: &super::ModelSource) -> Result<crate::serveur::activation::Demande, String> {
    use super::{Capability, Fallback, Provider};
    let a = source.activation.as_ref().ok_or("activation absente")?;
    if source.provider != Provider::Service || source.fallback != Fallback::Refuse {
        return Err("activation gérée : service sans repli requis".into());
    }
    let adresses = source.addresses(Capability::Embed, &super::process_env);
    if adresses.len() != 1 || source.dimensions.unwrap_or(0) == 0 {
        return Err("activation gérée : une adresse et une dimension explicites requises".into());
    }
    if a.profil.trim().is_empty() || a.peripherique.trim().is_empty()
        || a.budget_activation_secs == 0 || a.budget_attente_secs == 0 {
        return Err("activation gérée : profil, périphérique et budgets non vides requis".into());
    }
    let mut serveur = crate::daemon::DaemonEmbedder::serveur(&adresses[0], &a.programme)
        .args(a.args.iter().cloned())
        .env("RAG3WEAVER_EMBED_MODEL", &source.model)
        .env("RAG3WEAVER_ACTIVATION_PROFIL", &a.profil)
        .env("RAG3WEAVER_ACTIVATION_PERIPHERIQUE", &a.peripherique)
        .journal_dans(&a.journal)
        .attente(std::time::Duration::from_secs(a.budget_activation_secs));
    for (cle, valeur) in &a.env {
        if matches!(cle.as_str(), "RAG3WEAVER_EMBED_MODEL" | "RAG3WEAVER_ACTIVATION_PROFIL" | "RAG3WEAVER_ACTIVATION_PERIPHERIQUE") {
            return Err(format!("activation : {cle} est déterminé par l'identité déclarée"));
        }
        serveur = serveur.env(cle, valeur);
    }
    crate::serveur::activation::acquerir(&serveur, &a.coordination)
}

#[cfg(feature = "daemon")]
pub(super) fn connecter(source: &super::ModelSource, annule: &std::sync::atomic::AtomicBool)
    -> Result<(Box<dyn crate::embedder::Embedder>, super::Origin), String> {
    use crate::embedder::Embedder;
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    let demande = demander(source)?;
    let a = source.activation.as_ref().unwrap();
    let bail = demande.attendre(annule, std::time::Duration::from_secs(a.budget_attente_secs))?;
    let client = crate::daemon::DaemonEmbedder::joindre(bail.adresse()).map_err(|e| e.to_string())?;
    if client.name() != source.model || Some(client.dim()) != source.dimensions {
        return Err(format!("identité d'activation refusée : attendu {}/{:?}, reçu {}/{}", source.model, source.dimensions, client.name(), client.dim()));
    }
    let origine = super::Origin::Service(bail.adresse().to_string());
    Ok((Box::new(Gere { client, _bail: bail }), origine))
}

#[cfg(feature = "daemon")]
struct Gere { client: crate::daemon::DaemonEmbedder, _bail: crate::serveur::activation::Bail }

#[cfg(feature = "daemon")]
impl crate::embedder::Embedder for Gere {
    fn embed(&self, textes: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> { self.client.embed(textes) }
    fn dim(&self) -> usize { self.client.dim() }
    fn name(&self) -> &str { self.client.name() }
    fn is_mock(&self) -> bool { self.client.is_mock() }
    fn is_absent(&self) -> bool { self.client.is_absent() }
    fn distant(&self) -> bool { self.client.distant() }
    fn budget_conseille(&self) -> Option<(usize, usize)> { self.client.budget_conseille() }
    fn troncatures(&self) -> Option<(usize, usize)> { self.client.troncatures() }
    fn timeout(&self) -> Option<std::time::Duration> { self.client.timeout() }
}
