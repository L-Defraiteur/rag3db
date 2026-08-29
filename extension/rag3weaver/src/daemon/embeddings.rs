//! **Un modèle chargé une fois, servi à plusieurs.**
//!
//! Mesuré le 29 août 2026 : une passe E2E rechargeait BGE-M3 **sept fois**, et
//! sept chargements concurrents prenaient 531, 465, 341, 280, 175, 151 et 55 s
//! — 2 047 s de chargement contre 1 111 s de tests. Ce n'est pas le chargement
//! qui est lent (5 s tout seul), c'est la concurrence : sept processus tirant
//! chacun 2,2 Go du disque vers la même carte.
//!
//! **La file d'attente est le remède, pas un défaut.** Un démon ne rend pas un
//! embarquement plus rapide ; il transforme sept chargements simultanés en un
//! chargement et une file. C'est pour ça que les embarquements se sérialisent
//! ici sur un verrou : la carte est une ressource unique, et prétendre le
//! contraire est exactement ce qui a coûté les 2 047 s.
//!
//! # Ce qui traverse le fil
//!
//! - `GET /sante` → l'identité ([`Identite`]) : qui je suis, quel modèle, quelle
//!   dimension, **et si je suis factice**.
//! - `POST /embed` `{"texts":[…]}` → `{"vectors":[[…]]}`.
//! - `POST /embed_dual` → `{"dense":[[…]],"sparse":[{"indices":[…],"values":[…]}]}`.
//!
//! # Le factice doit traverser
//!
//! [`DaemonEmbedder::is_mock`] relaie ce que le serveur déclare. Sans ça, poser
//! un démon devant un `HashEmbedder` **désarmerait en silence** le garde-fou de
//! `Catalog::register_entity`, qui refuse les montages produisant des scores
//! plausibles et faux. Un intermédiaire ne doit jamais blanchir ce qu'il
//! transporte.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::{agent, base_url, erreur, identite_de, lire_json, poster, sonde, DaemonError, Service};
use crate::embedder::{DualEmbedder, EmbedError, Embedder};
use crate::serveur::{Fin, Serveur};
use crate::sparse_index::SparseVector;

/// Le nom que ce démon se donne.
pub const SERVICE: &str = "rag3weaver-embeddings";

/// Ce que `GET /sante` rend. **Tout ce qu'un client a besoin de savoir avant
/// d'envoyer quoi que ce soit** : la dimension pour se câbler, le caractère
/// factice pour ne pas mentir au catalogue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identite {
    /// Toujours [`SERVICE`].
    pub service: String,
    /// Le nom du modèle servi, tel que l'embedder le donne.
    pub modele: String,
    /// Dimension des vecteurs denses.
    pub dim: usize,
    /// Sait-il faire `embed_dual` (dense + creux en une passe) ?
    pub dual: bool,
    /// **Est-ce un embedder factice ?** Relayé tel quel jusqu'au catalogue.
    pub factice: bool,
}

// ─── Le serveur ──────────────────────────────────────────────────────────────

/// **Le démon** : il tient le modèle et répond aux embarquements.
pub struct EmbedDaemon {
    embedder: Arc<dyn Embedder>,
    dual: Option<Arc<dyn DualEmbedder>>,
    fils: usize,
    /// La carte est unique : une passe à la fois. Voir l'en-tête.
    passe: Mutex<()>,
}

impl EmbedDaemon {
    /// Un démon qui sert cet embedder.
    pub fn new(embedder: Arc<dyn Embedder>) -> Self {
        Self { embedder, dual: None, fils: 4, passe: Mutex::new(()) }
    }

    /// Le même modèle sait aussi rendre le creux en une passe : on l'expose.
    pub fn avec_dual(mut self, dual: Arc<dyn DualEmbedder>) -> Self {
        self.dual = Some(dual);
        self
    }

    /// Combien de fils répondent (défaut 4). Voir [`super::servir`].
    pub fn fils(mut self, n: usize) -> Self {
        self.fils = n.max(1);
        self
    }

    /// L'identité qu'il déclare.
    pub fn identite(&self) -> Identite {
        Identite {
            service: SERVICE.to_string(),
            modele: self.embedder.name().to_string(),
            dim: self.embedder.dim(),
            dual: self.dual.is_some(),
            factice: self.embedder.is_mock(),
        }
    }

