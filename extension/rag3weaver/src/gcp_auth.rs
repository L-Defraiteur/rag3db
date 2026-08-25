//! Jetons OAuth2 Google Cloud, pour Vertex AI — **sans aucun crate de plus**.
//!
//! Aucun crate d'authentification GCP maintenu n'est synchrone : `gcp_auth`
//! (94 crates transitives) et `google-cloud-auth` (132) tirent tous tokio et
//! hyper, c'est-à-dire le réacteur qu'on refuse (cf. [`crate::openai_llm`]).
//! Or ce qu'il y a à faire tient en deux gestes : signer un JWT RS256 et
//! l'échanger contre un jeton. `ring` et `base64` sont déjà dans l'arbre via
//! `ureq` → `rustls`, donc ce module coûte zéro ligne de `Cargo.lock`.
//!
//! Deux sources d'identité, dans l'ordre où [`TokenSource::from_env`] les
//! cherche :
//!
//! 1. **Compte de service** — `GOOGLE_APPLICATION_CREDENTIALS` pointe sur le
//!    JSON téléchargé (`"type": "service_account"`). JWT RS256 signé avec la
//!    clé privée, échangé contre un jeton d'une heure.
//! 2. **ADC utilisateur** — `~/.config/gcloud/application_default_credentials.json`,
//!    écrit par `gcloud auth application-default login`
//!    (`"type": "authorized_user"`). Pas de JWT : un simple `refresh_token`.
//!
//! Le jeton vaut une heure ; [`TokenSource::token`] le renouvelle tout seul
//! une minute avant l'échéance. Aucun secret n'apparaît dans un `Debug`, dans
//! une erreur, ni dans un journal.

use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::Value;

use crate::llm::LlmError;

/// Portée demandée : c'est celle qu'exige l'API Vertex AI.
pub const CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";
const TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
/// Marge de renouvellement : on ne laisse jamais partir une requête avec un
/// jeton qui expire dans moins d'une minute.
const REFRESH_MARGIN: Duration = Duration::from_secs(60);

fn err(msg: impl Into<String>) -> LlmError {
    LlmError::Model(msg.into())
}

/// De quoi on dispose pour obtenir un jeton. Pas de `Debug` dérivé : ces
/// champs sont des secrets.
enum Credentials {
    ServiceAccount {
        client_email: String,
        /// Clé privée en DER PKCS#8 (le PEM du JSON, décodé).
        private_key_der: Vec<u8>,
        token_uri: String,
    },
    AuthorizedUser {
        client_id: String,
        client_secret: String,
        refresh_token: String,
    },
}

impl Credentials {
    fn kind(&self) -> &'static str {
        match self {
            Credentials::ServiceAccount { .. } => "service_account",
            Credentials::AuthorizedUser { .. } => "authorized_user",
        }
    }
}

/// Source de jetons, avec cache et renouvellement. `Send + Sync` : elle vit à
/// côté d'un [`crate::openai_llm::OpenAiLlm`] dans le registre de services.
pub struct TokenSource {
    creds: Credentials,
    scope: String,
    /// `(jeton, instant d'expiration)`.
    cached: Mutex<Option<(String, Instant)>>,
}

