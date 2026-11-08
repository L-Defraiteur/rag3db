//! **Le chat de l'atelier dans l'IDE** (page 11 de everything declarative,
//! lot K1) : un service de l'hôte, par backend. L'agent dont l'objet est le
//! gabarit travaille avec les outils que la déclaration lui donne — en mode
//! `approbation`, il **propose** (la vue des propositions, page 08) ; dans
//! une tâche (`auto`, page 09), il déclare sur la branche de la tâche ; sur un
//! arbre inscrit, `task_writes` le renvoie vers une tâche (page 10).
//!
//! - Le tour publie sur le **bus du backend** (`agent`, `tokens`), que le flux
//!   de l'IDE relaie : plus de bus privé.
//! - Il s'inscrit dans les **runs en vol** (C4) : l'arrêter, c'est
//!   `stop_run`, par le même chemin que tout run ; `RunFinished` dit qui.
//! - Il est **compté** (`Consumed`, sous le slug du modèle) ; la réponse dit
//!   ce que le tour a consommé.
//! - Un tour à la fois par backend ; un second est refusé en le disant.
//! - **La session est de l'état de l'hôte** (c7) : `<racine des chats>/<id>.json`,
//!   jamais dans l'arbre du projet (`.atelier/` reste à `serve --atelier`).
//!   Relue au démarrage ([`DaemonHost::with_chats_reread`]) : une session dont
//!   le manifeste a disparu est **orpheline**, dite, jamais rouverte en silence.
//! - **Un seul message système** (c7) : le même agent, et le verbe vient du
//!   mode de la déclaration.

use super::*;
use crate::chat::{ChatConfig, ChatRunOptions, ChatSession, EventHandler};
use crate::propositions::ModeDeclarations;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Ce qui donne le modèle d'un backend : celui de son manifeste, sinon le
/// défaut de Vertex. Les témoins le remplacent par un faux modèle.
/// Il prend le manifeste préparé, pas le backend ouvert : un équipier dont la base
/// ne s'ouvre pas garde son modèle (`models.llm`), lu sans elle
/// (page 07-novembre-2026-everything-declarative/01, § 1).
pub type ChatModel = Arc<dyn Fn(&crate::backend::PreparedBackend) -> Result<Arc<dyn crate::llm::Llm>, String> + Send + Sync>;

/// La session d'un backend, et si un tour est en cours.
pub(super) struct ChatEntry {
    manifest: PathBuf,
    session: ChatSession,
    busy: Option<String>,
    /// Le dernier appel au modèle du dernier tour : la jauge du contexte. En
    /// mémoire seulement — après un redémarrage de l'hôte, inconnu jusqu'au
    /// prochain tour.
    last_call: Option<crate::agent::LastCall>,
}

/// Ce qui se garde sur le disque : le manifeste (pour la relecture) et la session.
#[derive(serde::Serialize, serde::Deserialize)]
struct ChatFile {
    manifest: PathBuf,
    session: ChatSession,
}

/// La racine des sessions : `RAG3WEAVER_CHATS_DIR`, sinon
/// `~/.local/share/rag3weaver/chats` — à côté des tâches.
pub(super) fn default_chats_root() -> PathBuf {
    if let Some(d) = std::env::var_os("RAG3WEAVER_CHATS_DIR") {
        return PathBuf::from(d);
    }
    super::taches::racine_par_defaut().parent().map(|p| p.join("chats")).unwrap_or_else(|| std::env::temp_dir().join("rag3weaver-chats"))
}

/// Le modèle par défaut : `models.llm` du manifeste, sinon `gemini-3.8-flash`
/// (`regime::modele_agentique_nomme`, `GOOGLE_CLOUD_PROJECT` et les
/// identifiants Vertex de `GOOGLE_APPLICATION_CREDENTIALS`).
pub(super) fn default_chat_model() -> ChatModel {
    Arc::new(|prepared: &crate::backend::PreparedBackend| -> Result<Arc<dyn crate::llm::Llm>, String> {
        #[cfg(feature = "openai-llm")]
        {
            if let Some(source) = prepared.manifest.models.get(&crate::model_source::Capability::Llm) {
                return crate::model_source::connect_llm(source).map(|(llm, _)| llm);
            }
            crate::regime::modele_agentique_nomme("ide-chat")
                .map(|(llm, _)| Arc::new(llm) as Arc<dyn crate::llm::Llm>)
                .ok_or_else(|| {
                    "pas de modèle : ni `models.llm` au manifeste, ni Vertex (GOOGLE_CLOUD_PROJECT et des \
                     identifiants par GOOGLE_APPLICATION_CREDENTIALS)"
                        .to_string()
                })
        }
        #[cfg(not(feature = "openai-llm"))]
        {
            let _ = prepared;
            Err("cet hôte est bâti sans la feature `openai-llm` : le chat est éteint".to_string())
        }
    })
}

