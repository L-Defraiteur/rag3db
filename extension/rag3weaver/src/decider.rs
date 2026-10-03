//! **La décision** — une capacité : un texte, une liste fermée d'options, une
//! probabilité par option.
//!
//! Premier client neuf de [`crate::model_source`] : s'il se déclare
//! « service llama-server à telle adresse » sans code à lui dans le graphe,
//! l'abstraction est la bonne.
//!
//! # Ce que la capacité ne sait pas
//!
//! **Rien de la formulation.** La question, les options enrichies d'extraits,
//! un temps ou deux : tout cela change encore (diagnostic en cours, 3 octobre
//! 2026 — « ça reste une expérimentation »). Le [`Decider`] reçoit un texte
//! **déjà composé** et les étiquettes des options telles que le modèle les
//! écrirait au jeton suivant (`A`, `B`… ou `oui`, `non`). Composer le texte
//! est l'affaire du graphe.
//!
//! # Ce qui est propre au fournisseur
//!
//! Lire les log-probabilités du jeton suivant. Pour llama-server : un
//! `/tokenize`, un `/completion` d'un seul jeton, et
//! `completion_probabilities[0].top_logprobs`. Le reste — restreindre aux
//! options, normaliser — est commun et pur ([`option_probabilities`]).

use std::fmt;

/// La clé du registre de services sous laquelle un nœud trouve le décideur
/// (`Arc<dyn Decider>`). Un backend l'enregistre quand son manifeste déclare
/// `models.decide` ; le nœud n'a ni adresse ni protocole à connaître.
pub const DECIDER_SERVICE: &str = "decider";

/// Une décision qui n'a pas pu être rendue.
#[derive(Debug, Clone, PartialEq)]
pub enum DecideError {
    /// Le fournisseur n'a pas répondu, ou pas ce qu'on attend.
    Provider(String),
    /// Aucune option n'apparaît parmi les jetons que le modèle envisage : le
    /// texte ne l'amène pas à choisir. C'est un défaut de formulation, pas
    /// une probabilité nulle.
    NoOptionInSight { seen: Vec<String> },
    /// La demande elle-même est mal formée.
    Invalid(String),
}

impl fmt::Display for DecideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(e) => write!(f, "décision : {e}"),
            Self::NoOptionInSight { seen } => {
                write!(f, "décision : aucune option parmi les jetons les plus probables ({}) — le texte n'amène pas le modèle à choisir", seen.join(", "))
            }
            Self::Invalid(e) => write!(f, "décision : {e}"),
        }
    }
}

impl std::error::Error for DecideError {}

/// **Décider** : une probabilité par option, dans l'ordre des options, de
/// somme 1.
///
/// **La calibration appartient à qui pose la question.** Le même modèle ne
/// se calibre pas pareil d'un critère à l'autre (mesuré le 3 octobre 2026 :
/// un seuil sans fausse fusion à 0,50 pour un critère, 0,07 pour un autre) ;
/// une température unique par modèle écraserait cet écart. Le nœud, qui
/// connaît le critère, la donne par [`Decider::decide_at`] ; [`Decider::decide`]
/// prend celle du fournisseur.
pub trait Decider: Send + Sync {
    /// Décide à cette température de softmax sur les options (1 : tel quel).
    fn decide_at(&self, prompt: &str, options: &[String], temperature: f64) -> Result<Vec<f64>, DecideError>;

    /// La température du fournisseur quand l'appelant n'en donne pas.
    fn default_temperature(&self) -> f64 {
        1.0
    }

    fn decide(&self, prompt: &str, options: &[String]) -> Result<Vec<f64>, DecideError> {
        self.decide_at(prompt, options, self.default_temperature())
    }

    fn name(&self) -> &str {
        "decider"
    }
}

impl<T: Decider + ?Sized> Decider for std::sync::Arc<T> {
    fn decide_at(&self, prompt: &str, options: &[String], temperature: f64) -> Result<Vec<f64>, DecideError> {
        (**self).decide_at(prompt, options, temperature)
    }
    fn default_temperature(&self) -> f64 {
        (**self).default_temperature()
    }
    fn name(&self) -> &str {
        (**self).name()
    }
}

