//! **Le journal des runs de l'hôte** (page 04 du 25 octobre 2026, c7) : à la fin
//! de tout run racine (un appel d'outil, une réaction, un tour de chat), l'hôte
//! écrit `runs_root/<backend>/<run>.json.zst` — les événements du bus pour ce run
//! et ses runs enfants, et ce que celui qui l'a lancé y ajoute (les tours d'un
//! chat tels que le modèle les a reçus, la tâche, les propositions). Le juge de
//! session le lit ; le DAG vivant pourra rejouer un run passé.
//!
//! Borné (page 06, Lucie : « par poids plutôt que par nombre ») :
//! - un run garde au plus `max_octets` d'événements (2 Mio ; le milieu tombe, et
//!   le fichier le dit). C'est un plafond, pas une taille : un appel d'outil
//!   ordinaire pèse quelques Kio ;
//! - le journal entier, tous backends confondus, garde au plus `max_total` octets
//!   compressés (5 Gio) et `max_jours` jours (90) : au-delà, les runs les plus
//!   vieux tombent, à la première borne atteinte, et l'hôte le dit ;
//! - un run **épinglé** (`epingle` dans son fichier) ne tombe jamais : la
//!   déclaration d'un outil, un défaut bloquant rangé par le juge, un appel
//!   explicite. Ce sont les runs qu'on veut retrouver longtemps après.
//!
//! Les fichiers sont compressés (zstd) ; le lecteur lit aussi un `.json` d'avant.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, SystemTime};

use crate::events::EventBus;

/// Les sujets dont les événements portent un run.
const SUJETS: [&str; 4] = ["agent", "dataflow", "reactions", "catalog"];
/// Le temps laissé aux événements d'un run encore en route sur un autre sujet
/// (chaque sujet a son fil) avant d'écrire son journal.
const GRACE: Duration = Duration::from_millis(250);
/// L'extension d'un journal écrit ; `.json` est celle d'avant la compression.
const EXTENSION: &str = "json.zst";
/// Le niveau de zstd : le défaut de la bibliothèque, rapide à l'écriture.
const NIVEAU_ZSTD: i32 = 3;

#[derive(Default)]
struct RunEnCours {
    backend: String,
    debut: Option<Value>,
    evenements: Vec<Value>,
    ajouts: Map<String, Value>,
}

#[derive(Default)]
struct Etat {
    /// Les runs racines en cours, par id.
    en_cours: HashMap<String, RunEnCours>,
    /// Un run enfant → sa racine.
    racine_de: HashMap<String, String>,
    /// Les runs écrits, par id → leur fichier (pour un ajout tardif).
    ecrits: HashMap<String, PathBuf>,
}

/// Un journal écrit, tel que la borne le compte.
struct Entree {
    octets: u64,
    /// Lu dans le fichier quand il devient candidat à tomber, puis gardé.
    epingle: Option<bool>,
}

/// **L'inventaire des journaux écrits**, par âge : relevé sur le disque à la
/// première écriture, tenu ensuite à chaque écriture et effacement (un relevé à
/// chaque run coûterait un parcours de centaines de milliers de fichiers).
#[derive(Default)]
struct Inventaire {
    par_age: BTreeMap<(SystemTime, PathBuf), Entree>,
    age_de: HashMap<PathBuf, SystemTime>,
    total: u64,
}

impl Inventaire {
    fn relever(racine: &Path) -> Self {
        let mut inv = Inventaire::default();
        let Ok(backends) = std::fs::read_dir(racine) else { return inv };
        for b in backends.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()) {
            let Ok(lus) = std::fs::read_dir(&b) else { continue };
            for p in lus.filter_map(Result::ok).map(|e| e.path()).filter(|p| est_un_journal(p)) {
                if let Ok(m) = std::fs::metadata(&p) {
                    inv.noter(p, m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len(), None);
                }
            }
        }
        inv
    }

    /// Un journal écrit ou réécrit : son âge reste celui de sa première écriture.
    fn noter(&mut self, p: PathBuf, age: SystemTime, octets: u64, epingle: Option<bool>) {
        let age = *self.age_de.entry(p.clone()).or_insert(age);
        let cle = (age, p);
        // Réécrit sans rien dire de l'épingle : ce qu'on en savait reste.
        let su = self.par_age.get(&cle).and_then(|e| e.epingle);
        let ancien = self.par_age.insert(cle, Entree { octets, epingle: epingle.or(su) });
        self.total = self.total + octets - ancien.map_or(0, |e| e.octets);
    }

    fn oublier(&mut self, p: &Path) {
        if let Some(age) = self.age_de.remove(p) {
            if let Some(e) = self.par_age.remove(&(age, p.to_path_buf())) {
                self.total -= e.octets;
            }
        }
    }
}