impl std::fmt::Debug for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenSource")
            .field("kind", &self.creds.kind())
            .field("scope", &self.scope)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl TokenSource {
    /// Cherche des identifiants comme le fait `gcloud` : d'abord
    /// `GOOGLE_APPLICATION_CREDENTIALS`, puis le fichier ADC bien connu.
    ///
    /// L'erreur dit quoi faire, jamais ce qu'elle a lu.
    pub fn from_env() -> Result<Self, LlmError> {
        if let Ok(path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            if !path.trim().is_empty() {
                return Self::from_file(&path);
            }
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| err("ni GOOGLE_APPLICATION_CREDENTIALS ni HOME ne sont définis"))?;
        let adc = format!("{home}/.config/gcloud/application_default_credentials.json");
        if std::path::Path::new(&adc).exists() {
            return Self::from_file(&adc);
        }
        Err(err(
            "aucun identifiant Google trouvé : pose GOOGLE_APPLICATION_CREDENTIALS \
             sur le JSON d'un compte de service, ou lance \
             `gcloud auth application-default login`",
        ))
    }

    /// Lit un fichier d'identifiants (compte de service ou ADC utilisateur).
    pub fn from_file(path: &str) -> Result<Self, LlmError> {
        let raw = std::fs::read_to_string(path)
            // Le chemin n'est pas un secret ; le contenu, si.
            .map_err(|e| err(format!("lecture de {path} impossible : {e}")))?;
        Self::from_json(&raw)
    }

    /// Parse le JSON d'identifiants. Le contenu n'est jamais réémis en erreur.
    pub fn from_json(raw: &str) -> Result<Self, LlmError> {
        let v: Value = serde_json::from_str(raw)
            .map_err(|e| err(format!("identifiants Google illisibles : {e}")))?;
        let get = |k: &str| -> Result<String, LlmError> {
            v[k].as_str()
                .map(str::to_string)
                .ok_or_else(|| err(format!("identifiants Google : champ `{k}` manquant")))
        };
        let creds = match v["type"].as_str() {
            Some("service_account") => Credentials::ServiceAccount {
                client_email: get("client_email")?,
                private_key_der: pem_to_der(&get("private_key")?)?,
                token_uri: v["token_uri"].as_str().unwrap_or(TOKEN_URI).to_string(),
            },
            Some("authorized_user") => Credentials::AuthorizedUser {
                client_id: get("client_id")?,
                client_secret: get("client_secret")?,
                refresh_token: get("refresh_token")?,
            },
            other => {
                return Err(err(format!(
                    "type d'identifiants Google non géré : {}",
                    other.unwrap_or("<absent>")
                )))
            }
        };
        Ok(Self { creds, scope: CLOUD_PLATFORM_SCOPE.to_string(), cached: Mutex::new(None) })
    }

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = scope.into();
        self
    }

    /// Le jeton d'accès courant, renouvelé si besoin. **Un appel réseau** vers
    /// `oauth2.googleapis.com` quand le cache est froid ou périmé.
    pub fn token(&self) -> Result<String, LlmError> {
        let mut cached = self.cached.lock().map_err(|_| err("TokenSource empoisonné"))?;
        if let Some((tok, exp)) = cached.as_ref() {
            if *exp > Instant::now() + REFRESH_MARGIN {
                return Ok(tok.clone());
            }
        }
        let (tok, ttl) = self.fetch()?;
        *cached = Some((tok.clone(), Instant::now() + ttl));
        Ok(tok)
    }

    /// Vide le cache — à appeler si le fournisseur a rendu 401.
    pub fn invalidate(&self) {
        if let Ok(mut c) = self.cached.lock() {
            *c = None;
        }
    }

    fn fetch(&self) -> Result<(String, Duration), LlmError> {
        let (url, form) = match &self.creds {
            Credentials::ServiceAccount { client_email, private_key_der, token_uri } => {
                let jwt = sign_jwt(client_email, &self.scope, token_uri, private_key_der)?;
                (
                    token_uri.clone(),
                    format!(
                        "grant_type={}&assertion={}",
                        form_encode("urn:ietf:params:oauth:grant-type:jwt-bearer"),
                        // Un JWT est du base64url : rien à échapper.
                        jwt
                    ),
                )
            }
            Credentials::AuthorizedUser { client_id, client_secret, refresh_token } => (
                TOKEN_URI.to_string(),
                format!(
                    "grant_type=refresh_token&client_id={}&client_secret={}&refresh_token={}",
                    form_encode(client_id),
                    form_encode(client_secret),
                    form_encode(refresh_token)
                ),
            ),
        };

        // `http_status_as_error(false)` : sans ça, un 400 (`invalid_grant`,
        // clé révoquée) devient une erreur ureq sans corps, et le message
        // utile — celui de Google — est perdu (25 août 2026).
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder().http_status_as_error(false).build(),
        );
        let mut resp = agent
            .post(&url)
            .header("content-type", "application/x-www-form-urlencoded")
            .send(form)
            // Ne jamais réémettre le corps envoyé : il porte l'assertion.
            .map_err(|e| err(format!("échange de jeton Google : {e}")))?;

        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        if status != 200 {
            // La réponse d'erreur d'OAuth2 ne contient pas le secret envoyé,
            // mais on la borne quand même.
            let mut b = body;
            b.truncate(256);
            return Err(err(format!("échange de jeton Google : HTTP {status}: {}", b.trim())));
        }
        let v: Value = serde_json::from_str(&body)
            .map_err(|_| err("réponse d'échange de jeton Google illisible"))?;
        let tok = v["access_token"]
            .as_str()
            .ok_or_else(|| err("réponse d'échange de jeton Google sans `access_token`"))?;
        let ttl = Duration::from_secs(v["expires_in"].as_u64().unwrap_or(3600));
        Ok((tok.to_string(), ttl))
    }
}

