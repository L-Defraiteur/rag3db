//! **The gate of the auto mode, v1** (page 07 of rag3weaver-archi, 21 Nov 2026): the hard guards
//! in Rust, the rules in `rules.json`, one judge reading the pair and the human turns only, and the
//! card when the judge does not authorize.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use super::teammate::TeammateFiles;
use super::{DaemonHost, GateJudge};

/// The classes that go to the judge, unless `rules.json` says otherwise.
pub const JUDGED_CLASSES: [&str; 4] = ["destroy_local", "outbound", "database", "system"];

/// What the gate decides for a command the guard held.
#[derive(Debug, Clone, PartialEq)]
pub enum GateOutcome {
    /// Granted (a safe class, or the judge): the command is granted for the task and replayed.
    Pass { by: String },
    /// The card: the user decides.
    Card { class: String },
}

/// The gate of a teammate's step: given the command the guard held, its outcome.
pub type StepGate = Arc<dyn Fn(&str) -> GateOutcome + Send + Sync>;

/// What the gate decides for a path outside the sandbox's roots (`judge_all`).
#[derive(Debug, Clone, PartialEq)]
pub enum OpenRootOutcome {
    /// The root is granted to the task's agent (the judge's yes): the command is replayed.
    Granted,
    /// The card, with what it shows (`command`, `root`, `mode`, and a `warning` for a broad root).
    Card { class: String, args: Value },
}

/// The `judge_all` gate of a teammate's step: given the sandbox's `open_root` request, its outcome.
pub type StepOpenRoot = Arc<dyn Fn(&Value) -> OpenRootOutcome + Send + Sync>;

/// The rules of the gate (`rules.json` in the gate's folder): the classes judged, the judge.
#[derive(Debug, Clone, Default)]
pub struct GateRules {
    pub judged: Vec<String>,
    /// `{address, prompt_file, threshold}` of the served judge (banc's Qwen), if any.
    pub judge: Option<Value>,
    /// The survivor's option (Lucie, 21 Nov 20 h 35): it would judge a broad root. Not in v1: the card decides, and says so.
    pub dangerous_standalone_survivor: bool,
    /// **The judge in the auto mode** (Lucie, 22 Nov 2026, 23 h 25: « au moins l'agent de code va au bout de son
    /// truc »): off by default, every command the sandbox runs passes in the auto mode, the sandbox being the night's
    /// only safety (bounded writes, no sudo, the protected folders) with the hard refusals (a secret shown, a git
    /// identity). On, the classes and the judge decide, as in the v1.
    pub judge_in_auto: bool,
}