fn est_un_journal(p: &Path) -> bool {
    let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    nom.ends_with(".json.zst") || nom.ends_with(".json")
}

/// **Lit un journal**, compressé ou d'avant la compression.
pub fn lire_le_fichier(p: &Path) -> Result<Value, String> {
    let octets = std::fs::read(p).map_err(|e| format!("{} : {e}", p.display()))?;
    let json = if p.extension().is_some_and(|x| x == "zst") {
        zstd::decode_all(octets.as_slice()).map_err(|e| format!("{} : {e}", p.display()))?
    } else {
        octets
    };
    serde_json::from_slice(&json).map_err(|e| format!("{} : {e}", p.display()))
}

/// Écrit un journal compressé ; rend son poids sur le disque.
fn ecrire_le_fichier(p: &Path, v: &Value) -> std::io::Result<u64> {
    let json = serde_json::to_vec(v).unwrap_or_default();
    let octets = zstd::encode_all(json.as_slice(), NIVEAU_ZSTD)?;
    std::fs::write(p, &octets)?;
    Ok(octets.len() as u64)
}

/// **Un appel qui déclare un outil** : `declare`, `declare_set` ou `propose`
/// dont un fichier est le graphe d'un outil (`%% tool:`) ou un manifeste qui en
/// nomme (`tools`). Son run est épinglé (la graine §7 veut le retrouver : qui a
/// déclaré l'outil, quand, dans quel contexte).
pub fn declare_un_outil(outil: &str, arguments: &Value) -> bool {
    if !matches!(outil, "declare" | "declare_set" | "propose") {
        return false;
    }
    let un = arguments.get("path").map(|p| json!({ "path": p, "content": arguments["content"] }));
    let fichiers: Vec<Value> = arguments["files"].as_array().cloned().unwrap_or_default().into_iter().chain(un).collect();
    fichiers.iter().any(|f| {
        let (chemin, contenu) = (f["path"].as_str().unwrap_or_default(), f["content"].as_str().unwrap_or_default());
        (chemin.ends_with(".mmd") && contenu.lines().any(|l| l.trim_start().starts_with("%% tool:")))
            || (chemin.ends_with(".json") && serde_json::from_str::<Value>(contenu).is_ok_and(|v| v["tools"].is_object()))
    })
}

pub struct JournalDesRuns {
    /// Faux : le journal ne suit rien (un hôte de témoin sans racine donnée).
    actif: bool,
    racine: PathBuf,
    /// Le poids de tout le journal, compressé.
    max_total: u64,
    /// L'âge d'un run, en jours.
    max_jours: u64,
    /// Les événements d'un run, non compressés.
    max_octets: usize,
    etat: Mutex<Etat>,
    /// Relevé à la première écriture.
    inventaire: Mutex<Option<Inventaire>>,
}

/// `RAG3WEAVER_RUNS_DIR`, sinon `~/.local/share/rag3weaver/runs`.
pub fn racine_par_defaut() -> PathBuf {
    std::env::var_os("RAG3WEAVER_RUNS_DIR").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share/rag3weaver/runs")
    })
}

