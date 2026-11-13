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
fn identite(source: &super::ModelSource) -> Result<crate::daemon::embeddings::IdentiteActivation, String> {
    let a = source.activation.as_ref().ok_or("activation absente")?;
    // Les budgets d'attente et chemins de coordination ne changent pas le
    // fournisseur. L'empreinte décrit la recette, elle ne divulgue pas son env.
    let recette = serde_json::to_vec(&(&source.model, source.dimensions, &a.programme, &a.args, &a.env, &a.profil, &a.peripherique))
        .map_err(|e| e.to_string())?;
    Ok(crate::daemon::embeddings::IdentiteActivation {
        profil: a.profil.clone(), peripherique: a.peripherique.clone(),
        recette: blake3::hash(&recette).to_hex().to_string(),
    })
}

#[cfg(feature = "daemon")]
pub fn demander(source: &super::ModelSource) -> Result<crate::serveur::activation::Demande, String> {
    use super::{Capability, Fallback, Provider};
    let a = source.activation.as_ref().ok_or("activation absente")?;
    let attendue = identite(source)?;
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
        .env("RAG3WEAVER_ACTIVATION_IDENTITE", &attendue.recette)
        .journal_dans(&a.journal)
        .attente(std::time::Duration::from_secs(a.budget_activation_secs));
    for (cle, valeur) in &a.env {
        if matches!(cle.as_str(), "RAG3WEAVER_EMBED_MODEL" | "RAG3WEAVER_ACTIVATION_PROFIL" | "RAG3WEAVER_ACTIVATION_PERIPHERIQUE" | "RAG3WEAVER_ACTIVATION_IDENTITE") {
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
    let debut = std::time::Instant::now();
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    let demande = demander(source)?;
    let a = source.activation.as_ref().unwrap();
    let bail = demande.attendre(annule, std::time::Duration::from_secs(a.budget_attente_secs))?;
    let restant = std::time::Duration::from_secs(a.budget_attente_secs).saturating_sub(debut.elapsed());
    if restant.is_zero() { return Err("budget de connexion à l'activation dépassé".into()); }
    let mut client = crate::daemon::DaemonEmbedder::joindre_dans(bail.adresse(), restant).map_err(|e| e.to_string())?;
    let delai_appel = source.timeout_secs.filter(|s| *s > 0).map(std::time::Duration::from_secs)
        .unwrap_or_else(crate::daemon::embeddings::default_embed_timeout);
    client = client.with_timeout(delai_appel);
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    if client.name() != source.model || Some(client.dim()) != source.dimensions {
        return Err(format!("identité d'activation refusée : attendu {}/{:?}, reçu {}/{}", source.model, source.dimensions, client.name(), client.dim()));
    }
    if client.identite().activation.as_ref() != Some(&identite(source)?) {
        return Err("identité d'activation refusée : recette/profil du fournisseur différent ou non attesté".into());
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

#[cfg(all(test, feature = "daemon", unix))]
mod tests {
    use super::*;
    use crate::model_source::{connect_embedder, connect_embedder_annulable, Addresses, ModelSource};
    use std::sync::atomic::AtomicBool;

    struct Fixture { dossier: tempfile::TempDir, source: ModelSource }
    impl Fixture {
        fn nouvelle() -> Self {
            let dossier = tempfile::tempdir().unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let adresse = listener.local_addr().unwrap().to_string();
            let mut source = ModelSource::service("fixture-residence-384");
            source.address = Addresses::parse(&adresse);
            source.dimensions = Some(384);
            source.timeout_secs = Some(1);
            source.activation = Some(Activation {
                programme: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py").into(),
                args: vec!["--journal".into(), dossier.path().join("events.jsonl").display().to_string(),
                    "--generation".into(), "appelant-reel".into()],
                env: Default::default(), profil: "fixture-cpu".into(), peripherique: "cpu".into(),
                coordination: dossier.path().join("coordination"), journal: dossier.path().join("journaux"),
                budget_activation_secs: 5, budget_attente_secs: 5, residence: Residence::Maintenir,
            });
            drop(listener);
            // Même chemin que le manifeste : la déclaration traverse serde.
            let source = serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
            Self { dossier, source }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let journal = self.dossier.path().join("events.jsonl");
            if let Ok(texte) = std::fs::read_to_string(journal) {
                if let Some(pid) = texte.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                    .find_map(|v| v.get("pid").and_then(|n| n.as_i64())) {
                    // PID produit par notre fixture privée, jamais pgrep global.
                    unsafe { libc::kill(pid as i32, libc::SIGTERM); libc::waitpid(pid as i32, std::ptr::null_mut(), 0); }
                }
            }
        }
    }

    #[test]
    fn activation_dense_depuis_declaration_preserve_identite_et_delai() {
        let f = Fixture::nouvelle();
        let (client, origine) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.name(), "fixture-residence-384");
        assert_eq!(client.dim(), 384);
        assert!(client.is_mock());
        assert_eq!(client.timeout(), Some(std::time::Duration::from_secs(1)));
        assert!(origine.remote());
        let vecteurs = client.embed(&["texte synthétique".into()]).unwrap();
        assert_eq!(vecteurs.len(), 1); assert_eq!(vecteurs[0].len(), 384);
        let d = crate::daemon::DaemonEmbedder::joindre(&f.source.address.0[0]).unwrap();
        assert!(d.identite().chauffe.is_none(), "legacy = chauffe inconnue");
    }

    #[test]
    fn activation_dense_refuse_dimension_etrangere() {
        let mut f = Fixture::nouvelle(); f.source.dimensions = Some(768);
        let erreur = connect_embedder(&f.source).err().unwrap();
        assert!(erreur.contains("identité"), "{erreur}");
    }

    #[test]
    fn demande_deja_annulee_ne_demarre_rien() {
        let f = Fixture::nouvelle();
        assert!(connect_embedder_annulable(&f.source, &AtomicBool::new(true)).err().unwrap().contains("annulée"));
        assert!(!f.dossier.path().join("events.jsonl").exists());
    }

    #[test]
    fn profil_etranger_entre_processus_est_refuse() {
        if let Ok(chemin) = std::env::var("CX_ACTIVATION_SOURCE_TEST") {
            let source: ModelSource = serde_json::from_slice(&std::fs::read(chemin).unwrap()).unwrap();
            let erreur = connect_embedder(&source).err().expect("profil étranger refusé");
            assert!(erreur.contains("recette/profil"), "{erreur}");
            return;
        }
        let f = Fixture::nouvelle();
        let (client, _) = connect_embedder(&f.source).unwrap();
        let mut etrangere = f.source.clone();
        etrangere.activation.as_mut().unwrap().profil = "autre-profil".into();
        let chemin = f.dossier.path().join("source-etrangere.json");
        std::fs::write(&chemin, serde_json::to_vec(&etrangere).unwrap()).unwrap();
        let sortie = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "model_source::activation::tests::profil_etranger_entre_processus_est_refuse", "--nocapture"])
            .env("CX_ACTIVATION_SOURCE_TEST", chemin).output().unwrap();
        assert!(sortie.status.success(), "{} {}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
        assert_eq!(client.embed(&["toujours disponible".into()]).unwrap()[0].len(), 384);
        let texte = std::fs::read_to_string(f.dossier.path().join("events.jsonl")).unwrap();
        assert_eq!(texte.lines().filter(|l| l.contains("constructeur_demarre")).count(), 1);
    }

    /// **Une activation déclarée sur le port 0** : le fournisseur se lie
    /// lui-même et annonce l'adresse prise, aucun port n'est réservé puis
    /// relâché (la course du ticket
    /// `docs/embarquement/tickets/2026-11-13-un-test-d-activation-entre-processus-rougit-une-fois-sous-la-lib-entiere.md`).
    /// Une seconde demande, après la première, retrouve ce fournisseur par
    /// l'adresse retenue au dossier de coordination : un seul démarrage.
    #[test]
    fn une_activation_sur_port_nul_annonce_son_adresse_et_la_retrouve() {
        let mut f = Fixture::nouvelle();
        f.source.address = Addresses::parse("127.0.0.1:0");
        let (premier, origine) = connect_embedder(&f.source).unwrap();
        let prise = match origine { super::super::Origin::Service(a) => a, autre => panic!("origine {autre:?}") };
        assert!(!prise.ends_with(":0"), "l'adresse rendue est celle que le fournisseur a prise : {prise}");
        assert_eq!(premier.embed(&["premier".into()]).unwrap()[0].len(), 384);
        let (second, origine) = connect_embedder(&f.source).unwrap();
        assert_eq!(origine, super::super::Origin::Service(prise.clone()), "la seconde demande retrouve le même fournisseur");
        assert_eq!(second.embed(&["second".into()]).unwrap()[0].len(), 384);
        let texte = std::fs::read_to_string(f.dossier.path().join("events.jsonl")).unwrap();
        assert_eq!(texte.lines().filter(|l| l.contains("constructeur_demarre")).count(), 1, "un seul démarrage :\n{texte}");
    }

    #[test]
    fn budget_de_sonde_ne_remplace_pas_delai_appel_par_defaut() {
        let mut f = Fixture::nouvelle(); f.source.timeout_secs = None;
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.timeout(), Some(crate::daemon::embeddings::default_embed_timeout()));
    }
}