impl GateRules {
    /// Read from `dir/rules.json`; absent, the defaults (the four classes judged, no served judge).
    pub fn read(dir: Option<&Path>) -> GateRules {
        let v: Value = dir
            .map(|d| d.join("rules.json"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        let judged = v["judged_classes"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_else(|| JUDGED_CLASSES.iter().map(|c| c.to_string()).collect());
        GateRules {
            judged,
            judge: v.get("judge").filter(|j| j.is_object()).cloned(),
            dangerous_standalone_survivor: v["dangerous_standalone_survivor"].as_bool().unwrap_or(false),
            judge_in_auto: v["judge_in_auto"].as_bool().unwrap_or(false),
        }
    }
}

/// **The class of a command line, by rules** (page 07 of rag3weaver-archi): the highest class of its
/// parts. `read` and `write_local` pass; `destroy_local` only for what is neither on a remote nor made
/// by the agent in the task (an untracked file of the task's tree is the agent's); `outbound`,
/// `database`, `system` by their families.
pub fn rule_class(ligne: &str, racine: &Path) -> String {
    let rang = |c: &str| match c {
        "read" => 0,
        "write_local" => 1,
        "destroy_local" => 2,
        "database" => 3,
        "outbound" => 4,
        _ => 5,
    };
    let parties: Vec<crate::commande::Commande> = match codeparsers::shell::decomposer(ligne) {
        Ok(v) => v.into_iter().map(Into::into).collect(),
        Err(_) => return "system".into(),
    };
    let mut haute = "read".to_string();
    for c in &parties {
        let cl = class_of(c, racine);
        if rang(&cl) > rang(&haute) {
            haute = cl;
        }
    }
    haute
}

fn class_of(c: &crate::commande::Commande, racine: &Path) -> String {
    let prog = Path::new(&c.programme).file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let a: Vec<&str> = c.args.iter().map(String::as_str).collect();
    let verbe = a.iter().find(|x| !x.starts_with('-')).copied().unwrap_or_default();
    let has = |x: &str| a.iter().any(|y| *y == x || y.starts_with(&format!("{x}=")));
    match prog.as_str() {
        "systemctl" | "crontab" | "service" | "mount" | "umount" | "useradd" | "usermod" | "apt" | "apt-get" | "pacman" | "dnf" | "yum" | "brew" | "snap" | "flatpak" | "shutdown" | "reboot" => "system".into(),
        "psql" | "mysql" | "mariadb" | "sqlite3" | "mongo" | "mongosh" | "redis-cli" => "database".into(),
        "ssh" | "scp" | "sftp" => "outbound".into(),
        "gh" => if matches!(verbe, "pr" | "repo" | "release" | "issue" | "api" | "gist") && !a.iter().any(|x| matches!(*x, "list" | "view" | "status" | "diff" | "checks")) { "outbound".into() } else { "read".into() },
        "curl" | "wget" => if has("-X") || has("--request") || has("-d") || has("--data") || has("-T") || has("--upload-file") || has("-F") || has("--form") { "outbound".into() } else { "read".into() },
        "npm" | "pnpm" | "yarn" | "cargo" | "twine" | "gem" if verbe == "publish" => "outbound".into(),
        // **An install of packages is judged** (Lucie, 21 Nov 20 h): it passes only if the user asked for it in so
        // many words; « prépare-moi un projet React » alone is not an install's consent. The network and many files
        // written: the outbound class, which the judge weighs with P3's explicit-install rule.
        "npm" | "pnpm" | "yarn" | "bun" if matches!(verbe, "install" | "i" | "add" | "ci" | "update" | "upgrade") => "outbound".into(),
        "npx" | "pnpx" | "bunx" => "outbound".into(),
        "pip" | "pip3" | "pipx" | "uv" | "poetry" | "conda" if matches!(verbe, "install" | "add" | "sync" | "update" | "upgrade") => "outbound".into(),
        "cargo" if matches!(verbe, "install" | "add" | "update") => "outbound".into(),
        "gem" | "go" if verbe == "install" || verbe == "get" => "outbound".into(),
        "git" => match verbe {
            "push" => "outbound".into(),
            "reset" if has("--hard") => "destroy_local".into(),
            "clean" => "destroy_local".into(),
            "branch" if has("-D") || has("--delete") && has("--force") => "destroy_local".into(),
            "checkout" | "restore" if a.iter().any(|x| *x == "--" || *x == ".") => "destroy_local".into(),
            "status" | "log" | "diff" | "show" | "branch" | "remote" | "blame" | "rev-parse" | "ls-files" | "describe" => "read".into(),
            _ => "write_local".into(),
        },
        "rm" | "rmdir" | "shred" | "truncate" | "unlink" => {
            let cibles: Vec<&str> = a.iter().filter(|x| !x.starts_with('-')).copied().collect();
            if !cibles.is_empty() && cibles.iter().all(|f| deletable_without_asking(racine, f)) { "write_local".into() } else { "destroy_local".into() }
        }
        "find" if has("-delete") => "destroy_local".into(),
        _ => {
            let f = crate::commande::observer(c);
            if f.reseau && f.ecrit { "outbound".into() } else if f.ecrit { "write_local".into() } else { "read".into() }
        }
    }
}

/// A file the agent may delete without asking: made by it in the task (untracked in the task's tree), or
/// whose current content is on a remote (tracked, unchanged, and its commit reachable from `refs/remotes/*`).
fn deletable_without_asking(racine: &Path, fichier: &str) -> bool {
    let git = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(racine).args(args).output().ok();
    let suivi = git(&["ls-files", "--error-unmatch", "--", fichier]).is_some_and(|o| o.status.success());
    if !suivi {
        // Untracked: the agent made it in the task (a task's tree starts at a commit).
        // v1's approximation, said: an ignored file counts as the agent's too (a build output, node_modules).
        return racine.join(fichier).exists();
    }
    let propre = git(&["diff", "--quiet", "HEAD", "--", fichier]).is_some_and(|o| o.status.success());
    let distant = git(&["branch", "-r", "--contains", "HEAD"]).is_some_and(|o| o.status.success() && !o.stdout.is_empty());
    propre && distant
}

/// **A root broader than a project** (Lucie, 21 Nov 20 h 35): the home folder or one of its parents, or a folder that
/// holds two repositories or more (one or two levels down). Its warning says what it holds; it goes to the card, never
/// to the judge alone.
pub fn broad_root(root: &Path) -> Option<String> {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from).and_then(|h| h.canonicalize().ok()) {
        if root == home || home.starts_with(root) {
            return Some(format!("cette racine contient tout ton dossier personnel ({})", home.display()));
        }
    }
    let mut depots = Vec::new();
    for e in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let p = e.path();
        if !p.is_dir() || p.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if p.join(".git").exists() {
            depots.push(p.strip_prefix(root).unwrap_or(&p).display().to_string());
            continue;
        }
        for e2 in std::fs::read_dir(&p).into_iter().flatten().flatten() {
            if e2.path().is_dir() && e2.path().join(".git").exists() {
                depots.push(e2.path().strip_prefix(root).unwrap_or(&e2.path()).display().to_string());
            }
        }
    }
    depots.sort();
    (depots.len() >= 2).then(|| {
        let plus = if depots.len() > 8 { format!(" et {} autres", depots.len() - 8) } else { String::new() };
        format!("cette racine contient aussi : {}{plus}", depots.iter().take(8).cloned().collect::<Vec<_>>().join(", "))
    })
}

/// The last turn of the human (its text, its time, its line), for a grant's proof.
fn last_human_turn(files: &TeammateFiles) -> Option<Value> {
    files.mailbox().into_iter().rev().find(|l| l["by"]["kind"] == "human" && l["kind"] != "answer")
}

/// The last turn of the agent: the reason it gave, shown on a protected folder's card.
pub(crate) fn last_agent_turn(files: &TeammateFiles) -> String {
    // Its own words (a report, a question), never a card it opened before (recherche's audit).
    files
        .mailbox()
        .into_iter()
        .rev()
        .find(|l| l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("report" | "question")))
        .and_then(|l| l["text"].as_str().map(str::to_string))
        .unwrap_or_default()
}

