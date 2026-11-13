//! **Le juge de session** (page 04 du 25 octobre 2026) : un agent déclaré
//! (`templates/agents/juge/`) qui relit un run par son journal, rejoue un outil
//! qui ne fait que lire, ouvre le résultat, se compare aux bilans précédents de
//! la même tâche, puis rend son verdict dans une forme fixe — le dernier pas,
//! pas le juge. L'hôte range le verdict en fiche de bilan dans la mémoire que
//! le backend cible déclare (`bilans.memoire`) et propose les tickets.
//!
//! Le juge tourne sans bus de backend : ses runs ne sont jamais journalisés,
//! donc jamais cibles.

use super::*;
use crate::agent::ToolBox;
use crate::chat::{ChatConfig, ChatRunOptions, ChatSession, EventHandler};
use crate::llm::{GenOptions, ResponseFormat, StringSink, ToolCall, Turn};
use crate::tools::ToolDef;
use std::sync::atomic::AtomicBool;

/// **La file des bilans** : la fin d'un tour y dépose `(backend, run)` quand le
/// backend déclare `bilans` ; un fil de fond tenu par le serveur
/// ([`DaemonHost::servir_les_bilans`]) les juge un par un. Un bilan qu'il faut
/// demander n'existerait pas (c7).
pub struct FileDesBilans {
    /// Fermé (`None`) par l'arrêt de l'hôte : un bilan demandé ensuite ne part plus.
    envoi: Mutex<Option<std::sync::mpsc::Sender<(String, String)>>>,
    reception: Mutex<Option<std::sync::mpsc::Receiver<(String, String)>>>,
    /// **Le fil du juge, et son arrêt** (13 novembre 2026, ticket « le journal des runs
    /// écrit après l'arrêt de son hôte ») : le juge tient l'hôte le temps d'un jugement,
    /// et ses appels à la mémoire écrivaient encore leurs runs après la fin d'un témoin,
    /// dans un dossier déjà parti (le résidu d'e2e_juge). L'arrêt de l'hôte ferme la file,
    /// dit au juge de ne plus juger ce qui reste, et attend son fil.
    fil: Mutex<Option<std::thread::JoinHandle<()>>>,
    arrete: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for FileDesBilans {
    fn default() -> Self {
        let (envoi, reception) = std::sync::mpsc::channel();
        Self {
            envoi: Mutex::new(Some(envoi)),
            reception: Mutex::new(Some(reception)),
            fil: Mutex::new(None),
            arrete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// Le temps qu'un arrêt de l'hôte laisse au jugement en cours.
const ARRET_DU_JUGE: std::time::Duration = std::time::Duration::from_secs(30);

/// Ce qu'un outil du juge rend au plus au modèle.
const BORNE: usize = 24 * 1024;

/// Le genre d'un run déposé par le hook `Stop` de Claude Code (`POST /journal/run`).
pub const TOUR_DE_CLAUDE_CODE: &str = "claude_code_turn";

/// Un tour de Claude Code : ses appels sont dans `tours[].tool_calls`, pas dans
/// des événements du bus.
fn est_un_tour_de_claude_code(journal: &Value) -> bool {
    journal["genre"] == TOUR_DE_CLAUDE_CODE
}

/// **Un appel d'outil d'un run**, quel que soit son genre : un `ToolCallStarted`
/// du bus (le chat, l'équipier), ou un `tool_calls` d'un tour de Claude Code.
#[derive(Debug, Clone, PartialEq)]
struct Appel {
    id: String,
    /// Le nom de l'outil du backend (`search_code`), sans le préfixe MCP.
    outil: String,
    /// Le serveur MCP de Claude Code (`code-rag3db`) ; `None` pour un appel du bus.
    serveur: Option<String>,
    arguments: Value,
    /// Ce que l'agent a reçu, quand le run le garde (le hook le borne à 2 000 caractères).
    recu: Option<Value>,
    /// Un outil de notre hôte : un appel du bus, ou un `mcp__<serveur>__<outil>`.
    /// Les outils propres de Claude Code (Bash, Read, Edit…) n'en sont pas.
    de_l_hote: bool,
}

/// `mcp__<serveur>__<outil>` → (serveur, outil) ; le serveur est la clé de
/// `mcpServers` de Claude Code, la même que dans la table `serveurs` du run.
fn nom_mcp(nom: &str) -> Option<(&str, &str)> {
    nom.strip_prefix("mcp__")?.split_once("__")
}

/// **Les appels d'outil du run**, dans l'ordre.
fn appels_du_run(journal: &Value) -> Vec<Appel> {
    if !est_un_tour_de_claude_code(journal) {
        return journal["evenements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] == "ToolCallStarted")
            .map(|e| Appel {
                id: e["call_id"].as_str().unwrap_or_default().to_string(),
                outil: e["tool"].as_str().unwrap_or_default().to_string(),
                serveur: None,
                arguments: serde_json::from_str(e["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({})),
                recu: None,
                de_l_hote: true,
            })
            .collect();
    }
    let tours = journal["tours"].as_array().cloned().unwrap_or_default();
    let recu = |id: &str| tours.iter().find(|t| t["role"] == "tool" && t["tool_use_id"] == id).map(|t| json!({ "content": t["content"], "is_error": t["is_error"] }));
    tours
        .iter()
        .filter(|t| t["role"] == "assistant")
        .flat_map(|t| t["tool_calls"].as_array().cloned().unwrap_or_default())
        .map(|c| {
            let nom = c["name"].as_str().unwrap_or_default();
            let id = c["id"].as_str().unwrap_or_default().to_string();
            let (serveur, outil, de_l_hote) = match nom_mcp(nom) {
                Some((s, o)) => (Some(s.to_string()), o.to_string(), true),
                None => (None, nom.to_string(), false),
            };
            Appel { recu: recu(&id), id, outil, serveur, arguments: c["input"].clone(), de_l_hote }
        })
        .collect()
}

fn borner(texte: String) -> String {
    if texte.len() <= BORNE {
        return texte;
    }
    let mut coupe = BORNE;
    while !texte.is_char_boundary(coupe) {
        coupe -= 1;
    }
    format!("{}… [{} octets de plus]", &texte[..coupe], texte.len() - coupe)
}

/// **Le nom d'une fiche de bilan**, stable (96) : `bilan : ` puis l'intitulé
/// en minuscules, espaces repliés, ponctuation de bord retirée. Une seule
/// fonction : deux formulations rangées par deux fonctions feraient deux fiches.
/// **Un pas d'équipier se juge sur son but** (8 novembre 2026, essai 3 de c7 : le pas
/// « lire le ticket et le code » noté 2/10, « n'a pu accomplir aucune action
/// corrective », jugé comme la tâche entière ; ses fiches portaient la demande
/// entière, 2 000 caractères, pour nom). Le run d'un pas : sa tâche (`tache.nom`)
/// et son numéro (`pas`), lus au journal.
fn pas_d_equipier(journal: &Value) -> Option<(String, u64)> {
    Some((journal["tache"]["nom"].as_str()?.to_string(), journal["pas"].as_u64()?))
}

/// La date d'un run, lue dans son nom (`agent-<ms en hexadécimal>-<n>`).
fn date_du_run(run: &str) -> Option<String> {
    let ms = u64::from_str_radix(run.split('-').nth(1)?, 16).ok()?;
    let t = jiff::Timestamp::from_millisecond(ms as i64).ok()?;
    Some(t.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d").to_string())
}

/// **La fiche d'une tâche d'équipier** : « bilan : <nom de la tâche> », court et
/// stable. Tous ses pas y vont, un apport par pas (« pas N · <date> » en tête), et ses
/// plans clos aussi ; le hook de Claude Code la lit sous le même nom.
pub fn nom_de_fiche_de_tache(tache: &str) -> String {
    format!("bilan : {tache}")
}

/// **Le nom de la fiche d'un run** : pour un pas d'équipier, la fiche de sa tâche
/// ([`nom_de_fiche_de_tache`]) ; sinon le nom de l'intitulé ([`nom_de_tache`]).
pub fn nom_de_fiche_du_run(_run: &str, journal: &Value, intitule: &str) -> String {
    match pas_d_equipier(journal) {
        Some((tache, _)) => nom_de_fiche_de_tache(&tache),
        None => nom_de_tache(intitule),
    }
}

/// L'en-tête de l'apport d'un pas : « pas N · <date> ».
fn entete_du_pas(run: &str, journal: &Value) -> Option<String> {
    let (_, pas) = pas_d_equipier(journal)?;
    Some(match date_du_run(run) {
        Some(d) => format!("pas {pas} · {d}"),
        None => format!("pas {pas}"),
    })
}

/// **Ce que le juge reçoit pour cible** : un pas d'équipier, avec son message de pas
/// (le plan, l'élément en cours, la boîte) comme but ; un autre run, sa tâche.
pub fn consigne_du_juge(run: &str, journal: &Value, intitule: &str) -> String {
    match pas_d_equipier(journal) {
        Some((tache, pas)) => {
            let message = journal["tours"]
                .as_array()
                .and_then(|t| t.iter().find(|t| t["role"] == "user"))
                .and_then(|t| t["content"].as_str())
                .unwrap_or_default();
            let mut coupe = message.len().min(3000);
            while !message.is_char_boundary(coupe) {
                coupe -= 1;
            }
            let message = if coupe < message.len() { format!("{}… [coupé]", &message[..coupe]) } else { message.to_string() };
            format!(
                "Juge ce run : {run}, le pas {pas} de la tâche « {tache} ». Juge le pas sur **son** but : ce que son message de pas lui demandait (l'élément du plan en cours, la boîte), cité ci-dessous, et non la tâche entière, qui se fait en plusieurs pas. Les outils déclarés sont ceux du journal, pas seulement ceux que le pas a appelés. Enquête, puis dis en une ligne que tu as assez.\n\nLe message du pas :\n{}",
                message
            )
        }
        None => format!("Juge ce run : {run}, sur la tâche « {intitule} ». Enquête, puis dis en une ligne que tu as assez."),
    }
}

pub fn nom_de_tache(intitule: &str) -> String {
    let replie = intitule.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let nu = replie.trim_matches(|c: char| !c.is_alphanumeric());
    format!("bilan : {nu}")
}

/// Le gabarit livré du juge : son `ChatConfig`, le schéma du bilan de session et
/// celui du bilan des outils (page 05).
fn gabarit() -> Result<(ChatConfig, Value, Value), String> {
    let lire = |nom: &str| -> Result<Value, String> {
        let octets = crate::builtin_templates::get(nom).ok_or_else(|| format!("le gabarit livré {nom} manque"))?;
        serde_json::from_slice(octets).map_err(|e| format!("{nom} : {e}"))
    };
    let config: ChatConfig = serde_json::from_value(lire("agents/juge/juge.json")?).map_err(|e| format!("agents/juge/juge.json : {e}"))?;
    Ok((config, lire("agents/juge/bilan.schema.json")?, lire("agents/juge/bilan_des_outils.schema.json")?))
}

/// **Les outils du juge** : sur le run cible, son backend et la mémoire.
struct OutilsDuJuge<'a> {
    host: &'a DaemonHost,
    cible: Arc<Hosted>,
    journal: &'a Value,
    memoire: Arc<Hosted>,
    fiche: &'a str,
}

impl OutilsDuJuge<'_> {
    /// **Le backend d'un appel** : celui du run pour un appel du bus ; pour un
    /// tour de Claude Code, celui que sert son serveur MCP, par la table
    /// `serveurs` du run (`{serveur: manifeste}`) ; un serveur absent de la
    /// table ne se rejoue pas, et c'est dit.
    fn backend_de(&self, appel: &Appel) -> Result<Arc<Hosted>, String> {
        if !appel.de_l_hote {
            return Err(format!("« {} » est un outil propre de Claude Code : le juge ne le rejoue pas", appel.outil));
        }
        let Some(serveur) = &appel.serveur else { return Ok(self.cible.clone()) };
        match self.journal["serveurs"][serveur].as_str() {
            Some(manifeste) => self.host.hosted(std::path::Path::new(manifeste)),
            None => Err(format!("« {} » : non rejoué, serveur « {serveur} » inconnu du run (absent de sa table serveurs)", appel.outil)),
        }
    }

    fn lire_le_run(&self, args: &Value) -> Result<Value, String> {
        let outil = args["outil"].as_str().filter(|s| !s.is_empty());
        let evenements: Vec<&Value> = self.journal["evenements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] != "Token")
            .filter(|e| outil.is_none_or(|o| e["tool"].as_str() == Some(o) || e["tool"].is_null()))
            .collect();
        // **Les déclarations des outils appelés, telles que l'agent les a vues**
        // (page 05) : leur description et leur schéma de paramètres, pris dans
        // les `tool_defs` du backend — sans elles, le juge devinerait un nom
        // trompeur ou un paramètre qui devrait être optionnel.
        let appels: Vec<Appel> = appels_du_run(self.journal).into_iter().filter(|a| outil.is_none_or(|o| a.outil == o)).collect();
        let mut declarations = serde_json::Map::new();
        for a in appels.iter().filter(|a| a.de_l_hote) {
            if declarations.contains_key(&a.outil) {
                continue;
            }
            let Ok(backend) = self.backend_de(a) else { continue };
            let defs = self.host.with_backend(&backend, |b| crate::backend_tools::BackendTools { backend: b }.tool_defs()).unwrap_or_default();
            if let Some(d) = defs.into_iter().find(|d| d.name == a.outil) {
                declarations.insert(d.name, json!({ "description": d.description, "parametres": d.parameters }));
            }
        }
        // Un tour de Claude Code n'a pas d'événements du bus : ses appels de
        // l'hôte se lisent ici, avec ce que Claude a reçu (borné à 2 000
        // caractères par le hook) ; ses outils propres restent dans les tours.
        let appels_de_l_hote: Vec<Value> = appels
            .iter()
            .filter(|a| a.de_l_hote && est_un_tour_de_claude_code(self.journal))
            .map(|a| json!({ "call_id": a.id, "serveur": a.serveur, "outil": a.outil, "arguments": a.arguments, "recu": a.recu }))
            .collect();
        Ok(json!({
            "declarations": declarations,
            "run": self.journal["run"],
            "debut": self.journal["debut"],
            "fin": self.journal["fin"],
            "evenements": evenements,
            "appels_de_l_hote": appels_de_l_hote,
            "tours": self.journal["tours"],
            "appel": self.journal["appel"],
            "tache": self.journal["tache"],
            "fichiers": self.journal["fichiers"],
            "propositions": self.journal["propositions"],
        }))
    }

    /// Rejoue un appel du run, mêmes arguments, même backend — seulement un
    /// outil qui ne fait que lire.
    fn rejouer_un_outil(&self, args: &Value) -> Result<Value, String> {
        let id = args["call_id"].as_str().ok_or("call_id manque")?;
        let appel = appels_du_run(self.journal).into_iter().find(|a| a.id == id).ok_or_else(|| format!("aucun appel {id} dans ce run"))?;
        let backend = self.backend_de(&appel)?;
        let outil = appel.outil.clone();
        let arguments = appel.arguments.clone();
        // Ce que l'agent a reçu, à côté du rejeu ; borné par le hook pour un
        // tour de Claude Code — une différence de longueur n'est pas un défaut.
        let recu = appel.recu.clone();
        let borne = est_un_tour_de_claude_code(self.journal).then_some("le contenu reçu est borné à 2 000 caractères par le hook");
        self.host
            .with_backend(&backend, |b| {
                let prepared = b.prepared();
                match prepared.manifest.tools.get(&outil) {
                    Some(t) if t.policy.write_files || t.policy.run_commands => {
                        Err(format!("« {outil} » écrit ou lance des commandes : le juge ne rejoue que ce qui lit"))
                    }
                    // Un outil des déclarations (propose, declare…) n'est pas au manifeste.
                    None => Err(format!("« {outil} » n'est pas un outil du backend : le juge ne le rejoue pas")),
                    Some(_) => b.call(&outil, arguments.clone()).map(|r| json!({ "outil": outil, "arguments": arguments, "rendu": r["result"], "recu": recu, "borne": borne })),
                }
            })
            .and_then(|r| r)
    }

    fn ouvrir_le_resultat(&self) -> Result<Value, String> {
        let tache = &self.journal["tache"];
        let relecture = match (tache["parent"].as_str(), tache["nom"].as_str()) {
            (Some(parent), Some(nom)) => self.host.relecture(parent, nom).unwrap_or_else(|e| json!({ "erreur": e })),
            _ => Value::Null,
        };
        Ok(json!({
            "tache": tache,
            "relecture": relecture,
            "fichiers": self.journal["fichiers"],
            "propositions": self.journal["propositions"],
            "appel": self.journal["appel"],
        }))
    }

    fn bilans_precedents(&self) -> Result<Value, String> {
        let options = json!({ "limit": 5, "filter_condition": { "field": { "key": "fiche", "value": self.fiche } } });
        self.host
            .with_backend(&self.memoire, |b| b.call("recall_apports", json!({ "query": self.fiche, "options": options })))
            .and_then(|r| r)
            .map(|r| json!({ "fiche": self.fiche, "apports": r["result"] }))
    }
}

impl ToolBox for OutilsDuJuge<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        let args: Value = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
        let rendu = match call.name.as_str() {
            "lire_le_run" => self.lire_le_run(&args),
            "rejouer_un_outil" => self.rejouer_un_outil(&args),
            "ouvrir_le_resultat" => self.ouvrir_le_resultat(),
            "bilans_precedents" => self.bilans_precedents(),
            autre => Err(format!("le juge n'a pas d'outil « {autre} »")),
        };
        let contenu = match rendu {
            Ok(v) => borner(v.to_string()),
            Err(e) => json!({ "ok": false, "error": e }).to_string(),
        };
        Turn::tool_result(&call.id, &call.name, contenu)
    }

    fn tool_defs(&self) -> Vec<ToolDef> {
        let def = |name: &str, description: &str, parameters: Value| ToolDef { name: name.into(), description: description.into(), parameters };
        let rien = json!({ "type": "object", "properties": {}, "additionalProperties": false });
        vec![
            def(
                "lire_le_run",
                "Le run cible : ses appels (arguments exacts), leurs fins (ok, durée, taille), les appels au modèle (jetons, reprises), les tours tels que l'agent les a reçus, la tâche, les fichiers écrits, les propositions. `outil` le restreint à un outil.",
                json!({ "type": "object", "properties": { "outil": { "type": "string" } }, "additionalProperties": false }),
            ),
            def(
                "rejouer_un_outil",
                "Rejoue un appel du run (par son call_id), mêmes arguments, même backend, pour voir ce que l'agent a reçu. Seulement un outil qui ne fait que lire.",
                json!({ "type": "object", "properties": { "call_id": { "type": "string" } }, "required": ["call_id"], "additionalProperties": false }),
            ),
            def("ouvrir_le_resultat", "Le résultat du run : la relecture de la tâche (fichiers, diff), les propositions, les fichiers écrits, ou ce qu'un appel a rendu.", rien.clone()),
            def("bilans_precedents", "Les bilans précédents de la même tâche, dans la mémoire, pour comparer.", rien),
        ]
    }
}

impl DaemonHost {
    /// La mémoire que le manifeste d'un backend déclare (`bilans.memoire`),
    /// résolue depuis son dossier : là où vont ses bilans et ses plans clos.
    pub(super) fn memoire_declaree(&self, hosted: &Arc<Hosted>) -> Result<Option<PathBuf>, String> {
        self.with_backend(hosted, |b| {
            let p = b.prepared();
            p.manifest.bilans.as_ref().map(|d| if d.memoire.is_absolute() { d.memoire.clone() } else { p.directory.join(&d.memoire) })
        })
    }

    /// **Juge un run** d'un backend hébergé (page 04) : rend la fiche, l'apport
    /// et le verdict, et les tickets proposés. Refusé, en le disant, si le
    /// backend ne déclare pas `bilans` ou si le run n'a pas de journal.
    pub fn juger(&self, backend_id: &str, run: &str) -> Result<Value, String> {
        // Le juge ne juge jamais ses propres runs (ils n'ont pas de journal ; et
        // un appel qui le demanderait est refusé en le disant).
        if run.starts_with("juge-") {
            return Err(format!("{run} est un run du juge : il ne se juge pas lui-même"));
        }
        let cible = self.hote_par_id(backend_id)?;
        let memoire_manifeste = self.memoire_declaree(&cible)?;
        let modele = self.with_backend(&cible, |b| (self.juge_model)(&b.prepared()))?;
        // Le manifeste jugé, par son nom de fichier dans son dossier : ce que
        // ses tickets proposent de changer.
        let propre = cible.manifest.file_name().map(|f| f.to_string_lossy().to_string());
        let memoire_manifeste = memoire_manifeste.ok_or("ce backend ne déclare pas `bilans` : rien ne range ses bilans")?;
        let llm = modele.map_err(|why| format!("le juge n'a pas de modèle : {why}"))?;
        let chemin = self.journal.fichier(backend_id, run).ok_or_else(|| format!("le run {run} n'a pas de journal (pas fini, ou effacé)"))?;
        let journal: Value = journal_des_runs::lire_le_fichier(&chemin)?;
        let memoire = self.hosted(&memoire_manifeste)?;

        // La tâche ciblée : la demande, quand le run la dit (un pas d'équipier :
        // tous ses pas vont sur la même fiche, page 07) ; sinon le premier
        // message de l'agent, sinon l'outil appelé.
        let intitule = journal["demande"]
            .as_str()
            .filter(|d| !d.trim().is_empty())
            .map(str::to_string)
            .or_else(|| journal["tours"]
            .as_array()
            .and_then(|t| t.iter().find(|t| t["role"] == "user"))
            .and_then(|t| t["content"].as_str())
            .map(str::to_string))
            .or_else(|| journal["appel"]["outil"].as_str().map(|o| format!("outil {o}")))
            .unwrap_or_else(|| format!("run {run}"));
        let memoire_pour_la_fiche = memoire.clone();
        let (fiche, dit_de_la_fiche) = if est_un_tour_de_claude_code(&journal) {
            fiche_d_un_tour_de_claude_code(self, &memoire_pour_la_fiche, &journal)?
        } else {
            (nom_de_fiche_du_run(run, &journal, &intitule), None)
        };
        if let Some(d) = &dit_de_la_fiche {
            eprintln!("[juge] {run} : {d}");
        }

        // L'enquête : la boucle de l'agent juge, sur ses outils.
        let (mut config, schema_session, schema_outils) = gabarit()?;
        let state_dir = self.chats_root.join("juge");
        std::fs::create_dir_all(&state_dir).map_err(|e| format!("{} : {e}", state_dir.display()))?;
        config.state_dir = state_dir;
        let outils = OutilsDuJuge { host: self, cible: cible.clone(), journal: &journal, memoire: memoire.clone(), fiche: &fiche };
        // Le message système vit dans la session (comme pour le chat de l'hôte) :
        // `run_with` ne le prend pas de la config.
        let mut session = ChatSession::default();
        session.turns.insert(0, Turn::system(&config.system_prompt));
        let debut = std::time::Instant::now();
        let silencieux: EventHandler = Arc::new(|_| {});
        let enquete = session.run_with(
            &consigne_du_juge(run, &journal, &intitule),
            llm.as_ref(),
            &outils,
            &config,
            silencieux,
            Arc::new(AtomicBool::new(false)),
            ChatRunOptions { bus: None, run_id: Some(format!("juge-{run}")), meter: Some(self.chat_meter.clone()), stopped_by: None, retry_budget: Some(crate::llm::BUDGET_DES_RECULS_DU_JUGE), ..Default::default() },
        )?;

        // Le dernier pas : le verdict, dans la forme fixe du schéma.
        // La session ne garde pas le message système : le verdict le reprend, pour
        // que le modèle le rende en juge.
        let mut tours = session.turns.clone();
        if tours.first().is_none_or(|t| t.role != "system") {
            tours.insert(0, Turn::system(&config.system_prompt));
        }
        // Deux rapports, l'un après l'autre (page 05) : les outils d'abord, puis la
        // session, qui renvoie aux défauts bloquants du premier.
        let rendre = |tours: &[Turn], nom: &str, schema: Value| -> Result<Value, String> {
            let opts = GenOptions {
                max_tokens: config.max_output_tokens,
                response_format: Some(ResponseFormat::strict_schema(nom, schema)),
                retry_budget: Some(crate::llm::BUDGET_DES_RECULS_DU_JUGE),
                ..Default::default()
            };
            let mut sink = StringSink::default();
            llm.generate(tours, &opts, &mut sink).map_err(|e| format!("le rapport {nom} du juge : {e:?}"))?;
            serde_json::from_str(sink.text.trim()).map_err(|e| format!("le rapport {nom} n'est pas du JSON : {e} — {}", sink.text))
        };
        tours.push(Turn::user(
            "Rends d'abord le bilan des outils, entier, dans la forme demandée : il juge les outils, pas l'agent ; chaque défaut avec son call_id, sa preuve exacte et la forme que tu proposes, rangé par gravité.",
        ));
        let bilan_des_outils = rendre(&tours, "bilan_des_outils", schema_outils)?;
        tours.push(Turn::new("assistant", bilan_des_outils.to_string()));
        tours.push(Turn::user(
            "Rends maintenant le bilan de la session, entier, dans la forme demandée ; renvoie aux défauts bloquants des outils sans les redire.",
        ));
        let verdict = rendre(&tours, "bilan", schema_session)?;
        let juge_ms = debut.elapsed().as_millis() as u64;

        let cout = cout_du_run(&journal);
        let texte = rendre_le_bilan(&verdict, &cout, run, juge_ms);
        // Ce que la fiche choisie doit dire au lecteur (tâche non nommée,
        // autres fiches citées), en tête du bilan.
        let texte = match &dit_de_la_fiche {
            Some(d) => format!("> {d}\n\n{texte}"),
            None => texte,
        };
        // L'apport d'un pas dit lequel, et quand : la fiche de la tâche en porte un par pas.
        let texte = match entete_du_pas(run, &journal) {
            Some(e) => format!("**{e}**\n\n{texte}"),
            None => texte,
        };
        // **La fiche d'un outil suit le code qu'il exécute** (96) : un graphe livré
        // a une fiche commune, « outil : <nom> » ; un graphe propre au manifeste,
        // « outil : <backend> / <nom> ». Le chemin du graphe sert aussi d'ancre.
        let (nom_du_backend, identites): (String, std::collections::BTreeMap<String, (String, String)>) = self.with_backend(&cible, |b| {
            let p = b.prepared();
            let carte = p
                .manifest
                .tools
                .iter()
                .map(|(nom, t)| {
                    let graphe = t.graph.to_string_lossy().to_string();
                    let resolu = p.directory.join(&t.graph);
                    let livre = graphe.starts_with('@')
                        || resolu.canonicalize().unwrap_or(resolu).to_string_lossy().contains("/templates/tools/");
                    let fiche = if livre { format!("outil : {nom}") } else { format!("outil : {} / {nom}", p.manifest.name) };
                    (nom.clone(), (fiche, graphe))
                })
                .collect();
            (p.manifest.name.clone(), carte)
        })?;
        let ecrit = self.ecrire_le_bilan(&memoire, &fiche, &intitule, run, &journal, &texte, &verdict, &bilan_des_outils, &identites, &nom_du_backend)?;
        let tickets = self.proposer_les_tickets(&cible, propre.as_deref(), &bilan_des_outils, &fiche);
        // **Un défaut bloquant rangé épingle le run** (page 06) : c'est sa preuve,
        // qu'on doit pouvoir relire longtemps après.
        let bloquants: Vec<&str> = bilan_des_outils["outils"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|o| o["defauts"]["bloquant"].as_array().is_some_and(|b| !b.is_empty()))
            .filter_map(|o| o["outil"].as_str())
            .collect();
        if !bloquants.is_empty() {
            self.journal.epingler(backend_id, run, &format!("défaut bloquant rangé par le juge ({})", bloquants.join(", ")));
        }
        Ok(json!({
            "fiche": fiche,
            "intitule": intitule,
            "source": ecrit["source"],
            "fiches_des_outils": ecrit["fiches_des_outils"],
            "verdict": verdict,
            "bilan_des_outils": bilan_des_outils,
            "cout": cout,
            "tickets": tickets,
            "enquete": { "iterations": enquete.iterations, "ms": juge_ms },
        }))
    }

    /// **Les bilans d'un backend**, pour la vue « bilans » de l'IDE : ses fiches
    /// (`via: juge`) dans la mémoire qu'il déclare, chacune avec ses apports dans
    /// l'ordre et la note de chacun (lue dans son bloc json), d'où la tendance.
    pub fn bilans_de(&self, backend_id: &str) -> Result<Value, String> {
        let cible = self.hote_par_id(backend_id)?;
        let memoire = self.with_backend(&cible, |b| {
            let p = b.prepared();
            p.manifest.bilans.as_ref().map(|d| if d.memoire.is_absolute() { d.memoire.clone() } else { p.directory.join(&d.memoire) })
        })?;
        let Some(memoire) = memoire else {
            return Ok(json!({ "declare": false, "fiches": [] }));
        };
        let memoire = self.hosted(&memoire)?;
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { self.with_backend(&memoire, |b| b.call(outil, args)).and_then(|r| r) };
        let lignes = |r: &Value| -> Vec<Value> {
            match &r["result"] {
                Value::Array(a) => a.clone(),
                Value::Object(o) => o.get("results").and_then(Value::as_array).cloned().unwrap_or_default(),
                _ => Vec::new(),
            }
        };
        // Les fiches du juge : celles des tâches (« bilan : … ») et celles des
        // outils (« outil : … »), chacune trouvée par son mot.
        let mut fiches = Vec::new();
        for mot in ["bilan", "outil"] {
            let r = appeler("recall_fiches", json!({ "query": mot, "options": { "limit": 100, "filter_condition": { "field": { "key": "via", "value": "juge" } } } }))?;
            fiches.extend(lignes(&r));
        }
        let mut vues = std::collections::BTreeSet::new();
        let mut sortie = Vec::new();
        for f in fiches {
            let data = if f["data"].is_object() { f["data"].clone() } else { f.clone() };
            let Some(nom) = data["name"].as_str().map(str::to_string) else { continue };
            if !vues.insert(nom.clone()) {
                continue;
            }
            let apports = appeler("recall_apports", json!({ "query": nom, "options": { "limit": 100, "filter_condition": { "field": { "key": "fiche", "value": nom } } } }))?;
            let mut liste: Vec<Value> = lignes(&apports)
                .into_iter()
                .map(|a| if a["data"].is_object() { a["data"].clone() } else { a })
                .map(|a| {
                    let texte = a["texte"].as_str().unwrap_or_default().to_string();
                    let verdict = texte
                        .rsplit_once("```json\n")
                        .and_then(|(_, j)| j.split_once("\n```"))
                        .and_then(|(j, _)| serde_json::from_str::<Value>(j).ok())
                        .unwrap_or(Value::Null);
                    json!({
                        "source": a["source"],
                        "creee": a["created_at"],
                        "texte": texte,
                        "note": verdict["note"],
                        "comparaison": verdict["comparaison"],
                        "defauts": verdict["defauts"],
                        "part_utilisee": verdict["part_utilisee"],
                    })
                })
                .collect();
            liste.sort_by_key(|a| a["creee"].as_i64().unwrap_or(0));
            sortie.push(json!({ "fiche": nom, "outil": nom.starts_with("outil : "), "about": data["about"], "apports": liste }));
        }
        Ok(json!({ "declare": true, "fiches": sortie }))
    }

    /// Le modèle du juge (un faux pour les témoins).
    pub fn with_juge_model(mut self, model: chat::ChatModel) -> Self {
        self.juge_model = model;
        self
    }

    /// Dépose un bilan à faire, si le backend déclare `bilans` (la fin d'un tour).
    pub(super) fn demander_un_bilan(&self, hosted: &Arc<Hosted>, run: &str) {
        let declare = self.with_backend(hosted, |b| b.prepared().manifest.bilans.is_some()).unwrap_or(false);
        if declare {
            if let Some(envoi) = self.bilans.envoi.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
                let _ = envoi.send((hosted.id.clone(), run.to_string()));
            }
        }
    }

    /// **Le fil des bilans**, lancé une fois par le serveur : il attend que le
    /// journal du run soit complet (ses tours ajoutés), puis le juge ; un refus
    /// ou une panne se disent, et le fil continue. Il finit avec l'hôte.
    pub fn servir_les_bilans(self: &Arc<Self>) {
        let Some(reception) = self.bilans.reception.lock().unwrap_or_else(|p| p.into_inner()).take() else { return };
        let hote = Arc::downgrade(self);
        let arrete = self.bilans.arrete.clone();
        let lance = std::thread::Builder::new().name("juge".into()).spawn(move || {
            let mut plafonds = PlafondDuJuge::default();
            for (backend, run) in reception {
                if arrete.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                let Some(h) = hote.upgrade() else { break };
                let plafond = h.with_backend_id(&backend, |b| b.prepared().manifest.bilans.as_ref().map(|d| d.max_per_hour)).ok().flatten().unwrap_or(20);
                if !plafonds.admettre(&backend, plafond, std::time::Instant::now()) {
                    h.dire_le_saut_du_juge(&backend, &run, plafond);
                    continue;
                }
                let fin = std::time::Instant::now() + std::time::Duration::from_secs(30);
                while !h
                    .journal
                    .lire(&backend, &run)
                    .is_some_and(|v| v["tours"].is_array() || v["appel"].is_object())
                {
                    if std::time::Instant::now() > fin {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                match h.juger(&backend, &run) {
                    Ok(v) => eprintln!("[juge] {backend} {run} : {} (note {})", v["fiche"], v["verdict"]["note"]),
                    Err(e) => eprintln!("[juge] {backend} {run} : {e}"),
                }
            }
        });
        match lance {
            Ok(fil) => *self.bilans.fil.lock().unwrap_or_else(|p| p.into_inner()) = Some(fil),
            Err(e) => eprintln!("[juge] le fil des bilans ne part pas : {e}"),
        }
    }

    /// **Arrêter le juge** (à l'arrêt de l'hôte) : la file se ferme, ce qui reste n'est
    /// plus jugé, et le jugement en cours est attendu, au plus `ARRET_DU_JUGE`. Rend une
    /// erreur dite si le juge tient encore. Sans fil (le juge jamais servi), rien.
    pub(super) fn arreter_le_juge(&self) -> Option<String> {
        self.bilans.arrete.store(true, std::sync::atomic::Ordering::SeqCst);
        drop(self.bilans.envoi.lock().unwrap_or_else(|p| p.into_inner()).take());
        let fil = self.bilans.fil.lock().unwrap_or_else(|p| p.into_inner()).take()?;
        let fin = std::time::Instant::now() + ARRET_DU_JUGE;
        while !fil.is_finished() {
            if std::time::Instant::now() > fin {
                return Some(format!("le juge juge encore après {} s : l'hôte s'arrête sans lui", ARRET_DU_JUGE.as_secs()));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = fil.join();
        None
    }

    /// **Un run que le plafond laisse sans jugement** : le saut se dit au journal du
    /// run (`bilan`), sur la sortie d'erreur, et dans la boîte quand le run est celui
    /// d'une tâche d'équipier — jamais en silence.
    fn dire_le_saut_du_juge(&self, backend: &str, run: &str, plafond: u32) {
        let pourquoi = format!("sauté : plafond de {plafond} jugements par heure atteint (bilans.max_per_hour)");
        eprintln!("[juge] {backend} {run} : {pourquoi}");
        let mut ajouts = serde_json::Map::new();
        ajouts.insert("bilan".into(), json!(pourquoi));
        self.journal.ajouter(backend, run, ajouts);
        let tache = self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().find(|t| t.id == backend).cloned();
        if let Some(t) = tache {
            if let Ok(files) = self.teammate_files(&t.parent, &t.nom) {
                let _ = files.append("host", "report", None, &format!("le run {run} n'est pas jugé : {pourquoi}"));
            }
        }
    }

    /// La fiche (créée si besoin, `via: juge`), le L0 (le journal, au chemin),
    /// le L1 de section `bilan`, son lien, et une ancre par outil appelé (96).
    #[allow(clippy::too_many_arguments)]
    fn ecrire_le_bilan(
        &self,
        memoire: &Arc<Hosted>,
        fiche: &str,
        intitule: &str,
        run: &str,
        journal: &Value,
        texte: &str,
        verdict: &Value,
        bilan_des_outils: &Value,
        identites: &std::collections::BTreeMap<String, (String, String)>,
        nom_du_backend: &str,
    ) -> Result<Value, String> {
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { self.with_backend(memoire, |b| b.call(outil, args)).and_then(|r| r) };
        // `get_fiche` rend une réponse même quand la fiche n'existe pas : elle
        // existe si la réponse porte son nom.
        let assurer_la_fiche = |nom_de_fiche: &str, about: &str| -> Result<(), String> {
            let nom = json!({ "name": nom_de_fiche }).to_string();
            let nom = &nom[1..nom.len() - 1];
            let existe = appeler("get_fiche", json!({ "record": { "name": nom_de_fiche } })).is_ok_and(|r| r["result"].to_string().contains(nom));
            if !existe {
                appeler("put_fiche", json!({ "record": { "name": nom_de_fiche, "about": about, "state": "current", "via": "juge" } }))?;
            }
            Ok(())
        };
        assurer_la_fiche(fiche, intitule)?;
        let jour = chrono_jour();
        let chemin = format!("sources/{jour}-{}.md", run.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-"));
        let contenu = format!("# Journal du run {run}\n\n```json\n{}\n```\n", serde_json::to_string_pretty(journal).unwrap_or_default());
        appeler(
            "put_source",
            json!({ "record": { "chemin": chemin, "titre": format!("le run {run}"), "contenu": contenu, "origine": "envoi", "session": run, "via": "juge" } }),
        )?;
        let texte_l1 = format!("{texte}\n\n```json\n{}\n```\n", serde_json::to_string_pretty(verdict).unwrap_or_default());
        appeler(
            "put_apport",
            json!({ "record": { "fiche": fiche, "source": chemin, "section": "bilan", "texte": texte_l1, "niveau": "L1", "state": "current", "via": "juge" } }),
        )?;
        appeler("relier_apport", json!({ "links": [{ "from": { "name": fiche }, "to": { "fiche": fiche, "source": chemin, "section": "bilan" } }] }))?;
        // La provenance de chaque apport (96) : une Ref doc:<source>#<section>.
        let _ = appeler("create_ref_type", json!({ "genre": "doc" }));
        let citer = |fiche_de_l_apport: &str, section: &str| {
            if let Err(e) = appeler(
                "citer_source",
                json!({ "genre": "doc", "valeur": format!("{chemin}#{section}"), "fiche": { "fiche": fiche_de_l_apport, "source": chemin, "section": section } }),
            ) {
                eprintln!("[juge] la provenance de « {fiche_de_l_apport} » ({section}) : {e}");
            }
        };
        citer(fiche, "bilan");
        // Une ancre par outil appelé : `cited_by(value: "outil:<nom>")` rend ses bilans.
        let _ = appeler("create_ref_type", json!({ "genre": "outil" }));
        let mut outils: Vec<String> = appels_du_run(journal)
            .into_iter()
            .filter(|a| a.de_l_hote)
            .map(|a| a.outil)
            .chain(journal["appel"]["outil"].as_str().map(str::to_string))
            .collect();
        outils.sort();
        outils.dedup();
        for o in &outils {
            if let Err(e) = appeler("ancrer_fiche", json!({ "genre": "outil", "valeur": format!("outil:{o}"), "fiche": { "name": fiche } })) {
                eprintln!("[juge] l'ancre outil:{o} de « {fiche} » : {e}");
            }
        }
        // **Une fiche par outil** (page 05, c7) : « outil : <nom> », un L1
        // « defauts » par run jugé, de la même source, ancrée `outil:<nom>`.
        let mut fiches_des_outils = Vec::new();
        for o in bilan_des_outils["outils"].as_array().into_iter().flatten() {
            let Some(nom_outil) = o["outil"].as_str() else { continue };
            let (fiche_outil, graphe) = identites.get(nom_outil).cloned().unwrap_or_else(|| (format!("outil : {nom_outil}"), String::new()));
            assurer_la_fiche(&fiche_outil, &format!("Les défauts de l'outil {nom_outil}, jugés run par run."))?;
            let texte_outil = format!("{}\n\n```json\n{}\n```\n", rendre_les_defauts(o, run, nom_du_backend), serde_json::to_string_pretty(o).unwrap_or_default());
            appeler(
                "put_apport",
                json!({ "record": { "fiche": fiche_outil, "source": chemin, "section": "defauts", "texte": texte_outil, "niveau": "L1", "state": "current", "via": "juge" } }),
            )?;
            appeler("relier_apport", json!({ "links": [{ "from": { "name": fiche_outil }, "to": { "fiche": fiche_outil, "source": chemin, "section": "defauts" } }] }))?;
            citer(&fiche_outil, "defauts");
            // Deux ancres (96) : le nom, pour trouver tous les homonymes ; le
            // graphe, pour savoir quel code a été jugé.
            let _ = appeler("create_ref_type", json!({ "genre": "gabarit" }));
            let mut ancres = vec![("outil", format!("outil:{nom_outil}"))];
            if !graphe.is_empty() {
                ancres.push(("gabarit", format!("gabarit:{graphe}")));
            }
            for (genre, valeur) in ancres {
                if let Err(e) = appeler("ancrer_fiche", json!({ "genre": genre, "valeur": valeur, "fiche": { "name": fiche_outil } })) {
                    eprintln!("[juge] l'ancre {valeur} de « {fiche_outil} » : {e}");
                }
            }
            fiches_des_outils.push(fiche_outil);
        }
        Ok(json!({ "source": chemin, "ancres": outils, "fiches_des_outils": fiches_des_outils }))
    }

    /// Un ticket par `description_trompeuse` (page 05 : le seul genre qui en
    /// devient un) : une proposition sur la clé
    /// `description` de l'outil dans le manifeste cible (les `.mmd` partagés
    /// sont hors de son dossier), appliquée ou rejetée comme les autres.
    fn proposer_les_tickets(&self, cible: &Arc<Hosted>, manifeste: Option<&str>, bilan_des_outils: &Value, fiche: &str) -> Vec<Value> {
        let Some(manifeste) = manifeste else { return Vec::new() };
        let mut tickets = Vec::new();
        let trompeuses = bilan_des_outils["outils"].as_array().into_iter().flatten().flat_map(|o| {
            ["bloquant", "genant", "mineur"]
                .into_iter()
                .flat_map(move |g| o["defauts"][g].as_array().cloned().unwrap_or_default())
                .filter(|d| d["genre"] == "description_trompeuse")
                .map(move |d| (o["outil"].as_str().unwrap_or_default().to_string(), d))
        });
        for (outil, d) in trompeuses {
            let outil = outil.as_str();
            let Some(proposee) = d["proposee"].as_str() else { continue };
            let a = json!({ "phrase": d["preuve"] });
            let fait = self.with_backend(cible, |b| {
                let p = b.prepared();
                if !p.manifest.tools.contains_key(outil) {
                    return Err(format!("« {outil} » n'est pas un outil de ce manifeste"));
                }
                let lu = std::fs::read_to_string(p.directory.join(manifeste)).map_err(|e| e.to_string())?;
                let mut m: Value = serde_json::from_str(&lu).map_err(|e| e.to_string())?;
                m["tools"][outil]["description"] = json!(proposee);
                let texte = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n";
                let raison = format!("le juge ({fiche}) : « {} » a trompé l'agent", a["phrase"].as_str().unwrap_or(""));
                b.proposer(&[(manifeste.to_string(), texte)], &raison, "juge").map(|p| json!({ "outil": outil, "proposition": p.id }))
            });
            match fait.and_then(|r| r) {
                Ok(t) => tickets.push(t),
                Err(e) => tickets.push(json!({ "outil": outil, "refus": e })),
            }
        }
        tickets
    }
}

/// **La fiche d'un tour de Claude Code** : la tâche courante de la session, par
/// `whois { handle }` de la mémoire (page 04 §6 : la fiche d'identité, son
/// dernier apport `identity`, la première fiche de sa ligne `doing`). Sans tâche
/// nommée, la fiche d'identité, et c'est dit (« aucune tâche nommée : nomme-la ») ;
/// sans fiche d'identité, le refus le dit. Rend aussi ce qui doit l'être dans le
/// bilan (les autres fiches citées, nommées sans y ranger le bilan).
fn fiche_d_un_tour_de_claude_code(host: &DaemonHost, memoire: &Arc<Hosted>, journal: &Value) -> Result<(String, Option<String>), String> {
    let handle = journal["session"].as_str().ok_or("le tour de Claude Code ne dit pas sa session (`session`)")?;
    let qui = host.with_backend(memoire, |b| b.call("whois", json!({ "handle": handle }))).and_then(|r| r)?;
    let qui = &qui["result"];
    let autres = qui["autres"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).filter(|s| !s.is_empty());
    match (qui["tache"].as_str(), qui["identite"].as_str()) {
        (Some(tache), _) => Ok((tache.to_string(), autres.map(|a| format!("autres fiches de la tâche, non rangées : {a}")))),
        (None, Some(identite)) => Ok((identite.to_string(), Some(format!("aucune tâche nommée : nomme-la ({})", qui["said"].as_str().unwrap_or("aucun apport identity"))))),
        (None, None) => Err(format!("{handle} : {}", qui["said"].as_str().unwrap_or("aucune fiche d'identité ne cite ce handle"))),
    }
}

/// Le coût du run, par slug et par unité, lu dans ses `LlmCall` et `Consumed`.
fn cout_du_run(journal: &Value) -> Value {
    if est_un_tour_de_claude_code(journal) {
        return json!({ "non_mesure": "tour de Claude Code : ses appels au modèle ne passent pas par l'hôte" });
    }
    let mut cout: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for e in journal["evenements"].as_array().into_iter().flatten() {
        if e["kind"] == "LlmCall" {
            let modele = e["model"].as_str().unwrap_or("?");
            for unite in ["prompt_tokens", "cached_prompt_tokens", "completion_tokens"] {
                *cout.entry(format!("{modele} {unite}")).or_default() += e[unite].as_i64().unwrap_or(0);
            }
        }
    }
    json!(cout)
}

pub(super) fn chrono_jour() -> String {
    jour_depuis(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

/// Le jour civil d'un instant (secondes depuis l'époque), `AAAA-MM-JJ`.
pub(super) fn jour_depuis(s: u64) -> String {
    let (j, _) = (s / 86_400, s % 86_400);
    // Le jour civil (algorithme de Howard Hinnant), sans dépendance.
    let z = j as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Le bilan lisible, tel que Lucie le lit dans la fiche : les rubriques en
/// prose courte.
fn rendre_le_bilan(v: &Value, cout: &Value, run: &str, juge_ms: u64) -> String {
    let liste = |cle: &str| -> String {
        let items: Vec<String> = v[cle].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(|s| format!("- {s}"))).collect();
        if items.is_empty() { "- rien".into() } else { items.join("\n") }
    };
    let faits: Vec<String> = v["faits"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| format!("- ({}) {}", f["nature"].as_str().unwrap_or("?"), f["texte"].as_str().unwrap_or("")))
        .collect();
    let renvois: Vec<String> = v["renvois_bloquants"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| format!("- `{}` : {} (voir sa fiche « outil : {} »)", r["outil"].as_str().unwrap_or("?"), r["genre"].as_str().unwrap_or("?"), r["outil"].as_str().unwrap_or("?")))
        .collect();
    format!(
        "**Note {}/10** (confiance {}), objectif {}. {}\n\n\
         Comparaison : {} — {}\n\n\
         Faits :\n{}\n\nDéfauts bloquants des outils :\n{}\n\nFrictions :\n{}\n\nAttentes :\n{}\n\n\
         Coût : {}\n\n_Run {run} ; jugé en {juge_ms} ms._",
        v["note"],
        v["confiance"].as_str().unwrap_or("?"),
        v["objectif"].as_str().unwrap_or("?"),
        v["resume"].as_str().unwrap_or(""),
        v["comparaison"]["verdict"].as_str().unwrap_or("?"),
        v["comparaison"]["sur"].as_str().unwrap_or(""),
        if faits.is_empty() { "- rien".into() } else { faits.join("\n") },
        if renvois.is_empty() { "- aucun".into() } else { renvois.join("\n") },
        liste("frictions"),
        liste("attentes"),
        cout,
    )
}

/// Les défauts d'un outil dans un run, lisibles dans sa fiche : par gravité,
/// chacun avec sa preuve et sa forme proposée.
fn rendre_les_defauts(o: &Value, run: &str, backend: &str) -> String {
    let mut texte = format!(
        "**`{}`** — {} appel(s), {} échec(s), {} inutile(s) ; part du retour utilisée : {}.",
        o["outil"].as_str().unwrap_or("?"),
        o["appels"],
        o["echecs"],
        o["inutiles"],
        o["part_utilisee"]
    );
    for (g, titre) in [("bloquant", "Bloquants"), ("genant", "Gênants"), ("mineur", "Mineurs")] {
        let defauts = o["defauts"][g].as_array().cloned().unwrap_or_default();
        texte.push_str(&format!("\n\n{titre} :"));
        if defauts.is_empty() {
            texte.push_str("\n- aucun");
        }
        for d in defauts {
            let genre = match d["genre"].as_str().unwrap_or("?") {
                "autre" => format!("autre — {}", d["libelle"].as_str().unwrap_or("")),
                g => g.to_string(),
            };
            texte.push_str(&format!(
                "\n- {genre} (appel {}) : « {} »{}\n  proposé : {}",
                d["call_id"].as_str().unwrap_or("?"),
                d["preuve"].as_str().unwrap_or(""),
                d["ce_que_l_agent_en_a_fait"].as_str().filter(|s| !s.is_empty()).map(|s| format!(" — l'agent : {s}")).unwrap_or_default(),
                d["proposee"].as_str().unwrap_or("")
            ));
        }
    }
    // Le backend du run (c7) : un défaut qui ne vient que d'une base se départage.
    texte.push_str(&format!("\n\n_Run {run}, backend {backend}._"));
    texte
}

/// **Le plafond du juge** (`bilans.max_per_hour`) : les heures des derniers
/// jugements, par backend, comptées sur une heure glissante.
#[derive(Default)]
pub(super) struct PlafondDuJuge {
    juges: std::collections::HashMap<String, std::collections::VecDeque<std::time::Instant>>,
}

impl PlafondDuJuge {
    /// Ce run peut-il être jugé maintenant ? S'il l'est, il compte.
    pub(super) fn admettre(&mut self, backend: &str, plafond: u32, maintenant: std::time::Instant) -> bool {
        let recents = self.juges.entry(backend.to_string()).or_default();
        while recents.front().is_some_and(|t| maintenant.duration_since(*t) >= std::time::Duration::from_secs(3600)) {
            recents.pop_front();
        }
        if recents.len() >= plafond as usize {
            return false;
        }
        recents.push_back(maintenant);
        true
    }
}

#[cfg(test)]
mod tests {

    /// **Le plafond du juge** (ticket « le juge des bilans n'a pas de plafond d'appels ») :
    /// à `max_per_hour: 2`, trois runs dans l'heure → deux jugés, le troisième non ; une heure
    /// plus tard, le compte est revenu ; un autre backend a son propre compte.
    #[test]
    fn the_judge_is_capped_per_backend_on_a_sliding_hour() {
        let t0 = std::time::Instant::now();
        let mut p = super::PlafondDuJuge::default();
        assert!(p.admettre("chat", 2, t0));
        assert!(p.admettre("chat", 2, t0 + std::time::Duration::from_secs(60)));
        assert!(!p.admettre("chat", 2, t0 + std::time::Duration::from_secs(120)), "le troisième dans l'heure est sauté");
        assert!(p.admettre("autre", 2, t0 + std::time::Duration::from_secs(120)), "un autre backend a son compte");
        assert!(p.admettre("chat", 2, t0 + std::time::Duration::from_secs(3600)), "une heure après le premier, une place revient");
        assert!(!p.admettre("chat", 2, t0 + std::time::Duration::from_secs(3601)), "le deuxième compte encore");
        assert!(!p.admettre("chat", 0, t0), "un plafond nul ne juge rien");
    }

    use super::*;

    /// **Un pas d'équipier se juge sur son but, sous le nom court de sa tâche** (8 novembre
    /// 2026, c7) : la fiche « bilan : <tâche> » (un apport par pas, « pas N · <date> » en
    /// tête), et le juge reçoit le message du pas pour but ; un run qui n'est pas un pas
    /// garde le nom de son intitulé et la consigne d'avant.
    #[test]
    fn a_teammate_step_has_its_own_fiche_and_is_judged_on_its_goal() {
        let run = format!("agent-{:x}-887", 1794098554713u64);
        let journal = json!({
            "tache": { "nom": "journaux-annulation-tmp-3", "parent": "e4d8635ecfe7" },
            "pas": 3,
            "demande": "Tâche réelle, bornée ".repeat(100),
            "tours": [{ "role": "user", "content": "[Pas 3]\n\nTon plan actif : écrire le témoin rouge" }],
        });
        let nom = nom_de_fiche_du_run(&run, &journal, journal["demande"].as_str().unwrap());
        assert_eq!(nom, "bilan : journaux-annulation-tmp-3", "le nom court de la tâche, pas sa demande");
        assert!(entete_du_pas(&run, &journal).is_some_and(|e| e.starts_with("pas 3 · 2026-11-")), "{:?}", entete_du_pas(&run, &journal));
        let consigne = consigne_du_juge(&run, &journal, "");
        assert!(consigne.contains("le pas 3 de la tâche « journaux-annulation-tmp-3 »") && consigne.contains("non la tâche entière") && consigne.contains("écrire le témoin rouge"), "{consigne}");
        let autre = json!({ "demande": "Dis salut" });
        assert_eq!(nom_de_fiche_du_run("chat-x", &autre, "Dis salut"), "bilan : dis salut");
        assert!(consigne_du_juge("chat-x", &autre, "Dis salut").contains("sur la tâche « Dis salut »"));
    }

    /// Le nom d'une fiche de bilan : une seule fonction (96).
    #[test]
    fn the_task_name_is_stable() {
        assert_eq!(nom_de_tache("  Dis SALUT  au lieu\nde bonjour ! "), "bilan : dis salut au lieu de bonjour");
        assert_eq!(nom_de_tache("« Écris une note. »"), "bilan : écris une note");
    }

    /// Le gabarit livré du juge se lit, et son schéma est strict.
    #[test]
    fn the_judge_template_reads() {
        let (config, schema, outils) = gabarit().unwrap();
        assert!(outils["properties"]["outils"]["items"]["properties"]["defauts"]["required"].as_array().unwrap().len() == 3, "les trois gravités, toujours");
        assert_eq!(config.name, "juge");
        assert!(config.system_prompt.contains("rejouer_un_outil"));
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema["required"].as_array().unwrap().iter().any(|r| r == "comparaison"));
    }

    /// **Les appels d'un tour de Claude Code** (lot 3 de rag3weaver archi) : ils
    /// viennent de `tours[].tool_calls`, avec ce que Claude a reçu ; un
    /// `mcp__<serveur>__<outil>` est un outil de l'hôte, un `Bash` n'en est pas.
    #[test]
    fn un_tour_de_claude_code_rend_ses_appels_de_l_hote_et_laisse_les_siens() {
        let tour = json!({
            "genre": TOUR_DE_CLAUDE_CODE,
            "session": "session:7d2e",
            "serveurs": { "code-rag3db": "/m/code.json" },
            "tours": [
                { "role": "user", "content": "trouve le juge" },
                { "role": "assistant", "tool_calls": [
                    { "id": "t1", "name": "mcp__code-rag3db__search_code", "input": { "query": "juge" } },
                    { "id": "t2", "name": "Bash", "input": { "command": "ls" } }
                ] },
                { "role": "tool", "tool_use_id": "t1", "is_error": false, "content": "juge.rs" },
                { "role": "tool", "tool_use_id": "t2", "is_error": false, "content": "src" }
            ],
            "evenements": []
        });
        let appels = appels_du_run(&tour);
        assert_eq!(appels.len(), 2);
        assert_eq!(appels[0].outil, "search_code");
        assert_eq!(appels[0].serveur.as_deref(), Some("code-rag3db"));
        assert!(appels[0].de_l_hote);
        assert_eq!(appels[0].arguments, json!({ "query": "juge" }));
        assert_eq!(appels[0].recu, Some(json!({ "content": "juge.rs", "is_error": false })));
        assert_eq!((appels[1].outil.as_str(), appels[1].de_l_hote, appels[1].serveur.clone()), ("Bash", false, None), "un outil propre de Claude Code");
        assert_eq!(cout_du_run(&tour)["non_mesure"].as_str().map(|s| s.contains("Claude Code")), Some(true), "le coût n'est pas un zéro");
    }

    /// Un run du bus garde sa lecture : ses `ToolCallStarted`, sans serveur.
    #[test]
    fn un_run_du_bus_rend_ses_appels_comme_avant() {
        let run = json!({ "evenements": [
            { "kind": "Token", "text": "x" },
            { "kind": "ToolCallStarted", "call_id": "c1", "tool": "read_file", "arguments": "{\"path\":\"a.rs\"}" }
        ] });
        let appels = appels_du_run(&run);
        assert_eq!(appels, vec![Appel { id: "c1".into(), outil: "read_file".into(), serveur: None, arguments: json!({ "path": "a.rs" }), recu: None, de_l_hote: true }]);
        assert!(cout_du_run(&run).get("non_mesure").is_none());
    }

    #[test]
    fn un_nom_mcp_se_lit_par_son_serveur_et_son_outil() {
        assert_eq!(nom_mcp("mcp__memoire-longue__recall_fiches"), Some(("memoire-longue", "recall_fiches")));
        assert_eq!(nom_mcp("Read"), None);
    }
}
