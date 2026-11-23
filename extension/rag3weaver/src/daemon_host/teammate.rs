//! **L'équipier** (page 07 du 25 octobre 2026) : un agent qui accomplit une
//! tâche seul, par pas, et parle à un superviseur par une boîte aux lettres.
//!
//! - Le mini-plan n'est pas obligatoire : une petite demande se fait sans. Quand
//!   il y en a, il vit dans `<dossier de la tâche>/plan.json` (une liste : au plus
//!   un plan `active`, des plans `paused`) ; l'agent l'édite par `edit_plan`,
//!   librement, et clôt un plan (`accomplished` ou `obsolete`) pour en ouvrir un
//!   autre. Un plan clos part dans `plans/<n>.json`, puis en L1 sur la fiche de
//!   session (plus tard dans ce lot).
//! - Un pas est un tour de chat dans la tâche : l'agent travaille, puis appelle
//!   `step_done`, un outil qui termine la boucle. Au-delà des bornes du pas (`teammate.step_context_tokens`, `max_tool_calls_per_step`)
//!   appels sans `step_done`, tout autre appel est refusé.
//! - La boîte est `<dossier de la tâche>/mailbox.jsonl`, une ligne par message,
//!   publiée aussi sur le sujet `mailbox.<task>` du bus de la tâche. Le
//!   superviseur y écrit par `POST /ide/teammate/mailbox` ; l'équipier lit ce
//!   qui est arrivé à la frontière d'un pas, jamais pendant. L'arrêt, lui, passe
//!   tout de suite (`stop_run`).
//! - Autonomie maximale (Lucie) : l'agent clôt seul ses éléments et son plan, ne
//!   demande rien pour ce qui est dans le plan ; le hors-plan part en `proposal`
//!   (`suggest`) et le plan continue sans attendre la réponse.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::DaemonHost;
use crate::llm::{ToolCall, Turn};
use crate::tools::ToolDef;

pub const EDIT_PLAN: &str = "edit_plan";
pub const STEP_DONE: &str = "step_done";
pub const REPORT: &str = "report";
/// `propose` est pris par les propositions de déclaration : l'outil du hors-plan s'appelle `suggest`.
pub const SUGGEST: &str = "suggest";
/// Mode pair, lot 3: write to a Claude Code session.
pub const SEND_TO_SESSION: &str = "send_to_session";
pub const FETCH: &str = "fetch";
pub const WEB_SEARCH: &str = "web_search";
/// Les pas d'un équipier lancé, au plus (un garde-fou, pas un réglage du grain).
const MAX_STEPS: usize = 40;
/// Les pas de suite sans `step_done` réussi avant que l'équipier s'arrête et le
/// dise : il tourne en rond.
const MAX_STALLED: usize = 3;
/// Les derniers `step_done` rappelés à chaque pas (les plus anciens se lisent au plan).
const MAX_FAITS_RAPPELES: usize = 20;

/// Ce que l'équipier lit en plus du message système du chat dans une tâche.
const SYSTEM_PROMPT: &str = concat!(include_str!("../../templates/teammate/prompts/system.md"), super::observed::observed_content_rule!());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Todo,
    Doing,
    Done,
    Blocked,
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Active,
    Paused,
    Accomplished,
    Obsolete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub text: String,
    pub status: Status,
    /// Faisable sans validation du superviseur.
    #[serde(default = "yes")]
    pub alone: bool,
    /// La question posée au superviseur, si l'élément demande une validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// Pourquoi il est bloqué, abandonné ou raté.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn yes() -> bool {
    true
}

/// **Un mini-plan** : la demande comprise, la proposition, les éléments, son état.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Le rang de ce plan dans la tâche (1, 2…).
    pub number: u32,
    pub request: String,
    pub proposal: String,
    pub items: Vec<Item>,
    pub state: PlanState,
    /// Pourquoi il est en pause, ou clos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Les runs des pas faits sous ce plan (pour le juge et le L1).
    #[serde(default)]
    pub steps: Vec<String>,
    /// Une intuition de l'agent, en texte : l'archiviste la lit comme une
    /// indication, jamais comme une ancre (96).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concepts: Vec<String>,
}

impl Plan {
    /// Un élément à faire, faisable seul.
    pub fn next_alone(&self) -> Option<&Item> {
        self.items.iter().find(|i| i.alone && matches!(i.status, Status::Todo | Status::Doing))
    }
    /// Les éléments qui attendent le superviseur.
    pub fn waiting(&self) -> Vec<&Item> {
        self.items
            .iter()
            .filter(|i| i.status == Status::Blocked || (!i.alone && matches!(i.status, Status::Todo | Status::Doing)))
            .collect()
    }
}

/// **`plan.json`** : les plans vivants de la tâche — au plus un `active`, les
/// autres `paused`. Les plans clos n'y sont plus.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plans {
    pub plans: Vec<Plan>,
}

impl Plans {
    pub fn active(&self) -> Option<&Plan> {
        self.plans.iter().find(|p| p.state == PlanState::Active)
    }
    pub fn active_mut(&mut self) -> Option<&mut Plan> {
        self.plans.iter_mut().find(|p| p.state == PlanState::Active)
    }
    pub fn paused(&self) -> Vec<&Plan> {
        self.plans.iter().filter(|p| p.state == PlanState::Paused).collect()
    }
}

/// **Le dossier d'un équipier** : les plans, les plans clos, la boîte.
/// `key` (`<parent>/<task>`) préfixe l'identifiant de chaque ligne de la boîte ;
/// `relay`, quand l'hôte le pose, relaie chaque ligne écrite au flux de l'IDE.
#[derive(Debug, Clone)]
pub struct TeammateFiles {
    pub dir: PathBuf,
    pub key: String,
    pub relay: Option<MailboxRelay>,
    /// **Le nom de l'agent** (Algareth, 15 novembre 2026) : écrit en `name` sur chaque ligne du rôle
    /// `teammate`, à côté du rôle, que les gardes de la boucle lisent seul.
    pub nom: Option<String>,
}

/// The `AgentMessage` of a mailbox line written between agents, or `None`: a `message` or an `answer` whose sender or
/// recipient is another agent (`agent:<key>`); `teammate` is the task's own key (`cle`).
pub fn agent_message(line: &Value, cle: Option<&str>, parent: &str, task: &str) -> Option<Value> {
    let kind = line["kind"].as_str()?;
    if !matches!(kind, "message" | "answer") {
        return None;
    }
    let qui = |v: &Value| -> Option<String> {
        match v.as_str()? {
            "teammate" => cle.map(str::to_string),
            autre => Some(autre.to_string()),
        }
    };
    let (from, to) = (qui(&line["from"])?, qui(&line["to"])?);
    if !from.starts_with("agent:") || !to.starts_with("agent:") {
        return None;
    }
    Some(json!({
        "kind": "AgentMessage",
        "from": from,
        "to": to,
        "parent": parent,
        "task": task,
        "id": line["id"],
        "text": line["text"],
        "reply_to": line["reply_to"],
        "at": line["at"],
    }))
}

/// **Le relais d'une boîte** vers le flux de l'IDE (sujet `mailbox`) : appelé
/// avec chaque ligne écrite (page 07-novembre-2026-everything-declarative/01, § 2).
#[derive(Clone)]
pub struct MailboxRelay(pub Arc<dyn Fn(&Value) + Send + Sync>);

impl std::fmt::Debug for MailboxRelay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MailboxRelay")
    }
}

/// Une écriture de boîte à la fois dans le processus : le rang de la ligne
/// (son identifiant) se lit et s'écrit sans qu'une autre s'intercale.
static MAILBOX_WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Les genres qui attendent une réponse (sortis par une ligne `reply_to`), et
/// ceux qui attendent d'être vus (sortis par `seen`).
const AWAITS_REPLY: [&str; 3] = ["question", "approval", "proposal"];
/// Ce qui attend d'être vu : un compte rendu, une fin, une livraison, et une
/// parole adressée d'un agent à un autre (`message`, `answer` qui porte un `to`) —
/// le pont (archi) : sans eux, un échange Claude ↔ Codex n'apparaissait jamais.
const AWAITS_SEEN: [&str; 5] = ["report", "ended", "delivery", "message", "answer"];

impl TeammateFiles {
    pub fn new(dir: PathBuf, key: impl Into<String>) -> Self {
        TeammateFiles { dir, key: key.into(), relay: None, nom: None }
    }
    pub fn plan_path(&self) -> PathBuf {
        self.dir.join("plan.json")
    }
    pub fn mailbox_path(&self) -> PathBuf {
        self.dir.join("mailbox.jsonl")
    }
    pub fn read_plans(&self) -> Plans {
        std::fs::read(self.plan_path()).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
    }
    pub fn write_plans(&self, plans: &Plans) -> Result<(), String> {
        let p = self.plan_path();
        if plans.plans.is_empty() {
            let _ = std::fs::remove_file(&p);
            return Ok(());
        }
        let neuf = p.with_extension("json.neuf");
        std::fs::write(&neuf, serde_json::to_vec_pretty(plans).unwrap_or_default())
            .and_then(|_| std::fs::rename(&neuf, &p))
            .map_err(|e| format!("{} : {e}", p.display()))
    }
    /// Un plan clos, gardé à côté (`plans/<n>.json`) : son histoire locale, et la
    /// source du L1.
    pub fn keep_closed(&self, plan: &Plan) -> Result<(), String> {
        let d = self.dir.join("plans");
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        std::fs::write(d.join(format!("{}.json", plan.number)), serde_json::to_vec_pretty(plan).unwrap_or_default()).map_err(|e| e.to_string())
    }
    /// Les plans clos, dans l'ordre.
    pub fn closed(&self) -> Vec<Plan> {
        let mut v: Vec<Plan> = std::fs::read_dir(self.dir.join("plans"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|e| std::fs::read(e.path()).ok())
            .filter_map(|o| serde_json::from_slice(&o).ok())
            .collect();
        v.sort_by_key(|p| p.number);
        v
    }
    /// Le rang du prochain plan : après tous ceux qui existent, vivants ou clos.
    pub fn next_number(&self) -> u32 {
        let vivants = self.read_plans().plans.iter().map(|p| p.number).max().unwrap_or(0);
        let clos = self.closed().last().map(|p| p.number).unwrap_or(0);
        vivants.max(clos) + 1
    }
    /// Combien de lignes de la boîte l'équipier a déjà lues (gardé entre deux
    /// lancements : une relance ne relit pas les anciennes).
    pub fn read_cursor(&self) -> usize {
        std::fs::read_to_string(self.dir.join("mailbox.read")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
    }
    /// **La trace de la demande** (forme B de c7, 15 novembre 2026) : le rang de la demande dont
    /// l'équipier a laissé quelque chose hors de la boîte (un fichier écrit, une commande, un plan
    /// clos). Sur disque, à côté du curseur : un redémarrage de l'hôte ne la perd pas.
    pub fn read_trace(&self) -> Option<usize> {
        std::fs::read_to_string(self.dir.join("request.trace")).ok().and_then(|s| s.trim().parse().ok())
    }
    pub fn write_trace(&self, demande: usize) {
        let _ = std::fs::write(self.dir.join("request.trace"), demande.to_string());
    }
    pub fn write_cursor(&self, n: usize) {
        let _ = std::fs::write(self.dir.join("mailbox.read"), n.to_string());
    }
    /// Les lignes de la boîte, toutes, chacune avec son `id` : une ligne écrite
    /// avant les identifiants (ou par un autre écrivain) reçoit celui de son rang.
    pub fn mailbox(&self) -> Vec<Value> {
        std::fs::read_to_string(self.mailbox_path())
            .unwrap_or_default()
            .lines()
            .enumerate()
            .filter_map(|(n, l)| {
                let mut v: Value = serde_json::from_str(l).ok()?;
                if v.get("id").is_none() && v.is_object() {
                    v["id"] = json!(format!("{}:{n}", self.key));
                }
                Some(v)
            })
            .collect()
    }
    /// Ajoute une ligne à la boîte ; rend la ligne écrite.
    pub fn append(&self, from: &str, kind: &str, item: Option<&str>, text: &str) -> Result<Value, String> {
        self.append_with(from, kind, item, text, &Value::Null)
    }
    /// Ajoute une ligne avec des champs de plus (`to`, `reply_to`, `approved`,
    /// les champs d'un `step_done`…), sans jamais remplacer les siens. Son `id`
    /// est `<parent>/<task>:<n>`, `n` son rang dans le fichier : la boîte ne fait
    /// qu'ajouter, le rang ne bouge pas.
    pub fn append_with(&self, from: &str, kind: &str, item: Option<&str>, text: &str, extra: &Value) -> Result<Value, String> {
        use std::io::Write;
        let _seul = MAILBOX_WRITE.lock().unwrap_or_else(|p| p.into_inner());
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let rang = std::fs::read_to_string(self.mailbox_path()).map(|t| t.lines().count()).unwrap_or(0);
        let mut line = json!({ "id": format!("{}:{rang}", self.key), "at": now_ms(), "from": from, "kind": kind, "text": text });
        if let Some(i) = item {
            line["item"] = json!(i);
        }
        if from == "teammate" {
            if let Some(nom) = &self.nom {
                line["name"] = json!(nom);
            }
        }
        if let Some(extra) = extra.as_object() {
            for (k, v) in extra {
                if line.get(k).is_none() && !v.is_null() {
                    line[k] = v.clone();
                }
            }
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(self.mailbox_path()).map_err(|e| e.to_string())?;
        writeln!(f, "{line}").map_err(|e| e.to_string())?;
        drop(_seul);
        if let Some(r) = &self.relay {
            (r.0)(&line);
        }
        Ok(line)
    }
    /// Le superviseur : l'auteur de la première `request` de la boîte.
    pub fn supervisor(&self) -> Option<String> {
        self.mailbox().iter().find(|l| l["kind"] == "request").and_then(|l| l["from"].as_str()).map(str::to_string)
    }
    fn seen_path(&self) -> PathBuf {
        self.dir.join("pending.seen.json")
    }
    /// Ce que `who` a déjà vu (`POST /ide/teammate/seen`).
    pub fn seen_by(&self, who: &str) -> std::collections::BTreeSet<String> {
        let tous: Value = std::fs::read(self.seen_path()).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null);
        tous[who].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_string)).collect()
    }
    /// Marque des lignes vues par `who` ; rend combien étaient neuves.
    pub fn mark_seen(&self, who: &str, ids: &[String]) -> Result<usize, String> {
        let _seul = MAILBOX_WRITE.lock().unwrap_or_else(|p| p.into_inner());
        let mut tous: Value = std::fs::read(self.seen_path()).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_else(|| json!({}));
        let mut vus: std::collections::BTreeSet<String> = tous[who].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_string)).collect();
        let avant = vus.len();
        vus.extend(ids.iter().filter(|i| i.starts_with(&format!("{}:", self.key))).cloned());
        let neufs = vus.len() - avant;
        tous[who] = json!(vus);
        std::fs::write(self.seen_path(), serde_json::to_vec_pretty(&tous).unwrap_or_default()).map_err(|e| e.to_string())?;
        Ok(neufs)
    }
    /// **Ce qui attend `who`** dans cette boîte : les lignes qui lui sont
    /// adressées (`to`), ou, sans `to`, celles de l'équipier et de l'hôte quand
    /// `who` est le superviseur. Une question, une approbation, une proposition
    /// attendent jusqu'à une ligne qui porte `reply_to` (ou, pour une ligne
    /// d'avant les identifiants, un `answer` sur le même `item`) ; un compte
    /// rendu, une fin, une livraison ratée attendent d'être vus. Lecture pure.
    pub fn pending_for(&self, who: &str) -> Vec<Value> {
        let lignes = self.mailbox();
        let superviseur = self.supervisor();
        let vus = self.seen_by(who);
        lignes
            .iter()
            .enumerate()
            .filter(|(_, l)| match l["to"].as_str() {
                Some(to) => to == who,
                None => matches!(l["from"].as_str(), Some("teammate" | "host")) && superviseur.as_deref() == Some(who),
            })
            .filter(|(_, l)| l["from"].as_str() != Some(who))
            .filter(|(n, l)| {
                let kind = l["kind"].as_str().unwrap_or_default();
                let id = l["id"].as_str().unwrap_or_default();
                if AWAITS_REPLY.contains(&kind) {
                    !lignes[n + 1..].iter().any(|r| {
                        r["reply_to"].as_str() == Some(id) || (r["kind"] == "answer" && !l["item"].is_null() && r["item"] == l["item"] && r["from"] != "teammate")
                    })
                } else if AWAITS_SEEN.contains(&kind) {
                    !vus.contains(id) && !(kind == "delivery" && l["delivered"] == true)
                } else {
                    false
                }
            })
            .map(|(_, l)| l.clone())
            .collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// **Une ligne est-elle pour l'équipier ?** Sans `to`, ou adressée à
/// `teammate:<parent>/<task>` de cette tâche. Une ligne d'un agent à un autre
/// (`claude-code:…` → `codex:…`) rangée dans la boîte ne le concerne pas : elle
/// ne le lance pas et il ne la lit pas à sa frontière.
pub fn addressed_to_teammate(line: &Value, key: &str) -> bool {
    match line["to"].as_str() {
        None => true,
        Some(to) => to == format!("teammate:{key}"),
    }
}

/// **Une nouvelle que l'équipier n'a pas lue** : une ligne après son curseur,
/// d'un autre que lui et l'hôte, qui lui est adressée — un `stop` seul ne compte
/// pas (il n'y a rien à arrêter). La fin d'un lancement la regarde : un oui
/// posté pendant le dernier pas ne se perd pas (le lot des commandes l'a vu,
/// 8 novembre 2026 : le oui arrivait entre le pas et la fin, et ne relançait
/// rien, l'équipier étant encore en route).
/// **Un refus du fournisseur du modèle**, lu dans l'erreur du tour : `Some(true)`
/// pour un quota épuisé (il ne revient pas en attendant), `Some(false)` pour un
/// quota dépassé ou un service en panne (il revient), `None` pour un autre échec.
/// La forme vient de `LlmError::Unavailable` (`src/llm.rs`).
pub(crate) fn refus_du_fournisseur(e: &str) -> Option<bool> {
    if e.contains("llm indisponible : quota épuisé") {
        Some(true)
    } else if e.contains("llm indisponible : quota dépassé") || e.contains("llm indisponible : service en panne") {
        Some(false)
    } else {
        None
    }
}

/// L'attente avant de rejouer un pas refusé : `base` secondes, doublées à chaque
/// refus d'affilée, au plus dix fois `base`.
pub(crate) fn attente_du_fournisseur(base: u64, refus: u32) -> u64 {
    // La règle du client du modèle (`crate::llm::attente_de_reprise`), une seule.
    crate::llm::attente_de_reprise(std::time::Duration::from_secs(base), refus, std::time::Duration::from_secs(base.saturating_mul(10))).as_secs()
}

/// **La question ouverte de l'équipier** (8 novembre 2026, essai 4 de c7 : trois pas
/// d'attente marqués « raté » arrêtaient la tâche, et chacun rappelait le modèle) : sa
/// dernière `question` ou `approval` sans réponse (`reply_to`) et sans ligne qui lui soit
/// adressée après elle (une `request`, une `answer`, une `instruction`). Tant qu'elle est
/// là, l'équipier attend sans appeler le modèle. Lue dans la boîte, elle survit à un
/// redémarrage de l'hôte. Rend son rang dans la boîte et la ligne.
pub(crate) fn open_question(files: &TeammateFiles) -> Option<(usize, Value)> {
    let lignes = files.mailbox();
    let mut ouverte: Option<(usize, Value)> = None;
    for (n, l) in lignes.iter().enumerate() {
        if l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("question" | "approval")) {
            let repondue = lignes.iter().any(|r| r["reply_to"] == l["id"]);
            ouverte = (!repondue).then(|| (n, l.clone()));
        } else if l["from"] != "teammate" && l["from"] != "host" && l["kind"] != "stop" && addressed_to_teammate(l, &files.key) {
            ouverte = None;
        }
    }
    ouverte
}

/// **Ce que la request en cours exige avant la fin** (`expects`, déclaré par le superviseur,
/// 13 novembre 2026) : `report` (un report de l'équipier dans la boîte) ou `approval` (un
/// `answer` `approved: true` du superviseur). Rend la valeur et le rang de la request.
pub(crate) fn expected_deliverable(lignes: &[Value]) -> Option<(usize, String)> {
    let n = lignes.iter().rposition(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host")?;
    lignes[n]["expects"].as_str().map(|e| (n, e.to_string()))
}

/// Le livrable exigé par la request en cours, s'il manque encore : son nom, pour la boîte.
pub(crate) fn missing_deliverable(lignes: &[Value]) -> Option<&'static str> {
    let (n, expects) = expected_deliverable(lignes)?;
    let apres = &lignes[n + 1..];
    match expects.as_str() {
        "report" => (!apres.iter().any(|l| l["from"] == "teammate" && l["kind"] == "report")).then_some("un report dans la boîte"),
        "approval" => (!apres.iter().any(|l| l["from"] != "teammate" && l["from"] != "host" && l["kind"] == "answer" && l["approved"] == true)).then_some("l'accord du superviseur"),
        _ => None,
    }
}

/// Les valeurs permises de `expects` sur une request.
pub const EXPECTS: [&str; 3] = ["report", "approval", "none"];

/// **Une demande sans `expects` attend au moins une ligne de l'équipier** (ticket rouvert le 15
/// novembre 2026) : un `report`, une `question` ou une `approval` dans la boîte depuis elle (le
/// texte d'un pas, acheminé, arrive en `report`). Rend le rang de la demande quand rien n'y est ;
/// `None` quand une ligne y est, ou quand la demande déclare `expects` (sa garde est la sienne,
/// `none` compris). La boucle ne relance que si l'équipier n'a rien laissé d'autre non plus : ni
/// fichier écrit, ni commande, ni plan clos (forme B de c7).
/// **A path of the agent's tree, in what the user reads, becomes her folder's** (23 Nov 2026): she runs commands where
/// she is, never in the task's tree.
pub(crate) fn in_her_folder(folders: Option<&(String, String)>, text: &str) -> String {
    match folders {
        Some((tree, folder)) if !tree.is_empty() && tree != folder => text.replace(tree.as_str(), folder),
        _ => text.to_string(),
    }
}

/// **The turn's opening line of a human, when the agent wrote nothing after it**: no report, question, approval,
/// message nor answer of the agent (its step text is copied as a report, so a step that wrote prose counts).
pub(crate) fn silent_turn(lignes: &[Value]) -> Option<usize> {
    let n = lignes.iter().rposition(|l| {
        matches!(l["kind"].as_str(), Some("request" | "instruction")) && l["from"] != "teammate" && l["from"] != "host" && l["by"]["kind"] == "human"
    })?;
    let ligne = lignes[n + 1..].iter().any(|l| l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("report" | "question" | "approval" | "message" | "answer")));
    (!ligne).then_some(n)
}

pub(crate) fn request_without_a_line(lignes: &[Value]) -> Option<usize> {
    let n = lignes.iter().rposition(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host")?;
    if !lignes[n]["expects"].is_null() {
        return None;
    }
    // A message sent to a session (mode pair) is the teammate's line too: the answer to a peer.
    let ligne = lignes[n + 1..].iter().any(|l| l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("report" | "question" | "approval" | "message")));
    (!ligne).then_some(n)
}

