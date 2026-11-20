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

#[path = "model_source/activation.rs"]
pub mod activation;

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
        // Le creux, le relecteur et l'OCR sont servis par le démon
        // d'embarquement : sans variable à eux, ils lisent les siennes.
        if matches!(self, Self::Sparse | Self::Rerank | Self::Ocr) {
            v.push(Self::Embed.variable());
        }
        if matches!(self, Self::Embed | Self::Sparse | Self::Rerank | Self::Ocr) {
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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fallback {
    #[default]
    Refuse,
    Local,
    /// **Un service de repli** (`{"service": "127.0.0.1:7879"}`), pour une
    /// activation : quand la carte qu'elle nomme est pleine (mesurée), ou que
    /// son débit tombe sous le seuil déclaré, l'embarqueur sert par ce service,
    /// modèle et dimension vérifiés, et le dit. Sans activation, il vaut refus.
    Service(String),
    /// **Une activation de repli** (`{"activation": "<nom>"}`, résolue au
    /// chargement en son bloc nommé) : même fournisseur que l'activation
    /// principale au placement près, sur sa propre carte, mesurée elle aussi.
    Activation(Box<activation::Activation>),
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

/// **La déclaration d'un modèle**, la même pour toute capacité. Elle se lit et s'écrit avec `access` (la
/// manière d'accéder) et `provider` (le fournisseur) ; l'ancienne forme, `provider` pour la manière et
/// `protocol` pour le fournisseur, se lit encore jusqu'au 29 novembre 2026 ([`ModelSourceWire`]). Dans le code,
/// les champs gardent leurs noms : `provider` porte la manière d'accéder, `protocol` le fournisseur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ModelSourceWire")]
pub struct ModelSource {
    /// Activation locale explicite ; absente, le service est seulement joint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<activation::Activation>,
    #[serde(default, rename = "access")]
    pub provider: Provider,
    /// Le modèle attendu. Un service à nous dit ce qu'il sert ; s'il sert
    /// autre chose, c'est un refus.
    pub model: String,
    /// Absente : elle vient de l'environnement ([`Capability::variables`]).
    #[serde(default, skip_serializing_if = "Addresses::is_empty")]
    pub address: Addresses,
    /// Pour `compatible` : `openai`, `llama_server`, `vertex`, `anthropic`. Écrit `provider`.
    #[serde(default, rename = "provider", skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Le **nom** de la variable qui porte la clé, jamais la clé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub fallback: Fallback,
    /// La dimension des vecteurs, pour les capacités qui en rendent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    /// La fenêtre de contexte, pour un modèle de langage : un fait du modèle,
    /// comme `dimensions` l'est d'un embarqueur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<usize>,
    /// **Le plafond d'une requête embarquée**, en caractères, pour un modèle
    /// d'embarquement (décision de c7, 15 novembre 2026) : ce que la carte
    /// tient, pas ce que le modèle accepte. Absent :
    /// [`crate::search::DEFAULT_QUERY_MAX_CHARS`] ; jamais au-dessus de
    /// `context_tokens × 3` (refusé au chargement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_max_chars: Option<usize>,
    /// Ce qu'un protocole demande en plus et que la déclaration commune n'a
    /// pas à connaître : `project` et `location` pour `vertex`. La frontière
    /// passe ici — un paramètre que deux capacités partagent devient un champ.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub params: std::collections::BTreeMap<String, String>,
    /// **Le délai d'un appel**, en secondes, pour un modèle servi par le
    /// réseau : au-delà, l'appel est perdu et se dit (un embarquement muet ne
    /// tient plus son fil). Absent : le défaut du client (60 s pour le service
    /// d'embarquement, `RAG3WEAVER_EMBED_TIMEOUT_SECS`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    /// **Le budget de raisonnement** d'un modèle de langage : `minimal`,
    /// `low`, `medium`, `high` (`reasoning_effort` du protocole). Absent :
    /// rien n'est envoyé, le modèle raisonne à son défaut — Lucie, 25 octobre
    /// 2026 : « garder par défaut autant qu'il veut, mais savoir régler ».
    /// Un appel qui fixe le sien (`GenOptions::reasoning`) l'emporte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<crate::llm::ReasoningEffort>,
    /// **Le nom d'un modèle de langage déclaré** (`models.llm` en liste) : la clé stable que la tâche
    /// retient et que `/model` affiche. Obligatoire dans une liste ; absent, le nom est `model`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// **Le plafond de sortie** d'un modèle de langage, en jetons (la forme (a) du plafond du juge).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
    /// **Le budget du contexte d'après une compaction**, en jetons ([`Self::budget_de_reprise`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_budget_tokens: Option<usize>,
    /// **Le modèle de langage par défaut** d'une liste `models.llm` : un seul l'est.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
    /// **The log of a service posed by hand**, an absolute path : when the
    /// service does not answer, its last line « ✗ arrêt : <cause> » is said
    /// (18 November 2026, entry 7). An activation has its own `journal`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<std::path::PathBuf>,
}

/// **A declared model as written in a manifest**, in either form (ticket of 15 November 2026, « provider et
/// protocol »): the new one, `access` and `provider`, or the old one, `provider` for the access and `protocol`
/// for the provider, read until 29 November 2026. A model that mixes both is refused, naming both words.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelSourceWire {
    #[serde(default)]
    activation: Option<activation::Activation>,
    #[serde(default)]
    access: Option<Provider>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    protocol: Option<String>,
    model: String,
    #[serde(default)]
    address: Addresses,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    fallback: Fallback,
    #[serde(default)]
    dimensions: Option<usize>,
    #[serde(default)]
    context_tokens: Option<usize>,
    #[serde(default)]
    query_max_chars: Option<usize>,
    #[serde(default)]
    params: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    reasoning: Option<crate::llm::ReasoningEffort>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    max_output_tokens: Option<usize>,
    #[serde(default)]
    restart_budget_tokens: Option<usize>,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    log: Option<std::path::PathBuf>,
}

/// The words of the old form for the access: a `provider` with one of these values is the old form.
const OLD_ACCESS_WORDS: [&str; 4] = ["local", "service", "daemon", "compatible"];

impl TryFrom<ModelSourceWire> for ModelSource {
    type Error = String;
    fn try_from(w: ModelSourceWire) -> Result<Self, String> {
        let old_access = w.provider.as_deref().filter(|p| OLD_ACCESS_WORDS.contains(p));
        let (provider, protocol) = match (w.access, old_access, w.protocol) {
            (Some(_), _, Some(_)) => {
                return Err("un modèle mêle les deux formes : `access` (la forme neuve) et `protocol` (l'ancienne) ; \
                            écrire `access` et `provider` (le fournisseur)".into())
            }
            (Some(_), Some(p), None) => {
                return Err(format!(
                    "un modèle mêle les deux formes : `access` (la forme neuve) et `provider: {p}` (l'ancien sens, la manière \
                     d'accéder) ; écrire `access: {p}`, et le fournisseur sous `provider`"
                ))
            }
            (Some(access), None, None) => (access, w.provider),
            // The old form: `provider` is the access, `protocol` the provider.
            (None, Some(p), protocol) => (Provider::deserialize(serde::de::value::StrDeserializer::<serde::de::value::Error>::new(p)).map_err(|e| e.to_string())?, protocol),
            (None, None, Some(protocol)) => {
                if w.provider.is_some() {
                    return Err("un modèle mêle les deux formes : `provider` au sens neuf (le fournisseur) et `protocol` (l'ancienne forme) ; écrire `provider` seul".into());
                }
                (Provider::default(), Some(protocol))
            }
            (None, None, None) => (Provider::default(), w.provider),
        };
        Ok(Self {
            activation: w.activation,
            provider,
            model: w.model,
            address: w.address,
            protocol,
            api_key_env: w.api_key_env,
            fallback: w.fallback,
            dimensions: w.dimensions,
            context_tokens: w.context_tokens,
            query_max_chars: w.query_max_chars,
            params: w.params,
            timeout_secs: w.timeout_secs,
            reasoning: w.reasoning,
            name: w.name,
            max_output_tokens: w.max_output_tokens,
            restart_budget_tokens: w.restart_budget_tokens,
            default: w.default,
            log: w.log,
        })
    }
}

impl ModelSource {
    /// **Why the provider stopped**, read at the last line of its log when it
    /// starts with « ✗ arrêt : » : an activation's own journal on this host,
    /// else the `log` of a service posed by hand. `None` otherwise (18 November
    /// 2026, entry 7 ; before, the activation's journal only).
    pub fn stop_cause(&self) -> Option<String> {
        let path = match (&self.activation, &self.log) {
            #[cfg(feature = "daemon")]
            (Some(a), _) if a.hote.is_none() => a.journal.join(format!("{}.log", crate::daemon::embeddings::SERVICE)),
            (None, Some(log)) => log.clone(),
            _ => return None,
        };
        let text = std::fs::read_to_string(path).ok()?;
        let line = text.lines().rev().find(|l| !l.trim().is_empty())?.trim();
        let cause = line.strip_prefix("✗ arrêt : ")?;
        Some(cause.split(" (code ").next().unwrap_or(cause).to_string())
    }

    /// **The `log` of a declaration, checked at load** : absolute, and never
    /// with an activation (two truths about one provider).
    pub fn check_log(&self, at: &str) -> Result<(), String> {
        let Some(log) = &self.log else { return Ok(()) };
        if !log.is_absolute() {
            return Err(format!("{at}.log ({}) : un chemin absolu est attendu", log.display()));
        }
        if self.activation.is_some() {
            return Err(format!(
                "{at}.log ({}) avec une activation : l'activation a déjà son journal ; ne garder que l'un",
                log.display()
            ));
        }
        Ok(())
    }

    pub fn local(model: impl Into<String>) -> Self {
        Self {
            activation: None,
            provider: Provider::Local,
            model: model.into(),
            address: Addresses::default(),
            protocol: None,
            api_key_env: None,
            fallback: Fallback::Refuse,
            dimensions: None,
            context_tokens: None,
            query_max_chars: None,
            params: Default::default(),
            timeout_secs: None,
            reasoning: None,
            name: None,
            max_output_tokens: None,
            restart_budget_tokens: None,
            log: None,
            default: false,
        }
    }
    /// **Le budget du contexte qui repart après une compaction** par l'archiviste (96, conception
    /// `docs/memoire/15-11-2026/03-la-compaction-est-l-archiviste.md`) : `restart_budget_tokens` s'il
    /// est déclaré ; sinon 100 000 jetons pour un modèle cloud, et pour un modèle local (servi ici, par
    /// un service à nous, ou un `compatible` à une adresse de boucle : un llama-server) la moitié de sa
    /// fenêtre, au plus 100 000.
    pub fn budget_de_reprise(&self) -> usize {
        const CLOUD: usize = 100_000;
        if let Some(b) = self.restart_budget_tokens {
            return b;
        }
        // A llama-server is declared `compatible` at its address: a loopback address makes it local. The protocol
        // `llama_server` is not a sign of it: `connect_llm` refuses it for a language model.
        let local = matches!(self.provider, Provider::Local | Provider::Service)
            || (!self.address.is_empty() && self.address.0.iter().all(|a| is_loopback_address(a)));
        if !local {
            return CLOUD;
        }
        self.context_tokens.map_or(CLOUD, |c| (c / 2).min(CLOUD))
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

/// **Is this address on this machine?** `127.0.0.1` (and the rest of `127.0.0.0/8`), `localhost` or `::1`, with or
/// without a scheme, a port or a path. A host that only starts like one (`localhost.example.com`) is not.
fn is_loopback_address(address: &str) -> bool {
    let rest = address.split_once("://").map_or(address, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if let Some(v6) = host_port.strip_prefix('[') {
        v6.split(']').next().unwrap_or("")
    } else {
        host_port.split(':').next().unwrap_or("")
    };
    let host = host.to_ascii_lowercase();
    host == "localhost" || host == "::1" || host.parse::<std::net::Ipv4Addr>().is_ok_and(|ip| ip.is_loopback())
}

/// **Les modèles déclarés d'un manifeste**, par capacité (`models` d'un manifeste de backend ou d'un
/// chat). Il se lit comme la carte d'avant : `models.get(&Capability::Llm)` rend le modèle de langage
/// **par défaut**. `models.llm` peut aussi être une liste de modèles nommés
/// ([`Self::modeles_de_langage`]), dont un seul est le défaut ; l'objet unique d'avant se lit comme une
/// liste à un élément, par défaut, au nom de son `name`, sinon de son `model`.
///
/// Ce nom replié n'est pas stable : il change si un `name` s'ajoute ou si `model` change. Une surcharge
/// de tâche qui le retenait reçoit alors le refus « nom inconnu », jamais le défaut en silence.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Modeles {
    par_capacite: std::collections::BTreeMap<Capability, ModelSource>,
    /// La liste déclarée de `models.llm`, quand c'en est une ; vide pour l'objet unique.
    langage: Vec<ModelSource>,
}

impl std::ops::Deref for Modeles {
    type Target = std::collections::BTreeMap<Capability, ModelSource>;
    fn deref(&self) -> &Self::Target {
        &self.par_capacite
    }
}

impl std::ops::DerefMut for Modeles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.par_capacite
    }
}

impl From<std::collections::BTreeMap<Capability, ModelSource>> for Modeles {
    fn from(par_capacite: std::collections::BTreeMap<Capability, ModelSource>) -> Self {
        Self { par_capacite, langage: Vec::new() }
    }
}

impl Modeles {
    /// Le nom sous lequel un modèle de langage se déclare et s'affiche.
    pub fn nom(source: &ModelSource) -> &str {
        source.name.as_deref().unwrap_or(&source.model)
    }

    /// Les modèles de langage déclarés, dans l'ordre du manifeste : la liste, ou l'objet unique.
    pub fn modeles_de_langage(&self) -> Vec<&ModelSource> {
        if self.langage.is_empty() {
            self.par_capacite.get(&Capability::Llm).into_iter().collect()
        } else {
            self.langage.iter().collect()
        }
    }

    /// Le modèle de langage de ce nom ; un nom inconnu est refusé, en listant les noms déclarés.
    pub fn modele_de_langage(&self, nom: &str) -> Result<&ModelSource, String> {
        let tous = self.modeles_de_langage();
        tous.iter().copied().find(|s| Self::nom(s) == nom).ok_or_else(|| {
            let noms: Vec<&str> = tous.iter().map(|s| Self::nom(s)).collect();
            if noms.is_empty() {
                format!("modèle de langage « {nom} » inconnu : le manifeste n'en déclare aucun (`models.llm`)")
            } else {
                format!("modèle de langage « {nom} » inconnu ; les modèles déclarés : {}", noms.join(", "))
            }
        })
    }
}

impl Serialize for Modeles {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut m = s.serialize_map(Some(self.par_capacite.len()))?;
        for (capacite, source) in &self.par_capacite {
            if *capacite == Capability::Llm && !self.langage.is_empty() {
                m.serialize_entry(capacite, &self.langage)?;
            } else {
                m.serialize_entry(capacite, source)?;
            }
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for Modeles {
    /// Chaque capacité porte un objet ; `llm` peut porter une liste, lue et vérifiée ici : chaque
    /// élément nommé, aucun nom en double, un seul défaut (pris d'office par une liste d'un élément).
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let brut = std::collections::BTreeMap::<Capability, serde_json::Value>::deserialize(d)?;
        let mut modeles = Modeles::default();
        for (capacite, valeur) in brut {
            let lire = |v: serde_json::Value, ou: String| serde_json::from_value::<ModelSource>(v).map_err(|e| D::Error::custom(format!("{ou} : {e}")));
            let serde_json::Value::Array(elements) = valeur else {
                modeles.par_capacite.insert(capacite, lire(valeur, format!("models.{}", capacite.name()))?);
                continue;
            };
            if capacite != Capability::Llm {
                return Err(D::Error::custom(format!(
                    "models.{} : seul `models.llm` se déclare en liste ; `models.{}` prend un objet",
                    capacite.name(),
                    capacite.name()
                )));
            }
            let mut liste = Vec::with_capacity(elements.len());
            for (i, v) in elements.into_iter().enumerate() {
                let source = lire(v, format!("models.llm[{i}]"))?;
                if source.name.as_deref().is_none_or(str::is_empty) {
                    return Err(D::Error::custom(format!("models.llm[{i}] : `name` est obligatoire dans une liste (le nom que la tâche retient et que /model affiche)")));
                }
                if liste.iter().any(|s: &ModelSource| s.name == source.name) {
                    return Err(D::Error::custom(format!("models.llm : le nom « {} » est déclaré deux fois", source.name.as_deref().unwrap_or_default())));
                }
                liste.push(source);
            }
            if liste.len() == 1 {
                liste[0].default = true;
            }
            let defauts: Vec<&str> = liste.iter().filter(|s| s.default).map(|s| Modeles::nom(s)).collect();
            match defauts.len() {
                1 => {}
                0 => {
                    let noms: Vec<&str> = liste.iter().map(|s| Modeles::nom(s)).collect();
                    return Err(D::Error::custom(format!("models.llm : aucun modèle par défaut parmi {} ; posez `default: true` sur l'un d'eux", noms.join(", "))));
                }
                _ => {
                    return Err(D::Error::custom(format!("models.llm : un seul modèle par défaut, et {} le sont ({})", defauts.len(), defauts.join(", "))));
                }
            }
            if let Some(defaut) = liste.iter().find(|s| s.default) {
                modeles.par_capacite.insert(Capability::Llm, defaut.clone());
            }
            modeles.langage = liste;
        }
        Ok(modeles)
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

/// **Le modèle de langage déclaré dont le fournisseur n'est pas dans ce binaire** : la phrase de l'état,
/// du reçu et du refus d'un tour de chat (décision de c7, 15 novembre 2026).
pub fn modele_de_langage_sans_sa_feature(nom: &str) -> String {
    format!("modèle de langage {nom} indisponible : la feature openai-llm n'est pas dans ce binaire")
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
    if source.activation.is_some() {
        return Err("activation déclarée : utiliser le connecteur dense géré ; autre capacité non encore prise en charge".into());
    }
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
            let build = builders.compatible.ok_or_else(|| {
                format!("{what} : cette capacité n'a pas de forme `compatible` dans ce binaire (le fournisseur demande peut-être une feature absente : openai-llm, anthropic-llm)")
            })?;
            let addresses = source.addresses(capability, env);
            match addresses.first() {
                None => Err(format!("ni `address` dans la déclaration, ni {} dans l'environnement", capability.variables().join(" / "))),
                Some(address) => build(address, source).map(|c| (c, Origin::Compatible(address.clone()))),
            }
        }
    };
    match (remote, &source.fallback) {
        (Ok(found), _) => Ok(found),
        (Err(e), Fallback::Refuse | Fallback::Service(_) | Fallback::Activation(_)) => Err(format!("{what} : {e}")),
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
    connect_embedder_annulable(source, &std::sync::atomic::AtomicBool::new(false))
}

/// L'annulation ne concerne que ce demandeur ; le chargement partagé continue.
pub fn connect_embedder_annulable(source: &ModelSource, annule: &std::sync::atomic::AtomicBool) -> Result<(Box<dyn crate::embedder::Embedder>, Origin), String> {
    if source.activation.is_some() {
        #[cfg(feature = "daemon")]
        return activation::connecter_ou_attendre(source, annule);
        #[cfg(not(feature = "daemon"))]
        return Err("activation d'embarquement : feature daemon requise".into());
    }
    if annule.load(std::sync::atomic::Ordering::Acquire) { return Err("connexion annulée".into()); }
    type Client = Box<dyn crate::embedder::Embedder>;
    #[cfg(feature = "burn-embedder")]
    let local = |s: &ModelSource| -> Result<Client, String> { local_embedder(&s.model) };
    #[cfg(feature = "daemon")]
    let service = |address: &str| -> Result<(Client, String), String> {
        let d = crate::daemon::DaemonEmbedder::joindre(address).map_err(|e| e.to_string())?;
        let d = match source.timeout_secs.filter(|s| *s > 0) {
            Some(s) => d.with_timeout(std::time::Duration::from_secs(s)),
            None => d,
        };
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
    let device = BurnDevice::for_role(BurnRole::Embedder)
        .check_embedder(std::env::var(crate::burn_device::ALLOW_INTEGRATED_GPU_VARIABLE).ok().as_deref())?;
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
    // Une activation déclarée : le fournisseur se lance ou s'attache par elle
    // (le groupe d'une activation nommée, à la demande ou maintenu).
    if source.activation.is_some() {
        #[cfg(feature = "daemon")]
        return activation::connecter_creux(source);
        #[cfg(not(feature = "daemon"))]
        return Err("activation : feature daemon requise".into());
    }
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

/// **Le relecteur d'une déclaration.** `local` : un cross-encoder burn dans
/// ce processus — `msmarco-minilm` (anglais), `mmarco-mminilm` (multilingue),
/// `bge-reranker-v2-m3` (multilingue, 2,2 Go). `service` : le démon
/// d'embarquement qui déclare ce relecteur (`POST /rerank`). Pas de tiers.
pub fn connect_reranker(source: &ModelSource) -> Result<(std::sync::Arc<dyn crate::reranker::Reranker>, Origin), String> {
    // Une activation déclarée : le fournisseur se lance ou s'attache par elle
    // (le groupe d'une activation nommée, à la demande ou maintenu).
    if source.activation.is_some() {
        #[cfg(feature = "daemon")]
        return activation::connecter_relecteur(source);
        #[cfg(not(feature = "daemon"))]
        return Err("activation : feature daemon requise".into());
    }
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
    #[cfg(feature = "daemon")]
    let service = |address: &str| -> Result<(Client, String), String> {
        let d = crate::daemon::embeddings::DaemonReranker::joindre(address).map_err(|e| e.to_string())?;
        let served = d.model().to_string();
        Ok((std::sync::Arc::new(d) as Client, served))
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
        compatible: None,
    };
    resolve(Capability::Rerank, source, &builders, &process_env)
}

/// **L'OCR d'une déclaration.** `local` : PP-OCR sur burn (`ppocrv6-tiny`),
/// les poids là où les tests les cherchent. `service` : le démon
/// d'embarquement qui déclare cet OCR (`POST /ocr`).
pub fn connect_ocr(source: &ModelSource) -> Result<(std::sync::Arc<dyn crate::ocr::Ocr>, Origin), String> {
    // Une activation déclarée : le fournisseur se lance ou s'attache par elle
    // (le groupe d'une activation nommée, à la demande ou maintenu).
    if source.activation.is_some() {
        #[cfg(feature = "daemon")]
        return activation::connecter_ocr(source);
        #[cfg(not(feature = "daemon"))]
        return Err("activation : feature daemon requise".into());
    }
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
    #[cfg(feature = "daemon")]
    let service = |address: &str| -> Result<(Client, String), String> {
        let d = crate::daemon::embeddings::DaemonOcr::joindre(address).map_err(|e| e.to_string())?;
        let served = d.model().to_string();
        Ok((std::sync::Arc::new(d) as Client, served))
    };
    let builders: Builders<Client> = Builders {
        #[cfg(feature = "burn-ocr")]
        local: Some(&local),
        #[cfg(not(feature = "burn-ocr"))]
        local: None,
        #[cfg(feature = "daemon")]
        service: Some(&service),
        #[cfg(not(feature = "daemon"))]
        service: None,
        compatible: None,
    };
    resolve(Capability::Ocr, source, &builders, &process_env)
}

// ─── Le modèle de langage ────────────────────────────────────────────────────

/// **Le modèle de langage d'une déclaration**, et d'où il calcule.
///
/// Une seule forme : `compatible`, par deux protocoles.
/// - `openai` (défaut) : un point d'accès compatible OpenAI à `address` — un
///   service tiers, ou un llama-server sur ce poste ou un autre. La clé vient
///   de la variable nommée par `api_key_env`.
/// - `vertex` : Gemini par Vertex AI. Pas d'adresse : elle se construit du
///   projet (`params.project`, sinon `GOOGLE_CLOUD_PROJECT`) et du lieu
///   (`params.location`, `global` par défaut) ; le jeton vient de
///   `gcp_auth::TokenSource`, gardée par le client : un jeton à chaque appel, et
///   un 401 avant le flux le renouvelle une fois.
/// - `anthropic` : l'API Messages d'Anthropic (feature `anthropic-llm`). Pas
///   d'adresse (ou une, pour un mandataire) ; la clé vient de la variable
///   nommée par `api_key_env`, `ANTHROPIC_API_KEY` par défaut.
///
/// Rend un `Arc<dyn Llm>` : deux clients, un seul type pour tous les
/// appelants (catalogue, hôte, chat), qui le partagent tel quel.
///
/// Ni `local` ni `service` : un modèle de langage sur ce poste est un
/// llama-server, donc `compatible` à une adresse locale.
#[cfg(feature = "openai-llm")]
pub fn connect_llm(source: &ModelSource) -> Result<(std::sync::Arc<dyn crate::llm::Llm>, Origin), String> {
    connect_llm_with_vertex(source, |project, location, model| {
        let tokens = crate::gcp_auth::TokenSource::from_env()
            .map_err(|e| format!("jeton Vertex : {e}"))?;
        Ok(crate::openai_llm::OpenAiLlm::vertex_with_source(project, location, std::sync::Arc::new(tokens), model))
    })
}

/// Même résolution déclarative ; seule la construction du fournisseur Vertex s'injecte.
/// Les autres protocoles n'appellent jamais cette fabrique.
#[cfg(feature = "openai-llm")]
pub(crate) fn connect_llm_with_vertex(
    source: &ModelSource,
    vertex: impl FnOnce(&str, &str, &str) -> Result<crate::openai_llm::OpenAiLlm, String>,
) -> Result<(std::sync::Arc<dyn crate::llm::Llm>, Origin), String> {
    use std::sync::Arc;
    use crate::openai_llm::{secret_from_env, Auth, OpenAiLlm};
    let what = format!("models.llm ({})", source.model);
    if source.model.trim().is_empty() {
        return Err("models.llm : un modèle est requis".into());
    }
    // Ce que la déclaration dit du modèle, posé sur le client quel que soit
    // le chemin : sa fenêtre, et son budget de raisonnement s'il en a un.
    let with_context = |llm: OpenAiLlm| {
        let llm = match source.context_tokens {
            Some(n) => llm.with_context_len(n),
            None => llm,
        };
        match source.reasoning {
            Some(effort) => llm.with_reasoning_default(effort),
            None => llm,
        }
    };
    if source.provider == Provider::Compatible && source.protocol.as_deref() == Some("anthropic") {
        #[cfg(feature = "anthropic-llm")]
        {
            use crate::anthropic_llm::{AnthropicLlm, API_KEY_ENV};
            let variable = source.api_key_env.as_deref().unwrap_or(API_KEY_ENV);
            let key = secret_from_env(variable).map_err(|e| format!("{what} : {e}"))?;
            let mut llm = AnthropicLlm::new(key, &source.model);
            if let Some(url) = source.address.0.first() {
                llm = llm.with_base_url(url);
            }
            if let Some(n) = source.context_tokens {
                llm = llm.with_context_len(n);
            }
            if let Some(effort) = source.reasoning {
                llm = llm.with_reasoning_default(effort);
            }
            return Ok((Arc::new(llm) as Arc<dyn crate::llm::Llm>, Origin::Compatible("anthropic".into())));
        }
        #[cfg(not(feature = "anthropic-llm"))]
        return Err(format!("{what} : le protocole anthropic demande la feature `anthropic-llm`"));
    }
    if source.provider == Provider::Compatible && source.protocol.as_deref() == Some("vertex") {
        let project = source
            .params
            .get("project")
            .cloned()
            .or_else(|| process_env("GOOGLE_CLOUD_PROJECT"))
            .ok_or_else(|| format!("{what} : le protocole vertex demande `params.project`, ou GOOGLE_CLOUD_PROJECT"))?;
        let location = source.params.get("location").cloned().unwrap_or_else(|| "global".to_string());
        let llm = with_context(vertex(&project, &location, &source.model).map_err(|e| format!("{what} : {e}"))?);
        return Ok((Arc::new(llm) as Arc<dyn crate::llm::Llm>, Origin::Compatible(format!("vertex:{project}/{location}"))));
    }
    let compatible = |address: &str, s: &ModelSource| -> Result<OpenAiLlm, String> {
        match s.protocol.as_deref().unwrap_or("openai") {
            "openai" => {
                if !(address.starts_with("http://") || address.starts_with("https://")) {
                    return Err(format!("`{address}` n'est pas une adresse http(s)"));
                }
                let mut llm = OpenAiLlm::new(address, &s.model);
                if let Some(variable) = &s.api_key_env {
                    llm = llm.with_auth(Auth::Bearer(secret_from_env(variable).map_err(|e| e.to_string())?));
                }
                Ok(llm)
            }
            other => Err(format!("protocole `{other}` inconnu pour un modèle de langage (openai, vertex, anthropic)")),
        }
    };
    let builders: Builders<OpenAiLlm> = Builders { local: None, service: None, compatible: Some(&compatible) };
    let (llm, origin) = resolve(Capability::Llm, source, &builders, &process_env)?;
    Ok((Arc::new(with_context(llm)) as Arc<dyn crate::llm::Llm>, origin))
}

/// Un artefact de modèle local : `<PRÉFIXE>_<SUFFIXE>` dans l'environnement,
/// sinon `~/.cache/rag3weaver/<dossier>/<fichier>`.
#[cfg(feature = "burn-embedder")]
pub(crate) fn local_artifact(folder: &str, prefix: &str, suffix: &str, file: &str) -> Result<std::path::PathBuf, String> {
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

    /// **`models.llm` en liste** (ticket `2026-11-15-models-llm-devient-une-liste-de-modeles-declares`).
    mod modeles_de_langage {
        use super::super::*;

        fn lire(json: &str) -> Result<Modeles, String> {
            serde_json::from_str::<Modeles>(json).map_err(|e| e.to_string())
        }

        #[test]
        fn l_objet_unique_se_lit_comme_une_liste_a_un_element_par_defaut() {
            let m = lire(r#"{"llm": {"provider": "compatible", "protocol": "vertex", "model": "gemini-2.5-pro"}}"#).unwrap();
            let noms: Vec<&str> = m.modeles_de_langage().iter().map(|s| Modeles::nom(s)).collect();
            assert_eq!(noms, ["gemini-2.5-pro"], "un élément, au nom de son modèle");
            assert_eq!(m.get(&Capability::Llm).map(|s| s.model.as_str()), Some("gemini-2.5-pro"), "le défaut");
            assert!(m.modele_de_langage("gemini-2.5-pro").is_ok());
        }

        #[test]
        fn une_liste_rend_son_defaut_et_chacun_par_son_nom() {
            let m = lire(r#"{"llm": [
                {"name": "rapide", "provider": "compatible", "protocol": "vertex", "model": "gemini-2.5-flash", "context_tokens": 1000000},
                {"name": "juge", "provider": "compatible", "protocol": "vertex", "model": "gemini-2.5-pro", "max_output_tokens": 8192, "default": true}
            ]}"#)
            .expect("une liste de deux modèles se lit");
            assert_eq!(m.get(&Capability::Llm).and_then(|s| s.name.as_deref()), Some("juge"), "le défaut est le second");
            let noms: Vec<&str> = m.modeles_de_langage().iter().map(|s| Modeles::nom(s)).collect();
            assert_eq!(noms, ["rapide", "juge"], "dans l'ordre du manifeste");
            assert_eq!(m.modele_de_langage("rapide").unwrap().context_tokens, Some(1_000_000));
            assert_eq!(m.modele_de_langage("juge").unwrap().max_output_tokens, Some(8192));
            let relu: Modeles = serde_json::from_value(serde_json::to_value(&m).unwrap()).expect("relu");
            assert_eq!(relu, m, "la liste survit à l'écriture");
        }

        #[test]
        fn deux_defauts_sont_refuses_en_les_nommant() {
            let e = lire(r#"{"llm": [
                {"name": "a", "model": "m1", "default": true},
                {"name": "b", "model": "m2", "default": true}
            ]}"#)
            .expect_err("deux défauts");
            assert!(e.contains("a") && e.contains("b") && e.contains("défaut"), "{e}");
        }

        #[test]
        fn une_liste_sans_defaut_ni_nom_est_refusee() {
            let e = lire(r#"{"llm": [{"name": "a", "model": "m1"}, {"name": "b", "model": "m2"}]}"#).expect_err("aucun défaut");
            assert!(e.contains("défaut"), "{e}");
            let e = lire(r#"{"llm": [{"model": "m1", "default": true}, {"name": "b", "model": "m2"}]}"#).expect_err("un élément sans nom");
            assert!(e.contains("name"), "{e}");
            let e = lire(r#"{"llm": [{"name": "a", "model": "m1", "default": true}, {"name": "a", "model": "m2"}]}"#).expect_err("deux noms égaux");
            assert!(e.contains("« a »"), "{e}");
            let seul = lire(r#"{"llm": [{"name": "seul", "model": "m1"}]}"#).expect("une liste d'un seul élément le prend pour défaut");
            assert_eq!(seul.get(&Capability::Llm).map(|s| s.model.as_str()), Some("m1"));
        }

        #[test]
        fn un_nom_inconnu_est_refuse_en_listant_les_noms() {
            let m = lire(r#"{"llm": [{"name": "a", "model": "m1", "default": true}, {"name": "b", "model": "m2"}]}"#).unwrap();
            let e = m.modele_de_langage("c").expect_err("inconnu");
            assert!(e.contains("« c »") && e.contains("a, b"), "{e}");
        }

        /// **Le budget du contexte d'après une compaction** (96, `docs/memoire/15-11-2026/03-la-compaction-est-l-archiviste.md`) :
        /// déclaré, il est pris ; absent, 100 000 jetons pour un modèle cloud, la moitié de sa fenêtre
        /// (au plus 100 000) pour un modèle local.
        #[test]
        fn le_budget_de_reprise_suit_le_modele() {
            let m = lire(r#"{"llm": [
                {"name": "cloud", "provider": "compatible", "protocol": "vertex", "model": "gemini-2.5-pro", "default": true},
                {"name": "regle", "provider": "compatible", "protocol": "anthropic", "model": "claude", "restart_budget_tokens": 50000},
                {"name": "local", "provider": "compatible", "protocol": "openai", "address": "http://127.0.0.1:8080/v1", "model": "qwen", "context_tokens": 32768}
            ]}"#)
            .expect("trois modèles");
            assert_eq!(m.modele_de_langage("cloud").unwrap().budget_de_reprise(), 100_000);
            assert_eq!(m.modele_de_langage("regle").unwrap().budget_de_reprise(), 50_000);
            assert_eq!(m.modele_de_langage("local").unwrap().budget_de_reprise(), 16_384);
            let seul = lire(r#"{"llm": {"provider": "compatible", "protocol": "vertex", "model": "g", "restart_budget_tokens": 80000}}"#).expect("l'objet unique le porte aussi");
            assert_eq!(seul.get(&Capability::Llm).unwrap().budget_de_reprise(), 80_000);
        }

        /// **A llama-server declared on a loopback address is local** (ticket of 15 November 2026, « un
        /// llama-server déclaré est classé cloud ») : it is declared `compatible`/`openai` at its address, not by a
        /// protocol of its own, and it was given the cloud budget. A loopback address, with or without a port,
        /// makes it local; any other address stays cloud.
        #[test]
        fn a_llama_server_on_a_loopback_address_has_a_local_budget() {
            for address in ["http://127.0.0.1:8080/v1", "http://localhost:8080/v1", "http://[::1]:8080/v1", "http://localhost/v1"] {
                let m = lire(&format!(r#"{{"llm": {{"provider": "compatible", "protocol": "openai", "address": "{address}", "model": "qwen", "context_tokens": 32000}}}}"#)).expect("a llama-server");
                assert_eq!(m.get(&Capability::Llm).unwrap().budget_de_reprise(), 16_000, "{address} is a loopback address");
            }
            let distant = lire(r#"{"llm": {"provider": "compatible", "protocol": "openai", "address": "https://api.example.com/v1", "model": "m", "context_tokens": 32000}}"#).unwrap();
            assert_eq!(distant.get(&Capability::Llm).unwrap().budget_de_reprise(), 100_000, "a remote address stays cloud");
            let lookalike = lire(r#"{"llm": {"provider": "compatible", "protocol": "openai", "address": "http://localhost.example.com/v1", "model": "m", "context_tokens": 32000}}"#).unwrap();
            assert_eq!(lookalike.get(&Capability::Llm).unwrap().budget_de_reprise(), 100_000, "a host that only starts like localhost is not local");
        }

        /// **`access` and `provider`, the words of a declared model** (ticket of 15 November 2026, decided by Lucie):
        /// `access` says how the model is reached (`local`, `service`, `compatible`), `provider` who serves it
        /// (`openai`, `vertex`…). The old form (`provider: compatible`, `protocol: vertex`) still reads, into the
        /// same declaration, until 29 November 2026; a model that mixes both forms is refused, naming both words.
        /// A declaration is written back in the new form.
        #[test]
        fn access_and_provider_read_like_the_old_provider_and_protocol() {
            let new = lire(r#"{"llm": {"access": "compatible", "provider": "vertex", "model": "g", "params": {"project": "p"}}}"#).expect("the new form");
            let old = lire(r#"{"llm": {"provider": "compatible", "protocol": "vertex", "model": "g", "params": {"project": "p"}}}"#).expect("the old form");
            assert_eq!(new.get(&Capability::Llm), old.get(&Capability::Llm), "the same declaration");
            let s = new.get(&Capability::Llm).unwrap();
            assert_eq!((s.provider, s.protocol.as_deref()), (Provider::Compatible, Some("vertex")));
            let local_new = lire(r#"{"embed": {"access": "local", "model": "e"}}"#).expect("access alone");
            let local_old = lire(r#"{"embed": {"provider": "local", "model": "e"}}"#).expect("the old provider alone");
            assert_eq!(local_new.get(&Capability::Embed), local_old.get(&Capability::Embed));
            for mixed in [
                r#"{"llm": {"access": "compatible", "protocol": "openai", "model": "m"}}"#,
                r#"{"llm": {"access": "compatible", "provider": "compatible", "model": "m"}}"#,
            ] {
                let e = lire(mixed).expect_err("a mixed form");
                assert!(e.contains("access") && (e.contains("protocol") || e.contains("provider")), "the refusal names both words: {e}");
            }
            let written = serde_json::to_value(s).unwrap();
            assert_eq!((written.get("access"), written.get("provider"), written.get("protocol")), (Some(&serde_json::json!("compatible")), Some(&serde_json::json!("vertex")), None), "{written}");
        }

        #[test]
        fn seul_models_llm_se_declare_en_liste() {
            let e = lire(r#"{"embed": [{"name": "a", "model": "m1", "default": true}]}"#).expect_err("embed en liste");
            assert!(e.contains("embed"), "{e}");
        }
    }

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

    fn source_gabarit_bge(nom: &str) -> ModelSource {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("templates/backends").join(nom).join("backend.json");
        let manifeste: crate::backend::BackendManifest = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        manifeste.embed_source().unwrap()
    }

    #[test]
    fn gabarits_bge_choisissent_le_modele_dans_les_adresses_du_poste() {
        for nom in ["notebook", "validated-result"] {
            let source = source_gabarit_bge(nom);
            assert_eq!((source.provider, source.model.as_str(), source.dimensions, source.fallback.clone()),
                (Provider::Service, "bge-m3", Some(1024), Fallback::Refuse));
            assert!(source.address.is_empty(), "{nom} ne doit pas imposer le port d'un autre poste : {:?}", source.address);
            let env = |k: &str| (k == "RAG3WEAVER_SERVICE_EMBED").then(|| "a:1,b:1".to_string());
            let (client, origine) = resolve(Capability::Embed, &source, &all(), &env).unwrap();
            assert_eq!((client, origine), ("client de b".into(), Origin::Service("b:1".into())));
            let alias = |k: &str| (k == "RAG3WEAVER_EMBED_SERVICE").then(|| "a:1,b:1".to_string());
            let (_, origine) = resolve(Capability::Embed, &source, &all(), &alias).unwrap();
            assert_eq!(origine, Origin::Service("b:1".into()));
            let commune_et_alias = |k: &str| match k {
                "RAG3WEAVER_SERVICE_EMBED" => Some("b:1".to_string()),
                "RAG3WEAVER_EMBED_SERVICE" => Some("a:1".to_string()),
                _ => None,
            };
            assert_eq!(source.addresses(Capability::Embed, &commune_et_alias), ["b:1"]);
        }
    }

    #[test]
    fn gabarits_bge_refusent_sans_fournisseur_compatible_et_sans_repli_implicite() {
        let appels_locaux = std::cell::Cell::new(0);
        let local = |_: &ModelSource| { appels_locaux.set(appels_locaux.get() + 1); Ok("local interdit".to_string()) };
        let builders = Builders { local: Some(&local), service: Some(&service), compatible: None };
        for nom in ["notebook", "validated-result"] {
            let source = source_gabarit_bge(nom);
            let erreur = resolve(Capability::Embed, &source, &builders, &no_env).unwrap_err();
            assert!(erreur.contains("models.embed (bge-m3)") && erreur.contains("RAG3WEAVER_SERVICE_EMBED"), "{nom}: {erreur}");
            let mauvais = ModelSource { address: Addresses::parse("a:1"), ..source.clone() };
            let env_bge = |k: &str| (k == "RAG3WEAVER_SERVICE_EMBED").then(|| "b:1".to_string());
            let erreur = resolve(Capability::Embed, &mauvais, &builders, &env_bge).unwrap_err();
            assert!(erreur.contains("bge-m3") && erreur.contains("a:1 sert granite-278m"), "{erreur}");
            assert_eq!(mauvais.addresses(Capability::Embed, &env_bge), ["a:1"], "l'environnement ne remplace pas une adresse explicitement écrite");
            assert_eq!(source.dimensions, Some(1024));
        }
        assert_eq!(appels_locaux.get(), 0, "aucune construction de modèle local");
    }

    #[test]
    fn gabarits_bge_qualifient_les_signaux_sans_ouvrir_la_base() {
        for (nom, requis) in [("notebook", true), ("validated-result", false)] {
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("templates/backends").join(nom).join("backend.json");
            let prepared = crate::backend::PreparedBackend::load(&p).unwrap();
            assert_eq!(prepared.needs_embeddings(), requis, "{nom}");
            assert_eq!(prepared.embed_source().model, "bge-m3");
            assert_eq!(prepared.embed_source().dimensions, Some(1024));
            assert!(!prepared.path(&prepared.manifest.database).exists(), "le chargement déclaratif n'ouvre pas la base");
        }
    }

    #[test]
    fn la_declaration_se_lit_et_garde_le_nom_d_avant() {
        let s: ModelSource = serde_json::from_str(r#"{"model":"granite-278m"}"#).unwrap();
        assert_eq!((s.provider, s.fallback.clone(), s.address.is_empty()), (Provider::Service, Fallback::Refuse, true));
        // `daemon` est le nom d'avant du fournisseur `service`.
        let s: ModelSource = serde_json::from_str(r#"{"provider":"daemon","model":"bge-m3","address":"a:1, b:1"}"#).unwrap();
        assert_eq!((s.provider, s.address.0.len()), (Provider::Service, 2));
        let s: ModelSource = serde_json::from_str(r#"{"provider":"compatible","protocol":"llama_server","model":"jevk5-4b","address":["http://x:7982"],"fallback":"local"}"#).unwrap();
        assert_eq!((s.provider, s.fallback.clone(), s.protocol.as_deref()), (Provider::Compatible, Fallback::Local, Some("llama_server")));
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
        // Une capacité que le démon d'embarquement ne sert pas ne lit pas ses variables.
        assert!(s.addresses(Capability::Decide, &old).is_empty());
        assert_eq!(s.addresses(Capability::Rerank, &old), ["a:1", "b:1"]);
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

    /// Le relecteur et l'OCR se déclarent en local ou par le démon ; sans
    /// adresse nulle part, le refus dit quelles variables poser ; un tiers
    /// est refusé avec sa raison, et un modèle inconnu se dit.
    #[test]
    fn le_relecteur_et_l_ocr_se_declarent_et_les_refus_disent_quoi() {
        #[cfg(feature = "daemon")]
        {
            let none = |_: &str| -> Option<String> { None };
            let _ = none;
            let mort = ModelSource { address: Addresses::parse("127.0.0.1:1"), ..ModelSource::service("bge-reranker-v2-m3") };
            let e = connect_reranker(&mort).err().expect("personne à cette adresse");
            assert!(e.contains("models.rerank") && e.contains("aucun service ne sert bge-reranker-v2-m3"), "{e}");
            let e = connect_ocr(&ModelSource { address: Addresses::parse("127.0.0.1:1"), ..ModelSource::service("ppocrv6-tiny") }).err().expect("personne");
            assert!(e.contains("models.ocr"), "{e}");
        }
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

    /// Le modèle de langage se déclare comme les autres : compatible OpenAI à
    /// une adresse (écrite ou par `RAG3WEAVER_SERVICE_LLM`), avec sa fenêtre.
    #[cfg(feature = "anthropic-llm")]
    #[test]
    fn le_protocole_anthropic_se_declare_sans_adresse_et_la_cle_par_sa_variable() {
        use crate::llm::Llm;
        let s: ModelSource = serde_json::from_value(serde_json::json!({
            "provider": "compatible", "protocol": "anthropic", "model": "claude-opus-5",
            "api_key_env": "RAG3WEAVER_TEST_ANTHROPIC_KEY", "context_tokens": 200000
        }))
        .unwrap();
        std::env::remove_var("RAG3WEAVER_TEST_ANTHROPIC_KEY");
        let err = connect_llm(&s).err().expect("sans clé, un refus");
        assert!(err.contains("RAG3WEAVER_TEST_ANTHROPIC_KEY") && !err.contains("sk-"), "{err}");
        std::env::set_var("RAG3WEAVER_TEST_ANTHROPIC_KEY", "sk-test");
        let (llm, origin) = connect_llm(&s).expect("la déclaration suffit");
        assert_eq!(origin, Origin::Compatible("anthropic".into()));
        assert_eq!((llm.name(), llm.context_len()), ("claude-opus-5", 200_000));
        std::env::remove_var("RAG3WEAVER_TEST_ANTHROPIC_KEY");
    }

    #[cfg(feature = "openai-llm")]
    #[test]
    fn le_modele_de_langage_se_declare_comme_les_autres() {
        use crate::llm::Llm;
        let s: ModelSource = serde_json::from_value(serde_json::json!({
            "provider": "compatible", "model": "qwen2.5-7b-instruct", "address": "http://127.0.0.1:7983/v1", "context_tokens": 32768
        }))
        .unwrap();
        let (llm, origin) = connect_llm(&s).expect("la déclaration suffit");
        assert_eq!(origin, Origin::Compatible("http://127.0.0.1:7983/v1".into()));
        assert_eq!((llm.name(), llm.context_len()), ("qwen2.5-7b-instruct", 32_768));
        assert_eq!(Capability::Llm.variable(), "RAG3WEAVER_SERVICE_LLM");

        // Sans adresse nulle part, une adresse qui n'en est pas une, un
        // protocole inconnu, une forme qui n'existe pas : chaque refus dit quoi.
        let sans = ModelSource { provider: Provider::Compatible, ..ModelSource::local("m") };
        if process_env("RAG3WEAVER_SERVICE_LLM").is_none() {
            assert!(connect_llm(&sans).err().expect("pas d'adresse").contains("RAG3WEAVER_SERVICE_LLM"));
        }
        let mal = ModelSource { address: Addresses::parse("127.0.0.1:7983"), ..sans.clone() };
        assert!(connect_llm(&mal).err().expect("pas http").contains("http(s)"));
        let autre = ModelSource { protocol: Some("grpc".into()), address: Addresses::parse("http://x"), ..sans.clone() };
        assert!(connect_llm(&autre).err().expect("protocole").contains("openai, vertex"));
        let local = connect_llm(&ModelSource::local("m")).err().expect("pas de forme locale");
        assert!(local.contains("models.llm") && local.contains("pas de forme locale"), "{local}");
        // Les paramètres propres à un protocole se lisent, et un champ inconnu reste une erreur.
        let v: ModelSource = serde_json::from_value(serde_json::json!({
            "provider": "compatible", "protocol": "vertex", "model": "google/gemini-3.5-flash", "params": {"project": "p", "location": "europe-west1"}
        }))
        .unwrap();
        assert_eq!((v.params["project"].as_str(), v.params["location"].as_str()), ("p", "europe-west1"));
        assert!(serde_json::from_value::<ModelSource>(serde_json::json!({"model": "m", "project": "p"})).is_err());
    }
}
