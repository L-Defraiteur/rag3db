//! **Un modèle, en service ou en local** — la déclaration commune à toute
//! capacité (`docs/3-octobre-2026-21h22/01-un-modele-en-service-ou-en-local.md`).
//!
//! Lucie, le 3 octobre 2026 : « trouver une abstraction pour que les modèles
//! puissent à chaque fois être déclarés en service ou en local […] que ce
//! soit simple de refaire la même chose avec l'OCR, ou TTS/STT ».
//!
//! # Ce que ce module sait, et ce qu'il ne sait pas
//!
//! Il sait **où vit un modèle** : dans ce processus, chez un démon à nous,
//! chez un service tiers ; à quelle adresse ; et quoi faire s'il ne répond
//! pas. Il ne sait rien de ce qu'un modèle reçoit ni de ce qu'il rend : la
//! frontière passe au trait de chaque capacité (`Embedder`, `Reranker`,
//! `Ocr`…). Une capacité lui donne trois façons de construire son client —
//! local, de service, compatible — et [`resolve`] choisit, refuse ou se
//! replie, et rend avec le client **d'où vient le calcul** ([`Origin`]).
//!
//! # Le repli se déclare
//!
//! Le défaut est [`Fallback::Refuse`] : un service injoignable est une erreur
//! qui dit ce que chaque adresse sert. [`Fallback::Local`] ne s'obtient qu'en
//! l'écrivant. Jamais de repli silencieux sur la carte qui porte l'écran.

use serde::{Deserialize, Serialize};

/// Ce qu'un modèle sait faire. Le nom est celui de la clé du manifeste
/// (`models.<nom>`) et de la variable d'environnement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Embed,
    Sparse,
    Rerank,
    Ocr,
    Decide,
    Llm,
}

impl Capability {
    pub fn name(self) -> &'static str {
        match self {
            Self::Embed => "embed",
            Self::Sparse => "sparse",
            Self::Rerank => "rerank",
            Self::Ocr => "ocr",
            Self::Decide => "decide",
            Self::Llm => "llm",
        }
    }

    /// `RAG3WEAVER_SERVICE_<CAPACITÉ>` : une adresse, ou plusieurs séparées
    /// par des virgules.
    pub fn variable(self) -> String {
        format!("RAG3WEAVER_SERVICE_{}", self.name().to_ascii_uppercase())
    }

    /// Les variables lues, dans l'ordre : la commune, puis l'alias
    /// d'avant — `RAG3WEAVER_EMBED_SERVICE`, que des passes en cours posent.
    ///
    /// Le creux est servi par le même démon que le dense : sans variable à
    /// lui, il lit celles de l'embarquement — un poste qui a posé
    /// `RAG3WEAVER_SERVICE_EMBED` n'a rien à poser de plus.
    pub fn variables(self) -> Vec<String> {
        let mut v = vec![self.variable()];
        if self == Self::Sparse {
            v.push(Self::Embed.variable());
        }
        if matches!(self, Self::Embed | Self::Sparse) {
            v.push("RAG3WEAVER_EMBED_SERVICE".to_string());
        }
        v
    }
}

/// Où vit le modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Notre moteur, dans ce processus.
    Local,
    /// Un démon à nous. `daemon` est le nom d'avant, gardé.
    #[default]
    #[serde(alias = "daemon")]
    Service,
    /// Un service tiers, par son protocole.
    Compatible,
}

/// Quoi faire quand le service ne répond pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fallback {
    #[default]
    Refuse,
    Local,
}

/// Une adresse, ou plusieurs — au manifeste, une chaîne ou une liste.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "AddressesRepr", into = "AddressesRepr")]
pub struct Addresses(pub Vec<String>);

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum AddressesRepr {
    One(String),
    Many(Vec<String>),
}

