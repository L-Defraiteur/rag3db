//! **La boîte de l'équipier, côté terminal** (lot tty-4) : ce qui attend une
//! réponse du superviseur, et la réponse qu'on renvoie. Sans ratatui, pour que
//! la vue et le témoin e2e (par le chemin de l'hôte) jouent la même logique.
//!
//! Les formes viennent de la page de l'équipier
//! (`extension/rag3weaver/docs/07-novembre-2026-everything-declarative/01-l-equipier-agent-de-code.md`,
//! § 2) : chaque ligne a un `id` stable (`"<parent>/<task>:<n>"`) ; une
//! `question`, une `approval` (avec sa `command`) ou une `proposal` de
//! l'équipier attend tant qu'aucune ligne ne porte `reply_to: <son id>` ; la
//! réponse est un `answer` avec `reply_to`, et `approved` pour une approbation.

use serde_json::{json, Value};

/// Une ligne de l'équipier qui attend la réponse du superviseur.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attente {
    pub id: String,
    /// `question`, `approval` ou `proposal`.
    pub kind: String,
    pub text: String,
    /// La commande d'une `approval`.
    pub command: Option<String>,
    /// **The card of the mode auto** (archi's page 07, 21 November 2026), boxed: an `approval` that names its tool.
    pub card: Option<Box<Card>>,
}

/// What an approval card of the mode auto carries: the tool asked for, its arguments, its class and its scope
/// (`once`), and the state the host last said (`approval_state`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Card {
    pub tool: String,
    pub args: Value,
    pub class: Option<String>,
    pub scope: Option<String>,
    pub state: Option<CardState>,
}

/// The state of a card, as the host's last `approval_state` line says it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardState {
    /// `recall`, `appeal`, `waiting_survivor` or `survivor`.
    pub stage: String,
    pub text: String,
    /// The end of the wait, in ms since the epoch (the survivor's countdown).
    pub until: Option<u64>,
}

impl Attente {
    /// **What the card asks for**: the command, or the tool and its arguments in the order of their names
    /// (`browser_click ref=e4 tab_id=7`).
    pub fn what(&self) -> String {
        match (&self.command, &self.card) {
            (Some(c), _) => c.clone(),
            (None, Some(card)) => {
                let t = &card.tool;
                let args: Vec<String> = card.args.as_object().into_iter().flatten().map(|(k, v)| {
                    let v = v.as_str().map_or_else(|| v.to_string(), str::to_string);
                    let v = if v.chars().count() > 40 { format!("{}…", v.chars().take(40).collect::<String>()) } else { v };
                    format!("{k}={v}")
                }).collect();
                if args.is_empty() { t.clone() } else { format!("{t} {}", args.join(" ")) }
            }
            (None, None) => self.text.lines().next().unwrap_or_default().to_string(),
        }
    }

    /// What the open card asks for, with « pour cet envoi seulement » when its scope is `once`.
    pub fn asked(&self) -> String {
        let once = self.card.as_ref().is_some_and(|c| c.scope.as_deref() == Some("once"));
        format!("{}{}", self.what(), if once { " — pour cet envoi seulement" } else { "" })
    }

    /// The card's state as said beside it, with the countdown of its wait at `now_ms`.
    pub fn state_said(&self, now_ms: u64) -> Option<String> {
        let st = self.card.as_ref()?.state.as_ref()?;
        Some(match st.until {
            Some(u) => format!("{} ({})", st.text, countdown(u, now_ms)),
            None => st.text.clone(),
        })
    }
}

/// **The closing line of a card**: who decided, and on what — the user, an authorization recalled (its quote), the
/// agent's appeal, or the verifier (its model, its reasons, its alternative). `None` when `answer` does not close an
/// approval (no `approved`).
pub fn closing_line(a: &Attente, answer: &Value) -> Option<String> {
    let approved = answer["approved"].as_bool()?;
    let what = a.what();
    let verb = if approved { "accordé" } else { "refusé" };
    Some(match answer["from"].as_str().unwrap_or("user") {
        "recall" => {
            let q = &answer["authorization"];
            let at = q["at"].as_u64().map(|ms| format!(", {}", local_date(ms))).unwrap_or_default();
            format!("{} par ton autorisation{at} : « {} » — {what}", if approved { "validé" } else { "refusé" }, q["quote"].as_str().unwrap_or_default())
        }
        "appeal" => format!("{verb} sur l'appel de l'agent : {what}"),
        "verifier" => {
            let model = answer["model"].as_str().map(|m| format!(" ({m})")).unwrap_or_default();
            let reasons = answer["reasons"].as_str().map(|r| format!(" : {r}")).unwrap_or_default();
            let alt = answer["alternative"]["text"].as_str().map(|t| format!(" — il propose : {t}")).unwrap_or_default();
            format!("{verb} par le vérificateur{model}{reasons} — {what}{alt}")
        }
        _ => format!("{verb} par toi : {what}"),
    })
}