/// A package manager's install (banc's `INSTALL_CMD`, word for word).
pub fn install_command(ligne: &str) -> bool {
    static INSTALL_CMD: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    INSTALL_CMD
        .get_or_init(|| regex::Regex::new(r"\b(?:(?:npm|pnpm|yarn)\s+(?:install|i|add|update|up)\b|pip3?\s+install\b|cargo\s+(?:add|install)\b|(?:apt|apt-get|dnf|brew)\s+install\b|pacman\s+-S)").expect("INSTALL_CMD"))
        .is_match(ligne)
}

/// Whether the user asked to install (banc's `INSTALL_WORD` and `SHORT_YES`, word for word): the last human
/// turn holds an install word, or it is a short yes to the agent's turn just before, which holds one.
pub fn install_asked(input: &Value) -> bool {
    static INSTALL_WORD: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static SHORT_YES: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let word = INSTALL_WORD.get_or_init(|| regex::Regex::new(r"(?i)\b(?:install\w*|ajout\w*\s.*\b(?:dépendance|paquet|package|crate)s?|add\b|mets?\s+à\s+jour|mettre\s+à\s+jour|update)").expect("INSTALL_WORD"));
    let yes = SHORT_YES.get_or_init(|| regex::Regex::new(r"(?i)^\s*(?:oui|yes|ok|vas[- ]y|d'accord|go)\b").expect("SHORT_YES"));
    let turns: Vec<&Value> = input["turns"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let Some(i) = turns.iter().rposition(|t| t["by"]["kind"] == "human") else { return false };
    let last = turns[i]["text"].as_str().unwrap_or_default();
    if word.is_match(last) {
        return true;
    }
    yes.is_match(last) && i > 0 && turns[i - 1]["by"]["kind"] == "agent" && word.is_match(turns[i - 1]["text"].as_str().unwrap_or_default())
}

/// **The judge's input** (banc's page 09): the last turns of the user and of the agent, each with the
/// `by` the host signed, oldest first; never a tool output. `user_turns`: the human ones only.
pub fn judge_input(files: &TeammateFiles, line: &str, class: &str) -> Value {
    let mut turns = Vec::new();
    for l in files.mailbox() {
        let human = l["by"]["kind"] == "human" && matches!(l["kind"].as_str(), Some("request" | "answer" | "message" | "instruction"));
        let agent = l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("question" | "report" | "approval"));
        if human || agent {
            turns.push(json!({ "at": l["at"], "by": { "kind": if human { "human" } else { "agent" } }, "text": l["text"] }));
        }
    }
    let start = turns.len().saturating_sub(6);
    let turns: Vec<Value> = turns[debut..].to_vec();
    let user_turns: Vec<Value> = turns.iter().filter(|t| t["by"]["kind"] == "human").cloned().collect();
    json!({ "action": { "tool": "run_command", "command": line }, "class": class, "turns": turns, "user_turns": user_turns })
}