    /// **Écoute, et ne rend jamais la main** (sauf erreur d'écoute).
    pub fn servir(self, adresse: &str) -> Result<(), DaemonError> {
        let fils = self.fils;
        super::servir(Arc::new(self), adresse, fils)
    }
}

#[derive(Deserialize)]
struct Textes {
    texts: Vec<String>,
}

impl Service for EmbedDaemon {
    fn nom(&self) -> &'static str {
        SERVICE
    }

    fn identite(&self) -> serde_json::Value {
        serde_json::to_value(EmbedDaemon::identite(self)).unwrap_or_default()
    }

    fn repondre(&self, methode: &str, chemin: &str, req: &mut tiny_http::Request) -> (u16, String) {
        match (methode, chemin) {
            ("POST", "/embed") => match lire_json::<Textes>(req) {
                Err(e) => (400, erreur(&e)),
                Ok(t) => {
                    let _passe = self.passe.lock();
                    match self.embedder.embed(&t.texts) {
                        Ok(v) => (200, serde_json::json!({ "vectors": v }).to_string()),
                        Err(e) => (500, erreur(&e.to_string())),
                    }
                }
            },
            ("POST", "/embed_dual") => match (&self.dual, lire_json::<Textes>(req)) {
                (None, _) => (404, erreur("ce démon ne sert pas le creux")),
                (Some(_), Err(e)) => (400, erreur(&e)),
                (Some(d), Ok(t)) => {
                    let _passe = self.passe.lock();
                    match d.embed_dual(&t.texts) {
                        Ok((dense, creux)) => (
                            200,
                            serde_json::json!({
                                "dense": dense,
                                "sparse": creux.iter().map(sparse_json).collect::<Vec<_>>(),
                            })
                            .to_string(),
                        ),
                        Err(e) => (500, erreur(&e.to_string())),
                    }
                }
            },
            _ => (404, erreur("route inconnue")),
        }
    }
}

fn sparse_json(v: &SparseVector) -> serde_json::Value {
    serde_json::json!({ "indices": v.indices, "values": v.values })
}

// ─── Le client ───────────────────────────────────────────────────────────────

/// **Un embedder qui vit dans un autre processus.**
///
/// Se comporte comme n'importe quel [`Embedder`] — c'est tout l'intérêt : ni le
/// catalogue ni un nœud n'ont à savoir que le modèle est ailleurs.
pub struct DaemonEmbedder {
    base: String,
    agent: ureq::Agent,
    identite: Identite,
    /// Gardée pour sa politique de fin ; sans ce champ, un démon lancé en
    /// `Fin::Arreter` mourrait aussitôt après avoir été assuré.
    _attache: Option<crate::serveur::Attache>,
}

impl std::fmt::Debug for DaemonEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonEmbedder")
            .field("base", &self.base)
            .field("identite", &self.identite)
            .finish()
    }
}

impl DaemonEmbedder {
    /// La description d'un démon d'embedding à cette adresse, lancé par ce
    /// programme — **avec la bonne sonde déjà réglée**.
    pub fn serveur(adresse: impl Into<String>, programme: impl Into<String>) -> Serveur {
        let adresse = adresse.into();
        Serveur::new(SERVICE, adresse.clone(), programme)
            .sonde(sonde(SERVICE))
            .arg("--adresse")
            .arg(adresse)
            // Un démon sert à survivre au processus qui l'a lancé.
            .fin(Fin::Laisser)
            // Charger 2,2 Go et compiler les noyaux prend bien plus que dix
            // secondes sur une carte froide.
            .attente(std::time::Duration::from_secs(300))
    }

    /// S'attacher à un démon qui **répond déjà**.
    pub fn joindre(adresse: &str) -> Result<Self, DaemonError> {
        Self::depuis(adresse, None)
    }

    /// **Le verbe courant** : s'il répond on s'y attache, sinon on le lance.
    pub fn assurer(serveur: &Serveur) -> Result<Self, DaemonError> {
        let attache = serveur.assurer()?;
        let adresse = attache.adresse().to_string();
        Self::depuis(&adresse, Some(attache))
    }

    fn depuis(adresse: &str, attache: Option<crate::serveur::Attache>) -> Result<Self, DaemonError> {
        let agent = agent();
        let identite: Identite = identite_de(&agent, adresse, SERVICE)?;
        Ok(Self { base: base_url(adresse), agent, identite, _attache: attache })
    }

    /// Ce que le démon déclare être.
    pub fn identite(&self) -> &Identite {
        &self.identite
    }

