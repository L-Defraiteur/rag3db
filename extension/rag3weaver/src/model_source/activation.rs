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
    /// **Activation distante** : la cible ssh du poste qui porte le
    /// fournisseur (`luciepc`). Absent : le fournisseur est lancé ici.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hote: Option<String>,
    /// L'adresse du fournisseur sur le poste distant (port fixe) ; l'adresse
    /// déclarée de la source est le bout local du tunnel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adresse_distante: Option<String>,
    /// La commande ssh à employer (défaut `ssh`) ; les témoins y mettent un faux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh: Option<Vec<String>>,
}

/// Le préfixe des unités que l'activation distante crée : jamais celui des
/// services posés à la main (`rag3weaver-embeddings@`).
pub const PREFIXE_UNITE: &str = "rag3weaver-fournisseur-";

/// La politique de résidence d'un fournisseur lancé par l'activation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Residence {
    /// Garder le fournisseur tant qu'il vit.
    Maintenir,
    /// Le décharger après `inactivite_secs` sans bail ni calcul : il le
    /// déclare dans son identité (`eviction`) et sort de lui-même.
    ADemande { inactivite_secs: u64 },
}

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
        .journal_dans(&a.journal)
        .attente(std::time::Duration::from_secs(a.budget_activation_secs));
    for (cle, valeur) in variables(source, &attendue)? {
        serveur = serveur.env(cle, valeur);
    }
    crate::serveur::activation::acquerir(&serveur, &a.coordination)
}

/// L'environnement d'un fournisseur lancé par l'activation, ici ou à
/// distance : son modèle, son identité attestée, sa politique de résidence,
/// puis l'environnement déclaré, qui ne peut pas les redéfinir.
#[cfg(feature = "daemon")]
fn variables(source: &super::ModelSource, attendue: &crate::daemon::embeddings::IdentiteActivation) -> Result<Vec<(String, String)>, String> {
    let a = source.activation.as_ref().ok_or("activation absente")?;
    let mut v = vec![
        ("RAG3WEAVER_EMBED_MODEL".to_string(), source.model.clone()),
        ("RAG3WEAVER_ACTIVATION_PROFIL".to_string(), a.profil.clone()),
        ("RAG3WEAVER_ACTIVATION_PERIPHERIQUE".to_string(), a.peripherique.clone()),
        ("RAG3WEAVER_ACTIVATION_IDENTITE".to_string(), attendue.recette.clone()),
    ];
    if let Residence::ADemande { inactivite_secs } = a.residence {
        if inactivite_secs == 0 {
            return Err("activation : un délai d'inactivité nul déchargerait aussitôt".into());
        }
        v.push((crate::daemon::embeddings::EVICTION_VARIABLE.to_string(), inactivite_secs.to_string()));
    }
    for (cle, valeur) in &a.env {
        if matches!(cle.as_str(), "RAG3WEAVER_EMBED_MODEL" | "RAG3WEAVER_ACTIVATION_PROFIL" | "RAG3WEAVER_ACTIVATION_PERIPHERIQUE" | "RAG3WEAVER_ACTIVATION_IDENTITE" | crate::daemon::embeddings::EVICTION_VARIABLE) {
            return Err(format!("activation : {cle} est déterminé par l'identité déclarée"));
        }
        v.push((cle.clone(), valeur.clone()));
    }
    Ok(v)
}