/// **The served judge** (banc's page 09): `POST <address>/v1/chat/completions`, P3 as the system
/// message, one token, its logprobs; p(Y) = Y / (Y + N). Below the threshold, a mass under 0.5, a
/// mute server or a timeout: not authorized, which is the card, never a grant.
pub fn served_judge(conf: &Value) -> Option<GateJudge> {
    served_judge_with(conf, crate::usage_ledger::user_ledger())
}

/// [`served_judge`] writing to `ledger` (the witnesses' own).
pub fn served_judge_with(conf: &Value, ledger: Arc<crate::usage_ledger::Ledger>) -> Option<GateJudge> {
    let adresse = conf["address"].as_str()?.to_string();
    let modele = conf["model"].as_str().unwrap_or("gate-judge").to_string();
    let prompt = std::fs::read_to_string(conf["prompt_file"].as_str()?).ok()?;
    let seuil = conf["threshold"].as_f64().unwrap_or(0.5);
    Some(Arc::new(move |input: &Value| -> Value {
        let mut texte = String::from("## Conversation (oldest first, the newest last)\n");
        for t in input["turns"].as_array().into_iter().flatten() {
            let qui = if t["by"]["kind"] == "human" { "user" } else { "agent" };
            texte.push_str(&format!("[{}] « {} »\n", qui, t["text"].as_str().unwrap_or_default().replace('\n', " ")));
        }
        texte.push_str(&format!("\n## Command to judge\ntool: Bash\nargs: {}\n", input["action"]["command"].as_str().unwrap_or_default()));
        let corps = json!({
            "messages": [{ "role": "system", "content": prompt }, { "role": "user", "content": texte }],
            "temperature": 0, "max_tokens": 1, "logprobs": true, "top_logprobs": 20,
            "chat_template_kwargs": { "enable_thinking": false }
        });
        // **Reserve before, settle after** (ED's ledger): the bound is the prompt's characters and the one token asked;
        // a refusal is said and is the card, never a call around the ledger.
        use crate::usage_ledger::{is_private_address, Caller, Tokens};
        let fournisseur = format!("http://{adresse}");
        let borne = Tokens { input: (prompt.len() + texte.len()) as u64, output: 1, ..Default::default() };
        let appelant = Caller { agent: "gate".into(), ..Default::default() };
        let reservation = match ledger.reserve(&modele, &fournisseur, is_private_address(&fournisseur), borne, &appelant, false) {
            Ok(r) => r,
            Err(e) => return json!({ "label": "not_authorized", "why": format!("the spending ledger refused the judge's call: {e}") }),
        };
        let agent = ureq::Agent::new_with_config(ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(20))).build());
        let reponse = agent
            .post(format!("http://{adresse}/v1/chat/completions"))
            .header("content-type", "application/json")
            .send(corps.to_string())
            .ok()
            .and_then(|r| r.into_body().read_to_string().ok())
            .and_then(|t| serde_json::from_str::<Value>(&t).ok());
        let Some(v) = reponse else {
            // Nothing billed before an answer: settled at zero.
            let _ = ledger.settle(&reservation, Some(Tokens::default()));
            return json!({ "label": "not_authorized", "why": "the judge does not answer" });
        };
        let utilise = match (v["usage"]["prompt_tokens"].as_u64(), v["usage"]["completion_tokens"].as_u64()) {
            (Some(i), Some(o)) => Some(Tokens { input: i, output: o, ..Default::default() }),
            _ => None,
        };
        let _ = ledger.settle(&reservation, utilise);
        let (mut y, mut n) = (0.0f64, 0.0f64);
        for t in v["choices"][0]["logprobs"]["content"][0]["top_logprobs"].as_array().into_iter().flatten() {
            let p = t["logprob"].as_f64().map(f64::exp).unwrap_or(0.0);
            match t["token"].as_str().unwrap_or_default().trim() {
                "Y" => y += p,
                "N" => n += p,
                _ => {}
            }
        }
        if y + n < 0.5 {
            return json!({ "label": "not_authorized", "why": format!("the judge did not answer Y or N (mass {:.2})", y + n) });
        }
        let py = y / (y + n);
        json!({ "label": if py >= seuil { "explicit" } else { "not_authorized" }, "p_yes": py })
    }))
}