/// **Un outil qui laisse une trace** hors de la boîte : il écrit les fichiers, lance une commande
/// ou attache un worktree, d'après la politique que son manifeste déclare. Un outil inconnu du
/// manifeste compte comme une trace : on ne relance pas sur un doute.
pub(crate) fn tool_leaves_a_trace(prepared: Option<&crate::backend::PreparedBackend>, name: &str) -> bool {
    let Some(p) = prepared else { return true };
    match p.manifest.tools.get(name) {
        Some(a) => a.policy.write_files || a.policy.run_commands || a.policy.attach_worktrees,
        None => true,
    }
}

pub(crate) fn unread_news(files: &TeammateFiles) -> bool {
    files
        .mailbox()
        .iter()
        .skip(files.read_cursor())
        .any(|l| l["from"] != "teammate" && l["from"] != "host" && l["kind"] != "stop" && addressed_to_teammate(l, &files.key))
}

/// Le sujet du bus de la boîte d'une tâche.
pub fn mailbox_topic(task: &str) -> String {
    format!("mailbox.{task}")
}

/// **Ce qu'un pas sait de lui-même** : les outils de l'équipier s'en servent,
/// la boucle le lit après le tour.
pub struct Step {
    /// **The agent's tree and the user's folder** (23 Nov 2026, `~/essai-2`: the agent told her to `cd` into its own
    /// tree): a path of the tree in what she reads becomes her folder's.
    pub folders: Option<(String, String)>,
    pub files: TeammateFiles,
    pub task: String,
    /// La demande d'origine (la première `request` de la boîte) : le nom de la
    /// fiche de session, pour le juge comme pour les plans clos.
    pub request: String,
    pub bus: Option<crate::events::EventBus>,
    pub number: usize,
    calls: AtomicUsize,
    /// The step's bounds (S1, 18 Nov 2026), from `teammate.*` in the manifest.
    max_tool_calls: usize,
    context_tokens: usize,
    max_model_calls: usize,
    /// The tokens of the last model call of the step (prompt and completion), from
    /// `generation_end`.
    last_context: std::sync::Arc<AtomicUsize>,
    /// The model's window, as its last call said it (`generation_end`'s `context_window`); 0 when unknown.
    context_window: std::sync::Arc<AtomicUsize>,
    /// The share of that window a step may fill (`teammate.step_context_share`).
    context_share: f64,
    /// **The step wrote text to the user** (its reply's tokens, seen as they stream).
    wrote_text: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The step_dones refused for want of a word, and whether the step ended without one.
    /// **Delivered up to this line of the mailbox** (Lucie's trial of 23 Nov 2026): what arrives during the step comes
    /// at the head of its next tool result; the next frontier still recalls it, with the rest of the step's news.
    delivered: AtomicUsize,
    /// The loop's texts, for what is delivered during the step.
    texts: Option<super::prompts::Texts>,
    /// The graph asked at each `step_done` (`templates/teammate/graphs/at_step_done.mmd`), and how many times each of
    /// its refusing verdicts refused in this step.
    at_step_done: Option<std::sync::Arc<super::step_graphs::StepGraph>>,
    verdict_refusals: std::sync::Mutex<std::collections::BTreeMap<String, usize>>,
    /// The step called a tool of work (not step_done nor report): a text must follow its last one.
    used_tools: std::sync::atomic::AtomicBool,
    /// The step wrote text at some point (kept across its tool calls): the rule of the silent turn reads it.
    wrote_any: std::sync::atomic::AtomicBool,
    answered_nothing: std::sync::atomic::AtomicBool,
    reply_first: String,
    /// Ce que `step_done` a déclaré.
    done: Mutex<Option<Value>>,
    /// The pair entry of the task, and its key (mode pair, lot 3).
    pair: Option<(Arc<super::pair::Pair>, String)>,
    /// The task's background commands: their scope and their mailbox (they outlive the step).
    background: Option<Arc<TaskBackground>>,
    /// The browser family (`teammate.browser`): its policy, and its tools or why the host does not listen.
    browser: Option<(crate::backend::BrowserPolicy, Result<super::browser_tools::BrowserTools, String>)>,
    /// The gate of the auto mode, when the task's backend is in it (page 07 of rag3weaver-archi).
    #[cfg(feature = "code")]
    pub gate: Option<super::gate::StepGate>,
    /// The `judge_all` gate: a path outside the sandbox's roots (page 07 of rag3weaver-archi, « suite »).
    #[cfg(feature = "code")]
    pub open_root: Option<super::gate::StepOpenRoot>,
    /// The task's gate level (`bac`, `judge`, `cards`), when it has one (22 Nov 2026).
    #[cfg(feature = "code")]
    pub gate_level: Option<String>,
    /// The relaunch of a cut reply: its text and bound (`teammate.cut_relaunch_*`).
    cut_relaunch: (String, usize),
    /// What the teammate may read on the web (`teammate.fetch`).
    fetch: crate::backend::FetchPolicy,
    /// The output limit of the step's model calls.
    max_output_tokens: usize,
    /// The bounds of the task's background commands (`teammate.background`).
    background_policy: crate::backend::BackgroundPolicy,
    /// The teammate's search by Gemini's grounding (`teammate.web_search`).
    web_search: crate::backend::WebSearchPolicy,
    /// Les plans clos pendant ce pas (à ranger en L1).
    closed: Mutex<Vec<Plan>>,
    /// L'agent a-t-il tranché sur le plan actif (continue, pause, close) ?
    decided: Mutex<Option<String>>,
    /// Par où l'équipier s'est réveillé avant ce pas (`bus`, `fichier`), pour le journal.
    pub woke_by: Option<String>,
    /// Les outils de travail aboutis pendant ce pas (sans erreur), par leur nom.
    succeeded_tools: Mutex<Vec<String>>,
    /// The paths the sandbox refused during the step, with their count.
    refused_paths: Mutex<std::collections::HashMap<String, usize>>,
    /// The path refused twice, once there is one: the step's work stops.
    refused_twice: Mutex<Option<String>>,
    /// What the step did, for the next frontier (S2): `read <path> (lines a-b)`, `wrote <path>`,
    /// and the last command with the end of its output.
    work: Mutex<StepWork>,
    /// The run of the step's turn and its model's name, noted when the turn starts: a refused step
    /// says on that run that it waits for the model (`Waiting`, 19 November 2026).
    turn_run: Mutex<Option<(String, String)>>,
}

/// What a step did with its work tools, kept for the next frontier (S2, 18 Nov 2026).
#[derive(Default)]
struct StepWork {
    read: Vec<String>,
    wrote: Vec<String>,
    command: Option<(String, String)>,
    /// **What the step's tools returned** (24 Nov 2026: a new step wrote names the step before had found otherwise):
    /// the tool's name and its result, bounded, for the declared `Previous step` to keep the facts it chooses.
    results: Vec<(String, String)>,
}

impl StepWork {
    /// **What a step did, as the input of a scripted `Previous step`** (`teammate.carry_over_node`): the same state
    /// [`Self::block`] reads, plus the step's number, its last text and the budget.
    fn to_json(&self, number: usize, last_text: &str, budget: usize) -> Value {
        serde_json::json!({
            "number": number,
            "read": self.read,
            "wrote": self.wrote,
            "command": self.command.as_ref().map(|(c, o)| serde_json::json!({ "command": c, "output": o })),
            "results": self.results.iter().map(|(t, r)| serde_json::json!({ "tool": t, "result": r })).collect::<Vec<_>>(),
            "last_text": last_text,
            "budget": budget,
        })
    }
}

/// **The `Previous step` block by a declared scripted node** (Lucie, 20 November 2026: « valider le scripting, qu'il
/// fonctionne bien, et qu'il est pratique là où il est placé »). The node named `type_name` is looked for among the
/// `*.node.json` of `nodes_dir` and **read again at each call**: an edited `.ts` serves at the next step, without
/// restarting the host. Its input port `work` takes [`StepWork::to_json`]; its output port `block` is the block. An
/// error names the script's file and line: the caller falls back to the Rust block and says it.
fn carry_over_by_node(nodes_dir: &std::path::Path, type_name: &str, config: &Value, work: Value) -> Result<String, String> {
    use crate::dataflow::node_registry::NodeFactory;
    use crate::dataflow::scripted_node::{read_declaration, ScriptedNodeFactory};
    // Looked for by its type name: a broken neighbour in the same folder does not stop it.
    let mut chemins: Vec<std::path::PathBuf> = std::fs::read_dir(nodes_dir)
        .map_err(|e| format!("{} : {e}", nodes_dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".node.json"))
        .collect();
    chemins.sort();
    let (chemin, decl) = chemins
        .into_iter()
        .find_map(|p| read_declaration(&p).ok().filter(|d| d.name == type_name).map(|d| (p, d)))
        .ok_or_else(|| format!("aucun nœud scripté « {type_name} » sous {}", nodes_dir.display()))?;
    // The error names the script's file: the engine gives its line, not its file.
    let script = std::fs::read_to_string(&chemin)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["script"].as_str().map(|s| nodes_dir.join(s)))
        .unwrap_or_else(|| chemin.clone());
    let nomme = |e: String| format!("{} : {e}", script.display());
    // The node's own errors name the script's file and its line (`ScriptedNodeDecl::script_file`).
    let fabrique = ScriptedNodeFactory::new(decl, crate::script::ScriptLimits::default())?;
    let mut noeud = fabrique.create("carry_over", config).map_err(nomme)?;
    let mut ctx = crate::dataflow::node::NodeContext::new();
    ctx.set_input("work", crate::dataflow::port::PortValue::new(work));
    noeud.execute(&mut ctx)?;
    ctx.drain_outputs()
        .get("block")
        .and_then(|v| v.downcast::<Value>())
        .and_then(|v| v.as_str().map(str::to_string))
        .ok_or_else(|| nomme(format!("{type_name} : le port « block » n'a pas rendu de texte")))
}

/// The tools that write a file, by their name.
const WRITING_TOOLS: [&str; 5] = ["write", "replace_text", "replace_lines", "insert_lines", "edit_file"];

impl StepWork {
    fn note(&mut self, tool: &str, args: &Value, result: &str) {
        // Every result, bounded, for the declared `Previous step` to choose its facts from (24 Nov 2026).
        const RESULTS_KEPT: usize = 30;
        const RESULT_CHARS: usize = 4_000;
        self.results.push((tool.to_string(), result.chars().take(RESULT_CHARS).collect()));
        if self.results.len() > RESULTS_KEPT {
            self.results.remove(0);
        }
        let path = args["path"].as_str().unwrap_or_default();
        if tool == "read_file" && !path.is_empty() {
            // The range the tool says it rendered (« — lines 1-3 of 3 »), else the one asked.
            let rendered = result.split(" — lines ").nth(1).and_then(|r| r.split(|c: char| c != '-' && !c.is_ascii_digit()).next()).filter(|r| r.contains('-')).map(str::to_string);
            let range = rendered.unwrap_or_else(|| {
                let from = args["offset"].as_u64().unwrap_or(1);
                match args["limit"].as_u64().or(args["length"].as_u64()) {
                    Some(n) => format!("{from}-{}", from + n.max(1) - 1),
                    None => format!("{from}-end"),
                }
            });
            let line = format!("{path} (lines {range})");
            if !self.read.contains(&line) {
                self.read.push(line);
            }
        } else if WRITING_TOOLS.contains(&tool) && !path.is_empty() {
            if !self.wrote.iter().any(|p| p == path) {
                self.wrote.push(path.to_string());
            }
        } else if tool == "run_command" {
            let command = args["command"].as_str().unwrap_or_default().to_string();
            let tail: String = { let c: Vec<char> = result.chars().collect(); c[c.len().saturating_sub(1500)..].iter().collect() };
            self.command = Some((command, tail));
        }
    }

    fn block(&self, number: usize, last_text: &str, budget: usize) -> String {
        if self.read.is_empty() && self.wrote.is_empty() && self.command.is_none() && last_text.trim().is_empty() {
            return String::new();
        }
        let mut b = format!("\n## Previous step (step {number})\n");
        if !self.read.is_empty() {
            b.push_str(&format!("Read: {}\n", self.read.join(", ")));
        }
        if !self.wrote.is_empty() {
            b.push_str(&format!("Wrote: {}\n", self.wrote.join(", ")));
        }
        if let Some((c, out)) = &self.command {
            b.push_str(&format!("Last command: {c}\nIts output ends with:\n{out}\n"));
        }
        if !last_text.trim().is_empty() {
            b.push_str(&format!("Your last text: {}\n", last_text.trim()));
        }
        b.push_str("Lines you read in that step can be replaced now without reading them again.\n");
        if b.chars().count() > budget {
            let cut: String = b.chars().take(budget.saturating_sub(80)).collect();
            b = format!("{cut}\n[Previous step cut at {budget} characters: teammate.carry_over_chars]\n");
        }
        b
    }
}

impl Step {
    /// The paths the step's code tools wrote (23 Nov 2026: what the host's end-of-step commit takes).
    #[cfg(feature = "code")]
    pub(crate) fn wrote(&self) -> Vec<String> {
        self.work.lock().map(|w| w.wrote.clone()).unwrap_or_default()
    }

    pub fn with_folders(mut self, folders: Option<(String, String)>) -> Self {
        self.folders = folders;
        self
    }

    pub fn new(files: TeammateFiles, task: &str, request: &str, bus: Option<crate::events::EventBus>, number: usize) -> Self {
        Self {
            folders: None,
            files,
            task: task.to_string(),
            request: request.to_string(),
            bus,
            number,
            calls: AtomicUsize::new(0),
            max_tool_calls: crate::backend::TeammatePolicy::default().max_tool_calls_per_step,
            context_tokens: crate::backend::TeammatePolicy::default().step_context_tokens,
            max_model_calls: crate::backend::TeammatePolicy::default().max_model_calls_per_step,
            last_context: std::sync::Arc::new(AtomicUsize::new(0)),
            context_window: std::sync::Arc::new(AtomicUsize::new(0)),
            context_share: crate::backend::TeammatePolicy::default().step_context_share,
            wrote_text: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            delivered: AtomicUsize::new(0),
            texts: None,
            at_step_done: None,
            verdict_refusals: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            used_tools: std::sync::atomic::AtomicBool::new(false),
            wrote_any: std::sync::atomic::AtomicBool::new(false),
            answered_nothing: std::sync::atomic::AtomicBool::new(false),
            reply_first: crate::backend::TeammatePolicy::default().reply_first,
            done: Mutex::new(None),
            closed: Mutex::new(Vec::new()),
            decided: Mutex::new(None),
            woke_by: None,
            succeeded_tools: Mutex::new(Vec::new()),
            refused_paths: Mutex::new(std::collections::HashMap::new()),
            refused_twice: Mutex::new(None),
            work: Mutex::new(StepWork::default()),
            turn_run: Mutex::new(None),
            pair: None,
            background: None,
            browser: None,
            #[cfg(feature = "code")]
            gate: None,
            #[cfg(feature = "code")]
            open_root: None,
            #[cfg(feature = "code")]
            gate_level: None,
            cut_relaunch: (crate::backend::TeammatePolicy::default().cut_relaunch_text, crate::backend::TeammatePolicy::default().cut_relaunches_per_step),
            fetch: crate::backend::FetchPolicy::default(),
            max_output_tokens: crate::backend::TeammatePolicy::default().max_output_tokens,
            background_policy: crate::backend::BackgroundPolicy::default(),
            web_search: crate::backend::WebSearchPolicy::default(),
        }
    }

    /// Notes the run of the step's turn and its model's name.
    pub fn note_turn_run(&self, run: &str, model: &str) {
        *self.turn_run.lock().unwrap_or_else(|p| p.into_inner()) = Some((run.to_string(), model.to_string()));
    }

    /// The run of the step's turn and its model's name, once the turn has started.
    pub fn turn_run(&self) -> Option<(String, String)> {
        self.turn_run.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// The pair entry this step's task holds (mode pair): it gets send_to_session.
    /// The browser family of the step (`teammate.browser`).
    pub fn with_browser(mut self, browser: Option<(crate::backend::BrowserPolicy, Result<super::browser_tools::BrowserTools, String>)>) -> Self {
        self.browser = browser;
        self
    }

    /// The task's background commands (their scope and their mailbox).
    pub fn with_background(mut self, background: Option<Arc<TaskBackground>>) -> Self {
        self.background = background;
        self
    }

    pub fn with_pair(mut self, pair: Option<(Arc<super::pair::Pair>, String)>) -> Self {
        self.pair = pair;
        self
    }

    /// The relaunch of a cut reply, for the step's chat turn.
    pub fn cut_relaunch(&self) -> Option<(String, usize)> {
        (self.cut_relaunch.1 > 0).then(|| self.cut_relaunch.clone())
    }

    /// The step's bounds, from the backend's `teammate` policy.
    pub fn with_policy(mut self, policy: &crate::backend::TeammatePolicy) -> Self {
        self.max_tool_calls = policy.max_tool_calls_per_step.max(1);
        self.context_tokens = policy.step_context_tokens.max(1);
        self.context_share = policy.step_context_share;
        self.max_model_calls = policy.max_model_calls_per_step.clamp(1, 100);
        self.cut_relaunch = (policy.cut_relaunch_text.clone(), policy.cut_relaunches_per_step.min(10));
        self.fetch = policy.fetch.clone();
        self.web_search = policy.web_search.clone();
        self.background_policy = policy.background.clone();
        self.max_output_tokens = policy.max_output_tokens.max(256);
        self.reply_first = policy.reply_first.clone();
        self
    }

    /// The refusal of a silent step_done, from the user's own template file when she has one (read at each step).
    /// **What the mailbox received before the step**: read by its frontier; what comes after is delivered during it.
    /// The graph asked at each `step_done`.
    pub(crate) fn with_at_step_done(mut self, graph: Option<std::sync::Arc<super::step_graphs::StepGraph>>) -> Self {
        self.at_step_done = graph;
        self
    }

    /// **The facts of the step**, as the step's graphs read them (`at_step_done.mmd`, `end_of_step.mmd`): whether it
    /// wrote after its last tool and in the step, whether it used tools, the `step_done` being called, the active plan,
    /// and the turn's mailbox lines since the last `ended`. A fact the step does not know is absent: it reads false.
    pub(crate) fn step_facts(&self, step_done: Option<&Value>) -> Value {
        let boite = self.files.mailbox();
        let debut = boite.iter().rposition(|l| l["kind"] == "ended").map_or(0, |n| n + 1);
        let turn: Vec<Value> = boite[debut..]
            .iter()
            .map(|l| json!({ "from": l["from"], "kind": l["kind"], "by": l["by"], "id": l["id"], "reply_to": l["reply_to"], "expects": l["expects"] }))
            .collect();
        let plans = self.files.read_plans();
        let plan = plans.active().map(|p| json!({ "items": p.items.iter().map(|i| json!({ "id": i.id, "status": i.status })).collect::<Vec<_>>() }));
        let since_tool = self.wrote_text.load(Ordering::SeqCst);
        json!({
            "text_since_last_tool": since_tool,
            "text_in_step": since_tool || self.wrote_any.load(Ordering::SeqCst),
            "used_tools": self.used_tools.load(Ordering::SeqCst),
            "step_done": step_done.map(|a| json!({ "item": a["item"], "succeeded": a["succeeded"], "remaining": a["remaining"] })),
            "plan": plan,
            "turn": turn,
        })
    }

    pub(crate) fn with_delivery(mut self, read: usize, texts: super::prompts::Texts) -> Self {
        self.delivered = AtomicUsize::new(read);
        self.texts = Some(texts);
        self
    }

    /// **The news of the mailbox, at the head of a tool result** (Lucie, 23 Nov 2026: « comme les sorties d'outils
    /// asynchrones »): the lines addressed to the teammate since the last delivery, up to a stop (the loop's), said by
    /// the loop's texts; a step_done or a report delivers nothing (the step ends there, the next frontier says them).
    fn deliver(&self, call: &ToolCall, mut turn: Turn) -> Turn {
        let Some(t) = self.texts.as_ref().filter(|_| !matches!(call.name.as_str(), STEP_DONE | REPORT)) else { return turn };
        let lignes = self.files.mailbox();
        let depuis = self.delivered.load(Ordering::SeqCst);
        if lignes.len() <= depuis {
            return turn;
        }
        let fin = lignes[depuis..].iter().position(|l| l["kind"] == "stop" && l["from"] != "teammate" && l["from"] != "host").map_or(lignes.len(), |n| depuis + n);
        let neuves: Vec<&Value> = lignes[depuis..fin].iter().filter(|l| l["from"] != "teammate" && l["from"] != "host" && addressed_to_teammate(l, &self.files.key)).collect();
        self.delivered.store(fin, Ordering::SeqCst);
        if neuves.is_empty() {
            return turn;
        }
        let mut tete = format!("{}
", t.get("mid_turn.head", &[]));
        for l in neuves {
            tete.push_str(&format!("{}
", t.get(
                "mid_turn.line",
                &[
                    ("kind", l["kind"].as_str().unwrap_or("?")),
                    ("from", l["from"].as_str().unwrap_or("?")),
                    ("text", l["text"].as_str().unwrap_or("")),
                    // A card's answer brings the host's grant with it.
                    ("granted", &l["granted"].as_str().map(|g| format!(" — {g}")).unwrap_or_default()),
                ],
            )));
        }
        turn.content = format!("{tete}
{}", turn.content);
        turn
    }

    /// The head of the teammate's prompt (`teammate.reply_first`).
    pub fn reply_first(&self) -> &str {
        &self.reply_first
    }

    /// **A tool call of the step**: a tool of work resets « a text was written », so that a text must follow it.
    pub fn tool_of_work(&self, name: &str) {
        if !matches!(name, STEP_DONE | REPORT) {
            self.used_tools.store(true, Ordering::SeqCst);
            if self.wrote_text.swap(false, Ordering::SeqCst) {
                self.wrote_any.store(true, Ordering::SeqCst);
            }
        }
    }

    /// Set when the step's reply streams a token of text ([`Self::wrote_text`]).
    pub fn text_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.wrote_text.clone()
    }

    /// The step ended on a step_done accepted after two refusals, without a word to the user.
    pub fn answered_nothing(&self) -> bool {
        self.answered_nothing.load(Ordering::SeqCst)
    }

    /// The output limit of the step's model calls (`teammate.max_output_tokens`).
    pub fn max_output_tokens(&self) -> usize {
        self.max_output_tokens
    }

    /// The `max_iterations` of the step's chat turn.
    pub fn max_model_calls(&self) -> usize {
        self.max_model_calls
    }

    /// Where the turn's event handler records the tokens of each model call of the step.
    pub fn context_meter(&self) -> ContextMeter {
        ContextMeter { used: self.last_context.clone(), window: self.context_window.clone() }
    }
}

/// Records the tokens of a model call (prompt and completion) from a `generation_end` event
/// of a step's turn.
pub fn record_context(meter: &ContextMeter, ev: &Value) {
    if ev["event"] == "generation_end" {
        let tokens = ev["prompt_tokens"].as_u64().unwrap_or(0) + ev["completion_tokens"].as_u64().unwrap_or(0);
        if tokens > 0 {
            meter.used.store(tokens as usize, Ordering::SeqCst);
        }
        if let Some(w) = ev["context_window"].as_u64().filter(|&w| w > 0) {
            meter.window.store(w as usize, Ordering::SeqCst);
        }
    }
}

/// **What a step's model calls tell of its context** (24 Nov 2026): the tokens of the last call, and the model's window.
#[derive(Clone)]
pub struct ContextMeter {
    used: std::sync::Arc<AtomicUsize>,
    window: std::sync::Arc<AtomicUsize>,
}

impl Step {

