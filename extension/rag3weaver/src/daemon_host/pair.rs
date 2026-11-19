//! **Le mode pair** (lot 1, page `docs/rag3weaver-archi/19-11-2026/05-le-mode-pair-en-lots.md`) :
//! une tâche dont le backend déclare `teammate.pair` est vue par les sessions Claude Code comme
//! l'une d'elles. L'hôte tient son entrée dans le registre des sessions (`<pid>.json`), la met à
//! jour au fil des pas, et la retire à la fin de la tâche, à l'arrêt de l'hôte, et au démarrage
//! suivant si l'hôte est mort sans la retirer.
//!
//! Le registre réel est `~/.claude/sessions` : il ne se touche qu'au mot de Lucie (lot 6). Les
//! témoins jouent sur `RAG3WEAVER_PAIR_SESSIONS_DIR` et `RAG3WEAVER_PAIR_SOCKS_DIR`.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Ce qu'une entrée dit d'elle-même : `nameSource`. Seules les entrées qui le portent sont les
/// nôtres ; le nettoyage ne touche jamais les autres.
pub const NAME_SOURCE: &str = "rag3weaver";

/// Le registre des sessions et le dossier de leurs sockets.
#[derive(Debug, Clone)]
pub struct Registre {
    pub sessions: PathBuf,
    pub socks: PathBuf,
}

impl Registre {
    /// `RAG3WEAVER_PAIR_SESSIONS_DIR`, sinon `~/.claude/sessions` ; `RAG3WEAVER_PAIR_SOCKS_DIR`,
    /// sinon `$XDG_RUNTIME_DIR/cc-socks`.
    pub fn depuis(lire: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let non_vide = |k: &str| lire(k).filter(|v| !v.is_empty());
        let sessions = match non_vide("RAG3WEAVER_PAIR_SESSIONS_DIR") {
            Some(d) => PathBuf::from(d),
            None => PathBuf::from(non_vide("HOME")?).join(".claude/sessions"),
        };
        let socks = match non_vide("RAG3WEAVER_PAIR_SOCKS_DIR") {
            Some(d) => PathBuf::from(d),
            None => PathBuf::from(non_vide("XDG_RUNTIME_DIR")?).join("cc-socks"),
        };
        Some(Registre { sessions, socks })
    }

    fn fichier(&self, pid: u32) -> PathBuf {
        self.sessions.join(format!("{pid}.json"))
    }

    /// Écrit l'entrée d'un coup : un fichier voisin, puis `rename`. Un lecteur ne voit jamais une
    /// entrée à moitié écrite.
    fn ecrire(&self, entree: &Value) -> Result<(), String> {
        let pid = entree["pid"].as_u64().unwrap_or_default() as u32;
        std::fs::create_dir_all(&self.sessions).map_err(|e| format!("{} : {e}", self.sessions.display()))?;
        let tmp = self.sessions.join(format!(".{pid}.json.rag3weaver-tmp"));
        std::fs::write(&tmp, entree.to_string()).map_err(|e| format!("{} : {e}", tmp.display()))?;
        std::fs::rename(&tmp, self.fichier(pid)).map_err(|e| format!("{} : {e}", self.fichier(pid).display()))
    }

    /// Retire les entrées laissées par un hôte mort (les nôtres seulement) : pid absent, ou pid
    /// repris par un autre processus (`procStart` différent). Rend les fichiers retirés.
    pub fn nettoyer(&self) -> Vec<PathBuf> {
        let Ok(lus) = std::fs::read_dir(&self.sessions) else { return Vec::new() };
        let mut retires = Vec::new();
        for d in lus.flatten() {
            let p = d.path();
            if p.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(e) = std::fs::read(&p).ok().and_then(|o| serde_json::from_slice::<Value>(&o).ok()) else { continue };
            if e["nameSource"] != NAME_SOURCE {
                continue;
            }
            let Some(pid) = e["pid"].as_u64() else { continue };
            let vivant = proc_start(pid as u32).is_some_and(|s| Some(s.as_str()) == e["procStart"].as_str());
            if !vivant && std::fs::remove_file(&p).is_ok() {
                retires.push(p);
            }
        }
        retires
    }
}