impl DaemonHost {
    /// The folder of the gate's configuration (`rules.json`, the judge's prompt).
    pub fn with_gate_dir(mut self, dir: PathBuf) -> Self {
        self.gate_dir = Some(dir);
        self
    }

    /// The gate's judge, given here instead of the model the configuration names.
    pub fn with_gate_judge(mut self, judge: GateJudge) -> Self {
        self.gate_judge = Some(judge);
        self
    }

    /// **The gate of a teammate's step** in the auto mode: a safe class is granted and replayed; a judged
    /// class asks the judge, whose yes is a grant and whose anything else is the card. `None` out of the
    /// auto mode (the cards of before stay).
    /// **The gate's level of a task** (Lucie, 22 Nov 2026, 23 h 20): `bac`, `judge` or `cards`. The level the launch
    /// chose (the mailbox's last `gate_level` line) beats the project's declaration; without one, a project in the
    /// auto mode is at `bac`, or `judge` when `rules.json` sets `judge_in_auto`; any other project has no level (the
    /// cards of before).
    pub(crate) fn gate_level(&self, backend_id: &str, files: &TeammateFiles) -> Option<String> {
        if let Some(l) = files.mailbox().iter().rev().find(|l| l["from"] == "host" && l["kind"] == "gate_level").and_then(|l| l["level"].as_str().map(str::to_string)) {
            return Some(l);
        }
        let mode = self.with_backend_id(backend_id, |b| b.garde().map(|g| g.mode())).ok().flatten()?;
        if mode != crate::commande::Mode::Auto {
            return None;
        }
        let rules = GateRules::read(self.gate_dir.as_deref().or(Some(default_gate_dir().as_path())));
        Some(if rules.judge_in_auto { "judge" } else { "bac" }.to_string())
    }

    pub(crate) fn step_gate(&self, backend_id: &str, files: &TeammateFiles, racine: PathBuf) -> Option<StepGate> {
        let garde = self.with_backend_id(backend_id, |b| b.garde().cloned()).ok().flatten()?;
        let level = self.gate_level(backend_id, files)?;
        let rules = GateRules::read(self.gate_dir.as_deref().or(Some(default_gate_dir().as_path())));
        let juge = self.gate_judge.clone().or_else(|| rules.judge.as_ref().and_then(served_judge));
        let files = files.clone();
        Some(Arc::new(move |ligne: &str| -> GateOutcome {
            let class = if level == "bac" { "auto".to_string() } else { rule_class(ligne, &racine) };
            let accorder = |by: &str| -> GateOutcome {
                let ctx = crate::commande::Contexte { accorde_par_l_utilisateur: true, domaine: Some(racine.clone()) };
                match garde.accorder_ligne(ligne, &ctx) {
                    Ok(_) => GateOutcome::Pass { by: by.to_string() },
                    Err(_) => GateOutcome::Card { class: class.clone() },
                }
            };
            // `bac` trusts the sandbox; `cards` asks for every command the guard holds (a write by the code tools never
            // reaches here: it is no command).
            match level.as_str() {
                "bac" => return accorder("auto"),
                "cards" => return GateOutcome::Card { class },
                _ => {}
            }
            if !rules.judged.iter().any(|c| *c == class) {
                return accorder("rules");
            }
            let entree = judge_input(&files, ligne, &class);
            // **The install rule, in code, before the judge** (Lucie, 21 Nov 20 h; banc's page 09): a package
            // manager passes to the judge only if the user's last message asks to install, or a short yes answers
            // an agent's turn that does; otherwise, the card.
            if install_command(ligne) && !install_asked(&entree) {
                return GateOutcome::Card { class };
            }
            let Some(juge) = &juge else { return GateOutcome::Card { class } };
            let verdict = juge(&entree);
            if !matches!(verdict["label"].as_str(), Some("explicit" | "covered")) {
                return GateOutcome::Card { class };
            }
            let issue = accorder("judge");
            if matches!(issue, GateOutcome::Pass { .. }) {
                // **Said in the mailbox** (c7, 22 Nov 2026: a pass by the judge left no trace): the command, its class,
                // and the user's words it rests on.
                let citation = verdict["quote"].as_str().map(str::to_string).or_else(|| last_human_turn(&files).and_then(|t| t["text"].as_str().map(str::to_string))).unwrap_or_default();
                let text = format!("commande accordée par le juge : {ligne} ({class}), sur ta phrase « {citation} »");
                let _ = files.append_with("host", "command_granted", None, &text, &json!({ "command": ligne, "class": class, "by": "judge", "quote": citation }));
            }
            issue
        }))
    }

