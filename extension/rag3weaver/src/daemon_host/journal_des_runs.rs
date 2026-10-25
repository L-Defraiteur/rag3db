//! **Le journal des runs de l'hôte** (page 04 du 25 octobre 2026, c7) : à la fin
//! de tout run racine (un appel d'outil, une réaction, un tour de chat), l'hôte
//! écrit `runs_root/<backend>/<run>.json` — les événements du bus pour ce run et
//! ses runs enfants, et ce que celui qui l'a lancé y ajoute (les tours d'un chat
//! tels que le modèle les a reçus, la tâche, les propositions). Le juge de
//! session le lit ; le DAG vivant pourra rejouer un run passé.
//!
//! Borné : un run garde au plus `max_octets` d'événements (le milieu tombe, et le
//! fichier le dit) ; un backend garde au plus `max_runs` fichiers (les plus vieux
//! sont effacés, et l'hôte le dit).

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use crate::events::EventBus;

/// Les sujets dont les événements portent un run.
const SUJETS: [&str; 4] = ["agent", "dataflow", "reactions", "catalog"];
/// Le temps laissé aux événements d'un run encore en route sur un autre sujet
/// (chaque sujet a son fil) avant d'écrire son journal.
const GRACE: Duration = Duration::from_millis(250);

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

pub struct JournalDesRuns {
    /// Faux : le journal ne suit rien (un hôte de témoin sans racine donnée).
    actif: bool,
    racine: PathBuf,
    max_runs: usize,
    max_octets: usize,
    etat: Mutex<Etat>,
}

/// `RAG3WEAVER_RUNS_DIR`, sinon `~/.local/share/rag3weaver/runs`.
pub fn racine_par_defaut() -> PathBuf {
    std::env::var_os("RAG3WEAVER_RUNS_DIR").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share/rag3weaver/runs")
    })
}

fn reglage(nom: &str, defaut: usize) -> usize {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

impl JournalDesRuns {
    /// Le journal sous `racine` ; ses bornes par `RAG3WEAVER_RUNS_MAX` (fichiers
    /// par backend, 200) et `RAG3WEAVER_RUN_MAX_OCTETS` (par run, 2 Mio).
    pub fn new(racine: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            actif: true,
            racine,
            max_runs: reglage("RAG3WEAVER_RUNS_MAX", 200).max(1),
            max_octets: reglage("RAG3WEAVER_RUN_MAX_OCTETS", 2 << 20).max(4096),
            etat: Mutex::new(Etat::default()),
        })
    }

    /// Les mêmes, bornes données (les témoins).
    pub fn avec_bornes(racine: PathBuf, max_runs: usize, max_octets: usize) -> Arc<Self> {
        Arc::new(Self { actif: true, racine, max_runs: max_runs.max(1), max_octets, etat: Mutex::new(Etat::default()) })
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

    /// Le fichier d'un run, s'il est écrit.
    pub fn fichier(&self, backend: &str, run: &str) -> Option<PathBuf> {
        let p = self.racine.join(backend).join(format!("{run}.json"));
        p.is_file().then_some(p)
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
    /// racine écrit son journal, après la grâce.
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
        if let Some(fichier) = etat.ecrits.get(run).cloned() {
            drop(etat);
            let mut v: Value = std::fs::read(&fichier).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_else(|| json!({}));
            if let Value::Object(o) = &mut v {
                o.extend(ajouts);
            }
            if let Err(e) = std::fs::write(&fichier, serde_json::to_vec_pretty(&v).unwrap_or_default()) {
                eprintln!("[journal des runs] {} : {e}", fichier.display());
            }
            return;
        }
        let r = etat.en_cours.entry(run.to_string()).or_insert_with(|| RunEnCours { backend: backend.to_string(), ..Default::default() });
        r.ajouts.extend(ajouts);
    }

    /// Écrit le journal d'un run racine, borné, puis efface les plus vieux du
    /// backend au-delà de la borne, en le disant.
    fn ecrire(&self, run: &str) {
        let mut etat = self.etat.lock().unwrap_or_else(|p| p.into_inner());
        let Some(r) = etat.en_cours.remove(run) else { return };
        etat.racine_de.retain(|_, racine| racine != run);
        let dossier = self.racine.join(&r.backend);
        let fichier = dossier.join(format!("{run}.json"));
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
        if let Err(e) = std::fs::create_dir_all(&dossier).and_then(|_| std::fs::write(&fichier, serde_json::to_vec_pretty(&v).unwrap_or_default())) {
            eprintln!("[journal des runs] {} : {e}", fichier.display());
            return;
        }
        self.elaguer(&dossier);
    }

    /// Les plus vieux journaux d'un backend au-delà de `max_runs`, effacés, et dits.
    fn elaguer(&self, dossier: &Path) {
        let Ok(lus) = std::fs::read_dir(dossier) else { return };
        let mut fichiers: Vec<(std::time::SystemTime, PathBuf)> = lus
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| std::fs::metadata(&p).and_then(|m| m.modified()).ok().map(|t| (t, p)))
            .collect();
        if fichiers.len() <= self.max_runs {
            return;
        }
        fichiers.sort();
        let trop = fichiers.len() - self.max_runs;
        for (_, p) in fichiers.into_iter().take(trop) {
            let _ = std::fs::remove_file(&p);
            self.etat.lock().unwrap_or_else(|e| e.into_inner()).ecrits.retain(|_, f| f != &p);
        }
        eprintln!("[journal des runs] {} : {trop} journal(aux) effacé(s), au-delà de {} par backend (RAG3WEAVER_RUNS_MAX)", dossier.display(), self.max_runs);
    }
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
            if let Some(p) = j.fichier(backend, run) {
                if let Ok(v) = serde_json::from_slice::<Value>(&std::fs::read(&p).unwrap()) {
                    return v;
                }
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

    /// **Borné** : au-delà de `max_runs` par backend, les plus vieux sont
    /// effacés ; au-delà de `max_octets`, le milieu tombe et le fichier le dit.
    #[test]
    fn the_journal_is_bounded_in_count_and_size() {
        let d = tempfile::tempdir().unwrap();
        let j = JournalDesRuns::avec_bornes(d.path().to_path_buf(), 2, 8192);
        let bus = EventBus::new(256);
        j.suivre("b1", &bus);
        std::thread::sleep(Duration::from_millis(50));
        for i in 0..3 {
            let run = format!("r{i}");
            bus.emit(debut(&run, None, "graph"));
            bus.emit(fin(&run, "graph"));
            attendre_le_fichier(&j, "b1", &run);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(j.fichier("b1", "r0").is_none(), "le plus vieux est effacé");
        assert!(j.fichier("b1", "r2").is_some());
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