/// Le champ 22 de `/proc/<pid>/stat` (l'heure de départ du processus) : un pid repris par un autre
/// processus ne le partage pas.
fn proc_start(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let apres = stat.get(stat.rfind(')')? + 2..)?;
    apres.split(' ').nth(19).map(str::to_string)
}

/// `linux:<machine-id>:pid:[<inode>]`, comme les entrées de Claude Code.
fn pid_domain() -> Option<String> {
    let machine = std::fs::read_to_string("/etc/machine-id").ok()?;
    let espace = std::fs::read_link("/proc/self/ns/pid").ok()?;
    Some(format!("linux:{}:{}", machine.trim(), espace.to_string_lossy()))
}

fn maintenant_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// Un identifiant de session au format UUID v4, tiré de `/dev/urandom`.
fn session_id() -> String {
    let mut o = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut o);
    }
    o[6] = (o[6] & 0x0f) | 0x40;
    o[8] = (o[8] & 0x3f) | 0x80;
    let h: String = o.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// La taille au-delà de laquelle une trame est refusée.
pub const TRAME_MAX: usize = 64 * 1024;

/// Ce qu'une connexion à la socket apporte (relevés du 16 et du 18 novembre 2026).
#[derive(Debug, Clone, PartialEq)]
pub enum Trame {
    /// `type: "user"` : le texte, enveloppe `<cross-session-message …>` comprise, faite par
    /// l'émetteur ; `from` est son adresse de réponse.
    Message { texte: String, from: String, from_name: String, msg_id: String, priority: String },
    /// `type: "control"`, `action: "notify_when_idle"` : l'émetteur veut un avis à la pause.
    Abonnement { from: String, msg_id: String },
}

/// Une connexion lue : `None` pour une sonde (rien d'écrit), la trame sinon, ou la raison du
/// refus (illisible, trop longue, d'une version ou d'un genre inconnus).
pub fn lire_trame(octets: &[u8]) -> Result<Option<Trame>, String> {
    if octets.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    if octets.len() > TRAME_MAX {
        return Err(format!("frame of {} bytes, over 64 KiB", octets.len()));
    }
    let v: Value = serde_json::from_slice(octets.trim_ascii()).map_err(|e| format!("unreadable frame: {e}"))?;
    if v["msgV"] != 1 {
        return Err(format!("unknown msgV: {}", v["msgV"]));
    }
    let texte = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    let from = v["from"].as_str().filter(|f| !f.is_empty()).ok_or("a frame without from")?.to_string();
    match (v["type"].as_str(), v["action"].as_str()) {
        (Some("user"), _) => {
            let contenu = v["message"]["content"].as_str().filter(|c| !c.trim().is_empty()).ok_or("an empty message")?;
            Ok(Some(Trame::Message {
                texte: contenu.to_string(),
                from,
                from_name: attribut(contenu, "from-name").unwrap_or_default(),
                msg_id: texte("msg_id"),
                priority: v["priority"].as_str().unwrap_or("next").to_string(),
            }))
        }
        (Some("control"), Some("notify_when_idle")) => Ok(Some(Trame::Abonnement { from, msg_id: texte("msg_id") })),
        (t, a) => Err(format!("unknown frame: type {t:?}, action {a:?}")),
    }
}

/// `nom="valeur"` dans l'enveloppe `<cross-session-message …>`.
fn attribut(enveloppe: &str, nom: &str) -> Option<String> {
    let tete = enveloppe.split('>').next()?;
    let debut = tete.find(&format!("{nom}=\""))? + nom.len() + 2;
    let fin = tete[debut..].find('"')?;
    Some(tete[debut..debut + fin].to_string())
}

/// La socket d'écoute et son fil.
struct Ecoute {
    chemin: PathBuf,
    arret: std::sync::Arc<std::sync::atomic::AtomicBool>,
    fil: Option<std::thread::JoinHandle<()>>,
}

impl Ecoute {
    fn arreter(mut self) {
        self.arret.store(true, std::sync::atomic::Ordering::SeqCst);
        // Réveille l'accept qui attend.
        let _ = std::os::unix::net::UnixStream::connect(&self.chemin);
        if let Some(f) = self.fil.take() {
            let _ = f.join();
        }
        let _ = std::fs::remove_file(&self.chemin);
    }
}