#[cfg(feature = "daemon")]
pub(super) fn connecter(source: &super::ModelSource, annule: &std::sync::atomic::AtomicBool)
    -> Result<(Box<dyn crate::embedder::Embedder>, super::Origin), String> {
    let debut = std::time::Instant::now();
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    if source.activation.as_ref().is_some_and(|a| a.hote.is_some()) {
        return connecter_distant(source, annule);
    }
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
    verifier_l_identite(source, &client)?;
    // Un bail d'usage chez le fournisseur : il compte cet usager tant que le
    // client vit (première étape de la résidence ; rien n'est déchargé). Un
    // fournisseur d'avant les baux ne compte pas : `None`, pas une erreur.
    let usage = client.prendre_bail(&format!("activation:{}:{}", std::process::id(), source.model), std::time::Duration::from_secs(60))
        .map_err(|e| format!("bail d'usage : {e}"))?;
    let origine = super::Origin::Service(bail.adresse().to_string());
    Ok((Box::new(Gere { client, _usage: usage, _tenue: Box::new(bail) }), origine))
}

/// Le fournisseur trouvé est-il celui déclaré : modèle, dimension, et
/// l'attestation de recette/profil/périphérique.
#[cfg(feature = "daemon")]
fn verifier_l_identite(source: &super::ModelSource, client: &crate::daemon::DaemonEmbedder) -> Result<(), String> {
    use crate::embedder::Embedder;
    if client.name() != source.model || Some(client.dim()) != source.dimensions {
        return Err(format!("identité d'activation refusée : attendu {}/{:?}, reçu {}/{}", source.model, source.dimensions, client.name(), client.dim()));
    }
    if client.identite().activation.as_ref() != Some(&identite(source)?) {
        return Err("identité d'activation refusée : recette/profil du fournisseur différent ou non attesté".into());
    }
    Ok(())
}

