//! **Démarrer un serveur, et savoir s'il est déjà là.**
//!
//! Le moteur savait charger un modèle, exécuter un graphe, tracer un run ; il
//! ne savait *rien lancer*. Trois besoins l'ont montré la même semaine — un
//! démon d'embedding (une passe E2E rechargeait BGE-M3 sept fois), un terminal
//! qui tient des agents vivants entre deux commandes, et la boucle étrange,
//! dont le critère de réussite est « un backend debout qu'on ouvre ».
//! Voir `docs/issues/29-08-2026/03-un-demon-et-la-fin-du-tout-synchrone.md`.
//!
//! # On ne demande pas si un processus vit, on demande si le service répond
//!
//! C'est la même exigence que `Cwd` : *ça ne ment pas*. Un pid peut avoir été
//! recyclé, un fichier de pid peut avoir survécu à son processus, et un port
//! peut être tenu par quelqu'un d'autre. La seule question dont la réponse est
//! vraie est celle qu'on pose au service lui-même — et il doit répondre **en
//! tant que lui**, d'où [`Sonde::Http`], qui cherche une identité dans la
//! réponse.
//!
//! D'où trois états et non deux ([`Etat`]) : le troisième, `Occupe`, est le
//! cas où quelqu'un répond et ce n'est pas lui. On ne le tue pas, on ne le
//! prend pas pour nous : on le dit.
//!
//! # Pourquoi la sonde n'utilise pas `ureq`
//!
//! Ce module doit exister **sans** la feature `daemon` : la boucle étrange
//! aura à lancer un backend qu'un agent vient d'écrire, dans une compilation
//! qui n'a aucune raison d'embarquer un client HTTP. Une requête `GET` sur une
//! socket locale tient en vingt lignes de `std` ; on cherche un marqueur dans
//! les octets bruts, ce qui reste vrai quel que soit le découpage de la
//! réponse.
//!
//! # Ce que ce module ne fait pas
//!
//! Il ne rend rien asynchrone. Lancer un processus et le sonder, c'est de
//! l'attente bornée et rare — le synchrone y est le bon outil. L'async sert
//! ce qui attend *souvent* : N appels cloud, des agents qui vivent. Les deux
//! questions sont indépendantes, et celle-ci n'attend pas l'autre.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Comment on demande au service s'il est là.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sonde {
    /// Quelqu'un écoute sur l'adresse. **Faible** : ça ne distingue pas notre
    /// serveur d'un autre qui aurait pris le port. À réserver au cas où le
    /// service ne parle pas HTTP.
    Ouvert,
    /// `GET <chemin>`, et la réponse doit contenir `contient` — le nom du
    /// service, sa version, le modèle qu'il sert : ce qui le distingue.
    Http {
        /// Chemin interrogé, avec sa barre de tête (`/sante`).
        chemin: String,
        /// Marqueur cherché dans les octets de la réponse.
        contient: String,
    },
}

/// Ce que la sonde a trouvé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Etat {
    /// Il est là, et c'est bien lui.
    Repond,
    /// Personne n'écoute.
    Absent,
    /// Quelqu'un écoute, mais ne se reconnaît pas. Porte la première ligne de
    /// la réponse, pour qu'un humain sache à qui il a affaire.
    Occupe(String),
}

/// Ce qu'on fait du processus quand l'attache tombe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fin {
    /// **Le défaut.** Un démon sert précisément à survivre à celui qui l'a
    /// lancé : le processus de test suivant doit le retrouver debout.
    Laisser,
    /// Le tuer — pour un serveur éphémère qui n'a de sens que le temps d'un
    /// test ou d'une démonstration.
    Arreter,
}

/// Ce qui a empêché d'assurer le service.
#[derive(Debug)]
pub enum ServeurError {
    /// L'adresse ne se résout pas.
    Adresse(String),
    /// Le port est tenu par quelqu'un qui n'est pas ce service. **On ne le tue
    /// pas** : c'est peut-être le serveur de quelqu'un d'autre.
    Occupe { nom: String, adresse: String, apercu: String },
    /// Le lancement lui-même a échoué (binaire absent, droits…).
    Lancement { nom: String, cause: String },
    /// Le processus est mort avant de répondre. Porte le journal, parce que
    /// c'est là qu'est la raison.
    Mort { nom: String, code: Option<i32>, journal: PathBuf },
    /// Il vit toujours mais n'a pas répondu dans le délai.
    Muet { nom: String, delai: Duration, journal: PathBuf },
}