    /// **The `judge_all` gate of a teammate's step** (Lucie, 21 Nov 2026, 20 h): the sandbox stopped a path outside its
    /// roots. A protected folder is its own card, never judged (a human yes opens it for this command only). A root
    /// broader than a project is a card with its warning. Otherwise, in the auto mode, the judge: its yes grants the root
    /// (the project's, as `proposed_root` suggests) to the task's agent, and the command is replayed; anything else is
    /// the card, whose yes grants it.
    pub(crate) fn step_open_root(&self, backend_id: &str, files: &TeammateFiles) -> Option<StepOpenRoot> {
        let hote = self.soi.get().cloned()?;
        let rules = GateRules::read(self.gate_dir.as_deref().or(Some(default_gate_dir().as_path())));
        let juge = self.gate_judge.clone().or_else(|| rules.judge.as_ref().and_then(served_judge));
        let backend_id = backend_id.to_string();
        let files = files.clone();
        Some(Arc::new(move |m: &Value| -> OpenRootOutcome {
            let replay = m["replay"].as_str().unwrap_or_default().to_string();
            let mode = m["mode"].as_str().filter(|x| matches!(*x, "read" | "write")).unwrap_or("read").to_string();
            let refused = PathBuf::from(m["refused"].as_str().unwrap_or_default());
            if m["protected"] == true {
                let args = json!({ "command": replay, "root": refused, "mode": "read", "reason": last_agent_turn(&files) });
                return OpenRootOutcome::Card { class: "protected".into(), args };
            }
            let Some(h) = hote.upgrade() else {
                return OpenRootOutcome::Card { class: "open_root".into(), args: json!({ "command": replay, "root": refused, "mode": mode }) };
            };
            let dossier = if refused.is_dir() { refused.clone() } else { refused.parent().map(Path::to_path_buf).unwrap_or_else(|| refused.clone()) };
            let root = h.proposed_root(&refused).unwrap_or(dossier);
            let mut args = json!({ "command": replay, "root": root, "mode": mode });
            if let Some(w) = broad_root(&root) {
                args["warning"] = json!(w);
                if rules.dangerous_standalone_survivor {
                    args["note"] = json!("dangerous_standalone_survivor : pas encore en v1, la carte décide");
                }
                return OpenRootOutcome::Card { class: "open_root".into(), args };
            }
            // **A path she wrote herself is read without a card** (Lucie, 23 Nov 2026, ~21 h): for reading only, from a
            // human's request of the turn, for `teammate.mentioned_path_grant_min` minutes; said in one line.
            if mode == "read" {
                let minutes = h.with_backend_id(&backend_id, |b| b.prepared().manifest.teammate.as_ref().map(|p| p.mentioned_path_grant_min)).ok().flatten().unwrap_or(10);
                if let (true, Some(citation), Some(agent)) = (minutes > 0, cited_by_a_human(&files.mailbox(), &refused), h.agent_key_of_backend(&backend_id)) {
                    let now = crate::catalog::retard::now_ms();
                    let proof = super::root_grants::Proof { quote: citation, at: Some(now), turn: None };
                    if h.grant_root_until(&agent, &root, proof, "mention", "read", Some(now + minutes * 60_000)).is_ok() {
                        let text = h.prompt_texts().get("grant.mentioned", &[("root", &root.display().to_string()), ("minutes", &minutes.to_string())]);
                        let _ = files.append_with("host", "root_granted", None, &text, &json!({ "root": root, "mode": "read", "by": "mention", "minutes": minutes }));
                        return OpenRootOutcome::Granted;
                    }
                }
            }
            let carte = |args: Value| OpenRootOutcome::Card { class: "open_root".into(), args };
            let auto = matches!(h.gate_level(&backend_id, &files).as_deref(), Some("bac" | "judge"));
            let (Some(agent), Some(juge), true) = (h.agent_key_of_backend(&backend_id), &juge, auto) else { return carte(args) };
            let mut entree = judge_input(&files, &replay, "open_root");
            entree["action"]["open_root"] = json!({ "path": root, "mode": mode });
            let verdict = juge(&entree);
            if !matches!(verdict["label"].as_str(), Some("explicit" | "covered")) {
                return carte(args);
            }
            let tour = last_human_turn(&files).unwrap_or(Value::Null);
            let proof = super::root_grants::Proof {
                quote: verdict["quote"].as_str().or(tour["text"].as_str()).unwrap_or_default().to_string(),
                at: tour["at"].as_u64(),
                turn: tour["id"].as_str().map(str::to_string),
            };
            let citation = proof.quote.clone();
            match h.grant_root(&agent, &root, proof, "judge", &mode) {
                Ok(_) => {
                    // Said in the mailbox: unlike a line's grant, a root lasts, and the user sees what it rests on.
                    let text = format!("racine accordée par le juge : {} ({mode}), sur ta phrase « {citation} »", root.display());
                    let _ = files.append_with("host", "root_granted", None, &text, &json!({ "root": root, "mode": mode, "by": "judge", "quote": citation }));
                    OpenRootOutcome::Granted
                }
                Err(_) => carte(args),
            }
        }))
    }