    /// Sait-il rendre le creux en une passe ?
    pub fn sait_dual(&self) -> bool {
        self.identite.dual
    }

    fn envoyer(&self, route: &str, textes: &[String]) -> Result<serde_json::Value, EmbedError> {
        let corps = serde_json::json!({ "texts": textes }).to_string();
        let texte = poster(&self.agent, &self.base, route, corps)
            .map_err(|e| EmbedError::ProviderError(e.to_string()))?;
        serde_json::from_str(&texte).map_err(|e| EmbedError::ProviderError(e.to_string()))
    }
}

impl Embedder for DaemonEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let v = self.envoyer("/embed", texts)?;
        serde_json::from_value(v["vectors"].clone())
            .map_err(|e| EmbedError::ProviderError(format!("champ 'vectors' : {e}")))
    }

    fn dim(&self) -> usize {
        self.identite.dim
    }

    /// **Relayé, jamais blanchi.** Voir l'en-tête du module.
    fn is_mock(&self) -> bool {
        self.identite.factice
    }

    fn name(&self) -> &str {
        &self.identite.modele
    }
}

impl DualEmbedder for DaemonEmbedder {
    fn embed_dual(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
        let v = self.envoyer("/embed_dual", texts)?;
        let dense: Vec<Vec<f32>> = serde_json::from_value(v["dense"].clone())
            .map_err(|e| EmbedError::ProviderError(format!("champ 'dense' : {e}")))?;
        let creux = v["sparse"]
            .as_array()
            .ok_or_else(|| EmbedError::ProviderError("champ 'sparse' absent".into()))?
            .iter()
            .map(|s| {
                let indices: Vec<u32> = serde_json::from_value(s["indices"].clone())
                    .map_err(|e| EmbedError::ProviderError(format!("'indices' : {e}")))?;
                let values: Vec<f32> = serde_json::from_value(s["values"].clone())
                    .map_err(|e| EmbedError::ProviderError(format!("'values' : {e}")))?;
                if indices.len() != values.len() {
                    return Err(EmbedError::ProviderError(
                        "vecteur creux mal formé : indices et valeurs de longueurs différentes".into(),
                    ));
                }
                Ok(SparseVector::new(indices, values))
            })
            .collect::<Result<Vec<_>, EmbedError>>()?;
        Ok((dense, creux))
    }

    fn dim(&self) -> usize {
        self.identite.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::MockEmbedder;
    use std::net::TcpListener;

    /// Un embedder reconnaissable : chaque texte donne un vecteur rempli de sa
    /// longueur, pour qu'un test distingue les réponses.
    #[derive(Debug)]
    struct Regle(usize);

    impl Embedder for Regle {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            Ok(texts.iter().map(|t| vec![t.len() as f32; self.0]).collect())
        }
        fn dim(&self) -> usize {
            self.0
        }
        fn name(&self) -> &str {
            "regle"
        }
    }

    impl DualEmbedder for Regle {
        fn embed_dual(
            &self,
            texts: &[String],
        ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
            let dense = Embedder::embed(self, texts)?;
            let creux = texts
                .iter()
                .map(|t| SparseVector::new(vec![t.len() as u32], vec![1.0]))
                .collect();
            Ok((dense, creux))
        }
        fn dim(&self) -> usize {
            self.0
        }
    }

    fn port_libre() -> String {
        let e = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = e.local_addr().unwrap().to_string();
        drop(e);
        a
    }