/// Décode le PEM PKCS#8 d'une clé privée de compte de service en DER.
/// N'affiche jamais le contenu, même tronqué.
fn pem_to_der(pem: &str) -> Result<Vec<u8>, LlmError> {
    let body: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .flat_map(|l| l.chars().filter(|c| !c.is_whitespace()))
        .collect();
    if body.is_empty() {
        return Err(err("clé privée du compte de service vide ou mal formée"));
    }
    STANDARD
        .decode(body.as_bytes())
        .map_err(|_| err("clé privée du compte de service : base64 invalide"))
}

/// Signe l'assertion JWT RS256 attendue par le flux `jwt-bearer` de Google.
fn sign_jwt(
    client_email: &str,
    scope: &str,
    audience: &str,
    private_key_der: &[u8],
) -> Result<String, LlmError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| err("horloge système avant 1970"))?
        .as_secs();
    // Une heure : c'est le maximum accepté par Google pour une assertion.
    let claims = serde_json::json!({
        "iss": client_email,
        "scope": scope,
        "aud": audience,
        "iat": now,
        "exp": now + 3600,
    });
    let header = br#"{"alg":"RS256","typ":"JWT"}"#;
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes())
    );

    let key = ring::signature::RsaKeyPair::from_pkcs8(private_key_der)
        .map_err(|_| err("clé privée du compte de service : PKCS#8 RSA invalide"))?;
    let mut sig = vec![0u8; key.public().modulus_len()];
    key.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        signing_input.as_bytes(),
        &mut sig,
    )
    .map_err(|_| err("signature RS256 du JWT impossible"))?;

    Ok(format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(&sig)))
}