/// Ce que l'hôte fait d'une trame reçue pour la tâche `cle`.
pub type Livreur = std::sync::Arc<dyn Fn(&str, Trame) + Send + Sync>;

/// L'entrée de l'hôte : une tâche en mode pair au plus.
pub struct Pair {
    registre: Registre,
    tenue: std::sync::Arc<Mutex<Option<(String, Value)>>>,
    livreur: Option<Livreur>,
    ecoute: Mutex<Option<Ecoute>>,
    abonnements: std::sync::Arc<Mutex<Vec<(String, String)>>>,
}

impl Pair {
    pub fn new(registre: Registre) -> Self {
        Pair { registre, tenue: Default::default(), livreur: None, ecoute: Mutex::new(None), abonnements: Default::default() }
    }

    /// Ce que l'hôte fait des trames reçues par la socket.
    pub fn avec_livreur(mut self, livreur: Livreur) -> Self {
        self.livreur = Some(livreur);
        self
    }

    /// Les abonnements aux avis de pause reçus, à servir au lot 5.
    pub fn abonnements(&self) -> Vec<(String, String)> {
        self.abonnements.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// La socket, en 600, et le fil qui lit une trame par connexion : une sonde ne laisse rien, un
    /// message va au livreur pour la tâche tenue, un abonnement est gardé, une trame refusée est
    /// journalisée.
    fn ecouter(&self) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        let mut ecoute = self.ecoute.lock().unwrap_or_else(|p| p.into_inner());
        if ecoute.is_some() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.registre.socks).map_err(|e| format!("{} : {e}", self.registre.socks.display()))?;
        let chemin = self.registre.socks.join(format!("{}.sock", std::process::id()));
        // Une socket à notre pid ne peut être que la nôtre, laissée par un hôte mort.
        let _ = std::fs::remove_file(&chemin);
        let l = std::os::unix::net::UnixListener::bind(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        std::fs::set_permissions(&chemin, std::fs::Permissions::from_mode(0o600)).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let arret = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tenue, livreur, abonnements, a) = (self.tenue.clone(), self.livreur.clone(), self.abonnements.clone(), arret.clone());
        let fil = std::thread::Builder::new()
            .name("pair-socket".into())
            .spawn(move || {
                for c in l.incoming() {
                    if a.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }
                    let Ok(c) = c else { continue };
                    let _ = c.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                    let mut octets = Vec::new();
                    use std::io::Read;
                    let _ = c.take(TRAME_MAX as u64 + 1).read_to_end(&mut octets);
                    match lire_trame(&octets) {
                        Ok(None) => {}
                        Ok(Some(Trame::Abonnement { from, msg_id })) => abonnements.lock().unwrap_or_else(|p| p.into_inner()).push((from, msg_id)),
                        Ok(Some(m)) => {
                            let cle = tenue.lock().unwrap_or_else(|p| p.into_inner()).as_ref().map(|(c, _)| c.clone());
                            match (cle, &livreur) {
                                (Some(cle), Some(livrer)) => livrer(&cle, m),
                                _ => eprintln!("[pair] a message arrived with no task to give it to"),
                            }
                        }
                        Err(e) => eprintln!("[pair] frame refused: {e}"),
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        *ecoute = Some(Ecoute { chemin, arret, fil: Some(fil) });
        Ok(())
    }

    /// **Écrire à une session** (lot 3) au nom de l'agent de la tâche `cle` : `to` est un nom du
    /// registre (lu, jamais écrit) ou l'adresse `uds:` d'une ligne reçue. La trame a la forme
    /// relevée le 16 novembre 2026, enveloppe faite ici ; une connexion, sans attendre de réponse.
    /// Rend le `msg_id` de la trame.
    pub fn envoyer(&self, cle: &str, to: &str, texte: &str) -> Result<String, String> {
        let _ = (cle, to, texte);
        Err("not yet".into())
    }

    fn cesser_d_ecouter(&self) {
        if let Some(e) = self.ecoute.lock().unwrap_or_else(|p| p.into_inner()).take() {
            e.arreter();
        }
    }

    /// Pose l'entrée de la tâche `cle` (`parent/task`), nommée `name`, dans `cwd`. Une seconde
    /// tâche est refusée en le disant.
    pub fn ouvrir(&self, cle: &str, name: &str, cwd: &Path) -> Result<(), String> {
        let mut tenue = self.tenue.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((autre, _)) = tenue.as_ref().filter(|(autre, _)| autre != cle) {
            return Err(format!("teammate.pair: this host already holds {autre} in the sessions registry, and holds one task at a time"));
        }
        if tenue.is_some() {
            // The same task again: its entry stays (same sessionId), busy.
            drop(tenue);
            self.statut(cle, true);
            return Ok(());
        }
        let pid = std::process::id();
        let t = maintenant_ms();
        let entree = json!({
            "pid": pid,
            "sessionId": session_id(),
            "cwd": cwd.to_string_lossy(),
            "startedAt": t,
            "procStart": proc_start(pid).unwrap_or_default(),
            "version": env!("CARGO_PKG_VERSION"),
            "peerProtocol": 1,
            "peerFeatures": [],
            "kind": "interactive",
            "entrypoint": "rag3weaver",
            "pidDomain": pid_domain().unwrap_or_default(),
            "messagingSocketPath": self.registre.socks.join(format!("{pid}.sock")).to_string_lossy(),
            "name": name,
            "nameSource": NAME_SOURCE,
            "nameSince": t,
            "updatedAt": t,
            "status": "busy",
            "statusUpdatedAt": t,
        });
        self.registre.ecrire(&entree)?;
        *tenue = Some((cle.to_string(), entree));
        drop(tenue);
        if let Err(e) = self.ecouter() {
            self.fermer(cle);
            return Err(format!("teammate.pair: the socket does not listen: {e}"));
        }
        Ok(())
    }

    /// `busy` pendant un pas, `idle` sinon.
    pub fn statut(&self, cle: &str, occupe: bool) {
        let mut tenue = self.tenue.lock().unwrap_or_else(|p| p.into_inner());
        let Some((tenante, entree)) = tenue.as_mut() else { return };
        if tenante != cle {
            return;
        }
        let t = maintenant_ms();
        entree["status"] = json!(if occupe { "busy" } else { "idle" });
        entree["statusUpdatedAt"] = json!(t);
        entree["updatedAt"] = json!(t);
        if let Err(e) = self.registre.ecrire(entree) {
            eprintln!("[pair] the entry of {cle} could not be updated: {e}");
        }
    }

    /// Retire l'entrée de la tâche `cle`, si c'est elle qui la tient.
    pub fn fermer(&self, cle: &str) {
        let mut tenue = self.tenue.lock().unwrap_or_else(|p| p.into_inner());
        if tenue.as_ref().is_some_and(|(tenante, _)| tenante == cle) {
            let _ = std::fs::remove_file(self.registre.fichier(std::process::id()));
            *tenue = None;
            drop(tenue);
            self.cesser_d_ecouter();
        }
    }
}

impl Drop for Pair {
    /// L'hôte qui s'arrête retire son entrée.
    fn drop(&mut self) {
        if self.tenue.lock().map(|t| t.is_some()).unwrap_or(true) {
            let _ = std::fs::remove_file(self.registre.fichier(std::process::id()));
        }
        self.cesser_d_ecouter();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registre(t: &tempfile::TempDir) -> Registre {
        Registre { sessions: t.path().join("sessions"), socks: t.path().join("socks") }
    }

    fn entree(r: &Registre) -> Value {
        let p = r.sessions.join(format!("{}.json", std::process::id()));
        serde_json::from_slice(&std::fs::read(&p).unwrap_or_default()).unwrap_or(Value::Null)
    }

    fn proc_start(pid: u32) -> String {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let apres = &stat[stat.rfind(')').unwrap() + 2..];
        apres.split(' ').nth(19).unwrap().to_string()
    }

    fn message(content: &str) -> Value {
        json!({ "msgV": 1, "msg_id": "m-1", "type": "user", "priority": "next",
                "from": "uds:/run/user/1000/cc-socks/42.sock",
                "message": { "role": "user", "content": content } })
    }

    const ENVELOPPE: &str = "<cross-session-message from=\"uds:/run/user/1000/cc-socks/42.sock\" from-name=\"rag3db-5a\" from-mode=\"prompting\">\nHello\n</cross-session-message>";

    #[test]
    fn a_frame_is_read_as_a_message_a_subscription_or_refused() {
        let m = lire_trame(format!("{}\n", message(ENVELOPPE)).as_bytes()).unwrap();
        assert_eq!(
            m,
            Some(Trame::Message {
                texte: ENVELOPPE.into(),
                from: "uds:/run/user/1000/cc-socks/42.sock".into(),
                from_name: "rag3db-5a".into(),
                msg_id: "m-1".into(),
                priority: "next".into()
            })
        );
        let c = json!({ "type": "control", "action": "notify_when_idle", "from": "uds:/x.sock", "from_mode": "prompting", "msgV": 1, "msg_id": "c-1" });
        assert_eq!(lire_trame(c.to_string().as_bytes()).unwrap(), Some(Trame::Abonnement { from: "uds:/x.sock".into(), msg_id: "c-1".into() }));
        assert_eq!(lire_trame(b"").unwrap(), None, "a probe writes nothing");
        let long = message(&"x".repeat(TRAME_MAX));
        assert!(lire_trame(long.to_string().as_bytes()).unwrap_err().contains("64"), "too long");
        let mut v2 = message("hi");
        v2["msgV"] = json!(2);
        assert!(lire_trame(v2.to_string().as_bytes()).unwrap_err().contains("msgV"));
        assert!(lire_trame(b"{not json").is_err());
        let autre = json!({ "msgV": 1, "type": "control", "action": "something_else", "from": "uds:/x" });
        assert!(lire_trame(autre.to_string().as_bytes()).is_err());
        let sans_texte = json!({ "msgV": 1, "type": "user", "from": "uds:/x", "message": { "role": "user", "content": "" } });
        assert!(lire_trame(sans_texte.to_string().as_bytes()).is_err(), "an empty message is refused");
    }

    #[test]
    fn the_socket_delivers_messages_keeps_subscriptions_and_ignores_probes() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::net::UnixStream;
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let recues = std::sync::Arc::new(Mutex::new(Vec::<(String, Trame)>::new()));
        let livreur: Livreur = {
            let recues = recues.clone();
            std::sync::Arc::new(move |cle: &str, trame: Trame| recues.lock().unwrap().push((cle.to_string(), trame)))
        };
        let pair = Pair::new(r.clone()).avec_livreur(livreur);
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        let chemin = PathBuf::from(entree(&r)["messagingSocketPath"].as_str().unwrap());
        assert_eq!(std::fs::metadata(&chemin).map(|m| m.permissions().mode() & 0o777).ok(), Some(0o600), "the socket is there, 600");
        drop(UnixStream::connect(&chemin).unwrap()); // a probe
        let mut c = UnixStream::connect(&chemin).unwrap();
        c.write_all(format!("{}\n", message(ENVELOPPE)).as_bytes()).unwrap();
        drop(c);
        let mut c = UnixStream::connect(&chemin).unwrap();
        c.write_all(json!({ "type": "control", "action": "notify_when_idle", "from": "uds:/x.sock", "msgV": 1, "msg_id": "c-1" }).to_string().as_bytes()).unwrap();
        drop(c);
        let mut c = UnixStream::connect(&chemin).unwrap();
        c.write_all(b"{garbage").unwrap();
        drop(c);
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while (recues.lock().unwrap().is_empty() || pair.abonnements().is_empty()) && std::time::Instant::now() < fin {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let recues = recues.lock().unwrap().clone();
        assert_eq!(recues.len(), 1, "one message, no probe, no garbage: {recues:?}");
        assert_eq!(recues[0].0, "p/a");
        assert!(matches!(&recues[0].1, Trame::Message { from_name, .. } if from_name == "rag3db-5a"), "{recues:?}");
        assert_eq!(pair.abonnements(), vec![("uds:/x.sock".to_string(), "c-1".to_string())]);
        pair.fermer("p/a");
        assert!(!chemin.exists(), "the socket goes with the entry");
        assert!(UnixStream::connect(&chemin).is_err());
    }

    /// Une fausse session : son entrée au registre, et sa socket qui garde ce qu'elle reçoit.
    fn fausse_session(r: &Registre, nom: &str) -> (PathBuf, std::sync::Arc<Mutex<Vec<Vec<u8>>>>) {
        std::fs::create_dir_all(&r.sessions).unwrap();
        std::fs::create_dir_all(&r.socks).unwrap();
        let chemin = r.socks.join(format!("{nom}.sock"));
        let l = std::os::unix::net::UnixListener::bind(&chemin).unwrap();
        std::fs::write(r.sessions.join(format!("{nom}.json")), json!({ "pid": 4194390u32, "name": nom, "nameSource": "user", "messagingSocketPath": chemin }).to_string()).unwrap();
        let recues = std::sync::Arc::new(Mutex::new(Vec::new()));
        let r2 = recues.clone();
        std::thread::spawn(move || {
            for c in l.incoming().flatten() {
                use std::io::Read;
                let mut o = Vec::new();
                let _ = (&c).read_to_end(&mut o);
                if !o.is_empty() {
                    r2.lock().unwrap().push(o);
                }
            }
        });
        (chemin, recues)
    }

    fn attendre(recues: &std::sync::Arc<Mutex<Vec<Vec<u8>>>>, n: usize) -> Vec<Value> {
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while recues.lock().unwrap().len() < n && std::time::Instant::now() < fin {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        recues.lock().unwrap().iter().map(|o| serde_json::from_slice(o).unwrap()).collect()
    }

    #[test]
    fn a_message_goes_to_a_session_by_name_or_address_in_the_frame_of_the_bus() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let (chemin, recues) = fausse_session(&r, "rag3db-5a");
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/a", "algareth", Path::new("/w")).unwrap();
        let notre = entree(&r)["messagingSocketPath"].as_str().unwrap().to_string();
        let id = pair.envoyer("p/a", "rag3db-5a", "4").unwrap();
        let f = attendre(&recues, 1);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0]["msgV"], 1);
        assert_eq!(f[0]["type"], "user");
        assert_eq!(f[0]["priority"], "next");
        assert_eq!(f[0]["msg_id"], id.as_str());
        assert_eq!(f[0]["from"], format!("uds:{notre}"));
        assert_eq!(f[0]["message"]["role"], "user");
        assert_eq!(
            f[0]["message"]["content"],
            format!("<cross-session-message from=\"uds:{notre}\" from-name=\"algareth\" from-mode=\"rag3weaver\">\n4\n</cross-session-message>")
        );
        // By the address a received line carries.
        pair.envoyer("p/a", &format!("uds:{}", chemin.display()), "encore").unwrap();
        assert_eq!(attendre(&recues, 2).len(), 2);
        // An unknown name names those present; a task that does not hold the entry cannot send.
        let e = pair.envoyer("p/a", "personne", "x").unwrap_err();
        assert!(e.contains("personne") && e.contains("rag3db-5a"), "{e}");
        assert!(pair.envoyer("p/autre", "rag3db-5a", "x").is_err());
        // A session whose socket does not answer is said.
        std::fs::remove_file(&chemin).unwrap();
        let e = pair.envoyer("p/a", "rag3db-5a", "x").unwrap_err();
        assert!(e.contains("rag3db-5a"), "{e}");
    }

    #[test]
    fn the_registry_is_read_from_the_environment() {
        let r = Registre::depuis(|k| match k {
            "RAG3WEAVER_PAIR_SESSIONS_DIR" => Some("/r/sessions".into()),
            "RAG3WEAVER_PAIR_SOCKS_DIR" => Some("/r/socks".into()),
            _ => None,
        })
        .expect("a registry");
        assert_eq!((r.sessions.as_path(), r.socks.as_path()), (Path::new("/r/sessions"), Path::new("/r/socks")));
        let r = Registre::depuis(|k| match k {
            "HOME" => Some("/h".into()),
            "XDG_RUNTIME_DIR" => Some("/run/user/7".into()),
            _ => None,
        })
        .expect("a registry");
        assert_eq!((r.sessions.as_path(), r.socks.as_path()), (Path::new("/h/.claude/sessions"), Path::new("/run/user/7/cc-socks")));
    }

    #[test]
    fn an_entry_is_posed_with_the_keys_of_the_registry() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/algareth", "algareth", Path::new("/work/tree")).unwrap();
        let e = entree(&r);
        let pid = std::process::id();
        assert_eq!(e["pid"], pid, "{e}");
        assert_eq!(e["procStart"], proc_start(pid), "{e}");
        assert_eq!(e["cwd"], "/work/tree");
        assert_eq!(e["name"], "algareth");
        assert_eq!(e["nameSource"], NAME_SOURCE);
        assert_eq!(e["kind"], "interactive");
        assert_eq!(e["entrypoint"], "rag3weaver");
        assert_eq!(e["peerProtocol"], 1);
        assert_eq!(e["peerFeatures"], json!([]), "no idle notices before lot 5");
        assert_eq!(e["status"], "busy");
        assert_eq!(e["messagingSocketPath"], r.socks.join(format!("{pid}.sock")).to_string_lossy().as_ref());
        assert!(e["pidDomain"].as_str().unwrap_or_default().starts_with("linux:"), "{e}");
        assert!(e["sessionId"].as_str().is_some_and(|s| s.len() == 36), "{e}");
        for k in ["startedAt", "updatedAt", "statusUpdatedAt", "nameSince"] {
            assert!(e[k].as_u64().is_some_and(|v| v > 1_700_000_000_000), "{k}: {e}");
        }
        let restes: Vec<_> = std::fs::read_dir(&r.sessions).unwrap().map(|d| d.unwrap().file_name()).collect();
        assert_eq!(restes.len(), 1, "written atomically, nothing beside it: {restes:?}");
    }

