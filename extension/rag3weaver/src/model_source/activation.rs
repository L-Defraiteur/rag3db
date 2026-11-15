//! Politique déclarative initiale : activation locale et maintien explicite.
//! Les identités de profil/périphérique qualifient la recette ; elles ne prouvent
//! pas le placement physique des allocations (à qualifier sur le fournisseur).
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    /// **Le nom de l'activation** : celui sous lequel `activations` la déclare
    /// au niveau du manifeste, et que chaque déclaration cite. Absent (un bloc
    /// écrit dans la déclaration) : le `model` de la déclaration. Deux
    /// déclarations qui citent le même nom partagent le processus, la
    /// coordination et l'éviction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nom: Option<String>,
    /// **Le modèle que le processus charge** (`RAG3WEAVER_EMBED_MODEL`) : pour un
    /// groupe cité par le relecteur ou l'OCR, ce n'est pas le modèle de la
    /// déclaration. Absent : le `model` de la déclaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modele: Option<String>,
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
    /// **La mémoire que le fournisseur prend sur sa carte**, en Mio. Sans elle,
    /// la carte n'est pas mesurée avant d'activer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_mib: Option<u64>,
    /// La marge exigée en plus de `required_mib` (défaut 2048 Mio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_mib: Option<u64>,
    /// Le débit, en textes par seconde, sous lequel l'embarqueur se replie
    /// après `window_secs`. Sans lui, le débit n'est pas mesuré.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_throughput: Option<u64>,
    /// La fenêtre du débit, et l'intervalle des remesures de la carte pendant
    /// un repli (défaut 60 s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_secs: Option<u64>,
    /// **La tenue d'un repli** : le temps minimal avant qu'il ne revienne à la
    /// carte (défaut 4 × `window_secs`), dit dans l'état (« replié jusqu'à
    /// <heure> au plus tôt »). Une carte qui se libère un instant ne fait pas
    /// alterner (décision de c7, 15 novembre 2026).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_hold_secs: Option<u64>,
}

// La racine drm des cartes factices d'un témoin, par fil (les témoins
// tournent en parallèle dans un processus ; hors témoins, la racine vient de
// `RAG3WEAVER_DRM_RACINE`, sinon `/sys/class/drm`).
#[cfg(all(test, feature = "daemon"))]
thread_local! {
    pub(crate) static RACINE_DRM_DU_FIL: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
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
    /// **À la demande** : rien n'est lancé à l'ouverture ; le premier calcul
    /// lance le fournisseur et prend un bail, rendu après
    /// `rendre_le_bail_secs` sans appel ; le fournisseur se décharge après
    /// `inactivite_secs` sans bail ni calcul (il le déclare dans son identité,
    /// `eviction`) ; un calcul suivant le relance. Les deux délais sont
    /// nommés dans les règles (`docs/embarquement/13-11-2026/01-…`) ;
    /// `rendre_le_bail_secs` est plus court qu'`inactivite_secs`.
    ADemande {
        inactivite_secs: u64,
        #[serde(default = "rendre_le_bail_par_defaut")]
        rendre_le_bail_secs: u64,
    },
}

/// Le délai par défaut au bout duquel un embarqueur à la demande rend son
/// bail sans appel.
pub const RENDRE_LE_BAIL_PAR_DEFAUT_SECS: u64 = 30;

fn rendre_le_bail_par_defaut() -> u64 {
    RENDRE_LE_BAIL_PAR_DEFAUT_SECS
}

impl Activation {
    /// Le nom de l'activation pour cette déclaration : déclaré, sinon son modèle.
    pub fn nom_pour<'a>(&'a self, source: &'a super::ModelSource) -> &'a str {
        self.nom.as_deref().unwrap_or(&source.model)
    }
    /// Le modèle que le processus charge : déclaré, sinon celui de la déclaration.
    pub fn modele_pour<'a>(&'a self, source: &'a super::ModelSource) -> &'a str {
        self.modele.as_deref().unwrap_or(&source.model)
    }
}

#[cfg(feature = "daemon")]
fn identite(source: &super::ModelSource) -> Result<crate::daemon::embeddings::IdentiteActivation, String> {
    let a = source.activation.as_ref().ok_or("activation absente")?;
    // Les budgets d'attente et chemins de coordination ne changent pas le
    // fournisseur. L'empreinte décrit la recette, elle ne divulgue pas son env.
    // Le nom et le bloc : deux déclarations qui citent la même activation (le
    // creux, le relecteur et l'OCR d'un groupe) attendent la même recette. La
    // dimension n'en est plus : elle se vérifie à part, pour le dense.
    let recette = serde_json::to_vec(&(a.nom_pour(source), a.modele_pour(source), &a.programme, &a.args, &a.env, &a.profil, &a.peripherique))
        .map_err(|e| e.to_string())?;
    Ok(crate::daemon::embeddings::IdentiteActivation {
        profil: a.profil.clone(), peripherique: a.peripherique.clone(),
        recette: blake3::hash(&recette).to_hex().to_string(),
    })
}

#[cfg(feature = "daemon")]
pub fn demander(source: &super::ModelSource) -> Result<crate::serveur::activation::Demande, String> {
    demander_pour(source, super::Capability::Embed)
}

/// [`demander`], pour une capacité : le creux, le relecteur et l'OCR n'ont pas
/// de dimension à déclarer.
#[cfg(feature = "daemon")]
pub fn demander_pour(source: &super::ModelSource, capacite: super::Capability) -> Result<crate::serveur::activation::Demande, String> {
    use super::{Capability, Fallback, Provider};
    let a = source.activation.as_ref().ok_or("activation absente")?;
    let attendue = identite(source)?;
    if source.provider != Provider::Service || !matches!(source.fallback, Fallback::Refuse | Fallback::Service(_)) {
        return Err("activation gérée : service sans repli requis".into());
    }
    let adresses = source.addresses(Capability::Embed, &super::process_env);
    if adresses.len() != 1 || (capacite == Capability::Embed && source.dimensions.unwrap_or(0) == 0) {
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
        ("RAG3WEAVER_EMBED_MODEL".to_string(), a.modele_pour(source).to_string()),
        ("RAG3WEAVER_ACTIVATION_PROFIL".to_string(), a.profil.clone()),
        ("RAG3WEAVER_ACTIVATION_PERIPHERIQUE".to_string(), a.peripherique.clone()),
        ("RAG3WEAVER_ACTIVATION_IDENTITE".to_string(), attendue.recette.clone()),
    ];
    if let Residence::ADemande { inactivite_secs, .. } = a.residence {
        if inactivite_secs == 0 {
            return Err("activation : un délai d'inactivité nul déchargerait aussitôt".into());
        }
        v.push((crate::daemon::embeddings::EVICTION_VARIABLE.to_string(), inactivite_secs.to_string()));
    }
    v.push((crate::daemon::embeddings::RELANCES_VARIABLE.to_string(), relances_notees(source).to_string()));
    for (cle, valeur) in &a.env {
        if matches!(cle.as_str(), "RAG3WEAVER_EMBED_MODEL" | "RAG3WEAVER_ACTIVATION_PROFIL" | "RAG3WEAVER_ACTIVATION_PERIPHERIQUE" | "RAG3WEAVER_ACTIVATION_IDENTITE" | crate::daemon::embeddings::EVICTION_VARIABLE | crate::daemon::embeddings::RELANCES_VARIABLE) {
            return Err(format!("activation : {cle} est déterminé par l'identité déclarée"));
        }
        v.push((cle.clone(), valeur.clone()));
    }
    Ok(v)
}

/// Le fichier du compte des relances d'une déclaration, dans son dossier de
/// coordination (une ligne : le nombre).
#[cfg(feature = "daemon")]
fn fichier_des_relances(source: &super::ModelSource) -> Option<std::path::PathBuf> {
    let a = source.activation.as_ref()?;
    let nom: String = source.model.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    Some(a.coordination.join(format!("{nom}.relances")))
}