    /// Note un outil de travail abouti : son rendu n'est pas une erreur.
    fn note_tool(&self, call: &ToolCall, turn: &Turn) {
        let v: Value = serde_json::from_str(&turn.content).unwrap_or(Value::Null);
        let echec = v["ok"] == false || !v["error"].is_null();
        // A command's output, or a tool's error: never a file's content that quotes the words.
        if call.name == "run_command" || echec {
            self.note_a_refused_path(&turn.content);
        }
        if !echec {
            self.succeeded_tools.lock().unwrap_or_else(|p| p.into_inner()).push(call.name.clone());
            let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
            self.work.lock().unwrap_or_else(|p| p.into_inner()).note(&call.name, &args, &turn.content);
        }
    }

    /// **A path the sandbox refuses twice stops the step's work** (scenario 2 of tty, the night of
    /// 20 Nov 2026: `index.lock` refused, the teammate went round its rights). The second refusal of
    /// the same path is said in the mailbox, and every work tool after it is refused, asking for
    /// step_done.
    fn note_a_refused_path(&self, rendu: &str) {
        let Some(chemin) = refused_path(rendu) else { return };
        let fois = {
            let mut refus = self.refused_paths.lock().unwrap_or_else(|p| p.into_inner());
            let n = refus.entry(chemin.clone()).or_insert(0);
            *n += 1;
            *n
        };
        if fois == 2 {
            *self.refused_twice.lock().unwrap_or_else(|p| p.into_inner()) = Some(chemin.clone());
            if let Ok(line) = self.files.append("host", "report", None, &format!("the sandbox refused `{chemin}` twice: the step stops its work there, retrying will not change it")) {
                publish(self.bus.as_ref(), &self.task, &line);
            }
        }
    }

    /// The `Previous step` block of the next frontier: what this step read, wrote and ran, and
    /// its last text, within `budget` characters.
    pub fn carry_over(&self, last_text: &str, budget: usize) -> String {
        self.work.lock().unwrap_or_else(|p| p.into_inner()).block(self.number, last_text, budget)
    }

    /// [`Self::carry_over`], by the scripted node `node` (`nodes/` folder, type name, config) when one is declared:
    /// `(block, error)`. On an error the block is the Rust one, and the error says why.
    pub fn carry_over_with(&self, last_text: &str, budget: usize, node: Option<(&std::path::Path, &str, &Value)>) -> (String, Option<String>) {
        let work = self.work.lock().unwrap_or_else(|p| p.into_inner());
        let rust = || work.block(self.number, last_text, budget);
        match node {
            None => (rust(), None),
            Some((dir, nom, config)) => match carry_over_by_node(dir, nom, config, work.to_json(self.number, last_text, budget)) {
                Ok(b) => (b, None),
                Err(e) => (rust(), Some(e)),
            },
        }
    }

    /// Les outils de travail aboutis pendant ce pas.
    pub fn succeeded_tools(&self) -> Vec<String> {
        self.succeeded_tools.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn system_prompt(&self) -> &'static str {
        SYSTEM_PROMPT
    }
    /// The teammate's prompt, for a witness that has no step at hand.
    #[cfg(test)]
    pub fn prompt_for_witnesses() -> &'static str {
        SYSTEM_PROMPT
    }
    /// Les outils qui terminent ce pas.
    pub fn terminal_tools() -> Vec<String> {
        vec![STEP_DONE.to_string()]
    }
    pub fn declared(&self) -> Option<Value> {
        self.done.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn closed_plans(&self) -> Vec<Plan> {
        self.closed.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn decision(&self) -> Option<String> {
        self.decided.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn tool_calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// Écrit dans la boîte et le publie.
    fn post(&self, kind: &str, item: Option<&str>, text: &str) -> Result<Value, String> {
        let text = in_her_folder(self.folders.as_ref(), text);
        let line = self.files.append("teammate", kind, item, &text)?;
        publish(self.bus.as_ref(), &self.task, &line);
        Ok(line)
    }

    fn decide(&self, what: &str) {
        *self.decided.lock().unwrap_or_else(|p| p.into_inner()) = Some(what.to_string());
    }

    /// **`edit_plan`**, dans l'ordre : `continue` / `pause` / `close` / `resume`
    /// sur les plans vivants, puis `plan` (un plan neuf, actif), puis `item` et
    /// `remove` sur le plan actif. Chaque question neuve part dans la boîte.
    fn edit_plan(&self, args: &Value) -> Result<Value, String> {
        let mut plans = self.files.read_plans();
        let mut questions: Vec<(String, String)> = Vec::new();
        let mut closed_now: Option<u32> = None;

        if args["continue"].as_bool() == Some(true) {
            plans.active().ok_or("continue : il n'y a pas de plan actif")?;
            self.decide("continue");
        }
        if let Some(p) = args.get("pause").filter(|p| p.is_object()) {
            let a = plans.active_mut().ok_or("pause : il n'y a pas de plan actif")?;
            a.state = PlanState::Paused;
            a.reason = Some(p["reason"].as_str().unwrap_or_default().to_string());
            self.decide("pause");
        }
        if let Some(c) = args.get("close").filter(|c| c.is_object()) {
            let outcome = match c["outcome"].as_str() {
                Some("accomplished") => PlanState::Accomplished,
                Some("obsolete") => PlanState::Obsolete,
                _ => return Err("close.outcome est accomplished ou obsolete".into()),
            };
            // Le plan nommé, sinon l'actif.
            let k = match c["number"].as_u64() {
                Some(n) => plans.plans.iter().position(|p| u64::from(p.number) == n).ok_or_else(|| format!("close : pas de plan {n} vivant"))?,
                None => plans.plans.iter().position(|p| p.state == PlanState::Active).ok_or("close : il n'y a pas de plan actif ; nomme le plan (number) pour clore un plan en pause")?,
            };
            let mut p = plans.plans.remove(k);
            p.state = outcome;
            p.reason = Some(c["reason"].as_str().unwrap_or_default().to_string());
            if let Some(cs) = c["concepts"].as_array() {
                p.concepts = cs.iter().filter_map(|c| c.as_str().map(str::to_string)).collect();
            }
            self.files.keep_closed(&p)?;
            closed_now = Some(p.number);
            self.closed.lock().unwrap_or_else(|q| q.into_inner()).push(p);
            self.decide("close");
        }
        if let Some(n) = args["resume"].as_u64() {
            if plans.active().is_some() {
                return Err("resume : un plan est déjà actif ; mets-le en pause ou clos-le d'abord".into());
            }
            let p = plans.plans.iter_mut().find(|p| u64::from(p.number) == n).ok_or_else(|| format!("resume : pas de plan {n} en pause"))?;
            p.state = PlanState::Active;
            p.reason = None;
            self.decide("resume");
        }
        if let Some(p) = args.get("plan").filter(|p| p.is_object()) {
            if plans.active().is_some() {
                return Err("un plan est déjà actif : continue-le (item), mets-le en pause (pause) ou clos-le (close) avant d'en ouvrir un autre".into());
            }
            let neuf = self.parse_plan(p, self.files.next_number().max(plans.plans.iter().map(|p| p.number).max().unwrap_or(0) + 1))?;
            questions.extend(neuf.items.iter().filter(|i| !i.alone || i.status == Status::Blocked).filter_map(|i| i.question.clone().map(|q| (i.id.clone(), q))));
            plans.plans.push(neuf);
        }
        if let Some(changes) = args.get("item").filter(|i| i.is_object()) {
            let p = plans.active_mut().ok_or("item : il n'y a pas de plan actif ; pose-le d'abord (plan) ou reprends-en un (resume)")?;
            let id = changes["id"].as_str().ok_or("item.id manque")?;
            let i = match p.items.iter_mut().position(|i| i.id == id) {
                Some(k) => &mut p.items[k],
                None => {
                    p.items.push(Item { id: id.into(), text: String::new(), status: Status::Todo, alone: true, question: None, reason: None });
                    p.items.last_mut().expect("poussé")
                }
            };
            if let Some(t) = changes["text"].as_str() {
                i.text = t.into();
            }
            if let Some(s) = changes.get("status").filter(|s| !s.is_null()) {
                i.status = serde_json::from_value(s.clone()).map_err(|_| format!("statut inconnu : {s} (todo, doing, done, blocked, dropped)"))?;
            }
            if let Some(a) = changes["alone"].as_bool() {
                i.alone = a;
            }
            if let Some(r) = changes["reason"].as_str() {
                i.reason = Some(r.into());
            }
            if let Some(q) = changes["question"].as_str() {
                if i.question.as_deref() != Some(q) {
                    questions.push((i.id.clone(), q.into()));
                }
                i.question = Some(q.into());
            }
        }
        if let Some(id) = args["remove"].as_str() {
            let p = plans.active_mut().ok_or("remove : il n'y a pas de plan actif")?;
            p.items.retain(|i| i.id != id);
        }
        self.files.write_plans(&plans)?;
        // **Les bloquants d'abord** : chaque question neuve part dans la boîte.
        for (id, q) in &questions {
            self.post("question", Some(id), q)?;
        }
        let mut r = json!({ "ok": true, "active": plans.active(), "paused": plans.paused().iter().map(|p| json!({ "number": p.number, "request": p.request, "reason": p.reason })).collect::<Vec<_>>(), "questions_sent": questions.len() });
        if let Some(n) = closed_now {
            r["closed"] = json!(n);
        }
        Ok(r)
    }

    fn parse_plan(&self, p: &Value, number: u32) -> Result<Plan, String> {
        let items: Vec<Item> = serde_json::from_value(p["items"].clone()).map_err(|e| format!("plan.items : {e}"))?;
        Ok(Plan {
            number,
            request: p["request"].as_str().unwrap_or_default().into(),
            proposal: p["proposal"].as_str().unwrap_or_default().into(),
            items,
            state: PlanState::Active,
            reason: None,
            steps: Vec::new(),
            concepts: Vec::new(),
        })
    }

    fn step_done(&self, args: &Value) -> Result<Value, String> {
        // **The declared decision first** (`at_step_done.mmd`, 23 Nov 2026): a verdict whose table says `refuse` sends
        // its text back to the model, numbered, up to its `max_refusals`; then the call is accepted, said answered
        // nothing. A graph that fails is said, and the step goes on as if it accepted.
        if let Some(graph) = &self.at_step_done {
            match graph.decide(&self.step_facts(Some(args))) {
                Ok((verdict, _why, a)) if a.action == "refuse" => {
                    let n = {
                        let mut counts = self.verdict_refusals.lock().unwrap_or_else(|p| p.into_inner());
                        let c = counts.entry(verdict).or_insert(0);
                        *c += 1;
                        *c - 1
                    };
                    if n < a.max_refusals {
                        let key = a.text_key.unwrap_or_default();
                        let text = self.texts.clone().unwrap_or_else(super::prompts::Texts::shipped).get(&key, &[]);
                        return Err(format!("{text} ({}/{})", n + 1, a.max_refusals));
                    }
                    self.answered_nothing.store(true, Ordering::SeqCst);
                }
                Ok(_) => {}
                Err(e) => eprintln!("[équipier] at_step_done : {e} ; l'appel est accepté"),
            }
        }
        // The two rules of a silent step_done (a turn with no word to the user; a closing step_done right after tools)
        // live in `templates/teammate/graphs/at_step_done.mmd` and its table, asked just above.
        let item = args["item"].as_str().unwrap_or_default();
        let succeeded = args["succeeded"].as_bool().ok_or("succeeded manque (true ou false)")?;
        let done = args["done"].as_str().unwrap_or_default();
        let remaining = args["remaining"].as_str().unwrap_or_default();
        if !item.is_empty() {
            let mut plans = self.files.read_plans();
            if let Some(i) = plans.active_mut().and_then(|p| p.items.iter_mut().find(|i| i.id == item)) {
                if succeeded {
                    i.status = Status::Done;
                    i.reason = None;
                } else if !remaining.is_empty() {
                    i.reason = Some(remaining.into());
                }
                self.files.write_plans(&plans)?;
            }
        }
        let text = if succeeded { format!("fait : {done}") } else { format!("raté : {done} — reste : {remaining}") };
        let champs = json!({ "succeeded": succeeded, "done": done, "remaining": remaining });
        let line = self.files.append_with("teammate", "step_done", (!item.is_empty()).then_some(item), &text, &champs)?;
        publish(self.bus.as_ref(), &self.task, &line);
        *self.done.lock().unwrap_or_else(|p| p.into_inner()) = Some(json!({ "item": item, "succeeded": succeeded, "done": done, "remaining": remaining }));
        Ok(json!({ "ok": true, "step": self.number, "item": item, "succeeded": succeeded }))
    }

    fn ours(&self, call: &ToolCall) -> Option<Turn> {
        let reply = |v: Result<Value, String>| {
            let v = v.unwrap_or_else(|e| json!({ "ok": false, "error": e }));
            Some(Turn::tool_result(&call.id, &call.name, v.to_string()))
        };
        let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
        match call.name.as_str() {
            EDIT_PLAN => {
                if self.gate(call).is_some() {
                    return self.refusal(call);
                }
                reply(self.edit_plan(&args))
            }
            STEP_DONE => reply(self.step_done(&args)),
            REPORT => {
                let text = args["text"].as_str().unwrap_or_default();
                if text.trim().is_empty() {
                    return reply(Err("report.text est vide".into()));
                }
                reply(self.post("report", None, text).map(|_| json!({ "ok": true })))
            }
            SEND_TO_SESSION => {
                let Some((pair, cle)) = &self.pair else {
                    return reply(Err("send_to_session: this task is not paired (teammate.pair)".into()));
                };
                let (to, text) = (args["to"].as_str().unwrap_or_default(), args["text"].as_str().unwrap_or_default());
                if to.trim().is_empty() || text.trim().is_empty() {
                    return reply(Err("send_to_session: to and text are required".into()));
                }
                reply(pair.envoyer(cle, to, text).and_then(|msg_id| {
                    let line = self.files.append_with("teammate", "message", None, text, &json!({ "direction": "out", "to": to, "msg_id": msg_id }))?;
                    publish(self.bus.as_ref(), &self.task, &line);
                    Ok(json!({ "ok": true, "msg_id": msg_id }))
                }))
            }
            // **What comes from outside is marked as observed** (tty, 21 November 2026): data, never instructions.
            FETCH => reply(super::web_fetch::fetch(&self.fetch, &args).map(|v| {
                let source = v["url"].as_str().or(args["url"].as_str()).unwrap_or("fetch").to_string();
                super::observed::mark(v, &source)
            })),
            name @ (super::browser_tools::BROWSER_TABS | super::browser_tools::BROWSER_READ | super::browser_tools::BROWSER_CONSOLE) if self.browser.is_some() => {
                reply(match self.browser.as_ref().map(|(_, t)| t) {
                    Some(Ok(t)) => t.call(name, &args).unwrap_or_else(|| Err(format!("{name} : pas de la famille du navigateur"))).map(|v| {
                        let source = v["url"].as_str().map_or_else(|| format!("{name}: the browser"), str::to_string);
                        super::observed::mark(v, &source)
                    }),
                    Some(Err(e)) => Err(format!("l'hôte n'écoute pas le navigateur : {e}")),
                    None => Err("teammate.browser n'est pas déclaré".into()),
                })
            }
            WEB_SEARCH => reply(super::web_search::web_search(&self.web_search, &args).map(|v| super::observed::mark(v, &format!("web_search: {}", args["query"].as_str().unwrap_or_default())))),
            SUGGEST => {
                let text = args["text"].as_str().unwrap_or_default();
                if text.trim().is_empty() {
                    return reply(Err("suggest.text est vide".into()));
                }
                reply(self.post("proposal", args["item"].as_str(), text).map(|_| json!({ "ok": true, "note": "proposé ; continue le plan courant, la réponse viendra à une frontière" })))
            }
            _ => None,
        }
    }

    /// The refusal of a work tool past a bound of the step, `None` under them.
    fn refusal(&self, call: &ToolCall) -> Option<Turn> {
        let context = self.last_context.load(Ordering::SeqCst);
        // The bound follows the model's window when it is known: the larger of the fixed bound and its declared share.
        let window = self.context_window.load(Ordering::SeqCst);
        let bound = self.context_tokens.max((window as f64 * self.context_share) as usize);
        let why = if context > bound {
            format!(
                "the context of the step is full ({context} tokens; the bound is {bound}: teammate.step_context_tokens = {}, teammate.step_context_share = {} of a {window}-token window)",
                self.context_tokens, self.context_share
            )
        } else if let Some(chemin) = self.refused_twice.lock().unwrap_or_else(|p| p.into_inner()).clone() {
            format!("the sandbox refused `{chemin}` twice in this step, and retrying will not change it: do not look for another way to the same path")
        } else if self.calls.load(Ordering::SeqCst) > self.max_tool_calls {
            format!("the step reached its tool calls ({}, teammate.max_tool_calls_per_step)", self.max_tool_calls)
        } else {
            return None;
        };
        Some(Turn::tool_result(
            &call.id,
            &call.name,
            json!({ "ok": false, "error": format!("{why}: declare step_done now (succeeded or not, what is done, what remains); the next step resumes from there") }).to_string(),
        ))
    }

    /// **The bounds of the step** (S1, 18 Nov 2026): its context, read on the last model
    /// call, and a count of work tool calls far above a real step. Past either, a work tool is
    /// refused; the refusal is a tool result, and the agent reads it.
    fn gate(&self, call: &ToolCall) -> Option<Turn> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.refusal(call)
    }

    /// send_to_session, given to a paired task's teammate only (mode pair, lot 3).
    pub fn send_to_session_def() -> ToolDef {
        ToolDef {
            name: SEND_TO_SESSION.into(),
            description: "Write to another agent session (Claude Code or a peer) in the sessions registry. `to` is its name, or the address a received line carries (reply_to_address, « uds:… »). A request whose text is a <cross-session-message from-name=… > came from such a session: answer it with this tool, to that session, not with report. The message is kept in your mailbox.".into(),
            parameters: json!({ "type": "object", "properties": { "to": { "type": "string", "description": "the session's name, or a uds: address" }, "text": { "type": "string", "description": "the message itself, as the session will read it" } }, "required": ["to", "text"] }),
        }
    }

    /// fetch, the teammate's page reader (the teammate's base, 20 November 2026).
    pub fn fetch_def() -> ToolDef {
        ToolDef {
            name: FETCH.into(),
            description: "Read a web page: its readable text (no script, no style, no markup), its HTTP status, its type, its final address and the redirects followed. The text is cut at max_bytes, and the cut is said. A local address (this machine, the local network) is refused unless teammate.fetch.allow_local says otherwise; an HTTP error is said with the page's own text.".into(),
            parameters: json!({ "type": "object", "properties": {
                "url": { "type": "string", "description": "the address to read, with its scheme (example: « https://docs.rs/ureq/latest/ureq/ »)" },
                "max_bytes": { "type": "integer", "description": "the most text bytes to bring back; default and ceiling: teammate.fetch.max_bytes (100 000)" }
            }, "required": ["url"] }),
        }
    }

    /// web_search, the teammate's search by Gemini's grounding on Vertex (the teammate's base, 20 November 2026).
    pub fn web_search_def() -> ToolDef {
        ToolDef {
            name: WEB_SEARCH.into(),
            description: "Search the web: Gemini answers the query with Google Search grounding, on Vertex, and gives the queries it ran and its sources — each with its title, its address, the passages of the answer it supports and its provenance. The sources are bounded (teammate.web_search.max_sources), and the cut is said. Read a source in full with fetch. Refused, with the reason, when the search model is not Gemini on Vertex.".into(),
            parameters: json!({ "type": "object", "properties": {
                "query": { "type": "string", "description": "the question or the words to search for, as you would type them (example: « what did Rust 1.98 stabilise? »)" }
            }, "required": ["query"] }),
        }
    }

    pub fn tool_defs() -> Vec<ToolDef> {
        let item = json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "l'identifiant court de l'élément, stable dans le plan (exemple : « lire-la-porte »)" },
                "text": { "type": "string", "description": "ce que l'élément fait, en une phrase" },
                "status": { "type": "string", "enum": ["todo", "doing", "done", "blocked", "dropped"], "description": "l'état de l'élément ; todo à sa création" },
                "alone": { "type": "boolean", "description": "true si tu peux le faire sans validation du superviseur" },
                "question": { "type": "string", "description": "la question posée au superviseur, si l'élément demande une validation" },
                "reason": { "type": "string", "description": "pourquoi l'élément est bloqué ou abandonné ; vide sinon" }
            },
            "required": ["id"]
        });
        vec![
            ToolDef {
                name: EDIT_PLAN.into(),
                description: "Tes mini-plans. `plan` en ouvre un (request, proposal, items) quand aucun n'est actif ; `item` change ou ajoute un élément du plan actif ; `remove` en retire un ; `continue` dit qu'une demande nouvelle rejoint le plan actif ; `pause` {reason} le met en pause ; `resume` <number> en reprend un ; `close` {outcome accomplished|obsolete, reason, number?, concepts?} le clôt. Une question posée part dans la boîte du superviseur.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "plan": {
                            "type": "object",
                            "description": "ouvre un plan, quand aucun n'est actif",
                            "properties": {
                                "request": { "type": "string", "description": "la demande telle que tu la comprends" },
                                "proposal": { "type": "string", "description": "ce que tu proposes" },
                                "items": { "type": "array", "items": item.clone(), "description": "les éléments du plan, dans l'ordre où tu les feras" }
                            },
                            "required": ["request", "proposal", "items"]
                        },
                        "item": { "description": "change ou ajoute un élément du plan actif, par son id", "type": "object", "properties": item["properties"].clone(), "required": item["required"].clone() },
                        "remove": { "type": "string", "description": "l'id d'un élément à retirer" },
                        "continue": { "type": "boolean", "description": "true : la demande nouvelle rejoint le plan actif, ou attend qu'il finisse" },
                        "pause": { "type": "object", "description": "met le plan actif en pause, pour une demande nouvelle", "properties": { "reason": { "type": "string", "description": "pourquoi le plan s'interrompt" } }, "required": ["reason"] },
                        "resume": { "type": "integer", "description": "le numéro d'un plan en pause" },
                        "close": {
                            "type": "object",
                            "description": "clôt un plan, accompli ou obsolète",
                            "properties": {
                                "outcome": { "type": "string", "enum": ["accomplished", "obsolete"], "description": "accomplished quand il est fait, obsolete quand il n'a plus lieu d'être" },
                                "reason": { "type": "string", "description": "pourquoi il se clôt, en une phrase" },
                                "number": { "type": "integer", "description": "le plan à clore ; sans lui, l'actif" },
                                "concepts": { "type": "array", "items": { "type": "string" }, "description": "les notions que ce plan a touchées, en mots (une indication pour la mémoire)" }
                            },
                            "required": ["outcome", "reason"]
                        }
                    }
                }),
            },
            ToolDef {
                name: STEP_DONE.into(),
                description: "Finit le pas : l'élément travaillé (vide sans plan), s'il a réussi, ce qui est fait, ce qui reste. Réussi, l'élément passe à done. Le pas s'arrête ici ; le suivant commence par la boîte et les plans.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "item": { "type": "string", "description": "l'id de l'élément travaillé ; vide (\"\") sans plan" },
                        "succeeded": { "type": "boolean", "description": "true si le pas a fait ce qu'il visait ; l'élément passe alors à done" },
                        "done": { "type": "string", "description": "ce que le pas a fait, en une phrase (exemple : « le test rouge passe, la porte refuse sccache »)" },
                        "remaining": { "type": "string", "description": "ce qui reste ; vide (\"\") quand rien ne reste, sinon un pas suivant part" }
                    },
                    "required": ["item", "succeeded", "done", "remaining"]
                }),
            },
            ToolDef {
                name: SUGGEST.into(),
                description: "Propose au superviseur une direction hors du plan (« voudrais-tu qu'on pousse dans ce sens ? »), avec l'id de l'élément qu'elle ouvrirait. Tu continues le plan courant sans attendre ; tu n'ouvres pas cette direction sans sa réponse.".into(),
                parameters: json!({ "type": "object", "properties": { "text": { "type": "string", "description": "la direction proposée, en une ou deux phrases" }, "item": { "type": "string", "description": "l'id de l'élément que la proposition ouvrirait" } }, "required": ["text"] }),
            },
            ToolDef {
                name: REPORT.into(),
                description: "Écrit au superviseur, dans sa boîte. À une demande de texte (une présentation, une explication, un résumé), c'est le texte lui-même, tel qu'il est à lire, jamais une phrase qui le décrit ; après un travail d'outils, ce qui a été fait, ce qui attend, ce que tu as décidé.".into(),
                parameters: json!({ "type": "object", "properties": { "text": { "type": "string", "description": "le texte à lire par le superviseur, tel quel : titres, listes et gras gardés" } }, "required": ["text"] }),
            },
        ]
    }
}