impl std::fmt::Display for ServeurError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adresse(a) => write!(f, "adresse illisible : {a}"),
            Self::Occupe { nom, adresse, apercu } => write!(
                f,
                "{adresse} est déjà pris, mais ce n'est pas « {nom} » : {apercu} — \
                 rien n'a été tué, choisissez un autre port ou arrêtez ce service à la main"
            ),
            Self::Lancement { nom, cause } => write!(f, "« {nom} » n'a pas pu être lancé : {cause}"),
            Self::Mort { nom, code, journal } => write!(
                f,
                "« {nom} » est mort avant de répondre (code {}) — le journal est dans {}",
                code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
                journal.display()
            ),
            Self::Muet { nom, delai, journal } => write!(
                f,
                "« {nom} » vit mais n'a pas répondu en {:.1} s — le journal est dans {}",
                delai.as_secs_f32(),
                journal.display()
            ),
        }
    }
}

impl std::error::Error for ServeurError {}

/// **La description d'un serveur** : comment le joindre, comment le
/// reconnaître, comment le lancer s'il n'est pas là.
#[derive(Debug, Clone)]
pub struct Serveur {
    nom: String,
    adresse: String,
    sonde: Sonde,
    programme: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    dossier_journal: PathBuf,
    attente: Duration,
    fin: Fin,
}

impl Serveur {
    /// Un serveur nommé, joignable à `adresse` (`hôte:port`), lancé par
    /// `programme`. Par défaut : sonde faible, dix secondes d'attente, journal
    /// dans le dossier temporaire, et on le laisse vivre.
    pub fn new(nom: impl Into<String>, adresse: impl Into<String>, programme: impl Into<String>) -> Self {
        Self {
            nom: nom.into(),
            adresse: adresse.into(),
            sonde: Sonde::Ouvert,
            programme: programme.into(),
            args: Vec::new(),
            env: Vec::new(),
            dossier_journal: std::env::temp_dir(),
            attente: Duration::from_secs(10),
            fin: Fin::Laisser,
        }
    }

    /// Un argument de la ligne de commande.
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    /// Plusieurs arguments d'un coup.
    pub fn args<I, S>(mut self, it: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(it.into_iter().map(Into::into));
        self
    }

    /// Une variable d'environnement pour le processus lancé.
    pub fn env(mut self, cle: impl Into<String>, valeur: impl Into<String>) -> Self {
        self.env.push((cle.into(), valeur.into()));
        self
    }

    /// Comment reconnaître le service. Voir [`Sonde`].
    pub fn sonde(mut self, s: Sonde) -> Self {
        self.sonde = s;
        self
    }

    /// Raccourci pour la sonde forte : `GET chemin`, et la réponse doit
    /// contenir `contient`.
    pub fn sante(self, chemin: impl Into<String>, contient: impl Into<String>) -> Self {
        self.sonde(Sonde::Http { chemin: chemin.into(), contient: contient.into() })
    }

    /// Délai laissé au serveur pour répondre après son lancement.
    pub fn attente(mut self, d: Duration) -> Self {
        self.attente = d;
        self
    }

    /// Où écrire la sortie du processus.
    ///
    /// **Un fichier, jamais un tube.** Un tube qu'on ne lit pas se remplit, et
    /// le serveur se fige à son premier gros message — un blocage qui
    /// ressemble à un serveur lent, ce qui est la pire forme de panne.
    pub fn journal_dans(mut self, d: impl Into<PathBuf>) -> Self {
        self.dossier_journal = d.into();
        self
    }

    /// Ce qu'on fait du processus quand l'attache tombe. Voir [`Fin`].
    pub fn fin(mut self, f: Fin) -> Self {
        self.fin = f;
        self
    }

    /// Le nom sous lequel on en parle.
    pub fn nom(&self) -> &str {
        &self.nom
    }

    /// L'adresse à laquelle on le joint.
    pub fn adresse(&self) -> &str {
        &self.adresse
    }