/// Les relances après plantage que cette déclaration a déjà eues.
#[cfg(feature = "daemon")]
fn relances_notees(source: &super::ModelSource) -> u64 {
    fichier_des_relances(source).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

/// Une relance de plus, notée avant de relancer : le relancé la dit à son
/// /sante. Écrit puis renommé (aucun lecteur ne voit un fichier à moitié).
#[cfg(feature = "daemon")]
fn noter_une_relance(source: &super::ModelSource) {
    let Some(p) = fichier_des_relances(source) else { return };
    let n = relances_notees(source) + 1;
    if let Some(dossier) = p.parent() { let _ = std::fs::create_dir_all(dossier); }
    let provisoire = p.with_extension(format!("relances.{}", std::process::id()));
    if std::fs::write(&provisoire, format!("{n}\n")).is_ok() {
        let _ = std::fs::rename(&provisoire, &p);
    }
}

/// **Un fournisseur activé pour une capacité** (son identité vérifiée) :
/// le bail d'usage, ce qui le tient (bail local d'activation ou tunnel) et
/// l'adresse où le joindre.
#[cfg(feature = "daemon")]
pub(super) struct Actif {
    usage: Option<crate::daemon::embeddings::BailDistant>,
    tenue: Box<dyn std::any::Any + Send + Sync>,
    adresse: String,
}

/// **Activer le fournisseur d'une déclaration, pour une capacité** : le lancer
/// ou s'y attacher (ici ou à distance), vérifier son identité pour cette
/// capacité, prendre un bail d'usage.
#[cfg(feature = "daemon")]
fn activer(source: &super::ModelSource, capacite: super::Capability, annule: &std::sync::atomic::AtomicBool) -> Result<Actif, String> {
    let debut = std::time::Instant::now();
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    if source.activation.as_ref().is_some_and(|a| a.hote.is_some()) {
        return activer_distant(source, capacite, annule);
    }
    let demande = demander_pour(source, capacite)?;
    let a = source.activation.as_ref().unwrap();
    let bail = demande.attendre(annule, std::time::Duration::from_secs(a.budget_attente_secs))?;
    let restant = std::time::Duration::from_secs(a.budget_attente_secs).saturating_sub(debut.elapsed());
    if restant.is_zero() { return Err("budget de connexion à l'activation dépassé".into()); }
    let mut client = crate::daemon::DaemonEmbedder::joindre_dans(bail.adresse(), restant).map_err(|e| e.to_string())?;
    let delai_appel = source.timeout_secs.filter(|s| *s > 0).map(std::time::Duration::from_secs)
        .unwrap_or_else(crate::daemon::embeddings::default_embed_timeout);
    client = client.with_timeout(delai_appel);
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    verifier_pour(source, capacite, &client)?;
    // Un bail d'usage chez le fournisseur : il compte cet usager tant que le
    // client vit. Un fournisseur d'avant les baux ne compte pas : `None`, pas
    // une erreur.
    let usage = client.prendre_bail(&format!("activation:{}:{}", std::process::id(), source.model), std::time::Duration::from_secs(60))
        .map_err(|e| format!("bail d'usage : {e}"))?;
    let adresse = bail.adresse().to_string();
    Ok(Actif { usage, tenue: Box::new(bail), adresse })
}

/// Le fournisseur trouvé est-il celui déclaré : modèle, dimension, et
/// l'attestation de recette/profil/périphérique.
#[cfg(feature = "daemon")]
fn verifier_pour(source: &super::ModelSource, capacite: super::Capability, client: &crate::daemon::DaemonEmbedder) -> Result<(), String> {
    sert(source, capacite, client).map_err(|e| format!("identité d'activation refusée : {e}"))?;
    if client.identite().activation.as_ref() != Some(&identite(source)?) {
        return Err("identité d'activation refusée : recette/profil du fournisseur différent ou non attesté".into());
    }
    Ok(())
}

/// **Le fournisseur sert-il la capacité déclarée** : modèle et dimension
/// (le dual et sa dimension pour un creux qui la déclare). Ce que le repli
/// vérifie aussi ; la recette, elle, ne vaut que pour le fournisseur activé
/// (elle porte le périphérique : un repli ne peut pas l'avoir).
#[cfg(feature = "daemon")]
fn sert(source: &super::ModelSource, capacite: super::Capability, client: &crate::daemon::DaemonEmbedder) -> Result<(), String> {
    use crate::embedder::Embedder;
    use super::Capability;
    let id = client.identite();
    let sert = match capacite {
        Capability::Embed => client.name() == source.model && Some(client.dim()) == source.dimensions,
        Capability::Sparse => id.modele == source.model && id.sparse,
        Capability::Rerank => id.reranker.as_deref() == Some(source.model.as_str()),
        Capability::Ocr => id.ocr.as_deref() == Some(source.model.as_str()),
        _ => false,
    };
    if !sert {
        return Err(format!(
            "{} {} attendu, le fournisseur sert {}/{} (creux {}, relecteur {:?}, OCR {:?})",
            capacite.name(), source.model, id.modele, id.dim, id.sparse, id.reranker, id.ocr
        ));
    }
    // Un creux qui déclare sa dimension déclare son dual (décision de c7, 14
    // novembre 2026) : le fournisseur le sait, à cette dimension.
    if capacite == Capability::Sparse {
        if let Some(d) = source.dimensions.filter(|d| *d > 0) {
            if !id.dual {
                return Err(format!("le fournisseur sert {} sans le dual (models.sparse déclare dimensions {d})", id.modele));
            }
            if id.dim != d {
                return Err(format!("le fournisseur sert {} en {} au lieu de {d} (models.sparse.dimensions)", id.modele, id.dim));
            }
        }
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
fn activer_distant(source: &super::ModelSource, capacite: super::Capability, annule: &std::sync::atomic::AtomicBool) -> Result<Actif, String> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    use super::{Capability, Fallback, Provider};
    let a = source.activation.as_ref().ok_or("activation absente")?;
    let hote = a.hote.as_deref().unwrap_or_default();
    if hote.is_empty() || !hote.chars().all(|c| c.is_ascii_alphanumeric() || "-._@".contains(c)) {
        return Err(format!("activation distante : hôte ssh invalide « {hote} »"));
    }
    if source.provider != Provider::Service || !matches!(source.fallback, Fallback::Refuse | Fallback::Service(_)) {
        return Err("activation gérée : service sans repli requis".into());
    }
    let adresses = source.addresses(Capability::Embed, &super::process_env);
    let distante = a.adresse_distante.clone().ok_or("activation distante : adresse_distante requise")?;
    if adresses.len() != 1 || (capacite == Capability::Embed && source.dimensions.unwrap_or(0) == 0) {
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
            verifier_pour(source, capacite, &c).map_err(|e| format!("activation distante : le port {distante} de {hote} est pris par un autre fournisseur ({e})"))?;
            c
        }
        Err(_) => {
            // 3. Personne : lancer l'unité transitoire, puis attendre qu'elle réponde.
            let modele: String = a.nom_pour(source).chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '-' }).collect();
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
    verifier_pour(source, capacite, &client)?;
    let delai_appel = source.timeout_secs.filter(|s| *s > 0).map(Duration::from_secs)
        .unwrap_or_else(crate::daemon::embeddings::default_embed_timeout);
    let client = client.with_timeout(delai_appel);
    let usage = client.prendre_bail(&format!("activation:{}:{}", std::process::id(), source.model), Duration::from_secs(60))
        .map_err(|e| format!("bail d'usage : {e}"))?;
    Ok(Actif { usage, tenue: Box::new(tunnel), adresse: locale })
}

/// **Le point d'entrée du dense** : une tenue, comme pour les trois autres
/// capacités (décision de c7, 14 novembre 2026 : un seul mécanisme). À la
/// demande, rien n'est activé avant le premier calcul ; maintenu, le
/// fournisseur est activé maintenant, et l'annulation de l'appelant vaut pour
/// cette activation.
#[cfg(feature = "daemon")]
pub(super) fn connecter_ou_attendre(source: &super::ModelSource, annule: &std::sync::atomic::AtomicBool)
    -> Result<(Box<dyn crate::embedder::Embedder>, super::Origin), String> {
    let a = source.activation.as_ref().ok_or("activation absente")?;
    let a_demande = matches!(a.residence, Residence::ADemande { .. });
    let mut dim = 0;
    let mut declaree = None;
    if let Residence::ADemande { inactivite_secs, rendre_le_bail_secs } = a.residence {
        if inactivite_secs == 0 || rendre_le_bail_secs == 0 || rendre_le_bail_secs >= inactivite_secs {
            return Err(format!(
                "activation à la demande : rendre_le_bail_secs ({rendre_le_bail_secs}) doit être non nul et plus court qu'inactivite_secs ({inactivite_secs})"
            ));
        }
        dim = source.dimensions.filter(|d| *d > 0).ok_or("activation à la demande : une dimension explicite requise")?;
        declaree = Some(source.addresses(super::Capability::Embed, &super::process_env).into_iter().next()
            .ok_or("activation à la demande : une adresse requise")?);
        if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    }
    let tenue = Tenue::new(source, super::Capability::Embed, |s, a| {
        crate::daemon::DaemonEmbedder::joindre(a).map(|d| d.with_timeout(delai_appel(s))).map_err(|e| e.to_string())
    }, annule)?;
    use crate::embedder::Embedder as _;
    // Maintenu : activé, son adresse prise (port 0 compris) et ce qu'il dit de lui.
    let vu = tenue.regarder(|c| (c.is_mock(), c.budget_conseille(), c.troncatures()));
    if !a_demande {
        dim = tenue.regarder(|c| c.dim()).unwrap_or(0);
    }
    let adresse = match declaree {
        Some(a) => a,
        None => tenue.adresse_active().ok_or("activation : fournisseur maintenu sans adresse")?,
    };
    Ok((Box::new(DenseTenu { tenue, dim, vu: std::sync::Mutex::new(vu) }), super::Origin::Service(adresse)))
}

/// **Un fournisseur tenu pour une capacité** : son client, et ce qui le
/// garde (le bail d'usage, puis le bail local d'activation ou le tunnel).
/// L'ordre des champs est l'ordre de leur fin.
#[cfg(feature = "daemon")]
struct Prise<C> {
    client: C,
    adresse: String,
    /// D'où viennent ses calculs : « carte card2 » (le fournisseur activé), ou
    /// « service 127.0.0.1:7879 » (le repli).
    provenance: String,
    _usage: Option<crate::daemon::embeddings::BailDistant>,
    _tenue: Box<dyn std::any::Any + Send + Sync>,
}

/// La marge par défaut exigée en plus de `required_mib`.
#[cfg(feature = "daemon")]
pub const MARGE_PAR_DEFAUT_MIO: u64 = 2048;

/// La fenêtre par défaut du débit, et des remesures pendant un repli.
#[cfg(feature = "daemon")]
pub const FENETRE_PAR_DEFAUT_SECS: u64 = 60;

/// Où se lisent les cartes : `RAG3WEAVER_DRM_RACINE`, sinon `/sys/class/drm`
/// (un témoin donne la sienne à son fil).
#[cfg(feature = "daemon")]
fn racine_drm() -> std::path::PathBuf {
    #[cfg(test)]
    if let Some(r) = RACINE_DRM_DU_FIL.with(|r| r.borrow().clone()) {
        return r;
    }
    std::env::var_os("RAG3WEAVER_DRM_RACINE").map(Into::into).unwrap_or_else(|| "/sys/class/drm".into())
}

/// **La VRAM libre d'une carte nommée**, en Mio, lue dans
/// `<racine>/<carte>/device/mem_info_vram_{total,used}` (amdgpu). `Err` dit
/// pourquoi elle ne se lit pas : jamais une carte devinée.
#[cfg(feature = "daemon")]
fn vram_libre_mio(carte: &str) -> Result<u64, String> {
    if carte.is_empty() || carte.contains('/') || carte == "." || carte == ".." {
        return Err(format!("« {carte} » ne nomme pas une carte drm"));
    }
    let device = racine_drm().join(carte).join("device");
    let lire = |nom: &str| {
        let chemin = device.join(nom);
        std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))
            .and_then(|t| t.trim().parse::<u64>().map_err(|e| format!("{} : {e}", chemin.display())))
    };
    let (total, prise) = (lire("mem_info_vram_total")?, lire("mem_info_vram_used")?);
    Ok(total.saturating_sub(prise) >> 20)
}