/// **Le message système** : un seul, et le verbe vient du mode de la
/// déclaration (c7) — deux textes divergeraient à la première retouche. Une
/// phrase sur le code suit le fait d'être, ou non, dans une tâche : le chat
/// n'écrit jamais hors d'une tâche (page 03 du 25 octobre 2026).
pub fn system_prompt(mode: ModeDeclarations, dans_une_tache: bool) -> String {
    let changer = match mode {
        ModeDeclarations::Approbation => {
            "To change them, propose them with propose (each file whole, with your reason): they are \
             checked as at load time WITHOUT being written, and a person applies or rejects the proposal \
             in the IDE. If a proposal is refused, read the refusal (it names the file and the line) and \
             propose again."
        }
        ModeDeclarations::Auto => {
            "To change them, declare them with declare (or declare_set for several files together): each \
             declaration is checked as at load time, put in service and committed with your reason — on \
             the task's branch when this backend is a task. If it is refused, read the refusal (it names \
             the file and the line) and correct it."
        }
    };
    let code = if dans_une_tache {
        "The code of this backend's tree is yours to change, on the task's branch only: find your way with \
         outline, grep and search, read before you change, then edit with edit_file (the content_hash that \
         read gives keeps you from overwriting a change). The person reviews the branch, then merges it."
    } else {
        "The code of this backend's tree can be read here, not written: if the person wants a change, tell \
         them to open a task in the IDE (its own branch, reviewed then merged) and to ask you there."
    };
    format!(
        "You work on a declared backend: its manifest, tool graphs (mermaid), JSON schemas, scripted nodes, \
         React components (tsx, with an optional click graph in mermaid) and views are files you can list and \
         read with list_declarations and read_declaration. {changer} Nothing is compiled by hand. {code} Use \
         the backend's own tools to look at its data. Answer briefly, in the language of the person."
    )
}

impl DaemonHost {
    /// La racine sous laquelle vivent les sessions de chat.
    pub fn with_chats_root(mut self, root: PathBuf) -> Self {
        self.chats_root = root;
        self
    }

    /// Le modèle des chats (un faux pour les témoins).
    pub fn with_chat_model(mut self, model: ChatModel) -> Self {
        self.chat_model = model;
        self
    }

    fn chat_path(&self, backend_id: &str) -> PathBuf {
        self.chats_root.join(format!("{backend_id}.json"))
    }

    /// Écrit la session d'un backend (un fichier neuf, renommé).
    fn write_chat(&self, backend_id: &str) -> Result<(), String> {
        let texte = {
            let chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
            let Some(e) = chats.get(backend_id) else { return Ok(()) };
            serde_json::to_vec(&json!({ "manifest": e.manifest, "session": e.session })).map_err(|e| e.to_string())?
        };
        std::fs::create_dir_all(&self.chats_root).map_err(|e| format!("{} : {e}", self.chats_root.display()))?;
        let chemin = self.chat_path(backend_id);
        let neuf = chemin.with_extension("json.neuf");
        std::fs::write(&neuf, texte).map_err(|e| format!("{} : {e}", neuf.display()))?;
        std::fs::rename(&neuf, &chemin).map_err(|e| format!("{} : {e}", chemin.display()))
    }

    /// **Relit les sessions** au démarrage : une session dont le manifeste est
    /// là est reprise (son backend inscrit, ouvert au premier appel) ; une autre
    /// est orpheline, dite, et son fichier reste où il est. Rend une ligne par
    /// session relue.
    pub fn reread_chats(&self) -> Vec<String> {
        let Ok(entrees) = std::fs::read_dir(&self.chats_root) else { return Vec::new() };
        let mut lignes = Vec::new();
        for entree in entrees.flatten() {
            let chemin = entree.path();
            if chemin.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let id = chemin.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            let lu: Result<ChatFile, String> = std::fs::read(&chemin)
                .map_err(|e| e.to_string())
                .and_then(|o| serde_json::from_slice(&o).map_err(|e| e.to_string()));
            match lu {
                Err(e) => lignes.push(format!("session de chat {} illisible, laissée : {e}", chemin.display())),
                Ok(f) if !f.manifest.is_file() => {
                    let pourquoi = format!("le manifeste {} a disparu", f.manifest.display());
                    lignes.push(format!("session de chat de {id} orpheline : {pourquoi}"));
                    self.orphan_chats.lock().unwrap_or_else(|p| p.into_inner()).push(json!({
                        "backend": id, "manifest": f.manifest, "pourquoi": pourquoi, "fichier": chemin,
                    }));
                }
                Ok(f) => match self.hosted(&f.manifest) {
                    Ok(h) => {
                        lignes.push(format!("session de chat de {} reprise", h.id));
                        self.chats
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .insert(h.id.clone(), ChatEntry { manifest: f.manifest, session: f.session, busy: None, last_call: None });
                    }
                    Err(e) => {
                        lignes.push(format!("session de chat de {id} orpheline : {e}"));
                        self.orphan_chats
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .push(json!({ "backend": id, "manifest": f.manifest, "pourquoi": e, "fichier": chemin }));
                    }
                },
            }
        }
        lignes
    }

    /// L'hôte, ses sessions de chat relues, chaque ligne dite : ce que fait le
    /// service au démarrage.
    pub fn with_chats_reread(self) -> Self {
        for l in self.reread_chats() {
            eprintln!("[démon] {l}");
        }
        self
    }