    /// Le fichier où part sa sortie.
    pub fn journal(&self) -> PathBuf {
        self.dossier_journal.join(format!("{}.log", self.nom))
    }

    /// **Répond-il ?** Une seule question, posée au service, dont la réponse
    /// est vraie maintenant.
    pub fn etat(&self) -> Etat {
        let Some(addr) = resoudre(&self.adresse) else {
            return Etat::Absent;
        };
        sonder(addr, &self.sonde)
    }

    /// **S'il répond, on s'y attache ; sinon on le lance et on attend.**
    ///
    /// C'est l'unique verbe : l'appelant n'a pas à savoir qui, du démon
    /// précédent ou de lui-même, a payé le chargement.
    pub fn assurer(&self) -> Result<Attache, ServeurError> {
        let addr = resoudre(&self.adresse)
            .ok_or_else(|| ServeurError::Adresse(self.adresse.clone()))?;

        match sonder(addr, &self.sonde) {
            Etat::Repond => {
                return Ok(Attache {
                    adresse: self.adresse.clone(),
                    nom: self.nom.clone(),
                    enfant: None,
                    journal: self.journal(),
                    demarre_par_nous: false,
                    fin: self.fin,
                })
            }
            Etat::Occupe(apercu) => {
                return Err(ServeurError::Occupe {
                    nom: self.nom.clone(),
                    adresse: self.adresse.clone(),
                    apercu,
                })
            }
            Etat::Absent => {}
        }

        let journal = self.journal();
        let mut enfant = self.lancer(&journal)?;

        let debut = Instant::now();
        loop {
            if let Ok(Some(statut)) = enfant.try_wait() {
                return Err(ServeurError::Mort {
                    nom: self.nom.clone(),
                    code: statut.code(),
                    journal,
                });
            }
            if sonder(addr, &self.sonde) == Etat::Repond {
                return Ok(Attache {
                    adresse: self.adresse.clone(),
                    nom: self.nom.clone(),
                    enfant: Some(enfant),
                    journal,
                    demarre_par_nous: true,
                    fin: self.fin,
                });
            }
            if debut.elapsed() >= self.attente {
                let _ = enfant.kill();
                let _ = enfant.wait();
                return Err(ServeurError::Muet {
                    nom: self.nom.clone(),
                    delai: self.attente,
                    journal,
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn lancer(&self, journal: &Path) -> Result<Child, ServeurError> {
        if let Some(parent) = journal.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // En ajout : la raison d'une mort précédente doit survivre au
        // relancement qui la suit.
        let ouvrir = || {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(journal)
                .map_err(|e| ServeurError::Lancement { nom: self.nom.clone(), cause: e.to_string() })
        };
        let sortie = ouvrir()?;
        let erreurs = ouvrir()?;

        let mut cmd = Command::new(&self.programme);
        cmd.args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(sortie))
            .stderr(Stdio::from(erreurs));
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.spawn()
            .map_err(|e| ServeurError::Lancement { nom: self.nom.clone(), cause: e.to_string() })
    }
}

/// **Un serveur qui répond**, et le peu qu'on sait de lui.
#[derive(Debug)]
pub struct Attache {
    nom: String,
    adresse: String,
    enfant: Option<Child>,
    journal: PathBuf,
    demarre_par_nous: bool,
    fin: Fin,
}

impl Attache {
    /// Le nom du service.
    pub fn nom(&self) -> &str {
        &self.nom
    }

    /// Où le joindre.
    pub fn adresse(&self) -> &str {
        &self.adresse
    }

    /// Son journal.
    pub fn journal(&self) -> &Path {
        &self.journal
    }

    /// **Est-ce nous qui l'avons lancé ?** Ce que ça coûte n'est pas le même
    /// des deux côtés : celui qui lance paie le chargement, les suivants non.
    pub fn demarre_par_nous(&self) -> bool {
        self.demarre_par_nous
    }

    /// L'arrêter maintenant, quelle que soit la politique de fin. Sans effet
    /// si ce n'est pas nous qui l'avons lancé — on n'arrête pas le serveur
    /// d'un autre.
    pub fn arreter(&mut self) {
        if let Some(mut e) = self.enfant.take() {
            let _ = e.kill();
            let _ = e.wait();
        }
    }
}

impl Drop for Attache {
    fn drop(&mut self) {
        if self.fin == Fin::Arreter {
            self.arreter();
        }
    }
}

fn resoudre(adresse: &str) -> Option<SocketAddr> {
    adresse.to_socket_addrs().ok()?.next()
}

/// La sonde, en `std` pur. Voir l'en-tête du module pour le pourquoi.
fn sonder(addr: SocketAddr, sonde: &Sonde) -> Etat {
    let Ok(mut flux) = TcpStream::connect_timeout(&addr, Duration::from_millis(300)) else {
        return Etat::Absent;
    };
    let Sonde::Http { chemin, contient } = sonde else {
        return Etat::Repond;
    };

    let _ = flux.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = flux.set_write_timeout(Some(Duration::from_millis(500)));
    let requete = format!(
        "GET {chemin} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nUser-Agent: rag3weaver\r\n\r\n"
    );
    if flux.write_all(requete.as_bytes()).is_err() {
        return Etat::Absent;
    }

    // Borné : on cherche un marqueur, pas une page. Un serveur qui parle un
    // autre protocole ne doit pas pouvoir nous faire lire indéfiniment.
    let mut reponse = Vec::new();
    let mut tampon = [0u8; 4096];
    loop {
        match flux.read(&mut tampon) {
            Ok(0) => break,
            Ok(n) => {
                reponse.extend_from_slice(&tampon[..n]);
                if reponse.len() >= 64 * 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let texte = String::from_utf8_lossy(&reponse);
    if texte.contains(contient.as_str()) {
        Etat::Repond
    } else {
        Etat::Occupe(premiere_ligne(&texte))
    }
}

fn premiere_ligne(texte: &str) -> String {
    let ligne = texte.lines().next().unwrap_or("").trim();
    if ligne.is_empty() {
        "aucune réponse lisible".to_string()
    } else if ligne.len() > 120 {
        format!("{}…", &ligne[..120])
    } else {
        ligne.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Un serveur d'une ligne, dans un fil : il répond `corps` à tout.
    fn faux_serveur(corps: &'static str) -> (String, Arc<AtomicBool>) {
        let ecoute = TcpListener::bind("127.0.0.1:0").expect("bind");
        let adresse = ecoute.local_addr().unwrap().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let temoin = stop.clone();
        std::thread::spawn(move || {
            for flux in ecoute.incoming() {
                if temoin.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(mut flux) = flux else { break };
                let mut tampon = [0u8; 1024];
                let _ = flux.set_read_timeout(Some(Duration::from_millis(200)));
                let _ = flux.read(&mut tampon);
                let _ = flux.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corps}",
                        corps.len()
                    )
                    .as_bytes(),
                );
            }
        });
        (adresse, stop)
    }

    fn port_libre() -> String {
        let e = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = e.local_addr().unwrap().to_string();
        drop(e);
        a
    }

    #[test]
    fn personne_n_ecoute_donc_absent() {
        let s = Serveur::new("fantome", port_libre(), "/bin/true").sante("/sante", "fantome");
        assert_eq!(s.etat(), Etat::Absent);
    }

    #[test]
    fn il_repond_et_se_nomme() {
        let (adresse, _stop) = faux_serveur(r#"{"service":"embeddings","modele":"bge-m3"}"#);
        let s = Serveur::new("embeddings", adresse, "/bin/true").sante("/sante", "\"service\":\"embeddings\"");
        assert_eq!(s.etat(), Etat::Repond);
    }

    /// **Le troisième état.** Quelqu'un répond, ce n'est pas lui : ni `Repond`
    /// (on lui enverrait des requêtes qu'il ne comprend pas), ni `Absent` (on
    /// lancerait un serveur sur un port pris).
    #[test]
    fn quelqu_un_d_autre_tient_le_port() {
        let (adresse, _stop) = faux_serveur("Bienvenue sur le serveur de quelqu'un d'autre");
        let s = Serveur::new("embeddings", adresse, "/bin/true").sante("/sante", "\"service\":\"embeddings\"");
        match s.etat() {
            Etat::Occupe(apercu) => assert!(apercu.starts_with("HTTP/1.1 200"), "aperçu : {apercu}"),
            autre => panic!("attendu Occupe, obtenu {autre:?}"),
        }
    }

    /// La sonde faible ne distingue rien — et c'est pour ça qu'elle n'est pas
    /// le défaut d'un service qui parle HTTP.
    #[test]
    fn la_sonde_faible_ne_voit_que_l_ecoute() {
        let (adresse, _stop) = faux_serveur("peu importe");
        let s = Serveur::new("x", adresse, "/bin/true").sonde(Sonde::Ouvert);
        assert_eq!(s.etat(), Etat::Repond);
    }

    #[test]
    fn assurer_ne_relance_pas_ce_qui_repond_deja() {
        let (adresse, _stop) = faux_serveur(r#"{"service":"embeddings"}"#);
        // Le programme est volontairement inlançable : s'il était lancé, on
        // aurait une erreur au lieu d'une attache.
        let s = Serveur::new("embeddings", adresse, "/programme/qui/n/existe/pas")
            .sante("/sante", "embeddings");
        let a = s.assurer().expect("attache");
        assert!(!a.demarre_par_nous(), "il répondait déjà, rien à lancer");
    }

    #[test]
    fn assurer_refuse_un_port_tenu_par_un_autre() {
        let (adresse, _stop) = faux_serveur("pas nous");
        let s = Serveur::new("embeddings", adresse.clone(), "/bin/true").sante("/sante", "embeddings");
        match s.assurer() {
            Err(ServeurError::Occupe { adresse: a, .. }) => assert_eq!(a, adresse),
            autre => panic!("attendu Occupe, obtenu {autre:?}"),
        }
    }

    #[test]
    fn un_programme_absent_se_dit_au_lancement() {
        let s = Serveur::new("x", port_libre(), "/programme/qui/n/existe/pas");
        match s.assurer() {
            Err(ServeurError::Lancement { nom, .. }) => assert_eq!(nom, "x"),
            autre => panic!("attendu Lancement, obtenu {autre:?}"),
        }
    }

    /// Le cas le plus fréquent en vrai : le serveur démarre, meurt, et la
    /// raison est dans le journal — donc l'erreur doit le nommer.
    #[test]
    fn un_serveur_qui_meurt_renvoie_a_son_journal() {
        let dossier = std::env::temp_dir().join("rag3weaver-tests-serveur");
        let s = Serveur::new("mourant", port_libre(), "/bin/false")
            .journal_dans(&dossier)
            .attente(Duration::from_secs(2));
        match s.assurer() {
            Err(ServeurError::Mort { journal, .. }) => {
                assert_eq!(journal, dossier.join("mourant.log"));
            }
            autre => panic!("attendu Mort, obtenu {autre:?}"),
        }
    }

    #[test]
    fn un_serveur_qui_vit_sans_repondre_est_muet_pas_mort() {
        let dossier = std::env::temp_dir().join("rag3weaver-tests-serveur");
        let s = Serveur::new("muet", port_libre(), "/bin/sleep")
            .arg("30")
            .journal_dans(&dossier)
            .attente(Duration::from_millis(400));
        match s.assurer() {
            Err(ServeurError::Muet { nom, .. }) => assert_eq!(nom, "muet"),
            autre => panic!("attendu Muet, obtenu {autre:?}"),
        }
    }

    /// Un démon doit survivre à celui qui l'a lancé : c'est toute sa raison
    /// d'être. `Fin::Arreter` est le cas particulier, jamais le défaut.
    #[test]
    fn le_defaut_laisse_vivre() {
        let s = Serveur::new("x", "127.0.0.1:1", "/bin/true");
        assert_eq!(s.fin, Fin::Laisser);
    }

    #[test]
    fn le_journal_porte_le_nom_du_service() {
        let s = Serveur::new("embeddings", "127.0.0.1:1", "/bin/true").journal_dans("/tmp/xyz");
        assert_eq!(s.journal(), PathBuf::from("/tmp/xyz/embeddings.log"));
    }

    #[test]
    fn une_adresse_illisible_se_dit() {
        let s = Serveur::new("x", "pas une adresse", "/bin/true");
        assert_eq!(s.etat(), Etat::Absent);
        assert!(matches!(s.assurer(), Err(ServeurError::Adresse(_))));
    }
}