/// Encodage `application/x-www-form-urlencoded` d'une valeur. Dix lignes
/// plutôt qu'une dépendance de plus.
fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_encode_escapes_what_matters() {
        assert_eq!(
            form_encode("urn:ietf:params:oauth:grant-type:jwt-bearer"),
            "urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer"
        );
        assert_eq!(form_encode("a/b+c d"), "a%2Fb%2Bc+d");
        assert_eq!(form_encode("Aa0-._~"), "Aa0-._~", "les non-réservés passent tels quels");
    }

    #[test]
    fn pem_to_der_strips_armor_and_whitespace() {
        // "hello" en base64 = aGVsbG8=
        let pem = "-----BEGIN PRIVATE KEY-----\naGVs\nbG8=\n-----END PRIVATE KEY-----\n";
        assert_eq!(pem_to_der(pem).unwrap(), b"hello");
        assert!(pem_to_der("-----BEGIN PRIVATE KEY-----\n-----END PRIVATE KEY-----").is_err());
        assert!(pem_to_der("pas du base64 !!!").is_err());
    }

    #[test]
    fn service_account_json_is_parsed_and_key_decoded() {
        let json = serde_json::json!({
            "type": "service_account",
            "client_email": "svc@projet.iam.gserviceaccount.com",
            "private_key": "-----BEGIN PRIVATE KEY-----\naGVsbG8=\n-----END PRIVATE KEY-----\n",
            "token_uri": "https://oauth2.googleapis.com/token"
        })
        .to_string();
        let ts = TokenSource::from_json(&json).unwrap();
        match &ts.creds {
            Credentials::ServiceAccount { client_email, private_key_der, token_uri } => {
                assert_eq!(client_email, "svc@projet.iam.gserviceaccount.com");
                assert_eq!(private_key_der, b"hello");
                assert_eq!(token_uri, TOKEN_URI);
            }
            _ => panic!("mauvais type"),
        }
        assert_eq!(ts.scope, CLOUD_PLATFORM_SCOPE);
    }

    #[test]
    fn authorized_user_json_is_parsed() {
        let json = serde_json::json!({
            "type": "authorized_user",
            "client_id": "id.apps.googleusercontent.com",
            "client_secret": "secret",
            "refresh_token": "1//refresh"
        })
        .to_string();
        let ts = TokenSource::from_json(&json).unwrap();
        assert_eq!(ts.creds.kind(), "authorized_user");
    }

    #[test]
    fn unknown_or_incomplete_credentials_are_refused() {
        assert!(TokenSource::from_json(r#"{"type":"external_account"}"#).is_err());
        assert!(TokenSource::from_json(r#"{"type":"service_account"}"#).is_err());
        assert!(TokenSource::from_json("pas du json").is_err());
    }

    #[test]
    fn no_secret_leaks_into_debug_or_errors() {
        let json = serde_json::json!({
            "type": "authorized_user",
            "client_id": "id",
            "client_secret": "SECRET-DE-LUCIE",
            "refresh_token": "1//SECRET-DE-LUCIE"
        })
        .to_string();
        let ts = TokenSource::from_json(&json).unwrap();
        let shown = format!("{ts:?}");
        assert!(!shown.contains("SECRET-DE-LUCIE"), "secret fuité : {shown}");
        assert!(shown.contains("redacted"));

        // Une clé privée illisible ne doit pas revenir dans le message.
        let json = serde_json::json!({
            "type": "service_account",
            "client_email": "a@b.c",
            "private_key": "-----BEGIN PRIVATE KEY-----\nSECRET-DE-LUCIE!!!\n-----END PRIVATE KEY-----"
        })
        .to_string();
        let e = TokenSource::from_json(&json).unwrap_err().to_string();
        assert!(!e.contains("SECRET-DE-LUCIE"), "secret fuité : {e}");
    }

    #[test]
    fn jwt_has_three_base64url_parts_and_the_expected_claims() {
        // Vraie clé RSA générée pour ce test, jamais un secret de production.
        let der = test_rsa_key();
        let jwt = sign_jwt("svc@p.iam.gserviceaccount.com", CLOUD_PLATFORM_SCOPE, TOKEN_URI, &der)
            .expect("signature");
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        // base64url sans remplissage : ni '+', ni '/', ni '='.
        assert!(!jwt.contains('+') && !jwt.contains('/') && !jwt.contains('='));

        let header = URL_SAFE_NO_PAD.decode(parts[0]).unwrap();
        let header: Value = serde_json::from_slice(&header).unwrap();
        assert_eq!(header["alg"], "RS256");
        assert_eq!(header["typ"], "JWT");

        let claims = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
        let claims: Value = serde_json::from_slice(&claims).unwrap();
        assert_eq!(claims["iss"], "svc@p.iam.gserviceaccount.com");
        assert_eq!(claims["aud"], TOKEN_URI);
        assert_eq!(claims["scope"], CLOUD_PLATFORM_SCOPE);
        let (iat, exp) = (claims["iat"].as_u64().unwrap(), claims["exp"].as_u64().unwrap());
        assert_eq!(exp - iat, 3600, "Google refuse au-delà d'une heure");

        // La signature se vérifie avec la clé publique correspondante : c'est
        // ce qui prouve qu'on a bien signé, pas seulement encodé.
        let pair = ring::signature::RsaKeyPair::from_pkcs8(&der).unwrap();
        let public = ring::signature::UnparsedPublicKey::new(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            pair.public().as_ref(),
        );
        let sig = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        public.verify(signing_input.as_bytes(), &sig).expect("signature invalide");
    }

    #[test]
    fn a_bogus_key_is_refused_without_echoing_it() {
        let e = sign_jwt("a@b.c", "s", TOKEN_URI, b"pas une cle").unwrap_err().to_string();
        assert!(e.contains("PKCS#8"));
        assert!(!e.contains("pas une cle"));
    }

    /// Clé RSA 2048 de test, PKCS#8 DER — générée hors ligne pour ce test.
    /// Aucune valeur de production ; elle ne protège rien.
    fn test_rsa_key() -> Vec<u8> {
        STANDARD.decode(include_str!("../tests/data/test_rsa_pkcs8.b64").trim().replace('\n', ""))
            .expect("clé de test illisible")
    }
}