/// **Des log-probabilités du jeton suivant aux probabilités des options.**
///
/// `top` : les jetons envisagés et leur log-probabilité. Un jeton compte pour
/// une option quand, espaces retirés, il lui est égal ; plusieurs jetons
/// peuvent écrire la même option (`A` et ` A`), leurs probabilités s'ajoutent.
/// Une option absente de `top` vaut zéro. `temperature` : la calibration du
/// softmax sur les options (1 : tel quel ; plus haut : plus plat).
pub fn option_probabilities(top: &[(String, f64)], options: &[String], temperature: f64) -> Result<Vec<f64>, DecideError> {
    if options.is_empty() {
        return Err(DecideError::Invalid("aucune option".into()));
    }
    if !(temperature > 0.0) {
        return Err(DecideError::Invalid(format!("température {temperature} : il faut un nombre positif")));
    }
    for (i, option) in options.iter().enumerate() {
        if option.trim().is_empty() || options[..i].iter().any(|o| o.trim() == option.trim()) {
            return Err(DecideError::Invalid(format!("option `{option}` vide ou en double")));
        }
    }
    // La masse de chaque option, en log : log(somme des exp) de ses jetons.
    let mut log_mass: Vec<Option<f64>> = vec![None; options.len()];
    for (token, logprob) in top {
        if !logprob.is_finite() {
            continue;
        }
        if let Some(i) = options.iter().position(|o| o.trim() == token.trim()) {
            log_mass[i] = Some(match log_mass[i] {
                None => *logprob,
                Some(m) => {
                    let (hi, lo) = if m > *logprob { (m, *logprob) } else { (*logprob, m) };
                    hi + (lo - hi).exp().ln_1p()
                }
            });
        }
    }
    let best = log_mass.iter().flatten().cloned().fold(f64::NEG_INFINITY, f64::max);
    if !best.is_finite() {
        return Err(DecideError::NoOptionInSight { seen: top.iter().take(8).map(|(t, _)| format!("`{t}`")).collect() });
    }
    let weights: Vec<f64> = log_mass.iter().map(|m| m.map_or(0.0, |m| ((m - best) / temperature).exp())).collect();
    let total: f64 = weights.iter().sum();
    Ok(weights.into_iter().map(|w| w / total).collect())
}

/// Un décideur de test : rend ce qu'on lui a donné.
#[derive(Debug, Clone)]
pub struct MockDecider(pub Vec<f64>);

impl Decider for MockDecider {
    fn decide_at(&self, _prompt: &str, options: &[String], _temperature: f64) -> Result<Vec<f64>, DecideError> {
        if self.0.len() != options.len() {
            return Err(DecideError::Invalid(format!("{} probabilités pour {} options", self.0.len(), options.len())));
        }
        Ok(self.0.clone())
    }
    fn name(&self) -> &str {
        "mock"
    }
}

// ─── llama-server ────────────────────────────────────────────────────────────

/// **llama-server** (llama.cpp), par ses routes natives : `/tokenize` puis
/// `/completion` d'un jeton avec ses log-probabilités.
#[cfg(feature = "daemon")]
pub struct LlamaServerDecider {
    base: String,
    model: String,
    /// Combien de jetons envisagés on demande. Une option au-delà vaut zéro.
    n_probs: usize,
    temperature: f64,
    client: ureq::Agent,
}

