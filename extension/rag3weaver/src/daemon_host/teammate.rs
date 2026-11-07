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
//!   `step_done`, un outil qui termine la boucle. Au-delà de [`STEP_BUDGET`]
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
/// Les appels d'outil permis dans un pas avant que `step_done` soit exigé.
pub const STEP_BUDGET: usize = 12;
/// Les pas d'un équipier lancé, au plus (un garde-fou, pas un réglage du grain).
const MAX_STEPS: usize = 40;
/// Les pas de suite sans `step_done` réussi avant que l'équipier s'arrête et le
/// dise : il tourne en rond.
const MAX_STALLED: usize = 3;

/// Ce que l'équipier lit en plus du message système du chat dans une tâche.
const SYSTEM_PROMPT: &str = "\
You are a teammate: you carry out tasks on your own, step by step, and talk to a supervisor through a mailbox.
- A plan is not mandatory. A greeting or a small change (center a div) needs none: answer or act, then call step_done. Write a mini-plan (edit_plan) only when the request needed a real analysis or splits into several items.
- A plan holds the request as you understand it, what you propose, then items. Each item has a status (todo, doing, done, blocked, dropped) and is either `alone` (doable without validation) or needs validation, with the `question` you ask; questions are sent to the supervisor when you set them. Edit it whenever you see fit; close it (accomplished or obsolete, with a reason) when it is.
- At most one plan is active. When a new request arrives while a plan is active, decide explicitly with edit_plan: `continue` (the request joins it or waits), `pause` (with a reason), or `close` as obsolete. A paused plan stays listed at every step until you `resume` it or close it.
- A step is one piece of work: do it with your tools, then call step_done { item, succeeded, done, remaining } (item may be empty without a plan). step_done ends the step.
- Maximum autonomy: you close your items and your plan (accomplished) yourself, and you ask nothing about what is in the plan. A `question` is only for a real blocker you cannot decide. Never wait for an answer: send the blocking questions first, then work on what you can do alone.
- Out of plan: when you see a direction the request did not ask for, write a short proposal to the supervisor (suggest: « voudrais-tu qu'on pousse dans ce sens ? », with the item it would open), then keep working on the current plan. Do not open that direction without the supervisor's word; the answer arrives at a later step, like an answer to a blocker.
- Each step starts with the mailbox (answers, new requests) and your plans. Read them before acting.
- At most 12 tool calls per step; after that, only step_done is accepted.
- When nothing is left, report what was done (report), close the active plan as accomplished, and call step_done.
- Write plans, reports and step_done texts in French.";

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
const AWAITS_SEEN: [&str; 3] = ["report", "ended", "delivery"];

impl TeammateFiles {
    pub fn new(dir: PathBuf, key: impl Into<String>) -> Self {
        TeammateFiles { dir, key: key.into(), relay: None }
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

/// Le sujet du bus de la boîte d'une tâche.
pub fn mailbox_topic(task: &str) -> String {
    format!("mailbox.{task}")
}

/// **Ce qu'un pas sait de lui-même** : les outils de l'équipier s'en servent,
/// la boucle le lit après le tour.
pub struct Step {
    pub files: TeammateFiles,
    pub task: String,
    /// La demande d'origine (la première `request` de la boîte) : le nom de la
    /// fiche de session, pour le juge comme pour les plans clos.
    pub request: String,
    pub bus: Option<crate::events::EventBus>,
    pub number: usize,
    calls: AtomicUsize,
    /// Ce que `step_done` a déclaré.
    done: Mutex<Option<Value>>,
    /// Les plans clos pendant ce pas (à ranger en L1).
    closed: Mutex<Vec<Plan>>,
    /// L'agent a-t-il tranché sur le plan actif (continue, pause, close) ?
    decided: Mutex<Option<String>>,
    /// Par où l'équipier s'est réveillé avant ce pas (`bus`, `fichier`), pour le journal.
    pub woke_by: Option<String>,
}

impl Step {
    pub fn new(files: TeammateFiles, task: &str, request: &str, bus: Option<crate::events::EventBus>, number: usize) -> Self {
        Self {
            files,
            task: task.to_string(),
            request: request.to_string(),
            bus,
            number,
            calls: AtomicUsize::new(0),
            done: Mutex::new(None),
            closed: Mutex::new(Vec::new()),
            decided: Mutex::new(None),
            woke_by: None,
        }
    }
    pub fn system_prompt(&self) -> &'static str {
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
        let line = self.files.append("teammate", kind, item, text)?;
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

    fn refusal(&self, call: &ToolCall) -> Option<Turn> {
        Some(Turn::tool_result(
            &call.id,
            &call.name,
            json!({ "ok": false, "error": format!("budget du pas atteint ({STEP_BUDGET} appels) : déclare d'abord step_done (réussi ou non, ce qui est fait, ce qui reste) ; le pas suivant reprendra") }).to_string(),
        ))
    }

    /// **Le budget du pas** : chaque appel aux outils de travail est compté, et
    /// refusé au-delà. Le refus est une réponse d'outil : l'agent le lit.
    fn gate(&self, call: &ToolCall) -> Option<Turn> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if n > STEP_BUDGET {
            self.refusal(call)
        } else {
            None
        }
    }

    pub fn tool_defs() -> Vec<ToolDef> {
        let item = json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "text": { "type": "string" },
                "status": { "type": "string", "enum": ["todo", "doing", "done", "blocked", "dropped"] },
                "alone": { "type": "boolean", "description": "true si tu peux le faire sans validation du superviseur" },
                "question": { "type": "string", "description": "la question posée au superviseur, si l'élément demande une validation" },
                "reason": { "type": "string" }
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
                            "properties": {
                                "request": { "type": "string", "description": "la demande telle que tu la comprends" },
                                "proposal": { "type": "string", "description": "ce que tu proposes" },
                                "items": { "type": "array", "items": item.clone() }
                            },
                            "required": ["request", "proposal", "items"]
                        },
                        "item": item,
                        "remove": { "type": "string", "description": "l'id d'un élément à retirer" },
                        "continue": { "type": "boolean" },
                        "pause": { "type": "object", "properties": { "reason": { "type": "string" } }, "required": ["reason"] },
                        "resume": { "type": "integer", "description": "le numéro d'un plan en pause" },
                        "close": {
                            "type": "object",
                            "properties": {
                                "outcome": { "type": "string", "enum": ["accomplished", "obsolete"] },
                                "reason": { "type": "string" },
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
                        "item": { "type": "string" },
                        "succeeded": { "type": "boolean" },
                        "done": { "type": "string" },
                        "remaining": { "type": "string" }
                    },
                    "required": ["item", "succeeded", "done", "remaining"]
                }),
            },
            ToolDef {
                name: SUGGEST.into(),
                description: "Propose au superviseur une direction hors du plan (« voudrais-tu qu'on pousse dans ce sens ? »), avec l'id de l'élément qu'elle ouvrirait. Tu continues le plan courant sans attendre ; tu n'ouvres pas cette direction sans sa réponse.".into(),
                parameters: json!({ "type": "object", "properties": { "text": { "type": "string" }, "item": { "type": "string", "description": "l'id de l'élément que la proposition ouvrirait" } }, "required": ["text"] }),
            },
            ToolDef {
                name: REPORT.into(),
                description: "Rend compte au superviseur, dans sa boîte : ce qui a été fait, ce qui attend, ce que tu as décidé.".into(),
                parameters: json!({ "type": "object", "properties": { "text": { "type": "string" } }, "required": ["text"] }),
            },
        ]
    }
}