/// Ce que le terminal sait de la boîte.
#[derive(Debug, Default)]
pub struct Boite {
    attentes: Vec<Attente>,
}

impl Boite {
    /// Une ligne de la boîte, dans l'ordre où elle arrive.
    pub fn ligne(&mut self, l: &Value) {
        let kind = l["kind"].as_str().unwrap_or_default();
        // **A state line changes the card, never closes it** (Lucie, 21 November 2026: one card, never hidden).
        if kind == "approval_state" {
            if let Some(a) = l["reply_to"].as_str().and_then(|r| self.attentes.iter_mut().find(|a| a.id == r)) {
                a.card.get_or_insert_with(Default::default).state = Some(CardState { stage: l["stage"].as_str().unwrap_or_default().to_string(), text: l["text"].as_str().unwrap_or_default().to_string(), until: l["until"].as_u64() });
            }
            return;
        }
        // Toute autre ligne qui y répond retire ce qu'elle vise, d'où qu'elle vienne.
        if let Some(r) = l["reply_to"].as_str() {
            self.attentes.retain(|a| a.id != r);
        }
        if l["from"] == "teammate" && matches!(kind, "question" | "approval" | "proposal") {
            if let Some(id) = l["id"].as_str() {
                if !self.attentes.iter().any(|a| a.id == id) {
                    self.attentes.push(Attente {
                        id: id.to_string(),
                        kind: kind.to_string(),
                        text: l["text"].as_str().unwrap_or_default().to_string(),
                        command: l["command"].as_str().map(str::to_string),
                        card: l["tool"].as_str().map(|t| {
                            Box::new(Card {
                                tool: t.to_string(),
                                args: l.get("args").cloned().unwrap_or(Value::Null),
                                class: l["class"].as_str().map(str::to_string),
                                scope: l["scope"].as_str().map(str::to_string),
                                state: None,
                            })
                        }),
                    });
                }
            }
        }
    }

    /// Ce qui attend, du plus ancien au plus récent.
    pub fn attentes(&self) -> &[Attente] {
        &self.attentes
    }

    /// Ce à quoi une réponse répond : le plus ancien en attente.
    pub fn premiere(&self) -> Option<&Attente> {
        self.attentes.first()
    }
}

/// **Le corps de la réponse** (`POST /ide/teammate/mailbox`) à `a`, de la part
/// du superviseur ; `accord` pour une approbation (`approved`).
pub fn reponse(backend: &str, task: &str, a: &Attente, texte: &str, accord: Option<bool>) -> Value {
    let mut v = json!({ "backend": backend, "task": task, "kind": "answer", "text": texte, "reply_to": a.id });
    if let Some(oui) = accord {
        v["approved"] = json!(oui);
    }
    v
}

/// **The time left before `until`** (ms since the epoch), as the card says it: « 9 min », « 45 s », « maintenant ».
pub fn countdown(until_ms: u64, now_ms: u64) -> String {
    match until_ms.saturating_sub(now_ms) / 1000 {
        0 => "maintenant".into(),
        s if s < 60 => format!("{s} s"),
        s => format!("{} min", s.div_ceil(60)),
    }
}

/// A date in local words: « le 21 nov. à 16 h 05 » (`month0` from 0).
pub fn said_date(day: u32, month0: u32, hour: u32, minute: u32) -> String {
    const MONTHS: [&str; 12] = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
    format!("le {day} {} à {hour} h {minute:02}", MONTHS[(month0 % 12) as usize])
}

/// **An instant in ms since the epoch, in local words** (« le 21 nov. à 16 h 05 »): local time by `localtime_r`;
/// where it is not at hand (Windows), in UTC, and said so.
pub fn local_date(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    #[cfg(unix)]
    {
        // SAFETY: localtime_r writes only into `tm`, which lives on this stack; `t` is a valid time_t.
        let t = secs as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        if !unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
            return said_date(tm.tm_mday as u32, tm.tm_mon as u32, tm.tm_hour as u32, tm.tm_min as u32);
        }
    }
    // Days since the epoch to a civil date (Howard Hinnant's algorithm), in UTC.
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month0 = if mp < 10 { mp + 2 } else { mp - 10 } as u32;
    format!("{} UTC", said_date(day, month0, (rem / 3600) as u32, ((rem % 3600) / 60) as u32))
}

/// Now, in ms since the epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// L'en-tête d'une ligne de boîte, en français : `[équipier · réponse] `
/// (lot tty-8). Un genre ou un auteur inconnu s'écrit tel quel.
pub fn en_tete(l: &Value) -> String {
    // L'agent a un nom (c7, 15 nov.) : ses lignes le portent dans `name`.
    let nom = l["name"].as_str().filter(|n| !n.trim().is_empty() && l["from"] == "teammate");
    let de = match l["from"].as_str().unwrap_or("?") {
        "teammate" if nom.is_some() => nom.unwrap_or_default(),
        "teammate" => "équipier",
        "supervisor" => "toi",
        "host" => "hôte",
        autre => autre,
    };
    let kind = match l["kind"].as_str().unwrap_or("?") {
        "request" => "demande",
        "instruction" => "consigne",
        "answer" => "réponse",
        "report" => "compte rendu",
        "step_done" => "étape faite",
        "edit_plan" => "plan revu",
        "error" => "erreur",
        "started" => "parti",
        "question" => "question",
        "approval" => "accord demandé",
        "message" => "message",
        "stop" => "arrêt",
        "ended" => "fini",
        autre => autre,
    };
    let item = l["item"].as_str().filter(|i| !i.is_empty()).map(|i| format!(" [{i}]")).unwrap_or_default();
    format!("[{de} · {kind}{item}] ")
}