#[cfg(feature = "daemon")]
impl LlamaServerDecider {
    /// `base_url` : `http://hôte:port`, sans chemin.
    pub fn new(base_url: &str, model: &str) -> Result<Self, String> {
        if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
            return Err(format!("llama_server : `{base_url}` n'est pas une adresse http(s)"));
        }
        Ok(Self {
            base: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            n_probs: 40,
            temperature: 1.0,
            client: ureq::Agent::new_with_config(
                ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(std::time::Duration::from_secs(60))).build(),
            ),
        })
    }

    /// La calibration du softmax quand l'appelant n'en donne pas (défaut 1).
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_n_probs(mut self, n: usize) -> Self {
        self.n_probs = n.max(1);
        self
    }

    fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, DecideError> {
        let url = format!("{}{path}", self.base);
        let mut response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .send(body.to_string().as_bytes())
            .map_err(|e| DecideError::Provider(format!("{url} ne répond pas ({e})")))?;
        if !response.status().is_success() {
            return Err(DecideError::Provider(format!("{url} : HTTP {}", response.status())));
        }
        let text = response
            .body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| DecideError::Provider(format!("{url} : réponse illisible ({e})")))?;
        serde_json::from_str(&text).map_err(|e| DecideError::Provider(format!("{url} : JSON invalide ({e})")))
    }

    /// Le serveur répond-il ? (`GET /health`)
    pub fn health(&self) -> Result<(), DecideError> {
        let url = format!("{}/health", self.base);
        let response = self.client.get(&url).call().map_err(|e| DecideError::Provider(format!("{url} ne répond pas ({e})")))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(DecideError::Provider(format!("{url} : HTTP {}", response.status())))
        }
    }

    /// Les jetons envisagés pour la suite de `prompt`, avec leur
    /// log-probabilité.
    pub fn next_token_logprobs(&self, prompt: &str) -> Result<Vec<(String, f64)>, DecideError> {
        // Les jetons d'abord : le texte peut porter les jetons spéciaux du
        // gabarit du modèle, que `/completion` ne lirait pas dans une chaîne.
        let tokens = self.post("/tokenize", serde_json::json!({ "content": prompt, "add_special": false, "parse_special": true }))?;
        let tokens = tokens.get("tokens").cloned().ok_or_else(|| DecideError::Provider("/tokenize : champ `tokens` absent".into()))?;
        let reply = self.post(
            "/completion",
            serde_json::json!({ "prompt": tokens, "n_predict": 1, "n_probs": self.n_probs, "temperature": 0, "cache_prompt": false }),
        )?;
        let top = reply
            .pointer("/completion_probabilities/0/top_logprobs")
            .and_then(|v| v.as_array())
            .ok_or_else(|| DecideError::Provider("/completion : `completion_probabilities[0].top_logprobs` absent — un llama.cpp trop ancien rend `probs` à la place".into()))?;
        top.iter()
            .map(|entry| match (entry.get("token").and_then(|t| t.as_str()), entry.get("logprob").and_then(|l| l.as_f64())) {
                (Some(token), Some(logprob)) => Ok((token.to_string(), logprob)),
                _ => Err(DecideError::Provider("/completion : une entrée de `top_logprobs` n'a pas `token` et `logprob`".into())),
            })
            .collect()
    }
}

#[cfg(feature = "daemon")]
impl Decider for LlamaServerDecider {
    fn decide_at(&self, prompt: &str, options: &[String], temperature: f64) -> Result<Vec<f64>, DecideError> {
        option_probabilities(&self.next_token_logprobs(prompt)?, options, temperature)
    }
    fn default_temperature(&self) -> f64 {
        self.temperature
    }
    fn name(&self) -> &str {
        &self.model
    }
}