/// **Les outils d'un pas** : ceux du chat dans la tâche, plus `edit_plan`,
/// `step_done` et `report`, sous les bornes du pas.
pub struct StepTools<'a> {
    pub inner: &'a (dyn crate::agent::ToolBox + Sync),
    pub step: &'a Step,
}

impl StepTools<'_> {
    /// **Une commande mise en attente devient une question dans la boîte**
    /// (page 07-novembre-2026-everything-declarative/01, § 1, lot 2) : le
    /// résultat de `run_command` qui commence par la marque de l'attente pose
    /// une ligne `approval` (la commande, telle que l'agent l'a écrite) ; le
    /// pas continue sans elle, et un « oui » (`answer`, `approved: true`, en
    /// réponse à cette ligne) l'accorde pour la suite de la tâche.
    /// **A background command belongs to the task, not to the step** (Lucie, 21 November 2026): the backend's tools
    /// reach the graph through `Backend::call`, which sees no invocation of the loop; for a background `run_command`,
    /// the task's (its scope, its bus and mailbox, whose ends wake the teammate) is set for the call's thread.
    ///
    /// **Its bounds** (Lucie, 22 November 2026, `teammate.background`): at most `max` at once in the task; no
    /// `timeout_s` is the default time (10 minutes); none above `max_timeout_s`; « never » (`timeout_s: 0`) is for a
    /// service, not a command. A refusal is said before anything runs.
    fn in_task(&self, call: &ToolCall, handle: Option<&str>, f: impl FnOnce(&ToolCall) -> Turn) -> Turn {
        let mut args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
        let (Some(bg), true) = (&self.step.background, call.name == "run_command" && args["background"] == true) else {
            return f(call);
        };
        let policy = &self.step.background_policy;
        let refuse = |why: String| Turn::tool_result(&call.id, &call.name, json!({ "ok": false, "error": why }).to_string());
        match args["timeout_s"].as_u64() {
            Some(0) => return refuse("timeout_s: 0 (jamais) n'est permis qu'à un service, pas à une commande de fond : donne un délai, ou lance un service (start_service)".into()),
            Some(t) if t > policy.max_timeout_s => {
                return refuse(format!("timeout_s: {t} dépasse le plafond de {} s (teammate.background.max_timeout_s) : donne-en moins, ou lance un service", policy.max_timeout_s));
            }
            Some(_) => {}
            None => args["timeout_s"] = json!(policy.default_timeout_s),
        }
        let running = crate::dataflow::background::registry().alive().iter().filter(|a| a.kind == crate::dataflow::background::Kind::Command && a.caller.as_deref() == Some(bg.inbox.as_str())).count();
        if running >= policy.max {
            return refuse(format!("{running} commande(s) de fond tournent déjà dans cette tâche, la borne est {} (teammate.background.max) : attends la fin d'une, elle te sera dite", policy.max));
        }
        let call = ToolCall { arguments: args.to_string(), ..call.clone() };
        let task_inv = Some(crate::backend::TaskInvocation {
            handle: handle.map_or_else(|| format!("#run_command-{}", self.step.number), str::to_string),
            bus: bg.bus.clone(),
            inbox: bg.inbox.clone(),
            scope: bg.scope.clone(),
        });
        crate::backend::with_task_invocation(task_inv, || f(&call))
    }

    #[cfg(feature = "code")]
    fn after(&self, call: &ToolCall, mut turn: Turn, rejouer: &dyn Fn(&ToolCall) -> Turn) -> Turn {
        // **At `bac`, a line the parser cannot reduce runs through `sh -c`** (c7, 23 Nov 2026): the parser refuses a
        // redirection, a substitution, an expansion or a glob; the sandbox bounds what the line reaches, and the gate
        // lets the shell through at this level. Said in the result.
        if call.name == "run_command" && self.step.gate_level.as_deref() == Some("bac") && turn.content.contains("**Commande refusée.**") {
            let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
            // Once: a wrapped line (its own form) is not wrapped again, and a refusal of it stays a refusal.
            const ENVELOPPE: &str = "sh -c 'eval \"$(printf %s ";
            if let (Some(ligne), false) = (args["command"].as_str().filter(|l| !l.starts_with(ENVELOPPE)), args["background"] == true) {
                use codeparsers::shell::Refus;
                if matches!(codeparsers::shell::decomposer(ligne), Err(Refus::Redirection | Refus::Substitution | Refus::Expansion | Refus::Joker)) {
                    // The parser refuses a `*` or a `?` even quoted: the line goes encoded (base64 has neither), and the
                    // result shows it as written.
                    use base64::Engine as _;
                    let code = base64::engine::general_purpose::STANDARD.encode(ligne);
                    let par_sh = format!("{ENVELOPPE}{code} | base64 -d)\"'");
                    let mut a = args.clone();
                    a["command"] = json!(par_sh);
                    let par_le_shell = ToolCall { arguments: a.to_string(), ..call.clone() };
                    let mut t = self.after(&par_le_shell, rejouer(&par_le_shell), rejouer);
                    t.content = format!(
                        "_À la porte `bac`, la ligne passe par `sh -c` : le parseur ne réduit pas ce qu'elle contient ; le bac borne ce qu'elle touche._\n\n{}",
                        t.content.replace(&par_sh, &format!("sh -c '{ligne}'"))
                    );
                    return t;
                }
            }
        }
        // **judge_all** (page 07, « suite »): the sandbox stopped a path outside its roots; the judge's yes grants the
        // root and the command is replayed now, anything else is a card whose yes grants it.
        if let (Some(m), Some(gate)) = (open_root_request(&turn.content), &self.step.open_root) {
            {
                match gate(&m) {
                    super::gate::OpenRootOutcome::Granted => return rejouer(call),
                    // **At `bac`, a path outside the tree is refused with a sentence** (c7, 23 Nov 2026, 06:11): no card,
                    // no suspended turn; the agent reads why and goes on.
                    super::gate::OpenRootOutcome::Card { args, .. } if self.step.gate_level.as_deref() == Some("bac") => {
                        turn.content = format!(
                            "**Hors de ton projet** : `{}` n'est pas dans l'arbre de la tâche, et la porte `bac` ne l'ouvre pas. Reste dans ton projet ; si tu en as vraiment besoin, dis-le dans ton compte rendu.",
                            args["root"].as_str().unwrap_or_default()
                        );
                        return turn;
                    }
                    super::gate::OpenRootOutcome::Card { class, args } => {
                        let ouverte = super::card::open_card(&self.step.files, "run_command", &args, &class, "once")
                            .and_then(|id| self.step.files.mailbox().into_iter().find(|l| l["id"] == id.as_str()).ok_or_else(|| "the card's line".to_string()));
                        if let Ok(line) = ouverte {
                            publish(self.step.bus.as_ref(), &self.step.task, &line);
                            turn.content.push_str(&format!(
                                "\n\n_La demande d'accès à {} est dans la boîte du superviseur (ligne {}) : continue sans elle ; si elle est accordée, relance ta commande mot pour mot._",
                                args["root"].as_str().unwrap_or_default(),
                                line["id"].as_str().unwrap_or_default()
                            ));
                        }
                        return turn;
                    }
                }
            }
        }
        if turn.content.contains(crate::dataflow::run_nodes::EN_ATTENTE) {
            let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
            if let Some(ligne) = args["command"].as_str() {
                // **The gate of the auto mode** (page 07): a safe class or the judge's yes grants the line, and it
                // is replayed now; anything else is the card, with the action's class.
                let mut class = None;
                if let Some(gate) = &self.step.gate {
                    match gate(ligne) {
                        super::gate::GateOutcome::Pass { .. } => return rejouer(call),
                        super::gate::GateOutcome::Card { class: c } => class = Some(c),
                    }
                }
                let ouverte = match &class {
                    Some(c) => super::card::open_card(&self.step.files, "run_command", &args, c, "once")
                        .and_then(|id| self.step.files.mailbox().into_iter().find(|l| l["id"] == id.as_str()).ok_or_else(|| "the card's line".to_string())),
                    None => self.step.files.append_with("teammate", "approval", None, &format!("commande en attente d'accord : {ligne}"), &json!({ "command": ligne })),
                };
                if let Ok(line) = ouverte {
                    publish(self.step.bus.as_ref(), &self.step.task, &line);
                    turn.content.push_str(&format!(
                        "\n\n_La demande est dans la boîte du superviseur (ligne {}) : continue sans cette commande ; sa réponse arrivera à une frontière, et un accord la laissera passer telle quelle._",
                        line["id"].as_str().unwrap_or_default()
                    ));
                }
            }
        }
        turn
    }
    #[cfg(not(feature = "code"))]
    fn after(&self, _call: &ToolCall, turn: Turn, _rejouer: &dyn Fn(&ToolCall) -> Turn) -> Turn {
        turn
    }
}

/// The sandbox's `open_root` request in a tool's result: the backend's tools return the node's error as
/// `{"error": "open_root: {…}"}`, its quotes escaped; a direct text carries it as is.
#[cfg(feature = "code")]
fn open_root_request(content: &str) -> Option<Value> {
    let marque = crate::dataflow::run_nodes::OPEN_ROOT_MARKER;
    let texte = serde_json::from_str::<Value>(content).ok().and_then(|v| v["error"].as_str().map(str::to_string)).unwrap_or_else(|| content.to_string());
    let i = texte.find(marque)?;
    serde_json::Deserializer::from_str(&texte[i + marque.len()..]).into_iter::<Value>().next()?.ok()
}

impl StepTools<'_> {
    /// Le rendu d'un outil de travail, noté au pas (aboutis seulement).
    fn noted(&self, call: &ToolCall, turn: Turn) -> Turn {
        self.step.note_tool(call, &turn);
        // A command that failed is kept for the turn's facts (23 Nov 2026).
        #[cfg(feature = "code")]
        if call.name == "run_command" {
            super::land::note_turn_failure(&self.step.files, &turn.content);
        }
        turn
    }
}

impl crate::agent::ToolBox for StepTools<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        let rejouer = |c: &ToolCall| self.in_task(c, None, |c| self.inner.call(c));
        self.step.tool_of_work(&call.name);
        let turn = self.step.ours(call).or_else(|| self.step.gate(call)).unwrap_or_else(|| self.noted(call, self.after(call, rejouer(call), &rejouer)));
        self.step.deliver(call, turn)
    }
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        let rejouer = |c: &ToolCall| self.in_task(c, None, |c| self.inner.call_in(c, run));
        self.step.tool_of_work(&call.name);
        let turn = self.step.ours(call).or_else(|| self.step.gate(call)).unwrap_or_else(|| self.noted(call, self.after(call, rejouer(call), &rejouer)));
        self.step.deliver(call, turn)
    }
    fn call_with(&self, call: &ToolCall, run: &str, inv: &crate::agent::ToolInvocation) -> Turn {
        // A replay (the gate's yes, a granted root, the bac's shell line) goes the way of the call itself: with its
        // invocation, so a background command stays in the task's background scope (23 Nov 2026).
        let rejouer = |c: &ToolCall| self.in_task(c, Some(&inv.handle), |c| self.inner.call_with(c, run, inv));
        self.step.tool_of_work(&call.name);
        let turn = self.step.ours(call).or_else(|| self.step.gate(call)).unwrap_or_else(|| self.noted(call, self.after(call, rejouer(call), &rejouer)));
        self.step.deliver(call, turn)
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        // Les nôtres gagnent sur un homonyme du backend : un fournisseur refuse
        // deux déclarations du même nom (Vertex : « Duplicate function declaration »).
        let mut ours = Step::tool_defs();
        ours.push(Step::fetch_def());
        ours.push(Step::web_search_def());
        ours.extend(super::browser_tools::family(self.step.browser.as_ref().map(|(p, _)| p)));
        if self.step.pair.is_some() {
            ours.push(Step::send_to_session_def());
        }
        let mut defs: Vec<ToolDef> = self.inner.tool_defs().into_iter().filter(|d| !ours.iter().any(|o| o.name == d.name)).collect();
        defs.extend(ours);
        defs
    }
    fn is_async(&self, tool: &str) -> bool {
        self.inner.is_async(tool)
    }
}

/// Publie une ligne de la boîte sur `mailbox.<task>` (et `messages`).
fn publish(bus: Option<&crate::events::EventBus>, task: &str, line: &Value) {
    if let Some(bus) = bus {
        let event = crate::events::Event::Message {
            run: format!("mailbox-{task}"),
            from: line["from"].as_str().unwrap_or_default().to_string(),
            to: format!("teammate:{task}"),
            content: line.to_string(),
            conversation: mailbox_topic(task),
        };
        bus.emit_on(&mailbox_topic(task), event.clone());
        bus.emit(event);
    }
}

/// **Ce que la frontière d'un pas dit à l'agent** : la boîte depuis le pas
/// précédent, le plan actif, les plans en pause (une ligne chacun, toujours),
/// et ce qu'on attend de ce pas.
/// **Le message d'un pas, et lui seul** (8 novembre 2026, la mesure du quota : l'entrée
/// montait de 5 275 jetons au pas 1 à 122 242 au pas 10, le fil entier renvoyé à chaque
/// pas). Un pas repart d'une session neuve : ce message porte la demande en cours (la
/// dernière reçue), ce que les pas faits depuis elle ont déclaré (`step_done`), la boîte
/// depuis le pas précédent et les plans.
/// **The earlier turns of a conversation**, oldest first: the user's requests and the teammate's answers (its step
/// reports, its reports, the host's end of each turn); the last eight in full (2 000 characters at most), the older
/// ones cut at 200, forty at most. `None` before a first turn.
fn conversation_before(lines: &[Value]) -> Option<String> {
    let mut tenus: Vec<String> = Vec::new();
    for l in lines {
        let (from, kind, text) = (l["from"].as_str().unwrap_or_default(), l["kind"].as_str().unwrap_or_default(), l["text"].as_str().unwrap_or_default());
        let qui = match (from, kind) {
            ("teammate", "step_done" | "report" | "question") => "teammate",
            ("host", "ended" | "turn_changes" | "merged") => "host",
            (f, "request") if f != "teammate" && f != "host" => "user",
            _ => continue,
        };
        if !text.trim().is_empty() {
            tenus.push(format!("{qui}: {}", text.trim()));
        }
    }
    if tenus.is_empty() {
        return None;
    }
    let debut = tenus.len().saturating_sub(40);
    let n = tenus.len();
    let lignes: Vec<String> = tenus[debut..]
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let max = if debut + i + 8 >= n { 2000 } else { 200 };
            if t.chars().count() > max { format!("{}…", t.chars().take(max).collect::<String>()) } else { t.clone() }
        })
        .collect();
    Some(format!("Earlier in this conversation (oldest first):\n- {}", lignes.join("\n- ")))
}

fn frontier_message(step: usize, news: &[Value], plans: &Plans, last: bool, request: &str, faits: &[Value], t: &super::prompts::Texts) -> String {
    let mut m = format!("{}\n", t.get("frontier.step", &[("step", &step.to_string())]));
    if !request.is_empty() && !news.iter().any(|l| l["kind"] == "request" && l["text"] == request) {
        m.push_str(&format!("\n{}\n", t.get("frontier.request", &[("request", request)])));
    }
    if !faits.is_empty() {
        m.push_str(&format!("\n{}\n", t.get("frontier.facts_head", &[])));
        let debut = faits.len().saturating_sub(MAX_FAITS_RAPPELES);
        if debut > 0 {
            m.push_str(&format!("{}\n", t.get("frontier.facts_older", &[("n", &debut.to_string())])));
        }
        for f in &faits[debut..] {
            let issue = if f["succeeded"] == true { t.get("frontier.fact_succeeded", &[]) } else { t.get("frontier.fact_failed", &[]) };
            let item = f["item"].as_str().filter(|i| !i.is_empty()).map(|i| t.get("frontier.fact_item", &[("item", i)])).unwrap_or_default();
            let reste = f["remaining"].as_str().filter(|r| !r.is_empty()).map(|r| t.get("frontier.fact_left", &[("left", r)])).unwrap_or_default();
            let done = f["done"].as_str().unwrap_or("");
            m.push_str(&format!("{}\n", t.get("frontier.fact", &[("issue", &issue), ("item", &item), ("done", done), ("left", &reste)])));
        }
    }
    if !news.is_empty() {
        m.push_str(&format!("\n{}\n", t.get("frontier.mailbox_head", &[])));
        for l in news {
            let item = l["item"].as_str().map(|i| t.get("frontier.fact_item", &[("item", i)])).unwrap_or_default();
            let accord = l["granted"].as_str().map(|g| format!(" — {g}")).unwrap_or_default();
            let line = t.get(
                "frontier.mailbox_line",
                &[("kind", l["kind"].as_str().unwrap_or("?")), ("from", l["from"].as_str().unwrap_or("?")), ("item", &item), ("text", l["text"].as_str().unwrap_or("")), ("granted", &accord)],
            );
            m.push_str(&format!("{line}\n"));
        }
    }
    let active = plans.active();
    let paused = plans.paused();
    if let Some(p) = active {
        m.push_str(&format!("\n{}\n```json\n{}\n```\n", t.get("frontier.active_plan", &[("n", &p.number.to_string())]), serde_json::to_string_pretty(p).unwrap_or_default()));
    }
    if !paused.is_empty() {
        m.push_str(&format!("\n{}\n", t.get("frontier.paused_head", &[])));
        for p in &paused {
            let line = t.get("frontier.paused_line", &[("n", &p.number.to_string()), ("request", &p.request), ("reason", p.reason.as_deref().unwrap_or(""))]);
            m.push_str(&format!("{line}\n"));
        }
    }
    let new_request = news.iter().any(|l| l["kind"] == "request" || l["kind"] == "instruction");
    m.push('\n');
    let consigne = match (active, new_request, last) {
        (Some(_), true, _) => "frontier.new_request_during_plan",
        (Some(_), false, true) if !paused.is_empty() => "frontier.nothing_left_with_paused",
        (Some(_), false, true) => "frontier.nothing_left",
        (Some(_), false, false) => "frontier.work_an_item",
        (None, _, _) if !paused.is_empty() && !new_request => "frontier.resume_or_close_paused",
        (None, _, _) => "frontier.answer_or_act",
    };
    m.push_str(&format!("{}\n", t.get(consigne, &[("user", "the user")])));
    m
}

/// Ce que la boucle rend de son travail.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Outcome {
    pub steps: Vec<Value>,
    pub ended: String,
}

/// **The background commands of a task** (Lucie, 21 November 2026): a bus and a mailbox of their own, on which their
/// ends arrive, and a scope that outlives the teammate's steps (emptied when the task closes).
pub struct TaskBackground {
    pub bus: crate::events::EventBus,
    pub inbox: String,
    pub scope: Arc<crate::agent::RunScope>,
}