    #[test]
    fn the_status_follows_the_step() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        let avant = entree(&r)["statusUpdatedAt"].as_u64().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        pair.statut("p/a", false);
        let e = entree(&r);
        assert_eq!(e["status"], "idle");
        assert!(e["statusUpdatedAt"].as_u64().unwrap() > avant, "{e}");
        pair.statut("p/a", true);
        assert_eq!(entree(&r)["status"], "busy");
        pair.statut("p/autre", false);
        assert_eq!(entree(&r)["status"], "busy", "another task does not move the entry");
    }

    #[test]
    fn the_entry_goes_with_the_task_and_with_the_host() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let fichier = r.sessions.join(format!("{}.json", std::process::id()));
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        pair.fermer("p/autre");
        assert!(fichier.exists(), "only its own task closes it");
        pair.fermer("p/a");
        assert!(!fichier.exists(), "gone at the end of the task");
        pair.ouvrir("p/b", "b", Path::new("/w")).unwrap();
        assert!(fichier.exists());
        drop(pair);
        assert!(!fichier.exists(), "gone with the host");
    }

    #[test]
    fn a_second_paired_task_is_refused() {
        let t = tempfile::tempdir().unwrap();
        let pair = Pair::new(registre(&t));
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        let e = pair.ouvrir("p/b", "b", Path::new("/w")).unwrap_err();
        assert!(e.contains("p/a") && e.contains("one task"), "{e}");
        assert!(pair.ouvrir("p/a", "a", Path::new("/w")).is_ok(), "the same task again is not a second one");
    }

    #[test]
    fn a_dead_entry_of_ours_is_cleaned_at_start_and_no_other() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        std::fs::create_dir_all(&r.sessions).unwrap();
        let ecrire = |nom: &str, v: Value| std::fs::write(r.sessions.join(nom), v.to_string()).unwrap();
        let moi = std::process::id();
        // A pid that cannot be alive (above pid_max).
        ecrire("4194399.json", json!({ "pid": 4194399u32, "procStart": "1", "nameSource": NAME_SOURCE }));
        ecrire("4194398.json", json!({ "pid": 4194398u32, "procStart": "1", "nameSource": "user" }));
        ecrire(&format!("{moi}.json"), json!({ "pid": moi, "procStart": "1", "nameSource": NAME_SOURCE }));
        ecrire("1.json", json!({ "pid": 1, "procStart": proc_start(1), "nameSource": NAME_SOURCE }));
        ecrire("illisible.json", Value::String("pas une entrée".into()));
        let mut retires: Vec<String> = r.nettoyer().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        retires.sort();
        let mut attendus = vec![format!("{moi}.json"), "4194399.json".to_string()];
        attendus.sort();
        assert_eq!(retires, attendus, "dead, or its pid taken by another process");
        for reste in ["4194398.json", "1.json", "illisible.json"] {
            assert!(r.sessions.join(reste).exists(), "{reste} is not ours or is alive");
        }
    }
}