    /// Les conversations tenues, et si un tour y court (la barre du chat).
    pub(super) fn chats_busy(&self) -> Vec<(String, bool)> {
        self.chats.lock().unwrap_or_else(|p| p.into_inner()).iter().map(|(k, e)| (k.clone(), e.busy.is_some())).collect()
    }

    /// **Ce que la page montre d'un chat** : allumé ou non (et pourquoi), le
    /// run en cours, le fil de la session (sans le message système), et les
    /// sessions orphelines.
    pub fn chat_state(&self, backend_id: &str) -> Result<Value, String> {
        let hosted = self.hote_par_id(backend_id)?;
        let llm = self.with_backend(&hosted, |b| (self.chat_model)(&b.prepared()).map(|llm| (llm.name().to_string(), llm.context_len())))?;
        let modele = llm.as_ref().map(|(nom, _)| nom.clone()).map_err(Clone::clone);
        let chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
        let (fil, en_cours, dernier) = match chats.get(backend_id) {
            Some(e) => (serde_json::to_value(&e.session).map_err(|e| e.to_string())?["turns"].clone(), e.busy.clone(), e.last_call),
            None => (json!([]), None, None),
        };
        // **La jauge du contexte** (forme accordée avec archi et everything
        // declarative) : `used`, les jetons d'entrée (cache compris) du DERNIER
        // appel au modèle, pas le cumul ; `max`, le contexte du modèle déclaré
        // (`Llm::context_len`, `models.llm.context_tokens`), null s'il est
        // inconnu ; `at`, l'heure de l'appel mesuré. Pas de ratio : la page le
        // calcule.
        let contexte = json!({
            "used": dernier.map(|c| c.prompt_tokens),
            "max": llm.as_ref().ok().map(|(_, n)| *n).filter(|n| *n > 0),
            "source": "run",
            "at": dernier.map(|c| c.at_ms),
        });
        let fil: Vec<Value> = fil.as_array().cloned().unwrap_or_default().into_iter().filter(|t| t["role"] != "system").collect();
        drop(chats);
        let (ecrites, pourquoi) = match self.memory_writes(&hosted, backend_id) {
            Ok((v, borne)) => (v, borne),
            Err(e) => (Vec::new(), Some(e)),
        };
        let (usage, taches_a_role) = self.usage_et_taches_a_role(backend_id);
        Ok(json!({
            "on": modele.is_ok(),
            "model": modele.as_ref().ok(),
            "context": contexte,
            "usage": usage,
            "role_runs": taches_a_role,
            "memory_writes": ecrites,
            "memory_writes_why": pourquoi,
            "why": modele.as_ref().err(),
            "run": en_cours,
            "turns": fil,
            "orphans": self.orphan_chats.lock().unwrap_or_else(|p| p.into_inner()).clone(),
        }))
    }

    /// **L'usage du modèle et les tâches à rôle, lus au journal des runs** (la barre
    /// latérale, liste de Lucie, 8 novembre 2026). L'usage : celui du dernier tour
    /// (`tour`, son run dans `tour_run`) et celui de la conversation (`session`, la
    /// somme de ses tours, `tours` leur nombre), par unité du compteur
    /// (`input_tokens`, `cached_input_tokens`, `output_tokens`,
    /// `reasoning_tokens`), lus dans les `Consumed` des runs du chat (`chat` = ce
    /// dépôt) ; au journal, il survit à un redémarrage de l'hôte, et la session est
    /// celle que le journal garde. Les tâches à rôle : les douze derniers runs de
    /// graphe dont le `RunStarted` porte un `target.role`, finis (le journal ne
    /// garde que les runs finis ; la page ajoute ceux en cours par le flux).
    /// Les noms trient avant toute lecture : `agent-…` pour le chat, `graph-…`
    /// pour les outils et les réactions.
    fn usage_et_taches_a_role(&self, backend_id: &str) -> (Value, Vec<Value>) {
        const RUNS_DU_CHAT: usize = 200;
        const GRAPHES_LUS: usize = 40;
        const TACHES: usize = 12;
        let runs = self.journal.runs_de(backend_id);
        let mut session = serde_json::Map::new();
        let (mut tour, mut tour_run, mut tours) = (Value::Null, Value::Null, 0u64);
        for r in runs.iter().rev().filter(|r| r.starts_with("agent-")).take(RUNS_DU_CHAT) {
            let Some(u) = self.resume_du_run(backend_id, r, |v| (v["chat"] == backend_id).then(|| Value::Object(unites_consommees(v)))) else { continue };
            if tour.is_null() {
                tour = u.clone();
                tour_run = json!(r);
            }
            for (nom, n) in u.as_object().into_iter().flatten() {
                let avant = session.get(nom).and_then(Value::as_u64).unwrap_or(0);
                session.insert(nom.clone(), json!(avant + n.as_u64().unwrap_or(0)));
            }
            tours += 1;
        }
        let mut taches = Vec::new();
        for r in runs.iter().rev().filter(|r| r.starts_with("graph-")).take(GRAPHES_LUS) {
            if taches.len() >= TACHES {
                break;
            }
            if let Some(t) = self.resume_du_run(backend_id, r, |v| tache_a_role(r, v)) {
                taches.push(t);
            }
        }
        // `lus_max` : la borne des tours lus ; la page dit « récents », la
        // conversation n'ayant ni début ni remise à zéro enregistrés.
        (json!({ "tour": tour, "tour_run": tour_run, "session": session, "tours": tours, "lus_max": RUNS_DU_CHAT }), taches)
    }