/// **Les outils d'un pas** : ceux du chat dans la tâche, plus `edit_plan`,
/// `step_done` et `report`, sous le budget du pas.
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
    #[cfg(feature = "code")]
    fn after(&self, call: &ToolCall, mut turn: Turn) -> Turn {
        if turn.content.contains(crate::dataflow::run_nodes::EN_ATTENTE) {
            let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
            if let Some(ligne) = args["command"].as_str() {
                let texte = format!("commande en attente d'accord : {ligne}");
                if let Ok(line) = self.step.files.append_with("teammate", "approval", None, &texte, &json!({ "command": ligne })) {
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
    fn after(&self, _call: &ToolCall, turn: Turn) -> Turn {
        turn
    }
}

impl crate::agent::ToolBox for StepTools<'_> {
    fn call(&self, call: &ToolCall) -> Turn {
        self.step.ours(call).or_else(|| self.step.gate(call)).unwrap_or_else(|| self.after(call, self.inner.call(call)))
    }
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        self.step.ours(call).or_else(|| self.step.gate(call)).unwrap_or_else(|| self.after(call, self.inner.call_in(call, run)))
    }
    fn call_with(&self, call: &ToolCall, run: &str, inv: &crate::agent::ToolInvocation) -> Turn {
        self.step.ours(call).or_else(|| self.step.gate(call)).unwrap_or_else(|| self.after(call, self.inner.call_with(call, run, inv)))
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        // Les nôtres gagnent sur un homonyme du backend : un fournisseur refuse
        // deux déclarations du même nom (Vertex : « Duplicate function declaration »).
        let ours = Step::tool_defs();
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
fn frontier_message(step: usize, news: &[Value], plans: &Plans, last: bool) -> String {
    let mut m = format!("[Pas {step}]\n");
    if !news.is_empty() {
        m.push_str("\nBoîte aux lettres, depuis le pas précédent :\n");
        for l in news {
            let item = l["item"].as_str().map(|i| format!(" (élément {i})")).unwrap_or_default();
            let accord = l["granted"].as_str().map(|g| format!(" — {g}")).unwrap_or_default();
            m.push_str(&format!("- {} de {}{item} : {}{accord}\n", l["kind"].as_str().unwrap_or("?"), l["from"].as_str().unwrap_or("?"), l["text"].as_str().unwrap_or("")));
        }
    }
    let active = plans.active();
    let paused = plans.paused();
    if let Some(p) = active {
        m.push_str(&format!("\nTon plan actif (n° {}) :\n```json\n{}\n```\n", p.number, serde_json::to_string_pretty(p).unwrap_or_default()));
    }
    if !paused.is_empty() {
        m.push_str("\nTes plans en pause (à reprendre ou à clore, jamais à oublier) :\n");
        for p in &paused {
            m.push_str(&format!("- n° {} — demande : {} — en pause : {}\n", p.number, p.request, p.reason.as_deref().unwrap_or("")));
        }
    }
    let new_request = news.iter().any(|l| l["kind"] == "request" || l["kind"] == "instruction");
    m.push('\n');
    match (active, new_request, last) {
        (Some(_), true, _) => m.push_str("Une demande nouvelle est arrivée pendant un plan actif : décide d'abord par edit_plan (continue, pause ou close), puis travaille, puis step_done.\n"),
        (Some(_), false, true) if !paused.is_empty() => m.push_str("Il ne reste rien de faisable ni de bloqué dans le plan actif : clos-le (edit_plan close), puis reprends un plan en pause (resume) ou clos-le aussi, puis step_done.\n"),
        (Some(_), false, true) => m.push_str("Il ne reste rien de faisable ni de bloqué : rends compte (report), clos le plan comme accompli (edit_plan close), puis appelle step_done.\n"),
        (Some(_), false, false) => m.push_str("Travaille un élément (le premier faisable seul, ou celui que la boîte débloque), puis step_done.\n"),
        (None, _, _) if !paused.is_empty() && !new_request => m.push_str("Aucun plan actif : reprends un plan en pause (resume) ou clos-le, puis travaille, puis step_done.\n"),
        (None, _, _) => m.push_str("Réponds ou agis. Un mini-plan (edit_plan) seulement si la demande demande une analyse ou se découpe en plusieurs éléments ; sinon fais, puis step_done.\n"),
    }
    m
}

/// Ce que la boucle rend de son travail.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Outcome {
    pub steps: Vec<Value>,
    pub ended: String,
}

impl DaemonHost {
    /// **Les fichiers de l'équipier d'une tâche** : son dossier.
    pub fn teammate_files(&self, parent: &str, task: &str) -> Result<TeammateFiles, String> {
        let t = self.tache(parent, task).ok_or_else(|| format!("pas de tâche « {task} » ouverte sur {parent}"))?;
        let dir = t.arbre.parent().map(Path::to_path_buf).ok_or("la tâche n'a pas de dossier")?;
        let mut files = TeammateFiles::new(dir, format!("{parent}/{task}"));
        // Chaque ligne écrite part aussi au flux de l'IDE (sujet `mailbox`), en
        // objet : l'IDE, le terminal et le hook la voient sans sonder la boîte.
        // Le `backend` de la trame est le manifeste de la tâche (son worktree), comme
        // pour ses autres événements : un client filtre le flux d'une tâche par lui seul.
        let (flux, backend, p, t) = (Arc::downgrade(self.ide_flux()), t.manifest.display().to_string(), parent.to_string(), task.to_string());
        files.relay = Some(MailboxRelay(Arc::new(move |line: &Value| {
            if let Some(flux) = flux.upgrade() {
                flux.publier("mailbox", &backend, json!({ "kind": "MailboxLine", "parent": p, "task": t, "line": line }));
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
        let mut lignes = Vec::new();
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
        if !matches!(kind, "request" | "instruction" | "answer" | "stop") {
            return Err(format!("genre inconnu : {kind} (request, instruction, answer, stop)"));
        }
        if text.trim().is_empty() && kind != "stop" {
            return Err("le texte est vide".into());
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
        // **Un oui à une commande en attente l'accorde** pour la suite de la
        // tâche (lot 2 de la page 01) : la ligne visée est une `approval` de
        // l'équipier, la commande passe par la porte du backend de la tâche,
        // dans son worktree. L'accord se pose **avant** que le oui s'écrive, et
        // le oui le porte (`granted`) : l'équipier, qui ne lit pas les lignes de
        // l'hôte, l'apprend dans la ligne qui le réveille ; la boîte le redit au
        // superviseur.
        let mut extra = extra.clone();
        if let Some(o) = extra.as_object_mut() {
            o.remove("granted"); // l'hôte seul l'écrit
        }
        #[allow(unused_mut)]
        let mut accord: Option<String> = None;
        #[cfg(feature = "code")]
        if kind == "answer" && extra["approved"] == true {
            let visee = extra["reply_to"].as_str().and_then(|r| files.mailbox().into_iter().find(|l| l["id"] == r));
            if let Some(ligne) = visee.as_ref().filter(|l| l["kind"] == "approval").and_then(|l| l["command"].as_str().map(str::to_string)) {
                let domaine = t.workspace.as_ref().map(|w| w.racine.clone());
                let accorde = self.with_backend_id(&t.id, |b| match b.garde() {
                    Some(g) => g.accorder_ligne(&ligne, &crate::commande::Contexte { accorde_par_l_utilisateur: true, domaine }),
                    None => Err("ce backend n'a pas de porte des commandes (workspace.commands)".into()),
                });
                let note = match accorde {
                    Ok(Ok(cs)) => format!("commande accordée pour la tâche : {}", cs.join(" ; ")),
                    Ok(Err(e)) | Err(e) => format!("l'accord n'a pas pu se poser : {e}"),
                };
                if let Some(o) = extra.as_object_mut() {
                    o.insert("granted".into(), Value::String(note.clone()));
                }
                accord = Some(note);
            }
        }
        let line = files.append_with(from, kind, item, text, &extra)?;
        // **D'où part la tâche** (c7, premier essai réel, 7 novembre) : ses
        // déclarations et son code sont ceux d'un commit ; une modification non
        // commitée n'y est pas, et sans le dire on la cherche longtemps.
        let depart = ouverte.then(|| {
            let court: String = t.depart.chars().take(9).collect();
            let note = format!(
                "tâche ouverte : branche {} depuis {court} ; ses déclarations et son code sont ceux de ce commit (une modification non commitée du manifeste ou du code n'y est pas)",
                t.branche
            );
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
        }
        let started = kind != "stop" && self.start_teammate(parent, task);
        let mut r = json!({ "line": line, "started": started });
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
                match h.run_teammate(&parent, &task) {
                    Ok(o) => eprintln!("[équipier] {parent}/{task} : {} pas, {}", o.steps.len(), o.ended),
                    Err(e) => eprintln!("[équipier] {parent}/{task} : {e}"),
                }
                h.teammates.lock().unwrap_or_else(|p| p.into_inner()).remove(&cle);
            }
        });
        if lance.is_err() {
            self.teammates.lock().unwrap_or_else(|p| p.into_inner()).remove(&key);
        }
        lance.is_ok()
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
    pub fn run_teammate(&self, parent: &str, task: &str) -> Result<Outcome, String> {
        let files = self.teammate_files(parent, task)?;
        // La fin, dans la boîte, erreur comprise : le superviseur la lit sans IDE.
        let outcome = match self.teammate_loop(parent, task, &files) {
            Ok(o) => o,
            Err(e) => Outcome { steps: Vec::new(), ended: format!("erreur : {e}") },
        };
        let line = files.append("host", "ended", None, &outcome.ended)?;
        if let Ok(bus) = self.tache(parent, task).ok_or(()).and_then(|t| self.with_backend_id(&t.id, |b| b.bus_de_publication()).map_err(|_| ())) {
            publish(Some(&bus), task, &line);
        }
        files.write_cursor(files.mailbox().len());
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
        let mut woke_by: Option<String> = None;
        let mut outcome = Outcome::default();
        let mut read = files.read_cursor();
        let mut stalled = 0usize;
        // Un plan a-t-il existé pendant ce lancement ? (la fin le dit autrement)
        let mut plan_seen = false;
        let request = files.mailbox().iter().find(|l| l["kind"] == "request").and_then(|l| l["text"].as_str()).unwrap_or_default().to_string();
        for number in 1..=MAX_STEPS {
            let lines = files.mailbox();
            let news: Vec<Value> = lines.iter().skip(read).filter(|l| l["from"] != "teammate" && l["from"] != "host").cloned().collect();
            // Le curseur d'avant ce pas : un pas qui n'aboutit pas rend ses nouvelles
            // au suivant (sans elles, l'équipier attendrait une boîte déjà lue).
            let avant = read;
            read = lines.len();
            files.write_cursor(read);
            if news.iter().any(|l| l["kind"] == "stop") {
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
            let work = !news.is_empty() || alone || (active.is_some() && !waiting) || (active.is_none() && !plans.paused().is_empty() && number == 1);
            let last = active.is_some() && !alone && !waiting && news.is_empty();
            if !work {
                // Rien à faire sans le superviseur : on attend la boîte, par le
                // bus quand il y en a un, et toujours par le fichier (toutes les
                // 2 s) : un backend fermé ne publie plus. Le journal du pas
                // suivant dit par où l'équipier s'est réveillé.
                let depuis = files.mailbox().len();
                let mut tour = 0u32;
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
            let message = frontier_message(number, &news, &plans, last);
            let mut step = Step::new(files.clone(), task, &request, bus.clone(), number);
            step.woke_by = woke_by.take();
            // **Un pas qui n'aboutit pas ne termine pas l'équipier** : l'échec va
            // dans la boîte et le pas suivant repart de la frontière ; seuls
            // `teammate.max_failed_steps` échecs d'affilée l'arrêtent.
            let r = match self.chat_turn_with(&t.id, &message, Some(&step)) {
                Ok(r) => {
                    failed = 0;
                    r
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
            // **Un pas ne finit jamais en silence** (c7, premier essai réel) : sans
            // `step_done`, la réponse finale du modèle va dans la boîte en `report`,
            // pour que le superviseur la voie (pending) au lieu d'un équipier qui
            // attend sans rien avoir dit.
            if declared.is_none() && r["stopped_by"].is_null() {
                let texte = r["text"].as_str().unwrap_or_default().trim();
                let dit = if texte.is_empty() {
                    format!("le pas {number} s'est terminé sans step_done ni réponse")
                } else {
                    format!("le pas {number} s'est terminé sans step_done ; sa réponse : {texte}")
                };
                if let Ok(line) = files.append("teammate", "report", None, &dit) {
                    publish(bus.as_ref(), task, &line);
                }
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
            if !r["stopped_by"].is_null() {
                outcome.ended = format!("arrêté par {}", r["stopped_by"]);
                return Ok(outcome);
            }
            // **La fin se juge après le pas**, sur les plans : plus aucun plan
            // vivant (ni actif ni en pause) et un pas déclaré fini. Un plan actif
            // qui n'a plus rien à faire n'est pas la fin : l'agent le clôt, puis
            // reprend un plan en pause ou le clôt aussi (Lucie).
            let after = files.read_plans();
            plan_seen |= !step.closed_plans().is_empty();
            if after.active().is_none() && after.paused().is_empty() && declared.is_some() {
                outcome.ended = if plan_seen { "plus rien à faire : compte rendu".into() } else { "fait sans plan".into() };
                return Ok(outcome);
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
        let appeler = |outil: &str, args: Value| -> Result<Value, String> { self.with_backend(&memoire, |b| b.call(outil, args)).and_then(|r| r) };
        let fiche = super::juge::nom_de_tache(if request.trim().is_empty() { &plan.request } else { request });
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

    fn call(name: &str, args: Value) -> ToolCall {
        ToolCall { id: "c".into(), name: name.into(), arguments: args.to_string(), provider_extra: None, at_ms: None }
    }

    /// Le plan s'écrit, ses questions partent dans la boîte, `step_done` passe
    /// l'élément à done ; le plan se clôt et un autre s'ouvre.
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
        let m = frontier_message(3, &[], &plans, false);
        assert!(m.contains("plans en pause") && m.contains("la doc") && m.contains("le superviseur demande autre chose"), "{m}");
        s.ours(&call(EDIT_PLAN, json!({ "close": { "outcome": "accomplished", "reason": "centré" } }))).unwrap();
        let m = frontier_message(4, &[], &s.files.read_plans(), false);
        assert!(m.contains("reprends un plan en pause"), "{m}");
        s.ours(&call(EDIT_PLAN, json!({ "resume": 1 }))).unwrap();
        let plans = s.files.read_plans();
        assert_eq!((plans.active().unwrap().number, plans.paused().len()), (1, 0));
    }

    /// Sans plan, la frontière dit d'agir ; une demande pendant un plan actif
    /// demande une décision.
    #[test]
    fn the_frontier_says_what_the_step_expects() {
        let m = frontier_message(1, &[json!({ "kind": "request", "from": "c7", "text": "Bonjour" })], &Plans::default(), false);
        assert!(m.contains("Bonjour") && m.contains("Réponds ou agis"), "{m}");
        let plans = Plans { plans: vec![Plan { number: 1, request: "r".into(), proposal: "p".into(), items: vec![], state: PlanState::Active, reason: None, steps: vec![], concepts: vec![] }] };
        let m = frontier_message(2, &[json!({ "kind": "request", "from": "c7", "text": "autre chose" })], &plans, false);
        assert!(m.contains("décide d'abord"), "{m}");
    }

    /// Au-delà du budget, tout appel autre que `step_done` est refusé.
    #[test]
    fn past_the_budget_only_step_done_passes() {
        let d = tempfile::tempdir().unwrap();
        let s = step(d.path());
        for _ in 0..STEP_BUDGET {
            assert!(s.gate(&call("read_file", json!({}))).is_none());
        }
        let refus = s.gate(&call("read_file", json!({}))).expect("refusé");
        assert!(refus.content.contains("step_done"), "{}", refus.content);
        let refus = s.ours(&call(EDIT_PLAN, json!({ "continue": true }))).unwrap();
        assert!(refus.content.contains("budget"), "{}", refus.content);
        assert!(s.ours(&call(STEP_DONE, json!({ "item": "", "succeeded": false, "done": "", "remaining": "tout" }))).unwrap().content.contains("\"ok\":true"));
    }
}