    /// Lance un démon dans un fil et rend son adresse, une fois qu'il répond.
    fn demon(d: EmbedDaemon) -> String {
        let adresse = port_libre();
        let a = adresse.clone();
        std::thread::spawn(move || {
            let _ = d.servir(&a);
        });
        for _ in 0..200 {
            if DaemonEmbedder::joindre(&adresse).is_ok() {
                return adresse;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("le démon n'a pas répondu");
    }

    #[test]
    fn il_se_nomme_et_dit_sa_dimension() {
        let adresse = demon(EmbedDaemon::new(Arc::new(Regle(8))));
        let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
        assert_eq!(c.identite().service, SERVICE);
        assert_eq!(c.identite().modele, "regle");
        assert_eq!(Embedder::dim(&c), 8);
        assert!(!c.sait_dual(), "aucun dual n'a été branché");
    }

    #[test]
    fn il_embarque_pour_de_vrai() {
        let adresse = demon(EmbedDaemon::new(Arc::new(Regle(4))));
        let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
        let v = c.embed(&["ab".to_string(), "abcde".to_string()]).expect("embed");
        assert_eq!(v, vec![vec![2.0; 4], vec![5.0; 4]]);
    }

    #[test]
    fn un_lot_vide_rend_un_lot_vide() {
        let adresse = demon(EmbedDaemon::new(Arc::new(Regle(4))));
        let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
        assert!(c.embed(&[]).expect("embed").is_empty());
    }

    #[test]
    fn le_creux_traverse_le_fil() {
        let regle = Arc::new(Regle(3));
        let adresse = demon(EmbedDaemon::new(regle.clone()).avec_dual(regle));
        let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
        assert!(c.sait_dual());
        let (dense, creux) = c.embed_dual(&["abc".to_string()]).expect("dual");
        assert_eq!(dense, vec![vec![3.0; 3]]);
        assert_eq!(creux, vec![SparseVector::new(vec![3], vec![1.0])]);
    }

    #[test]
    fn demander_le_creux_a_qui_ne_le_sert_pas_se_dit() {
        let adresse = demon(EmbedDaemon::new(Arc::new(Regle(3))));
        let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
        let e = c.embed_dual(&["abc".to_string()]).expect_err("404 attendu");
        assert!(format!("{e}").contains("404"), "erreur : {e}");
    }

    /// **Le point qui compte.** Un démon devant un factice reste factice, sinon
    /// le garde-fou du catalogue tomberait sans que personne le voie.
    #[test]
    fn le_factice_traverse_le_fil() {
        let adresse = demon(EmbedDaemon::new(Arc::new(MockEmbedder::new(6))));
        let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
        assert!(c.is_mock(), "un démon ne doit pas blanchir ce qu'il transporte");
        assert!(c.identite().factice);
    }

    #[test]
    fn un_vrai_ne_se_declare_pas_factice() {
        let adresse = demon(EmbedDaemon::new(Arc::new(Regle(2))));
        assert!(!DaemonEmbedder::joindre(&adresse).expect("joindre").is_mock());
    }

    #[test]
    fn personne_au_bout_du_fil_se_dit() {
        match DaemonEmbedder::joindre(&port_libre()) {
            Err(DaemonError::Injoignable { .. }) => {}
            autre => panic!("attendu Injoignable, obtenu {autre:?}"),
        }
    }

    /// **On ne se trompe pas de démon.** Le démon de base répond à la même
    /// route, avec une autre identité : le client doit le refuser franchement.
    #[test]
    fn s_attacher_au_mauvais_demon_se_dit() {
        let adresse = port_libre();
        let a = adresse.clone();
        std::thread::spawn(move || {
            let _ = super::super::db::DbDaemon::new(Arc::new(
                crate::connection::CallbackConnection::new(|_, _| {
                    Ok(crate::connection::QueryResult::default())
                }),
            ))
            .servir(&a);
        });
        for _ in 0..200 {
            match DaemonEmbedder::joindre(&adresse) {
                Err(DaemonError::Etranger { trouve, .. }) => {
                    assert_eq!(trouve, super::super::db::SERVICE);
                    return;
                }
                Err(DaemonError::Injoignable { .. }) => {
                    std::thread::sleep(std::time::Duration::from_millis(10))
                }
                autre => panic!("attendu Etranger, obtenu {autre:?}"),
            }
        }
        panic!("le démon de base n'a jamais répondu");
    }

    #[test]
    fn la_description_porte_la_sonde() {
        let s = DaemonEmbedder::serveur("127.0.0.1:7878", "/bin/true");
        assert_eq!(s.adresse(), "127.0.0.1:7878");
        assert_eq!(s.etat(), crate::serveur::Etat::Absent);
    }

    /// Plusieurs clients à la fois : c'est le cas d'usage entier.
    #[test]
    fn plusieurs_clients_en_meme_temps() {
        let adresse = demon(EmbedDaemon::new(Arc::new(Regle(4))).fils(4));
        let fils: Vec<_> = (0..8)
            .map(|i| {
                let adresse = adresse.clone();
                std::thread::spawn(move || {
                    let c = DaemonEmbedder::joindre(&adresse).expect("joindre");
                    let texte = "x".repeat(i + 1);
                    c.embed(&[texte]).expect("embed")
                })
            })
            .collect();
        for (i, f) in fils.into_iter().enumerate() {
            assert_eq!(f.join().expect("fil"), vec![vec![(i + 1) as f32; 4]]);
        }
    }
}