    /// Le résumé d'un run du journal, lu une fois : un run fini ne change plus.
    /// Un run illisible n'est pas retenu (il le sera au rendu suivant).
    fn resume_du_run(&self, backend_id: &str, run: &str, resumer: impl FnOnce(&Value) -> Option<Value>) -> Option<Value> {
        const RETENUS_MAX: usize = 8192;
        let cle = (backend_id.to_string(), run.to_string());
        if let Some(r) = self.resumes_des_runs.lock().unwrap_or_else(|p| p.into_inner()).get(&cle) {
            return r.clone();
        }
        let v = self.journal.lire(backend_id, run)?;
        let r = resumer(&v);
        let mut m = self.resumes_des_runs.lock().unwrap_or_else(|p| p.into_inner());
        if m.len() >= RETENUS_MAX {
            m.clear();
        }
        m.insert(cle, r.clone());
        r
    }

    /// **Les fiches écrites pendant une conversation** (forme accordée avec
    /// everything declarative et 96) : les Sources de la mémoire déclarée
    /// (`bilans.memoire`) dont `session` est un run de la conversation (le juge
    /// les date), puis les Apports qui citent ces sources — par les outils de
    /// lecture de la mémoire (`select_sources`, `select_apports`), jamais en
    /// lisant son catalogue par-dessous. Les écritures de l'agent lui-même
    /// n'ont pas encore de `session` (M7 de la mémoire longue). `Err` dit
    /// pourquoi la liste ne peut pas se faire ; le second membre dit qu'elle
    /// est coupée à sa borne (une lecture plafonnée à `MEMORY_WRITES_MAX` lignes,
    /// garde de 96 : avec un filtre, `select_*` rend tout).
    fn memory_writes(&self, hosted: &Arc<super::Hosted>, backend_id: &str) -> Result<(Vec<Value>, Option<String>), String> {
        const MEMORY_WRITES_MAX: usize = 500;
        let Some(memoire) = self.memoire_declaree(hosted)? else {
            return Err("ce dépôt ne déclare pas de mémoire (bilans.memoire)".into());
        };
        // **La page ne fait pas ouvrir la mémoire** (relecture d'everything
        // declarative) : elle n'est lue que si l'hôte la tient déjà ouverte ; sinon,
        // la liste dit pourquoi au lieu d'ouvrir une base entière pour un rendu.
        let ouverte = std::fs::canonicalize(&memoire)
            .ok()
            .and_then(|m| self.hosted_existing(&m))
            .filter(|h| h.backend.try_read().is_ok_and(|s| s.is_some()));
        let Some(memoire) = ouverte else {
            return Err("la mémoire n'est pas ouverte dans l'hôte : la liste viendra quand elle le sera".into());
        };
        // Les runs de la conversation : ceux du journal qui portent `chat`, parmi
        // les vingt derniers runs du dépôt (un fichier relu chacun, à chaque rendu).
        let runs: Vec<String> = self
            .journal
            .runs_de(backend_id)
            .into_iter()
            .rev()
            .take(20)
            .filter(|r| self.journal.lire(backend_id, r).is_some_and(|v| v["chat"] == backend_id))
            .collect();
        if runs.is_empty() {
            return Ok((Vec::new(), None));
        }
        let declare = self.with_backend(&memoire, |b| {
            let t = &b.prepared().manifest.tools;
            t.contains_key("select_sources") && t.contains_key("select_apports")
        })?;
        if !declare {
            return Err("la mémoire ne déclare pas select_sources / select_apports (son gabarit est plus ancien) : la liste viendra quand elle sera à jour".into());
        }
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { self.with_backend(&memoire, |b| b.call(outil, args)).and_then(|r| r) };
        let lignes = |r: &Value| -> Vec<Value> {
            let a = match &r["result"] {
                Value::Array(a) => a.clone(),
                Value::Object(o) => o.get("results").and_then(Value::as_array).cloned().unwrap_or_default(),
                _ => Vec::new(),
            };
            a.into_iter().map(|l| if l["data"].is_object() { l["data"].clone() } else { l }).collect()
        };
        let sources = lignes(&appeler("select_sources", json!({ "filter": { "field": { "key": "session", "value": runs } }, "limit": MEMORY_WRITES_MAX }))?);
        let run_de: HashMap<String, String> = sources
            .iter()
            .filter_map(|s| Some((s["chemin"].as_str()?.to_string(), s["session"].as_str()?.to_string())))
            .collect();
        if run_de.is_empty() {
            return Ok((Vec::new(), None));
        }
        let chemins: Vec<&String> = run_de.keys().collect();
        let apports = lignes(&appeler("select_apports", json!({ "filter": { "field": { "key": "source", "value": chemins } }, "limit": MEMORY_WRITES_MAX }))?);
        let borne = (sources.len() >= MEMORY_WRITES_MAX || apports.len() >= MEMORY_WRITES_MAX)
            .then(|| format!("liste coupée à {MEMORY_WRITES_MAX} lignes lues : la conversation en a écrit davantage"));
        let mut ecrites: Vec<Value> = apports
            .into_iter()
            .filter_map(|a| {
                let run = run_de.get(a["source"].as_str()?)?;
                Some(json!({ "fiche": a["fiche"], "kind": "apport", "section": a["section"], "at_ms": a["created_at"], "run": run }))
            })
            .collect();
        ecrites.sort_by_key(|w| std::cmp::Reverse(w["at_ms"].as_u64().unwrap_or(0)));
        Ok((ecrites, borne))
    }