fn reglage(nom: &str, defaut: u64) -> u64 {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

impl JournalDesRuns {
    /// Le journal sous `racine` ; ses bornes par `RAG3WEAVER_RUNS_MAX_OCTETS`
    /// (tout le journal, compressé, 5 Gio), `RAG3WEAVER_RUNS_MAX_JOURS` (90) et
    /// `RAG3WEAVER_RUN_MAX_OCTETS` (par run, 2 Mio).
    pub fn new(racine: PathBuf) -> Arc<Self> {
        Self::avec_bornes(
            racine,
            reglage("RAG3WEAVER_RUNS_MAX_OCTETS", 5 << 30),
            reglage("RAG3WEAVER_RUNS_MAX_JOURS", 90),
            reglage("RAG3WEAVER_RUN_MAX_OCTETS", 2 << 20).max(4096) as usize,
        )
    }

    /// Les mêmes, bornes données (les témoins).
    pub fn avec_bornes(racine: PathBuf, max_total: u64, max_jours: u64, max_octets: usize) -> Arc<Self> {
        Arc::new(Self {
            actif: true,
            racine,
            max_total,
            max_jours: max_jours.max(1),
            max_octets,
            etat: Mutex::new(Etat::default()),
            inventaire: Mutex::new(None),
        })
    }

    /// **Le journal par défaut d'un hôte** : sous `RAG3WEAVER_RUNS_DIR`, sinon
    /// `~/.local/share/rag3weaver/runs`. Sous les témoins de la lib, il ne suit
    /// rien tant qu'on ne lui donne pas de racine (`with_runs_root`, ou la
    /// variable) : un hôte de témoin n'écrit pas dans le dossier de la personne.
    pub fn par_defaut() -> Arc<Self> {
        let mut j = Self::new(racine_par_defaut());
        if cfg!(test) && std::env::var_os("RAG3WEAVER_RUNS_DIR").is_none() {
            Arc::get_mut(&mut j).expect("neuf").actif = false;
        }
        j
    }

    pub fn racine(&self) -> &Path {
        &self.racine
    }

    /// Le fichier d'un run, s'il est écrit (compressé, ou d'avant).
    pub fn fichier(&self, backend: &str, run: &str) -> Option<PathBuf> {
        let dossier = self.racine.join(backend);
        [format!("{run}.{EXTENSION}"), format!("{run}.json")].into_iter().map(|n| dossier.join(n)).find(|p| p.is_file())
    }

    /// Le journal d'un run, s'il est écrit et se lit.
    pub fn lire(&self, backend: &str, run: &str) -> Option<Value> {
        lire_le_fichier(&self.fichier(backend, run)?).ok()
    }

    /// Le poids du journal entier, tel que la borne le compte (relevé s'il ne
    /// l'est pas encore).
    pub fn octets(&self) -> u64 {
        let mut inv = self.inventaire.lock().unwrap_or_else(|p| p.into_inner());
        inv.get_or_insert_with(|| Inventaire::relever(&self.racine)).total
    }

    /// **Suit le bus d'un backend** qui vient de s'ouvrir : un fil par sujet, à
    /// lui (pas le runtime des graphes : voir `Flux::relayer`), qui finit avec
    /// le bus ou le journal.
    pub fn suivre(self: &Arc<Self>, backend: &str, bus: &EventBus) {
        if !self.actif {
            return;
        }
        for sujet in SUJETS {
            let mut rx = bus.subscribe(sujet);
            let (journal, backend) = (Arc::downgrade(self), backend.to_string());
            let lance = std::thread::Builder::new().name(format!("journal-{sujet}")).spawn(move || loop {
                let evenement = match rx.recv_blocking() {
                    Ok(e) => e,
                    Err(async_broadcast::RecvError::Overflowed(_)) => continue,
                    Err(async_broadcast::RecvError::Closed) => break,
                };
                let Some(journal) = journal.upgrade() else { break };
                journal.noter(&backend, &evenement);
            });
            if let Err(e) = lance {
                eprintln!("[journal des runs] le fil de « {sujet} » ne part pas : {e}");
            }
        }
    }

    /// Un événement du bus : rangé sous la racine de son run ; la fin d'un run
    /// racine écrit son journal, après la grâce. Un appel d'agent qui déclare un
    /// outil épingle sa racine.
    fn noter(self: &Arc<Self>, backend: &str, e: &crate::events::Event) {
        let Some(run) = e.run().map(str::to_string) else { return };
        let json = e.to_json();
        let mut etat = self.etat.lock().unwrap_or_else(|p| p.into_inner());
        if let crate::events::Event::RunStarted { parent, .. } = e {
            if let Some(p) = parent {
                let racine = etat.racine_de.get(p).cloned().unwrap_or_else(|| p.clone());
                etat.racine_de.insert(run.clone(), racine);
            } else {
                let r = etat.en_cours.entry(run.clone()).or_default();
                r.backend = backend.to_string();
                r.debut = Some(json.clone());
            }
        }
        let racine = etat.racine_de.get(&run).cloned().unwrap_or_else(|| run.clone());
        // Un événement d'un run déjà écrit, ou d'un run dont le début n'a pas
        // été vu (le journal monté en cours de route) : rangé quand même, sous
        // un run racine neuf s'il le faut.
        let r = etat.en_cours.entry(racine.clone()).or_insert_with(|| RunEnCours { backend: backend.to_string(), ..Default::default() });
        r.evenements.push(json);
        if let crate::events::Event::ToolCallStarted { tool, arguments, .. } = e {
            let arguments = serde_json::from_str::<Value>(arguments).unwrap_or(Value::Null);
            if declare_un_outil(tool, &arguments) {
                r.ajouts.insert("epingle".into(), epingle(&format!("déclaration d'un outil ({tool})")));
            }
        }
        let fini = matches!(e, crate::events::Event::RunFinished { .. }) && run == racine;
        drop(etat);
        if fini {
            let journal: Weak<Self> = Arc::downgrade(self);
            let _ = std::thread::Builder::new().name("journal-ecrire".into()).spawn(move || {
                std::thread::sleep(GRACE);
                if let Some(j) = journal.upgrade() {
                    j.ecrire(&racine);
                }
            });
        }
    }

    /// **Ajoute au journal d'un run** ce que celui qui l'a lancé en sait (les
    /// tours d'un chat, la tâche, les propositions) : gardé jusqu'à l'écriture,
    /// ou fondu dans le fichier s'il est déjà écrit.
    pub fn ajouter(&self, backend: &str, run: &str, ajouts: Map<String, Value>) {
        if !self.actif {
            return;
        }
        let mut etat = self.etat.lock().unwrap_or_else(|p| p.into_inner());
        let ecrit = etat.ecrits.get(run).cloned().or_else(|| self.fichier(backend, run));
        if let Some(fichier) = ecrit {
            drop(etat);
            self.fondre(&fichier, ajouts);
            return;
        }
        let r = etat.en_cours.entry(run.to_string()).or_insert_with(|| RunEnCours { backend: backend.to_string(), ..Default::default() });
        r.ajouts.extend(ajouts);
    }

    /// **Épingle un run** : il ne tombera jamais, ni par le poids ni par l'âge.
    /// Écrit ou pas encore : l'épingle suit le run jusqu'à son fichier.
    pub fn epingler(&self, backend: &str, run: &str, raison: &str) {
        let mut a = Map::new();
        a.insert("epingle".into(), epingle(raison));
        self.ajouter(backend, run, a);
    }

    /// Fond des ajouts dans un journal écrit : relu, réécrit compressé (un
    /// `.json` d'avant devient `.json.zst`), son poids et son épingle tenus.
    fn fondre(&self, fichier: &Path, ajouts: Map<String, Value>) {
        let mut v = lire_le_fichier(fichier).unwrap_or_else(|_| json!({}));
        if let Value::Object(o) = &mut v {
            o.extend(ajouts);
        }
        let cible = if fichier.extension().is_some_and(|x| x == "zst") { fichier.to_path_buf() } else { fichier.with_extension(EXTENSION) };
        match ecrire_le_fichier(&cible, &v) {
            Err(e) => eprintln!("[journal des runs] {} : {e}", cible.display()),
            Ok(octets) => {
                let mut inv = self.inventaire.lock().unwrap_or_else(|p| p.into_inner());
                let inv = inv.get_or_insert_with(|| Inventaire::relever(&self.racine));
                if cible != fichier {
                    let _ = std::fs::remove_file(fichier);
                    inv.oublier(fichier);
                }
                inv.noter(cible.clone(), SystemTime::now(), octets, v.get("epingle").map(|_| true));
                if let Some(run) = v["run"].as_str() {
                    self.etat.lock().unwrap_or_else(|p| p.into_inner()).ecrits.insert(run.to_string(), cible);
                }
            }
        }
    }

    /// Écrit le journal d'un run racine, borné et compressé, puis fait tomber
    /// les plus vieux au-delà des bornes, en le disant.
    fn ecrire(&self, run: &str) {
        let mut etat = self.etat.lock().unwrap_or_else(|p| p.into_inner());
        let Some(r) = etat.en_cours.remove(run) else { return };
        etat.racine_de.retain(|_, racine| racine != run);
        let dossier = self.racine.join(&r.backend);
        let fichier = dossier.join(format!("{run}.{EXTENSION}"));
        etat.ecrits.insert(run.to_string(), fichier.clone());
        drop(etat);

        let (evenements, tronques) = borner(r.evenements, self.max_octets);
        let fin = evenements.iter().rev().find(|e| e["kind"] == "RunFinished").cloned().unwrap_or(Value::Null);
        let mut v = json!({
            "run": run,
            "backend": r.backend,
            "debut": r.debut,
            "fin": fin,
            "evenements": evenements,
        });
        if tronques > 0 {
            v["evenements_tronques"] = json!(tronques);
        }
        if let Value::Object(o) = &mut v {
            o.extend(r.ajouts);
        }
        let ecrit = std::fs::create_dir_all(&dossier).and_then(|_| ecrire_le_fichier(&fichier, &v));
        match ecrit {
            Err(e) => eprintln!("[journal des runs] {} : {e}", fichier.display()),
            Ok(octets) => {
                let mut inv = self.inventaire.lock().unwrap_or_else(|p| p.into_inner());
                let inv = inv.get_or_insert_with(|| Inventaire::relever(&self.racine));
                inv.noter(fichier, SystemTime::now(), octets, Some(v.get("epingle").is_some()));
                self.elaguer(inv);
            }
        }
    }

    /// **Les plus vieux runs tombent**, tous backends confondus, tant que le
    /// journal dépasse son poids ou qu'ils dépassent leur âge ; un run épinglé
    /// reste (lu dans son fichier la première fois qu'il serait tombé).
    fn elaguer(&self, inv: &mut Inventaire) {
        let limite_d_age = SystemTime::now().checked_sub(Duration::from_secs(self.max_jours * 86_400)).unwrap_or(SystemTime::UNIX_EPOCH);
        let (mut par_poids, mut par_age, mut octets_tombes) = (0usize, 0usize, 0u64);
        let cles: Vec<(SystemTime, PathBuf)> = inv.par_age.keys().cloned().collect();
        for cle in cles {
            let trop_vieux = cle.0 < limite_d_age;
            if !trop_vieux && inv.total <= self.max_total {
                break;
            }
            let Some(entree) = inv.par_age.get_mut(&cle) else { continue };
            let epingle = *entree.epingle.get_or_insert_with(|| lire_le_fichier(&cle.1).is_ok_and(|v| v.get("epingle").is_some()));
            if epingle {
                continue;
            }
            let octets = entree.octets;
            let _ = std::fs::remove_file(&cle.1);
            inv.oublier(&cle.1);
            self.etat.lock().unwrap_or_else(|e| e.into_inner()).ecrits.retain(|_, f| f != &cle.1);
            octets_tombes += octets;
            if trop_vieux {
                par_age += 1;
            } else {
                par_poids += 1;
            }
        }
        if par_poids + par_age > 0 {
            eprintln!(
                "[journal des runs] {} : {} run(s) tombé(s) ({octets_tombes} octets) — {par_age} au-delà de {} jours (RAG3WEAVER_RUNS_MAX_JOURS), {par_poids} au-delà de {} octets (RAG3WEAVER_RUNS_MAX_OCTETS) ; reste {} octets",
                self.racine.display(),
                par_poids + par_age,
                self.max_jours,
                self.max_total,
                inv.total
            );
        }
        if inv.total > self.max_total {
            eprintln!(
                "[journal des runs] {} : {} octets, au-delà de la borne de {} : il ne reste que des runs épinglés",
                self.racine.display(),
                inv.total,
                self.max_total
            );
        }
    }
}

/// La marque d'un run épinglé : pourquoi, et quand.
fn epingle(raison: &str) -> Value {
    let instant = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    json!({ "raison": raison, "instant": instant })
}

/// Garde la tête et la queue des événements dans `max_octets` ; rend le nombre
/// d'événements du milieu qui sont tombés.
fn borner(evenements: Vec<Value>, max_octets: usize) -> (Vec<Value>, usize) {
    let taille = |e: &Value| e.to_string().len();
    if evenements.iter().map(taille).sum::<usize>() <= max_octets {
        return (evenements, 0);
    }
    let moitie = max_octets / 2;
    let (mut tete, mut pris) = (Vec::new(), 0);
    for e in &evenements {
        if pris + taille(e) > moitie {
            break;
        }
        pris += taille(e);
        tete.push(e.clone());
    }
    let (mut queue, mut pris) = (Vec::new(), 0);
    for e in evenements.iter().rev() {
        if pris + taille(e) > moitie || tete.len() + queue.len() >= evenements.len() {
            break;
        }
        pris += taille(e);
        queue.push(e.clone());
    }
    queue.reverse();
    let tombes = evenements.len() - tete.len() - queue.len();
    tete.push(json!({ "kind": "EvenementsTronques", "count": tombes }));
    tete.extend(queue);
    (tete, tombes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;

    fn attendre_le_fichier(j: &JournalDesRuns, backend: &str, run: &str) -> Value {
        let fin = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(v) = j.lire(backend, run) {
                return v;
            }
            assert!(std::time::Instant::now() < fin, "le journal de {run} n'est jamais écrit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn debut(run: &str, parent: Option<&str>, kind: &str) -> Event {
        Event::RunStarted {
            run: run.into(),
            parent: parent.map(str::to_string),
            kind: kind.into(),
            name: "t".into(),
            domain: String::new(),
            scope: None,
            target: None,
        }
    }

    fn fin(run: &str, kind: &str) -> Event {
        Event::RunFinished { run: run.into(), kind: kind.into(), ms: 3, ok: true, cause: None, cancelled_by: None }
    }

    /// **Un run racine laisse son journal**, avec les événements de ses runs
    /// enfants (le graphe d'un outil appelé par l'agent) et ce que son lanceur y
    /// ajoute, avant ou après l'écriture.
    #[test]
    fn a_root_run_leaves_its_journal_with_its_children_and_additions() {
        let d = tempfile::tempdir().unwrap();
        let j = JournalDesRuns::new(d.path().to_path_buf());
        let bus = EventBus::new(64);
        j.suivre("b1", &bus);
        std::thread::sleep(Duration::from_millis(50));
        let mut avant = Map::new();
        avant.insert("tache".into(), json!({ "nom": "salut" }));
        j.ajouter("b1", "a1", avant);
        bus.emit(debut("a1", None, "agent"));
        bus.emit(Event::ToolCallStarted { run: "a1".into(), agent: "chat".into(), call_id: "c1".into(), tool: "read_file".into(), arguments: "{}".into() });
        bus.emit(debut("g1", Some("a1"), "graph"));
        bus.emit(fin("g1", "graph"));
        bus.emit(fin("a1", "agent"));
        let v = attendre_le_fichier(&j, "b1", "a1");
        let genres: Vec<&str> = v["evenements"].as_array().unwrap().iter().filter_map(|e| e["kind"].as_str()).collect();
        assert!(genres.contains(&"ToolCallStarted"), "{genres:?}");
        assert_eq!(genres.iter().filter(|g| **g == "RunStarted").count(), 2, "l'enfant est rangé sous sa racine : {genres:?}");
        assert!(j.fichier("b1", "g1").is_none(), "un run enfant n'a pas de fichier à lui");
        assert_eq!(v["tache"]["nom"], "salut", "l'ajout d'avant l'écriture : {v}");
        assert_eq!(v["fin"]["kind"], "RunFinished");
        let mut apres = Map::new();
        apres.insert("tours".into(), json!([{ "role": "user", "content": "dis salut" }]));
        j.ajouter("b1", "a1", apres);
        let v = attendre_le_fichier(&j, "b1", "a1");
        assert_eq!(v["tours"][0]["content"], "dis salut", "l'ajout d'après l'écriture est fondu : {v}");
    }

    /// Un run, de son début à sa fin, sous la racine `run`.
    fn un_run(bus: &EventBus, run: &str, appels: usize) {
        bus.emit(debut(run, None, "agent"));
        for i in 0..appels {
            bus.emit(Event::ToolCallStarted { run: run.into(), agent: "a".into(), call_id: format!("c{i}"), tool: "read_file".into(), arguments: format!("{{\"path\":\"src/f{i}.rs\"}}") });
        }
        bus.emit(fin(run, "agent"));
    }

    /// **Borné par poids, tous backends confondus ; l'épinglé reste** (page 06,
    /// c7) : trois runs sous une borne minuscule, dont un épinglé ; les deux
    /// autres tombent dans l'ordre, l'épinglé reste, et le compte des octets est
    /// celui des fichiers sur le disque.
    #[test]
    fn the_journal_is_bounded_by_weight_and_a_pinned_run_stays() {
        let d = tempfile::tempdir().unwrap();
        let bus1 = EventBus::new(256);
        let bus2 = EventBus::new(256);
        // La borne : un peu plus que deux runs ; le troisième fait tomber le plus vieux.
        let sonde = JournalDesRuns::avec_bornes(d.path().join("sonde"), u64::MAX, 90, 1 << 20);
        sonde.suivre("b", &bus1);
        std::thread::sleep(Duration::from_millis(50));
        un_run(&bus1, "s", 3);
        attendre_le_fichier(&sonde, "b", "s");
        let un = std::fs::metadata(sonde.fichier("b", "s").unwrap()).unwrap().len();
        drop(sonde);

        let bus1 = EventBus::new(256);
        let j = JournalDesRuns::avec_bornes(d.path().join("runs"), un * 2 + un / 2, 90, 1 << 20);
        j.suivre("b1", &bus1);
        j.suivre("b2", &bus2);
        std::thread::sleep(Duration::from_millis(50));
        j.epingler("b1", "r0", "le témoin");
        un_run(&bus1, "r0", 3);
        attendre_le_fichier(&j, "b1", "r0");
        std::thread::sleep(Duration::from_millis(20));
        un_run(&bus2, "r1", 3);
        attendre_le_fichier(&j, "b2", "r1");
        std::thread::sleep(Duration::from_millis(20));
        un_run(&bus1, "r2", 3);
        attendre_le_fichier(&j, "b1", "r2");
        assert!(j.fichier("b1", "r0").is_some(), "l'épinglé reste");
        assert!(j.fichier("b2", "r1").is_none(), "le plus vieux non épinglé tombe, sur un autre backend");
        assert!(j.fichier("b1", "r2").is_some());
        std::thread::sleep(Duration::from_millis(20));
        un_run(&bus2, "r3", 3);
        attendre_le_fichier(&j, "b2", "r3");
        assert!(j.fichier("b1", "r2").is_none(), "puis le suivant, dans l'ordre");
        assert!(j.fichier("b1", "r0").is_some() && j.fichier("b2", "r3").is_some());
        assert_eq!(j.lire("b1", "r0").unwrap()["epingle"]["raison"], "le témoin");
        let disque: u64 = ["b1/r0", "b2/r3"].iter().map(|r| std::fs::metadata(d.path().join("runs").join(format!("{r}.json.zst"))).unwrap().len()).sum();
        assert_eq!(j.octets(), disque, "le compte des octets est celui du disque");
        // Relevé à neuf (un hôte relancé) : le même compte.
        let relu = JournalDesRuns::avec_bornes(d.path().join("runs"), u64::MAX, 90, 1 << 20);
        assert_eq!(relu.octets(), disque);
    }

    /// **Borné par l'âge** : un run plus vieux que la borne tombe à la prochaine
    /// écriture, l'épinglé jamais ; un `.json` d'avant la compression se lit,
    /// et un ajout le réécrit compressé.
    #[test]
    fn an_old_run_falls_by_age_and_an_uncompressed_journal_still_reads() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("runs");
        std::fs::create_dir_all(racine.join("b1")).unwrap();
        let vieux = std::time::SystemTime::now() - Duration::from_secs(3 * 86_400);
        for (run, epingle) in [("vieux", false), ("vieux-epingle", true)] {
            let mut v = json!({ "run": run, "backend": "b1", "evenements": [] });
            if epingle {
                v["epingle"] = json!({ "raison": "le témoin" });
            }
            let p = racine.join("b1").join(format!("{run}.json"));
            std::fs::write(&p, v.to_string()).unwrap();
            std::fs::File::options().write(true).open(&p).unwrap().set_modified(vieux).unwrap();
        }
        let j = JournalDesRuns::avec_bornes(racine.clone(), u64::MAX, 2, 1 << 20);
        assert_eq!(j.lire("b1", "vieux").unwrap()["run"], "vieux", "un .json d'avant se lit");
        let bus = EventBus::new(64);
        j.suivre("b1", &bus);
        std::thread::sleep(Duration::from_millis(50));
        un_run(&bus, "neuf", 1);
        attendre_le_fichier(&j, "b1", "neuf");
        assert!(j.fichier("b1", "vieux").is_none(), "au-delà de deux jours, il tombe");
        assert!(j.fichier("b1", "vieux-epingle").is_some(), "l'épinglé jamais");
        let mut a = Map::new();
        a.insert("note".into(), json!(1));
        j.ajouter("b1", "vieux-epingle", a);
        let p = j.fichier("b1", "vieux-epingle").unwrap();
        assert!(p.to_string_lossy().ends_with(".json.zst"), "réécrit compressé : {}", p.display());
        let v = j.lire("b1", "vieux-epingle").unwrap();
        assert_eq!((v["note"].clone(), v["epingle"]["raison"].clone()), (json!(1), json!("le témoin")));
    }

    /// Un appel qui déclare un outil : le graphe d'un outil, ou un manifeste qui en nomme.
    #[test]
    fn a_tool_declaration_is_recognised() {
        assert!(declare_un_outil("declare", &json!({ "path": "graphs/t.mmd", "content": "%% tool: t\ngraph LR\n" })));
        assert!(declare_un_outil("declare_set", &json!({ "files": [{ "path": "backend.json", "content": "{\"tools\":{}}" }] })));
        assert!(declare_un_outil("propose", &json!({ "files": [{ "path": "g.mmd", "content": "  %% tool: x" }] })));
        assert!(!declare_un_outil("declare", &json!({ "path": "views/v.html", "content": "%% tool: x" })));
        assert!(!declare_un_outil("read_file", &json!({ "path": "g.mmd", "content": "%% tool: x" })));
    }

    /// **Un run garde au plus `max_octets` d'événements** : le milieu tombe et
    /// le fichier le dit.
    #[test]
    fn a_run_is_capped_in_size() {
        let d = tempfile::tempdir().unwrap();
        let j = JournalDesRuns::avec_bornes(d.path().to_path_buf(), u64::MAX, 90, 8192);
        let bus = EventBus::new(256);
        j.suivre("b1", &bus);
        std::thread::sleep(Duration::from_millis(50));
        bus.emit(debut("gros", None, "agent"));
        for i in 0..200 {
            bus.emit(Event::ToolCallStarted { run: "gros".into(), agent: "a".into(), call_id: format!("c{i}"), tool: "read_file".into(), arguments: "x".repeat(200) });
        }
        bus.emit(fin("gros", "agent"));
        let v = attendre_le_fichier(&j, "b1", "gros");
        assert!(v["evenements_tronques"].as_u64().is_some_and(|n| n > 0), "le fichier dit ce qui est tombé : {}", v["evenements_tronques"]);
        assert!(v.to_string().len() < 8192 * 2, "borné");
        assert_eq!(v["fin"]["kind"], "RunFinished", "la fin est gardée");
    }
}