/// Une carte mesurée contre ce que l'activation exige.
#[cfg(feature = "daemon")]
struct Mesure {
    carte: String,
    libres: u64,
    requis: u64,
    marge: u64,
}

#[cfg(feature = "daemon")]
impl Mesure {
    fn pleine(&self) -> bool {
        self.libres < self.requis + self.marge
    }
}

/// L'heure d'un repli ou d'un retour, à dire.
#[cfg(feature = "daemon")]
fn heure() -> String {
    jiff::Zoned::now().strftime("%H h %M").to_string()
}

/// **Ce que la tenue dit de sa provenance** : les textes calculés par
/// provenance depuis la dernière lecture, les changements à dire, l'état, et
/// ce qu'il faut pour décider (le repli en cours, la fenêtre du débit).
#[cfg(feature = "daemon")]
#[derive(Default)]
struct Suivi {
    comptes: std::collections::BTreeMap<String, usize>,
    dits: Vec<String>,
    etat: Option<String>,
    /// Replié : depuis la dernière mesure de la carte (remesurée après la fenêtre).
    replie: Option<std::time::Instant>,
    /// Replié : pas de retour à la carte avant cet instant (la tenue).
    tenu_jusqu_a: Option<std::time::Instant>,
    /// Le débit de la carte : le début de la fenêtre et les textes calculés depuis.
    fenetre: Option<(std::time::Instant, usize)>,
    illisible_dit: bool,
}

#[cfg(feature = "daemon")]
type EtatPrise<C> = std::sync::Arc<std::sync::Mutex<Option<(Prise<C>, std::time::Instant)>>>;

/// **La tenue d'un fournisseur pour une capacité** (le creux, le relecteur,
/// l'OCR d'un groupe ; décision de c7, 14 novembre 2026). Elle porte la
/// résidence déclarée, pour toute capacité :
/// - `a_demande` : rien à l'ouverture ; le premier appel active (identité
///   vérifiée pour cette capacité, un bail à elle) ; le client est rendu,
///   bail compris, après `rendre_le_bail_secs` sans appel ;
/// - `maintenir` : activé tout de suite, tenu.
///
/// Un appel qui trouve le fournisseur parti (évincé, planté) le relance une
/// fois, et sinon se refuse en disant le départ et la relance. Le dense passe
/// par elle aussi (`DenseTenu`) : un seul mécanisme pour les quatre capacités.
#[cfg(feature = "daemon")]
struct Tenue<C: Send + 'static> {
    source: super::ModelSource,
    capacite: super::Capability,
    joindre: fn(&super::ModelSource, &str) -> Result<C, String>,
    etat: EtatPrise<C>,
    arret: std::sync::Arc<std::sync::atomic::AtomicBool>,
    maintenir: bool,
    suivi: std::sync::Mutex<Suivi>,
}