impl DaemonHost {
    /// **The task's background**, made at its first use. Each end that arrives on its mailbox is posted to the task's
    /// mailbox (`from: background`, a `message`), which wakes the teammate. `None` when the host does not know itself
    /// (`se_connaitre`): a background command then stays tied to its step, as before.
    pub(crate) fn task_background(&self, parent: &str, task: &str) -> Option<Arc<TaskBackground>> {
        let soi = self.soi.get().cloned()?;
        let key = format!("{parent}/{task}");
        let mut all = self.task_backgrounds.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(b) = all.get(&key) {
            return Some(b.clone());
        }
        let bus = crate::events::EventBus::new(64);
        let inbox = format!("teammate-background:{key}");
        let mut rx = bus.subscribe(&crate::events::inbox_topic(&inbox));
        let (parent, task) = (parent.to_string(), task.to_string());
        let spawned = std::thread::Builder::new().name(format!("fond:{key}")).spawn(move || loop {
            match rx.try_recv() {
                Ok(crate::events::Event::Message { content, .. }) => {
                    let Some(h) = soi.upgrade() else { return };
                    if let Err(e) = h.post_to_teammate(&parent, &task, "background", "message", None, &content, &Value::Null) {
                        eprintln!("[fond] {parent}/{task} : la fin d'une commande de fond n'a pas rejoint la boîte : {e}");
                    }
                }
                Ok(_) => {}
                Err(async_broadcast::TryRecvError::Empty) => {
                    if soi.strong_count() == 0 {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                // Ends lost to an overflow are said by `tail`; the thread keeps reading.
                Err(async_broadcast::TryRecvError::Overflowed(_)) => {}
                Err(async_broadcast::TryRecvError::Closed) => return,
            }
        });
        if let Err(e) = spawned {
            eprintln!("[fond] {key} : le fil des fins ne part pas ({e}) : les commandes de fond restent liées au pas");
            return None;
        }
        let b = Arc::new(TaskBackground { bus, inbox, scope: Arc::new(crate::agent::RunScope::default()) });
        all.insert(key, b.clone());
        Some(b)
    }

    /// The task's background commands die with the task (closed, merged or rejected).
    pub(crate) fn end_task_background(&self, parent: &str, task: &str) {
        if let Some(b) = self.task_backgrounds.lock().unwrap_or_else(|p| p.into_inner()).remove(&format!("{parent}/{task}")) {
            b.scope.vider();
        }
    }
}

impl DaemonHost {
    /// **Les fichiers de l'équipier d'une tâche** : son dossier.
    pub fn teammate_files(&self, parent: &str, task: &str) -> Result<TeammateFiles, String> {
        let t = self.tache(parent, task).ok_or_else(|| format!("pas de tâche « {task} » ouverte sur {parent}"))?;
        let dir = t.arbre.parent().map(Path::to_path_buf).ok_or("la tâche n'a pas de dossier")?;
        let mut files = TeammateFiles::new(dir, format!("{parent}/{task}"));
        // Le nom de l'agent : celui de la tâche au registre, sinon `agent.name` de son manifeste, sinon
        // « équipier » (le défaut d'ED).
        files.nom = Some(t.nom_de_l_agent.clone().unwrap_or_else(|| {
            std::fs::read(&t.manifest)
                .ok()
                .and_then(|o| serde_json::from_slice::<Value>(&o).ok())
                .and_then(|m| m["agent"]["name"].as_str().map(str::to_string))
                .unwrap_or_else(|| "équipier".to_string())
        }));
        // Chaque ligne écrite part aussi au flux de l'IDE (sujet `mailbox`), en
        // objet : l'IDE, le terminal et le hook la voient sans sonder la boîte.
        // Le `backend` de la trame est le manifeste de la tâche (son worktree), comme
        // pour ses autres événements : un client filtre le flux d'une tâche par lui seul.
        let cle = t.agent_key.clone();
        let (flux, backend, p, t) = (Arc::downgrade(self.ide_flux()), t.manifest.display().to_string(), parent.to_string(), task.to_string());
        files.relay = Some(MailboxRelay(Arc::new(move |line: &Value| {
            if let Some(flux) = flux.upgrade() {
                flux.publier("mailbox", &backend, json!({ "kind": "MailboxLine", "parent": p, "task": t, "line": line }));
                // **A message between agents** (c7 for Lucie, 22 November 2026: « des agents qui se parlent »): a line
                // from or to another agent (`agent:<key>`) is also an `AgentMessage` on the subject `agents`, its sender
                // and recipient as agent keys, the task's `teammate` said by the task's own key.
                if let Some(message) = agent_message(line, cle.as_deref(), &p, &t) {
                    flux.publier("agents", &backend, message);
                }
            }
        })));
        Ok(files)
    }

    /// **L'état de l'équipier de chaque tâche**, par l'id du backend de la tâche
    /// : la logique de [`Self::teammate_list`] (`running`, `ended`, `error`,
    /// `waiting`), et si une question ou une approbation de l'équipier attend
    /// encore la réponse de son superviseur. Pour la barre du chat
    /// (`chat_sessions` de l'état de l'hôte).
    pub(super) fn teammate_states(&self) -> std::collections::HashMap<String, (&'static str, bool)> {
        self.every_teammate()
            .into_iter()
            .map(|(t, f)| {
                let lignes = f.mailbox();
                let state = self.teammate_state(&t, &lignes);
                let attend = f
                    .supervisor()
                    .map(|s| f.pending_for(&s).iter().any(|l| matches!(l["kind"].as_str(), Some("question" | "approval"))))
                    .unwrap_or(false);
                (t.id.clone(), (state, attend))
            })
            .collect()
    }

    /// **L'état d'un équipier**, la seule règle que lisent `teammate_list` et
    /// `teammate_states` (relecture d'everything declarative) : `running` s'il vit
    /// et qu'un tour court dans le chat de sa tâche ; `ended` ou `error` (le texte
    /// de sa ligne de fin) s'il s'est arrêté ; `waiting` sinon.
    fn teammate_state(&self, t: &super::taches::Tache, lignes: &[Value]) -> &'static str {
        let fin = lignes.iter().rev().find(|l| l["kind"] == "ended");
        let vivant = self.teammate_running(&t.parent, &t.nom);
        if vivant && self.run_du_chat_en_cours(&t.id).is_some() {
            "running"
        } else if let (Some(fin), false) = (fin, vivant) {
            if fin["text"].as_str().unwrap_or_default().starts_with("erreur") { "error" } else { "ended" }
        } else {
            "waiting"
        }
    }

    /// Toutes les tâches de l'hôte, avec les fichiers de leur équipier.
    fn every_teammate(&self) -> Vec<(super::taches::Tache, TeammateFiles)> {
        let taches: Vec<super::taches::Tache> = self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        taches.into_iter().filter_map(|t| self.teammate_files(&t.parent, &t.nom).ok().map(|f| (t, f))).collect()
    }

    /// **Ce qui attend `who`** dans toutes les boîtes (`GET /ide/teammate/pending?for=`) :
    /// lecture pure, rien n'y est consommé (page 07-novembre-2026-everything-declarative/01, § 2).
    pub fn teammate_pending(&self, who: &str) -> Value {
        let items: Vec<Value> = self
            .every_teammate()
            .into_iter()
            .flat_map(|(t, f)| {
                f.pending_for(who).into_iter().map(move |l| {
                    let mut item = json!({ "id": l["id"], "task": t.nom, "parent": t.parent, "from": l["from"], "kind": l["kind"], "text": l["text"], "at": l["at"] });
                    for k in ["item", "command", "delivered"] {
                        if !l[k].is_null() {
                            item[k] = l[k].clone();
                        }
                    }
                    item
                })
            })
            .collect();
        json!({ "for": who, "items": items })
    }

    /// **Marquer vu** (`POST /ide/teammate/seen`) : un compte rendu, une fin, une
    /// livraison sortent de `pending` pour `who`. Rend combien étaient neufs.
    pub fn teammate_seen(&self, who: &str, ids: &[String]) -> Result<Value, String> {
        if who.is_empty() {
            return Err("for manque : la clé de qui a vu (claude-code:<id>, codex:<nom>…)".into());
        }
        let mut neufs = 0;
        for (_, f) in self.every_teammate() {
            neufs += f.mark_seen(who, ids)?;
        }
        Ok(json!({ "for": who, "seen": neufs }))
    }

    /// **La reprise après un arrêt de l'hôte** (page
    /// 07-novembre-2026-everything-declarative/01, § 1) : au démarrage, chaque tâche
    /// relue dont la dernière demande n'a ni fin (`ended`) ni arrêt (`stop`) avait un
    /// équipier au travail ou en attente quand l'hôte s'est arrêté. Sa boîte le dit (le
    /// pas en cours, s'il y en avait un, n'a pas fini : il n'a pas de `step_done`), et
    /// l'équipier repart à sa frontière, avec son plan et son curseur de boîte gardés
    /// sur le disque. `teammate.resume_after_restart: false` le laisse en attente.
    /// Rend une ligne par équipier.
    pub fn reprendre_les_equipiers(self: &Arc<Self>) -> Vec<String> {
        // **A guest host takes over nothing** of the live host whose root it shares.
        if let Some(de) = &self.invite {
            return vec![format!("guest host of {de}: its teammates are left to it, none is resumed here")];
        }
        let mut lignes = Vec::new();
        // A pair is reachable between two works: its open task is in the registry again, idle.
        for (t, _) in self.every_teammate() {
            if self.pair_de_la_tache(&t.parent, &t.nom, false).is_some() {
                lignes.push(format!("pair {}/{} reachable again in the sessions registry", t.parent, t.nom));
            }
        }
        for (t, files) in self.every_teammate() {
            let boite = files.mailbox();
            let Some(demande) = boite.iter().rposition(|l| l["kind"] == "request") else { continue };
            if boite[demande..].iter().any(|l| l["kind"] == "ended" || l["kind"] == "stop") {
                continue;
            }
            let policy = crate::backend::PreparedBackend::load(&t.manifest).ok().and_then(|p| p.manifest.teammate.clone()).unwrap_or_default();
            if !policy.resume_after_restart {
                let _ = files.append("host", "report", None, "l'hôte a redémarré pendant le travail de l'équipier ; teammate.resume_after_restart est faux : il attend une nouvelle demande");
                lignes.push(format!("équipier {}/{} laissé en attente (resume_after_restart = false)", t.parent, t.nom));
                continue;
            }
            let _ = files.append(
                "host",
                "report",
                None,
                "l'hôte s'est arrêté pendant le travail de l'équipier (un pas en cours n'a pas fini) ; reprise à la frontière",
            );
            let started = self.start_teammate(&t.parent, &t.nom);
            lignes.push(format!("équipier {}/{} {}", t.parent, t.nom, if started { "repris" } else { "déjà en route" }));
        }
        lignes
    }

    /// **Les équipiers d'un superviseur** (`GET /ide/teammate/list?supervisor=`) :
    /// running (un pas est en vol : un tour du chat de la tâche), waiting (pas de
    /// pas en vol et pas de fin : l'équipier attend sa boîte, ou entre deux pas),
    /// ended, error (la fin dit l'erreur).
    pub fn teammate_list(&self, supervisor: &str) -> Value {
        let teammates: Vec<Value> = self
            .every_teammate()
            .into_iter()
            .filter(|(_, f)| f.supervisor().as_deref() == Some(supervisor))
            .map(|(t, f)| {
                let lignes = f.mailbox();
                let derniere = lignes.last().cloned().unwrap_or(Value::Null);
                let state = self.teammate_state(&t, &lignes);
                json!({ "task": t.nom, "parent": t.parent, "state": state, "last_kind": derniere["kind"], "at": derniere["at"] })
            })
            .collect();
        json!({ "supervisor": supervisor, "teammates": teammates })
    }

    /// **Écrire à l'équipier d'une tâche** (`POST /ide/teammate/mailbox`) : la
    /// ligne va dans la boîte et sur le bus. Une `request` à une tâche absente
    /// l'ouvre. Un `stop` arrête tout de suite le pas en cours. L'équipier qui
    /// attend se réveille par le bus ; celui qui travaille lira la ligne à la
    /// frontière. Rend la ligne, et si un équipier a été lancé.
    /// `extra` porte `to`, `reply_to`, `approved` quand ils sont donnés.
    pub fn post_to_teammate(self: &Arc<Self>, parent: &str, task: &str, from: &str, kind: &str, item: Option<&str>, text: &str, extra: &Value) -> Result<Value, String> {
        if !matches!(kind, "request" | "instruction" | "answer" | "message" | "stop") {
            return Err(format!("genre inconnu : {kind} (request, instruction, answer, message, stop)"));
        }
        if text.trim().is_empty() && kind != "stop" {
            return Err("le texte est vide".into());
        }
        // **Ce que la request exige avant la fin** se déclare, il ne se devine pas.
        match extra.get("expects").filter(|v| !v.is_null()) {
            None => {}
            Some(_) if kind != "request" => return Err(format!("expects se déclare sur une request, pas sur un {kind}")),
            Some(e) if e.as_str().is_some_and(|e| EXPECTS.contains(&e)) => {}
            Some(e) => return Err(format!("expects vaut report, approval ou none, pas {e}")),
        }
        let gate = extra.get("gate").filter(|v| !v.is_null());
        match gate {
            None => {}
            Some(_) if kind != "request" => return Err(format!("gate se déclare sur une request, pas sur un {kind}")),
            Some(g) if g.as_str().is_some_and(|g| matches!(g, "bac" | "judge" | "cards")) => {}
            Some(g) => return Err(format!("gate vaut bac, judge ou cards, pas {g}")),
        }
        let ouverte = self.tache(parent, task).is_none();
        if ouverte {
            if kind != "request" {
                return Err(format!("pas de tâche « {task} » sur {parent} : une request l'ouvre"));
            }
            self.ouvrir_une_tache(parent, task)?;
        }
        let t = self.tache(parent, task).ok_or("la tâche vient de s'ouvrir et n'est plus là")?;
        let files = self.teammate_files(parent, task)?;
        // **The launch's level of the gate** (Lucie, 22 Nov 2026, 23 h 20), said before the request so that its first
        // step reads it; said again only when it changes.
        if let Some(g) = gate.and_then(Value::as_str) {
            let avant = files.mailbox().iter().rev().find(|l| l["from"] == "host" && l["kind"] == "gate_level").and_then(|l| l["level"].as_str().map(str::to_string));
            if avant.as_deref() != Some(g) {
                let dit = match g {
                    "bac" => "porte : bac (toute commande que le bac lance passe)",
                    "judge" => "porte : judge (le juge décide, une carte s'il dit non)",
                    _ => "porte : cards (toute commande tenue demande ; les écritures des outils de code passent)",
                };
                files.append_with("host", "gate_level", None, dit, &json!({ "level": g }))?;
            }
        }
        // **Une réponse va à l'auteur de ce qu'elle répond** (le pont, archi) : une
        // `answer` sans `to` dont le `reply_to` vise la ligne d'un autre agent
        // (ni l'équipier ni l'hôte) lui est adressée. `mail reply` ne connaît que
        // l'id ; c'est la boîte qui sait qui l'a écrit.
        let mut extra = extra.clone();
        if kind == "answer" && extra["to"].is_null() {
            if let Some(r) = extra["reply_to"].as_str() {
                if let Some(auteur) = files.mailbox().iter().find(|l| l["id"] == r).and_then(|l| l["from"].as_str().map(str::to_string)) {
                    if !matches!(auteur.as_str(), "teammate" | "host") {
                        if !extra.is_object() {
                            extra = json!({});
                        }
                        extra["to"] = json!(auteur);
                    }
                }
            }
        }
        // **Un oui à une commande en attente l'accorde** pour la suite de la
        // tâche (lot 2 de la page 01) : la ligne visée est une `approval` de
        // l'équipier, la commande passe par la porte du backend de la tâche,
        // dans son worktree. L'accord se pose **avant** que le oui s'écrive, et
        // le oui le porte (`granted`) : l'équipier, qui ne lit pas les lignes de
        // l'hôte, l'apprend dans la ligne qui le réveille ; la boîte le redit au
        // superviseur.
        if let Some(o) = extra.as_object_mut() {
            o.remove("granted"); // l'hôte seul l'écrit
        }
        #[allow(unused_mut)]
        let mut accord: Option<String> = None;
        // **Only a human's yes grants** (c7, 22 Nov 2026: the answer's branch never read `by`): a yes signed by an agent,
        // or by no one, is kept in the mailbox, grants nothing, and says why.
        #[cfg(feature = "code")]
        if kind == "answer" && extra["approved"] == true && extra["by"]["kind"] != "human" {
            if let Some(o) = extra.as_object_mut() {
                o.insert(
                    "granted".into(),
                    Value::String(
                        "pas accordé : seule une réponse humaine, depuis ta session (l'IDE ouvert par ton lanceur, ou le terminal), accorde une carte".into(),
                    ),
                );
            }
        }
        #[cfg(feature = "code")]
        if kind == "answer" && extra["approved"] == true && extra["by"]["kind"] == "human" {
            let visee = extra["reply_to"].as_str().and_then(|r| files.mailbox().into_iter().find(|l| l["id"] == r));
            if let Some(ligne) = visee.as_ref().filter(|l| l["kind"] == "approval").and_then(|l| l["command"].as_str().map(str::to_string)) {
                let carte = visee.clone().unwrap_or(Value::Null);
                let classe = carte["class"].as_str().unwrap_or_default().to_string();
                let note = if classe == "open_root" || classe == "protected" {
                    // **A yes on a root's card** (judge_all): the root is granted to the task's agent, durably for
                    // open_root, for this command only for a protected folder; never a line's grant.
                    let racine = std::path::PathBuf::from(carte["args"]["root"].as_str().unwrap_or_default());
                    let mode = carte["args"]["mode"].as_str().unwrap_or("read").to_string();
                    let preuve = super::root_grants::Proof {
                        quote: text.to_string(),
                        at: None,
                        turn: extra["reply_to"].as_str().map(str::to_string),
                    };
                    let accorde = match self.agent_key_of_backend(&t.id) {
                        None => Err("la tâche n'a pas de clé d'agent : la racine ne peut pas lui être accordée".to_string()),
                        Some(agent) if classe == "protected" => self.grant_once(&agent, &racine, preuve, "user", "read"),
                        Some(agent) => self.grant_root(&agent, &racine, preuve, "user", &mode).map(|_| ()),
                    };
                    match accorde {
                        Ok(()) => format!("racine accordée : {} ({mode}) — relance ta commande mot pour mot : {ligne}", racine.display()),
                        Err(e) => format!("l'accord de la racine n'a pas pu se poser : {e}"),
                    }
                } else {
                    let domaine = t.workspace.as_ref().map(|w| w.racine.clone());
                    let accorde = self.with_backend_id(&t.id, |b| match b.garde() {
                        Some(g) => g.accorder_ligne(&ligne, &crate::commande::Contexte { accorde_par_l_utilisateur: true, domaine }),
                        None => Err("ce backend n'a pas de porte des commandes (workspace.commands)".into()),
                    });
                    match accorde {
                        // The line word for word: the grant is keyed by it, and the agent replays what it reads.
                        Ok(Ok(_)) => format!("commande accordée pour la tâche : {ligne} — relance-la mot pour mot"),
                        Ok(Err(e)) | Err(e) => format!("l'accord n'a pas pu se poser : {e}"),
                    }
                };
                if let Some(o) = extra.as_object_mut() {
                    o.insert("granted".into(), Value::String(note.clone()));
                }
                accord = Some(note);
            }
        }
        // **A no to a held command is said to the teammate** (the auto mode's v1): it reads it in the line that
        // wakes it, and the host says it in the mailbox.
        #[allow(unused_mut)]
        let mut refus: Option<String> = None;
        if kind == "answer" && extra["approved"] == false {
            let visee = extra["reply_to"].as_str().and_then(|r| files.mailbox().into_iter().find(|l| l["id"] == r));
            if let Some(ligne) = visee.as_ref().filter(|l| l["kind"] == "approval").and_then(|l| l["command"].as_str().map(str::to_string)) {
                let note = format!("commande refusée par l'utilisateur : {ligne} — ne la relance pas ; continue sans elle, ou demande autre chose");
                if let Some(o) = extra.as_object_mut() {
                    o.insert("granted".into(), Value::String(note.clone()));
                }
                refus = Some(note);
            }
        }
        let line = files.append_with(from, kind, item, text, &extra)?;
        if let Some(note) = refus {
            let _ = files.append("host", "report", None, &note);
        }
        // **D'où part la tâche** (c7, premier essai réel, 7 novembre) : ses
        // déclarations et son code sont ceux d'un commit ; une modification non
        // commitée n'y est pas, et sans le dire on la cherche longtemps.
        let depart = ouverte.then(|| {
            let court: String = t.depart.chars().take(9).collect();
            // Sans git (page 03 d'everything declarative, § 1) : la boîte le dit.
            let en_place = t.workspace.as_ref().and_then(|w| w.code.as_ref()).filter(|c| c.en_place);
            let note = match (t.workspace.as_ref().filter(|w| w.copie.is_some()), en_place) {
                (Some(w), _) => format!("tâche ouverte : {} (une copie de {})", super::taches::SANS_GIT, w.racine_principale.display()),
                // **In the user's folder** (23 November 2026): no branch of the task, no commit to start from.
                (None, Some(c)) => format!("tâche ouverte : l'agent travaille dans ton dossier {}, sur ta branche courante, depuis {court}", c.depot.display()),
                (None, None) => format!(
                    "tâche ouverte : branche {} depuis {court} ; ses déclarations et son code sont ceux de ce commit (une modification non commitée du manifeste ou du code n'y est pas){}",
                    t.branche,
                    if t.git_init_fait { " ; la racine est devenue un dépôt à l'ouverture (git_init déclaré), premier commit par rag3weaver" } else { "" }
                ),
            };
            let _ = files.append("host", "report", None, &note);
            json!({ "branche": t.branche, "depart": t.depart })
        });
        if let Some(note) = &accord {
            let _ = files.append("host", "report", None, note);
        }
        let bus = self.with_backend_id(&t.id, |b| b.bus_de_publication()).ok();
        publish(bus.as_ref(), task, &line);
        if kind == "stop" {
            if let Some(run) = self.run_du_chat_en_cours(&t.id) {
                let _ = self.stop_run(&t.id, &run, from);
            }
            // **The tool calls in flight stop too** (23 Nov 2026: a wait_output held 300 s after a stop): every run of
            // the task's backend, the tools' graphs included. A background command lives on with its task.
            let _ = self.with_backend_id(&t.id, |b| {
                let registre = b.runs_in_flight_registry();
                for run in registre.runs() {
                    let _ = registre.stop(&run, from);
                }
            });
        }
        // **Seule une ligne adressée à l'équipier le lance** (archi, 7 novembre
        // 2026) : une parole d'un agent à un autre rangée dans cette boîte, ou une
        // réponse à une ligne qui n'est pas de lui, est rangée et relayée, sans
        // lancer personne (ni dépenser un appel au modèle).
        let pour_lui = kind == "request"
            || (kind != "stop"
                && addressed_to_teammate(&line, &files.key)
                && match line["reply_to"].as_str() {
                    None => true,
                    Some(r) => files.mailbox().iter().find(|l| l["id"] == r).is_none_or(|l| matches!(l["from"].as_str(), Some("teammate" | "host"))),
                });
        let mut r = if pour_lui {
            let started = self.start_teammate(parent, task);
            json!({ "line": line, "started": started })
        } else {
            json!({ "line": line, "started": false, "why": if kind == "stop" { Value::Null } else { json!("not_addressed") } })
        };
        if let Some(d) = depart {
            r["task"] = d;
        }
        Ok(r)
    }

    /// Lance la boucle de l'équipier d'une tâche, sur son fil, s'il n'y en a
    /// pas déjà une. Rend vrai si elle part.
    pub fn start_teammate(self: &Arc<Self>, parent: &str, task: &str) -> bool {
        let key = format!("{parent}/{task}");
        {
            let mut running = self.teammates.lock().unwrap_or_else(|p| p.into_inner());
            if !running.insert(key.clone()) {
                return false;
            }
        }
        let (host, parent, task, cle) = (Arc::downgrade(self), parent.to_string(), task.to_string(), key.clone());
        let lance = std::thread::Builder::new().name(format!("teammate-{task}")).spawn(move || {
            if let Some(h) = host.upgrade() {
                let fini = match h.run_teammate(&parent, &task) {
                    Ok(o) => {
                        eprintln!("[équipier] {parent}/{task} : {} pas, {}", o.steps.len(), o.ended);
                        // Une fin « plus rien à faire » seulement : un arrêt (la
                        // boîte, des pas ratés, la borne des pas) ne se relance pas.
                        o.ended == "fait sans plan" || o.ended.starts_with("plus rien à faire")
                    }
                    Err(e) => {
                        eprintln!("[équipier] {parent}/{task} : {e}");
                        false
                    }
                };
                h.teammates.lock().unwrap_or_else(|p| p.into_inner()).remove(&cle);
                // Une ligne postée pendant que ce lancement finissait n'a rien
                // relancé (il tenait encore la clé) : on la relance ici. La ligne
                // s'écrit avant que la poste tente le lancement, et la clé part
                // avant cette relecture : l'une des deux la voit toujours. Après
                // une fin « plus rien à faire » seulement : une erreur ou un arrêt
                // gardent leurs nouvelles non lues, et relancer sur eux bouclerait.
                if fini && h.tache(&parent, &task).is_some() && h.teammate_files(&parent, &task).is_ok_and(|f| unread_news(&f)) {
                    h.start_teammate(&parent, &task);
                }
            }
        });
        if lance.is_err() {
            self.teammates.lock().unwrap_or_else(|p| p.into_inner()).remove(&key);
        }
        lance.is_ok()
    }

    /// The pair mode's entry for this task, if its backend declares `teammate.pair`: made at the
    /// first paired task (the registry is cleaned of our dead entries then, never before), posed
    /// busy, named after the agent, in the task's folder. A refusal (a second paired task, a
    /// registry that cannot be written) is said in the mailbox, and the task runs without it.
    pub(super) fn pair_de_la_tache(&self, parent: &str, task: &str, occupe: bool) -> Option<Arc<super::pair::Pair>> {
        let files = self.teammate_files(parent, task).ok();
        let dire = |texte: &str| {
            if let Some(f) = &files {
                let _ = f.append("host", "report", None, texte);
            }
        };
        let t = self.tache(parent, task)?;
        let policy = crate::backend::PreparedBackend::load(&t.manifest).ok().and_then(|p| p.manifest.teammate.clone()).unwrap_or_default();
        if !policy.pair {
            return None;
        }
        let pair = self
            .pair
            .get_or_init(|| {
                let registre = self.pair_registre.clone().or_else(|| super::pair::Registre::depuis(|k| std::env::var(k).ok()))?;
                registre.nettoyer();
                let mut pair = super::pair::Pair::new(registre);
                // A message from a session is a request from « session » in the task's mailbox: it
                // starts the teammate, or joins the running one's mailbox (never a second one).
                if let Some(hote) = self.soi.get().cloned() {
                    pair = pair.avec_livreur(Arc::new(move |cle: &str, trame: super::pair::Trame| {
                        let (Some(h), super::pair::Trame::Message { texte, from, from_name, msg_id, priority }) = (hote.upgrade(), trame) else { return };
                        let Some((parent, task)) = cle.split_once('/') else { return };
                        let extra = json!({ "peer": from_name, "reply_to_address": from, "msg_id": msg_id, "priority": priority });
                        if let Err(e) = h.post_to_teammate(parent, task, "session", "request", None, &texte, &extra) {
                            eprintln!("[pair] the message for {cle} did not reach its mailbox: {e}");
                        }
                    }));
                }
                Some(Arc::new(pair))
            })
            .clone();
        let Some(pair) = pair else {
            dire("teammate.pair: no sessions registry here (neither HOME nor XDG_RUNTIME_DIR): the task runs without the pair mode");
            return None;
        };
        let nom = self.with_backend_id(&t.id, |b| b.prepared().nom_de_l_agent()).unwrap_or_else(|_| task.to_string());
        let dossier = t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or_else(|| t.arbre.clone());
        let cle = format!("{parent}/{task}");
        match pair.ouvrir(&cle, &nom, &dossier) {
            Ok(()) => {
                if !occupe {
                    pair.statut(&cle, false);
                }
                Some(pair)
            }
            Err(e) => {
                dire(&format!("{e}: the task runs without the pair mode"));
                None
            }
        }
    }

    /// The pair entry of a closed task goes, with its socket.
    pub(super) fn pair_fermer(&self, cle: &str) {
        if let Some(Some(p)) = self.pair.get() {
            p.fermer(cle);
        }
    }

    /// `busy` or `idle` on the pair entry, if this task holds it.
    fn pair_statut(&self, cle: &str, occupe: bool) {
        if let Some(Some(p)) = self.pair.get() {
            p.statut(cle, occupe);
        }
    }

    /// Un équipier tourne-t-il sur cette tâche ?
    pub fn teammate_running(&self, parent: &str, task: &str) -> bool {
        self.teammates.lock().unwrap_or_else(|p| p.into_inner()).contains(&format!("{parent}/{task}"))
    }

    /// **La boucle de l'équipier** : des pas, chacun un tour de chat dans la
    /// tâche ; à chaque frontière, la boîte et les plans. Elle finit quand il
    /// n'y a plus rien à faire (après un dernier pas qui rend compte), sur un
    /// arrêt, ou quand l'agent tourne en rond. Sans rien à faire mais des
    /// éléments qui attendent le superviseur, elle attend la boîte.
    /// **Plays the verification gate** of a task: each command through the task backend's
    /// `run_command` (the same gate and sandbox as the teammate's). The first red one, with what it
    /// said; a command that is refused or waits for a grant counts as red.
    fn verify_the_tree(&self, backend_id: &str, commandes: &[String]) -> Result<(), (String, String)> {
        use crate::agent::ToolBox;
        for (i, commande) in commandes.iter().enumerate() {
            let call = ToolCall {
                id: format!("verify-{i}"),
                name: "run_command".into(),
                arguments: json!({ "command": commande }).to_string(),
                provider_extra: None,
                at_ms: None,
            };
            let rendu = self
                .with_backend_id(backend_id, |b| {
                    let outils = crate::backend_tools::BackendTools { backend: b };
                    let declares = crate::declare::DeclarationTools { backend: b, inner: &outils };
                    let du_chat = super::chat::OutilsDuChat::de(b, &declares, true);
                    let rendu = du_chat.call(&call).content;
                    rendu
                })
                .map_err(|e| (commande.clone(), e))?;
            let vert = rendu.contains("\ncode 0 ") && !rendu.contains("_Arrêt : cette commande a échoué._");
            if !vert {
                return Err((commande.clone(), rendu));
            }
        }
        Ok(())
    }

    pub fn run_teammate(&self, parent: &str, task: &str) -> Result<Outcome, String> {
        let files = self.teammate_files(parent, task)?;
        // **The pair mode**: the entry in the sessions registry lives as long as the teammate.
        let cle = format!("{parent}/{task}");
        let pair = self.pair_de_la_tache(parent, task, true);
        // The turn's start, for its facts at the end (23 Nov 2026).
        // The turn's start, kept for its facts (at its end, or before it waits), and its failures emptied.
        #[cfg(feature = "code")]
        self.mark_turn_start(parent, task, &files);
        // La fin, dans la boîte, erreur comprise : le superviseur la lit sans IDE.
        let outcome = match self.teammate_loop(parent, task, &files) {
            Ok(o) => o,
            Err(e) => Outcome { steps: Vec::new(), ended: format!("erreur : {e}") },
        };
        // A pair stays reachable between two works: idle, until its task is closed.
        if let Some(p) = &pair {
            p.statut(&cle, false);
        }
        // **The facts of the turn, by the host**, before its end: what git sees changed, or that nothing did.
        #[cfg(feature = "code")]
        let faits = self.say_turn_facts(&files);
        #[cfg(not(feature = "code"))]
        let faits: Option<Value> = None;
        // **A stopped turn says so in fields** (23 Nov 2026, for the views): who stopped it, what landed.
        let line = if outcome.ended.starts_with("arrêté") {
            let boite = files.mailbox();
            let qui = boite.iter().rev().find(|l| l["kind"] == "stop").map(|l| match l["by"]["kind"].as_str() {
                Some("human") => "human",
                Some("agent") => "agent",
                _ => "host",
            });
            let debut = boite.iter().rposition(|l| l["kind"] == "ended").map_or(0, |n| n + 1);
            let mut atterris: Vec<Value> = Vec::new();
            for l in boite[debut..].iter().filter(|l| l["from"] == "host" && l["kind"] == "merged") {
                for p in l["paths"].as_array().or(l["files"].as_array()).into_iter().flatten() {
                    if !atterris.contains(p) {
                        atterris.push(p.clone());
                    }
                }
            }
            files.append_with("host", "ended", None, &outcome.ended, &json!({ "stopped": true, "by": qui.unwrap_or("host"), "landed": atterris }))?
        } else {
            files.append("host", "ended", None, &outcome.ended)?
        };
        if let Ok(bus) = self.tache(parent, task).ok_or(()).and_then(|t| self.with_backend_id(&t.id, |b| b.bus_de_publication()).map_err(|_| ())) {
            if let Some(f) = &faits {
                publish(Some(&bus), task, f);
            }
            publish(Some(&bus), task, &line);
        }
        // **Le curseur reste où la boucle l'a laissé** : le marquer au bout de
        // la boîte effaçait une ligne arrivée pendant le dernier pas (le oui du
        // lot des commandes, 8 novembre 2026), que la relance de start_teammate
        // doit voir. Les lignes de l'équipier et de l'hôte ne sont jamais des
        // nouvelles ; celles qu'un pas raté a rendues se relisent au lancement
        // suivant.
        if outcome.ended.starts_with("erreur : ") {
            return Err(outcome.ended);
        }
        Ok(outcome)
    }

    fn teammate_loop(&self, parent: &str, task: &str, files: &TeammateFiles) -> Result<Outcome, String> {
        let t = self.tache(parent, task).ok_or_else(|| format!("pas de tâche « {task} » sur {parent}"))?;
        // **Le bus, s'il y en a un** : un backend qui ne s'ouvre pas n'en a pas,
        // et l'équipier se réveille alors par la relecture du fichier de boîte.
        let bus = self.with_backend_id(&t.id, |b| b.bus_de_publication()).ok();
        // L'abonnement au sujet de la boîte, pris avant de lire : une ligne
        // arrivée entre la lecture et l'attente réveille quand même.
        let mut wake = bus.as_ref().map(|b| b.subscribe(&mailbox_topic(task)));
        let policy = crate::backend::PreparedBackend::load(&t.manifest).ok().and_then(|p| p.manifest.teammate.clone()).unwrap_or_default();
        let mut failed = 0u32;
        // Les refus du fournisseur d'affilée (429, panne) : ni pas raté ni pas compté.
        let mut unavailable = 0u32;
        let mut woke_by: Option<String> = None;
        let mut outcome = Outcome::default();
        let mut read = files.read_cursor();
        let mut stalled = 0usize;
        // Un plan a-t-il existé pendant ce lancement ? (la fin le dit autrement)
        let mut plan_seen = false;
        // **Un pas a dit ce qui reste** (sans plan vivant) : le suivant part sans attendre la boîte.
        let mut encore = false;
        // **La garde d'une demande sans ligne** (forme B de c7, 15 novembre 2026) : la demande
        // suivie, ce que l'équipier a laissé hors de la boîte depuis elle (un fichier écrit, une
        // commande, un plan clos), la demande déjà relancée (une fois), le texte de la relance
        // pour la frontière suivante, et l'échéance de l'attente du superviseur.
        let pour_les_traces = crate::backend::PreparedBackend::load(&t.manifest).ok();
        let mut demande_suivie: Option<usize> = None;
        let mut trace = false;
        let mut relancee: Option<usize> = None;
        let mut relance: Option<String> = None;
        // The red verifications of this task (`teammate.verify`).
        let mut rouges = 0usize;
        // What the last step did, carried to the next frontier (S2).
        let mut previous_step = String::new();
        // The last error of the scripted `Previous step`, said once until it changes.
        let mut erreur_du_bloc: Option<String> = None;
        let mut attente_bornee: Option<std::time::Instant> = None;
        let request = files.mailbox().iter().find(|l| l["kind"] == "request").and_then(|l| l["text"].as_str()).unwrap_or_default().to_string();
        // `number` numérote les pas joués ; `counted` ne compte pas ceux qui
        // finissent en attente du superviseur : MAX_STEPS borne le travail, pas
        // l'attente.
        let mut number = 0usize;
        let mut counted = 0usize;
        while counted < MAX_STEPS {
            let lines = files.mailbox();
            let news: Vec<Value> = lines
                .iter()
                .skip(read)
                .filter(|l| l["from"] != "teammate" && l["from"] != "host" && addressed_to_teammate(l, &files.key))
                .cloned()
                .collect();
            // Le curseur d'avant ce pas : un pas qui n'aboutit pas rend ses nouvelles
            // au suivant (sans elles, l'équipier attendrait une boîte déjà lue).
            let avant = read;
            read = lines.len();
            files.write_cursor(read);
            // **A stop is worth for the turn in progress only** (23 Nov 2026: it ate the next message): it ends the
            // turn when nothing addressed came after it.
            let dernier_stop = news.iter().rposition(|l| l["kind"] == "stop");
            let derniere_demande = news.iter().rposition(|l| l["kind"] != "stop");
            if dernier_stop.is_some_and(|s| derniere_demande.is_none_or(|d| s > d)) {
                outcome.ended = "arrêté par la boîte".into();
                return Ok(outcome);
            }
            let plans = files.read_plans();
            plan_seen |= !plans.plans.is_empty();
            let active = plans.active();
            let alone = active.is_some_and(|p| p.next_alone().is_some());
            let waiting = active.is_some_and(|p| !p.waiting().is_empty());
            // Un pas de travail : une nouvelle dans la boîte, un élément faisable,
            // ou un plan actif qui ne fait plus qu'attendre rien (à clore).
            let work = std::mem::take(&mut encore) || !news.is_empty() || alone || (active.is_some() && !waiting) || (active.is_none() && !plans.paused().is_empty() && number == 0);
            let last = active.is_some() && !alone && !waiting && news.is_empty();
            // **En attente du superviseur** : une question ou une commande
            // demandée, sans réponse ni ligne nouvelle, suspend l'équipier.
            let suspendu = news.is_empty() && open_question(files).is_some();
            if !work || suspendu {
                // Rien à faire sans le superviseur : on attend la boîte, par le
                // bus quand il y en a un, et toujours par le fichier (toutes les
                // 2 s) : un backend fermé ne publie plus. Le journal du pas
                // suivant dit par où l'équipier s'est réveillé.
                let depuis = files.mailbox().len();
                let mut tour = 0u32;
                self.pair_statut(&format!("{parent}/{task}"), false);
                woke_by = loop {
                    if let Some(rx) = wake.as_mut() {
                        match rx.try_recv() {
                            Ok(_) | Err(async_broadcast::TryRecvError::Overflowed(_)) => break Some("bus".to_string()),
                            Err(async_broadcast::TryRecvError::Closed) => wake = None,
                            Err(async_broadcast::TryRecvError::Empty) => {}
                        }
                    }
                    tour += 1;
                    if tour % 10 == 0 {
                        if attente_bornee.is_some_and(|fin| std::time::Instant::now() >= fin) {
                            let why = format!("sans réponse de l'équipier : la demande est restée sans ligne de lui après sa relance, et le superviseur n'a rien écrit en {} s (teammate.reply_wait_s)", policy.reply_wait_s);
                            let line = files.append("host", "report", None, &why)?;
                            publish(bus.as_ref(), task, &line);
                            outcome.ended = why;
                            return Ok(outcome);
                        }
                        if self.tache(parent, task).is_none() {
                            outcome.ended = "la tâche s'est fermée pendant l'attente".into();
                            return Ok(outcome);
                        }
                        if files.mailbox().len() > depuis {
                            break Some("fichier".to_string());
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                };
                continue;
            }
            number += 1;
            let debut_du_pas = read;
            // La demande en cours : la dernière reçue ; les pas faits : ceux
            // déclarés depuis elle (une relance sur une demande nouvelle ne
            // relit pas l'ancienne).
            let derniere = lines.iter().rposition(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host");
            // **A follow-up adds to the open request** (S3, 18 Nov 2026): the open request is
            // every supervisor request since the last `ended`; the first one is the brief, the
            // later ones are its follow-ups (a « continue » no longer replaces it).
            let ouverture = lines.iter().rposition(|l| l["kind"] == "ended").map_or(0, |n| n + 1);
            let ouvertes: Vec<usize> = (ouverture..lines.len()).filter(|&n| lines[n]["kind"] == "request" && lines[n]["from"] != "teammate" && lines[n]["from"] != "host").collect();
            let consigne = match ouvertes.as_slice() {
                [] => String::new(),
                [seule] => lines[*seule]["text"].as_str().unwrap_or_default().to_string(),
                [premiere, suites @ ..] => {
                    let mut c = format!("Request (opened at {}): {}", lines[*premiere]["at"].as_str().map(str::to_string).unwrap_or_else(|| lines[*premiere]["at"].to_string()), lines[*premiere]["text"].as_str().unwrap_or_default());
                    for n in suites {
                        c.push_str(&format!("\nFollow-up: {}", lines[*n]["text"].as_str().unwrap_or_default()));
                    }
                    c
                }
            };
            // **A turn receives the turns before it** (c7, 23 Nov 2026): the conversation's earlier requests and what the
            // teammate answered, the last ones in full, the older ones bounded, before this turn's request.
            let consigne = match conversation_before(&lines[..ouverture]) {
                Some(avant) if !consigne.is_empty() => format!("{avant}\n\nThis turn's request:\n{consigne}"),
                _ => consigne,
            };
            let premiere_ouverte = ouvertes.first().copied().or(derniere);
            if derniere != demande_suivie {
                demande_suivie = derniere;
                trace = derniere.is_some() && files.read_trace() == derniere;
            }
            attente_bornee = None;
            let faits: Vec<Value> = lines.iter().skip(premiere_ouverte.map_or(0, |n| n + 1)).filter(|l| l["from"] == "teammate" && l["kind"] == "step_done").cloned().collect();
            // The loop's texts, read at each step: an edit of the user's texts.json is in the next step (23 Nov 2026).
            let textes = self.prompt_texts();
            let mut message = frontier_message(number, &news, &plans, last, &consigne, &faits, &textes);
            // **Its tree is not her folder** (23 Nov 2026, `~/essai-2`).
            if let Some(w) = t.workspace.as_ref().filter(|w| w.racine != w.racine_principale) {
                message.push_str(&format!(
                    "\nYou work in your own tree ({}); the user is in her folder {}, where your work lands at the end of each step. Any command or path you give the user names {}, never your tree.\n",
                    w.racine.display(),
                    w.racine_principale.display(),
                    w.racine_principale.display()
                ));
            }
            match expected_deliverable(&lines).map(|(_, e)| e) {
                Some(e) if e == "report" => message.push_str("\nLa demande exige un report (outil report) avant la fin : sans lui, la tâche attend le superviseur.\n"),
                Some(e) if e == "approval" => message.push_str("\nLa demande exige l'accord du superviseur avant la fin : propose (report, question), puis attends sa réponse.\n"),
                _ => {}
            }
            if let Some(r) = relance.take() {
                message.push_str(&format!("\n{r}\n"));
            }
            message.push_str(&previous_step);
            self.pair_statut(&format!("{parent}/{task}"), true);
            let cle = format!("{parent}/{task}");
            let tenue = self.pair.get().cloned().flatten().filter(|p| p.tient(&cle)).map(|p| (p, cle));
            let mut step = Step::new(files.clone(), task, &request, bus.clone(), number)
                .with_folders(t.workspace.as_ref().map(|w| (w.racine.display().to_string(), w.racine_principale.display().to_string()))).with_policy(&policy).with_background(self.task_background(parent, task)).with_pair(tenue).with_browser(policy.browser.clone().map(|p| {
                let tools = self.browser_listening().map(|ecoute| super::browser_tools::BrowserTools { policy: p.clone(), ecoute, session: format!("teammate:{parent}/{task}") });
                (p, tools)
            }));
            #[cfg(feature = "code")]
            {
                if let Some(t) = self.tache(parent, task) {
                    let racine = t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or_else(|| t.arbre.clone());
                    step.gate = self.step_gate(&t.id, &files, racine);
                    step.open_root = self.step_open_root(&t.id, &files);
                    step.gate_level = self.gate_level(&t.id, &files);
                }
            }
            step.woke_by = woke_by.take();
            // The refusal of a silent step_done, from the user's own template file, read at each step.
            step = step.with_delivery(read, textes.clone()).with_at_step_done(self.at_step_done_graph());
            // **Un pas qui n'aboutit pas ne termine pas l'équipier** : l'échec va
            // dans la boîte et le pas suivant repart de la frontière ; seuls
            // `teammate.max_failed_steps` échecs d'affilée l'arrêtent.
            let r = match self.chat_turn_with(&t.id, &message, Some(&step)) {
                Ok(r) => {
                    failed = 0;
                    unavailable = 0;
                    // **The turn's commits land in the user's folder** (the terminal's night, 22 Nov 2026).
                    #[cfg(feature = "code")]
                    self.land_the_turn(&t, &files, &step.wrote(), !r["stopped_by"].is_null());
                    // By the declared scripted node when there is one (its `nodes/` folder is the live backend's), else
                    // the Rust block; a failing script falls back to the Rust block, and the box says it once.
                    let dossier = policy
                        .carry_over_node
                        .as_ref()
                        .and_then(|_| self.with_backend_id(&t.id, |b| b.prepared().directory.join("nodes")).ok());
                    let noeud = match (&policy.carry_over_node, &dossier) {
                        (Some(nom), Some(d)) => Some((d.as_path(), nom.as_str(), &policy.carry_over_node_config)),
                        _ => None,
                    };
                    let (bloc, erreur) = step.carry_over_with(r["text"].as_str().unwrap_or_default(), policy.carry_over_chars, noeud);
                    previous_step = bloc;
                    if let Some(e) = erreur.filter(|e| erreur_du_bloc.as_ref() != Some(e)) {
                        let line = files.append(
                            "host",
                            "report",
                            None,
                            &format!("le bloc « Previous step » du pas {number} vient du repli Rust : le nœud teammate.carry_over_node a échoué : {e}"),
                        )?;
                        publish(bus.as_ref(), task, &line);
                        erreur_du_bloc = Some(e);
                    }
                    r
                }
                Err(e) if refus_du_fournisseur(&e).is_some() => {
                    // **Le fournisseur n'a pas répondu** : le même pas rejoue
                    // après une fenêtre, ses nouvelles rendues ; un quota épuisé
                    // ne revient pas en attendant, il arrête en le disant.
                    read = avant;
                    files.write_cursor(read);
                    number -= 1;
                    unavailable += 1;
                    let epuise = refus_du_fournisseur(&e) == Some(true);
                    if epuise || unavailable > policy.max_unavailable_steps {
                        let why = if epuise {
                            format!("erreur : {e} (un quota épuisé ne revient pas en attendant : l'équipier s'arrête)")
                        } else {
                            format!("erreur : {e} ({} refus du fournisseur d'affilée, teammate.max_unavailable_steps = {})", unavailable, policy.max_unavailable_steps)
                        };
                        outcome.ended = why;
                        return Ok(outcome);
                    }
                    let attente = attente_du_fournisseur(policy.quota_wait_s, unavailable);
                    let line = files.append(
                        "host",
                        "report",
                        None,
                        &format!("le pas {} attend le fournisseur ({unavailable} refus sur {} permis) : {e} ; reprise dans {attente} s", number + 1, policy.max_unavailable_steps),
                    )?;
                    publish(bus.as_ref(), task, &line);
                    // **On the bus too** (19 November 2026): the wait for the model, on the run of the
                    // refused step, then its end when the step plays again.
                    let attendu = std::time::Instant::now();
                    let sur_le_bus = bus.as_ref().zip(step.turn_run());
                    if let Some((b, (run, model))) = &sur_le_bus {
                        b.emit(crate::events::Event::Waiting { run: run.clone(), cause: "model".into(), what: model.clone(), since_ms: 0, wait_ms: Some(attente * 1000) });
                    }
                    // L'attente se coupe sur un `stop` de la boîte, ou si la tâche se ferme.
                    let fin = std::time::Instant::now() + std::time::Duration::from_secs(attente);
                    while std::time::Instant::now() < fin {
                        if files.mailbox().iter().skip(read).any(|l| l["kind"] == "stop" && l["from"] != "teammate" && l["from"] != "host") {
                            outcome.ended = "arrêté par la boîte".into();
                            return Ok(outcome);
                        }
                        if self.tache(parent, task).is_none() {
                            outcome.ended = "la tâche s'est fermée pendant l'attente".into();
                            return Ok(outcome);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(500).min(fin.saturating_duration_since(std::time::Instant::now())));
                    }
                    if let Some((b, (run, _))) = &sur_le_bus {
                        b.emit(crate::events::Event::Resumed { run: run.clone(), cause: "model".into(), waited_ms: attendu.elapsed().as_millis() as u64 });
                    }
                    continue;
                }
                Err(e) => {
                    failed += 1;
                    let line = files.append("host", "report", None, &format!("le pas {number} n'a pas abouti : {e}"))?;
                    publish(bus.as_ref(), task, &line);
                    if failed >= policy.max_failed_steps {
                        outcome.ended = format!("erreur : {e} ({failed} pas d'affilée n'ont pas abouti, teammate.max_failed_steps = {})", policy.max_failed_steps);
                        return Ok(outcome);
                    }
                    read = avant;
                    files.write_cursor(read);
                    std::thread::sleep(std::time::Duration::from_secs(2 * u64::from(failed)));
                    continue;
                }
            };
            let declared = step.declared();
            let run = r["run"].as_str().unwrap_or_default().to_string();
            // The text the model wrote in this step, for the guard « un compte rendu n'est pas le contenu ».
            let mut texte_du_pas = String::new();
            // **Le texte écrit dans un pas qui finit par step_done** (ticket de tty, 14 novembre
            // 2026) : c'est souvent la réponse elle-même (un texte demandé) ; il va dans la boîte en
            // `report`, sauf si un report de ce pas le porte déjà (le report l'emporte, c7).
            if declared.is_some() && r["stopped_by"].is_null() {
                // Les textes de l'assistant depuis le message du pas (sa frontière) : `r["text"]` ne
                // garde que celui du dernier appel, souvent vide après step_done.
                let tours = self.chat_state(&t.id).map(|e| e["turns"].as_array().cloned().unwrap_or_default()).unwrap_or_default();
                let depuis = tours.iter().rposition(|t| t["role"] == "user").map_or(0, |i| i + 1);
                let textes: Vec<String> = tours[depuis..]
                    .iter()
                    .filter(|t| t["role"] == "assistant")
                    .filter_map(|t| t["content"].as_str().map(str::trim).filter(|c| !c.is_empty()).map(str::to_string))
                    .collect();
                let texte = textes.join("\n\n");
                let texte = texte.trim();
                texte_du_pas = texte.to_string();
                let deja = files
                    .mailbox()
                    .iter()
                    .skip(avant)
                    .any(|l| l["from"] == "teammate" && l["kind"] == "report" && l["text"].as_str().is_some_and(|t| t.contains(texte)));
                if !texte.is_empty() && !deja {
                    // Une copie faite par l'hôte du texte du modèle : `origin` le dit, le texte est la
                    // réponse seule (tty, 16 novembre 2026).
                    let line = files.append_with("teammate", "report", None, &in_her_folder(step.folders.as_ref(), texte), &json!({ "origin": "host_copy" }))?;
                    publish(bus.as_ref(), task, &line);
                }
            }
            // **Un pas ne finit jamais en silence** (c7, premier essai réel) : sans
            // `step_done`, la réponse finale du modèle va dans la boîte en `report`,
            // pour que le superviseur la voie (pending) au lieu d'un équipier qui
            // attend sans rien avoir dit.
            // **Un pas qui répond sans step_done n'arrête pas la tâche** (Lucie, 16 novembre 2026 :
            // « Ce qui reste à exécuter au prochain pas : … », et l'équipier attendait) : sa réponse
            // va dans la boîte, seule, en copie de l'hôte, et le pas suivant part de lui-même, sa
            // frontière disant de continuer jusqu'au bout. MAX_STALLED borne la suite.
            if declared.is_none() && r["stopped_by"].is_null() {
                let texte = r["text"].as_str().unwrap_or_default().trim();
                if texte.is_empty() {
                    // **An empty reply says why, to the user and to the agent** (23 Nov 2026, Lucie: « il faut
                    // absolument qu'on sache pourquoi un appel a foiré »).
                    let line = match r["empty_reply"].as_str() {
                        Some(pourquoi) => files.append_with(
                            "host",
                            "report",
                            None,
                            &format!("réponse vide du modèle au pas {number} : {pourquoi} ; le pas suivant reprend"),
                            &json!({ "cut": true, "why": pourquoi }),
                        )?,
                        None => files.append("host", "report", None, &format!("le pas {number} a fini sans step_done ni réponse ; le pas suivant reprend"))?,
                    };
                    publish(bus.as_ref(), task, &line);
                } else {
                    let deja = files.mailbox().iter().skip(avant).any(|l| l["from"] == "teammate" && l["kind"] == "report" && l["text"].as_str().is_some_and(|t| t.contains(texte)));
                    if !deja {
                        let line = files.append_with("teammate", "report", None, &in_her_folder(step.folders.as_ref(), texte), &json!({ "origin": "host_copy" }))?;
                        publish(bus.as_ref(), task, &line);
                    }
                }
                relance = Some(textes.get("relaunch.no_step_done", &[]));
                encore = true;
            }
            {
                let mut plans = files.read_plans();
                if let Some(p) = plans.active_mut() {
                    p.steps.push(run.clone());
                    files.write_plans(&plans)?;
                }
            }
            // Les plans clos pendant ce pas : leur L1 sur la fiche de session.
            for closed in step.closed_plans() {
                let mut closed = closed;
                if !closed.steps.contains(&run) {
                    closed.steps.push(run.clone());
                }
                files.keep_closed(&closed)?;
                // La mémoire en dernier : un rangement manqué se dit, il ne
                // bloque ni le pas ni le plan.
                if let Err(e) = self.file_closed_plan(&t, task, &request, &closed) {
                    eprintln!("[équipier] {task} : le plan {} clos ne se range pas : {e}", closed.number);
                    let _ = files.append("host", "memory_failed", None, &format!("le plan {} clos ne se range pas en mémoire : {e}", closed.number));
                }
            }
            outcome.steps.push(json!({ "step": number, "run": run, "step_done": declared, "tool_calls": step.tool_calls(), "decision": step.decision() }));
            if !trace && (!step.closed_plans().is_empty() || step.succeeded_tools().iter().any(|n| tool_leaves_a_trace(pour_les_traces.as_ref(), n))) {
                trace = true;
                if let Some(n) = demande_suivie {
                    files.write_trace(n);
                }
            }
            if !r["stopped_by"].is_null() {
                outcome.ended = format!("arrêté par {}", r["stopped_by"]);
                return Ok(outcome);
            }
            // **La fin se juge après le pas**, sur les plans : plus aucun plan
            // vivant (ni actif ni en pause) et un pas déclaré fini. Un plan actif
            // qui n'a plus rien à faire n'est pas la fin : l'agent le clôt, puis
            // reprend un plan en pause ou le clôt aussi (Lucie).
            // **Un pas qui finit sur une question attend la réponse** : il ne
            // compte ni réussi, ni raté, ni dans les MAX_STEPS. La boîte le dit
            // une fois, à l'heure où l'attente commence.
            if let Some((rang, q)) = open_question(files) {
                if rang >= debut_du_pas {
                    let heure = jiff::Zoned::now().strftime("%H:%M").to_string();
                    // The turn's facts before it waits (23 Nov 2026): its work may have landed already.
                    #[cfg(feature = "code")]
                    if let Some(l) = self.say_turn_facts(&files) {
                        publish(bus.as_ref(), task, &l);
                    }
                    let texte = format!("en attente du superviseur depuis {heure} : {} {}", q["kind"].as_str().unwrap_or("question"), q["id"].as_str().unwrap_or(""));
                    if let Ok(line) = files.append_with("host", "report", None, &texte, &json!({ "awaiting": q["id"] })) {
                        publish(bus.as_ref(), task, &line);
                    }
                }
                continue;
            }
            counted += 1;
            let after = files.read_plans();
            plan_seen |= !step.closed_plans().is_empty();
            if after.active().is_none() && after.paused().is_empty() && declared.is_some() && !unread_news(&files) {
                // **Un `step_done` qui dit ce qui reste n'est pas une fin** (ticket du 9
                // novembre 2026) : le pas suivant part ; MAX_STEPS et MAX_STALLED bornent.
                // **Still silent after its relaunch: the user reads its done** (23 Nov 2026, `~/essai-2`: twice a
                // step_done and nothing for her), in her folder's words, rather than nothing.
                if step.answered_nothing() {
                    // Twice refused, it still wrote nothing: a human line, never its done; no relaunch.
                    let line = files.append_with("host", "report", None, "l'agent n'a pas répondu ; reformule ou réessaie", &json!({ "answered_nothing": true }))?;
                    publish(bus.as_ref(), task, &line);
                    #[cfg(feature = "code")]
                    if let Some(l) = self.say_turn_facts(&files) {
                        publish(bus.as_ref(), task, &l);
                    }
                    outcome.ended = "l'agent n'a pas répondu".into();
                    return Ok(outcome);
                } else if let Some(demande) = silent_turn(&files.mailbox()).filter(|d| relancee == Some(*d)) {
                    let _ = demande;
                    if let Some(dit) = declared.as_ref().and_then(|d| d["done"].as_str()).map(str::trim).filter(|d| !d.is_empty()) {
                        let texte = in_her_folder(step.folders.as_ref(), dit);
                        let line = files.append_with("teammate", "report", None, &texte, &json!({ "origin": "step_done" }))?;
                        publish(bus.as_ref(), task, &line);
                    }
                }
                let reste = declared.as_ref().and_then(|d| d["remaining"].as_str()).map(str::trim).filter(|r| !r.is_empty()).map(str::to_string);
                if let Some(r) = reste {
                    let line = files.append("host", "report", None, &format!("pas fini : reste « {r} » ; le pas suivant part"))?;
                    publish(bus.as_ref(), task, &line);
                    encore = true;
                } else if let Some(manque) = missing_deliverable(&files.mailbox()) {
                    // **Le livrable exigé manque** : l'équipier attend la boîte, sans
                    // appeler le modèle ; une ligne du superviseur le relance.
                    #[cfg(feature = "code")]
                    if let Some(l) = self.say_turn_facts(&files) {
                        publish(bus.as_ref(), task, &l);
                    }
                    let line = files.append("host", "report", None, &format!("fait sans le livrable demandé ({manque}) : j'attends le superviseur"))?;
                    publish(bus.as_ref(), task, &line);
                    continue;
                } else if let Some(demande) = silent_turn(&files.mailbox()).filter(|d| relancee != Some(*d)) {
                    // **A turn never ends without a word to its user** (Lucie's trial, 23 Nov 2026: files written,
                    // step_done, and not a sentence to her): relaunched once, whatever the step left behind.
                    relancee = Some(demande);
                    let line = files.append("host", "report", None, "relance : l'agent n'a rien écrit à l'utilisatrice pendant ce tour ; le pas suivant lui écrit ce qu'il a fait")?;
                    publish(bus.as_ref(), task, &line);
                    relance = Some(textes.get("relaunch.silent", &[]));
                    encore = true;
                    continue;
                } else if let Some(demande) = (!trace).then(|| request_without_a_line(&files.mailbox())).flatten() {
                    if relancee != Some(demande) {
                        // **Relancé une fois** : le `step_done` qui finirait la demande sans rien
                        // laissé est cité, et le pas suivant part avec la relance. Ce n'est ni un
                        // pas raté ni un pas sans suite.
                        relancee = Some(demande);
                        let dit = declared.as_ref().and_then(|d| d["done"].as_str()).map(str::trim).filter(|d| !d.is_empty()).unwrap_or("fait");
                        let texte = textes.get("relaunch.request_unanswered", &[("done", dit)]);
                        let line = files.append("host", "report", None, &texte)?;
                        publish(bus.as_ref(), task, &line);
                        relance = Some(texte);
                        encore = true;
                        continue;
                    }
                    #[cfg(feature = "code")]
                    if let Some(l) = self.say_turn_facts(&files) {
                        publish(bus.as_ref(), task, &l);
                    }
                    let line = files.append("host", "report", None, "fait sans le livrable demandé (une ligne de l'équipier dans la boîte) : j'attends le superviseur")?;
                    publish(bus.as_ref(), task, &line);
                    attente_bornee = Some(std::time::Instant::now() + std::time::Duration::from_secs(policy.reply_wait_s));
                    continue;
                } else {
                    // **Un compte rendu n'est pas le contenu demandé** (archi, 19 novembre 2026, sur la
                    // proposition de tty) : un pas réussi sans texte, sous une request sans `expects`, dont
                    // les reports sont courts, se fait juger par le modèle de la tâche, hors du tour.
                    match self.judge_the_content(&t, &run, &files.mailbox(), avant, &texte_du_pas) {
                        ContentVerdict::Missing => {
                            let demande = demande_suivie.unwrap_or(usize::MAX);
                            if relancee != Some(demande) {
                                // La relance unique de la request, partagée avec la garde B ; un pas, compté.
                                relancee = Some(demande);
                                let line = files.append("host", "report", None, "ended without the requested content: your reports describe the work, they are not the requested text; the next step starts to write it")?;
                                publish(bus.as_ref(), task, &line);
                                relance = Some(textes.get("relaunch.text_missing", &[]));
                                encore = true;
                                continue;
                            }
                            let line = files.append("host", "report", None, "ended without the requested content, after the relaunch: waiting for the supervisor")?;
                            publish(bus.as_ref(), task, &line);
                            attente_bornee = Some(std::time::Instant::now() + std::time::Duration::from_secs(policy.reply_wait_s));
                            continue;
                        }
                        ContentVerdict::Failed(why) => {
                            let line = files.append("host", "report", None, &format!("ended without a text, to be checked (the content judge did not answer: {why})"))?;
                            publish(bus.as_ref(), task, &line);
                        }
                        ContentVerdict::NotAsked | ContentVerdict::Delivered | ContentVerdict::Action => {}
                    }
                    // **The verification gate** (the night replays of 21 Nov 2026): never « done »
                    // on a tree the declared commands find red.
                    if !policy.verify.is_empty() {
                        match self.verify_the_tree(&t.id, &policy.verify) {
                            Ok(()) => {
                                let line = files.append("host", "report", None, &format!("verification green: {}", policy.verify.join(" ; ")))?;
                                publish(bus.as_ref(), task, &line);
                            }
                            Err((commande, sortie)) => {
                                rouges += 1;
                                let tail: String = { let c: Vec<char> = sortie.chars().collect(); c[c.len().saturating_sub(3000)..].iter().collect() };
                                if rouges <= policy.verify_relaunches {
                                    let line = files.append("host", "report", None, &format!("verification red ({rouges}/{}): `{commande}` failed; the next step starts from its output", policy.verify_relaunches))?;
                                    publish(bus.as_ref(), task, &line);
                                    relance = Some(textes.get("relaunch.verify_red", &[("command", commande.as_str()), ("tail", tail.as_str())]));
                                    encore = true;
                                    continue;
                                }
                                let why = format!("failure: the verification stays red after {} relaunch(es): `{commande}`", policy.verify_relaunches);
                                let line = files.append("host", "report", None, &format!("{why}\n{tail}"))?;
                                publish(bus.as_ref(), task, &line);
                                outcome.ended = why;
                                return Ok(outcome);
                            }
                        }
                    }
                    outcome.ended = if plan_seen { "plus rien à faire : compte rendu".into() } else { "fait sans plan".into() };
                    return Ok(outcome);
                }
            }

            let _ = last;
            match &declared {
                Some(d) if d["succeeded"] == true => stalled = 0,
                _ => stalled += 1,
            }
            if stalled >= MAX_STALLED {
                let why = format!("{MAX_STALLED} pas de suite sans élément réussi : je m'arrête, le plan dit où j'en suis");
                let line = files.append("host", "report", None, &why)?;
                publish(bus.as_ref(), task, &line);
                outcome.ended = why;
                return Ok(outcome);
            }
        }
        outcome.ended = format!("{MAX_STEPS} pas atteints");
        Ok(outcome)
    }
}

/// **The path a tool's output says the system refused** (« Permission denied », « Permission non
/// accordée », « os error 13 »): the first quoted path of that line, or the line itself.
fn refused_path(rendu: &str) -> Option<String> {
    const SIGNES: [&str; 3] = ["Permission denied", "Permission non accordée", "os error 13"];
    let ligne = rendu.lines().find(|l| SIGNES.iter().any(|s| l.contains(s)))?.trim();
    for (ouvre, ferme) in [('\'', '\''), ('"', '"'), ('`', '`'), ('«', '»')] {
        let mut reste = ligne;
        while let Some(i) = reste.find(ouvre) {
            let apres = &reste[i + ouvre.len_utf8()..];
            let Some(j) = apres.find(ferme) else { break };
            let dedans = apres[..j].trim();
            if dedans.contains('/') {
                return Some(dedans.to_string());
            }
            reste = &apres[j + ferme.len_utf8()..];
        }
    }
    Some(ligne.to_string())
}

/// **What the judge of the content says** (archi, 19 November 2026): the guard « un compte rendu
/// n'est pas le contenu demandé ».
pub(crate) enum ContentVerdict {
    /// The narrow case was not met: a text was written, the request declares `expects`, or a report is long.
    NotAsked,
    Delivered,
    /// The request asked for an action, not a text: the task ends as before.
    Action,
    Missing,
    /// The judge did not answer, or not with one of its three words: the end is to be checked.
    Failed(String),
}

/// The closed question to the judge, in English (a host text for the model).
pub(crate) const CONTENT_QUESTION: &str = "Does the request ask for a text, and is that text in the step's report or answer? Answer with one word: delivered, missing or action.";

impl DaemonHost {
    /// **The judge of the content**, only in the narrow case (layer 1, read from what the step noted):
    /// no text from the model, reports of fewer than 300 characters, a request without `expects`.
    /// One call to the task's model, outside the turn (it does not enter the task's session),
    /// temperature 0, the least reasoning and room for it (archi's review: 2 805 reasoning tokens in
    /// D1 on gemini-3.8-flash would leave 50 tokens empty). Its consumption is recorded on the step's
    /// run by the chat's meter, as `content-judge`. The verdict is the answer's first word.
    fn judge_the_content(&self, t: &super::taches::Tache, run: &str, boite: &[Value], avant: usize, texte_du_pas: &str) -> ContentVerdict {
        if !texte_du_pas.trim().is_empty() {
            return ContentVerdict::NotAsked;
        }
        let Some(n) = boite.iter().rposition(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host") else {
            return ContentVerdict::NotAsked;
        };
        if !boite[n]["expects"].is_null() {
            return ContentVerdict::NotAsked;
        }
        let reports: Vec<&str> = boite.iter().skip(avant.max(n + 1)).filter(|l| l["from"] == "teammate" && l["kind"] == "report").filter_map(|l| l["text"].as_str()).collect();
        if reports.is_empty() || reports.iter().any(|r| r.chars().count() >= 300) {
            return ContentVerdict::NotAsked;
        }
        let llm = match self.with_backend_id(&t.id, |b| self.llm_du_chat(&t.manifest, &b.prepared())) {
            Ok(Ok(llm)) => llm,
            Ok(Err(e)) | Err(e) => return ContentVerdict::Failed(e),
        };
        let question = format!(
            "The request:\n{}\n\nThe step's reports:\n{}\n\nThe step's answer: none.\n\n{CONTENT_QUESTION}",
            boite[n]["text"].as_str().unwrap_or_default(),
            reports.join("\n---\n")
        );
        let opts = crate::llm::GenOptions { max_tokens: 512, temperature: 0.0, reasoning: Some(crate::llm::ReasoningEffort::Minimal), ..Default::default() };
        let mut sink = crate::llm::StringSink::default();
        match llm.generate(&[crate::llm::Turn::user(question)], &opts, &mut sink) {
            Err(e) => ContentVerdict::Failed(e.to_string()),
            Ok((_, usage)) => {
                let nom = llm.name().rsplit('/').next().unwrap_or(llm.name());
                let c = crate::meter::Consumption::new(format!("llm.{nom}"), "unknown")
                    .by(run, "content-judge")
                    .with(crate::meter::Unit::InputTokens, usage.prompt_tokens.saturating_sub(usage.cached_prompt_tokens) as u64)
                    .with(crate::meter::Unit::CachedInputTokens, usage.cached_prompt_tokens as u64)
                    .with(crate::meter::Unit::OutputTokens, usage.completion_tokens.saturating_sub(usage.reasoning_tokens) as u64)
                    .with(crate::meter::Unit::ReasoningTokens, usage.reasoning_tokens as u64);
                if !c.is_empty() {
                    self.chat_meter.record(c);
                }
                let mot = sink.text.split(|c: char| !c.is_alphabetic()).find(|w| !w.is_empty()).unwrap_or_default().to_lowercase();
                eprintln!("[équipier] {} : le juge du contenu dit « {} »", t.nom, sink.text.trim());
                match mot.as_str() {
                    "missing" => ContentVerdict::Missing,
                    "delivered" => ContentVerdict::Delivered,
                    "action" => ContentVerdict::Action,
                    _ => ContentVerdict::Failed(format!("unreadable answer: « {} »", sink.text.trim())),
                }
            }
        }
    }
}

/// **Un plan clos, lisible** : le L0 de sa source, et la tête de son L1.
pub fn render_plan(plan: &Plan) -> String {
    let etat = match plan.state {
        PlanState::Accomplished => "accompli",
        PlanState::Obsolete => "obsolète",
        PlanState::Paused => "en pause",
        PlanState::Active => "actif",
    };
    let mut t = format!("**Plan n° {}** — {etat}{}.\n\n**Demande** : {}\n\n**Proposition** : {}\n", plan.number, plan.reason.as_deref().map(|r| format!(" ({r})")).unwrap_or_default(), plan.request, plan.proposal);
    for (statut, titre) in [(Status::Done, "Faits"), (Status::Dropped, "Abandonnés"), (Status::Blocked, "Bloqués"), (Status::Doing, "En cours"), (Status::Todo, "À faire")] {
        let items: Vec<&Item> = plan.items.iter().filter(|i| i.status == statut).collect();
        if items.is_empty() {
            continue;
        }
        t.push_str(&format!("\n{titre} :\n"));
        for i in items {
            let q = i.question.as_deref().map(|q| format!(" — question : {q}")).unwrap_or_default();
            let r = i.reason.as_deref().map(|r| format!(" — {r}")).unwrap_or_default();
            t.push_str(&format!("- {} : {}{q}{r}\n", i.id, i.text));
        }
    }
    if !plan.concepts.is_empty() {
        t.push_str(&format!("\nNotions, selon l'agent : {}.\n", plan.concepts.join(", ")));
    }
    t.push_str(&format!("\n_{} pas : {}._\n", plan.steps.len(), plan.steps.join(", ")));
    t
}

impl DaemonHost {
    /// **Un plan clos va sur la fiche de session** (page 07, forme de 96) : un L1
    /// de section `plan`, de source `sources/<jour>-<tâche>-plan-<n>.md`, avec
    /// le plan lisible et son JSON ; ses références sur l'apport par
    /// `citer_source` : `path:` les fichiers que ses pas ont écrits, `outil:`
    /// les outils qu'ils ont appelés, `doc:` les bilans du juge de ces runs (la
    /// Ref même que le juge pose). Rien si le backend ne déclare pas `bilans`.
    pub fn file_closed_plan(&self, t: &super::taches::Tache, task: &str, request: &str, plan: &Plan) -> Result<Option<Value>, String> {
        let cible = self.hote_par_id(&t.id)?;
        let Some(memoire_manifeste) = self.memoire_declaree(&cible)? else { return Ok(None) };
        let memoire = self.hosted(&memoire_manifeste)?;
        // Made for the task's agent (M7): `writes.author` signs the fiche, the Source and the Apport with its key.
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { crate::backend::as_agent(t.agent_key.as_deref(), || self.with_backend(&memoire, |b| b.call(outil, args))).and_then(|r| r) };
        // La fiche de la tâche, sous son nom court : celle où le juge range les bilans
        // de ses pas (`juge::nom_de_fiche_de_tache`).
        let _ = request;
        let fiche = super::juge::nom_de_fiche_de_tache(task);
        {
            let nom = json!({ "name": fiche }).to_string();
            let nom = &nom[1..nom.len() - 1];
            let existe = appeler("get_fiche", json!({ "record": { "name": fiche } })).is_ok_and(|r| r["result"].to_string().contains(nom));
            if !existe {
                appeler("put_fiche", json!({ "record": { "name": fiche, "about": request, "state": "current", "via": "teammate" } }))?;
            }
        }
        let jour = super::juge::chrono_jour();
        let propre = |s: &str| s.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "-");
        let chemin = format!("sources/{jour}-{}-plan-{}.md", propre(task), plan.number);
        let lisible = render_plan(plan);
        let json_du_plan = serde_json::to_string_pretty(plan).unwrap_or_default();
        appeler(
            "put_source",
            json!({ "record": { "chemin": chemin, "titre": format!("le plan {} de la tâche {task}", plan.number), "contenu": format!("# Plan {} de la tâche {task}\n\n{lisible}\n```json\n{json_du_plan}\n```\n", plan.number), "origine": "envoi", "session": format!("teammate-{task}"), "via": "teammate" } }),
        )?;
        appeler(
            "put_apport",
            json!({ "record": { "fiche": fiche, "source": chemin, "section": "plan", "texte": format!("{lisible}\n```json\n{json_du_plan}\n```\n"), "niveau": "L1", "state": "current", "via": "teammate" } }),
        )?;
        appeler("relier_apport", json!({ "links": [{ "from": { "name": fiche }, "to": { "fiche": fiche, "source": chemin, "section": "plan" } }] }))?;
        // Les références de l'apport : ce que ce plan a touché.
        let mut refs: Vec<(&str, String)> = vec![("doc", format!("{chemin}#plan"))];
        let mut paths = Vec::new();
        let mut outils = Vec::new();
        for run in &plan.steps {
            let Some(j) = self.journal.lire(&t.id, run) else { continue };
            paths.extend(j["fichiers"].as_array().into_iter().flatten().filter_map(|f| f.as_str().map(str::to_string)));
            outils.extend(j["evenements"].as_array().into_iter().flatten().filter(|e| e["kind"] == "ToolCallStarted").filter_map(|e| e["tool"].as_str().map(str::to_string)));
            // Le jour du run : celui de son journal, comme le juge l'a daté.
            let jour_du_run = self.journal.fichier(&t.id, run).and_then(|p| std::fs::metadata(p).ok()).and_then(|m| m.modified().ok()).and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| super::juge::jour_depuis(d.as_secs())).unwrap_or_else(|| jour.clone());
            let source_du_run = format!("sources/{jour_du_run}-{}.md", propre(run));
            refs.push(("doc", format!("{source_du_run}#bilan")));
        }
        paths.sort();
        paths.dedup();
        outils.sort();
        outils.dedup();
        outils.retain(|o| !matches!(o.as_str(), EDIT_PLAN | STEP_DONE | REPORT | SUGGEST));
        refs.extend(paths.iter().map(|p| ("path", format!("path:{p}"))));
        refs.extend(outils.iter().map(|o| ("outil", format!("outil:{o}"))));
        for genre in ["doc", "path", "outil"] {
            let _ = appeler("create_ref_type", json!({ "genre": genre }));
        }
        for (genre, valeur) in &refs {
            if let Err(e) = appeler("citer_source", json!({ "genre": genre, "valeur": valeur, "fiche": { "fiche": fiche, "source": chemin, "section": "plan" } })) {
                eprintln!("[équipier] la référence {valeur} du plan {} : {e}", plan.number);
            }
        }
        Ok(Some(json!({ "fiche": fiche, "source": chemin, "refs": refs.iter().map(|(_, v)| v).collect::<Vec<_>>() })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(d: &Path) -> Step {
        Step::new(TeammateFiles::new(d.to_path_buf(), "p/t"), "t", "la demande", None, 1)
    }

    #[test]
    fn a_path_refused_twice_stops_the_steps_work_saying_so() {
        let d = tempfile::tempdir().unwrap();
        let s = step(d.path());
        let refus = |chemin: &str| Turn::tool_result("c", "run_command", format!("### `git add x`\n\ncode 128 · 2ms\n\nstderr :\n```\nfatal : Impossible de créer '{chemin}' : Permission non accordée\n```\n"));
        let lire = call("read_file", json!({ "path": "a" }));
        s.note_tool(&call("run_command", json!({ "command": "git add x" })), &refus("/r/.git/worktrees/x/index.lock"));
        s.note_tool(&call("run_command", json!({ "command": "git add y" })), &refus("/r/.git/autre"));
        assert!(s.gate(&lire).is_none(), "two paths once each: the step goes on");
        s.note_tool(&call("run_command", json!({ "command": "git commit" })), &refus("/r/.git/worktrees/x/index.lock"));
        let r = s.gate(&lire).expect("the same path twice: the work stops");
        assert!(r.content.contains("index.lock") && r.content.contains("twice") && r.content.contains("step_done"), "{}", r.content);
        let boite = TeammateFiles::new(d.path().to_path_buf(), "p/t").mailbox();
        assert!(boite.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("index.lock") && t.contains("twice"))), "said in the mailbox: {boite:#?}");
        assert_eq!(refused_path("Permission denied (os error 13)").as_deref(), Some("Permission denied (os error 13)"), "no quoted path: the line");
        assert_eq!(refused_path("tout va bien"), None);
    }

    fn call(name: &str, args: Value) -> ToolCall {
        ToolCall { id: "c".into(), name: name.into(), arguments: args.to_string(), provider_extra: None, at_ms: None }
    }

    /// Le plan s'écrit, ses questions partent dans la boîte, `step_done` passe
    /// l'élément à done ; le plan se clôt et un autre s'ouvre.
    #[test]
    fn a_provider_refusal_is_read_from_the_turn_error_and_the_wait_doubles() {
        assert_eq!(super::refus_du_fournisseur("le chat : llm indisponible : quota dépassé (HTTP 429) après 2 reprise(s) en 186.0 s — RESOURCE_EXHAUSTED"), Some(false));
        assert_eq!(super::refus_du_fournisseur("llm indisponible : service en panne (HTTP 503) après 3 reprise(s) en 9.0 s — x"), Some(false));
        assert_eq!(super::refus_du_fournisseur("llm indisponible : quota épuisé (HTTP 429), il ne revient pas en attendant — x"), Some(true));
        assert_eq!(super::refus_du_fournisseur("llm model error: boum"), None);
        assert_eq!((1..=6).map(|n| super::attente_du_fournisseur(60, n)).collect::<Vec<_>>(), vec![60, 120, 240, 480, 600, 600]);
    }

    #[test]
    fn the_plan_is_free_and_its_questions_go_to_the_mailbox() {
        let d = tempfile::tempdir().unwrap();
        let s = step(d.path());
        let r = s.ours(&call(EDIT_PLAN, json!({ "plan": { "request": "r", "proposal": "p", "items": [
            { "id": "a", "text": "A", "status": "todo", "alone": true },
            { "id": "b", "text": "B", "status": "blocked", "alone": false, "question": "on garde le titre ?" }
        ] } }))).unwrap();
        assert!(r.content.contains("\"questions_sent\":1"), "{}", r.content);
        let boite = s.files.mailbox();
        assert_eq!((boite[0]["kind"].clone(), boite[0]["item"].clone()), (json!("question"), json!("b")));
        assert_eq!(s.files.read_plans().active().unwrap().next_alone().unwrap().id, "a");
        s.ours(&call(STEP_DONE, json!({ "item": "a", "succeeded": true, "done": "A écrit", "remaining": "" }))).unwrap();
        assert_eq!(s.files.read_plans().active().unwrap().items[0].status, Status::Done);
        assert_eq!(s.declared().unwrap()["item"], "a");
        let r = s.ours(&call(EDIT_PLAN, json!({ "close": { "outcome": "obsolete", "reason": "la demande a changé", "concepts": ["titre"] }, "plan": { "request": "r2", "proposal": "p2", "items": [] } }))).unwrap();
        assert!(r.content.contains("\"closed\":1"), "{}", r.content);
        assert_eq!(s.files.read_plans().active().unwrap().number, 2);
        let clos = s.files.closed();
        assert_eq!((clos[0].state, clos[0].concepts.clone()), (PlanState::Obsolete, vec!["titre".to_string()]));
        assert_eq!(s.closed_plans().len(), 1);
    }

    /// **Un plan actif oblige** (Lucie) : un second plan est refusé tant que
    /// l'actif n'est ni en pause ni clos ; en pause, il reste dans plan.json et
    /// la frontière le rappelle ; repris, il redevient l'actif.
    #[test]
    fn one_active_plan_and_paused_plans_are_recalled() {
        let d = tempfile::tempdir().unwrap();
        let s = step(d.path());
        s.ours(&call(EDIT_PLAN, json!({ "plan": { "request": "la doc", "proposal": "p", "items": [{ "id": "a", "text": "A", "status": "todo" }] } }))).unwrap();
        let refus = s.ours(&call(EDIT_PLAN, json!({ "plan": { "request": "autre", "proposal": "p", "items": [] } }))).unwrap();
        assert!(refus.content.contains("déjà actif"), "{}", refus.content);
        s.ours(&call(EDIT_PLAN, json!({ "pause": { "reason": "le superviseur demande autre chose" }, "plan": { "request": "centre le div", "proposal": "p", "items": [{ "id": "c", "text": "C", "status": "todo" }] } }))).unwrap();
        assert_eq!(s.decision().as_deref(), Some("pause"));
        let plans = s.files.read_plans();
        assert_eq!((plans.active().unwrap().number, plans.paused()[0].number), (2, 1));
        let m = frontier_message(3, &[], &plans, false, "", &[], &crate::daemon_host::prompts::Texts::shipped());
        assert!(m.contains("plans en pause") && m.contains("la doc") && m.contains("le superviseur demande autre chose"), "{m}");
        s.ours(&call(EDIT_PLAN, json!({ "close": { "outcome": "accomplished", "reason": "centré" } }))).unwrap();
        let m = frontier_message(4, &[], &s.files.read_plans(), false, "", &[], &crate::daemon_host::prompts::Texts::shipped());
        assert!(m.contains("reprends un plan en pause"), "{m}");
        s.ours(&call(EDIT_PLAN, json!({ "resume": 1 }))).unwrap();
        let plans = s.files.read_plans();
        assert_eq!((plans.active().unwrap().number, plans.paused().len()), (1, 0));
    }

    /// Sans plan, la frontière dit d'agir ; une demande pendant un plan actif
    /// demande une décision.
    /// **L'équipier sait à qui il parle** (second essai de Lucie, 15 novembre 2026 : un report
    /// signé « Réponse apportée au superviseur ») : dans une tâche ouverte par le terminal ou le
    /// chat, le superviseur est l'utilisateur ; il lui répond directement, une fois par tour.
    /// **Le pas va au bout de la demande** (16 novembre 2026) : l'invite ne présente plus le pas
    /// comme une unité de compte rendu.
    #[test]
    fn the_teammate_prompt_says_to_work_the_request_to_its_end() {
        assert!(SYSTEM_PROMPT.contains("until the request is done"), "{SYSTEM_PROMPT}");
        assert!(!SYSTEM_PROMPT.contains("A step is one piece of work"), "{SYSTEM_PROMPT}");
    }

    #[test]
    fn the_teammate_prompt_says_the_supervisor_is_the_user() {
        assert!(SYSTEM_PROMPT.contains("the supervisor is the user"), "{SYSTEM_PROMPT}");
        assert!(SYSTEM_PROMPT.contains("once per turn") && SYSTEM_PROMPT.contains("report done"), "{SYSTEM_PROMPT}");
        // La réponse est du texte libre, qui streame (c7, 15 novembre) ; report n'en est plus la voie.
        assert!(SYSTEM_PROMPT.contains("you speak to them in the text of your reply"), "{SYSTEM_PROMPT}");
    }

    #[test]
    fn the_frontier_says_what_the_step_expects() {
        let m = frontier_message(1, &[json!({ "kind": "request", "from": "c7", "text": "Bonjour" })], &Plans::default(), false, "", &[], &crate::daemon_host::prompts::Texts::shipped());
        assert!(m.contains("Bonjour") && m.contains("Réponds ou agis"), "{m}");
        let plans = Plans { plans: vec![Plan { number: 1, request: "r".into(), proposal: "p".into(), items: vec![], state: PlanState::Active, reason: None, steps: vec![], concepts: vec![] }] };
        let m = frontier_message(2, &[json!({ "kind": "request", "from": "c7", "text": "autre chose" })], &plans, false, "", &[], &crate::daemon_host::prompts::Texts::shipped());
        assert!(m.contains("décide d'abord"), "{m}");
    }

    /// Au-delà du budget, tout appel autre que `step_done` est refusé.
    #[test]
    fn past_the_budget_only_step_done_passes() {
        let d = tempfile::tempdir().unwrap();
        let s = step(d.path());
        for _ in 0..crate::backend::TeammatePolicy::default().max_tool_calls_per_step {
            assert!(s.gate(&call("read_file", json!({}))).is_none());
        }
        let refus = s.gate(&call("read_file", json!({}))).expect("refusé");
        assert!(refus.content.contains("step_done"), "{}", refus.content);
        let refus = s.ours(&call(EDIT_PLAN, json!({ "continue": true }))).unwrap();
        assert!(refus.content.contains("teammate.max_tool_calls_per_step"), "{}", refus.content);
        assert!(s.ours(&call(STEP_DONE, json!({ "item": "", "succeeded": false, "done": "", "remaining": "tout" }))).unwrap().content.contains("\"ok\":true"));
    }
}

#[cfg(test)]
#[path = "teammate/context_share_tests.rs"]
mod context_share_tests;

#[cfg(test)]
mod carry_over_tests {
    //! **The scripted `Previous step`, against the Rust one** (validation of the scripted node, 21 November 2026).
    use super::*;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// The shipped node, copied into a folder of its own: the tests edit their copy.
    fn nodes() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code/nodes");
        let dir = d.path().join("nodes");
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["previous_step.node.json", "previous_step.ts"] {
            std::fs::copy(src.join(f), dir.join(f)).unwrap();
        }
        (d, dir)
    }

    fn works() -> Vec<(StepWork, String, usize)> {
        let w = |read: &[&str], wrote: &[&str], command: Option<(&str, &str)>| StepWork {
            read: read.iter().map(|s| s.to_string()).collect(),
            wrote: wrote.iter().map(|s| s.to_string()).collect(),
            command: command.map(|(c, o)| (c.to_string(), o.to_string())),
            results: Vec::new(),
        };
        let long: String = "é😀".repeat(5_000);
        vec![
            (w(&[], &[], None), String::new(), 8_000),
            (w(&[], &[], None), "  \n ".into(), 8_000),
            (w(&["src/a.rs (lines 1-120)", "src/é.rs (lines 40-end)"], &["src/a.rs"], Some(("cargo test", "ok\n"))), "  Je lis, puis j'édite.  ".into(), 8_000),
            (w(&["src/a.rs (lines 1-3)"], &[], None), "fini".into(), 8_000),
            // Over the budget, with characters outside the BMP right at the cut: a character is not a UTF-16 unit.
            (w(&["src/b.rs (lines 1-9)"], &["src/b.rs"], Some(("make", &long))), long.clone(), 8_000),
            (w(&["x (lines 1-2)"], &[], Some(("ls", "😀😀😀"))), "😀".repeat(200), 150),
            (w(&["x (lines 1-2)"], &[], None), "z".into(), 10),
        ]
    }

    /// **The shipped script gives the Rust block, to the byte**, on every state, the cut included (accents and
    /// characters outside the BMP: the script counts code points, as `chars()`).
    #[test]
    fn the_scripted_previous_step_is_the_rust_one_to_the_byte() {
        let (_d, dir) = nodes();
        for (i, (w, texte, budget)) in works().into_iter().enumerate() {
            let rust = w.block(3, &texte, budget);
            let script = carry_over_by_node(&dir, "PreviousStep", &Value::Null, w.to_json(3, &texte, budget))
                .unwrap_or_else(|e| panic!("state {i}: the script failed: {e}"));
            assert!(script == rust, "state {i}: the script differs from the Rust block ({} and {} characters)", script.chars().count(), rust.chars().count());
        }
    }

    /// **An edited script serves at the next call**, without restarting anything; **two configurations** of the same
    /// script, on the same input, give their two blocks.
    #[test]
    fn an_edited_script_serves_at_the_next_call_and_two_configurations_give_two_blocks() {
        let (_d, dir) = nodes();
        let (w, texte, budget) = works().remove(2);
        let complet = carry_over_by_node(&dir, "PreviousStep", &json!({"style": "full"}), w.to_json(3, &texte, budget)).unwrap();
        let compact = carry_over_by_node(&dir, "PreviousStep", &json!({"style": "compact"}), w.to_json(3, &texte, budget)).unwrap();
        assert!(complet.contains("Its output ends with:") && !compact.contains("Its output ends with:"), "{complet:?} / {compact:?}");
        let ts = dir.join("previous_step.ts");
        let edite = std::fs::read_to_string(&ts).unwrap().replace("## Previous step (step", "## The step before (step");
        std::fs::write(&ts, edite).unwrap();
        let apres = carry_over_by_node(&dir, "PreviousStep", &Value::Null, w.to_json(3, &texte, budget)).unwrap();
        assert!(apres.contains("## The step before (step 3)"), "the edit is not served: {apres:?}");
    }

    /// **The cost of the scripted block, against the Rust one** (a measure, not a witness: `--ignored`, in release).
    /// A state of real size (200 reads, 50 writes, a command and its 1 500 last characters, a last text of 3 000
    /// characters, over the 8 000 budget): the Rust block; the node as a step calls it (the declaration read and the
    /// script prepared again, which gives the hot reload); the script alone, prepared once.
    #[test]
    #[ignore]
    fn measure_the_scripted_previous_step_against_the_rust_one() {
        let (_d, dir) = nodes();
        let w = StepWork {
            read: (0..200).map(|i| format!("src/module_{i}/fichier_é.rs (lines {i}-{})", i + 40)).collect(),
            wrote: (0..50).map(|i| format!("src/écrit_{i}.rs")).collect(),
            command: Some(("cargo test --lib".into(), "test ok 😀\n".repeat(150))),
            results: Vec::new(),
        };
        let texte = "Je relis le module, puis j'édite la fonction. ".repeat(65);
        let n = 200;
        let mediane = |mut v: Vec<std::time::Duration>| {
            v.sort();
            v[v.len() / 2]
        };
        let rust: Vec<_> = (0..n).map(|_| { let t = std::time::Instant::now(); std::hint::black_box(w.block(7, &texte, 8_000)); t.elapsed() }).collect();
        let noeud: Vec<_> = (0..n)
            .map(|_| {
                let t = std::time::Instant::now();
                std::hint::black_box(carry_over_by_node(&dir, "PreviousStep", &Value::Null, w.to_json(7, &texte, 8_000)).unwrap());
                t.elapsed()
            })
            .collect();
        use crate::dataflow::node_registry::NodeFactory;
        let decl = crate::dataflow::scripted_node::read_declaration(&dir.join("previous_step.node.json")).unwrap();
        let fabrique = crate::dataflow::scripted_node::ScriptedNodeFactory::new(decl, crate::script::ScriptLimits::default()).unwrap();
        let script: Vec<_> = (0..n)
            .map(|_| {
                let mut noeud = fabrique.create("m", &Value::Null).unwrap();
                let mut ctx = crate::dataflow::node::NodeContext::new();
                ctx.set_input("work", crate::dataflow::port::PortValue::new(w.to_json(7, &texte, 8_000)));
                let t = std::time::Instant::now();
                noeud.execute(&mut ctx).unwrap();
                t.elapsed()
            })
            .collect();
        assert_eq!(carry_over_by_node(&dir, "PreviousStep", &Value::Null, w.to_json(7, &texte, 8_000)).unwrap(), w.block(7, &texte, 8_000));
        eprintln!(
            "[mesure] Previous step, médianes sur {n} appels : Rust {:?} ; nœud tel qu'un pas l'appelle (relu et préparé) {:?} ; script seul, préparé une fois {:?}",
            mediane(rust),
            mediane(noeud),
            mediane(script)
        );
    }

    /// **A broken script never breaks the step**: the Rust block serves, and the error names the script's file and
    /// its line.
    #[test]
    fn a_broken_script_falls_back_to_the_rust_block_and_names_its_file_and_line() {
        let (_d, dir) = nodes();
        let ts = dir.join("previous_step.ts");
        let casse = std::fs::read_to_string(&ts).unwrap().replace("  const w = inputs.work;", "  const w = inputs.work;\n  throw new Error(\"cassé exprès\");");
        std::fs::write(&ts, casse).unwrap();
        let step = Step::new(TeammateFiles::new(_d.path().join("boite"), "t"), "t", "r", None, 3);
        step.work.lock().unwrap().read.push("src/a.rs (lines 1-3)".into());
        let (bloc, erreur) = step.carry_over_with("fini", 8_000, Some((&dir, "PreviousStep", &Value::Null)));
        assert_eq!(bloc, step.carry_over("fini", 8_000), "the Rust block serves");
        let e = erreur.expect("the error is said");
        assert!(e.contains("previous_step.ts") && e.contains("cassé exprès") && e.contains("line"), "{e}");
    }

    /// The witness of the lot `wasm-node-trial`, in a file of its own.
    #[cfg(feature = "wasm-nodes")]
    mod wasm_previous_step;

    /// **The facts the code index gave stay for the next step** (the night of 24 Nov 2026: the step that searched found
    /// `teammate_loop` at `teammate.rs:2415`; the next one, which wrote the report, had only prose, and invented 13
    /// names). The shipped node keeps the `name — path:line` lines of the index tools' results, and over the budget it
    /// cuts the prose before them.
    #[test]
    fn the_index_facts_survive_into_the_next_step() {
        let (_d, dir) = nodes();
        let w = StepWork {
            read: vec!["src/daemon_host/teammate.rs (lines 2400-2460)".into()],
            wrote: Vec::new(),
            command: None,
            results: vec![
                ("usages".into(), "**1 usage** of `teammate_loop`\n- method teammate_loop — src/daemon_host/teammate.rs:2415, called by run_teammate (src/daemon_host/teammate.rs:2363)\n".into()),
                ("read_file".into(), "fn quelque_chose() — src/ailleurs.rs:12\n".into()),
            ],
        };
        let prose = "Je cherche la boucle de l'équipier. ".repeat(40);
        for budget in [8_000, 400] {
            let block = carry_over_by_node(&dir, "PreviousStep", &json!({}), w.to_json(3, &prose, budget)).unwrap();
            assert!(block.contains("teammate_loop — src/daemon_host/teammate.rs:2415"), "the index's fact is kept (budget {budget}): {block}");
            assert!(!block.contains("src/ailleurs.rs:12"), "a file's content is no fact of the index: {block}");
        }
    }
}

/// The witness of the lot `step-hooks`, in a file of its own.
#[cfg(test)]
mod step_graph_tests;
