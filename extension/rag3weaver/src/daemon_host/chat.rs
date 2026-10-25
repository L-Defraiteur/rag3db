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
use std::path::PathBuf;
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
/// déclaration (c7) — deux textes divergeraient à la première retouche.
pub fn system_prompt(mode: ModeDeclarations) -> String {
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
    format!(
        "You work on a declared backend: its manifest, tool graphs (mermaid), JSON schemas, scripted nodes, \
         React components (tsx, with an optional click graph in mermaid) and views are files you can list and \
         read with list_declarations and read_declaration. {changer} Nothing is compiled by hand. Use the \
         backend's own tools to look at its data. Answer briefly, in the language of the person."
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
        let hosted = self.hote_par_id(backend_id)?;
        let run_id = crate::events::new_run_id("agent");
        let mut session = {
            let mut chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
            let e = chats.entry(backend_id.to_string()).or_insert_with(|| ChatEntry {
                manifest: hosted.manifest.clone(),
                session: ChatSession::default(),
                busy: None,
            });
            if let Some(run) = &e.busy {
                return Err(format!("un tour est déjà en cours sur ce backend (run {run}) : attendez sa fin, ou arrêtez-le"));
            }
            e.busy = Some(run_id.clone());
            std::mem::take(&mut e.session)
        };
        let resultat = self
            .with_backend(&hosted, |b| self.run_chat_turn(b, backend_id, &run_id, &mut session, message))
            .and_then(|r| r);
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

    fn run_chat_turn(&self, b: &Backend, backend_id: &str, run_id: &str, session: &mut ChatSession, message: &str) -> Result<Value, String> {
        let llm = (self.chat_model)(b).map_err(|why| format!("le chat est éteint : {why}"))?;
        let prompt = system_prompt(b.mode_declarations());
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
        // En vol (C4) : arrêter le tour, c'est lever son drapeau et dire qui.
        let annule = Arc::new(AtomicBool::new(false));
        let arrete_par = Arc::new(Mutex::new(None::<String>));
        let _en_vol = b.runs_in_flight_registry().register_stop(run_id, {
            let (annule, arrete_par) = (annule.clone(), arrete_par.clone());
            Arc::new(move |by: &str| {
                arrete_par.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(|| by.to_string());
                annule.store(true, Ordering::SeqCst);
            })
        });
        let outils = crate::declare::DeclarationTools { backend: b, inner: &crate::backend_tools::BackendTools { backend: b } };
        let fait = session.run_with(
            message,
            llm.as_ref(),
            &outils,
            &config,
            events,
            annule,
            ChatRunOptions {
                bus: Some(bus),
                run_id: Some(run_id.to_string()),
                meter: Some(self.chat_meter.clone()),
                stopped_by: Some(arrete_par.clone()),
            },
        );
        let consomme: Vec<Value> = self
            .chat_meter
            .totals_for_run(run_id)
            .into_iter()
            .map(|((slug, unite), n)| json!({ "slug": slug, "unit": unite.as_str(), "amount": n }))
            .collect();
        let arrete = arrete_par.lock().unwrap_or_else(|p| p.into_inner()).clone();
        match fait {
            Ok(run) => Ok(json!({ "run": run_id, "text": run.text, "stopped_by": arrete, "consumed": consomme })),
            // Un tour arrêté n'est pas une panne : il dit qui l'a arrêté.
            Err(_) if arrete.is_some() => Ok(json!({ "run": run_id, "text": "", "stopped_by": arrete, "consumed": consomme })),
            Err(e) => Err(e),
        }
    }
}

/// Le chemin d'une session sur le disque, pour les témoins.
#[cfg(all(test, feature = "rag3db-native", feature = "code"))]
fn chat_file(root: &std::path::Path, backend_id: &str) -> PathBuf {
    root.join(format!("{backend_id}.json"))
}

#[cfg(all(test, feature = "rag3db-native", feature = "code"))]
mod tests;