    /// Le run du tour de chat en cours sur un backend, s'il y en a un.
    pub(super) fn run_du_chat_en_cours(&self, backend_id: &str) -> Option<String> {
        self.chats.lock().unwrap_or_else(|p| p.into_inner()).get(backend_id).and_then(|e| e.busy.clone())
    }

    /// **Un tour de chat** sur le backend `backend_id`. Rend le texte, le run,
    /// qui l'a arrêté s'il l'a été, et ce qu'il a consommé.
    pub fn chat_turn(&self, backend_id: &str, message: &str) -> Result<Value, String> {
        self.chat_turn_with(backend_id, message, None)
    }

    /// Le même tour, **comme un pas d'équipier** quand `step` est donné (page
    /// 07) : le message système de l'équipier s'ajoute, ses outils (`edit_plan`,
    /// `step_done`, `report`) et son budget enveloppent ceux du chat, `step_done`
    /// termine le tour, et le journal du run dit la demande (la fiche du juge).
    pub fn chat_turn_with(&self, backend_id: &str, message: &str, step: Option<&super::teammate::Step>) -> Result<Value, String> {
        let hosted = self.hote_par_id(backend_id)?;
        let run_id = crate::events::new_run_id("agent");
        // L'arrêt du tour : lever son drapeau et dire qui l'a demandé.
        let annule = Arc::new(AtomicBool::new(false));
        let arrete_par = Arc::new(Mutex::new(None::<String>));
        let arreter: crate::dataflow::runs_in_flight::Stop = {
            let (annule, arrete_par) = (annule.clone(), arrete_par.clone());
            Arc::new(move |by: &str| {
                arrete_par.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(|| by.to_string());
                annule.store(true, Ordering::SeqCst);
            })
        };
        let (mut session, _en_vol) = {
            let mut chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
            let e = chats.entry(backend_id.to_string()).or_insert_with(|| ChatEntry {
                manifest: hosted.manifest.clone(),
                session: ChatSession::default(),
                busy: None,
                last_call: None,
            });
            if let Some(run) = &e.busy {
                return Err(format!("un tour est déjà en cours sur ce backend (run {run}) : attendez sa fin, ou arrêtez-le"));
            }
            // **En vol avant d'être vu** : le run que `chat_state` rend est
            // toujours arrêtable. Inscrit après, un arrêt arrivé entre les deux
            // était refusé (« aucun run en vol ») pour un tour qui allait
            // partir — trouvé par la seconde passe du 25 octobre 2026.
            // Le pas d'un équipier part même si le backend ne s'ouvre pas : son
            // arrêt passe alors par le drapeau du tour seul (le stop de la boîte).
            let en_vol = match self.with_backend(&hosted, |b| b.runs_in_flight_registry().register_stop(&run_id, arreter)) {
                Ok(v) => Some(v),
                Err(_) if step.is_some() => None,
                Err(e) => return Err(e),
            };
            e.busy = Some(run_id.clone());
            (std::mem::take(&mut e.session), en_vol)
        };
        let resultat = match step {
            None => self
                .with_backend(&hosted, |b| {
                    let declares = crate::declare::DeclarationTools { backend: b, inner: &crate::backend_tools::BackendTools { backend: b } };
                    let dans_une_tache = b.prepared().surcharges().workspace.is_some();
                    let outils = OutilsDuChat::de(b, &declares, dans_une_tache);
                    let tour = Tour { prepared: b.prepared(), mode: b.mode_declarations(), dans_une_tache, bus: Some(b.bus_de_publication()), boite: &outils, ecrivent: &outils.ecrivent, repli: None };
                    self.run_chat_turn(tour, backend_id, &hosted.manifest, &run_id, &mut session, message, annule, arrete_par, None)
                })
                .and_then(|r| r),
            // **Un pas repart d'une session neuve** : son message porte la demande,
            // les pas faits et la boîte (`frontier_message`). La session de la
            // tâche garde chaque pas pour l'affichage ; elle n'est plus renvoyée
            // au modèle.
            Some(s) => {
                let mut neuve = ChatSession::default();
                let r = self.run_teammate_turn(&hosted, backend_id, &run_id, &mut neuve, message, annule, arrete_par, s);
                session.turns.extend(neuve.turns.into_iter().filter(|t| t.role != "system"));
                r
            }
        };
        // Un tour fini : son bilan, si le backend déclare `bilans` (page 04).
        if resultat.is_ok() {
            self.demander_un_bilan(&hosted, &run_id);
        }
        {
            let mut chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(e) = chats.get_mut(backend_id) {
                e.session = session;
                e.busy = None;
            }
        }
        self.write_chat(backend_id)?;
        resultat
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn run_chat_turn(
        &self,
        tour: Tour<'_>,
        backend_id: &str,
        manifest: &Path,
        run_id: &str,
        session: &mut ChatSession,
        message: &str,
        annule: Arc<AtomicBool>,
        arrete_par: Arc<Mutex<Option<String>>>,
        step: Option<&super::teammate::Step>,
    ) -> Result<Value, String> {
        let llm = (self.chat_model)(&tour.prepared).map_err(|why| format!("le chat est éteint : {why}"))?;
        let dans_une_tache = tour.dans_une_tache;
        let mut prompt = system_prompt(tour.mode, dans_une_tache);
        // **La consigne déclarée du chat** (`chat_instructions`) : celle de l'atelier dit
        // l'ordre de ses outils.
        if let Some(c) = tour.prepared.manifest.chat_instructions.as_deref().filter(|c| !c.trim().is_empty()) {
            prompt.push_str("\n\n");
            prompt.push_str(c.trim());
        }
        if let Some(s) = step {
            prompt.push_str("\n\n");
            prompt.push_str(s.system_prompt());
        }
        // Le message système suit le mode de la déclaration : il est réécrit à
        // chaque tour (une tâche fusionnée, un mode changé).
        match session.turns.first_mut() {
            Some(t) if t.role == "system" => *t = crate::llm::Turn::system(&prompt),
            _ => session.turns.insert(0, crate::llm::Turn::system(&prompt)),
        }
        let state_dir = self.chats_root.join(backend_id);
        std::fs::create_dir_all(&state_dir).map_err(|e| format!("{} : {e}", state_dir.display()))?;
        let config: ChatConfig = serde_json::from_value(json!({
            "name": format!("chat:{backend_id}"),
            "system_prompt": prompt,
            "state_dir": state_dir,
        }))
        .map_err(|e| e.to_string())?;
        let bus = tour.bus.clone();
        // Les jetons du tour, sur `tokens` : le flux de l'IDE les relaie.
        let events: EventHandler = {
            let (bus, run, rang) = (bus.clone(), run_id.to_string(), Arc::new(AtomicU64::new(0)));
            Arc::new(move |ev: Value| {
                let Some(bus) = &bus else { return };
                if ev["event"] == "token" {
                    if let Some(text) = ev["text"].as_str() {
                        bus.emit(crate::events::Event::Token {
                            run: run.clone(),
                            node: "chat".to_string(),
                            seq: rang.fetch_add(1, Ordering::SeqCst),
                            text: text.to_string(),
                            count: 1,
                        });
                    }
                }
            })
        };
        let du_pas = step.map(|s| super::teammate::StepTools { inner: tour.boite, step: s });
        let boite: &(dyn crate::agent::ToolBox + Sync) = match &du_pas {
            Some(p) => p,
            None => tour.boite,
        };
        let avant = session.turns.len();
        let fait = session.run_with(
            message,
            llm.as_ref(),
            boite,
            &config,
            events,
            annule,
            ChatRunOptions {
                bus,
                run_id: Some(run_id.to_string()),
                meter: Some(self.chat_meter.clone()),
                stopped_by: Some(arrete_par.clone()),
                terminal_tools: step.map(|_| super::teammate::Step::terminal_tools()).unwrap_or_default(),
            },
        );
        let consomme: Vec<Value> = self
            .chat_meter
            .totals_for_run(run_id)
            .into_iter()
            .map(|((slug, unite), n)| json!({ "slug": slug, "unit": unite.as_str(), "amount": n }))
            .collect();
        let arrete = arrete_par.lock().unwrap_or_else(|p| p.into_inner()).clone();
        let du_tour = session.turns.get(avant..).unwrap_or_default();
        let propositions = propositions_du_tour(du_tour);
        let fichiers = fichiers_ecrits_du_tour(du_tour, tour.ecrivent);
        let tache = if dans_une_tache { self.tache_de(manifest) } else { Value::Null };
        // Le journal du run : les tours de ce tour, tels que le modèle les a
        // reçus, la tâche, ce qu'il a écrit et proposé.
        {
            let mut ajouts = serde_json::Map::new();
            ajouts.insert("chat".into(), json!(backend_id));
            ajouts.insert("tours".into(), serde_json::to_value(du_tour).unwrap_or(Value::Null));
            ajouts.insert("tache".into(), tache.clone());
            ajouts.insert("fichiers".into(), json!(fichiers));
            ajouts.insert("propositions".into(), json!(propositions));
            if let Some(s) = step {
                ajouts.insert("demande".into(), json!(s.request));
                ajouts.insert("pas".into(), json!(s.number));
                // Par où l'équipier s'est réveillé (bus, fichier de boîte), et
                // les replis de ce pas sur les fichiers.
                if let Some(r) = &s.woke_by {
                    ajouts.insert("reveil".into(), json!(r));
                }
            }
            let repli = tour.repli();
            if !repli.is_empty() {
                ajouts.insert("repli".into(), json!(repli));
            }
            self.journal.ajouter(backend_id, run_id, ajouts);
        }
        let reponse = |text: &str| {
            json!({ "run": run_id, "text": text, "stopped_by": arrete, "consumed": consomme, "propositions": propositions, "tache": tache, "fichiers": fichiers })
        };
        match fait {
            Ok(run) => {
                if let Some(e) = self.chats.lock().unwrap_or_else(|p| p.into_inner()).get_mut(backend_id) {
                    e.last_call = run.last_call;
                }
                Ok(reponse(&run.text))
            }
            // Un tour arrêté n'est pas une panne : il dit qui l'a arrêté.
            Err(_) if arrete.is_some() => Ok(reponse("")),
            Err(e) => Err(e),
        }
    }
}

/// **Ce qu'un tour prend du backend** : le manifeste préparé, le mode des
/// déclarations, son bus s'il est ouvert, la boîte d'outils. Le chat le construit
/// sous le backend ouvert ; le pas d'un équipier, hors de lui (il prend le backend
/// à chaque appel d'outil, `super::teammate_tools::Etanche`).
pub(super) struct Tour<'a> {
    pub prepared: Arc<crate::backend::PreparedBackend>,
    pub mode: ModeDeclarations,
    pub dans_une_tache: bool,
    pub bus: Option<crate::events::EventBus>,
    pub boite: &'a (dyn crate::agent::ToolBox + Sync),
    pub ecrivent: &'a HashSet<String>,
    /// Les replis du tour, que la boîte étanche note (aucun pour le chat).
    pub repli: Option<&'a Mutex<Vec<String>>>,
}