/// **L'état d'une tâche** dans `GET /ide/teammate/list` (`running`, `waiting`,
/// `ended`, `error`), cherché par son dépôt (`parent`) et son nom.
pub fn etat_dans_la_liste(liste: &Value, parent: &str, task: &str) -> Option<String> {
    let equipiers = liste["teammates"].as_array().or_else(|| liste["result"]["teammates"].as_array())?;
    equipiers.iter().find(|t| t["parent"] == parent && t["task"] == task).and_then(|t| t["state"].as_str()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(n: usize, from: &str, kind: &str, extra: Value) -> Value {
        let mut l = json!({ "id": format!("a1/doc:{n}"), "at": n, "from": from, "kind": kind, "text": format!("ligne {n}") });
        if let (Some(o), Some(e)) = (l.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                o.insert(k.clone(), v.clone());
            }
        }
        l
    }

    /// **L'agent a un nom** (c7, 15 nov.) : une ligne de l'équipier qui porte `name` se dit par lui,
    /// `[Algareth · compte rendu]` ; sans `name`, « équipier » ; les autres auteurs gardent le leur.
    #[test]
    fn a_teammate_line_with_a_name_is_headed_by_that_name() {
        assert_eq!(en_tete(&ligne(1, "teammate", "report", json!({ "name": "Algareth" }))), "[Algareth · compte rendu] ");
        assert_eq!(en_tete(&ligne(2, "teammate", "report", json!({ "name": "" }))), "[équipier · compte rendu] ");
        assert_eq!(en_tete(&ligne(3, "teammate", "report", json!({}))), "[équipier · compte rendu] ");
        assert_eq!(en_tete(&ligne(4, "supervisor", "request", json!({ "name": "Algareth" }))), "[toi · demande] ", "le nom est celui de l'agent, pas de toi");
    }

    #[test]
    fn what_the_teammate_asks_waits_until_a_line_replies_to_it() {
        let mut b = Boite::default();
        b.ligne(&ligne(0, "supervisor", "request", json!({})));
        b.ligne(&ligne(1, "teammate", "question", json!({ "item": "titre" })));
        b.ligne(&ligne(2, "teammate", "approval", json!({ "command": "cargo build" })));
        b.ligne(&ligne(3, "teammate", "report", json!({})));
        assert_eq!(b.attentes().iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), vec!["a1/doc:1", "a1/doc:2"], "un report n'attend pas de réponse");
        assert_eq!(b.attentes()[1].command.as_deref(), Some("cargo build"));
        b.ligne(&ligne(4, "supervisor", "answer", json!({ "reply_to": "a1/doc:1" })));
        assert_eq!(b.premiere().map(|a| a.id.as_str()), Some("a1/doc:2"), "la question répondue ne tient plus");
        b.ligne(&ligne(5, "host", "report", json!({ "reply_to": "a1/doc:2" })));
        assert!(b.attentes().is_empty(), "toute ligne qui y répond la retire");
    }

    #[test]
    fn the_answer_body_replies_to_the_item_and_carries_the_approval() {
        let a = Attente { id: "a1/doc:2".into(), kind: "approval".into(), text: "commande en attente d'accord : echo".into(), command: Some("echo".into()), ..Default::default() };
        assert_eq!(
            reponse("a1", "doc", &a, "oui", Some(true)),
            json!({ "backend": "a1", "task": "doc", "kind": "answer", "text": "oui", "reply_to": "a1/doc:2", "approved": true })
        );
        let q = Attente { id: "a1/doc:1".into(), kind: "question".into(), text: "Quel titre ?".into(), command: None, ..Default::default() };
        let r = reponse("a1", "doc", &q, "Petit dépôt", None);
        assert_eq!(r["reply_to"], "a1/doc:1");
        assert!(r.get("approved").is_none(), "une question n'a pas d'accord : {r}");
    }

    #[test]
    fn the_state_of_a_task_is_found_by_its_repo_and_name() {
        let liste = json!({ "supervisor": "supervisor", "teammates": [
            { "task": "doc", "parent": "b2", "state": "running" },
            { "task": "doc", "parent": "a1", "state": "waiting" }
        ] });
        assert_eq!(etat_dans_la_liste(&liste, "a1", "doc").as_deref(), Some("waiting"));
        assert_eq!(etat_dans_la_liste(&liste, "a1", "autre"), None);
    }
}