    /// The gate's folder and whether a judge is given (said by `/sante`).
    pub fn gate_state(&self) -> (Option<&PathBuf>, bool) {
        (self.gate_dir.as_ref(), self.gate_judge.is_some())
    }
}

/// **The human line of the current turn that wrote `refused` or a folder holding it** (Lucie, 23 Nov 2026, ~21 h): its
/// text, else `None`. Only a request or an instruction whose `by.kind` is human, since the turn's last `ended`; never a
/// path from an agent, a file read or a tool's result. A word is a path when it starts with `/` or `~/`, its quotes and
/// its end punctuation taken off.
pub(crate) fn cited_by_a_human(lines: &[Value], refused: &Path) -> Option<String> {
    let start = lines.iter().rposition(|l| l["kind"] == "ended").map_or(0, |n| n + 1);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let refused_path = refused.canonicalize().unwrap_or_else(|_| refused.to_path_buf());
    lines[start..]
        .iter()
        .filter(|l| matches!(l["kind"].as_str(), Some("request" | "instruction")) && l["from"] != "teammate" && l["from"] != "host" && l["by"]["kind"] == "human")
        .filter_map(|l| l["text"].as_str())
        .find(|text| {
            text.split_whitespace().any(|word| {
                let word = word.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | '«' | '»' | '(' | ')' | '<' | '>' | ',' | ';')).trim_end_matches(|c: char| matches!(c, '.' | ':' | '!' | '?'));
                let path = match (word.strip_prefix("~/"), &home) {
                    (Some(rest), Some(m)) => m.join(rest),
                    _ if word.starts_with('/') => PathBuf::from(word),
                    _ => return false,
                };
                let path = path.canonicalize().unwrap_or(path);
                path.components().count() > 2 && refused_path.starts_with(&path)
            })
        })
        .map(str::to_string)
}

/// The user's gate folder: `~/.config/rag3weaver/gate`.
pub fn default_gate_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config/rag3weaver/gate")
}

/// Witnesses: the served judge goes through the spending ledger (c7 and ED, 22 Nov 2026).
#[cfg(test)]
#[path = "gate_ledger_tests.rs"]
mod ledger_tests;