impl Tour<'_> {
    fn repli(&self) -> Vec<String> {
        self.repli.map(|r| r.lock().unwrap_or_else(|p| p.into_inner()).clone()).unwrap_or_default()
    }
}

/// **Les outils du chat** : ceux du backend et des déclarations, sauf ce que
/// le chat ne fait pas (page 03 du 25 octobre 2026, c7). Hors d'une tâche, un
/// outil qui écrit les fichiers n'est pas proposé au modèle, et un appel par son
/// nom est refusé en disant d'ouvrir une tâche — quel que soit le manifeste
/// (`task_writes` ne vise que l'agent d'un workspace qui le dit). Un outil qui
/// lance des commandes n'est proposé nulle part : coder, ici, c'est lire et
/// éditer.
pub(crate) struct OutilsDuChat<'a> {
    inner: &'a (dyn crate::agent::ToolBox + Sync),
    /// Les outils qui écrivent les fichiers (proposés dans une tâche).
    pub(crate) ecrivent: HashSet<String>,
    /// Ceux qui ne sont pas proposés, et pourquoi.
    retires: HashMap<String, String>,
}

impl<'a> OutilsDuChat<'a> {
    pub(crate) fn de(b: &Backend, inner: &'a (dyn crate::agent::ToolBox + Sync), dans_une_tache: bool) -> Self {
        let prepared = b.prepared();
        let mut ecrivent = HashSet::new();
        let mut retires = HashMap::new();
        // Dans une tâche, les commandes passent par la porte déclarée du workspace
        // (lot 2 de la page 07-novembre-2026-everything-declarative/01) ; hors
        // d'une tâche, ou sans porte, le chat n'en lance pas.
        let porte = prepared
            .manifest
            .workspace
            .as_ref()
            .is_some_and(|w| w.commands != crate::backend_code::CommandGate::Off && (dans_une_tache || w.commands_outside_tasks));
        for (nom, outil) in &prepared.manifest.tools {
            if outil.policy.run_commands && !porte {
                retires.insert(nom.clone(), format!("« {nom} » lance des commandes, et le chat n'en lance pas : il lit et édite le code"));
            } else if outil.policy.write_files {
                ecrivent.insert(nom.clone());
                if !dans_une_tache {
                    retires.insert(
                        nom.clone(),
                        format!(
                            "« {nom} » écrit les fichiers, et le chat n'écrit jamais hors d'une tâche : demandez à la personne \
                             d'ouvrir une tâche dans l'IDE (sa branche, relue puis fusionnée), et éditez-y"
                        ),
                    );
                }
            }
        }
        Self { inner, ecrivent, retires }
    }

