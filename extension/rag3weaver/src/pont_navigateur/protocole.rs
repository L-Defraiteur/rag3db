//! **Le protocole de l'extension**, révision 5 (`src/protocol.ts` de jackwener/browsermcp au SHA
//! `d6597aacb`). Les champs absents ne s'écrivent pas ; les champs inconnus venus de l'extension
//! sont tolérés.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// La révision du protocole que cet hôte parle.
pub const REVISION: u32 = 5;
/// Le nom de l'hôte Native Messaging de l'extension du fork (l'amont disait `com.browsermcp.host`).
pub const NOM_HOTE: &str = super::hote_natif::NOM_HOTE_NATIF;

/// **Une commande** de l'hôte vers l'extension. `id` est posé par le [`crate::pont_navigateur::pont::Pont`].
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commande {
    pub id: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    /// L'onglet Chrome, en chaîne : l'extension vérifie qu'il est à la session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chrome_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chrome_args: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdp_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdp_params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
    /// `deliverable`, `handoff`, ou `null` pour retirer la marque : la valeur JSON telle quelle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep: Option<Value>,
    /// L'`ActSpec` de l'amont, tel quel : l'extension le résout et le joue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// L'échéance en millisecondes depuis l'époque Unix, posée par le pont.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_at: Option<u64>,
}

impl Commande {
    /// Une commande d'`action`, sans `id` (le pont le pose).
    pub fn nouvelle(action: &str) -> Commande {
        Commande { action: action.to_string(), ..Default::default() }
    }
}

/// `claim` : `tabId` ou `active` choisissent un onglet ; `url` ou `title` en trouvent un seul ;
/// `expectedUrl` et `expectedTitle` vérifient son identité.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_title: Option<String>,
}

/// **Le résultat d'une commande**, à son `id`.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resultat {
    pub id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

/// **Un événement** du navigateur : `kind` dit lequel (`tab_acquired`, `tab_released`,
/// `tab_closed`, `tab_navigated`, `session_released`…) ; le reste est gardé.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Evenement {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `user` ou `agent`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(flatten)]
    pub reste: serde_json::Map<String, Value>,
}

/// L'enveloppe de l'hôte vers l'extension.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum VersExtension {
    Command { command: Commande },
    Ready { version: String },
}

/// L'enveloppe de l'extension vers l'hôte.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum VersHote {
    Hello {
        #[serde(rename = "browserId")]
        browser_id: String,
        #[serde(rename = "extensionVersion")]
        extension_version: String,
        #[serde(rename = "protocolRevision", default, skip_serializing_if = "Option::is_none")]
        protocol_revision: Option<u32>,
        #[serde(default)]
        features: Vec<String>,
    },
    Result { result: Resultat },
    Event { event: Evenement },
}