impl From<AddressesRepr> for Addresses {
    fn from(r: AddressesRepr) -> Self {
        match r {
            AddressesRepr::One(s) => Self::parse(&s),
            AddressesRepr::Many(v) => Self(v.iter().flat_map(|s| Self::parse(s).0).collect()),
        }
    }
}

impl From<Addresses> for AddressesRepr {
    fn from(a: Addresses) -> Self {
        match a.0.len() {
            1 => Self::One(a.0.into_iter().next().unwrap_or_default()),
            _ => Self::Many(a.0),
        }
    }
}

impl Addresses {
    /// `a, b` → deux adresses ; le vide n'en est pas une.
    pub fn parse(s: &str) -> Self {
        Self(s.split(',').map(str::trim).filter(|a| !a.is_empty()).map(String::from).collect())
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// **La déclaration d'un modèle**, la même pour toute capacité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSource {
    #[serde(default)]
    pub provider: Provider,
    /// Le modèle attendu. Un service à nous dit ce qu'il sert ; s'il sert
    /// autre chose, c'est un refus.
    pub model: String,
    /// Absente : elle vient de l'environnement ([`Capability::variables`]).
    #[serde(default, skip_serializing_if = "Addresses::is_empty")]
    pub address: Addresses,
    /// Pour `compatible` : `openai`, `llama_server`, `vertex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Le **nom** de la variable qui porte la clé, jamais la clé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub fallback: Fallback,
    /// La dimension des vecteurs, pour les capacités qui en rendent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
}

impl ModelSource {
    pub fn local(model: impl Into<String>) -> Self {
        Self { provider: Provider::Local, model: model.into(), address: Addresses::default(), protocol: None, api_key_env: None, fallback: Fallback::Refuse, dimensions: None }
    }
    pub fn service(model: impl Into<String>) -> Self {
        Self { provider: Provider::Service, ..Self::local(model) }
    }

    /// Les adresses à essayer : celles de la déclaration, sinon celles de
    /// l'environnement. `env` : la lecture d'une variable, en paramètre pour
    /// qu'un test ne touche pas au processus.
    pub fn addresses(&self, capability: Capability, env: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
        if !self.address.is_empty() {
            return self.address.0.clone();
        }
        capability.variables().iter().find_map(|v| env(v).map(|s| Addresses::parse(&s)).filter(|a| !a.is_empty())).map(|a| a.0).unwrap_or_default()
    }
}

/// **D'où vient le calcul.** C'est ce que lisent le régulateur de rafale, la
/// classe de carte et l'estimation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Ce processus : la carte d'ici.
    Local,
    /// Un démon à nous, à cette adresse.
    Service(String),
    /// Un service tiers, à cette adresse.
    Compatible(String),
}

impl Origin {
    /// Le calcul se fait-il ailleurs que sur la carte d'ici ?
    pub fn remote(&self) -> bool {
        !matches!(self, Self::Local)
    }
    /// L'étiquette sous laquelle un débit se range.
    pub fn label(&self) -> &'static str {
        if self.remote() {
            "service"
        } else {
            "local"
        }
    }
}

/// Ce qu'une capacité sait construire. Chaque constructeur est facultatif :
/// une capacité qui n'a pas de forme de service le dit en ne le donnant pas,
/// et la déclaration qui la demande est refusée avec cette raison.
pub struct Builders<'a, T> {
    /// Le client local, pour ce modèle.
    pub local: Option<&'a dyn Fn(&ModelSource) -> Result<T, String>>,
    /// Le client d'un démon à nous, à cette adresse, et **le modèle qu'il
    /// sert** — pour choisir parmi plusieurs adresses.
    pub service: Option<&'a dyn Fn(&str) -> Result<(T, String), String>>,
    /// Le client d'un service tiers, à cette adresse.
    pub compatible: Option<&'a dyn Fn(&str, &ModelSource) -> Result<T, String>>,
}