    fn refus(&self, call: &crate::llm::ToolCall) -> Option<crate::llm::Turn> {
        self.retires
            .get(&call.name)
            .map(|pourquoi| crate::llm::Turn::tool_result(&call.id, &call.name, json!({ "ok": false, "error": pourquoi }).to_string()))
    }
}

impl crate::agent::ToolBox for OutilsDuChat<'_> {
    fn call(&self, call: &crate::llm::ToolCall) -> crate::llm::Turn {
        self.refus(call).unwrap_or_else(|| self.inner.call(call))
    }
    fn call_in(&self, call: &crate::llm::ToolCall, run: &str) -> crate::llm::Turn {
        self.refus(call).unwrap_or_else(|| self.inner.call_in(call, run))
    }
    fn call_with(&self, call: &crate::llm::ToolCall, run: &str, inv: &crate::agent::ToolInvocation) -> crate::llm::Turn {
        self.refus(call).unwrap_or_else(|| self.inner.call_with(call, run, inv))
    }
    fn tool_defs(&self) -> Vec<crate::tools::ToolDef> {
        self.inner.tool_defs().into_iter().filter(|d| !self.retires.contains_key(&d.name)).collect()
    }
    fn is_async(&self, tool: &str) -> bool {
        self.inner.is_async(tool)
    }
}