#[cfg(feature = "daemon")]
impl<C: Send + 'static> Tenue<C> {
    fn new(source: &super::ModelSource, capacite: super::Capability, joindre: fn(&super::ModelSource, &str) -> Result<C, String>,
        annule: &std::sync::atomic::AtomicBool) -> Result<Self, String> {
        let a = source.activation.as_ref().ok_or("activation absente")?;
        let etat: EtatPrise<C> = Default::default();
        let arret = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let maintenir = match a.residence {
            Residence::Maintenir => true,
            Residence::ADemande { inactivite_secs, rendre_le_bail_secs } => {
                if inactivite_secs == 0 || rendre_le_bail_secs == 0 || rendre_le_bail_secs >= inactivite_secs {
                    return Err(format!(
                        "activation à la demande : rendre_le_bail_secs ({rendre_le_bail_secs}) doit être non nul et plus court qu'inactivite_secs ({inactivite_secs})"
                    ));
                }
                let rendre = std::time::Duration::from_secs(rendre_le_bail_secs);
                let (e, stop) = (std::sync::Arc::downgrade(&etat), arret.clone());
                let _ = std::thread::Builder::new().name("rendre-le-bail".into()).spawn(move || {
                    while !stop.load(std::sync::atomic::Ordering::Acquire) {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        let Some(etat) = e.upgrade() else { return };
                        let rendu = {
                            let mut g = etat.lock().unwrap_or_else(|e| e.into_inner());
                            if g.as_ref().is_some_and(|(_, dernier)| dernier.elapsed() >= rendre) { g.take() } else { None }
                        };
                        // Lâché hors du verrou : rendre le bail est un appel HTTP.
                        drop(rendu);
                    }
                });
                false
            }
        };
        let t = Self { source: source.clone(), capacite, joindre, etat, arret, maintenir, suivi: Default::default() };
        if maintenir {
            let tenu = t.prendre(annule)?;
            *t.etat.lock().unwrap_or_else(|e| e.into_inner()) = Some((tenu, std::time::Instant::now()));
        }
        Ok(t)
    }

    fn activer(&self) -> Result<Prise<C>, String> {
        self.activer_annulable(&std::sync::atomic::AtomicBool::new(false))
    }

    fn activer_annulable(&self, annule: &std::sync::atomic::AtomicBool) -> Result<Prise<C>, String> {
        let actif = activer(&self.source, self.capacite, annule)?;
        let client = (self.joindre)(&self.source, &actif.adresse)?;
        Ok(Prise { client, adresse: actif.adresse, provenance: self.provenance_activee(), _usage: actif.usage, _tenue: actif.tenue })
    }

    fn suivi(&self) -> std::sync::MutexGuard<'_, Suivi> {
        self.suivi.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn fenetre(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.source.activation.as_ref().and_then(|a| a.window_secs).filter(|s| *s > 0).unwrap_or(FENETRE_PAR_DEFAUT_SECS))
    }

    /// La tenue d'un repli : `fallback_hold_secs`, sinon quatre fenêtres.
    fn tenue_du_repli(&self) -> std::time::Duration {
        self.source.activation.as_ref().and_then(|a| a.fallback_hold_secs).map(std::time::Duration::from_secs).unwrap_or(4 * self.fenetre())
    }

    /// La provenance du fournisseur activé : sa carte, quand elle se lit ;
    /// sinon l'activation et son périphérique déclaré (`cpu`, une carte
    /// d'un poste distant).
    fn provenance_activee(&self) -> String {
        let a = self.source.activation.as_ref().expect("une tenue porte une activation");
        match (&a.hote, vram_libre_mio(&a.peripherique)) {
            (None, Ok(_)) => format!("carte {}", a.peripherique),
            (Some(hote), _) => format!("activation {} ({} de {hote})", a.nom_pour(&self.source), a.peripherique),
            (None, Err(_)) => format!("activation {} ({})", a.nom_pour(&self.source), a.peripherique),
        }
    }

    /// **La carte que l'activation nomme, mesurée** — `None` sans
    /// `required_mib`, pour une activation distante (sa carte est à son poste),
    /// ou une carte qui ne se lit pas (dit une fois).
    fn mesurer(&self) -> Option<Mesure> {
        let a = self.source.activation.as_ref()?;
        let requis = a.required_mib?;
        if a.hote.is_some() {
            return None;
        }
        match vram_libre_mio(&a.peripherique) {
            Ok(libres) => Some(Mesure { carte: a.peripherique.clone(), libres, requis, marge: a.margin_mib.unwrap_or(MARGE_PAR_DEFAUT_MIO) }),
            Err(e) => {
                let mut s = self.suivi();
                if !s.illisible_dit {
                    s.illisible_dit = true;
                    s.dits.push(format!("carte {} non mesurée ({e}) : activée sans mesure", a.peripherique));
                }
                None
            }
        }
    }

    /// **Le service de repli, joint** : il doit servir la capacité déclarée
    /// (modèle et dimension) ; sa recette n'est pas vérifiée, et l'état le dit.
    /// Aucun bail n'y est pris : ce n'est pas un fournisseur que l'on gère.
    fn joindre_le_repli(&self, adresse: &str) -> Result<Prise<C>, String> {
        let sonde = crate::daemon::DaemonEmbedder::joindre(adresse)
            .map(|d| d.with_timeout(delai_appel(&self.source)))
            .map_err(|e| format!("repli {adresse} injoignable : {e}"))?;
        sert(&self.source, self.capacite, &sonde).map_err(|e| format!("repli {adresse} refusé : {e}"))?;
        let client = (self.joindre)(&self.source, adresse).map_err(|e| format!("repli {adresse} : {e}"))?;
        Ok(Prise { client, adresse: adresse.to_string(), provenance: format!("service {adresse}"), _usage: None, _tenue: Box::new(()) })
    }

    /// Le repli, dit une fois (un repli rendu puis repris par le bail ne se redit pas).
    fn dire_le_repli(&self, adresse: &str, cause: &str) {
        let mut s = self.suivi();
        if s.replie.is_none() {
            let h = heure();
            let tenue = self.tenue_du_repli();
            let jusqu_a = jiff::SignedDuration::try_from(tenue).ok().and_then(|d| jiff::Zoned::now().checked_add(d).ok())
                .map(|z| z.strftime("%H h %M").to_string()).unwrap_or_else(|| format!("{} s", tenue.as_secs()));
            s.dits.push(format!("replié vers {adresse} à {h}, cause : {cause}"));
            s.etat = Some(format!(
                "replié vers {adresse} depuis {h}, jusqu'à {jusqu_a} au plus tôt (cause : {cause}) — repli sans recette vérifiée"
            ));
            s.tenu_jusqu_a = Some(std::time::Instant::now() + tenue);
        }
        s.replie = Some(std::time::Instant::now());
        s.fenetre = None;
    }

    /// **Une prise neuve** : le fournisseur activé si sa carte a la place (ou
    /// n'est pas mesurée) ; carte pleine, le repli déclaré ; sans repli,
    /// l'activation quand même, et l'état le dit.
    fn prendre(&self, annule: &std::sync::atomic::AtomicBool) -> Result<Prise<C>, String> {
        let mesure = self.mesurer();
        match (mesure.as_ref().filter(|m| m.pleine()), &self.source.fallback) {
            (Some(m), super::Fallback::Service(adresse)) => {
                let prise = self.joindre_le_repli(adresse)?;
                self.dire_le_repli(adresse, &format!("{} à {} Mio libres, {} requis + {} de marge", m.carte, m.libres, m.requis, m.marge));
                Ok(prise)
            }
            (Some(m), _) => {
                let dit = format!("carte pleine ({} : {} Mio libres, {} requis), pas de repli déclaré", m.carte, m.libres, m.requis + m.marge);
                {
                    let mut s = self.suivi();
                    if s.etat.as_deref() != Some(dit.as_str()) {
                        s.dits.push(dit.clone());
                    }
                    s.etat = Some(dit);
                }
                self.activer_annulable(annule)
            }
            (None, _) => {
                {
                    let mut s = self.suivi();
                    if mesure.is_some() && s.etat.as_deref().is_some_and(|e| e.starts_with("carte pleine")) {
                        s.etat = None;
                    }
                }
                self.activer_annulable(annule)
            }
        }
    }

    /// **Replié, revenir quand la carte a la place** : après la fenêtre, la
    /// carte se remesure ; libre, le fournisseur déclaré est activé, et le
    /// retour se dit.
    fn peut_etre_revenir(&self, g: &mut Option<(Prise<C>, std::time::Instant)>) {
        let due = {
            let s = self.suivi();
            s.replie.is_some_and(|t| t.elapsed() >= self.fenetre())
                && s.tenu_jusqu_a.is_none_or(|t| std::time::Instant::now() >= t)
        };
        if !due {
            return;
        }
        match self.mesurer() {
            Some(m) if !m.pleine() => {
                // Le repli lâché avant d'activer.
                *g = None;
                match self.activer() {
                    Ok(prise) => {
                        *g = Some((prise, std::time::Instant::now()));
                        let mut s = self.suivi();
                        s.dits.push(format!("revenu à la carte {} à {}, {} Mio libres", m.carte, heure(), m.libres));
                        s.etat = None;
                        s.replie = None;
                        s.tenu_jusqu_a = None;
                        s.fenetre = None;
                    }
                    Err(e) => {
                        let mut s = self.suivi();
                        s.dits.push(format!("retour à la carte {} en échec ({e}) : le repli reste", m.carte));
                        s.replie = Some(std::time::Instant::now());
                    }
                }
            }
            _ => self.suivi().replie = Some(std::time::Instant::now()),
        }
    }

    /// **Un appel réussi, compté** par provenance ; sur la carte, son débit
    /// mesuré par fenêtre (la première fenêtre s'ouvre après le premier appel :
    /// l'activation chauffée), et le repli pris s'il tombe sous le seuil.
    fn compter(&self, g: &mut Option<(Prise<C>, std::time::Instant)>, textes: usize) {
        let Some(provenance) = g.as_ref().map(|(prise, _)| prise.provenance.clone()) else { return };
        let sur_le_repli = provenance.starts_with("service ");
        let mut s = self.suivi();
        *s.comptes.entry(provenance).or_default() += textes;
        let Some(min) = self.source.activation.as_ref().and_then(|a| a.min_throughput).filter(|m| *m > 0) else { return };
        let super::Fallback::Service(adresse) = &self.source.fallback else { return };
        if sur_le_repli {
            s.fenetre = None;
            return;
        }
        let fenetre = self.fenetre();
        if s.fenetre.is_none() {
            s.fenetre = Some((std::time::Instant::now(), 0));
            return;
        }
        let (ecoule, faits) = {
            let (debut, faits) = s.fenetre.as_mut().expect("la fenêtre ouverte");
            *faits += textes;
            (debut.elapsed(), *faits)
        };
        if ecoule < fenetre {
            return;
        }
        let debit = faits as f64 / ecoule.as_secs_f64();
        s.fenetre = Some((std::time::Instant::now(), 0));
        if debit >= min as f64 {
            return;
        }
        drop(s);
        let cause = format!("débit {} textes/s sous {min} pendant {} s", format!("{debit:.1}").replace('.', ","), fenetre.as_secs());
        match self.joindre_le_repli(adresse) {
            Ok(repli) => {
                // Le fournisseur de la carte lâché : son bail rendu, l'éviction fait le reste.
                *g = Some((repli, std::time::Instant::now()));
                self.dire_le_repli(adresse, &cause);
            }
            Err(e) => self.suivi().dits.push(format!("{cause} : {e} ; la carte reste")),
        }
    }

    fn lire_les_comptes(&self) -> Vec<(String, usize)> {
        std::mem::take(&mut self.suivi().comptes).into_iter().collect()
    }

    fn lire_les_dits(&self) -> Vec<String> {
        std::mem::take(&mut self.suivi().dits)
    }

    fn etat_du_repli(&self) -> Option<String> {
        self.suivi().etat.clone()
    }

    /// Un regard sur le client tenu, s'il l'est (ce qu'un maintenu dit de lui).
    fn regarder<R>(&self, f: impl Fn(&C) -> R) -> Option<R> {
        self.etat.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(p, _)| f(&p.client))
    }

    /// L'adresse où le fournisseur tenu répond (celle qu'il a prise, port 0 compris).
    fn adresse_active(&self) -> Option<String> {
        self.etat.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(p, _)| p.adresse.clone())
    }

    /// Un appel au fournisseur tenu : l'activer s'il ne l'est pas ; s'il est
    /// parti, le relancer une fois.
    fn appeler<R, E: std::fmt::Display>(&self, textes: usize, f: impl Fn(&C) -> Result<R, E>, parti: impl Fn(&E) -> bool, erreur: impl Fn(String) -> E) -> Result<R, E> {
        let mut g = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        self.peut_etre_revenir(&mut g);
        if g.is_none() {
            let tenu = self.prendre(&std::sync::atomic::AtomicBool::new(false)).map_err(|e| erreur(format!("activation à la demande : {e}")))?;
            *g = Some((tenu, std::time::Instant::now()));
        }
        let mut r = f(&g.as_ref().unwrap().0.client);
        if let Err(e) = &r {
            if parti(e) {
                let depart = e.to_string();
                let modele = &self.source.model;
                let (declare, quoi) = if self.maintenir {
                    (" déclaré maintenir", " — rouvrez le backend ou corrigez la déclaration")
                } else {
                    ("", "")
                };
                *g = None;
                noter_une_relance(&self.source);
                let relance = self.prendre(&std::sync::atomic::AtomicBool::new(false)).map_err(|e| erreur(format!(
                    "le fournisseur {modele}{declare} est parti ({depart}) ; relancé une fois, il n'a pas répondu : {e}{quoi}")))?;
                *g = Some((relance, std::time::Instant::now()));
                r = f(&g.as_ref().unwrap().0.client);
                if let Err(e) = &r {
                    if parti(e) {
                        let m = format!("le fournisseur {modele}{declare} est parti ({depart}) ; relancé une fois, il est reparti : {e}{quoi}");
                        r = Err(erreur(m));
                    }
                }
            }
        }
        if let Some((_, dernier)) = g.as_mut() {
            *dernier = std::time::Instant::now();
        }
        if r.is_ok() && textes > 0 {
            self.compter(&mut g, textes);
        }
        r
    }
}

#[cfg(feature = "daemon")]
impl<C: Send + 'static> Drop for Tenue<C> {
    fn drop(&mut self) {
        self.arret.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Le fournisseur n'est plus là : le message d'un appel qui n'a trouvé personne.
#[cfg(feature = "daemon")]
fn parti_dit(m: &str) -> bool {
    m.contains("ne répond pas") || m.contains("refused") || m.contains("Connection reset")
}

#[cfg(feature = "daemon")]
fn parti_calcul(e: &crate::embedder::EmbedError) -> bool {
    match e {
        crate::embedder::EmbedError::Timeout => true,
        crate::embedder::EmbedError::ProviderError(m) => parti_dit(m),
        _ => false,
    }
}

#[cfg(feature = "daemon")]
fn parti_ocr(e: &crate::ocr::OcrError) -> bool {
    matches!(e, crate::ocr::OcrError::Model(m) if parti_dit(m))
}

/// La seule adresse déclarée d'une source activée.
#[cfg(feature = "daemon")]
fn adresse_declaree(source: &super::ModelSource) -> Result<String, String> {
    source.addresses(super::Capability::Embed, &super::process_env).into_iter().next()
        .ok_or_else(|| "activation : une adresse explicite requise".to_string())
}

#[cfg(feature = "daemon")]
fn delai_appel(source: &super::ModelSource) -> std::time::Duration {
    source.timeout_secs.filter(|s| *s > 0).map(std::time::Duration::from_secs)
        .unwrap_or_else(crate::daemon::embeddings::default_embed_timeout)
}

/// **Le dense d'une activation** : un embarqueur tenu. Il dit son modèle et sa
/// dimension déclarés sans fournisseur ; ce que le client activé dit de lui
/// (factice, lot conseillé, troncatures) est gardé après qu'il est rendu.
#[cfg(feature = "daemon")]
struct DenseTenu {
    tenue: Tenue<crate::daemon::DaemonEmbedder>,
    dim: usize,
    vu: std::sync::Mutex<Option<(bool, Option<(usize, usize)>, Option<(usize, usize)>)>>,
}

#[cfg(feature = "daemon")]
impl crate::embedder::Embedder for DenseTenu {
    fn embed(&self, textes: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> {
        self.tenue.appeler(textes.len(), |c| {
            let r = c.embed(textes);
            if r.is_ok() {
                *self.vu.lock().unwrap_or_else(|e| e.into_inner()) = Some((c.is_mock(), c.budget_conseille(), c.troncatures()));
            }
            r
        }, parti_calcul, crate::embedder::EmbedError::ProviderError)
    }
    fn dim(&self) -> usize { self.dim }
    fn name(&self) -> &str { &self.tenue.source.model }
    fn is_mock(&self) -> bool { self.vu.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|(m, _, _)| *m) }
    fn is_absent(&self) -> bool { false }
    fn distant(&self) -> bool { true }
    fn budget_conseille(&self) -> Option<(usize, usize)> { self.vu.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|(_, b, _)| *b) }
    fn troncatures(&self) -> Option<(usize, usize)> { self.vu.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|(_, _, t)| *t) }
    fn timeout(&self) -> Option<std::time::Duration> { Some(delai_appel(&self.tenue.source)) }
    fn provenance_counts(&self) -> Vec<(String, usize)> { self.tenue.lire_les_comptes() }
    fn provenance_events(&self) -> Vec<String> { self.tenue.lire_les_dits() }
    fn provenance_state(&self) -> Option<String> { self.tenue.etat_du_repli() }
}

