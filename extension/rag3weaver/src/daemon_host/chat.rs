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
pub type ChatModel = Arc<dyn Fn(&Backend) -> Result<Arc<dyn crate::llm::Llm>, String> + Send + Sync>;

/// La session d'un backend, et si un tour est en cours.
pub(super) struct ChatEntry {
    manifest: PathBuf,
    session: ChatSession,
    busy: Option<String>,
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
    Arc::new(|backend: &Backend| -> Result<Arc<dyn crate::llm::Llm>, String> {
        #[cfg(feature = "openai-llm")]
        {
            if let Some(source) = backend.prepared().manifest.models.get(&crate::model_source::Capability::Llm) {
                return crate::model_source::connect_llm(source).map(|(llm, _)| Arc::new(llm) as Arc<dyn crate::llm::Llm>);
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
            let _ = backend;
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
                            .insert(h.id.clone(), ChatEntry { manifest: f.manifest, session: f.session, busy: None });
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

    /// **Ce que la page montre d'un chat** : allumé ou non (et pourquoi), le
    /// run en cours, le fil de la session (sans le message système), et les
    /// sessions orphelines.
    pub fn chat_state(&self, backend_id: &str) -> Result<Value, String> {
        let hosted = self.hote_par_id(backend_id)?;
        let modele = self.with_backend(&hosted, |b| (self.chat_model)(b).map(|llm| llm.name().to_string()))?;
        let chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
        let (fil, en_cours) = match chats.get(backend_id) {
            Some(e) => (serde_json::to_value(&e.session).map_err(|e| e.to_string())?["turns"].clone(), e.busy.clone()),
            None => (json!([]), None),
        };
        let fil: Vec<Value> = fil.as_array().cloned().unwrap_or_default().into_iter().filter(|t| t["role"] != "system").collect();
        Ok(json!({
            "on": modele.is_ok(),
            "model": modele.as_ref().ok(),
            "why": modele.as_ref().err(),
            "run": en_cours,
            "turns": fil,
            "orphans": self.orphan_chats.lock().unwrap_or_else(|p| p.into_inner()).clone(),
        }))
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
            });
            if let Some(run) = &e.busy {
                return Err(format!("un tour est déjà en cours sur ce backend (run {run}) : attendez sa fin, ou arrêtez-le"));
            }
            // **En vol avant d'être vu** : le run que `chat_state` rend est
            // toujours arrêtable. Inscrit après, un arrêt arrivé entre les deux
            // était refusé (« aucun run en vol ») pour un tour qui allait
            // partir — trouvé par la seconde passe du 25 octobre 2026.
            let en_vol = self.with_backend(&hosted, |b| b.runs_in_flight_registry().register_stop(&run_id, arreter))?;
            e.busy = Some(run_id.clone());
            (std::mem::take(&mut e.session), en_vol)
        };
        let resultat = self
            .with_backend(&hosted, |b| self.run_chat_turn(b, backend_id, &hosted.manifest, &run_id, &mut session, message, annule, arrete_par, step))
            .and_then(|r| r);
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
    fn run_chat_turn(
        &self,
        b: &Backend,
        backend_id: &str,
        manifest: &Path,
        run_id: &str,
        session: &mut ChatSession,
        message: &str,
        annule: Arc<AtomicBool>,
        arrete_par: Arc<Mutex<Option<String>>>,
        step: Option<&super::teammate::Step>,
    ) -> Result<Value, String> {
        let llm = (self.chat_model)(b).map_err(|why| format!("le chat est éteint : {why}"))?;
        // Une tâche : le backend porte sa surcharge de workspace (son worktree).
        let dans_une_tache = b.prepared().surcharges().workspace.is_some();
        let mut prompt = system_prompt(b.mode_declarations(), dans_une_tache);
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
        let bus = b.bus_de_publication();
        // Les jetons du tour, sur `tokens` : le flux de l'IDE les relaie.
        let events: EventHandler = {
            let (bus, run, rang) = (bus.clone(), run_id.to_string(), Arc::new(AtomicU64::new(0)));
            Arc::new(move |ev: Value| {
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
        let declares = crate::declare::DeclarationTools { backend: b, inner: &crate::backend_tools::BackendTools { backend: b } };
        let outils = OutilsDuChat::de(b, &declares, dans_une_tache);
        let du_pas = step.map(|s| super::teammate::StepTools { inner: &outils, step: s });
        let boite: &(dyn crate::agent::ToolBox + Sync) = match &du_pas {
            Some(p) => p,
            None => &outils,
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
                bus: Some(bus),
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
        let fichiers = fichiers_ecrits_du_tour(du_tour, &outils.ecrivent);
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
            }
            self.journal.ajouter(backend_id, run_id, ajouts);
        }
        let reponse = |text: &str| {
            json!({ "run": run_id, "text": text, "stopped_by": arrete, "consumed": consomme, "propositions": propositions, "tache": tache, "fichiers": fichiers })
        };
        match fait {
            Ok(run) => Ok(reponse(&run.text)),
            // Un tour arrêté n'est pas une panne : il dit qui l'a arrêté.
            Err(_) if arrete.is_some() => Ok(reponse("")),
            Err(e) => Err(e),
        }
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
        for (nom, outil) in &prepared.manifest.tools {
            if outil.policy.run_commands {
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

#[cfg(all(test, feature = "rag3db-native", feature = "code"))]
mod tests;