/// **Le décideur d'une déclaration**, et d'où il calcule. Aujourd'hui une
/// seule forme : `compatible`, protocole `llama_server`. Ni forme locale ni
/// démon à nous — la déclaration qui les demande est refusée en le disant.
pub fn connect_decider(
    source: &crate::model_source::ModelSource,
) -> Result<(std::sync::Arc<dyn Decider>, crate::model_source::Origin), String> {
    #[allow(unused_imports)]
    use crate::model_source::{resolve, Builders, Capability, ModelSource};
    type Client = std::sync::Arc<dyn Decider>;
    #[cfg(feature = "daemon")]
    let compatible = |address: &str, s: &ModelSource| -> Result<Client, String> {
        match s.protocol.as_deref() {
            Some("llama_server") => {
                let decider = LlamaServerDecider::new(address, &s.model)?;
                decider.health().map_err(|e| e.to_string())?;
                Ok(std::sync::Arc::new(decider) as Client)
            }
            other => Err(format!("protocole `{}` inconnu pour la décision (llama_server)", other.unwrap_or("absent"))),
        }
    };
    let builders: Builders<Client> = Builders {
        local: None,
        service: None,
        #[cfg(feature = "daemon")]
        compatible: Some(&compatible),
        #[cfg(not(feature = "daemon"))]
        compatible: None,
    };
    resolve(Capability::Decide, source, &builders, &crate::model_source::process_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|l| l.to_string()).collect()
    }
    fn top(entries: &[(&str, f64)]) -> Vec<(String, f64)> {
        entries.iter().map(|(t, l)| (t.to_string(), *l)).collect()
    }

    #[test]
    fn les_probabilites_se_restreignent_aux_options_et_somment_a_un() {
        // ln 0,6 ; ln 0,3 ; un jeton hors options qui prend le reste.
        let t = top(&[("A", 0.6f64.ln()), ("B", 0.3f64.ln()), ("Le", 0.1f64.ln())]);
        let p = option_probabilities(&t, &options(&["A", "B"]), 1.0).unwrap();
        assert!((p[0] - 2.0 / 3.0).abs() < 1e-9 && (p[1] - 1.0 / 3.0).abs() < 1e-9, "{p:?}");
        assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        // L'ordre rendu est celui des options, pas celui des jetons.
        let p = option_probabilities(&t, &options(&["B", "A"]), 1.0).unwrap();
        assert!(p[0] < p[1], "{p:?}");
    }

    #[test]
    fn deux_jetons_pour_la_meme_option_s_ajoutent_et_une_option_absente_vaut_zero() {
        let t = top(&[(" A", 0.4f64.ln()), ("A", 0.2f64.ln()), ("B", 0.2f64.ln())]);
        let p = option_probabilities(&t, &options(&["A", "B", "C"]), 1.0).unwrap();
        assert!((p[0] - 0.75).abs() < 1e-9 && (p[1] - 0.25).abs() < 1e-9, "{p:?}");
        assert_eq!(p[2], 0.0, "C n'est pas parmi les jetons envisagés");
    }

    #[test]
    fn la_temperature_aplatit_sans_changer_l_ordre() {
        let t = top(&[("oui", 0.9f64.ln()), ("non", 0.1f64.ln())]);
        let tel_quel = option_probabilities(&t, &options(&["oui", "non"]), 1.0).unwrap();
        let aplati = option_probabilities(&t, &options(&["oui", "non"]), 1.367).unwrap();
        assert!((tel_quel[0] - 0.9).abs() < 1e-9);
        assert!(aplati[0] < tel_quel[0] && aplati[0] > 0.5, "{aplati:?}");
    }

    #[test]
    fn un_texte_qui_n_amene_aucune_option_est_une_erreur_pas_un_zero() {
        let t = top(&[("Le", -0.2), (" chat", -2.0)]);
        let e = option_probabilities(&t, &options(&["A", "B"]), 1.0).unwrap_err();
        assert!(matches!(&e, DecideError::NoOptionInSight { seen } if seen.len() == 2), "{e:?}");
        assert!(e.to_string().contains("n'amène pas le modèle à choisir"), "{e}");
        // Une demande mal formée se dit aussi.
        assert!(matches!(option_probabilities(&t, &[], 1.0), Err(DecideError::Invalid(_))));
        assert!(matches!(option_probabilities(&t, &options(&["A", " A"]), 1.0), Err(DecideError::Invalid(_))));
        assert!(matches!(option_probabilities(&t, &options(&["A"]), 0.0), Err(DecideError::Invalid(_))));
    }

    /// **Un faux llama-server** : le client envoie des jetons, un seul jeton
    /// demandé, et lit `top_logprobs`.
    #[cfg(feature = "daemon")]
    #[test]
    fn le_client_llama_server_tokenise_puis_lit_les_log_probabilites() {
        use std::io::Read;
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let worker = std::thread::spawn(move || {
            for expected in ["/health", "/tokenize", "/completion"] {
                let mut request = server.recv_timeout(std::time::Duration::from_secs(5)).unwrap().unwrap();
                assert_eq!(request.url(), expected);
                let mut body = String::new();
                Read::read_to_string(request.as_reader(), &mut body).unwrap();
                let reply = match expected {
                    "/health" => serde_json::json!({"status": "ok"}),
                    "/tokenize" => {
                        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
                        assert_eq!(b, serde_json::json!({"content": "Garder ? Réponse :", "add_special": false, "parse_special": true}));
                        serde_json::json!({"tokens": [1, 2, 3]})
                    }
                    _ => {
                        let b: serde_json::Value = serde_json::from_str(&body).unwrap();
                        assert_eq!(b["prompt"], serde_json::json!([1, 2, 3]), "le prompt part en jetons");
                        assert_eq!((b["n_predict"].as_i64(), b["n_probs"].as_i64(), b["cache_prompt"].as_bool()), (Some(1), Some(40), Some(false)));
                        serde_json::json!({"completion_probabilities": [{"token": " oui", "logprob": -0.2, "top_logprobs": [
                            {"token": " oui", "logprob": 0.8f64.ln(), "id": 7, "bytes": []},
                            {"token": " non", "logprob": 0.2f64.ln(), "id": 8, "bytes": []}
                        ]}]})
                    }
                };
                request.respond(tiny_http::Response::from_string(reply.to_string())).unwrap();
            }
        });
        // Par la déclaration, comme un backend le ferait.
        let source: crate::model_source::ModelSource = serde_json::from_value(
            serde_json::json!({"provider": "compatible", "protocol": "llama_server", "model": "jevk5-4b", "address": address}),
        )
        .unwrap();
        let (decider, origin) = connect_decider(&source).expect("la déclaration suffit");
        assert!(origin.remote());
        assert_eq!(decider.name(), "jevk5-4b");
        let p = decider.decide("Garder ? Réponse :", &options(&["oui", "non"])).unwrap();
        worker.join().unwrap();
        assert!((p[0] - 0.8).abs() < 1e-9 && (p[1] - 0.2).abs() < 1e-9, "{p:?}");
    }

    /// Ni forme locale ni démon : la déclaration qui les demande est refusée
    /// avec sa raison, et un service injoignable ne se replie sur rien.
    #[test]
    fn la_decision_n_a_qu_une_forme_et_le_dit() {
        use crate::model_source::ModelSource;
        let e = connect_decider(&ModelSource::local("jevk5-4b")).err().expect("pas de forme locale");
        assert!(e.contains("models.decide") && e.contains("pas de forme locale"), "{e}");
        let e = connect_decider(&ModelSource::service("jevk5-4b")).err().expect("pas de démon");
        assert!(e.contains("pas encore de démon"), "{e}");
    }

    /// Le vrai service, quand il est là : `RAG3WEAVER_SERVICE_DECIDE` posée
    /// (par exemple `http://127.0.0.1:7982`, le tunnel vers l'autre poste).
    #[cfg(feature = "daemon")]
    #[test]
    #[ignore]
    fn le_vrai_service_de_decision_repond() {
        let source: crate::model_source::ModelSource =
            serde_json::from_value(serde_json::json!({"provider": "compatible", "protocol": "llama_server", "model": "jevk5-4b"})).unwrap();
        let (decider, origin) = connect_decider(&source).expect("RAG3WEAVER_SERVICE_DECIDE doit désigner un llama-server");
        let t = std::time::Instant::now();
        // Des étiquettes d'un seul jeton : « Vrai » s'écrit en plusieurs, et
        // le service le dit (`NoOptionInSight`, jetons vus : ` V`, ` F`…).
        let p = decider
            .decide("Paris est la capitale de la France.\nA : c'est vrai\nB : c'est faux\nRéponse (A ou B) :", &options(&["A", "B"]))
            .expect("décision");
        eprintln!("[decide] {origin:?} : A {:.3}, B {:.3} en {:?}", p[0], p[1], t.elapsed());
        assert!(p[0] > p[1], "{p:?}");
        assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    /// La calibration se donne à l'appel : le même décideur, deux critères.
    #[test]
    fn la_temperature_se_donne_par_appel() {
        struct Fixe;
        impl Decider for Fixe {
            fn decide_at(&self, _: &str, options: &[String], temperature: f64) -> Result<Vec<f64>, DecideError> {
                option_probabilities(&top(&[("A", 0.9f64.ln()), ("B", 0.1f64.ln())]), options, temperature)
            }
            fn default_temperature(&self) -> f64 {
                2.0
            }
        }
        let o = options(&["A", "B"]);
        let net = Fixe.decide_at("…", &o, 1.0).unwrap();
        let plat = Fixe.decide_at("…", &o, 4.0).unwrap();
        assert!(net[0] > plat[0] && plat[0] > 0.5, "{net:?} {plat:?}");
        // Sans température donnée : celle du fournisseur, pas 1.
        assert_eq!(Fixe.decide("…", &o).unwrap(), Fixe.decide_at("…", &o, 2.0).unwrap());
        // Et par un Arc, comme un nœud le reçoit du registre de services.
        let partage: std::sync::Arc<dyn Decider> = std::sync::Arc::new(Fixe);
        assert_eq!(partage.default_temperature(), 2.0);
        assert_eq!(partage.decide("…", &o).unwrap(), plat_at(&partage, &o, 2.0));
        fn plat_at(d: &std::sync::Arc<dyn Decider>, o: &[String], t: f64) -> Vec<f64> {
            d.decide_at("…", o, t).unwrap()
        }
    }
}