/// **Le creux d'une activation** (le groupe bge-m3) : un embarqueur creux
/// tenu. Son dual se déclare par `dimensions` (voir [`DualTenu`]) : un client
/// à la demande ne connaît pas l'identité du fournisseur avant son premier appel.
#[cfg(feature = "daemon")]
struct CreuxTenu(std::sync::Arc<Tenue<crate::daemon::DaemonEmbedder>>);

/// **Le dual d'une activation** : le creux qui déclare sa dimension rend aussi
/// un dual, sur **la même tenue** (un bail, une activation, une éviction).
#[cfg(feature = "daemon")]
struct DualTenu { tenue: std::sync::Arc<Tenue<crate::daemon::DaemonEmbedder>>, dim: usize }

#[cfg(feature = "daemon")]
impl crate::embedder::DualEmbedder for DualTenu {
    fn embed_dual(&self, textes: &[String]) -> Result<(Vec<Vec<f32>>, Vec<crate::sparse_index::SparseVector>), crate::embedder::EmbedError> {
        self.tenue.appeler(0, |c| c.embed_dual(textes), parti_calcul, crate::embedder::EmbedError::ProviderError)
    }
    fn dim(&self) -> usize { self.dim }
    fn distant(&self) -> bool { true }
}

#[cfg(feature = "daemon")]
impl crate::embedder::SparseEmbedder for CreuxTenu {
    fn embed_sparse(&self, textes: &[String]) -> Result<Vec<crate::sparse_index::SparseVector>, crate::embedder::EmbedError> {
        self.0.appeler(0, |c| c.embed_sparse(textes), parti_calcul, crate::embedder::EmbedError::ProviderError)
    }
    fn embed_sparse_spans(&self, textes: &[String]) -> Result<Vec<(crate::sparse_index::SparseVector, Vec<crate::sparse_index::SparseSpan>)>, crate::embedder::EmbedError> {
        self.0.appeler(0, |c| c.embed_sparse_spans(textes), parti_calcul, crate::embedder::EmbedError::ProviderError)
    }
    fn distant(&self) -> bool { true }
}

/// **Le relecteur d'une activation** : un relecteur tenu.
#[cfg(feature = "daemon")]
struct RelecteurTenu { tenue: Tenue<crate::daemon::embeddings::DaemonReranker>, nom: String }

#[cfg(feature = "daemon")]
impl crate::reranker::Reranker for RelecteurTenu {
    fn rerank(&self, requete: &str, passages: &[String]) -> Result<Vec<f32>, crate::embedder::EmbedError> {
        self.tenue.appeler(0, |c| c.rerank(requete, passages), parti_calcul, crate::embedder::EmbedError::ProviderError)
    }
    fn name(&self) -> &str { &self.nom }
}

/// **L'OCR d'une activation** : un OCR tenu.
#[cfg(feature = "daemon")]
struct OcrTenu { tenue: Tenue<crate::daemon::embeddings::DaemonOcr>, nom: String }

#[cfg(feature = "daemon")]
impl crate::ocr::Ocr for OcrTenu {
    fn recognize(&self, image: &crate::ocr::OcrImage) -> Result<crate::ocr::OcrOutput, crate::ocr::OcrError> {
        self.tenue.appeler(0, |c| c.recognize(image), parti_ocr, crate::ocr::OcrError::Model)
    }
    fn name(&self) -> &str { &self.nom }
}

/// **Le creux déclaré avec une activation** : à la demande ou maintenu.
#[cfg(feature = "daemon")]
pub(super) fn connecter_creux(source: &super::ModelSource) -> Result<(super::SparseClients, super::Origin), String> {
    let adresse = adresse_declaree(source)?;
    let tenue = Tenue::new(source, super::Capability::Sparse, |s, a| {
        crate::daemon::DaemonEmbedder::joindre(a).map(|d| d.with_timeout(delai_appel(s))).map_err(|e| e.to_string())
    }, &std::sync::atomic::AtomicBool::new(false))?;
    let tenue = std::sync::Arc::new(tenue);
    let dual = source.dimensions.filter(|d| *d > 0).map(|dim| {
        std::sync::Arc::new(DualTenu { tenue: tenue.clone(), dim }) as std::sync::Arc<dyn crate::embedder::DualEmbedder>
    });
    Ok((super::SparseClients { sparse: std::sync::Arc::new(CreuxTenu(tenue)), dual }, super::Origin::Service(adresse)))
}

/// **Le relecteur déclaré avec une activation.**
#[cfg(feature = "daemon")]
pub(super) fn connecter_relecteur(source: &super::ModelSource) -> Result<(std::sync::Arc<dyn crate::reranker::Reranker>, super::Origin), String> {
    let adresse = adresse_declaree(source)?;
    let tenue = Tenue::new(source, super::Capability::Rerank, |_, a| {
        crate::daemon::embeddings::DaemonReranker::joindre(a).map_err(|e| e.to_string())
    }, &std::sync::atomic::AtomicBool::new(false))?;
    Ok((std::sync::Arc::new(RelecteurTenu { tenue, nom: source.model.clone() }), super::Origin::Service(adresse)))
}