/// La variable d'environnement, lue pour de vrai.
pub fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// **Résout une déclaration en un client**, et dit d'où vient le calcul.
pub fn resolve<T>(
    capability: Capability,
    source: &ModelSource,
    builders: &Builders<T>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<(T, Origin), String> {
    let what = format!("models.{} ({})", capability.name(), source.model);
    let local = |why: &str| -> Result<(T, Origin), String> {
        match builders.local {
            Some(build) => build(source).map(|c| (c, Origin::Local)).map_err(|e| format!("{what} : {why}local : {e}")),
            None => Err(format!("{what} : {why}cette capacité n'a pas de forme locale dans ce binaire")),
        }
    };
    let remote = match source.provider {
        Provider::Local => return local(""),
        Provider::Service => {
            let build = builders.service.ok_or_else(|| format!("{what} : cette capacité n'a pas encore de démon — déclarer `local` ou `compatible`"))?;
            let addresses = source.addresses(capability, env);
            if addresses.is_empty() {
                Err(format!("ni `address` dans la déclaration, ni {} dans l'environnement", capability.variables().join(" / ")))
            } else {
                let mut seen = Vec::new();
                let mut found = None;
                for address in &addresses {
                    match build(address) {
                        Ok((client, served)) if served == source.model => {
                            found = Some((client, Origin::Service(address.clone())));
                            break;
                        }
                        Ok((_, served)) => seen.push(format!("{address} sert {served}")),
                        Err(e) => seen.push(format!("{address} : {e}")),
                    }
                }
                found.ok_or_else(|| format!("aucun service ne sert {} — {}", source.model, seen.join(" ; ")))
            }
        }
        Provider::Compatible => {
            let build = builders.compatible.ok_or_else(|| format!("{what} : cette capacité n'a pas de forme `compatible`"))?;
            let addresses = source.addresses(capability, env);
            match addresses.first() {
                None => Err(format!("ni `address` dans la déclaration, ni {} dans l'environnement", capability.variables().join(" / "))),
                Some(address) => build(address, source).map(|c| (c, Origin::Compatible(address.clone()))),
            }
        }
    };
    match (remote, source.fallback) {
        (Ok(found), _) => Ok(found),
        (Err(e), Fallback::Refuse) => Err(format!("{what} : {e}")),
        // Le repli n'arrive que parce qu'on l'a écrit, et il se dit.
        (Err(e), Fallback::Local) => {
            eprintln!("[rag3weaver] {what} : {e} — repli sur le modèle local, comme déclaré (`fallback: local`)");
            local("repli ")
        }
    }
}

// ─── L'embarquement dense ────────────────────────────────────────────────────

/// **L'embarqueur d'une déclaration**, et d'où il calcule.
///
/// - `local` : notre moteur burn, dans ce processus (feature
///   `burn-embedder`) — les poids là où le démon les cherche.
/// - `service` : un démon `rag3weaver-embeddings` (feature `daemon`) ; on s'y
///   attache, on ne le lance ni ne l'arrête.
/// - `compatible` : le contrat OpenAI `/embeddings` (`HttpEmbedder`).
pub fn connect_embedder(source: &ModelSource) -> Result<(Box<dyn crate::embedder::Embedder>, Origin), String> {
    type Client = Box<dyn crate::embedder::Embedder>;
    #[cfg(feature = "burn-embedder")]
    let local = |s: &ModelSource| -> Result<Client, String> { local_embedder(&s.model) };
    #[cfg(feature = "daemon")]
    let service = |address: &str| -> Result<(Client, String), String> {
        let d = crate::daemon::DaemonEmbedder::joindre(address).map_err(|e| e.to_string())?;
        let served = d.identite().modele.clone();
        Ok((Box::new(d) as Client, served))
    };
    #[cfg(feature = "daemon")]
    let compatible = |address: &str, s: &ModelSource| -> Result<Client, String> {
        match s.protocol.as_deref().unwrap_or("openai") {
            "openai" => {
                let dimensions = s.dimensions.ok_or("un embarqueur `compatible` doit déclarer `dimensions`")?;
                Ok(Box::new(crate::http_embedder::HttpEmbedder::new(address, &s.model, dimensions, s.api_key_env.as_deref())?) as Client)
            }
            other => Err(format!("protocole `{other}` inconnu pour l'embarquement (openai)")),
        }
    };
    let builders: Builders<Client> = Builders {
        #[cfg(feature = "burn-embedder")]
        local: Some(&local),
        #[cfg(not(feature = "burn-embedder"))]
        local: None,
        #[cfg(feature = "daemon")]
        service: Some(&service),
        #[cfg(not(feature = "daemon"))]
        service: None,
        #[cfg(feature = "daemon")]
        compatible: Some(&compatible),
        #[cfg(not(feature = "daemon"))]
        compatible: None,
    };
    resolve(Capability::Embed, source, &builders, &process_env)
}

/// Un embarqueur burn dans ce processus, par son nom. Les poids :
/// `RAG3WEAVER_<MODÈLE>_BPK` / `_TOKENIZER`, sinon
/// `~/.cache/rag3weaver/<modèle>/` — la convention du démon et des tests.
#[cfg(feature = "burn-embedder")]
pub fn local_embedder(model: &str) -> Result<Box<dyn crate::embedder::Embedder>, String> {
    use crate::burn_device::{BurnDevice, BurnRole};
    let (folder, prefix) = match model {
        "bge-m3" => ("bge-m3", "RAG3WEAVER_BGE_M3"),
        "granite-107m" => ("granite-107m", "RAG3WEAVER_GRANITE_107M"),
        "granite-278m" => ("granite-278m", "RAG3WEAVER_GRANITE_278M"),
        other => return Err(format!("modèle local `{other}` inconnu (granite-278m, granite-107m, bge-m3)")),
    };
    let (weights, tokenizer) = (local_artifact(folder, prefix, "BPK", "model.bpk")?, local_artifact(folder, prefix, "TOKENIZER", "tokenizer.json")?);
    let device = BurnDevice::for_role(BurnRole::Embedder);
    let built: Box<dyn crate::embedder::Embedder> = match model {
        "bge-m3" => Box::new(crate::burn_bge_m3_embedder::BurnBgeM3Embedder::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?),
        "granite-107m" => Box::new(crate::BurnGranite107m::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?),
        _ => Box::new(crate::BurnGranite278m::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?),
    };
    Ok(built)
}

// ─── Le creux, et le dual ────────────────────────────────────────────────────

/// Ce qu'une déclaration `models.sparse` rend : l'embarqueur creux, et — quand
/// le même modèle sait rendre dense et creux en une passe — le dual.
pub struct SparseClients {
    pub sparse: std::sync::Arc<dyn crate::embedder::SparseEmbedder>,
    pub dual: Option<std::sync::Arc<dyn crate::embedder::DualEmbedder>>,
}

/// **L'embarqueur creux d'une déclaration**, et d'où il calcule.
///
/// - `service` : un démon `rag3weaver-embeddings` qui sert le creux pour ce
///   modèle (bge-m3). Un démon qui sert le bon modèle sans le creux est
///   refusé en le disant.
/// - `local` : BGE-M3 sur burn, dans ce processus.
/// - `compatible` : aucune forme aujourd'hui — le contrat OpenAI ne rend pas
///   de vecteurs creux.
pub fn connect_sparse(source: &ModelSource) -> Result<(SparseClients, Origin), String> {
    #[cfg(feature = "burn-embedder")]
    let local = |s: &ModelSource| -> Result<SparseClients, String> {
        if s.model != "bge-m3" {
            return Err(format!("modèle creux local `{}` inconnu (bge-m3)", s.model));
        }
        let weights = local_artifact("bge-m3", "RAG3WEAVER_BGE_M3", "BPK", "model.bpk")?;
        let tokenizer = local_artifact("bge-m3", "RAG3WEAVER_BGE_M3", "TOKENIZER", "tokenizer.json")?;
        let device = crate::burn_device::BurnDevice::for_role(crate::burn_device::BurnRole::Embedder);
        let model = std::sync::Arc::new(
            crate::burn_bge_m3_embedder::BurnBgeM3Embedder::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?,
        );
        Ok(SparseClients { sparse: model.clone(), dual: Some(model) })
    };
    #[cfg(feature = "daemon")]
    let service = |address: &str| -> Result<(SparseClients, String), String> {
        let d = crate::daemon::DaemonEmbedder::joindre(address).map_err(|e| e.to_string())?;
        let served = d.identite().modele.clone();
        if !d.sait_sparse() {
            return Err(format!("sert {served} sans le creux"));
        }
        let dual = d.sait_dual();
        let d = std::sync::Arc::new(d);
        Ok((SparseClients { sparse: d.clone(), dual: dual.then(|| d as std::sync::Arc<dyn crate::embedder::DualEmbedder>) }, served))
    };
    let builders: Builders<SparseClients> = Builders {
        #[cfg(feature = "burn-embedder")]
        local: Some(&local),
        #[cfg(not(feature = "burn-embedder"))]
        local: None,
        #[cfg(feature = "daemon")]
        service: Some(&service),
        #[cfg(not(feature = "daemon"))]
        service: None,
        compatible: None,
    };
    resolve(Capability::Sparse, source, &builders, &process_env)
}

// ─── Le relecteur et l'OCR ───────────────────────────────────────────────────

/// **Le relecteur d'une déclaration.** Une seule forme aujourd'hui : `local`,
/// un cross-encoder burn dans ce processus — `msmarco-minilm` (anglais),
/// `mmarco-mminilm` (multilingue), `bge-reranker-v2-m3` (multilingue, 2,2 Go).
/// Ni démon ni tiers : la déclaration qui les demande est refusée en le disant.
pub fn connect_reranker(source: &ModelSource) -> Result<(std::sync::Arc<dyn crate::reranker::Reranker>, Origin), String> {
    type Client = std::sync::Arc<dyn crate::reranker::Reranker>;
    #[cfg(feature = "burn-embedder")]
    let local = |s: &ModelSource| -> Result<Client, String> {
        use crate::burn_device::{BurnDevice, BurnRole};
        let (folder, prefix) = match s.model.as_str() {
            "msmarco-minilm" => ("msmarco-minilm", "RAG3WEAVER_MSMARCO"),
            "mmarco-mminilm" => ("mmarco-mminilm", "RAG3WEAVER_MMARCO"),
            "bge-reranker-v2-m3" => ("bge-reranker-v2-m3", "RAG3WEAVER_BGE_RERANKER"),
            other => return Err(format!("relecteur local `{other}` inconnu (msmarco-minilm, mmarco-mminilm, bge-reranker-v2-m3)")),
        };
        let weights = local_artifact(folder, prefix, "BPK", "model.bpk")?;
        let tokenizer = local_artifact(folder, prefix, "TOKENIZER", "tokenizer.json")?;
        let device = BurnDevice::for_role(BurnRole::Reranker);
        let built: Client = match s.model.as_str() {
            "msmarco-minilm" => std::sync::Arc::new(crate::BurnMiniLmReranker::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?),
            "mmarco-mminilm" => std::sync::Arc::new(crate::BurnMMiniLmReranker::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?),
            _ => std::sync::Arc::new(crate::BurnBgeRerankerV2M3::from_files(&weights, &tokenizer, device).map_err(|e| e.to_string())?),
        };
        Ok(built)
    };
    let builders: Builders<Client> = Builders {
        #[cfg(feature = "burn-embedder")]
        local: Some(&local),
        #[cfg(not(feature = "burn-embedder"))]
        local: None,
        service: None,
        compatible: None,
    };
    resolve(Capability::Rerank, source, &builders, &process_env)
}

/// **L'OCR d'une déclaration.** Une seule forme aujourd'hui : `local`,
/// PP-OCR sur burn (`ppocrv6-tiny`), les poids là où les tests les cherchent.
pub fn connect_ocr(source: &ModelSource) -> Result<(std::sync::Arc<dyn crate::ocr::Ocr>, Origin), String> {
    type Client = std::sync::Arc<dyn crate::ocr::Ocr>;
    #[cfg(feature = "burn-ocr")]
    let local = |s: &ModelSource| -> Result<Client, String> {
        use crate::burn_device::{BurnDevice, BurnRole};
        if s.model != "ppocrv6-tiny" {
            return Err(format!("OCR local `{}` inconnu (ppocrv6-tiny)", s.model));
        }
        let dir = crate::BurnPpOcr::default_cache_dir();
        let ocr = crate::BurnPpOcr::from_cache_dir(&dir, BurnDevice::for_role(BurnRole::Ocr))
            .map_err(|e| format!("{} : {e} — donnez RAG3WEAVER_PPOCR_DIR, ou placez-y det.bpk, rec.bpk et dict.txt", dir.display()))?;
        Ok(std::sync::Arc::new(ocr) as Client)
    };
    let builders: Builders<Client> = Builders {
        #[cfg(feature = "burn-ocr")]
        local: Some(&local),
        #[cfg(not(feature = "burn-ocr"))]
        local: None,
        service: None,
        compatible: None,
    };
    resolve(Capability::Ocr, source, &builders, &process_env)
}

/// Un artefact de modèle local : `<PRÉFIXE>_<SUFFIXE>` dans l'environnement,
/// sinon `~/.cache/rag3weaver/<dossier>/<fichier>`.
#[cfg(feature = "burn-embedder")]
fn local_artifact(folder: &str, prefix: &str, suffix: &str, file: &str) -> Result<std::path::PathBuf, String> {
    let variable = format!("{prefix}_{suffix}");
    let path = match std::env::var(&variable) {
        Ok(v) => std::path::PathBuf::from(v),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".cache/rag3weaver").join(folder).join(file),
    };
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("{} est introuvable — donnez {variable}, ou placez le fichier là", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    /// Un faux démon : chaque adresse sert un modèle, ou ne répond pas.
    fn service(address: &str) -> Result<(String, String), String> {
        match address {
            "a:1" => Ok(("client de a".into(), "granite-278m".into())),
            "b:1" => Ok(("client de b".into(), "bge-m3".into())),
            _ => Err("connexion refusée".into()),
        }
    }
    fn local(source: &ModelSource) -> Result<String, String> {
        Ok(format!("local {}", source.model))
    }
    fn compatible(address: &str, source: &ModelSource) -> Result<String, String> {
        Ok(format!("{} @ {address}", source.protocol.as_deref().unwrap_or("openai")))
    }
    fn all<'a>() -> Builders<'a, String> {
        Builders { local: Some(&local), service: Some(&service), compatible: Some(&compatible) }
    }

    #[test]
    fn la_declaration_se_lit_et_garde_le_nom_d_avant() {
        let s: ModelSource = serde_json::from_str(r#"{"model":"granite-278m"}"#).unwrap();
        assert_eq!((s.provider, s.fallback, s.address.is_empty()), (Provider::Service, Fallback::Refuse, true));
        // `daemon` est le nom d'avant du fournisseur `service`.
        let s: ModelSource = serde_json::from_str(r#"{"provider":"daemon","model":"bge-m3","address":"a:1, b:1"}"#).unwrap();
        assert_eq!((s.provider, s.address.0.len()), (Provider::Service, 2));
        let s: ModelSource = serde_json::from_str(r#"{"provider":"compatible","protocol":"llama_server","model":"jevk5-4b","address":["http://x:7982"],"fallback":"local"}"#).unwrap();
        assert_eq!((s.provider, s.fallback, s.protocol.as_deref()), (Provider::Compatible, Fallback::Local, Some("llama_server")));
        // Un champ inconnu est une erreur : une faute de frappe ne passe pas en silence.
        assert!(serde_json::from_str::<ModelSource>(r#"{"model":"x","adress":"a:1"}"#).is_err());
    }

    #[test]
    fn la_variable_suit_la_capacite_et_l_alias_tient() {
        assert_eq!(Capability::Decide.variable(), "RAG3WEAVER_SERVICE_DECIDE");
        assert_eq!(Capability::Embed.variables(), ["RAG3WEAVER_SERVICE_EMBED", "RAG3WEAVER_EMBED_SERVICE"]);
        let s = ModelSource::service("granite-278m");
        // L'alias d'avant est lu…
        let old = |v: &str| (v == "RAG3WEAVER_EMBED_SERVICE").then(|| "a:1,b:1".to_string());
        assert_eq!(s.addresses(Capability::Embed, &old), ["a:1", "b:1"]);
        // … la variable commune passe devant, et l'adresse écrite devant tout.
        let both = |v: &str| Some(if v == "RAG3WEAVER_SERVICE_EMBED" { "c:1" } else { "a:1" }.to_string());
        assert_eq!(s.addresses(Capability::Embed, &both), ["c:1"]);
        let written = ModelSource { address: Addresses::parse("d:1"), ..s.clone() };
        assert_eq!(written.addresses(Capability::Embed, &both), ["d:1"]);
        // Une autre capacité ne lit pas l'alias de l'embarquement.
        assert!(s.addresses(Capability::Rerank, &old).is_empty());
        // Le creux, servi par le même démon, lit les variables de l'embarquement
        // quand il n'a pas la sienne.
        assert_eq!(Capability::Sparse.variables(), ["RAG3WEAVER_SERVICE_SPARSE", "RAG3WEAVER_SERVICE_EMBED", "RAG3WEAVER_EMBED_SERVICE"]);
        assert_eq!(s.addresses(Capability::Sparse, &old), ["a:1", "b:1"]);
    }

    #[test]
    fn le_service_se_choisit_par_le_modele_servi() {
        let s = ModelSource { address: Addresses::parse("mort:1, a:1, b:1"), ..ModelSource::service("bge-m3") };
        let (client, origin) = resolve(Capability::Embed, &s, &all(), &no_env).unwrap();
        assert_eq!((client.as_str(), origin.clone()), ("client de b", Origin::Service("b:1".into())));
        assert!(origin.remote());
        assert_eq!(origin.label(), "service");
    }

    #[test]
    fn un_service_qui_ne_sert_pas_le_modele_est_un_refus_qui_dit_quoi() {
        let s = ModelSource { address: Addresses::parse("mort:1, a:1"), ..ModelSource::service("granite-107m") };
        let e = resolve(Capability::Embed, &s, &all(), &no_env).unwrap_err();
        assert!(e.contains("models.embed (granite-107m)") && e.contains("aucun service ne sert granite-107m"), "{e}");
        assert!(e.contains("a:1 sert granite-278m") && e.contains("mort:1 : connexion refusée"), "{e}");
        // Sans adresse nulle part : on dit quelles variables poser.
        let e = resolve(Capability::Embed, &ModelSource::service("granite-278m"), &all(), &no_env).unwrap_err();
        assert!(e.contains("RAG3WEAVER_SERVICE_EMBED") && e.contains("RAG3WEAVER_EMBED_SERVICE"), "{e}");
    }

    /// **Le repli n'arrive que s'il est écrit.**
    #[test]
    fn le_repli_local_ne_vient_que_s_il_est_declare() {
        let dead = ModelSource { address: Addresses::parse("mort:1"), ..ModelSource::service("granite-278m") };
        assert!(resolve(Capability::Embed, &dead, &all(), &no_env).is_err(), "défaut : refus");
        let declared = ModelSource { fallback: Fallback::Local, ..dead };
        let (client, origin) = resolve(Capability::Embed, &declared, &all(), &no_env).unwrap();
        assert_eq!((client.as_str(), origin.clone()), ("local granite-278m", Origin::Local));
        assert!(!origin.remote());
    }

    #[test]
    fn local_et_compatible_passent_par_leurs_constructeurs() {
        let (client, origin) = resolve(Capability::Rerank, &ModelSource::local("bge-reranker-v2-m3"), &all(), &no_env).unwrap();
        assert_eq!((client.as_str(), origin), ("local bge-reranker-v2-m3", Origin::Local));
        let s = ModelSource { provider: Provider::Compatible, protocol: Some("llama_server".into()), ..ModelSource::local("jevk5-4b") };
        // L'adresse d'un tiers peut venir de l'environnement, comme les autres.
        let env = |v: &str| (v == "RAG3WEAVER_SERVICE_DECIDE").then(|| "http://127.0.0.1:7982".to_string());
        let (client, origin) = resolve(Capability::Decide, &s, &all(), &env).unwrap();
        assert_eq!(client, "llama_server @ http://127.0.0.1:7982");
        assert_eq!(origin, Origin::Compatible("http://127.0.0.1:7982".into()));
    }

    /// Une capacité sans démon le dit, au lieu d'échouer plus loin.
    #[test]
    fn une_forme_qui_n_existe_pas_est_refusee_avec_sa_raison() {
        let only_local: Builders<String> = Builders { local: Some(&local), service: None, compatible: None };
        let e = resolve(Capability::Ocr, &ModelSource::service("ppocrv6-tiny"), &only_local, &no_env).unwrap_err();
        assert!(e.contains("models.ocr") && e.contains("pas encore de démon"), "{e}");
        let none: Builders<String> = Builders { local: None, service: None, compatible: None };
        let e = resolve(Capability::Ocr, &ModelSource::local("ppocrv6-tiny"), &none, &no_env).unwrap_err();
        assert!(e.contains("pas de forme locale"), "{e}");
    }

    /// Le relecteur et l'OCR n'ont qu'une forme, locale : un démon ou un tiers
    /// demandé est refusé avec sa raison, et un modèle inconnu se dit.
    #[test]
    fn le_relecteur_et_l_ocr_ne_se_declarent_qu_en_local_et_le_disent() {
        let e = connect_reranker(&ModelSource::service("bge-reranker-v2-m3")).err().expect("pas de démon de relecture");
        assert!(e.contains("models.rerank") && e.contains("pas encore de démon"), "{e}");
        let e = connect_ocr(&ModelSource::service("ppocrv6-tiny")).err().expect("pas de démon d'OCR");
        assert!(e.contains("models.ocr") && e.contains("pas encore de démon"), "{e}");
        let tiers = ModelSource { provider: Provider::Compatible, address: Addresses::parse("http://x:1"), ..ModelSource::local("x") };
        assert!(connect_reranker(&tiers).err().expect("pas de tiers").contains("pas de forme `compatible`"));
        // Un nom que le moteur local ne connaît pas : l'erreur liste ceux qu'il connaît.
        #[cfg(feature = "burn-embedder")]
        {
            let e = connect_reranker(&ModelSource::local("un-autre")).err().expect("inconnu");
            assert!(e.contains("msmarco-minilm") && e.contains("bge-reranker-v2-m3"), "{e}");
        }
        #[cfg(feature = "burn-ocr")]
        {
            let e = connect_ocr(&ModelSource::local("un-autre")).err().expect("inconnu");
            assert!(e.contains("ppocrv6-tiny"), "{e}");
        }
    }
}