/// **Les fichiers qu'un tour a écrits** : les `path` des appels à un outil qui
/// écrit, dont le résultat n'est pas un refus, dans l'ordre et sans doublon.
fn fichiers_ecrits_du_tour(turns: &[crate::llm::Turn], ecrivent: &HashSet<String>) -> Vec<String> {
    let reussi = |id: &str| {
        turns.iter().find(|t| t.role == "tool" && t.tool_call_id.as_deref() == Some(id)).is_some_and(|t| {
            match serde_json::from_str::<Value>(&t.content) {
                Ok(Value::Object(o)) => !o.contains_key("error") && o.get("ok") != Some(&json!(false)),
                _ => true,
            }
        })
    };
    let mut fichiers: Vec<String> = Vec::new();
    for appel in turns.iter().flat_map(|t| t.tool_calls.iter()) {
        if !ecrivent.contains(&appel.name) || !reussi(&appel.id) {
            continue;
        }
        let args: Value = serde_json::from_str(&appel.arguments).unwrap_or(Value::Null);
        if let Some(p) = args["path"].as_str() {
            if !fichiers.iter().any(|f| f == p) {
                fichiers.push(p.to_string());
            }
        }
    }
    fichiers
}

/// **Les propositions qu'un tour a faites**, lues dans ses résultats de
/// `propose`, sous la forme de `PropositionChanged` (id, chemins, acceptée).
/// La réponse du tour les porte : le flux les dit aussi, mais sur
/// `declarations`, relayé à part d'`agent` — une page peut voir le tour fini
/// avant elles.
fn propositions_du_tour(turns: &[crate::llm::Turn]) -> Vec<Value> {
    turns
        .iter()
        .filter(|t| t.role == "tool" && t.tool_name.as_deref() == Some(crate::declare::PROPOSE))
        .filter_map(|t| serde_json::from_str::<Value>(&t.content).ok())
        .filter(|r| r["ok"] == true)
        .map(|r| {
            let p = &r["proposal"];
            json!({
                "id": p["id"],
                "paths": p["fichiers"].as_array().into_iter().flatten().map(|f| f["path"].clone()).collect::<Vec<_>>(),
                "accepted": p["refus"].is_null(),
            })
        })
        .collect()
}

/// Le chemin d'une session sur le disque, pour les témoins.
#[cfg(all(test, feature = "rag3db-native", feature = "code"))]
fn chat_file(root: &std::path::Path, backend_id: &str) -> PathBuf {
    root.join(format!("{backend_id}.json"))
}

/// Les jetons d'un run, par unité du compteur, sommés sur ses `Consumed`.
fn unites_consommees(v: &Value) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    for e in v["evenements"].as_array().into_iter().flatten().filter(|e| e["kind"] == "Consumed") {
        for u in e["units"].as_array().into_iter().flatten() {
            if let (Some(nom), Some(n)) = (u["unit"].as_str(), u["amount"].as_u64()) {
                let avant = m.get(nom).and_then(Value::as_u64).unwrap_or(0);
                m.insert(nom.to_string(), json!(avant + n));
            }
        }
    }
    m
}

/// Une tâche à rôle, si le `RunStarted` du run en porte un : son état est
/// « fini », « échec », ou « inconnu » quand aucune fin ne se lit.
fn tache_a_role(run: &str, v: &Value) -> Option<Value> {
    let debut = v["evenements"].as_array().and_then(|e| e.iter().find(|e| e["kind"] == "RunStarted")).cloned().unwrap_or(Value::Null);
    let role = debut["target"]["role"].as_str()?;
    let fini = v["fin"]["ok"].as_bool().or_else(|| v["evenements"].as_array().and_then(|e| e.iter().rev().find(|e| e["kind"] == "RunFinished")).and_then(|e| e["ok"].as_bool()));
    // L'heure d'un run : son nom porte l'instant de son départ, en
    // millisecondes et en hexadécimal (`graph-<ms>-<n>`).
    let heure = run.split('-').nth(1).and_then(|h| u64::from_str_radix(h, 16).ok());
    Some(json!({
        "role": role,
        "run": run,
        "tool": debut["target"]["tool"],
        "reaction": debut["target"]["reaction"],
        "state": match fini { Some(true) => "fini", Some(false) => "échec", None => "inconnu" },
        "at_ms": heure,
    }))
}

#[cfg(all(test, feature = "rag3db-native", feature = "code"))]
mod tests;