/// **L'OCR déclaré avec une activation.**
#[cfg(feature = "daemon")]
pub(super) fn connecter_ocr(source: &super::ModelSource) -> Result<(std::sync::Arc<dyn crate::ocr::Ocr>, super::Origin), String> {
    let adresse = adresse_declaree(source)?;
    let tenue = Tenue::new(source, super::Capability::Ocr, |_, a| {
        crate::daemon::embeddings::DaemonOcr::joindre(a).map_err(|e| e.to_string())
    }, &std::sync::atomic::AtomicBool::new(false))?;
    Ok((std::sync::Arc::new(OcrTenu { tenue, nom: source.model.clone() }), super::Origin::Service(adresse)))
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
                hote: None, adresse_distante: None, ssh: None, nom: None, modele: None, required_mib: None, margin_mib: None, min_throughput: None, window_secs: None, fallback_hold_secs: None,
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
            residence: Residence::ADemande { inactivite_secs: inactivite, rendre_le_bail_secs: RENDRE_LE_BAIL_PAR_DEFAUT_SECS.min(inactivite.saturating_sub(1).max(1)) },
            hote: Some(hote.clone()), adresse_distante: Some(format!("127.0.0.1:{port_distant}")), ssh: None, nom: None, modele: None, required_mib: None, margin_mib: None, min_throughput: None, window_secs: None, fallback_hold_secs: None,
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

    /// **4. Un fournisseur qui s'arrête en le disant est dit arrêté, avec sa
    /// cause** (lot du 503, c7 : « service d'embarquement arrêté : <cause> »
    /// au lieu d'« injoignable ») : la fixture rend un 503 nommé, écrit sa ligne
    /// d'arrêt au journal de l'activation et sort ; relancée, elle sort aussitôt.
    /// L'appel suivant dit la cause lue au journal.
    #[test]
    fn un_fournisseur_arrete_en_le_disant_est_dit_arrete_avec_sa_cause() {
        let mut f = a_la_demande(4, 2);
        let marqueur = f.dossier.path().join("carte-morte");
        f.source.activation.as_mut().unwrap().args.extend(["--arret-nomme".to_string(), marqueur.display().to_string()]);
        let (client, _) = connect_embedder(&f.source).unwrap();
        let e = client.embed(&["premier".into()]).err().expect("un 503 nommé").to_string();
        assert!(e.contains("calcul en panique"), "{e}");
        std::thread::sleep(std::time::Duration::from_millis(600));
        let e = client.embed(&["second".into()]).err().expect("le fournisseur est arrêté").to_string();
        assert!(
            e.contains("service d'embarquement arrêté : le calcul a paniqué (failed to reserve 4102146304 bytes) : serveur de la carte empoisonné"),
            "l'arrêt se dit avec sa cause : {e}"
        );
    }

    /// Le pid du fournisseur de la fixture démarré en `rang` (0 pour le premier).
    fn pid_demarre(f: &Fixture, rang: usize) -> i32 {
        let texte = std::fs::read_to_string(f.dossier.path().join("events.jsonl")).unwrap_or_default();
        texte.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| v["event"] == "constructeur_demarre").nth(rang)
            .and_then(|v| v["pid"].as_i64()).expect("fournisseur démarré") as i32
    }

    /// Tuer un fournisseur comme un plantage (SIGKILL), sans le réclamer : son
    /// parent (le client) garde la main sur son processus.
    fn planter(pid: i32) {
        unsafe { libc::kill(pid, libc::SIGKILL); }
        let debut = std::time::Instant::now();
        // Mort quand son /proc ne dit plus la fixture (zombie ou parti).
        while debut.elapsed() < std::time::Duration::from_secs(5) {
            let vivant = std::fs::read(format!("/proc/{pid}/cmdline")).map(|c| String::from_utf8_lossy(&c).contains("fournisseur.py")).unwrap_or(false);
            if !vivant { return; }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("le fournisseur {pid} ne meurt pas");
    }

    /// **Un fournisseur à la demande qui plante sous bail se relance une fois**
    /// (la reprise après plantage, « Manquant » 6 de la page des règles) : un
    /// index est une suite de calculs du même client ; le fournisseur tué entre
    /// deux, le suivant le relance, identité revérifiée, et rend ses vecteurs,
    /// vite, sans erreur pour l'appelant.
    #[test]
    fn a_la_demande_un_fournisseur_plante_sous_bail_se_relance_une_fois() {
        // Des délais longs : ni bail rendu ni éviction pendant le témoin.
        let f = a_la_demande(60, 30);
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["premier lot".into()]).unwrap();
        planter(pid_demarre(&f, 0));
        let debut = std::time::Instant::now();
        let v = client.embed(&["lot suivant".into()]).unwrap_or_else(|e| panic!("le calcul après le plantage : {e}"));
        assert_eq!(v[0].len(), 384);
        assert!(debut.elapsed() < std::time::Duration::from_secs(10), "relancé vite : {:?}", debut.elapsed());
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "relancé une fois");
        // Le relancé dit sa relance au /sante, comme sous `Maintenir`.
        assert_eq!(identite_du_fournisseur(&f)["relances"], 1, "le relancé dit sa relance au /sante");
        // Le relancé tient le bail : un troisième calcul ne relance rien.
        client.embed(&["encore".into()]).unwrap();
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "pas de relance sans plantage");
    }

    /// **Une relance qui échoue se refuse en le disant** : le fournisseur
    /// planté, sa relance n'arrive pas à se préparer (la fixture attend un
    /// fichier qui n'est plus là) ; le calcul rend une erreur qui dit que le
    /// fournisseur est parti et que la relance unique a échoué, dans le budget
    /// d'activation, sans seconde relance.
    #[test]
    fn a_la_demande_une_relance_qui_echoue_se_refuse_en_le_disant() {
        let mut f = a_la_demande(60, 30);
        let liberer = f.dossier.path().join("liberer-chargement");
        std::fs::write(&liberer, b"").unwrap();
        let a = f.source.activation.as_mut().unwrap();
        a.args.extend(["--liberer-chargement".into(), liberer.display().to_string(), "--budget-pret-ms".into(), "1500".into()]);
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["premier lot".into()]).unwrap();
        std::fs::remove_file(&liberer).unwrap();
        planter(pid_demarre(&f, 0));
        let debut = std::time::Instant::now();
        let erreur = client.embed(&["lot suivant".into()]).err().expect("la relance échoue").to_string();
        assert!(debut.elapsed() < std::time::Duration::from_secs(15), "refus dans le budget : {:?}", debut.elapsed());
        assert!(erreur.contains("parti") && erreur.contains("relancé une fois"), "le refus dit le plantage et la relance : {erreur}");
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "une seule relance");
    }

    /// L'identité du fournisseur de la fixture, par l'adresse que son dossier
    /// de coordination retient (port 0 annoncé).
    fn identite_du_fournisseur(f: &Fixture) -> serde_json::Value {
        let a = f.source.activation.as_ref().unwrap();
        let adresse = crate::serveur::activation::adresse_retenue(&a.coordination, &f.source.address.0[0]).expect("adresse retenue");
        crate::daemon::identite_de::<serde_json::Value>(&crate::daemon::agent_with_timeout(std::time::Duration::from_secs(2)), &adresse, crate::daemon::embeddings::SERVICE)
            .unwrap_or_else(|e| panic!("identité du fournisseur à {adresse} : {e}"))
    }

    /// **Un fournisseur déclaré `Maintenir` qui plante se relance une fois**
    /// (décision de c7, 14 novembre 2026 : « maintenir » veut dire tenir le
    /// fournisseur debout) : le calcul suivant le relance, identité revérifiée,
    /// et le relancé dit `relances: 1` à son /sante — un fournisseur qui meurt
    /// en boucle se voit sans lire un journal.
    #[test]
    fn maintenir_un_fournisseur_plante_se_relance_une_fois_et_le_compte() {
        let f = Fixture::nouvelle();
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["premier lot".into()]).unwrap();
        let au_lancement = identite_du_fournisseur(&f)["relances"].clone();
        planter(pid_demarre(&f, 0));
        let debut = std::time::Instant::now();
        let v = client.embed(&["lot suivant".into()]).unwrap_or_else(|e| panic!("le calcul après le plantage : {e}"));
        assert_eq!(v[0].len(), 384);
        assert!(debut.elapsed() < std::time::Duration::from_secs(10), "relancé vite : {:?}", debut.elapsed());
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "relancé une fois");
        assert_eq!(au_lancement, 0, "lancé par l'activation, jamais relancé : relances 0");
        assert_eq!(identite_du_fournisseur(&f)["relances"], 1, "le relancé dit sa relance");
        client.embed(&["encore".into()]).unwrap();
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "pas de relance sans plantage");
    }

    /// **Sa relance qui échoue se refuse en le disant** : déclaré maintenir,
    /// parti, relancé une fois, pas répondu — et quoi faire.
    #[test]
    fn maintenir_une_relance_qui_echoue_se_refuse_en_le_disant() {
        let mut f = Fixture::nouvelle();
        let liberer = f.dossier.path().join("liberer-chargement");
        std::fs::write(&liberer, b"").unwrap();
        let a = f.source.activation.as_mut().unwrap();
        a.args.extend(["--liberer-chargement".into(), liberer.display().to_string(), "--budget-pret-ms".into(), "1500".into()]);
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["premier lot".into()]).unwrap();
        std::fs::remove_file(&liberer).unwrap();
        planter(pid_demarre(&f, 0));
        let debut = std::time::Instant::now();
        let erreur = client.embed(&["lot suivant".into()]).err().expect("la relance échoue").to_string();
        assert!(debut.elapsed() < std::time::Duration::from_secs(15), "refus dans le budget : {:?}", debut.elapsed());
        for mot in ["déclaré maintenir", "parti", "relancé une fois", "rouvrez"] {
            assert!(erreur.contains(mot), "le refus dit « {mot} » : {erreur}");
        }
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "une seule relance");
    }

    /// **Le groupe** (décision de c7, 14 novembre 2026 : une activation nommée,
    /// citée par chaque déclaration) : le creux, le relecteur et l'OCR servis
    /// par un même fournisseur, déclaré une fois sous le nom « groupe », chacun
    /// à son modèle, à la même adresse et la même coordination.
    fn groupe(inactivite: u64, rendre: u64) -> (Fixture, ModelSource, ModelSource, ModelSource) {
        let f = Fixture::nouvelle();
        let d = f.dossier.path();
        let bloc = serde_json::json!({
            "nom": "groupe", "modele": "fixture-groupe",
            "programme": concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py"),
            "args": ["--journal", d.join("events.jsonl").display().to_string(), "--generation", "groupe", "--modele", "fixture-groupe", "--sparse"],
            "env": {"RAG3WEAVER_RERANK_MODEL": "fixture-relecteur", "RAG3WEAVER_OCR_MODEL": "fixture-ocr"},
            "profil": "fixture-cpu", "peripherique": "cpu",
            "coordination": d.join("coordination").display().to_string(), "journal": d.join("journaux").display().to_string(),
            "budget_activation_secs": 5, "budget_attente_secs": 5,
            "residence": {"a_demande": {"inactivite_secs": inactivite, "rendre_le_bail_secs": rendre}},
        });
        let source = |modele: &str| -> ModelSource {
            serde_json::from_value(serde_json::json!({
                "provider": "service", "model": modele, "address": "127.0.0.1:0", "timeout_secs": 2, "activation": bloc.clone(),
            })).unwrap_or_else(|e| panic!("la déclaration du groupe ({modele}) : {e}"))
        };
        let (creux, relecteur, ocr) = (source("fixture-groupe"), source("fixture-relecteur"), source("fixture-ocr"));
        (f, creux, relecteur, ocr)
    }

    fn une_image() -> crate::ocr::OcrImage {
        crate::ocr::OcrImage::from_rgb(2, 2, vec![255; 12]).unwrap()
    }

    /// **Ouvrir le groupe ne lance rien** : trois capacités déclarées à la
    /// demande rendent leurs clients sans fournisseur.
    #[test]
    fn groupe_ouvrir_trois_capacites_ne_lance_rien() {
        let (f, creux, relecteur, ocr) = groupe(2, 1);
        crate::model_source::connect_sparse(&creux).unwrap_or_else(|e| panic!("le creux à la demande : {e}"));
        crate::model_source::connect_reranker(&relecteur).unwrap_or_else(|e| panic!("le relecteur à la demande : {e}"));
        crate::model_source::connect_ocr(&ocr).unwrap_or_else(|e| panic!("l'OCR à la demande : {e}"));
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert_eq!(evenements(&f, "constructeur_demarre"), 0, "ouvrir le groupe ne lance pas le fournisseur");
    }

    /// **Un relecteur lance le groupe, l'OCR ne le relance pas** : un seul
    /// fournisseur, deux baux, chacun le sien.
    #[test]
    fn groupe_un_relecteur_lance_l_ocr_ne_relance_pas() {
        let (f, _, relecteur, ocr) = groupe(60, 30);
        let (r, _) = crate::model_source::connect_reranker(&relecteur).unwrap_or_else(|e| panic!("{e}"));
        let (o, _) = crate::model_source::connect_ocr(&ocr).unwrap_or_else(|e| panic!("{e}"));
        let scores = r.rerank("chat noir", &["un chat noir".into(), "une voiture".into()]).unwrap_or_else(|e| panic!("relire : {e}"));
        assert_eq!(scores.len(), 2);
        assert!(scores[0] > scores[1], "le relecteur de la fixture classe par mots communs : {scores:?}");
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "le relecteur a lancé le groupe");
        let lu = o.recognize(&une_image()).unwrap_or_else(|e| panic!("lire l'image : {e}"));
        assert_eq!(lu.lines.len(), 1);
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "l'OCR s'attache au même fournisseur");
        assert_eq!(identite_du_fournisseur(&f)["usage"]["baux"], 2, "deux baux, un par capacité");
    }

    /// **Le groupe s'évince quand plus aucun bail ne le tient**, puis **un
    /// appel d'après l'éviction le relance**.
    #[test]
    fn groupe_s_evince_quand_plus_aucun_bail_puis_se_relance() {
        let (f, creux, relecteur, _) = groupe(2, 1);
        let (r, _) = crate::model_source::connect_reranker(&relecteur).unwrap_or_else(|e| panic!("{e}"));
        let (c, _) = crate::model_source::connect_sparse(&creux).unwrap_or_else(|e| panic!("{e}"));
        r.rerank("a", &["a".into()]).unwrap_or_else(|e| panic!("relire : {e}"));
        c.sparse.embed_sparse(&["un texte".into()]).unwrap_or_else(|e| panic!("le creux : {e}"));
        assert_eq!(evenements(&f, "constructeur_demarre"), 1);
        assert!(attendre_l_evenement(&f, "evince", 1, std::time::Duration::from_secs(15)), "évincé quand plus aucun bail");
        assert!(attendre_l_evenement(&f, "arrete", 1, std::time::Duration::from_secs(5)));
        let v = c.sparse.embed_sparse(&["après".into()]).unwrap_or_else(|e| panic!("le creux après l'éviction : {e}"));
        assert_eq!(v.len(), 1);
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "relancé une fois");
    }

    /// **Le groupe planté sous bail se relance une fois** et le dit au /sante.
    #[test]
    fn groupe_plante_sous_bail_se_relance_une_fois() {
        let (f, _, relecteur, _) = groupe(60, 30);
        let (r, _) = crate::model_source::connect_reranker(&relecteur).unwrap_or_else(|e| panic!("{e}"));
        r.rerank("a", &["a".into()]).unwrap_or_else(|e| panic!("relire : {e}"));
        planter(pid_demarre(&f, 0));
        let scores = r.rerank("a", &["a".into(), "b".into()]).unwrap_or_else(|e| panic!("relire après le plantage : {e}"));
        assert_eq!(scores.len(), 2);
        assert_eq!(evenements(&f, "constructeur_demarre"), 2, "relancé une fois");
        assert_eq!(identite_du_fournisseur(&f)["relances"], 1);
    }

    /// Un creux du groupe à la demande, déclaré avec `dimensions` (le dual se
    /// déclare : décision de c7, 14 novembre 2026), sur une fixture qui sait le
    /// dual ou non, à la dimension `dim_servie`.
    fn creux_dual(dual: bool, dimensions: u64) -> (Fixture, ModelSource) {
        let f = Fixture::nouvelle();
        let d = f.dossier.path();
        let mut args = vec!["--journal".to_string(), d.join("events.jsonl").display().to_string(), "--generation".into(), "dual".into(),
            "--modele".into(), "fixture-groupe".into(), "--sparse".into()];
        if dual { args.push("--dual".into()); }
        let source: ModelSource = serde_json::from_value(serde_json::json!({
            "provider": "service", "model": "fixture-groupe", "address": "127.0.0.1:0", "timeout_secs": 2, "dimensions": dimensions,
            "activation": {
                "nom": "groupe", "modele": "fixture-groupe",
                "programme": concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py"), "args": args,
                "profil": "fixture-cpu", "peripherique": "cpu",
                "coordination": d.join("coordination").display().to_string(), "journal": d.join("journaux").display().to_string(),
                "budget_activation_secs": 5, "budget_attente_secs": 5,
                "residence": {"a_demande": {"inactivite_secs": 60, "rendre_le_bail_secs": 30}},
            },
        })).unwrap_or_else(|e| panic!("la déclaration du creux dual : {e}"));
        (f, source)
    }

    /// **Un creux à la demande qui déclare sa dimension rend un dual** (ticket
    /// `2026-11-14-un-creux-a-la-demande-n-a-pas-de-dual`), et l'ouvrir ne lance rien.
    #[test]
    fn dual_un_creux_qui_declare_sa_dimension_rend_un_dual_sans_lancer() {
        let (f, creux) = creux_dual(true, 384);
        let (clients, _) = crate::model_source::connect_sparse(&creux).unwrap_or_else(|e| panic!("{e}"));
        let dual = clients.dual.expect("un creux qui déclare dimensions rend un dual");
        assert_eq!(dual.dim(), 384, "la dimension déclarée, sans fournisseur");
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert_eq!(evenements(&f, "constructeur_demarre"), 0, "ouvrir ne lance rien");
    }

    /// **Le dual et le creux partagent une tenue** : un lancement, un bail.
    #[test]
    fn dual_puis_creux_un_lancement_un_bail() {
        let (f, creux) = creux_dual(true, 384);
        let (clients, _) = crate::model_source::connect_sparse(&creux).unwrap_or_else(|e| panic!("{e}"));
        let dual = clients.dual.expect("le dual");
        let (dense, creux_v) = dual.embed_dual(&["un texte".into()]).unwrap_or_else(|e| panic!("le dual : {e}"));
        assert_eq!((dense.len(), dense[0].len(), creux_v.len()), (1, 384, 1));
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "le dual a lancé le groupe");
        assert_eq!(identite_du_fournisseur(&f)["usage"]["baux"], 1, "un bail pour le dual");
        clients.sparse.embed_sparse(&["un autre".into()]).unwrap_or_else(|e| panic!("le creux : {e}"));
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "le creux ne relance pas");
        assert_eq!(identite_du_fournisseur(&f)["usage"]["baux"], 1, "le creux ne prend pas de second bail : même tenue");
    }

    /// **Un fournisseur sans le dual, ou d'une autre dimension, se refuse en le disant.**
    #[test]
    fn dual_un_fournisseur_sans_dual_ou_d_une_autre_dimension_se_refuse() {
        let (_f, creux) = creux_dual(false, 384);
        let (clients, _) = crate::model_source::connect_sparse(&creux).unwrap_or_else(|e| panic!("{e}"));
        let e = clients.dual.expect("le dual déclaré").embed_dual(&["x".into()]).err().expect("sans dual : refus").to_string();
        assert!(e.contains("sans le dual"), "le refus le dit : {e}");
        let (_f, creux) = creux_dual(true, 768);
        let (clients, _) = crate::model_source::connect_sparse(&creux).unwrap_or_else(|e| panic!("{e}"));
        let e = clients.dual.expect("le dual déclaré").embed_dual(&["x".into()]).err().expect("autre dimension : refus").to_string();
        assert!(e.contains("768") && e.contains("384"), "le refus dit les deux dimensions : {e}");
    }

    #[test]
    fn budget_de_sonde_ne_remplace_pas_delai_appel_par_defaut() {
        let mut f = Fixture::nouvelle(); f.source.timeout_secs = None;
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.timeout(), Some(crate::daemon::embeddings::default_embed_timeout()));
    }

    // ─── Le repli vers un service quand la carte est pleine (15 novembre 2026) ───
    // Conception : docs/embarquement/13-11-2026/01-activation-et-residence-livre-et-manquant.md,
    // « Le repli vers un service quand la carte est pleine », relue par c7. Une
    // carte factice est un dossier `<racine>/<carte>/device/mem_info_vram_{total,used}`,
    // la racine donnée au fil du témoin.

    const CARTE: &str = "carte-factice";
    const TOTAL_MIO: u64 = 24_576;

    fn ecrire_la_carte(f: &Fixture, libres_mio: u64) {
        let device = f.dossier.path().join("drm").join(CARTE).join("device");
        std::fs::create_dir_all(&device).unwrap();
        std::fs::write(device.join("mem_info_vram_total"), format!("{}\n", TOTAL_MIO << 20)).unwrap();
        std::fs::write(device.join("mem_info_vram_used"), format!("{}\n", (TOTAL_MIO - libres_mio) << 20)).unwrap();
    }

    /// Une déclaration à la demande sur la carte factice, qui exige 11 500 Mio
    /// (la marge par défaut, 2 048, s'y ajoute), remesurée chaque seconde.
    fn sur_carte(libres_mio: u64) -> Fixture {
        let mut f = a_la_demande(4, 2);
        ecrire_la_carte(&f, libres_mio);
        let racine = f.dossier.path().join("drm");
        super::RACINE_DRM_DU_FIL.with(|r| *r.borrow_mut() = Some(racine));
        let a = f.source.activation.as_mut().unwrap();
        a.peripherique = CARTE.into();
        a.required_mib = Some(11_500);
        a.window_secs = Some(1);
        f
    }

    /// **Un service posé à la main**, comme le 7879 : la fixture lancée hors de
    /// toute activation, qui ne dit donc pas de recette. Tué à la fin du témoin.
    struct ServicePose { enfant: std::process::Child, adresse: String }
    impl Drop for ServicePose {
        fn drop(&mut self) {
            let _ = self.enfant.kill();
            let _ = self.enfant.wait();
        }
    }

    fn poser_un_service(f: &Fixture, modele: &str) -> ServicePose {
        let journal = f.dossier.path().join(format!("pose-{modele}.jsonl"));
        let mut commande = std::process::Command::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/residence/fournisseur.py"));
        commande.args(["--adresse", "127.0.0.1:0", "--generation", "pose-a-la-main", "--modele", modele, "--journal"]).arg(&journal)
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        for v in ["RAG3WEAVER_ACTIVATION_PROFIL", "RAG3WEAVER_ACTIVATION_PERIPHERIQUE", "RAG3WEAVER_ACTIVATION_IDENTITE", "RAG3WEAVER_EVICTION_INACTIVITE_S"] {
            commande.env_remove(v);
        }
        let enfant = commande.spawn().expect("la fixture posée à la main");
        let debut = std::time::Instant::now();
        while debut.elapsed() < std::time::Duration::from_secs(10) {
            let texte = std::fs::read_to_string(&journal).unwrap_or_default();
            if let Some(adresse) = texte.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .find(|v| v["event"] == "pret").and_then(|v| v["adresse"].as_str().map(String::from)) {
                return ServicePose { enfant, adresse };
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("la fixture posée à la main n'a pas dit prêt : {}", std::fs::read_to_string(&journal).unwrap_or_default());
    }

    fn replier_vers(f: &mut Fixture, pose: &ServicePose) {
        f.source.fallback = super::super::Fallback::Service(pose.adresse.clone());
    }

    /// **1. La carte a la place** : le fournisseur se lance comme avant, et ses
    /// vecteurs viennent de la carte ; rien à dire.
    #[test]
    fn repli_carte_libre_le_fournisseur_se_lance() {
        let mut f = sur_carte(20_000);
        f.source.fallback = super::super::Fallback::Service("127.0.0.1:9".into());
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.embed(&["sur la carte".into()]).unwrap()[0].len(), 384);
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "la carte a la place : le fournisseur se lance");
        assert_eq!(client.provenance_counts(), vec![(format!("carte {CARTE}"), 1)]);
        assert!(client.provenance_events().is_empty());
        assert_eq!(client.provenance_state(), None);
    }

    /// **2. La carte est pleine, un repli est déclaré** : rien ne se lance ; les
    /// vecteurs viennent du service, et le repli se dit avec sa cause. Le service
    /// posé à la main ne dit pas de recette : l'état le dit.
    #[test]
    fn repli_carte_pleine_sert_par_le_service_et_le_dit() {
        let mut f = sur_carte(1_000);
        let pose = poser_un_service(&f, "fixture-residence-384");
        replier_vers(&mut f, &pose);
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.embed(&["par le repli".into()]).unwrap()[0].len(), 384);
        assert_eq!(evenements(&f, "constructeur_demarre"), 0, "la carte pleine : rien n'est lancé");
        assert_eq!(client.provenance_counts(), vec![(format!("service {}", pose.adresse), 1)]);
        let dits = client.provenance_events();
        assert!(dits.iter().any(|e| e.contains(&format!("replié vers {}", pose.adresse))
            && e.contains(&format!("{CARTE} à 1000 Mio libres, 11500 requis + 2048 de marge"))), "{dits:?}");
        let etat = client.provenance_state().expect("l'état dit le repli");
        assert!(etat.contains(&format!("replié vers {}", pose.adresse)) && etat.contains("repli sans recette vérifiée"), "{etat}");
    }

    /// **3. La carte se libère** : au calcul qui suit la fenêtre, le fournisseur
    /// déclaré est activé, et le retour se dit.
    #[test]
    fn repli_carte_liberee_le_fournisseur_revient_et_le_dit() {
        let mut f = sur_carte(1_000);
        let pose = poser_un_service(&f, "fixture-residence-384");
        replier_vers(&mut f, &pose);
        f.source.activation.as_mut().unwrap().fallback_hold_secs = Some(1);
        let (client, _) = connect_embedder(&f.source).unwrap();
        client.embed(&["replié".into()]).unwrap();
        assert_eq!(evenements(&f, "constructeur_demarre"), 0);
        let _ = (client.provenance_counts(), client.provenance_events());
        ecrire_la_carte(&f, 20_000);
        std::thread::sleep(std::time::Duration::from_millis(1_300));
        assert_eq!(client.embed(&["revenu".into()]).unwrap()[0].len(), 384);
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "la carte libre : le fournisseur déclaré est activé");
        assert_eq!(client.provenance_counts(), vec![(format!("carte {CARTE}"), 1)]);
        let dits = client.provenance_events();
        assert!(dits.iter().any(|e| e.contains(&format!("revenu à la carte {CARTE}")) && e.contains("20000 Mio libres")), "{dits:?}");
        assert_eq!(client.provenance_state(), None, "revenu : plus rien à dire");
    }

    /// **4. La carte est pleine, sans repli** : l'activation se fait comme avant,
    /// et l'état dit la carte pleine et l'absence de repli.
    #[test]
    fn repli_carte_pleine_sans_repli_active_et_le_dit() {
        let f = sur_carte(1_000);
        let (client, _) = connect_embedder(&f.source).unwrap();
        assert_eq!(client.embed(&["sans repli".into()]).unwrap()[0].len(), 384);
        assert_eq!(evenements(&f, "constructeur_demarre"), 1);
        let attendu = format!("carte pleine ({CARTE} : 1000 Mio libres, 13548 requis), pas de repli déclaré");
        assert_eq!(client.provenance_state().as_deref(), Some(attendu.as_str()));
        let dits = client.provenance_events();
        assert!(dits.iter().any(|e| e.contains(&attendu)), "{dits:?}");
        assert_eq!(client.provenance_counts(), vec![(format!("carte {CARTE}"), 1)]);
    }

    /// **5. Un repli qui ne sert pas le modèle** : refusé, nommé ; rien n'est lancé.
    #[test]
    fn repli_d_un_autre_modele_est_refuse_nomme() {
        let mut f = sur_carte(1_000);
        let pose = poser_un_service(&f, "autre-modele");
        replier_vers(&mut f, &pose);
        let (client, _) = connect_embedder(&f.source).unwrap();
        let erreur = client.embed(&["refusé".into()]).err().expect("un repli d'un autre modèle est refusé").to_string();
        assert!(erreur.contains(&format!("repli {}", pose.adresse)) && erreur.contains("autre-modele")
            && erreur.contains("fixture-residence-384"), "{erreur}");
        assert_eq!(evenements(&f, "constructeur_demarre"), 0);
    }

    /// **La tenue du repli** (`fallback_hold_secs`, décision de c7) : la carte
    /// libérée avant la fin de la tenue, le retour est refusé ; après, il se
    /// fait. L'état dit jusqu'à quand le repli tient au moins.
    #[test]
    fn repli_tenu_ne_revient_pas_avant_sa_tenue() {
        let mut f = sur_carte(1_000);
        let pose = poser_un_service(&f, "fixture-residence-384");
        replier_vers(&mut f, &pose);
        f.source.activation.as_mut().unwrap().fallback_hold_secs = Some(3);
        let (client, _) = connect_embedder(&f.source).unwrap();
        let debut = std::time::Instant::now();
        client.embed(&["replié".into()]).unwrap();
        let etat = client.provenance_state().expect("l'état dit le repli");
        assert!(etat.contains("au plus tôt"), "l'état dit la tenue : {etat}");
        ecrire_la_carte(&f, 20_000);
        std::thread::sleep(std::time::Duration::from_millis(1_300));
        client.embed(&["tenu".into()]).unwrap();
        assert_eq!(evenements(&f, "constructeur_demarre"), 0, "avant la tenue, la carte libre ne reprend pas");
        assert_eq!(client.provenance_counts(), vec![(format!("service {}", pose.adresse), 2)]);
        std::thread::sleep(std::time::Duration::from_secs(3).saturating_sub(debut.elapsed()) + std::time::Duration::from_millis(300));
        client.embed(&["après la tenue".into()]).unwrap();
        assert_eq!(evenements(&f, "constructeur_demarre"), 1, "après la tenue, le fournisseur revient");
        assert_eq!(client.provenance_counts(), vec![(format!("carte {CARTE}"), 1)]);
    }

    /// **6. Le débit sous le seuil pendant la fenêtre** : la fixture ralentie
    /// (`--lent-ms 400`, un texte par appel : 2,5 textes/s, sous 20),
    /// l'embarqueur se replie, et la cause dit le débit.
    #[test]
    fn repli_un_debit_sous_le_seuil_replie_et_dit_sa_cause() {
        let mut f = sur_carte(20_000);
        let pose = poser_un_service(&f, "fixture-residence-384");
        replier_vers(&mut f, &pose);
        let a = f.source.activation.as_mut().unwrap();
        a.args.extend(["--lent-ms".to_string(), "400".to_string()]);
        a.min_throughput = Some(20);
        let (client, _) = connect_embedder(&f.source).unwrap();
        let service = format!("service {}", pose.adresse);
        let debut = std::time::Instant::now();
        let mut vus: Vec<(String, usize)> = Vec::new();
        while debut.elapsed() < std::time::Duration::from_secs(8) && !vus.iter().any(|(p, _)| *p == service) {
            client.embed(&["lent".into()]).unwrap();
            vus.extend(client.provenance_counts());
        }
        assert!(vus.iter().any(|(p, _)| *p == format!("carte {CARTE}")), "d'abord la carte : {vus:?}");
        assert!(vus.iter().any(|(p, _)| *p == service), "puis le repli : {vus:?}");
        let dits = client.provenance_events();
        assert!(dits.iter().any(|e| e.contains(&format!("replié vers {}", pose.adresse)) && e.contains("débit")
            && e.contains("sous 20")), "{dits:?}");
    }
}