/// Le tunnel ssh d'une activation distante, tenu tant que le client vit.
#[cfg(feature = "daemon")]
struct Tunnel(std::process::Child);
#[cfg(feature = "daemon")]
impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// **L'activation distante** (règles : `docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md`).
/// Un tunnel `ssh -N -L <adresse locale>:<adresse distante>` tenu par le
/// client ; par lui, la sonde de l'identité : le fournisseur déclaré déjà là,
/// on s'y attache ; un autre, refus ; personne, on le lance par
/// `ssh <hôte> systemd-run --user --unit=rag3weaver-fournisseur-<modèle>-<port> --collect --property=Restart=on-failure`.
/// ssh en `BatchMode` : un échec se refuse net, sans nouvel essai ni repli.
#[cfg(feature = "daemon")]
fn connecter_distant(source: &super::ModelSource, annule: &std::sync::atomic::AtomicBool)
    -> Result<(Box<dyn crate::embedder::Embedder>, super::Origin), String> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    use super::{Capability, Fallback, Provider};
    let a = source.activation.as_ref().ok_or("activation absente")?;
    let hote = a.hote.as_deref().unwrap_or_default();
    if hote.is_empty() || !hote.chars().all(|c| c.is_ascii_alphanumeric() || "-._@".contains(c)) {
        return Err(format!("activation distante : hôte ssh invalide « {hote} »"));
    }
    if source.provider != Provider::Service || source.fallback != Fallback::Refuse {
        return Err("activation gérée : service sans repli requis".into());
    }
    let adresses = source.addresses(Capability::Embed, &super::process_env);
    let distante = a.adresse_distante.clone().ok_or("activation distante : adresse_distante requise")?;
    if adresses.len() != 1 || source.dimensions.unwrap_or(0) == 0 {
        return Err("activation gérée : une adresse et une dimension explicites requises".into());
    }
    let locale = adresses[0].clone();
    let port_distant = {
        let mut port = 0;
        for (role, adr) in [("locale", &locale), ("distante", &distante)] {
            let sa = adr.parse::<std::net::SocketAddr>().map_err(|_| format!("activation distante : adresse {role} numérique requise"))?;
            if !sa.ip().is_loopback() || sa.port() == 0 {
                return Err(format!("activation distante : adresse {role} locale à port fixe requise"));
            }
            port = sa.port();
        }
        port
    };
    if a.budget_activation_secs == 0 || a.budget_attente_secs == 0 {
        return Err("activation gérée : budgets non vides requis".into());
    }
    let attendue = identite(source)?;
    let env = variables(source, &attendue)?;
    let ssh = a.ssh.clone().unwrap_or_else(|| vec!["ssh".into()]);
    let commande = |args: Vec<String>| {
        let mut c = Command::new(&ssh[0]);
        c.args(&ssh[1..]).args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5"]).args(args).stdin(Stdio::null());
        c
    };
    let refus = |cause: String| format!("activation distante : ssh {hote} : {}", cause.trim());

    // 1. Le tunnel. Un ssh qui échoue sort vite (BatchMode) : c'est le refus net.
    let mut enfant = commande(vec!["-N".into(), "-o".into(), "ExitOnForwardFailure=yes".into(), "-L".into(), format!("{locale}:{distante}"), hote.into()])
        .stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| refus(e.to_string()))?;
    let debut = Instant::now();
    while debut.elapsed() < Duration::from_millis(1000) {
        if let Ok(Some(statut)) = enfant.try_wait() {
            let mut erreur = String::new();
            if let Some(mut e) = enfant.stderr.take() { let _ = std::io::Read::read_to_string(&mut e, &mut erreur); }
            return Err(refus(if erreur.trim().is_empty() { format!("tunnel fermé ({statut})") } else { erreur }));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let tunnel = Tunnel(enfant);

    // 2. Par le tunnel : le fournisseur déclaré est-il déjà là ?
    let joindre = || crate::daemon::DaemonEmbedder::joindre_dans(&locale, Duration::from_secs(2));
    let client = match joindre() {
        Ok(c) => {
            verifier_l_identite(source, &c).map_err(|e| format!("activation distante : le port {distante} de {hote} est pris par un autre fournisseur ({e})"))?;
            c
        }
        Err(_) => {
            // 3. Personne : lancer l'unité transitoire, puis attendre qu'elle réponde.
            let modele: String = source.model.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' }).collect();
            let unite = format!("{PREFIXE_UNITE}{modele}-{port_distant}");
            let mut distants = vec!["systemd-run".to_string(), "--user".into(), format!("--unit={unite}"), "--collect".into(), "--property=Restart=on-failure".into()];
            distants.extend(env.iter().map(|(k, v)| format!("--setenv={k}={v}")));
            distants.push("--".into());
            distants.push(a.programme.clone());
            distants.extend(a.args.iter().cloned());
            distants.push("--adresse".into());
            distants.push(distante.clone());
            // Cités pour le shell distant (fish ou sh) : entre apostrophes, et
            // rien qui puisse en sortir.
            if let Some(mauvais) = distants.iter().find(|x| x.contains('\'') || x.contains('\\')) {
                return Err(format!("activation distante : argument non citable « {mauvais} »"));
            }
            let mut args = vec![hote.to_string()];
            args.extend(distants.iter().map(|x| format!("'{x}'")));
            let sortie = commande(args).output().map_err(|e| refus(e.to_string()))?;
            if !sortie.status.success() {
                return Err(refus(String::from_utf8_lossy(&sortie.stderr).to_string()));
            }
            let echeance = Instant::now() + Duration::from_secs(a.budget_activation_secs);
            loop {
                if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
                if let Ok(c) = joindre() { break c; }
                if Instant::now() >= echeance {
                    return Err(format!("activation distante : {unite} sur {hote} ne répond pas après {} s (journal : ssh {hote} journalctl --user -u {unite})", a.budget_activation_secs));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    };
    verifier_l_identite(source, &client)?;
    let delai_appel = source.timeout_secs.filter(|s| *s > 0).map(Duration::from_secs)
        .unwrap_or_else(crate::daemon::embeddings::default_embed_timeout);
    let client = client.with_timeout(delai_appel);
    let usage = client.prendre_bail(&format!("activation:{}:{}", std::process::id(), source.model), Duration::from_secs(60))
        .map_err(|e| format!("bail d'usage : {e}"))?;
    let origine = super::Origin::Service(locale);
    Ok((Box::new(Gere { client, _usage: usage, _tenue: Box::new(tunnel) }), origine))
}

#[cfg(feature = "daemon")]
struct Gere {
    client: crate::daemon::DaemonEmbedder,
    // Rendu avant ce qui tient le fournisseur (bail local ou tunnel) : l'ordre
    // des champs est l'ordre de leur fin.
    _usage: Option<crate::daemon::embeddings::BailDistant>,
    _tenue: Box<dyn std::any::Any + Send + Sync>,
}

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
            // Le port 0 : le fournisseur prend le sien et l'annonce. Réserver un
            // port puis le relâcher laissait un autre test le prendre entre les
            // deux (le rouge d'une lib entière sur sept, 13 novembre 2026).
            let mut source = ModelSource::service("fixture-residence-384");
            source.address = Addresses::parse("127.0.0.1:0");
            source.dimensions = Some(384);
            source.timeout_secs = Some(1);
            source.activation = Some(Activation {
                programme: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py").into(),
                args: vec!["--journal".into(), dossier.path().join("events.jsonl").display().to_string(),
                    "--generation".into(), "appelant-reel".into()],
                env: Default::default(), profil: "fixture-cpu".into(), peripherique: "cpu".into(),
                coordination: dossier.path().join("coordination"), journal: dossier.path().join("journaux"),
                budget_activation_secs: 5, budget_attente_secs: 5, residence: Residence::Maintenir,
                hote: None, adresse_distante: None, ssh: None,
            });
            // Même chemin que le manifeste : la déclaration traverse serde.
            let source = serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
            Self { dossier, source }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let journal = self.dossier.path().join("events.jsonl");
            if let Ok(texte) = std::fs::read_to_string(journal) {
                let lignes: Vec<serde_json::Value> = texte.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
                let mut pids: Vec<i64> = lignes.iter().filter_map(|v| v.get("pid").and_then(|n| n.as_i64())).collect();
                pids.sort_unstable();
                pids.dedup();
                for pid in pids {
                    // Seulement un fournisseur que son journal ne dit pas arrêté, et
                    // dont le processus est encore notre fixture : un pid mort peut
                    // avoir été repris par un autre processus. Jamais pgrep global.
                    let arrete = lignes.iter().any(|v| v["pid"].as_i64() == Some(pid) && v["event"] == "arrete");
                    let notre = std::fs::read(format!("/proc/{pid}/cmdline")).map(|c| String::from_utf8_lossy(&c).contains("fournisseur.py")).unwrap_or(false);
                    if !arrete && notre {
                        unsafe { libc::kill(pid as i32, libc::SIGTERM); libc::waitpid(pid as i32, std::ptr::null_mut(), 0); }
                    }
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
        let prise = crate::serveur::activation::adresse_retenue(&f.source.activation.as_ref().unwrap().coordination, &f.source.address.0[0])
            .expect("l'adresse prise est retenue au dossier de coordination");
        let d = crate::daemon::DaemonEmbedder::joindre(&prise).unwrap();
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
        let f = Fixture::nouvelle();
        assert_eq!(f.source.address.0[0], "127.0.0.1:0", "la fixture déclare le port 0");
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

    /// Une déclaration distante sur un faux ssh (`tests/fixtures/residence/faux_ssh.py`) :
    /// le « poste distant » est ce poste-ci, le tunnel ne relaie rien et le
    /// fournisseur écoute sur le port local même. Le port est réservé puis
    /// relâché (une adresse distante est fixe par construction).
    fn distante(f: &Fixture, echec: bool) -> (ModelSource, u16) {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut source = f.source.clone();
        source.address = Addresses::parse(&format!("127.0.0.1:{port}"));
        let a = source.activation.as_mut().unwrap();
        a.hote = Some("faux".into());
        a.adresse_distante = Some(format!("127.0.0.1:{port}"));
        let mut ssh = vec!["python3".to_string(), concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/faux_ssh.py").to_string(),
            "--journal".to_string(), f.dossier.path().join("ssh.jsonl").display().to_string()];
        if echec { ssh.push("--echec".into()); }
        a.ssh = Some(ssh);
        (source, port)
    }

    fn appels_ssh(f: &Fixture) -> Vec<String> {
        std::fs::read_to_string(f.dossier.path().join("ssh.jsonl")).unwrap_or_default().lines().map(String::from).collect()
    }

    /// **L'activation distante lance une unité transitoire par ssh** (règles de
    /// `docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md`) :
    /// `systemd-run --user`, préfixe `rag3weaver-fournisseur-`, `--collect`,
    /// `Restart=on-failure`, jamais une unité `rag3weaver-embeddings@` ; un
    /// tunnel `ssh -N -L` en `BatchMode` ; la seconde demande trouve le
    /// fournisseur par le tunnel et ne relance rien.
    #[test]
    fn une_activation_distante_lance_une_unite_transitoire_par_ssh() {
        let f = Fixture::nouvelle();
        let (source, port) = distante(&f, false);
        let (client, _) = connect_embedder(&source).unwrap_or_else(|e| panic!("{e}\n{:?}", appels_ssh(&f)));
        assert_eq!(client.embed(&["distant".into()]).unwrap()[0].len(), 384);
        let appels = appels_ssh(&f);
        let lances: Vec<&String> = appels.iter().filter(|l| l.contains("systemd-run")).collect();
        assert_eq!(lances.len(), 1, "{appels:?}");
        let lance = lances[0];
        for attendu in [format!("rag3weaver-fournisseur-fixture-residence-384-{port}"), "--user".into(), "--collect".into(), "Restart=on-failure".into()] {
            assert!(lance.contains(&attendu), "{attendu} absent : {lance}");
        }
        assert!(appels.iter().all(|l| !l.contains("rag3weaver-embeddings@")), "jamais une unité posée à la main : {appels:?}");
        assert!(appels.iter().any(|l| l.contains("\"-N\"") && l.contains("\"-L\"") && l.contains("BatchMode=yes")), "le tunnel : {appels:?}");
        drop(client);
        let (encore, _) = connect_embedder(&source).unwrap();
        assert_eq!(encore.embed(&["encore".into()]).unwrap()[0].len(), 384);
        assert_eq!(appels_ssh(&f).iter().filter(|l| l.contains("systemd-run")).count(), 1, "le fournisseur déjà là ne se relance pas");
    }

    /// **Le vrai jeu, une fois** : depuis ce poste, un fournisseur lancé sur
    /// un autre par ssh, joint par le tunnel, compté, puis évincé après son
    /// délai et son unité effacée. Ignoré par défaut ; piloté par
    /// `RAG3WEAVER_ESSAI_DISTANT_{HOTE,PROGRAMME,MODELE,DIM,PORT_DISTANT,PORT_LOCAL,ENV,INACTIVITE}`
    /// (`ENV` : `CLE=valeur;CLE=valeur`). Une variable absente rougit : il ne
    /// se saute pas.
    #[test]
    #[ignore = "lance un fournisseur réel sur un autre poste par ssh"]
    fn une_activation_distante_reelle_lance_compte_et_evince() {
        let v = |n: &str| std::env::var(format!("RAG3WEAVER_ESSAI_DISTANT_{n}")).unwrap_or_else(|_| panic!("RAG3WEAVER_ESSAI_DISTANT_{n} requise"));
        let (hote, modele) = (v("HOTE"), v("MODELE"));
        let dim: usize = v("DIM").parse().unwrap();
        let inactivite: u64 = v("INACTIVITE").parse().unwrap();
        let (port_distant, port_local) = (v("PORT_DISTANT"), v("PORT_LOCAL"));
        let dossier = tempfile::tempdir().unwrap();
        let mut source = ModelSource::service(&modele);
        source.address = Addresses::parse(&format!("127.0.0.1:{port_local}"));
        source.dimensions = Some(dim);
        source.timeout_secs = Some(60);
        source.activation = Some(Activation {
            programme: v("PROGRAMME"), args: vec![],
            env: v("ENV").split(';').filter(|x| !x.is_empty()).map(|kv| { let (k, v) = kv.split_once('=').unwrap(); (k.to_string(), v.to_string()) }).collect(),
            profil: "essai-distant".into(), peripherique: "card2".into(),
            coordination: dossier.path().join("coordination"), journal: dossier.path().join("journaux"),
            budget_activation_secs: 180, budget_attente_secs: 180,
            residence: Residence::ADemande { inactivite_secs: inactivite },
            hote: Some(hote.clone()), adresse_distante: Some(format!("127.0.0.1:{port_distant}")), ssh: None,
        });
        let unite = format!("{PREFIXE_UNITE}{modele}-{port_distant}");
        let ssh = |cmd: &str| {
            let s = std::process::Command::new("ssh").args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5", &hote, cmd]).output().unwrap();
            String::from_utf8_lossy(&s.stdout).trim().to_string()
        };
        let debut = std::time::Instant::now();
        let (client, origine) = connect_embedder(&source).unwrap_or_else(|e| panic!("{e}"));
        println!("▸ activé en {:?} : {origine:?}", debut.elapsed());
        let vecteurs = client.embed(&["un texte".into(), "un autre".into()]).unwrap();
        assert!(vecteurs.iter().all(|x| x.len() == dim));
        let id = crate::daemon::identite_de::<serde_json::Value>(&crate::daemon::agent_with_timeout(std::time::Duration::from_secs(5)), &format!("127.0.0.1:{port_local}"), crate::daemon::embeddings::SERVICE).unwrap();
        println!("▸ identité : usage {} eviction {} executable {}", id["usage"], id["eviction"], id["executable"]);
        assert!(id["usage"]["baux"].as_u64().unwrap_or(0) >= 1, "le bail compte : {id}");
        assert_eq!(id["eviction"]["inactivite_s"], inactivite, "évincable : {id}");
        println!("▸ unité : {}", ssh(&format!("systemctl --user is-active {unite}")));
        drop(client);
        let rendu = std::time::Instant::now();
        let mut etat = String::new();
        while rendu.elapsed() < std::time::Duration::from_secs(inactivite + 60) {
            std::thread::sleep(std::time::Duration::from_secs(5));
            etat = ssh(&format!("systemctl --user is-active {unite}"));
            if etat != "active" && etat != "activating" { break; }
        }
        println!("▸ après {:?} sans usage : {etat}", rendu.elapsed());
        assert!(etat != "active" && etat != "activating", "évincé après son délai : {etat}");
        let restantes = ssh(&format!("systemctl --user list-units --all --no-legend '{PREFIXE_UNITE}*'"));
        assert!(!restantes.contains(&unite), "unité effacée (--collect) : {restantes}");
    }

    /// **Un ssh en échec se refuse net** : nommé, sans attente ni nouvel essai.
    #[test]
    fn une_activation_distante_refuse_net_un_ssh_en_echec() {
        let f = Fixture::nouvelle();
        let (source, _) = distante(&f, true);
        let debut = std::time::Instant::now();
        let erreur = connect_embedder(&source).err().expect("refus");
        assert!(erreur.contains("activation distante : ssh faux"), "{erreur}");
        assert!(debut.elapsed() < std::time::Duration::from_secs(4), "refus net, sans attente : {:?}", debut.elapsed());
        assert!(appels_ssh(&f).iter().all(|l| !l.contains("systemd-run")), "rien n'est lancé derrière un tunnel refusé");
    }

    /// Une déclaration **à la demande** sur la fixture : le fournisseur
    /// s'évince après `inactivite` secondes sans bail ni calcul ; le client
    /// rend son bail après `rendre` secondes sans appel.
    fn a_la_demande(inactivite: u64, rendre: u64) -> Fixture {
        let mut f = Fixture::nouvelle();
        f.source.activation.as_mut().unwrap().residence = Residence::ADemande { inactivite_secs: inactivite, rendre_le_bail_secs: rendre };
        f
    }

    fn evenements(f: &Fixture, nom: &str) -> usize {
        std::fs::read_to_string(f.dossier.path().join("events.jsonl")).unwrap_or_default().lines()
            .filter(|l| l.contains(&format!("\"event\": \"{nom}\""))).count()
    }

    fn attendre_l_evenement(f: &Fixture, nom: &str, combien: usize, delai: std::time::Duration) -> bool {
        let debut = std::time::Instant::now();
        while debut.elapsed() < delai {
            if evenements(f, nom) >= combien { return true; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        false
    }

    /// **À la demande, ouvrir ne lance rien** (ticket
    /// `2026-11-13-un-fournisseur-a-la-demande-ouvert-par-l-hote-ne-s-evince-jamais`) :
    /// l'embarqueur dit son modèle et sa dimension déclarés sans fournisseur.
    #[test]
    fn a_la_demande_ouvrir_ne_lance_rien() {
        let f = a_la_demande(2, 1);
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.name(), "fixture-residence-384");
        assert_eq!(client.dim(), 384);
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert_eq!(evenements(&f, "constructeur_demarre"), 0, "ouvrir ne lance pas le fournisseur");
    }

    /// **Le premier calcul lance le fournisseur** et rend ses vecteurs, en
    /// moins d'une minute.
    #[test]
    fn a_la_demande_un_calcul_lance_et_rend_ses_vecteurs() {
        let f = a_la_demande(2, 1);
        let (client, _) = connect_embedder(&f.source).unwrap();
        let debut = std::time::Instant::now();
        let v = client.embed(&["premier".into()]).unwrap();
        assert_eq!(v[0].len(), 384);
        assert!(debut.elapsed() < std::time::Duration::from_secs(60), "{:?}", debut.elapsed());
        assert_eq!(evenements(&f, "constructeur_demarre"), 1);
    }

    /// **Le bail rendu, le fournisseur s'évince** : après le calcul, le client
    /// rend son bail au bout de son délai, et le fournisseur s'en va au bout
    /// du sien, même si le client vit encore.
    #[test]
    fn a_la_demande_le_bail_rendu_le_fournisseur_s_evince() {
        let f = a_la_demande(2, 1);
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["avant".into()]).unwrap();
        assert!(attendre_l_evenement(&f, "evince", 1, std::time::Duration::from_secs(15)), "évincé alors que le client vit");
        drop(client);
    }

    /// **Un calcul après l'éviction relance** le fournisseur, sans erreur pour
    /// l'appelant, et l'identité est revérifiée.
    #[test]
    fn a_la_demande_un_second_calcul_relance() {
        let f = a_la_demande(2, 1);
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["avant".into()]).unwrap();
        assert!(attendre_l_evenement(&f, "evince", 1, std::time::Duration::from_secs(15)));
        // Le premier fournisseur est parti ; son pid n'est plus à tuer par la fixture.
        assert!(attendre_l_evenement(&f, "arrete", 1, std::time::Duration::from_secs(5)));
        let v = client.embed(&["après".into()]).unwrap();
        assert_eq!(v[0].len(), 384);
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "relancé une fois");
    }

    #[test]
    fn budget_de_sonde_ne_remplace_pas_delai_appel_par_defaut() {
        let mut f = Fixture::nouvelle(); f.source.timeout_secs = None;
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.timeout(), Some(crate::daemon::embeddings::default_embed_timeout()));
    }
}
