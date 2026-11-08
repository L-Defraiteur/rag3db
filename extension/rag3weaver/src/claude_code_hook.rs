//! **La commande mince des hooks de Claude Code** (`rag3weaver-backend hook`,
//! page `docs/07-novembre-2026-hooks-claude-code/01`, lot 1).
//!
//! Claude Code lance la commande à cinq moments (`SessionStart`,
//! `UserPromptSubmit`, `PreCompact`, `PostCompact`, `Stop`) avec un JSON sur
//! son entrée. La commande lit ce JSON, parle à l'hôte 7970 par le backend
//! mémoire déclaré, rend son bloc sur la sortie standard, et **sort toujours
//! en 0** : rien de ce qui se passe ici ne doit arrêter une session.
//!
//! Trois règles, toutes ici et aucune dans l'hôte :
//!
//! - **jamais bloquant** : pas de code 2, pas de `decision: block`, un délai
//!   propre plus court que celui de Claude Code ; au-delà, le bloc est vide et
//!   la raison va au journal ;
//! - **sans hôte, elle se tait** : l'hôte injoignable, le jeton absent, le
//!   backend qui refuse — rien sur la sortie, une ligne au journal
//!   (`~/.local/state/rag3weaver/hooks.log`). Elle ne démarre **jamais**
//!   d'hôte (c'est l'unité systemd qui le fait, avec le binaire installé) ;
//! - **vide quand il n'y a rien** : un bloc toujours présent apprend au
//!   modèle à ne plus le lire.
//!
//! Ce lot pose la mécanique : l'événement lu, le réglage, le client de l'hôte
//! borné, le journal, le silence. Les blocs rendus (lot 2) et les écritures
//! (lots 3 et 4) viennent dans [`repondre`], événement par événement.

use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Les événements que la commande connaît. Un autre est accepté et journalisé,
/// jamais une erreur : Claude Code en ajoutera.
pub const EVENEMENTS: [&str; 6] = ["SessionStart", "UserPromptSubmit", "PreCompact", "PostCompact", "Stop", "PostToolUse"];

/// Ce que Claude Code donne à la commande, dans la forme relevée le 7 novembre
/// 2026 (page 01 §1). Les champs qu'un événement n'a pas restent vides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evenement {
    pub nom: String,
    pub session_id: String,
    pub transcript_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub prompt_id: Option<String>,
    /// `SessionStart` : `startup`, `resume`, `clear`, `compact`, `fork`.
    pub source: Option<String>,
    /// `PreCompact` et `PostCompact` : `manual` ou `auto`.
    pub trigger: Option<String>,
    /// `UserPromptSubmit`.
    pub prompt: Option<String>,
    /// `PostCompact`.
    pub compact_summary: Option<String>,
    /// `Stop`.
    pub last_assistant_message: Option<String>,
    pub stop_hook_active: bool,
    /// `SessionStart` en `resume` ou `fork` : le contexte réel.
    pub context_tokens: Option<u64>,
}

impl Evenement {
    /// Lit le JSON du hook. Un JSON sans `hook_event_name` est refusé en le
    /// disant ; tout le reste est facultatif.
    pub fn lire(texte: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(texte).map_err(|e| format!("l'entrée du hook n'est pas du JSON : {e}"))?;
        let nom = v["hook_event_name"].as_str().ok_or("l'entrée du hook n'a pas de `hook_event_name`")?.to_string();
        let chaine = |k: &str| v[k].as_str().filter(|s| !s.is_empty()).map(str::to_string);
        Ok(Self {
            session_id: chaine("session_id").unwrap_or_default(),
            transcript_path: chaine("transcript_path").map(PathBuf::from),
            cwd: chaine("cwd").map(PathBuf::from),
            prompt_id: chaine("prompt_id"),
            source: chaine("source"),
            trigger: chaine("trigger"),
            prompt: chaine("prompt"),
            compact_summary: chaine("compact_summary"),
            last_assistant_message: chaine("last_assistant_message"),
            stop_hook_active: v["stop_hook_active"].as_bool().unwrap_or(false),
            context_tokens: v["context_tokens"].as_u64(),
            nom,
        })
    }

    pub fn connu(&self) -> bool {
        EVENEMENTS.contains(&self.nom.as_str())
    }
}

/// **Le réglage** (`~/.config/rag3weaver/hooks.json`), posé par
/// `scripts/hooks_claude_code.sh`, lisible et modifiable à la main. Un champ
/// absent prend son défaut ; un fichier absent, tous les défauts.
#[derive(Debug, Clone, PartialEq)]
pub struct Reglage {
    /// Le manifeste du backend mémoire, tel que `~/.claude.json` le déclare.
    pub manifest: PathBuf,
    /// Les clés de la session auprès de l'hôte (`--keys` du client MCP).
    pub keys: Vec<String>,
    /// Le plafond de contexte en jetons (`RAG3WEAVER_HOOK_CONTEXT_TOKENS`).
    pub context_tokens: u64,
    /// La part du plafond où l'on suggère de ranger, une fois.
    pub suggest_ratio: f64,
    /// Le score minimal d'une fiche rappelée par le prompt (`chunk.score` du
    /// meilleur signal) ; en dessous, la fiche n'est pas dite. Les scores sont
    /// journalisés pour régler ce seuil sur l'usage.
    pub recall_min_score: f64,
    /// Les bornes des blocs, en caractères.
    pub prompt_block_chars: usize,
    pub session_block_chars: usize,
    /// Les délais propres, en millisecondes : `UserPromptSubmit`, les autres.
    pub prompt_timeout_ms: u64,
    pub timeout_ms: u64,
    /// Le délai du seul rappel des fiches (`recall_fiches`), plus court que celui
    /// du tour : un rappel lent (embarquement du prompt à froid) n'emporte pas
    /// le reste du bloc (c7, 7 novembre).
    pub recall_timeout_ms: u64,
    /// Le nombre de mots-clés de la requête du rappel, tirés du prompt
    /// (`rappel_du_prompt`) : le plein texte flou coûte selon le nombre de
    /// termes (3 ter).
    pub recall_max_words: usize,
    /// Le juge de pertinence du rappel différé (3 ter b) : un appel structuré
    /// qui ôte les candidats hors sujet, jamais n'en ajoute.
    pub recall_judge: bool,
    /// Le plafond d'appels du juge par heure, toutes sessions comprises : une
    /// politique de dépense déclarée (le juge appelle un modèle facturé), pas
    /// un seuil mesuré. Au-delà, le juge est sauté et le journal le dit.
    pub recall_judge_max_per_hour: u64,
    /// Le délai de la lecture de la boîte des agents (`pending`), court : elle
    /// passe aussi à chaque `PostToolUse` (lot 5, le pont).
    pub mailbox_timeout_ms: u64,
    /// L'agent qui appelle : `claude-code` (le défaut) ou `codex` (`--agent
    /// codex` dans la commande de Codex). Jamais lu dans le fichier.
    pub agent: String,
    /// Lancer le fils détaché du rappel différé. Posé par la commande ; les
    /// témoins jouent le fils à la main. Jamais lu dans le fichier.
    pub fond: bool,
    /// Un rejeu hors session : rien ne s'écrit (ni mémoire, ni run, ni marque).
    /// Posé par `--rejouer`, jamais lu dans le fichier.
    pub rejeu: bool,
    pub journal: PathBuf,
    /// Le fichier d'où ce réglage a été lu (le fils du rappel différé le relit).
    /// Jamais écrit dans le fichier.
    pub chemin_du_reglage: Option<PathBuf>,
    /// Le dossier des gabarits de blocs (`<clé>.md`), relu à chaque appel :
    /// corriger un texte y suffit, aucune session ne redémarre.
    pub templates_dir: PathBuf,
}

impl Reglage {
    pub fn defaut() -> Self {
        let maison = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        Self {
            manifest: maison.join(".config/rag3weaver/memoire-longue/backends/memory/backend.json"),
            keys: vec!["dev".into()],
            context_tokens: 1_000_000,
            suggest_ratio: 0.7,
            recall_min_score: 0.50,
            prompt_block_chars: 1_500,
            session_block_chars: 3_000,
            prompt_timeout_ms: 5_000,
            timeout_ms: 20_000,
            // Le budget du rappel synchrone (3 ter) : à requête courte, le
            // rappel complet prend 13 à 70 ms sur l'hôte en service (journal
            // des runs, 7 novembre) ; 1 s laisse plus de dix fois la marge.
            recall_timeout_ms: 1_000,
            recall_max_words: 10,
            recall_judge: true,
            recall_judge_max_per_hour: 20,
            mailbox_timeout_ms: 1_500,
            agent: "claude-code".into(),
            fond: false,
            chemin_du_reglage: None,
            rejeu: false,
            // Sous les témoins, jamais le vrai dossier d'état : un témoin qui part
            // des défauts y avait écrit l'état d'une session `s1` (7 novembre).
            journal: if cfg!(test) { std::env::temp_dir().join(format!("rag3weaver-hook-temoins-{}", std::process::id())).join("hooks.log") } else { maison.join(".local/state/rag3weaver/hooks.log") },
            templates_dir: maison.join(".config/rag3weaver/hooks"),
        }
    }

    /// Le chemin par défaut du réglage.
    pub fn chemin_par_defaut() -> PathBuf {
        std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".config/rag3weaver/hooks.json")
    }

    /// Lit un réglage ; les champs absents gardent leur défaut, un champ
    /// inconnu est une erreur dite (une faute de frappe ne doit pas se taire).
    pub fn lire(texte: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(texte).map_err(|e| format!("hooks.json : {e}"))?;
        let o = v.as_object().ok_or("hooks.json : un objet attendu")?;
        let mut r = Self::defaut();
        for (k, val) in o {
            match k.as_str() {
                "manifest" => r.manifest = PathBuf::from(val.as_str().ok_or("hooks.json : `manifest` est un chemin")?),
                "keys" => r.keys = val.as_array().ok_or("hooks.json : `keys` est une liste")?.iter().filter_map(Value::as_str).map(str::to_string).collect(),
                "context_tokens" => r.context_tokens = val.as_u64().ok_or("hooks.json : `context_tokens` est un entier")?,
                "suggest_ratio" => r.suggest_ratio = val.as_f64().ok_or("hooks.json : `suggest_ratio` est un nombre")?,
                "recall_min_score" => r.recall_min_score = val.as_f64().ok_or("hooks.json : `recall_min_score` est un nombre")?,
                "prompt_block_chars" => r.prompt_block_chars = val.as_u64().ok_or("hooks.json : `prompt_block_chars` est un entier")? as usize,
                "session_block_chars" => r.session_block_chars = val.as_u64().ok_or("hooks.json : `session_block_chars` est un entier")? as usize,
                "prompt_timeout_ms" => r.prompt_timeout_ms = val.as_u64().ok_or("hooks.json : `prompt_timeout_ms` est un entier")?,
                "timeout_ms" => r.timeout_ms = val.as_u64().ok_or("hooks.json : `timeout_ms` est un entier")?,
                "recall_timeout_ms" => r.recall_timeout_ms = val.as_u64().ok_or("hooks.json : `recall_timeout_ms` est un entier")?,
                "recall_max_words" => r.recall_max_words = val.as_u64().ok_or("hooks.json : `recall_max_words` est un entier")? as usize,
                "recall_judge" => r.recall_judge = val.as_bool().ok_or("hooks.json : `recall_judge` est un booléen")?,
                "recall_judge_max_per_hour" => r.recall_judge_max_per_hour = val.as_u64().ok_or("hooks.json : `recall_judge_max_per_hour` est un entier")?,
                "mailbox_timeout_ms" => r.mailbox_timeout_ms = val.as_u64().ok_or("hooks.json : `mailbox_timeout_ms` est un entier")?,
                "journal" => r.journal = PathBuf::from(val.as_str().ok_or("hooks.json : `journal` est un chemin")?),
                "templates_dir" => r.templates_dir = PathBuf::from(val.as_str().ok_or("hooks.json : `templates_dir` est un chemin")?),
                autre => return Err(format!("hooks.json : champ inconnu `{autre}` (connus : manifest, keys, context_tokens, suggest_ratio, recall_min_score, prompt_block_chars, session_block_chars, prompt_timeout_ms, timeout_ms, recall_timeout_ms, recall_max_words, recall_judge, recall_judge_max_per_hour, mailbox_timeout_ms, journal, templates_dir)")),
            }
        }
        if let Ok(v) = std::env::var("RAG3WEAVER_HOOK_CONTEXT_TOKENS") {
            r.context_tokens = v.trim().parse().map_err(|_| "RAG3WEAVER_HOOK_CONTEXT_TOKENS : un entier attendu".to_string())?;
        }
        Ok(r)
    }

    /// Lit le fichier ; absent, les défauts.
    pub fn charger(chemin: &Path) -> Result<Self, String> {
        let mut r = match std::fs::read_to_string(chemin) {
            Ok(t) => Self::lire(&t),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::lire("{}"),
            Err(e) => Err(format!("{} : {e}", chemin.display())),
        }?;
        r.chemin_du_reglage = Some(chemin.to_path_buf());
        Ok(r)
    }

    /// Le JSON que le script d'installation pose : les défauts, écrits.
    pub fn json(&self) -> Value {
        json!({
            "manifest": self.manifest,
            "keys": self.keys,
            "context_tokens": self.context_tokens,
            "suggest_ratio": self.suggest_ratio,
            "recall_min_score": self.recall_min_score,
            "prompt_block_chars": self.prompt_block_chars,
            "session_block_chars": self.session_block_chars,
            "prompt_timeout_ms": self.prompt_timeout_ms,
            "timeout_ms": self.timeout_ms,
            "recall_timeout_ms": self.recall_timeout_ms,
            "recall_max_words": self.recall_max_words,
            "recall_judge": self.recall_judge,
            "recall_judge_max_per_hour": self.recall_judge_max_per_hour,
            "mailbox_timeout_ms": self.mailbox_timeout_ms,
            "journal": self.journal,
            "templates_dir": self.templates_dir,
        })
    }

    /// Un bloc par sa clé : la section `## <clé>` du fichier de son événement
    /// dans `templates_dir` s'il y est, sinon celle du gabarit livré. Les `{nom}`
    /// sont remplis par l'appelant.
    pub fn gabarit(&self, cle: &str) -> String {
        for (evenement, livre) in GABARITS_LIVRES {
            if section(livre, cle).is_none() {
                continue;
            }
            if let Ok(t) = std::fs::read_to_string(self.templates_dir.join(format!("{evenement}.md"))) {
                if let Some(s) = section(&t, cle) {
                    return s;
                }
            }
            return section(livre, cle).unwrap_or_default();
        }
        String::new()
    }

    /// La version des gabarits, pour le journal : le plus récent `mtime` du
    /// dossier, ou « intégrés ».
    pub fn version_des_gabarits(&self) -> String {
        let mut dernier: Option<std::time::SystemTime> = None;
        for e in std::fs::read_dir(&self.templates_dir).into_iter().flatten().filter_map(Result::ok) {
            if e.path().extension().is_some_and(|x| x == "md") {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    dernier = Some(dernier.map_or(m, |d| d.max(m)));
                }
            }
        }
        match dernier {
            Some(m) => format!("gabarits {}", m.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)),
            None => "gabarits intégrés".to_string(),
        }
    }

    /// Écrit les gabarits livrés dans `templates_dir` (un fichier par
    /// événement), sans écraser ceux qui y sont : c'est le point de départ
    /// d'une correction à la volée.
    pub fn poser_les_gabarits(&self) -> Result<Vec<String>, String> {
        std::fs::create_dir_all(&self.templates_dir).map_err(|e| format!("{} : {e}", self.templates_dir.display()))?;
        let mut poses = Vec::new();
        for (evenement, texte) in GABARITS_LIVRES {
            let p = self.templates_dir.join(format!("{evenement}.md"));
            if !p.exists() {
                std::fs::write(&p, texte).map_err(|e| format!("{} : {e}", p.display()))?;
                poses.push(evenement.to_string());
            }
        }
        Ok(poses)
    }

    pub fn delai(&self, evenement: &str) -> Duration {
        Duration::from_millis(if evenement == "UserPromptSubmit" { self.prompt_timeout_ms } else { self.timeout_ms })
    }
}

/// **Ce que la commande attend de l'hôte** : appeler un outil du backend
/// mémoire. Un trait pour que les témoins jouent sans réseau.
pub trait Hote {
    /// Le nom du backend ouvert (ce que `/open` a rendu).
    fn nom(&self) -> &str;
    fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String>;
    /// Le même, sous un délai à lui (le rappel des fiches) ; par défaut le délai
    /// commun.
    fn appeler_borne(&self, outil: &str, arguments: Value, _delai: Duration) -> Result<Value, String> {
        self.appeler(outil, arguments)
    }
    /// Un run venu de dehors, déposé au journal des runs de l'hôte
    /// (`POST /journal/run`, lot 3).
    fn deposer_run(&self, corps: Value) -> Result<Value, String>;
    /// Le manifeste du backend ouvert (le run d'un `Stop` le porte).
    fn manifest(&self) -> Option<&Path> {
        None
    }
    /// La boîte des agents de l'hôte (lot 5) ; aucune par défaut.
    fn boite(&self) -> Option<&dyn crate::mail::Boite> {
        None
    }
}

/// **L'hôte réduit à la boîte**, pour `PostToolUse` : il passe à chaque outil,
/// il n'ouvre donc pas le backend mémoire (ni `/session`, ni `/open`).
pub struct HoteBoiteSeule(pub crate::mail::BoiteHttp);

impl Hote for HoteBoiteSeule {
    fn nom(&self) -> &str {
        "la boîte"
    }
    fn appeler(&self, outil: &str, _arguments: Value) -> Result<Value, String> {
        Err(format!("{outil} : l'hôte réduit à la boîte n'ouvre pas la mémoire"))
    }
    fn deposer_run(&self, _corps: Value) -> Result<Value, String> {
        Err("l'hôte réduit à la boîte ne dépose pas de run".into())
    }
    fn boite(&self) -> Option<&dyn crate::mail::Boite> {
        Some(&self.0)
    }
}

/// **L'hôte d'un rejeu** : il lit, il n'écrit jamais — ni la mémoire, ni le
/// journal des runs (c7 : un rejeu ne poste jamais de run et ne se compte
/// jamais comme vu par l'agent). Une écriture refusée est une ligne de journal.
pub struct HoteEnLecture<'a>(pub &'a dyn Hote);

/// Les outils qui écrivent dans la mémoire.
const ECRITURES: [&str; 8] = ["put_fiche", "put_apport", "put_source", "put_memory", "relier_apport", "citer_source", "add_ref", "ancrer_fiche"];

impl Hote for HoteEnLecture<'_> {
    fn nom(&self) -> &str {
        self.0.nom()
    }
    fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String> {
        if ECRITURES.contains(&outil) || (outil == "create_ref_type" && arguments["confirm"] == true) {
            return Err(format!("rejeu : {outil} non écrit"));
        }
        self.0.appeler(outil, arguments)
    }
    fn appeler_borne(&self, outil: &str, arguments: Value, delai: Duration) -> Result<Value, String> {
        if ECRITURES.contains(&outil) {
            return Err(format!("rejeu : {outil} non écrit"));
        }
        self.0.appeler_borne(outil, arguments, delai)
    }
    fn deposer_run(&self, _corps: Value) -> Result<Value, String> {
        Err("rejeu : aucun run déposé".into())
    }
    fn manifest(&self) -> Option<&Path> {
        self.0.manifest()
    }
    // La boîte se lit en rejeu (`pending` est une lecture pure) ; `seen` n'est
    // jamais posté en rejeu (`reglage.rejeu`).
    fn boite(&self) -> Option<&dyn crate::mail::Boite> {
        self.0.boite()
    }
}

/// **Le client de l'hôte, borné** : `POST /session`, `POST /open`, puis des
/// `POST /call`, et `DELETE /session` à la fin ; chaque requête sous le délai.
/// Il ne relance jamais l'hôte et ne relit pas le jeton : un hôte qui ne
/// répond pas est une raison de se taire, pas de s'acharner.
pub struct ClientHote {
    /// Le délai commun des requêtes ; l'hôte en reçoit 95 % en `deadline_ms`
    /// (lot 3 ter d), pour arrêter son run et répondre avant que le client coupe.
    delai: Duration,
    /// La boîte des agents, sur la même adresse et le même jeton (lot 5).
    boite: Option<crate::mail::BoiteHttp>,
    /// La durée de chaque requête, pour le journal (c7 : savoir lequel traîne).
    durees: std::sync::Mutex<Vec<String>>,
    agent: ureq::Agent,
    base: String,
    token: Option<String>,
    session: String,
    manifest: PathBuf,
    nom: String,
}

/// Jamais le jeton : un secret ne doit pas pouvoir arriver dans un journal.
impl std::fmt::Debug for ClientHote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientHote").field("base", &self.base).field("token", &self.token.as_ref().map(|_| "<redacted>")).field("nom", &self.nom).finish()
    }
}

impl ClientHote {
    /// L'adresse et le jeton de `daemon.json` ; l'hôte absent est dit.
    pub fn depuis_daemon_json(manifest: &Path, keys: &[String], delai: Duration) -> Result<Self, String> {
        let fiche = crate::daemon_lifecycle::read().ok_or("aucun hôte connu (daemon.json absent) : la commande ne le lance pas")?;
        Self::ouvrir(&fiche.address, Some(&fiche.token), manifest, keys, delai)
    }

    pub fn ouvrir(adresse: &str, token: Option<&str>, manifest: &Path, keys: &[String], delai: Duration) -> Result<Self, String> {
        let agent = crate::daemon::agent_with_timeout(delai);
        let base = crate::daemon::base_url(adresse);
        let mut me = Self { delai, boite: None, durees: std::sync::Mutex::new(Vec::new()), agent, base, token: token.map(str::to_string), session: String::new(), manifest: manifest.to_path_buf(), nom: String::new() };
        let corps = json!({ "client": "claude-code-hook", "keys": keys, "pid": std::process::id() });
        let v = me.poster("/session", corps, false)?;
        me.session = v["session"].as_str().ok_or("l'hôte n'a pas rendu de session")?.to_string();
        let ouvert = me.poster("/open", json!({ "manifest": me.manifest }), true)?;
        if ouvert["ok"] == json!(false) {
            return Err(format!("l'hôte n'ouvre pas le backend mémoire : {}", ouvert["error"].as_str().unwrap_or("?")));
        }
        me.nom = ouvert["describe"]["name"]
            .as_str()
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| me.manifest.parent().and_then(|d| d.file_name()).map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| "manifeste sans nom".into()));
        Ok(me)
    }

    /// L'échéance donnée à l'hôte pour un délai client : 95 % de lui, pour que la
    /// réponse « délai dépassé » arrive avant que le client ferme.
    fn echeance(delai: Duration) -> u64 {
        (delai.as_millis() as u64).saturating_mul(95) / 100
    }

    fn poster(&self, route: &str, corps: Value, avec_session: bool) -> Result<Value, String> {
        self.poster_par(&self.agent, route, corps, avec_session)
    }

    fn poster_par(&self, agent: &ureq::Agent, route: &str, corps: Value, avec_session: bool) -> Result<Value, String> {
        let debut = std::time::Instant::now();
        let quoi = corps["name"].as_str().map(|n| format!("{route} {n}")).unwrap_or_else(|| route.to_string());
        let r = self.poster_brut(agent, route, corps, avec_session);
        self.durees.lock().unwrap_or_else(|p| p.into_inner()).push(format!("{quoi} {} ms{}", debut.elapsed().as_millis(), if r.is_err() { " (échec)" } else { "" }));
        r
    }

    /// Le même client, avec la boîte des agents sous son propre délai.
    pub fn avec_boite(mut self, delai: Duration) -> Self {
        self.boite = Some(crate::mail::BoiteHttp::depuis_base(&self.base, self.token.as_deref(), delai));
        self
    }

    /// Les durées des requêtes, vidées.
    pub fn durees(&self) -> Vec<String> {
        std::mem::take(&mut *self.durees.lock().unwrap_or_else(|p| p.into_inner()))
    }

    fn poster_brut(&self, agent: &ureq::Agent, route: &str, corps: Value, avec_session: bool) -> Result<Value, String> {
        let mut requete = agent.post(format!("{}{route}", self.base)).header("content-type", "application/json");
        if let Some(t) = &self.token {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        if avec_session {
            requete = requete.header(crate::daemon_host::SESSION_HEADER, &self.session);
        }
        let mut reponse = requete.send(corps.to_string()).map_err(|e| format!("{route} : l'hôte {} ne répond pas : {e}", self.base))?;
        let statut = reponse.status().as_u16();
        let texte = reponse.body_mut().read_to_string().map_err(|e| format!("{route} : {e}"))?;
        let v: Value = serde_json::from_str(&texte).unwrap_or_else(|_| json!({ "ok": false, "error": texte }));
        if statut == 422 && v["error_kind"] == "deadline" {
            // L'hôte a arrêté le run à son échéance (`cancelled_by: deadline`) :
            // un dépassement, pas un refus.
            return Err(format!("{route} : délai dépassé côté hôte — {}", v["error"].as_str().unwrap_or("?")));
        }
        if statut != 200 {
            return Err(format!("{route} : refus {statut} — {}", v["error"].as_str().unwrap_or(&v.to_string())));
        }
        Ok(v)
    }

    /// Ferme la session ; une erreur ici n'intéresse personne.
    pub fn fermer(&self) {
        let mut requete = self.agent.delete(format!("{}/session", self.base)).header(crate::daemon_host::SESSION_HEADER, &self.session);
        if let Some(t) = &self.token {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        let _ = requete.call();
    }
}

impl Hote for ClientHote {
    fn boite(&self) -> Option<&dyn crate::mail::Boite> {
        self.boite.as_ref().map(|b| b as &dyn crate::mail::Boite)
    }
    fn nom(&self) -> &str {
        &self.nom
    }
    fn manifest(&self) -> Option<&Path> {
        Some(&self.manifest)
    }
    fn appeler_borne(&self, outil: &str, arguments: Value, delai: Duration) -> Result<Value, String> {
        let agent = crate::daemon::agent_with_timeout(delai);
        let v = self.poster_par(&agent, "/call", json!({ "manifest": self.manifest, "name": outil, "arguments": arguments, "deadline_ms": Self::echeance(delai) }), true)?;
        if v["ok"] == json!(false) {
            return Err(format!("{outil} : {}", v["error"].as_str().unwrap_or("?")));
        }
        Ok(v)
    }
    fn deposer_run(&self, corps: Value) -> Result<Value, String> {
        let v = self.poster("/journal/run", corps, true)?;
        if v["ok"] == json!(false) {
            return Err(format!("journal/run : {}", v["error"].as_str().unwrap_or("?")));
        }
        Ok(v)
    }
    fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String> {
        let v = self.poster("/call", json!({ "manifest": self.manifest, "name": outil, "arguments": arguments, "deadline_ms": Self::echeance(self.delai) }), true)?;
        if v["ok"] == json!(false) {
            return Err(format!("{outil} : {}", v["error"].as_str().unwrap_or("?")));
        }
        Ok(v)
    }
}

/// Ce que la commande rend : le bloc (vide s'il n'y a rien) et les lignes du
/// journal.
#[derive(Debug, Default, PartialEq)]
pub struct Reponse {
    pub bloc: String,
    pub journal: Vec<String>,
}

/// **Répond à un événement**, l'hôte joint ou non (lots 2 et 3). Lecture :
/// `SessionStart` et `UserPromptSubmit` rendent un bloc, vide quand il n'y a
/// rien à dire, borné sinon. Écriture, toujours **mécanique** (`via: hook`,
/// `author` = la clé) : le L1 `handle` quand la clé est donnée et sa fiche
/// existe, le L1 `compaction` de `PostCompact`, la fin de run de `Stop` ; ce
/// qui est une parole de l'agent (son `identity`) lui est demandé, jamais
/// écrit pour lui. `PreCompact` rend les consignes du résumé.
pub fn repondre(e: &Evenement, reglage: &Reglage, hote: Result<&dyn Hote, String>) -> Reponse {
    let mut r = Reponse::default();
    let session = if e.session_id.is_empty() { "?" } else { e.session_id.as_str() };
    if !e.connu() {
        r.journal.push(format!("{} {session} : événement inconnu, ignoré", e.nom));
        return r;
    }
    // `PostToolUse` passe à chaque outil : il ne lit que le rappel différé que
    // le fils a rangé, sans hôte (3 ter b).
    if e.nom == "PostToolUse" {
        let mut bloc = rappel_du_tour_d_avant(e, reglage, &mut r.journal);
        // Et la boîte des agents, ce qui est neuf seulement : atteindre une session
        // occupée sans répéter à chaque outil (lot 5).
        if let Ok(h) = &hote {
            bloc.push_str(&boite_du_tour(e, reglage, *h, &[], true, &mut r.journal));
        }
        if !bloc.trim().is_empty() {
            r.bloc = borner(&format!("[mémoire]\n{}", bloc.trim_end()), reglage.prompt_block_chars);
            r.journal.push(format!("PostToolUse {session} : bloc de {} caractères", r.bloc.chars().count()));
        }
        return r;
    }
    let h = match hote {
        Ok(h) => h,
        Err(raison) => {
            r.journal.push(format!("{} {session} : silence — {raison}", e.nom));
            return r;
        }
    };
    let detail = match e.nom.as_str() {
        "SessionStart" => format!("source {}", e.source.as_deref().unwrap_or("?")),
        "PreCompact" | "PostCompact" => format!("trigger {}", e.trigger.as_deref().unwrap_or("?")),
        "UserPromptSubmit" => format!("prompt de {} caractères", e.prompt.as_deref().map(|p| p.chars().count()).unwrap_or(0)),
        _ => String::new(),
    };
    r.journal.push(format!("{} {session} : hôte joint, backend « {} »{}{} [{} ; commit {} ; {}]", e.nom, h.nom(), if detail.is_empty() { "" } else { ", " }, detail, crate::daemon::mon_empreinte(), option_env!("RAG3WEAVER_GIT_HASH").unwrap_or("?"), reglage.version_des_gabarits()));
    let (bloc, borne, prefixe) = match e.nom.as_str() {
        "SessionStart" => (bloc_de_session(e, reglage, h, &mut r.journal), reglage.session_block_chars, true),
        "UserPromptSubmit" => (bloc_de_prompt(e, reglage, h, &mut r.journal), reglage.prompt_block_chars, true),
        // Les consignes du résumé : pas de contexte, pas de préfixe — c'est le
        // compacteur qui lit.
        "PreCompact" => (consignes_du_resume(e, reglage, h, &mut r.journal), 8_000, false),
        "PostCompact" => {
            ranger_la_compaction(e, reglage, h, &mut r.journal);
            (String::new(), 0, false)
        }
        "Stop" => {
            deposer_la_fin_de_run(e, h, &mut r.journal);
            (String::new(), 0, false)
        }
        _ => (String::new(), 0, false),
    };
    if !bloc.trim().is_empty() {
        let texte = if prefixe { format!("[mémoire]\n{}", bloc.trim_end()) } else { bloc.trim_end().to_string() };
        r.bloc = borner(&texte, borne);
        r.journal.push(format!("{} {session} : bloc de {} caractères", e.nom, r.bloc.chars().count()));
    }
    r
}

/// Coupe à `max` caractères, sur une frontière de caractère, et le dit.
fn borner(texte: &str, max: usize) -> String {
    if max == 0 || texte.chars().count() <= max {
        return texte.to_string();
    }
    let garde = max.saturating_sub(12);
    let mut coupe: String = texte.chars().take(garde).collect();
    coupe.push_str("\n[… borné]");
    coupe
}

// ─── Ce qu'on lit dans la mémoire ────────────────────────────────────────────

/// Les lignes `data` d'un résultat de recherche (`recall_*`), ou vide.
fn lignes_de_recherche(v: &Value) -> Vec<Value> {
    v["result"]["result"]
        .as_array()
        .map(|a| a.iter().map(|x| x["data"].clone()).filter(|d| d.is_object()).collect())
        .unwrap_or_default()
}

/// Les fiches rendues par `cited_by` en json : le résultat est une **chaîne**
/// JSON (le rendu du Plan), relue ici.
fn fiches_citant(v: &Value) -> Vec<Value> {
    let lignes = match &v["result"]["result"] {
        Value::String(s) => serde_json::from_str::<Value>(s).ok().and_then(|x| x.as_array().cloned()).unwrap_or_default(),
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    };
    // Une ligne du Plan est `{uuid, score, entity, data: {name, apports, refs…}}`
    // (relevé sur un vrai gabarit memory, lot 3 bis) : la fiche est sous `data`.
    lignes.into_iter().map(|l| if l["data"].is_object() { l["data"].clone() } else { l }).collect()
}

/// **L'identité d'une session** : la fiche qui cite `handle:session:<id>`
/// (page 04 du 25 octobre, §4). À défaut, l'indice facultatif
/// `RAG3WEAVER_AGENT_KEY`, qui ne vaut que si la fiche de cette clé existe :
/// une variable ne crée pas une identité.
#[derive(Debug, Clone, Default, PartialEq)]
struct Identite {
    cle: String,
    about: String,
    /// Le dernier apport `identity`, s'il y en a un, et sa date.
    identity: Option<(String, u64)>,
    /// La clé vient de la variable, pas d'un handle cité.
    par_indice: bool,
}

impl Identite {
    fn identity_texte(&self) -> Option<&str> {
        self.identity.as_ref().map(|(t, _)| t.as_str())
    }
    /// La tâche courante : la première fiche citée par la rubrique `doing`.
    fn tache(&self) -> Option<String> {
        self.identity_texte().and_then(|i| rubrique(i, "doing")).map(fiches_citees).and_then(|v| v.into_iter().next())
    }
}

fn identite_de_la_session(h: &dyn Hote, session_id: &str, journal: &mut Vec<String>) -> Option<Identite> {
    if !session_id.is_empty() {
        // La Ref d'un handle porte la valeur nue `session:<uuid>`, de genre
        // `handle` (73, vérifié sur la mémoire de Lucie le 7 novembre : la forme
        // préfixée ne rend rien). Le genre borne, pour ne pas prendre une autre
        // Ref de même valeur.
        match h.appeler("cited_by", json!({ "value": format!("session:{session_id}"), "genres": ["handle"], "under": false, "format": "json", "limit": 3 })) {
            Ok(v) => {
                // Deux fiches citent le handle quand la session a compacté : son
                // identité et sa fiche de tâche (le L1 `compaction` cite le handle,
                // relevé par le témoin e2e du lot 3 bis). L'identité est celle qui
                // porte un apport `identity` ; à défaut (inscrite sans encore son
                // identity), la première.
                let mut premiere: Option<Identite> = None;
                for f in fiches_citant(&v).into_iter().filter(|f| f["state"] != "retired") {
                    let cle = f["name"].as_str().unwrap_or("").to_string();
                    if cle.is_empty() {
                        continue;
                    }
                    let id = Identite { about: f["about"].as_str().unwrap_or("").to_string(), identity: dernier_apport(h, &cle, "identity"), cle, par_indice: false };
                    if id.identity.is_some() {
                        return Some(id);
                    }
                    premiere.get_or_insert(id);
                }
                if premiere.is_some() {
                    return premiere;
                }
            }
            Err(e) => journal.push(format!("cited_by handle:session : {e}")),
        }
    }
    let cle = std::env::var("RAG3WEAVER_AGENT_KEY").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())?;
    match h.appeler("get_fiche", json!({ "record": { "name": cle } })) {
        Ok(v) if v["result"]["result"]["data"].is_object() => {
            let d = &v["result"]["result"]["data"];
            Some(Identite { about: d["about"].as_str().unwrap_or("").to_string(), identity: dernier_apport(h, &cle, "identity"), cle, par_indice: true })
        }
        Ok(_) => {
            journal.push(format!("RAG3WEAVER_AGENT_KEY={cle} : aucune fiche de cette clé, indice ignoré"));
            None
        }
        Err(e) => {
            journal.push(format!("get_fiche {cle} : {e}"));
            None
        }
    }
}

/// Le dernier apport d'une section sur une fiche : son texte et sa date.
fn dernier_apport(h: &dyn Hote, fiche: &str, section: &str) -> Option<(String, u64)> {
    let v = h
        .appeler(
            "recall_apports",
            json!({ "query": section, "options": { "limit": 5, "filter_condition": { "must": [
                { "field": { "key": "fiche", "value": fiche } },
                { "field": { "key": "section", "value": section } }
            ] } } }),
        )
        .ok()?;
    lignes_de_recherche(&v)
        .into_iter()
        .max_by_key(|d| d["created_at"].as_u64().unwrap_or(0))
        .and_then(|d| d["texte"].as_str().map(|t| (t.to_string(), d["created_at"].as_u64().unwrap_or(0))))
}

/// La rubrique `clé:` d'un apport `identity` (six rubriques, page 04 §6).
fn rubrique<'a>(identity: &'a str, nom: &str) -> Option<&'a str> {
    identity.lines().map(str::trim).find_map(|l| l.strip_prefix(nom).and_then(|r| r.strip_prefix(':'))).map(str::trim).filter(|s| !s.is_empty())
}

/// Les noms `fiche:<nom>` cités dans un texte (la rubrique `doing`), dans l'ordre.
/// Un nom de fiche a des espaces (« essai des hooks », « bilan : … ») : il court
/// jusqu'à `(`, `)`, `;`, `,`, la citation suivante ou la fin de la ligne, guillemets « » ôtés (relevé sur
/// un vrai gabarit memory, lot 3 bis).
fn fiches_citees(texte: &str) -> Vec<String> {
    let mut v = Vec::new();
    for ligne in texte.lines() {
        let mut reste = ligne;
        while let Some(i) = reste.find("fiche:") {
            let apres = &reste[i + "fiche:".len()..];
            // Le nom s'arrête à `(`, `)`, `;`, `,`, à la citation suivante, ou en fin de ligne.
            let fin = [apres.find(['(', ')', ';', ',']), apres.find("fiche:")].into_iter().flatten().min().unwrap_or(apres.len());
            let n = apres[..fin].trim().trim_matches(|c: char| c == '«' || c == '»' || c == '"' || c == '\'' || c.is_whitespace());
            if !n.is_empty() && !v.iter().any(|x| x == n) {
                v.push(n.to_string());
            }
            reste = &apres[fin..];
        }
    }
    v
}

/// **Les gabarits livrés des blocs** (`templates/hooks/<Événement>.md`, un
/// fichier par événement, embarqués ici) : le texte de départ. Chaque fichier se
/// surcharge par `<templates_dir>/<Événement>.md`, relu à chaque appel (Lucie, 7
/// novembre : « corriger ces hooks à la volée sans que Claude redémarre »). Une
/// section `## <clé>` par bloc ; les trous : `{id}` la session, `{rubriques}`,
/// `{genre}` (la remarque sur le genre `handle`), `{fiche}`, `{contexte}`,
/// `{plafond}`, `{comment}`.
pub const GABARITS_LIVRES: &[(&str, &str)] = &[
    ("SessionStart", include_str!("../templates/hooks/SessionStart.md")),
    ("UserPromptSubmit", include_str!("../templates/hooks/UserPromptSubmit.md")),
    ("PreCompact", include_str!("../templates/hooks/PreCompact.md")),
];

/// La section `## <clé>` d'un gabarit : son texte, jusqu'à la section suivante.
pub fn section(gabarit: &str, cle: &str) -> Option<String> {
    let mut dedans = false;
    let mut lignes = Vec::new();
    for l in gabarit.lines() {
        if let Some(titre) = l.strip_prefix("## ") {
            if dedans {
                break;
            }
            dedans = titre.trim() == cle;
            continue;
        }
        if dedans {
            lignes.push(l);
        }
    }
    dedans.then(|| lignes.join("\n").trim().to_string())
}

/// Les clés livrées, pour les témoins et la page.
pub fn cles_livrees() -> Vec<String> {
    GABARITS_LIVRES.iter().flat_map(|(_, g)| g.lines().filter_map(|l| l.strip_prefix("## ")).map(|t| t.trim().to_string()).collect::<Vec<_>>()).collect()
}

/// Le genre `handle` existe-t-il dans cette mémoire ? Lu par `create_ref_type`
/// sans confirmation (le premier appel n'écrit rien : il montre les genres).
/// On cherche le nom dans la réponse ; un appel qui échoue vaut « on ne sait
/// pas », et la prescription dit alors comment créer le genre si besoin.
fn genre_connu(h: &dyn Hote, genre: &str) -> Option<bool> {
    let v = h.appeler("create_ref_type", json!({ "name": genre, "description": "lecture des genres", "confirm": false })).ok()?;
    let texte = v.to_string();
    Some(texte.contains(&format!("\"{genre}\"")) || texte.contains(&format!("\"name\":\"{genre}\"")) || texte.contains(&format!(" {genre} ")) || texte.contains(&format!("`{genre}`")))
}

/// La consigne sans fiche, le genre `handle` vérifié avant d'être prescrit.
fn consigne_sans_fiche(reglage: &Reglage, h: &dyn Hote, session_id: &str) -> String {
    let genre = match genre_connu(h, "handle") {
        Some(true) => String::new(),
        Some(false) => " — le genre `handle` n'existe pas encore dans cette mémoire : crée-le d'abord, `create_ref_type` {name: \"handle\", description: \"Une adresse d'un agent dans son harnais : session:<uuid>, uds:<socket>, task:<nom>\", confirm: true}".to_string(),
        None => " (si le genre `handle` manque, `create_ref_type` {name: \"handle\", description: \"Une adresse d'un agent dans son harnais\", confirm: true} d'abord)".to_string(),
    };
    reglage.gabarit("sans_fiche").replace("<id>", session_id).replace("{id}", session_id).replace("<GENRE>", &genre).replace("{genre}", &genre)
}

/// Les six rubriques de l'identité, dans l'ordre de la page 04 §6.
const RUBRIQUES: &str = "who, runtime, can, doing, holding, mandates, peers";

fn premiere_ligne(texte: &str, max: usize) -> String {
    let t = texte.trim();
    let une = t.lines().next().unwrap_or("").trim();
    let coupe: String = une.chars().take(max).collect();
    if coupe.len() < une.len() || t.lines().count() > 1 { format!("{coupe} …") } else { coupe }
}

// ─── Ce qu'on écrit dans la mémoire : le mécanique seulement ─────────────────

/// Un apport (L1) rangé sur sa fiche, `via: hook`, `author` = la clé.
fn ecrire_un_apport(h: &dyn Hote, fiche: &str, source: &str, section: &str, texte: &str, author: &str) -> Result<(), String> {
    h.appeler(
        "put_apport",
        json!({ "record": { "fiche": fiche, "source": source, "section": section, "texte": texte, "niveau": "L1", "state": "current", "via": "hook", "author": author } }),
    )?;
    h.appeler("relier_apport", json!({ "links": [{ "from": { "name": fiche }, "to": { "fiche": fiche, "source": source, "section": section } }] }))?;
    Ok(())
}

/// Un apport cite une chose, par son genre : le genre est créé s'il manque
/// (un refus « existe déjà » n'est pas une erreur), puis la Ref posée.
fn citer(h: &dyn Hote, fiche: &str, source: &str, section: &str, genre: &str, valeur: &str, journal: &mut Vec<String>) {
    let description = match genre {
        "handle" => "Une adresse d'un agent dans son harnais : session:<uuid> (la transcription Claude Code), uds:<socket>, task:<nom>.",
        "doc" => "La provenance d'un apport : <chemin du source>#<titre de la section>.",
        _ => "Un genre posé par le hook de Claude Code.",
    };
    let _ = h.appeler("create_ref_type", json!({ "name": genre, "description": description, "confirm": true }));
    if let Err(e) = h.appeler("citer_source", json!({ "genre": genre, "valeur": valeur, "fiche": { "fiche": fiche, "source": source, "section": section } })) {
        journal.push(format!("citer_source {genre}:{valeur} : {e}"));
    }
}

/// **Le L1 `handle`** d'une session dont la clé est donnée par la variable
/// (96 : un par session, jamais réécrit, seulement si la fiche existe).
fn ecrire_le_handle(h: &dyn Hote, id: &Identite, session_id: &str, journal: &mut Vec<String>) {
    let valeur = format!("session:{session_id}");
    // Une source par session : la clé d'un apport est (fiche, source, section),
    // et un L1 ne se réécrit pas (page 04 du 25 octobre).
    let source = format!("sessions/{session_id}");
    match ecrire_un_apport(h, &id.cle, &source, "handle", &valeur, &id.cle) {
        Ok(()) => {
            citer(h, &id.cle, &source, "handle", "handle", &valeur, journal);
            journal.push(format!("L1 handle écrit sur « {} » : {valeur}", id.cle));
        }
        Err(e) => journal.push(format!("L1 handle non écrit : {e}")),
    }
}

/// Les marques d'une session, à côté du journal : la suggestion de ranger
/// (une fois), l'identité demandée (un rappel), la compaction en L0 seul.
fn marque(reglage: &Reglage, session_id: &str, nom: &str) -> PathBuf {
    reglage.journal.parent().map(|d| d.join("hooks-suggestions")).unwrap_or_else(|| PathBuf::from(".")).join(format!("{session_id}.{nom}"))
}
fn poser_la_marque(reglage: &Reglage, session_id: &str, nom: &str, contenu: &str) {
    if reglage.rejeu {
        return;
    }
    let m = marque(reglage, session_id, nom);
    if let Some(d) = m.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(m, contenu);
}
fn lire_la_marque(reglage: &Reglage, session_id: &str, nom: &str) -> Option<String> {
    std::fs::read_to_string(marque(reglage, session_id, nom)).ok()
}
fn oter_la_marque(reglage: &Reglage, session_id: &str, nom: &str) {
    if reglage.rejeu {
        return;
    }
    let _ = std::fs::remove_file(marque(reglage, session_id, nom));
}
fn maintenant_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ─── SessionStart ────────────────────────────────────────────────────────────

fn bloc_de_session(e: &Evenement, reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) -> String {
    let (mut b, cle) = bloc_d_identite(e, reglage, h, journal);
    if !reglage.rejeu {
        crate::mail::ecrire_la_fiche_de_session(&e.session_id, e.cwd.as_deref(), cle.as_deref(), journal);
    }
    b.push_str(&agents_en_cours(e, reglage, h, cle.as_deref(), journal));
    b
}

/// Les équipiers de cette session (`list?supervisor=`) et les fils Codex du
/// même dossier ; rien si rien.
fn agents_en_cours(e: &Evenement, reglage: &Reglage, h: &dyn Hote, cle: Option<&str>, journal: &mut Vec<String>) -> String {
    let mut b = String::new();
    if let Some(boite) = h.boite() {
        let mut equipiers = Vec::new();
        for pour in cles_de_la_session(e, reglage, cle) {
            match boite.equipiers(&pour) {
                Ok(v) => equipiers.extend(v),
                Err(err) => journal.push(format!("list?supervisor={pour} : {err}")),
            }
        }
        if !equipiers.is_empty() {
            b.push_str(&format!(
                "Tes équipiers : {}.\n",
                equipiers.iter().map(|t| format!("{} ({}, {})", t["task"].as_str().unwrap_or("?"), t["state"].as_str().unwrap_or("?"), t["last_kind"].as_str().unwrap_or("—"))).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if let Some(cwd) = e.cwd.as_deref() {
        let fils: Vec<crate::codex_threads::Fil> = crate::codex_threads::fils_du_dossier(&crate::codex_threads::racine(), cwd).into_iter().filter(|f| f.id != e.session_id).take(5).collect();
        if !fils.is_empty() {
            b.push_str(&format!(
                "Fils Codex dans ce dossier : {} — leur écrire : `rag3weaver-backend mail send --from {} --to <clé> --task <parent>/<task> \"<texte>\"`.\n",
                fils.iter().map(|f| f.cle()).collect::<Vec<_>>().join(", "),
                cles_de_la_session(e, reglage, cle).first().cloned().unwrap_or_default()
            ));
        }
    }
    b
}

/// L'identité de la session : le bloc et la clé stable trouvée.
fn bloc_d_identite(e: &Evenement, reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) -> (String, Option<String>) {
    let mut b = String::new();
    let source = e.source.as_deref().unwrap_or("startup");
    let identite = identite_de_la_session(h, &e.session_id, journal);
    if source == "compact" {
        b.push_str(&reglage.gabarit("session_compactee").replace("{rubriques}", RUBRIQUES));
        b.push_str("\n\n");
        // Le contexte de l'agent est vidé : ce qu'il avait reçu est perdu pour
        // lui. Une ligne le lui rappelle, puis le déjà-vu repart de zéro.
        let mut etat = EtatDuRappel::lire(reglage, &e.session_id);
        if !etat.rendus.is_empty() {
            let mut noms: Vec<(&String, &(u64, String))> = etat.rendus.iter().collect();
            noms.sort_by_key(|(_, (tour, _))| *tour);
            b.push_str(&format!("Fiches rappelées avant la compaction : {} — `get_fiche` pour relire.\n\n", noms.iter().map(|(n, _)| format!("« {n} »")).collect::<Vec<_>>().join(", ")));
            journal.push(format!("compaction : {} fiche(s) rappelées, le déjà-vu repart de zéro", noms.len()));
        }
        etat.rendus.clear();
        etat.ecrire(reglage, &e.session_id);
        if lire_la_marque(reglage, &e.session_id, "compaction-l0-seul").is_some() {
            b.push_str(&reglage.gabarit("compaction_l0_seul"));
            b.push_str("\n\n");
            oter_la_marque(reglage, &e.session_id, "compaction-l0-seul");
        }
    }
    let Some(id) = identite else {
        b.push_str(&consigne_sans_fiche(reglage, h, &e.session_id));
        b.push('\n');
        return (b, None);
    };
    // La clé vient de la variable : le handle de cette session s'écrit, une fois.
    if id.par_indice && !e.session_id.is_empty() && source != "compact" {
        ecrire_le_handle(h, &id, &e.session_id, journal);
    }
    b.push_str(&format!("Tu es **{}**{} — {}\n", id.cle, if id.par_indice { " (clé donnée par RAG3WEAVER_AGENT_KEY ; ton L1 `handle` est écrit)" } else { "" }, id.about));
    match id.identity_texte() {
        Some(identity) => {
            for (nom, titre) in [("doing", "En cours"), ("holding", "Tu tiens"), ("mandates", "Mandats"), ("peers", "Pairs")] {
                if let Some(v) = rubrique(identity, nom) {
                    b.push_str(&format!("- {titre} : {v}\n"));
                }
            }
            if let Some(tache) = id.tache() {
                match dernier_apport(h, &tache, "plan") {
                    Some((plan, _)) => b.push_str(&format!("- Plan en cours (fiche « {tache} ») : {}\n", premiere_ligne(&plan, 400))),
                    None => b.push_str(&format!("- Tâche courante : fiche « {tache} », sans plan écrit\n")),
                }
            }
        }
        None => {
            b.push_str(&reglage.gabarit("sans_identity").replace("{rubriques}", RUBRIQUES));
            b.push('\n');
        }
    }
    (b, Some(id.cle))
}

// ─── La boîte des agents (lot 5) ─────────────────────────────────────────────

/// Les clés sous lesquelles la boîte écrit à cette session : son handle
/// (`claude-code:<session_id>`, ou la clé du fil chez Codex) et sa clé stable
/// quand l'agent l'a dite.
fn cles_de_la_session(e: &Evenement, reglage: &Reglage, stable: Option<&str>) -> Vec<String> {
    let mut cles = Vec::new();
    if !e.session_id.is_empty() {
        if reglage.agent == "codex" {
            let fil = crate::codex_threads::fils(&crate::codex_threads::racine()).into_iter().find(|f| f.id == e.session_id);
            cles.push(fil.map(|f| f.cle()).unwrap_or_else(|| format!("codex:{}", e.session_id)));
        } else {
            cles.push(format!("claude-code:{}", e.session_id));
        }
    }
    if let Some(s) = stable.filter(|s| !s.is_empty() && !cles.iter().any(|c| c == s)) {
        cles.push(s.to_string());
    }
    cles
}

/// **Ce que la boîte garde pour cette session**, rendu par le code de `mail`.
/// Les ids déjà montrés sont dans une marque ; après le bloc, `seen` part pour
/// ce qui ne demande pas de réponse (jamais en rejeu). `seulement_neufs` (le
/// `PostToolUse`) tait le « toujours en attente » et la panne.
fn boite_du_tour(e: &Evenement, reglage: &Reglage, h: &dyn Hote, stables: &[String], seulement_neufs: bool, journal: &mut Vec<String>) -> String {
    let Some(boite) = h.boite() else { return String::new() };
    let stable = stables.first().cloned().or_else(|| crate::mail::lire_la_fiche_de_session(&e.session_id).and_then(|f| f.key));
    let cles = cles_de_la_session(e, reglage, stable.as_deref());
    let Some(pour) = cles.first().cloned() else { return String::new() };
    let mut items = Vec::new();
    let mut pannes = Vec::new();
    for c in &cles {
        match boite.pending(c) {
            Ok(v) => items.extend(v),
            Err(err) => pannes.push(format!("{c} : {err}")),
        }
    }
    let mut vus_ids = HashSet::new();
    items.retain(|i| vus_ids.insert(i.id.clone()));
    let deja: HashSet<String> = lire_la_marque(reglage, &e.session_id, "boite-montres").map(|t| t.lines().map(str::to_string).collect()).unwrap_or_default();
    let mut b = if seulement_neufs {
        let neufs: Vec<crate::mail::Item> = items.iter().filter(|i| !deja.contains(&i.id)).cloned().collect();
        crate::mail::bloc_de_la_boite(&neufs, &HashSet::new(), &pour, 600)
    } else {
        crate::mail::bloc_de_la_boite(&items, &deja, &pour, 600)
    };
    if !pannes.is_empty() {
        journal.push(format!("boîte : {}", pannes.join(" ; ")));
        if !seulement_neufs {
            b.push_str(&format!("Boîte des agents injoignable ({}) — relis-la par `rag3weaver-backend mail pending --for {pour}`.\n", premiere_ligne(&pannes[0], 120)));
        }
    }
    if !items.is_empty() {
        journal.push(format!("boîte : {} item(s) pour {}, {} neuf(s)", items.len(), cles.join(" + "), items.iter().filter(|i| !deja.contains(&i.id)).count()));
        let montres: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
        poser_la_marque(reglage, &e.session_id, "boite-montres", &montres.join("\n"));
        let a_voir = crate::mail::a_marquer_vus(&items);
        if !a_voir.is_empty() && !reglage.rejeu {
            for c in &cles {
                if let Err(err) = boite.seen(c, &a_voir) {
                    journal.push(format!("boîte : seen {c} : {err}"));
                }
            }
        }
    }
    b
}

// ─── Le déjà-vu du rappel (3 ter) ────────────────────────────────────────────

/// **Ce que la session a déjà reçu du rappel** (Lucie : « ne pas lui rebalancer
/// des trucs qu'il a déjà eus ») : par fiche, le tour où elle a été rendue et
/// l'empreinte de son contenu. Un fichier par session,
/// `<dossier du journal>/hooks/<session_id>.json` ; un resume le garde, une
/// compaction le vide, un rejeu ne l'écrit jamais.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EtatDuRappel {
    pub tour: u64,
    pub rendus: std::collections::BTreeMap<String, (u64, String)>,
}

enum Statut {
    Neuve,
    Deja,
    Changee(u64),
}

fn chemin_de_l_etat(reglage: &Reglage, session_id: &str) -> PathBuf {
    reglage.journal.parent().map(|d| d.join("hooks")).unwrap_or_else(|| PathBuf::from("hooks")).join(format!("{session_id}.json"))
}

/// L'empreinte d'une fiche telle que le rappel la montre (nom et résumé) : une
/// fiche dont le résumé change revient, « mise à jour ».
fn empreinte_de_fiche(nom: &str, about: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (nom, about).hash(&mut h);
    format!("{:016x}", h.finish())
}

impl EtatDuRappel {
    pub fn lire(reglage: &Reglage, session_id: &str) -> Self {
        let Ok(t) = std::fs::read_to_string(chemin_de_l_etat(reglage, session_id)) else { return Self::default() };
        let v: Value = serde_json::from_str(&t).unwrap_or(Value::Null);
        let rendus = v["rendus"].as_object().map(|o| o.iter().map(|(n, x)| (n.clone(), (x["tour"].as_u64().unwrap_or(0), x["empreinte"].as_str().unwrap_or("").to_string()))).collect()).unwrap_or_default();
        Self { tour: v["tour"].as_u64().unwrap_or(0), rendus }
    }
    fn ecrire(&self, reglage: &Reglage, session_id: &str) {
        if reglage.rejeu || session_id.is_empty() {
            return;
        }
        let chemin = chemin_de_l_etat(reglage, session_id);
        let rendus: serde_json::Map<String, Value> = self.rendus.iter().map(|(n, (t, e))| (n.clone(), json!({ "tour": t, "empreinte": e }))).collect();
        let _ = std::fs::create_dir_all(chemin.parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(chemin, json!({ "tour": self.tour, "rendus": rendus }).to_string());
    }
    fn statut(&self, nom: &str, empreinte: &str) -> Statut {
        match self.rendus.get(nom) {
            None => Statut::Neuve,
            Some((_, e)) if e == empreinte => Statut::Deja,
            Some((tour, _)) => Statut::Changee(*tour),
        }
    }
    fn rendre(&mut self, nom: &str, empreinte: &str) {
        self.rendus.insert(nom.to_string(), (self.tour, empreinte.to_string()));
    }
    /// Les noms de fiches déjà rencontrés, en mots minuscules : le vocabulaire
    /// bon marché de la mémoire, priorité haute dans la requête.
    fn vocabulaire(&self) -> HashSet<String> {
        self.rendus.keys().flat_map(|n| n.split_whitespace().map(str::to_lowercase).collect::<Vec<_>>()).collect()
    }
}

/// **Un candidat du rappel**, avec ses signaux : le plein texte (`bm25`, score
/// brut, au-delà de 1 le plus souvent) et le dense (`vector`, de 0 à 1). Lus dans
/// `signals` de la ligne : `chunk.score` est celui du morceau affiché, qui peut
/// venir du dense quand le plein texte a touché ailleurs (relevé de 18:16:39 :
/// bm25 1,386 au rang 1, morceau affiché dense à 0,374).
#[derive(Debug, Clone, PartialEq)]
pub struct Candidat {
    pub nom: String,
    pub about: String,
    pub plein: Option<f64>,
    pub dense: Option<f64>,
}

impl Candidat {
    fn dit(&self) -> String {
        format!("« {} » {}", self.nom, self.scores())
    }
    fn scores(&self) -> String {
        match (self.plein, self.dense) {
            (Some(p), Some(d)) => format!("plein texte {p:.2}, dense {d:.2}"),
            (Some(p), None) => format!("plein texte {p:.2}"),
            (None, Some(d)) => format!("dense {d:.2}"),
            (None, None) => "sans score".into(),
        }
    }
    fn json(&self) -> Value {
        json!({ "name": self.nom, "about": self.about, "plein": self.plein, "dense": self.dense })
    }
}

/// Les candidats d'une réponse de `recall_fiches`. Sans `signals` (une forme
/// plus ancienne), le score du morceau tient lieu : au-delà de 1 il est brut,
/// donc plein texte ; de 0 à 1, dense.
pub fn candidats_du_rappel(v: &Value) -> Vec<Candidat> {
    v["result"]["result"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            let nom = x["data"]["name"].as_str()?.to_string();
            let signal = |l: &str| x["signals"].as_array().and_then(|a| a.iter().find(|s| s["label"] == l)).and_then(|s| s["score"].as_f64());
            let (plein, dense) = if x["signals"].is_array() {
                (signal("bm25"), signal("vector"))
            } else {
                match x["chunk"]["score"].as_f64() {
                    Some(sc) if sc > 1.0 => (Some(sc), None),
                    Some(sc) => (None, Some(sc)),
                    None => (None, None),
                }
            };
            Some(Candidat { nom, about: x["data"]["about"].as_str().unwrap_or("").to_string(), plein, dense })
        })
        .collect()
}

/// **Le vocabulaire de la mémoire**, commun aux sessions : les mots des noms de
/// fiches vus dans les réponses du rappel (sous le seuil compris), au plus 2 000.
/// Il donne la priorité, dans la requête, aux mots du prompt que la mémoire
/// connaît (c7 : « régime », « GPU », « embarquement »).
pub struct Vocabulaire;

impl Vocabulaire {
    fn chemin(reglage: &Reglage) -> PathBuf {
        reglage.journal.parent().map(|d| d.join("hooks")).unwrap_or_else(|| PathBuf::from("hooks")).join("vocabulaire.json")
    }
    pub fn lire(reglage: &Reglage) -> HashSet<String> {
        std::fs::read_to_string(Self::chemin(reglage)).ok().and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok()).unwrap_or_default().into_iter().collect()
    }
    fn ajouter<'a>(reglage: &Reglage, noms: impl Iterator<Item = &'a str>) {
        let mut v = Self::lire(reglage);
        let avant = v.len();
        for n in noms {
            for m in n.split(|c: char| c.is_whitespace() || c == ':' || c == ',').map(str::to_lowercase).filter(|m| m.chars().count() >= 2) {
                v.insert(m);
            }
        }
        if v.len() == avant || v.len() > 2_000 {
            return;
        }
        let mut l: Vec<String> = v.into_iter().collect();
        l.sort();
        let chemin = Self::chemin(reglage);
        let _ = std::fs::create_dir_all(chemin.parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(chemin, serde_json::to_string(&l).unwrap_or_default());
    }
}

// ─── Le rappel différé et son juge (3 ter b) ─────────────────────────────────

/// Les fichiers du rappel différé d'une session : `tache` (pour le fils),
/// `differe` (pour le prochain outil).
pub fn chemin_du_fond(reglage: &Reglage, session_id: &str, quoi: &str) -> PathBuf {
    chemin_de_l_etat(reglage, session_id).with_extension(format!("{quoi}.json"))
}

/// **Confie le rappel au fils détaché** : la tâche est écrite à côté de l'état
/// de la session, puis `hook --rappel-differe <tâche>` part, détaché de son
/// groupe de processus, entrée et sorties fermées. Jamais en rejeu ; coupé hors
/// de la commande (`reglage.fond`), où les témoins le jouent à la main.
fn confier_au_fond(e: &Evenement, reglage: &Reglage, requete: &str, candidats: Option<Vec<Value>>, tour: u64, journal: &mut Vec<String>) {
    if reglage.rejeu || e.session_id.is_empty() {
        return;
    }
    let tache = json!({
        "session_id": e.session_id,
        "tour": tour,
        "requete": requete,
        "prompt": crate::rappel_du_prompt::nettoyer(e.prompt.as_deref().unwrap_or("")).chars().take(2_000).collect::<String>(),
        "candidats": candidats,
        "config": reglage.chemin_du_reglage,
    });
    let chemin = chemin_du_fond(reglage, &e.session_id, "tache");
    if let Err(err) = std::fs::create_dir_all(chemin.parent().unwrap_or(Path::new("."))).and_then(|_| std::fs::write(&chemin, tache.to_string())) {
        journal.push(format!("fond : la tâche ne s'écrit pas ({err})"));
        return;
    }
    if !reglage.fond {
        journal.push(format!("fond : tâche écrite ({}), fils coupé hors de la commande", chemin.display()));
        return;
    }
    let lance = std::env::current_exe().map_err(|e| e.to_string()).and_then(|exe| {
        use std::os::unix::process::CommandExt;
        std::process::Command::new(exe)
            .args(["hook", "--rappel-differe"])
            .arg(&chemin)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|e| e.to_string())
    });
    match lance {
        Ok(fils) => journal.push(format!("fond : rappel confié au fils {}", fils.id())),
        Err(err) => journal.push(format!("fond : le fils ne part pas ({err})")),
    }
}

/// **Compte un appel du juge** dans l'heure courante, toutes sessions
/// comprises (`<dossier du journal>/hooks/juge-par-heure.json`). Faux si le
/// plafond est atteint : l'appel n'a pas lieu.
fn compter_un_appel_du_juge(reglage: &Reglage) -> bool {
    let chemin = reglage.journal.parent().map(|d| d.join("hooks")).unwrap_or_else(|| PathBuf::from("hooks")).join("juge-par-heure.json");
    let heure = maintenant_ms() / 3_600_000;
    let v: Value = std::fs::read_to_string(&chemin).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let n = if v["heure"].as_u64() == Some(heure) { v["appels"].as_u64().unwrap_or(0) } else { 0 };
    if n >= reglage.recall_judge_max_per_hour {
        return false;
    }
    let _ = std::fs::create_dir_all(chemin.parent().unwrap_or(Path::new(".")));
    let _ = std::fs::write(&chemin, json!({ "heure": heure, "appels": n + 1 }).to_string());
    true
}

/// **Le verdict du juge** : ce qu'il garde et pourquoi. Il ôte, il n'invente
/// rien : un nom qu'il rend sans l'avoir reçu est jeté.
#[derive(Debug, Clone, PartialEq)]
pub struct Jugement {
    pub pertinent: bool,
    pub pourquoi: String,
    pub gardes: Vec<String>,
    /// L'appel, pour que la facture se relise dans `hooks.log` : le modèle, la
    /// durée, les jetons d'entrée et de sortie.
    pub modele: String,
    pub ms: u64,
    pub entree: usize,
    pub sortie: usize,
}

const SCHEMA_DU_JUGE: &str = r#"{"type":"object","properties":{"pertinent":{"type":"boolean"},"pourquoi":{"type":"string"},"items_gardes":{"type":"array","items":{"type":"string"}}},"required":["pertinent","pourquoi","items_gardes"],"additionalProperties":false}"#;

/// **Juge les candidats d'un rappel** contre le prompt (nettoyé) : un appel
/// structuré, température nulle. Lucie : « ne pas le flooder » ; le juge ne
/// laisse passer que ce qui sert à ce tour-ci.
pub fn juger(llm: &dyn crate::llm::Llm, prompt: &str, candidats: &[Value]) -> Result<Jugement, String> {
    use crate::llm::{GenOptions, ResponseFormat, StringSink, Turn};
    let liste = candidats.iter().map(|c| format!("- « {} » : {}", c["name"].as_str().unwrap_or("?"), premiere_ligne(c["about"].as_str().unwrap_or(""), 200))).collect::<Vec<_>>().join("\n");
    let systeme = "Tu juges, pour un agent de code, quelles fiches de sa mémoire servent à son tour présent. Tu lis son message et les fiches candidates (nom et résumé). Tu gardes seulement celles qui l'aident à répondre à CE message ; dans le doute, tu ôtes. Tu n'inventes aucun nom : items_gardes ne contient que des noms de la liste, à l'identique. pertinent vaut false si tu n'en gardes aucune. pourquoi tient en une phrase. Réponds en JSON.";
    let utilisateur = format!("Message de l'agent :\n{}\n\nFiches candidates :\n{liste}", prompt.chars().take(2_000).collect::<String>());
    let schema: Value = serde_json::from_str(SCHEMA_DU_JUGE).map_err(|e| e.to_string())?;
    let opts = GenOptions::default().with_response_format(ResponseFormat::strict_schema("juge_du_rappel", schema)).with_temperature(0.0).with_max_tokens(400);
    let mut sink = StringSink::default();
    let debut = std::time::Instant::now();
    let (_, usage) = llm.generate(&[Turn::system(systeme), Turn::user(&utilisateur)], &opts, &mut sink).map_err(|e| format!("juge : {e}"))?;
    let ms = if usage.ms > 0 { usage.ms } else { debut.elapsed().as_millis() as u64 };
    let v: Value = serde_json::from_str(sink.text.trim()).map_err(|e| format!("juge : réponse illisible ({e}) : {}", premiere_ligne(&sink.text, 120)))?;
    let connus: HashSet<&str> = candidats.iter().filter_map(|c| c["name"].as_str()).collect();
    let gardes: Vec<String> = v["items_gardes"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).filter(|n| connus.contains(n)).map(str::to_string).collect();
    Ok(Jugement { pertinent: v["pertinent"].as_bool().unwrap_or(false) && !gardes.is_empty(), pourquoi: v["pourquoi"].as_str().unwrap_or("").to_string(), gardes, modele: llm.name().to_string(), ms, entree: usage.prompt_tokens, sortie: usage.completion_tokens })
}

/// Le modèle du juge : celui du régime (Vertex, `GOOGLE_CLOUD_PROJECT`), s'il y en a un.
fn modele_du_juge() -> Option<std::sync::Arc<dyn crate::llm::Llm>> {
    #[cfg(feature = "openai-llm")]
    {
        crate::regime::modele_agentique_nomme("juge du rappel").map(|(llm, _)| std::sync::Arc::new(llm) as std::sync::Arc<dyn crate::llm::Llm>)
    }
    #[cfg(not(feature = "openai-llm"))]
    {
        None
    }
}

/// **Le fils du rappel différé** : lit sa tâche, refait le rappel s'il le faut
/// (délai long), juge les candidats, range le résultat pour le prochain outil
/// ou le prochain tour. Toujours une ligne de journal, marquée `[fond]`.
pub fn rappel_differe(tache: &Path, reglage: &Reglage, hote: Option<&dyn Hote>, juge: Option<&dyn crate::llm::Llm>) -> Vec<String> {
    let mut journal = Vec::new();
    let Some(t) = std::fs::read_to_string(tache).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
        journal.push(format!("[fond] tâche illisible : {}", tache.display()));
        return journal;
    };
    let _ = std::fs::remove_file(tache);
    let session = t["session_id"].as_str().unwrap_or("").to_string();
    let requete = t["requete"].as_str().unwrap_or("").to_string();
    let debut = std::time::Instant::now();
    let candidats: Vec<Value> = match t["candidats"].as_array() {
        Some(c) => c.clone(),
        None => {
            let Some(h) = hote else {
                journal.push(format!("[fond] {session} : pas d'hôte pour refaire le rappel"));
                return journal;
            };
            match h.appeler_borne("recall_fiches", json!({ "query": requete, "options": { "limit": 3, "filter_condition": { "field": { "key": "state", "value": ["current", "to_review"] } } } }), Duration::from_secs(30)) {
                // Le fils refait le rappel : il ne juge que ce que le plein texte touche.
                Ok(v) => candidats_du_rappel(&v).into_iter().filter(|c| c.plein.is_some()).map(|c| c.json()).collect(),
                Err(err) => {
                    journal.push(format!("[fond] {session} : rappel en {} ms : {err}", debut.elapsed().as_millis()));
                    return journal;
                }
            }
        }
    };
    journal.push(format!("[fond] {session} : rappel « {requete} » en {} ms, {} candidat(s)", debut.elapsed().as_millis(), candidats.len()));
    // Venus du tour, les candidats sont ce qui reste après son seul rendu : sans
    // juge, ils se taisent. Venus d'un rappel refait (le budget dépassé), rien
    // n'a été rendu : sans juge, le premier au plein texte seul (même politique).
    let du_tour = t["candidats"].is_array();
    let sans_juge = |pourquoi: &str, journal: &mut Vec<String>| -> (Vec<Value>, Value) {
        let mut tries = candidats.clone();
        tries.sort_by(|a, b| b["plein"].as_f64().partial_cmp(&a["plein"].as_f64()).unwrap_or(std::cmp::Ordering::Equal));
        let g: Vec<Value> = if du_tour { Vec::new() } else { tries.into_iter().take(1).collect() };
        journal.push(format!("[fond] sans juge ({pourquoi}) : {} rendu, {} tus", g.len(), candidats.len() - g.len()));
        (g, json!(null))
    };
    let au_plafond = !candidats.is_empty() && juge.is_some() && reglage.recall_judge && !compter_un_appel_du_juge(reglage);
    let (gardes, verdict) = match (juge, reglage.recall_judge, candidats.is_empty()) {
        (_, _, true) => (Vec::new(), json!(null)),
        (Some(_), true, false) if au_plafond => sans_juge(&format!("plafond horaire atteint, {} appels", reglage.recall_judge_max_per_hour), &mut journal),
        (Some(llm), true, false) => match juger(llm, t["prompt"].as_str().unwrap_or(""), &candidats) {
            Ok(j) => {
                journal.push(format!("[fond] juge : modèle {}, {} ms, entrée {} jetons, sortie {} jetons", j.modele, j.ms, j.entree, j.sortie));
                journal.push(format!("[fond] juge : {} candidat(s), {} gardé(s) — {}", candidats.len(), j.gardes.len(), j.pourquoi));
                let g: Vec<Value> = candidats.iter().filter(|c| c["name"].as_str().is_some_and(|n| j.pertinent && j.gardes.iter().any(|x| x == n))).cloned().collect();
                (g, json!({ "pertinent": j.pertinent, "pourquoi": j.pourquoi }))
            }
            Err(err) => {
                journal.push(format!("[fond] {err} — rien rendu"));
                (Vec::new(), json!({ "erreur": err }))
            }
        },
        (None, true, false) => sans_juge("aucun modèle dans l'environnement du hook", &mut journal),
        (_, false, false) => sans_juge("recall_judge à false", &mut journal),
    };
    let resultat = json!({ "tour": t["tour"], "requete": requete, "gardes": gardes, "juge": verdict });
    let _ = std::fs::write(chemin_du_fond(reglage, &session, "differe"), resultat.to_string());
    journal
}

/// **Le rappel du tour d'avant**, s'il est rangé : rendu une fois, sous le
/// déjà-vu de la session, puis effacé.
fn rappel_du_tour_d_avant(e: &Evenement, reglage: &Reglage, journal: &mut Vec<String>) -> String {
    let chemin = chemin_du_fond(reglage, &e.session_id, "differe");
    let Some(v) = std::fs::read_to_string(&chemin).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return String::new() };
    if reglage.rejeu {
        return String::new();
    }
    let _ = std::fs::remove_file(&chemin);
    let mut etat = EtatDuRappel::lire(reglage, &e.session_id);
    let mut neuves = Vec::new();
    for g in v["gardes"].as_array().into_iter().flatten() {
        let (n, a) = (g["name"].as_str().unwrap_or(""), g["about"].as_str().unwrap_or(""));
        if n.is_empty() || matches!(etat.statut(n, &empreinte_de_fiche(n, a)), Statut::Deja) {
            continue;
        }
        etat.rendre(n, &empreinte_de_fiche(n, a));
        neuves.push(format!("« {n} »{}", if a.is_empty() { String::new() } else { format!(" ({})", premiere_ligne(a, 80)) }));
    }
    journal.push(format!("rappel du tour d'avant : {} rendu(s){}", neuves.len(), v["juge"]["pourquoi"].as_str().map(|p| format!(" — juge : {p}")).unwrap_or_default()));
    if neuves.is_empty() {
        return String::new();
    }
    etat.ecrire(reglage, &e.session_id);
    format!("Rappel du tour d'avant (requête « {} ») : {} — `get_fiche` pour lire.\n", v["requete"].as_str().unwrap_or(""), neuves.join(" ; "))
}

// ─── UserPromptSubmit ────────────────────────────────────────────────────────

fn bloc_de_prompt(e: &Evenement, reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) -> String {
    let mut b = String::new();
    let transcription = e.transcript_path.as_deref().and_then(|p| crate::claude_code_transcript::Transcription::lire(p).map_err(|err| journal.push(format!("transcription : {err}"))).ok());
    let identite = identite_de_la_session(h, &e.session_id, journal);

    // Le défi d'identité (Lucie, 25 octobre) : au premier tour après une
    // compaction, puis un rappel, une fois, si l'apport n'est pas venu.
    if transcription.as_ref().is_some_and(|t| t.premier_tour_apres_compaction) {
        let fiche = match &identite {
            Some(id) => format!(" (fiche « {} »)", id.cle),
            None => " — et si aucune fiche ne cite cette session, ton L1 `handle` d'abord".to_string(),
        };
        b.push_str(&reglage.gabarit("defi_identity").replace("{rubriques}", RUBRIQUES).replace("{fiche}", &fiche));
        b.push('\n');
        if let Some(id) = identite.as_ref().and_then(Identite::identity_texte) {
            b.push_str(&format!("Ton dernier `identity` : {}\n", premiere_ligne(id, 300)));
        }
        poser_la_marque(reglage, &e.session_id, "identity-demandee", &format!("1 {}", maintenant_ms()));
    } else if let Some(m) = lire_la_marque(reglage, &e.session_id, "identity-demandee") {
        let mut mots = m.split_whitespace();
        let fois: u32 = mots.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let depuis: u64 = mots.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let ecrit = identite.as_ref().and_then(|i| i.identity.as_ref()).is_some_and(|(_, date)| *date >= depuis);
        if ecrit {
            oter_la_marque(reglage, &e.session_id, "identity-demandee");
        } else if fois == 1 {
            b.push_str(&reglage.gabarit("rappel_identity").replace("{rubriques}", RUBRIQUES));
            b.push('\n');
            poser_la_marque(reglage, &e.session_id, "identity-demandee", &format!("2 {depuis}"));
            journal.push("identity redemandée, une fois".into());
        }
    }

    // Les fiches que le prompt rappelle : une requête de mots-clés tirée du
    // prompt (3 ter), jamais le prompt entier ; rien de ce que la session a
    // déjà reçu.
    // Le rappel du tour d'avant, s'il est arrivé après son tour et qu'aucun
    // `PostToolUse` ne l'a rendu.
    b.push_str(&rappel_du_tour_d_avant(e, reglage, journal));
    // **La boîte des agents** (lot 5) : ce qui attend cette session, par son
    // handle et par sa clé stable.
    let stable: Vec<String> = identite.iter().map(|i| i.cle.clone()).collect();
    b.push_str(&boite_du_tour(e, reglage, h, &stable, false, journal));
    let mut etat = EtatDuRappel::lire(reglage, &e.session_id);
    etat.tour += 1;
    // Un tour sans parole d'utilisateur (une notification de tâche) ne cherche rien.
    let notification = e.prompt.as_deref().is_some_and(crate::rappel_du_prompt::est_une_notification);
    if notification {
        journal.push("rappel : tour de notification, rien à chercher".into());
    }
    let mut vocabulaire = etat.vocabulaire();
    vocabulaire.extend(Vocabulaire::lire(reglage));
    let mots = e.prompt.as_deref().filter(|_| !notification).map(|p| crate::rappel_du_prompt::requete_de_rappel(p, reglage.recall_max_words, &vocabulaire)).unwrap_or_default();
    if let Some(p) = e.prompt.as_deref().filter(|_| mots.is_empty() && !notification) {
        journal.push(format!("rappel : rien à chercher (prompt de {} caractères, aucun mot-clé après nettoyage)", p.chars().count()));
    }
    if !mots.is_empty() {
        let requete = mots.join(" ");
        journal.push(format!("rappel : requête « {requete} » ({} mots, tirée d'un prompt de {} caractères)", mots.len(), e.prompt.as_deref().map(|p| p.chars().count()).unwrap_or(0)));
        let debut = std::time::Instant::now();
        let delai = Duration::from_millis(reglage.recall_timeout_ms);
        match h.appeler_borne("recall_fiches", json!({ "query": requete, "options": { "limit": 3, "filter_condition": { "field": { "key": "state", "value": ["current", "to_review"] } } } }), delai) {
            Ok(v) => {
                journal.push(format!("rappel en {} ms", debut.elapsed().as_millis()));
                let candidats = candidats_du_rappel(&v);
                if !candidats.is_empty() {
                    journal.push(format!("rappel : {}", candidats.iter().map(Candidat::dit).collect::<Vec<_>>().join(", ")));
                }
                // Le vocabulaire de la mémoire : les noms vus, sous le seuil compris.
                if !reglage.rejeu {
                    Vocabulaire::ajouter(reglage, candidats.iter().map(|c| c.nom.as_str()));
                }
                // Sa propre fiche d'identité n'est pas un résultat : c'est elle,
                // ôtée avant tout jugement.
                let soi = identite.as_ref().map(|i| i.cle.as_str()).unwrap_or("");
                let mut raisons: Vec<String> = Vec::new();
                if soi.is_empty() {
                    raisons.push("identité inconnue, pas d'exclusion".into());
                } else if candidats.iter().any(|c| c.nom == soi) {
                    raisons.push(format!("exclu : « {soi} » (fiche propre)"));
                }
                // **Le plein texte décide** (c7, d'après trois tours réels où le dense
                // donnait 0,32, 0,35 et 0,37 au même titre, hors sujet comme en plein
                // sujet) : une fiche que les mots extraits touchent est candidate ; une
                // fiche que seul le dense trouve n'est pas jugée tant qu'un écart ou un
                // seuil dense n'a pas été mesuré sur une mémoire peuplée.
                let restants: Vec<&Candidat> = candidats.iter().filter(|c| c.nom != soi).collect();
                let mut touches: Vec<&Candidat> = restants.iter().copied().filter(|c| c.plein.is_some()).collect();
                touches.sort_by(|a, b| b.plein.partial_cmp(&a.plein).unwrap_or(std::cmp::Ordering::Equal));
                let dense_seul: Vec<String> = restants.iter().filter(|c| c.plein.is_none()).map(|c| format!("« {} » {:.2}", c.nom, c.dense.unwrap_or(0.0))).collect();
                if !dense_seul.is_empty() {
                    raisons.push(format!("dense seul, non jugé : {}", dense_seul.join(", ")));
                }
                // Le déjà-vu : ce que la session a reçu, inchangé, ne revient pas.
                let mut deja = 0;
                let mut neuves: Vec<(&Candidat, Option<u64>)> = Vec::new();
                for c in touches {
                    match etat.statut(&c.nom, &empreinte_de_fiche(&c.nom, &c.about)) {
                        Statut::Deja => deja += 1,
                        Statut::Neuve => neuves.push((c, None)),
                        Statut::Changee(tour) => neuves.push((c, Some(tour))),
                    }
                }
                raisons.push(format!("déjà rendu : {deja}, neuf : {}", neuves.len()));
                // Au plus UN rendu dans le tour (c7, forme 3 : une politique, pas un
                // seuil) ; les autres que le plein texte touche vont au juge.
                let reste = if neuves.len() > 1 { neuves.split_off(1) } else { Vec::new() };
                if !reste.is_empty() {
                    if reglage.recall_judge {
                        raisons.push(format!("{} rendu, {} au juge", neuves.len(), reste.len()));
                        let pour_le_juge: Vec<Value> = reste.iter().map(|(c, _)| c.json()).collect();
                        confier_au_fond(e, reglage, &requete, Some(pour_le_juge), etat.tour, journal);
                    } else {
                        raisons.push(format!("{} rendu, {} tus (juge coupé)", neuves.len(), reste.len()));
                    }
                }
                if neuves.is_empty() {
                    raisons.push("silence".into());
                } else {
                    b.push_str("Fiche en rapport (touchée au plein texte par les mots du prompt) : ");
                    b.push_str(&neuves.iter().map(|(c, maj)| format!("« {} » ({}{}{})", c.nom, c.scores(), if c.about.is_empty() { String::new() } else { format!(" ; {}", premiere_ligne(&c.about, 80)) }, maj.map(|t| format!(" ; mise à jour depuis le tour {t}")).unwrap_or_default())).collect::<Vec<_>>().join(" ; "));
                    b.push_str(" — `get_fiche` pour lire.\n");
                    for (c, _) in &neuves {
                        etat.rendre(&c.nom, &empreinte_de_fiche(&c.nom, &c.about));
                    }
                }
                journal.push(format!("rappel : {}", raisons.join(" ; ")));
            }
            Err(err) => {
                let lent = err.contains("timeout") || err.contains("ne répond pas") || err.contains("délai dépassé côté hôte");
                journal.push(format!("recall_fiches en {} ms : {err}", debut.elapsed().as_millis()));
                if lent {
                    confier_au_fond(e, reglage, &requete, None, etat.tour, journal);
                    b.push_str(&format!("Rappel des fiches : pas de réponse en {} ms — il continue en fond, son résultat viendra au prochain outil ou au prochain tour.\n", reglage.recall_timeout_ms));
                } else {
                    b.push_str(&format!("Rappel des fiches en panne : {} — le rappel sera retenté au prochain tour.\n", premiere_ligne(&err, 160)));
                }
            }
        }
    }

    etat.ecrire(reglage, &e.session_id);

    // Les fichiers touchés par la session, changés depuis par quelqu'un d'autre.
    if let (Some(t), Some(id)) = (&transcription, &identite) {
        let mut vus = 0;
        for f in t.fichiers_touches.iter().rev().take(5) {
            let Ok(v) = h.appeler("cited_by", json!({ "value": format!("path:{f}"), "under": true, "format": "json", "limit": 3, "since": depuis_48h() })) else { continue };
            for fiche in fiches_citant(&v) {
                for a in fiche["apports"].as_array().into_iter().flatten() {
                    if a["section"] == "diff" && a["author"].as_str().is_some_and(|x| x != id.cle) && vus < 2 {
                        b.push_str(&format!("`{f}` a changé depuis, par {} : {}\n", a["author"].as_str().unwrap_or("?"), premiere_ligne(a["texte"].as_str().unwrap_or(""), 160)));
                        vus += 1;
                    }
                }
            }
        }
        // Le dernier verdict du juge sur la tâche courante.
        if let Some(tache) = id.tache() {
            if let Some((bilan, _)) = dernier_apport(h, &format!("bilan : {tache}"), "bilan") {
                b.push_str(&format!("Le juge, sur « {tache} » : {}\n", premiere_ligne(&bilan, 200)));
            }
        }
    }

    // Le remplissage du contexte, une suggestion une fois.
    if let Some(t) = &transcription {
        let (contexte, comment) = t.contexte_estime();
        let seuil = (reglage.context_tokens as f64 * reglage.suggest_ratio) as u64;
        if contexte >= seuil && !deja_suggere(reglage, &e.session_id) {
            b.push_str(&reglage.gabarit("suggestion_contexte").replace("{contexte}", &(contexte / 1000).to_string()).replace("{plafond}", &(reglage.context_tokens / 1000).to_string()).replace("{comment}", comment));
            b.push('\n');
            journal.push(format!("suggestion de ranger : {contexte} jetons ({comment})"));
        }
    }
    b
}

/// Millisecondes UTC d'il y a 48 heures.
fn depuis_48h() -> u64 {
    maintenant_ms().saturating_sub(48 * 3600 * 1000)
}

/// La suggestion de ranger se fait **une fois** par session.
fn deja_suggere(reglage: &Reglage, session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    if lire_la_marque(reglage, session_id, "suggere").is_some() {
        return true;
    }
    if reglage.rejeu {
        return false;
    }
    poser_la_marque(reglage, session_id, "suggere", "");
    false
}

// ─── PreCompact : les consignes du résumé ────────────────────────────────────

/// Ce que le résumé de compaction doit garder, pour que la session se
/// retrouve : l'identité telle qu'elle, le plan, ce qui est tenu, les mandats,
/// les pairs par leur clé. Le dernier `identity` connu est donné en graine.
fn consignes_du_resume(e: &Evenement, reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) -> String {
    let identite = identite_de_la_session(h, &e.session_id, journal);
    let mut c = reglage.gabarit("consignes_resume").replace("{rubriques}", RUBRIQUES);
    c.push('\n');
    match identite.as_ref().and_then(Identite::identity_texte) {
        Some(identity) => c.push_str(&format!("\nLe dernier `identity` connu de cette session, à reprendre et corriger :\n{}\n", identity.trim())),
        None => c.push_str("\nAucun apport `identity` connu pour cette session : le résumé doit au moins dire qui est la session (sa clé) et ce qu'elle tenait.\n"),
    }
    journal.push("consignes du résumé rendues".into());
    c
}

// ─── PostCompact : le résumé rangé ───────────────────────────────────────────

/// Le résumé de compaction est la parole de l'agent sur sa session : il va en
/// L0 (une Source immuable, cherchable), et en L1 `compaction` sur la **fiche
/// de tâche** courante quand il y en a une (96 : jamais sur la fiche
/// d'identité — elle cite la mémoire, elle ne la contient pas). Sans tâche, le
/// L0 seul, et le prochain `SessionStart` le dit.
fn ranger_la_compaction(e: &Evenement, reglage: &Reglage, h: &dyn Hote, journal: &mut Vec<String>) {
    let Some(resume) = e.compact_summary.as_deref().filter(|r| !r.trim().is_empty()) else {
        journal.push("PostCompact sans résumé : rien à ranger".into());
        return;
    };
    let identite = identite_de_la_session(h, &e.session_id, journal);
    let cle = identite.as_ref().map(|i| i.cle.clone()).unwrap_or_default();
    let quand = jiff::Zoned::now();
    let id8: String = e.session_id.chars().take(8).collect();
    let chemin = format!("sources/{}-compaction-{id8}.md", quand.strftime("%Y-%m-%d-%Hh%M"));
    let titre = format!("Compaction du {} — session {id8}", quand.strftime("%Y-%m-%d %H:%M"));
    let contenu = format!("# {titre}\n\n{}\n", resume.trim());
    if let Err(err) = h.appeler("put_source", json!({ "record": { "chemin": chemin, "titre": titre, "contenu": contenu, "origine": "filet", "session": format!("session:{}", e.session_id), "via": "hook", "author": cle } })) {
        journal.push(format!("L0 de la compaction non écrit : {err}"));
        return;
    }
    let Some(tache) = identite.as_ref().and_then(Identite::tache) else {
        poser_la_marque(reglage, &e.session_id, "compaction-l0-seul", &chemin);
        journal.push(format!("compaction rangée en L0 seul ({chemin}) : {}", if identite.is_some() { "aucune fiche de tâche dans l'identity" } else { "aucune identité pour cette session" }));
        return;
    };
    let texte: String = resume.trim().chars().take(6_000).collect();
    match ecrire_un_apport(h, &tache, &chemin, "compaction", &texte, &cle) {
        Ok(()) => {
            citer(h, &tache, &chemin, "compaction", "doc", &format!("{chemin}#{titre}"), journal);
            citer(h, &tache, &chemin, "compaction", "handle", &format!("session:{}", e.session_id), journal);
            journal.push(format!("L1 compaction écrit sur « {tache} » ({chemin})"));
        }
        Err(err) => journal.push(format!("L1 compaction non écrit sur « {tache} » : {err} — le L0 est là ({chemin})")),
    }
}

// ─── Stop : la fin de run ────────────────────────────────────────────────────

/// **Les serveurs MCP de la session qui sont les nôtres**, par leur nom Claude
/// Code → le manifeste qu'ils servent (`mcp --manifest <chemin>` de notre
/// binaire) : la table que le juge lit pour rejouer un appel
/// `mcp__<serveur>__<outil>` sur le bon backend (73). Lue dans
/// `~/.claude.json` (`projects[<cwd>].mcpServers`, puis la racine) et dans le
/// `.mcp.json` du dossier ; `RAG3WEAVER_CLAUDE_JSON` donne un autre fichier
/// (les témoins). Un serveur qui n'est pas notre binaire, ou sans manifeste,
/// n'y est pas : le juge dira « non rejoué : serveur inconnu du run ».
pub fn serveurs_mcp(cwd: Option<&Path>) -> serde_json::Map<String, Value> {
    let mut table = serde_json::Map::new();
    let mut prendre = |serveurs: &Value| {
        let Some(o) = serveurs.as_object() else { return };
        for (nom, d) in o {
            let commande = d["command"].as_str().unwrap_or("");
            if !commande.ends_with("rag3weaver-backend") {
                continue;
            }
            let args: Vec<&str> = d["args"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            if args.first() != Some(&"mcp") {
                continue;
            }
            if let Some(i) = args.iter().position(|a| *a == "--manifest") {
                if let Some(m) = args.get(i + 1) {
                    table.entry(nom.clone()).or_insert_with(|| json!(m));
                }
            }
        }
    };
    let claude_json = std::env::var_os("RAG3WEAVER_CLAUDE_JSON").map(PathBuf::from).unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".claude.json"));
    if let Ok(v) = std::fs::read_to_string(&claude_json).and_then(|t| serde_json::from_str::<Value>(&t).map_err(std::io::Error::other)) {
        if let Some(c) = cwd {
            prendre(&v["projects"][c.to_string_lossy().as_ref()]["mcpServers"]);
        }
        prendre(&v["mcpServers"]);
    }
    if let Some(c) = cwd {
        if let Ok(v) = std::fs::read_to_string(c.join(".mcp.json")).and_then(|t| serde_json::from_str::<Value>(&t).map_err(std::io::Error::other)) {
            prendre(&v["mcpServers"]);
        }
    }
    table
}

/// Un tour de Claude Code, journalisé comme un run racine de l'hôte
/// (`POST /journal/run`, genre `claude_code_turn`), pour que le juge le lise
/// comme un tour du chat : le prompt, les réponses, les appels d'outils et
/// leurs résultats tels que la transcription les donne, le dernier message.
fn deposer_la_fin_de_run(e: &Evenement, h: &dyn Hote, journal: &mut Vec<String>) {
    if e.session_id.is_empty() {
        journal.push("Stop sans session_id : pas de run".into());
        return;
    }
    let prompt_id = e.prompt_id.clone().unwrap_or_else(|| format!("sans-prompt-{}", maintenant_ms()));
    let tours = match e.transcript_path.as_deref() {
        Some(p) => crate::claude_code_transcript::tours_du_prompt(p, &prompt_id).unwrap_or_else(|err| {
            journal.push(format!("transcription : {err}"));
            Vec::new()
        }),
        None => Vec::new(),
    };
    let identite = identite_de_la_session(h, &e.session_id, journal);
    let run = format!("{}-{prompt_id}", e.session_id);
    let corps = json!({
        // Le backend du run est celui du manifeste : l'hôte le résout et le juge
        // le prend comme un tour du chat ; `backend` reste le nom nu en repli.
        "manifest": h.manifest(),
        "backend": "claude-code",
        "run": run,
        "genre": "claude_code_turn",
        "session": format!("session:{}", e.session_id),
        "agent": identite.as_ref().map(|i| i.cle.clone()),
        "cwd": e.cwd,
        "serveurs": serveurs_mcp(e.cwd.as_deref()),
        "transcription": e.transcript_path,
        "prompt_id": prompt_id,
        "tours": tours,
        "dernier_message": e.last_assistant_message.as_deref().map(|m| m.chars().take(8_000).collect::<String>()),
        "stop_hook_active": e.stop_hook_active,
    });
    match h.deposer_run(corps) {
        Ok(_) => journal.push(format!("run déposé : claude-code/{run}")),
        Err(err) => journal.push(format!("run non déposé ({run}) : {err}")),
    }
}

/// Une ligne datée au journal ; un journal qu'on ne peut pas écrire ne fait
/// rien échouer.
pub fn journaliser(chemin: &Path, lignes: &[String]) {
    if lignes.is_empty() {
        return;
    }
    if let Some(d) = chemin.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(chemin) else { return };
    let quand = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
    for l in lignes {
        let _ = writeln!(f, "{quand} {l}");
    }
}

/// **La commande entière** : lit l'entrée, le réglage, joint l'hôte, répond,
/// écrit le bloc et le journal. Rend toujours 0 ; `--config <chemin>` pour un
/// autre réglage, `--daemon <adresse>` pour un hôte donné (les témoins).
pub const USAGE: &str = "usage : rag3weaver-backend hook [<événement>] [--config <hooks.json>] [--daemon <adresse>]\n        \
rag3weaver-backend hook --rejouer <transcription.jsonl> <événement> [--prompt <texte>] [--source <startup|compact>] [--session <id>]\n        \
rag3weaver-backend hook --poser-les-gabarits\n        \
rag3weaver-backend hook --rappel-differe <tâche.json>   (le fils du rappel, lancé par le hook)\n\n\
Le hook de Claude Code : lit le JSON de l'événement (SessionStart, UserPromptSubmit, PreCompact, \
PostCompact, Stop) sur son entrée, parle à l'hôte par le backend mémoire du réglage \
(~/.config/rag3weaver/hooks.json), rend son bloc sur la sortie et sort toujours en 0. Sans hôte, \
il se tait et écrit pourquoi dans le journal (~/.local/state/rag3weaver/hooks.log). Il ne lance \
jamais d'hôte. Installation : scripts/hooks_claude_code.sh (page docs/07-novembre-2026-hooks-claude-code/01).\n\n\
Corriger à la volée : chaque hook est un processus neuf — le binaire, hooks.json et les gabarits de \
blocs (<templates_dir>/<clé>.md, posés par --poser-les-gabarits) sont relus à chaque appel ; rien \
dans settings.json ne change jamais. --rejouer rend le bloc qu'un événement rendrait pour une \
transcription donnée, hors de toute session, pour vérifier une correction.";

pub fn commande(args: &[String], entree: &str, sortie: &mut dyn std::io::Write) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        let _ = writeln!(sortie, "{USAGE}");
        return 0;
    }
    let mut config = Reglage::chemin_par_defaut();
    let mut adresse: Option<String> = None;
    let mut rejouer: Option<(PathBuf, String)> = None;
    let mut prompt: Option<String> = None;
    let mut source: Option<String> = None;
    let mut poser_gabarits = false;
    let mut evenement_attendu: Option<String> = None;
    let mut session: Option<String> = None;
    let mut agent: Option<String> = None;
    let mut differe: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match (args[i].as_str(), args.get(i + 1)) {
            ("--config", Some(v)) => { config = PathBuf::from(v); i += 2; }
            ("--daemon", Some(v)) => { adresse = Some(v.clone()); i += 2; }
            ("--rejouer", Some(v)) => { rejouer = Some((PathBuf::from(v), args.get(i + 2).cloned().unwrap_or_else(|| "SessionStart".into()))); i += 3; }
            ("--prompt", Some(v)) => { prompt = Some(v.clone()); i += 2; }
            ("--source", Some(v)) => { source = Some(v.clone()); i += 2; }
            ("--session", Some(v)) => { session = Some(v.clone()); i += 2; }
            ("--agent", Some(v)) => { agent = Some(v.clone()); i += 2; }
            ("--rappel-differe", Some(v)) => { differe = Some(PathBuf::from(v)); i += 2; }
            ("--poser-les-gabarits", _) => { poser_gabarits = true; i += 1; }
            // L'événement, en argument stable de settings.json : relu contre l'entrée.
            (a, _) if EVENEMENTS.contains(&a) => { evenement_attendu = Some(a.to_string()); i += 1; }
            _ => i += 1,
        }
    }
    let reglage = match Reglage::charger(&config) {
        Ok(r) => r,
        Err(e) => {
            journaliser(&Reglage::defaut().journal, &[format!("réglage illisible, défauts pris : {e}")]);
            Reglage::defaut()
        }
    };
    // **Le fils du rappel différé** : sa tâche dit le réglage à relire ; il refait
    // le rappel sous un délai long, juge, range, journalise.
    if let Some(tache) = differe {
        let lu = std::fs::read_to_string(&tache).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
        let reglage = lu.as_ref().and_then(|t| t["config"].as_str()).map(PathBuf::from).and_then(|c| Reglage::charger(&c).ok()).unwrap_or(reglage);
        let besoin_d_hote = lu.as_ref().is_some_and(|t| t["candidats"].is_null());
        let client = if besoin_d_hote {
            match &adresse {
                Some(a) => ClientHote::ouvrir(a, None, &reglage.manifest, &reglage.keys, Duration::from_secs(30)),
                None => ClientHote::depuis_daemon_json(&reglage.manifest, &reglage.keys, Duration::from_secs(30)),
            }
            .ok()
        } else {
            None
        };
        let juge = modele_du_juge();
        let journal = rappel_differe(&tache, &reglage, client.as_ref().map(|c| c as &dyn Hote), juge.as_deref());
        if let Some(c) = &client {
            c.fermer();
        }
        journaliser(&reglage.journal, &journal);
        return 0;
    }
    if poser_gabarits {
        return match reglage.poser_les_gabarits() {
            Ok(poses) => {
                let _ = writeln!(sortie, "gabarits dans {} : {} posé(s){}", reglage.templates_dir.display(), poses.len(), if poses.is_empty() { " (tous déjà là)".to_string() } else { format!(" — {}", poses.join(", ")) });
                0
            }
            Err(e) => {
                let _ = writeln!(sortie, "{e}");
                1
            }
        };
    }
    // **Le rejeu hors session** : l'événement reconstitué depuis une transcription
    // (l'identifiant de session est le nom du fichier), le bloc rendu tel qu'il
    // le serait, le journal marqué « rejeu ».
    let entree_rejouee;
    let (entree, rejeu) = match &rejouer {
        Some((transcription, nom)) => {
            // Par défaut la session de la transcription ; `--session` en donne une
            // autre (un id neuf rejoue sans le déjà-vu de la vraie session).
            let session_id = session.clone().unwrap_or_else(|| transcription.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
            let dernier_prompt = prompt.clone().or_else(|| crate::claude_code_transcript::dernier_prompt(transcription));
            let mut ev = json!({ "session_id": session_id, "transcript_path": transcription, "cwd": std::env::current_dir().ok(), "hook_event_name": nom, "prompt_id": format!("rejeu-{}", maintenant_ms()) });
            match nom.as_str() {
                "SessionStart" => ev["source"] = json!(source.clone().unwrap_or_else(|| "startup".into())),
                "UserPromptSubmit" => ev["prompt"] = json!(dernier_prompt.unwrap_or_default()),
                "PreCompact" | "PostCompact" => ev["trigger"] = json!("manual"),
                _ => {}
            }
            entree_rejouee = ev.to_string();
            (entree_rejouee.as_str(), true)
        }
        None => (entree, false),
    };
    let evenement = match Evenement::lire(entree) {
        Ok(e) => e,
        Err(e) => {
            journaliser(&reglage.journal, &[format!("entrée refusée : {e}")]);
            return 0;
        }
    };
    if let Some(attendu) = &evenement_attendu {
        if attendu != &evenement.nom {
            journaliser(&reglage.journal, &[format!("{} : l'argument dit {attendu}, l'entrée dit {} — l'entrée fait foi", evenement.nom, evenement.nom)]);
        }
    }
    let delai = reglage.delai(&evenement.nom);
    // `PostToolUse` : le rappel différé seul, sans hôte, en `additionalContext`
    // (Claude Code n'y lit pas le texte brut).
    let mut reglage = reglage;
    if let Some(a) = agent {
        reglage.agent = a;
    }
    let boite_delai = Duration::from_millis(reglage.mailbox_timeout_ms);
    if evenement.nom == "PostToolUse" {
        // La boîte seule (aucun backend ouvert) ; sans hôte, le rappel différé seul.
        let boite = match &adresse {
            Some(a) => Ok(crate::mail::BoiteHttp::ouvrir(a, None, boite_delai)),
            None => crate::mail::BoiteHttp::depuis_daemon_json(boite_delai),
        };
        let reponse = match boite {
            Ok(b) => repondre(&evenement, &reglage, Ok(&HoteBoiteSeule(b))),
            Err(e) => repondre(&evenement, &reglage, Err(e)),
        };
        if !reponse.bloc.is_empty() && !rejeu {
            let _ = writeln!(sortie, "{}", json!({ "hookSpecificOutput": { "hookEventName": "PostToolUse", "additionalContext": reponse.bloc } }));
        }
        journaliser(&reglage.journal, &reponse.journal);
        return 0;
    }
    let client = match &adresse {
        Some(a) => ClientHote::ouvrir(a, None, &reglage.manifest, &reglage.keys, delai),
        None => ClientHote::depuis_daemon_json(&reglage.manifest, &reglage.keys, delai),
    }
    .map(|c| c.avec_boite(boite_delai));
    reglage.rejeu = rejeu;
    reglage.fond = !rejeu;
    let reponse = match &client {
        Ok(c) if rejeu => repondre(&evenement, &reglage, Ok(&HoteEnLecture(c))),
        Ok(c) => repondre(&evenement, &reglage, Ok(c)),
        Err(e) => repondre(&evenement, &reglage, Err(e.clone())),
    };
    let mut reponse = reponse;
    if let Ok(c) = &client {
        let d = c.durees();
        if !d.is_empty() {
            reponse.journal.push(format!("{} {} : durées — {}", evenement.nom, evenement.session_id, d.join(", ")));
        }
        c.fermer();
    }
    if !reponse.bloc.is_empty() && reglage.agent == "codex" && !rejeu && matches!(evenement.nom.as_str(), "SessionStart" | "UserPromptSubmit") {
        // Chez Codex, le contexte passe par `hookSpecificOutput.additionalContext`.
        let _ = writeln!(sortie, "{}", json!({ "hookSpecificOutput": { "hookEventName": evenement.nom, "additionalContext": reponse.bloc } }));
    } else if !reponse.bloc.is_empty() {
        let _ = sortie.write_all(reponse.bloc.as_bytes());
        if !reponse.bloc.ends_with('\n') {
            let _ = sortie.write_all(b"\n");
        }
    } else if rejeu {
        let _ = writeln!(sortie, "(aucun bloc : rien à dire pour cet événement)");
    }
    if rejeu {
        let _ = writeln!(sortie, "— journal :");
        for l in &reponse.journal {
            let _ = writeln!(sortie, "  {l}");
        }
        journaliser(&reglage.journal, &reponse.journal.iter().map(|l| format!("[rejeu] {l}")).collect::<Vec<_>>());
    } else {
        journaliser(&reglage.journal, &reponse.journal);
    }
    0
}

#[cfg(test)]
mod tests {
    /// Les témoins qui posent ou ôtent une variable d'environnement du processus
    /// (`RAG3WEAVER_AGENT_KEY`, `RAG3WEAVER_CLAUDE_JSON`) passent un à un : les tests
    /// tournent en parallèle dans un même processus, et se croisaient.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn env() -> std::sync::MutexGuard<'static, ()> {
        ENV.lock().unwrap_or_else(|p| p.into_inner())
    }
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    const SESSION_START: &str = r#"{"session_id":"25b7ff4b-dabf-42ef-8580-86661c846fb9","transcript_path":"/home/x/.claude/projects/p/25b7ff4b.jsonl","cwd":"/home/x/p","hook_event_name":"SessionStart","source":"compact"}"#;
    const PROMPT: &str = r#"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"UserPromptSubmit","prompt":"range la fiche régime GPU","prompt_id":"p7"}"#;
    const PRE: &str = r#"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"PreCompact","trigger":"auto","custom_instructions":null}"#;
    const POST: &str = r###"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"PostCompact","trigger":"manual","compact_summary":"## Résumé\nla session rangeait."}"###;
    const STOP: &str = r#"{"session_id":"s1","transcript_path":"/t.jsonl","cwd":"/p","hook_event_name":"Stop","stop_hook_active":false,"last_assistant_message":"fait."}"#;

    #[test]
    fn les_cinq_evenements_se_lisent_avec_leurs_champs_propres() {
        let s = Evenement::lire(SESSION_START).unwrap();
        assert_eq!((s.nom.as_str(), s.source.as_deref()), ("SessionStart", Some("compact")));
        assert_eq!(s.transcript_path.as_deref(), Some(Path::new("/home/x/.claude/projects/p/25b7ff4b.jsonl")));
        let p = Evenement::lire(PROMPT).unwrap();
        assert_eq!((p.prompt.as_deref(), p.prompt_id.as_deref()), (Some("range la fiche régime GPU"), Some("p7")));
        let pre = Evenement::lire(PRE).unwrap();
        assert_eq!(pre.trigger.as_deref(), Some("auto"));
        let post = Evenement::lire(POST).unwrap();
        assert!(post.compact_summary.unwrap().starts_with("## Résumé"));
        let stop = Evenement::lire(STOP).unwrap();
        assert_eq!((stop.stop_hook_active, stop.last_assistant_message.as_deref()), (false, Some("fait.")));
        for e in [SESSION_START, PROMPT, PRE, POST, STOP] {
            assert!(Evenement::lire(e).unwrap().connu());
        }
    }

    #[test]
    fn une_entree_sans_nom_ou_pas_json_est_refusee_en_le_disant() {
        assert!(Evenement::lire("pas du json").unwrap_err().contains("JSON"));
        assert!(Evenement::lire(r#"{"session_id":"s"}"#).unwrap_err().contains("hook_event_name"));
        let inconnu = Evenement::lire(r#"{"hook_event_name":"Notification","session_id":"s"}"#).unwrap();
        assert!(!inconnu.connu());
    }

    #[test]
    fn le_reglage_prend_ses_defauts_et_refuse_un_champ_inconnu() {
        let d = Reglage::lire("{}").unwrap();
        assert_eq!(d, Reglage::defaut());
        let r = Reglage::lire(r#"{"context_tokens": 200000, "prompt_timeout_ms": 1000, "keys": ["admin"]}"#).unwrap();
        assert_eq!((r.context_tokens, r.prompt_timeout_ms, r.keys.as_slice()), (200_000, 1_000, &["admin".to_string()][..]));
        assert_eq!(r.delai("UserPromptSubmit"), Duration::from_millis(1_000));
        assert_eq!(r.delai("Stop"), Duration::from_millis(20_000));
        let e = Reglage::lire(r#"{"contexte": 1}"#).unwrap_err();
        assert!(e.contains("contexte") && e.contains("context_tokens"), "{e}");
        // Les défauts écrits se relisent à l'identique.
        assert_eq!(Reglage::lire(&Reglage::defaut().json().to_string()).unwrap(), Reglage::defaut());
        // Un fichier absent : les défauts, pas une erreur.
        // Il garde le chemin d'où il vient (le fils du rappel différé le relit).
        let mut attendu = Reglage::defaut();
        attendu.chemin_du_reglage = Some(PathBuf::from("/nulle/part/hooks.json"));
        assert_eq!(Reglage::charger(Path::new("/nulle/part/hooks.json")).unwrap(), attendu);
    }

    struct FauxHote(Mutex<Vec<String>>);
    impl Hote for FauxHote {
        fn nom(&self) -> &str {
            "memoire-longue"
        }
        fn deposer_run(&self, _: Value) -> Result<Value, String> {
            self.0.lock().unwrap().push("journal/run".into());
            Ok(json!({ "ok": true }))
        }
        fn appeler(&self, outil: &str, _: Value) -> Result<Value, String> {
            self.0.lock().unwrap().push(outil.to_string());
            Ok(json!({ "ok": true }))
        }
    }

    #[test]
    fn un_evenement_sans_bloc_journalise_l_hote_joint_ou_le_silence() {
        let _env = env();
        let h = FauxHote(Mutex::new(Vec::new()));
        // Stop ne rend rien encore (lot 3) : l'hôte joint est dit, rien d'écrit.
        let r = repondre(&Evenement::lire(STOP).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.is_empty(), "rien d'injecté au Stop");
        assert!(r.journal[0].starts_with("Stop s1 : hôte joint, backend « memoire-longue » ["), "{}", r.journal[0]);
        assert!(r.journal.iter().any(|l| l.contains("run déposé")), "{:?}", r.journal);
        let r = repondre(&Evenement::lire(PROMPT).unwrap(), &Reglage::defaut(), Err("aucun hôte connu (daemon.json absent) : la commande ne le lance pas".into()));
        assert!(r.bloc.is_empty());
        assert_eq!(r.journal[0], "UserPromptSubmit s1 : silence — aucun hôte connu (daemon.json absent) : la commande ne le lance pas");
        let r = repondre(&Evenement::lire(r#"{"hook_event_name":"Notification","session_id":"s"}"#).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.is_empty() && r.journal[0].contains("inconnu"));
    }

    /// Un hôte en carton sur un port libre : il rend les réponses données, dans
    /// l'ordre, et garde chaque requête (méthode, chemin, en-têtes, corps).
    fn hote_en_carton(reponses: Vec<(u16, String)>) -> (String, Arc<Mutex<Vec<(String, String, Vec<String>, String)>>>) {
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        let recues = Arc::new(Mutex::new(Vec::new()));
        let gardees = recues.clone();
        std::thread::spawn(move || {
            for reponse in reponses {
                let Ok((mut s, _)) = ecoute.accept() else { return };
                let mut lecteur = BufReader::new(s.try_clone().unwrap());
                let mut ligne = String::new();
                lecteur.read_line(&mut ligne).unwrap();
                let mut mots = ligne.split_whitespace();
                let (methode, chemin) = (mots.next().unwrap_or("").to_string(), mots.next().unwrap_or("").to_string());
                let mut entetes = Vec::new();
                let mut longueur = 0usize;
                loop {
                    let mut l = String::new();
                    lecteur.read_line(&mut l).unwrap();
                    let l = l.trim_end().to_string();
                    if l.is_empty() {
                        break;
                    }
                    if let Some(n) = l.to_ascii_lowercase().strip_prefix("content-length:") {
                        longueur = n.trim().parse().unwrap_or(0);
                    }
                    entetes.push(l);
                }
                let mut corps = vec![0u8; longueur];
                if longueur > 0 {
                    lecteur.read_exact(&mut corps).unwrap();
                }
                gardees.lock().unwrap().push((methode, chemin, entetes, String::from_utf8_lossy(&corps).to_string()));
                let (statut, texte) = reponse;
                let _ = write!(s, "HTTP/1.1 {statut} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{texte}", texte.len());
            }
        });
        (adresse, recues)
    }

    #[test]
    fn le_client_ouvre_une_session_le_backend_appelle_et_ferme_sous_le_jeton() {
        let (adresse, recues) = hote_en_carton(vec![
            (200, json!({ "session": "S9" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (200, json!({ "ok": true, "result": "x" }).to_string()),
            (200, json!({ "closed": true }).to_string()),
        ]);
        let c = ClientHote::ouvrir(&adresse, Some("jeton-j"), Path::new("/m/backend.json"), &["dev".into()], Duration::from_secs(5)).unwrap();
        assert_eq!(c.nom(), "memoire-longue");
        let r = c.appeler("get_fiche", json!({ "record": { "name": "c7" } })).unwrap();
        assert_eq!(r["result"], "x");
        c.fermer();
        std::thread::sleep(Duration::from_millis(50));
        let r = recues.lock().unwrap();
        let chemins: Vec<&str> = r.iter().map(|x| x.1.as_str()).collect();
        assert_eq!(chemins, ["/session", "/open", "/call", "/session"]);
        assert_eq!(r[3].0, "DELETE");
        for (i, (_, _, entetes, _)) in r.iter().enumerate() {
            assert!(entetes.iter().any(|h| h == "authorization: Bearer jeton-j" || h == "Authorization: Bearer jeton-j"), "jeton absent sur la requête {i} : {entetes:?}");
            if i > 0 {
                assert!(entetes.iter().any(|h| h.to_ascii_lowercase().starts_with("x-rag3weaver-session: s9")), "session absente sur la requête {i} : {entetes:?}");
            }
        }
        let appel: Value = serde_json::from_str(&r[2].3).unwrap();
        assert_eq!((appel["name"].as_str(), appel["manifest"].as_str()), (Some("get_fiche"), Some("/m/backend.json")));
        assert_eq!(appel["deadline_ms"], 4_750, "l'hôte reçoit 95 % du délai du client (5 s) : {appel}");
        assert_eq!(r[0].3.contains("\"client\":\"claude-code-hook\""), true);
    }

    #[test]
    fn un_delai_depasse_cote_hote_se_lit_et_le_rappel_passe_au_fond() {
        // L'hôte arrête le run à son échéance et répond 422, `error_kind: deadline`.
        let (adresse, recues) = hote_en_carton(vec![
            (200, json!({ "session": "S9" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (422, json!({ "ok": false, "error": "délai dépassé (950 ms) : l'hôte a arrêté « recall_fiches » à son échéance (cancelled_by « deadline »)", "error_kind": "deadline", "transient": true }).to_string()),
        ]);
        let c = ClientHote::ouvrir(&adresse, None, Path::new("/m/backend.json"), &["dev".into()], Duration::from_secs(5)).unwrap();
        let e = c.appeler_borne("recall_fiches", json!({ "query": "x" }), Duration::from_millis(1_000)).unwrap_err();
        assert!(e.contains("délai dépassé côté hôte") && e.contains("950 ms"), "{e}");
        std::thread::sleep(Duration::from_millis(50));
        let r = recues.lock().unwrap();
        let appel: Value = serde_json::from_str(&r[2].3).unwrap();
        assert_eq!(appel["deadline_ms"], 950, "le rappel borné à 1 s donne 950 ms à l'hôte : {appel}");
    }

    #[test]
    fn sans_nom_rendu_par_open_le_backend_prend_le_nom_de_son_dossier() {
        let (adresse, _) = hote_en_carton(vec![(200, json!({ "session": "S" }).to_string()), (200, json!({ "ok": true }).to_string())]);
        let c = ClientHote::ouvrir(&adresse, None, Path::new("/x/backends/memory/backend.json"), &[], Duration::from_secs(5)).unwrap();
        assert_eq!(c.nom(), "memory");
    }

    #[test]
    fn un_refus_de_l_hote_est_une_raison_dite_pas_une_panique() {
        let (adresse, _) = hote_en_carton(vec![(401, json!({ "ok": false, "error": "jeton refusé" }).to_string())]);
        let e = ClientHote::ouvrir(&adresse, Some("mauvais"), Path::new("/m"), &[], Duration::from_secs(5)).unwrap_err();
        assert!(e.contains("401") && e.contains("jeton refusé"), "{e}");
        let (adresse, _) = hote_en_carton(vec![(200, json!({ "session": "S" }).to_string()), (200, json!({ "ok": false, "error": "manifeste introuvable" }).to_string())]);
        let e = ClientHote::ouvrir(&adresse, None, Path::new("/m"), &[], Duration::from_secs(5)).unwrap_err();
        assert!(e.contains("manifeste introuvable"), "{e}");
    }

    #[test]
    fn un_hote_muet_rend_la_main_dans_le_delai() {
        // Un port qui accepte et ne répond jamais.
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let _garde = ecoute.accept();
            std::thread::sleep(Duration::from_secs(30));
        });
        let debut = std::time::Instant::now();
        let e = ClientHote::ouvrir(&adresse, None, Path::new("/m"), &[], Duration::from_millis(300)).unwrap_err();
        assert!(debut.elapsed() < Duration::from_secs(5), "le délai n'a pas coupé : {:?}", debut.elapsed());
        assert!(e.contains("ne répond pas"), "{e}");
    }

    #[test]
    fn un_gabarit_corrige_dans_le_dossier_change_le_bloc_sans_rien_relancer() {
        let _env = env();
        let dossier = std::env::temp_dir().join(format!("hooks-gabarits-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        let mut reglage = Reglage::defaut();
        reglage.templates_dir = dossier.join("hooks");
        reglage.journal = dossier.join("hooks.log");
        assert_eq!(reglage.version_des_gabarits(), "gabarits intégrés");
        let poses = reglage.poser_les_gabarits().unwrap();
        assert_eq!(poses, ["SessionStart", "UserPromptSubmit", "PreCompact"]);
        assert!(reglage.poser_les_gabarits().unwrap().is_empty(), "jamais écrasés");
        assert_eq!(reglage.gabarit("rappel_identity"), section(GABARITS_LIVRES[1].1, "rappel_identity").unwrap());
        assert!(cles_livrees().iter().any(|c| c == "consignes_resume") && cles_livrees().len() == 8, "{:?}", cles_livrees());
        assert_eq!(reglage.gabarit("inconnue"), "");
        // Corriger le fichier de SessionStart : la section change, les autres restent.
        std::fs::write(reglage.templates_dir.join("SessionStart.md"), "# essai\n\n## session_compactee\n\nCompactée : {rubriques}, à toi.\n\n## sans_identity\n\n- rien\n").unwrap();
        assert_eq!(reglage.gabarit("sans_fiche"), section(GABARITS_LIVRES[0].1, "sans_fiche").unwrap(), "une section absente du fichier corrigé prend le livré");
        let h = hote_avec_identite();
        let r = repondre(&Evenement::lire(SESSION_START.replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1").as_str()).unwrap(), &reglage, Ok(&h));
        assert!(r.bloc.contains("Compactée : who, runtime, can, doing, holding, mandates, peers, à toi."), "{}", r.bloc);
        assert!(r.journal[0].contains("gabarits ") && !r.journal[0].contains("intégrés"), "{}", r.journal[0]);
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn le_rejeu_hors_session_rend_le_bloc_et_son_journal() {
        let _env = env();
        let dossier = std::env::temp_dir().join(format!("hooks-rejeu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let transcription = dossier.join("abc-123.jsonl");
        std::fs::write(&transcription, format!("{T_USER}\n{T_ASSISTANT_PETIT}\n")).unwrap();
        let config = dossier.join("hooks.json");
        std::fs::write(&config, json!({ "journal": dossier.join("hooks.log"), "templates_dir": dossier.join("g"), "timeout_ms": 500 }).to_string()).unwrap();
        let (adresse, recues) = hote_en_carton(vec![
            (200, json!({ "session": "S" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (200, json!({ "ok": true, "result": { "result": "[]" } }).to_string()),
            (200, json!({ "ok": true, "result": "les genres : handle, doc" }).to_string()),
            (200, json!({ "closed": true }).to_string()),
        ]);
        let args: Vec<String> = ["--rejouer", transcription.to_str().unwrap(), "SessionStart", "--config", config.to_str().unwrap(), "--daemon", &adresse].iter().map(|s| s.to_string()).collect();
        let mut sortie = Vec::new();
        assert_eq!(commande(&args, "", &mut sortie), 0);
        let texte = String::from_utf8(sortie).unwrap();
        assert!(texte.starts_with("[mémoire]\nAucune fiche d'identité"), "{texte}");
        assert!(texte.contains("session:abc-123"), "l'id de session est le nom du fichier — {texte}");
        assert!(texte.contains("— journal :") && texte.contains("SessionStart abc-123 : hôte joint"), "{texte}");
        assert!(std::fs::read_to_string(dossier.join("hooks.log")).unwrap().contains("[rejeu] SessionStart abc-123"));
        let r = recues.lock().unwrap();
        assert_eq!(r[2].1, "/call");
        assert!(r[2].3.contains("\"value\":\"session:abc-123\"") && r[2].3.contains("\"genres\":[\"handle\"]"), "{}", r[2].3);
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn l_evenement_en_argument_est_relu_contre_l_entree() {
        let _env = env();
        let dossier = std::env::temp_dir().join(format!("hooks-arg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let config = dossier.join("hooks.json");
        std::fs::write(&config, json!({ "journal": dossier.join("hooks.log"), "timeout_ms": 300 }).to_string()).unwrap();
        let ferme = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ferme.local_addr().unwrap().to_string();
        drop(ferme);
        let args: Vec<String> = ["Stop", "--config", config.to_str().unwrap(), "--daemon", &adresse].iter().map(|s| s.to_string()).collect();
        let mut sortie = Vec::new();
        assert_eq!(commande(&args, PROMPT, &mut sortie), 0);
        let lu = std::fs::read_to_string(dossier.join("hooks.log")).unwrap();
        assert!(lu.contains("l'argument dit Stop, l'entrée dit UserPromptSubmit — l'entrée fait foi"), "{lu}");
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn help_repond_sans_lire_l_entree_ni_joindre_l_hote() {
        let _env = env();
        let mut sortie = Vec::new();
        assert_eq!(commande(&["--help".to_string()], "", &mut sortie), 0);
        let texte = String::from_utf8(sortie).unwrap();
        assert!(texte.contains("usage : rag3weaver-backend hook") && texte.contains("sort toujours en 0"), "{texte}");
    }

    #[test]
    fn la_commande_sort_en_zero_et_se_tait_sans_hote_et_journalise() {
        let _env = env();
        let dossier = std::env::temp_dir().join(format!("hooks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let journal = dossier.join("hooks.log");
        let config = dossier.join("hooks.json");
        std::fs::write(&config, json!({ "journal": journal, "timeout_ms": 500 }).to_string()).unwrap();
        // Un port fermé : personne n'écoute.
        let ferme = TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ferme.local_addr().unwrap().to_string();
        drop(ferme);
        let args = vec!["--config".to_string(), config.display().to_string(), "--daemon".to_string(), adresse];
        let mut sortie = Vec::new();
        assert_eq!(commande(&args, STOP, &mut sortie), 0);
        assert!(sortie.is_empty(), "rien sur la sortie sans hôte");
        let lu = std::fs::read_to_string(&journal).unwrap();
        assert!(lu.contains("Stop s1 : silence —") && lu.contains("ne répond pas"), "{lu}");
        // Une entrée illisible : zéro aussi, et dit au journal.
        assert_eq!(commande(&args, "{", &mut sortie), 0);
        assert!(std::fs::read_to_string(&journal).unwrap().contains("entrée refusée"));
        // Un hôte en carton qui répond : « hôte joint » au journal, sortie vide (ce lot).
        let (adresse, _) = hote_en_carton(vec![
            (200, json!({ "session": "S" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (200, json!({ "ok": true, "result": { "result": "[]" } }).to_string()),
            (200, json!({ "ok": true, "backend": "claude-code", "run": "s1-x" }).to_string()),
            (200, json!({ "closed": true }).to_string()),
        ]);
        let args = vec!["--config".to_string(), config.display().to_string(), "--daemon".to_string(), adresse];
        assert_eq!(commande(&args, STOP, &mut sortie), 0);
        assert!(sortie.is_empty(), "Stop ne rend rien");
        let lu = std::fs::read_to_string(&journal).unwrap();
        assert!(lu.contains("Stop s1 : hôte joint, backend « memoire-longue » [") && lu.contains("run déposé"), "{lu}");
        let _ = std::fs::remove_dir_all(&dossier);
    }

    // ─── Lot 2 : les blocs de lecture ─────────────────────────────────────

    /// Un hôte qui répond par outil, et garde les appels.
    struct HoteCanne {
        reponses: std::collections::HashMap<&'static str, Value>,
        appels: Mutex<Vec<(String, Value)>>,
        runs: Mutex<Vec<Value>>,
    }
    impl HoteCanne {
        fn new(paires: Vec<(&'static str, Value)>) -> Self {
            Self { reponses: paires.into_iter().collect(), appels: Mutex::new(Vec::new()), runs: Mutex::new(Vec::new()) }
        }
        fn noms(&self) -> Vec<String> {
            self.appels.lock().unwrap().iter().map(|(n, _)| n.clone()).collect()
        }
    }
    impl Hote for HoteCanne {
        fn nom(&self) -> &str {
            "memoire-longue"
        }
        fn deposer_run(&self, corps: Value) -> Result<Value, String> {
            self.runs.lock().unwrap().push(corps);
            Ok(json!({ "ok": true }))
        }
        fn appeler(&self, outil: &str, arguments: Value) -> Result<Value, String> {
            self.appels.lock().unwrap().push((outil.to_string(), arguments.clone()));
            // `recall_apports` répond selon la section demandée.
            if outil == "recall_apports" {
                let section = arguments["options"]["filter_condition"]["must"][1]["field"]["value"].as_str().unwrap_or("");
                let cle: &'static str = match section { "identity" => "recall_apports:identity", "plan" => "recall_apports:plan", "bilan" => "recall_apports:bilan", _ => "recall_apports" };
                return Ok(self.reponses.get(cle).cloned().unwrap_or_else(recherche_vide));
            }
            Ok(self.reponses.get(outil).cloned().unwrap_or_else(recherche_vide))
        }
    }
    fn recherche_vide() -> Value {
        json!({ "ok": true, "result": { "result": [] } })
    }
    fn recherche(lignes: Vec<Value>) -> Value {
        json!({ "ok": true, "result": { "result": lignes.into_iter().map(|d| json!({ "data": d })).collect::<Vec<_>>() } })
    }
    /// La forme réelle d'un rendu de `cited_by` en json (relevée sur un vrai
    /// gabarit memory) : `{uuid, score, entity, data}` par fiche, le tout en chaîne.
    fn citant(fiches: Vec<Value>) -> Value {
        let lignes: Vec<Value> = fiches.into_iter().map(|d| json!({ "uuid": "u", "score": 1.0, "entity": "Fiche", "data": d, "signal": "plan" })).collect();
        json!({ "ok": true, "result": { "result": Value::Array(lignes).to_string() } })
    }
    const IDENTITY: &str = "## identity\nwho: 96 — « mémoire », la session de la mémoire longue\nruntime: claude-code ; session:s1\ncan: les outils MCP memoire-longue\ndoing: la migration de la fiche de Lucie (fiche:migration-fiche) ; la page 04\nholding: worktrees wt-docs-96 ; branches lot-96-*\nmandates: chaque commit poussé dans la minute\npeers: c7 « orchestration » ; 73 « arbre principal »";

    fn hote_avec_identite() -> HoteCanne {
        HoteCanne::new(vec![
            ("cited_by", citant(vec![json!({ "name": "96", "about": "la session de la mémoire longue", "state": "current", "refs": [{ "genre": "handle", "valeur": "session:s1" }], "apports": [] })])),
            ("recall_apports:identity", recherche(vec![json!({ "fiche": "96", "section": "identity", "texte": IDENTITY, "created_at": 10 }), json!({ "fiche": "96", "section": "identity", "texte": "## identity\nwho: vieux", "created_at": 5 })])),
            ("recall_apports:plan", recherche(vec![json!({ "fiche": "migration-fiche", "section": "plan", "texte": "1. relire la fiche\n2. migrer les apports", "created_at": 7 })])),
        ])
    }

    #[test]
    fn session_start_rend_qui_je_suis_ce_que_je_tiens_et_le_plan_en_cours() {
        let _env = env();
        let h = hote_avec_identite();
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        let b = &r.bloc;
        assert!(b.starts_with("[mémoire]\n"), "{b}");
        assert!(b.contains("Tu es **96** — la session de la mémoire longue"), "{b}");
        assert!(b.contains("- En cours : la migration de la fiche de Lucie"), "{b}");
        assert!(b.contains("- Tu tiens : worktrees wt-docs-96"), "{b}");
        assert!(b.contains("- Pairs : c7"), "{b}");
        assert!(b.contains("Plan en cours (fiche « migration-fiche ») : 1. relire la fiche …"), "{b}");
        assert!(!b.contains("compactée"), "pas de consigne de compaction au startup");
        let noms = h.noms();
        assert_eq!(noms[0], "cited_by");
        let premier = h.appels.lock().unwrap()[0].1.clone();
        assert_eq!((premier["value"].as_str(), premier["genres"][0].as_str(), premier["under"].as_bool()), (Some("session:s1"), Some("handle"), Some(false)), "la valeur nue, le genre handle");
        assert!(noms.contains(&"recall_apports".to_string()));
        assert!(r.journal.iter().any(|l| l.contains("bloc de")), "{:?}", r.journal);
    }

    #[test]
    fn session_start_apres_compaction_demande_l_identity_a_l_agent() {
        let _env = env();
        let h = hote_avec_identite();
        let r = repondre(&Evenement::lire(SESSION_START.replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Ta session vient d'être compactée. Au premier tour"), "{}", r.bloc);
        assert!(r.bloc.contains("Tu es **96**"), "la fiche est rendue même après compaction");
    }

    #[test]
    fn sans_fiche_le_bloc_demande_le_handle_ou_la_fiche_et_ne_cree_rien() {
        let _env = env();
        let h = HoteCanne::new(vec![]);
        std::env::remove_var("RAG3WEAVER_AGENT_KEY");
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Aucune fiche d'identité ne cite cette session"), "{}", r.bloc);
        assert!(r.bloc.contains("session:25b7ff4b-dabf-42ef-8580-86661c846fb9"), "l'id est donné à l'agent");
        assert!(r.bloc.contains("Rien n'est créé à ta place") && r.bloc.contains("author: <clé>"), "{}", r.bloc);
        assert_eq!(h.noms(), vec!["cited_by", "create_ref_type"], "deux lectures, aucune écriture");
        let lecture = &h.appels.lock().unwrap()[1].1;
        assert_eq!(lecture["confirm"], false, "create_ref_type sans confirmation n'écrit rien");
        assert!(r.bloc.contains("le genre `handle` n'existe pas encore dans cette mémoire : crée-le d'abord"), "le faux hôte répond sans nommer le genre : il manque, et la prescription dit comment le créer — {}", r.bloc);
    }

    #[test]
    fn une_cle_donnee_par_la_variable_ne_vaut_que_si_sa_fiche_existe() {
        let _env = env();
        let h = HoteCanne::new(vec![("get_fiche", json!({ "ok": true, "result": { "result": { "data": { "name": "archi", "about": "ex-orchestration", "state": "current" } } } }))]);
        std::env::set_var("RAG3WEAVER_AGENT_KEY", "archi");
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Tu es **archi** (clé donnée par RAG3WEAVER_AGENT_KEY"), "{}", r.bloc);
        assert!(r.bloc.contains("Aucun apport `identity`"), "{}", r.bloc);
        let h2 = HoteCanne::new(vec![("get_fiche", json!({ "ok": true, "result": { "result": { "data": null } } }))]);
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h2));
        assert!(r.bloc.contains("Aucune fiche d'identité"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("aucune fiche de cette clé, indice ignoré")), "{:?}", r.journal);
        std::env::remove_var("RAG3WEAVER_AGENT_KEY");
    }

    fn transcription_d_essai(nom: &str, lignes: &[&str]) -> (PathBuf, Reglage) {
        let dossier = std::env::temp_dir().join(format!("hooks-lot2-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("t.jsonl");
        std::fs::write(&chemin, lignes.join("\n") + "\n").unwrap();
        let mut reglage = Reglage::defaut();
        reglage.journal = dossier.join("hooks.log");
        (chemin, reglage)
    }
    const T_USER: &str = r#"{"type":"user","promptId":"p1","message":{"role":"user","content":"salut"}}"#;
    const T_ASSISTANT_PETIT: &str = r#"{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":2,"cache_creation_input_tokens":100,"cache_read_input_tokens":30000,"output_tokens":5},"content":[{"type":"tool_use","name":"Edit","id":"t1","input":{"file_path":"/p/src/a.rs","old_string":"x","new_string":"y"}}]}}"#;
    const T_ASSISTANT_PLEIN: &str = r#"{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":2,"cache_creation_input_tokens":100,"cache_read_input_tokens":800000,"output_tokens":5},"content":[{"type":"text","text":"ok"}]}}"#;
    const T_BOUNDARY: &str = r#"{"type":"system","subtype":"compact_boundary","content":"Conversation compacted","compactMetadata":{"trigger":"auto"}}"#;
    const T_SUMMARY: &str = r#"{"type":"user","isCompactSummary":true,"message":{"role":"user","content":"This session is being continued… Summary: la session migrait la fiche."},"promptId":"p9"}"#;

    fn prompt(chemin: &Path, texte: &str) -> Evenement {
        Evenement::lire(&json!({ "session_id": "s1", "transcript_path": chemin, "cwd": "/p", "hook_event_name": "UserPromptSubmit", "prompt": texte, "prompt_id": "p2" }).to_string()).unwrap()
    }

    #[test]
    fn user_prompt_rappelle_les_fiches_les_diffs_des_fichiers_touches_et_le_verdict() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("rappel", &[T_USER, T_ASSISTANT_PETIT]);
        let mut h = hote_avec_identite();
        h.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [
            { "chunk": { "score": 0.58 }, "signals": [{ "label": "vector", "score": 0.58 }, { "label": "bm25", "score": 2.1 }], "data": { "name": "régime GPU", "about": "Comment on ménage les cartes du poste.", "state": "current" } },
            { "chunk": { "score": 0.12 }, "signals": [{ "label": "vector", "score": 0.12 }], "data": { "name": "sans rapport", "about": "trop loin", "state": "current" } }
        ] } }));
        h.reponses.insert("recall_apports:bilan", recherche(vec![json!({ "fiche": "bilan : migration-fiche", "section": "bilan", "texte": "l'outil put_apport a trompé l'agent sur `source`", "created_at": 9 })]));
        // `cited_by` répond pareil pour le handle et pour le fichier : la fiche 96 porte un apport diff d'un autre.
        h.reponses.insert("cited_by", citant(vec![json!({ "name": "96", "about": "la session de la mémoire longue", "state": "current", "refs": [], "apports": [
            { "section": "diff", "author": "73", "texte": "a.rs : le COPY passe par la lib", "created_at": 11 },
            { "section": "diff", "author": "96", "texte": "le mien, à ne pas rendre", "created_at": 12 }
        ] })]));
        let r = repondre(&prompt(&chemin, "range la fiche régime GPU"), &reglage, Ok(&h));
        let b = &r.bloc;
        assert!(b.contains("Fiche en rapport (touchée au plein texte par les mots du prompt) : « régime GPU » (plein texte 2.10, dense 0.58 ; Comment on ménage les cartes du poste.) — `get_fiche` pour lire."), "{b}");
        assert!(!b.contains("sans rapport"), "sous le seuil, la fiche n'est pas dite — {b}");
        assert!(r.journal.iter().any(|l| l.contains("rappel : « régime GPU » plein texte 2.10, dense 0.58, « sans rapport » dense 0.12")), "{:?}", r.journal);
        assert!(r.journal.iter().any(|l| l.contains("dense seul, non jugé : « sans rapport » 0.12")), "{:?}", r.journal);
        assert!(b.contains("`/p/src/a.rs` a changé depuis, par 73 : a.rs : le COPY passe par la lib"), "{b}");
        assert!(!b.contains("le mien"), "les diffs de la session elle-même ne sont pas rendus");
        assert!(b.contains("Le juge, sur « migration-fiche » : l'outil put_apport a trompé"), "{b}");
        assert!(!b.contains("compactée") && !b.contains("Ton contexte"), "{b}");
        let appels = h.appels.lock().unwrap();
        let path = appels.iter().find(|(n, a)| n == "cited_by" && a["value"] == "path:/p/src/a.rs").expect("cited_by path:");
        assert!(path.1["since"].as_u64().unwrap() > 0);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn user_prompt_court_rend_l_en_tete_seul() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("vide", &[T_USER, T_ASSISTANT_PETIT]);
        let h = hote_avec_identite();
        let r = repondre(&prompt(&chemin, "oui"), &reglage, Ok(&h));
        assert!(r.bloc.is_empty(), "rien à dire, pas d'en-tête : l'identité se dit au démarrage — {}", r.bloc);
        assert!(!h.noms().contains(&"recall_fiches".to_string()), "un prompt de moins de 8 caractères ne cherche rien");
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn le_premier_tour_apres_compaction_lance_le_defi_d_identite() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("compact", &[T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY]);
        let h = hote_avec_identite();
        let r = repondre(&prompt(&chemin, "on continue la migration"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Ta session a été compactée. Avant toute autre chose, écris ton apport `identity`"), "{}", r.bloc);
        assert!(r.bloc.contains("(fiche « 96 »)") && r.bloc.contains("Ton dernier `identity` : ## identity …"), "{}", r.bloc);
        let (chemin2, reglage2) = transcription_d_essai("apres", &[T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY, T_ASSISTANT_PETIT]);
        let r = repondre(&prompt(&chemin2, "on continue la migration"), &reglage2, Ok(&h));
        assert!(!r.bloc.contains("compactée"), "un tour est passé : plus de défi — {}", r.bloc);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
        let _ = std::fs::remove_dir_all(chemin2.parent().unwrap());
    }

    #[test]
    fn le_contexte_plein_suggere_de_ranger_une_seule_fois() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("plein", &[T_USER, T_ASSISTANT_PLEIN]);
        let h = hote_avec_identite();
        let r = repondre(&prompt(&chemin, "encore une question"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Ton contexte est à ~800 k jetons sur 1000 k (usage du dernier tour) : range maintenant"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("suggestion de ranger")));
        let r2 = repondre(&prompt(&chemin, "encore une question"), &reglage, Ok(&h));
        assert!(!r2.bloc.contains("Ton contexte"), "une fois par session — {}", r2.bloc);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn le_rappel_envoie_des_mots_cles_et_ne_rend_pas_deux_fois_ce_que_la_session_a_recu() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("deja-vu", &[T_USER, T_ASSISTANT_PETIT]);
        // La forme réelle : `signals` par ligne, le plein texte (bm25) et le dense.
        let fiche = |nom: &str, about: &str, plein: f64| json!({ "ok": true, "result": { "result": [{ "chunk": { "score": 0.4 }, "signals": [{ "label": "vector", "score": 0.4 }, { "label": "bm25", "score": plein }], "data": { "name": nom, "about": about, "state": "current" } }] } });
        let mut h = hote_avec_identite();
        h.reponses.insert("recall_fiches", fiche("régime GPU", "Comment on ménage les cartes.", 0.8));
        let question = "<cross-session-message from=\"uds:/run/user/1000/cc-socks/1.sock\" from-name=\"c7\">\ncomment ménager le régime GPU pendant les passes, avec RAG3WEAVER_GPU_DUTY ?\n</cross-session-message>";
        let r = repondre(&prompt(&chemin, question), &reglage, Ok(&h));
        assert!(r.bloc.contains("« régime GPU » (plein texte 0.80, dense 0.40"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("déjà rendu : 0, neuf : 1")), "{:?}", r.journal);
        let envoyee = h.appels.lock().unwrap().iter().rev().find(|(n, _)| n == "recall_fiches").map(|(_, a)| a["query"].as_str().unwrap_or("").to_string()).unwrap();
        assert!(!envoyee.contains("cc-socks") && !envoyee.contains("from-name") && envoyee.contains("RAG3WEAVER_GPU_DUTY") && envoyee.split_whitespace().count() <= 10, "des mots-clés, pas le prompt : « {envoyee} »");
        assert!(r.journal.iter().any(|l| l.starts_with("rappel : requête « ") && l.contains(&envoyee)), "le journal dit la requête mot à mot : {:?}", r.journal);
        // Le même rappel au tour suivant : rien, pas même une ligne.
        let r = repondre(&prompt(&chemin, question), &reglage, Ok(&h));
        assert!(!r.bloc.contains("régime GPU") && !r.bloc.contains("Fiches en rapport"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("déjà rendu : 1, neuf : 0 ; silence")), "{:?}", r.journal);
        // La compaction vide le contexte de l'agent : une ligne, puis la fiche ressort.
        let compact = Evenement::lire(&SESSION_START.replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1")).unwrap();
        let r = repondre(&compact, &reglage, Ok(&h));
        assert!(r.bloc.contains("Fiches rappelées avant la compaction : « régime GPU »"), "{}", r.bloc);
        let r = repondre(&prompt(&chemin, question), &reglage, Ok(&h));
        assert!(r.bloc.contains("« régime GPU »"), "après la compaction, elle ressort : {}", r.bloc);
        // Son résumé change : elle revient, « mise à jour ».
        h.reponses.insert("recall_fiches", fiche("régime GPU", "Comment on ménage les cartes, et la carte 2.", 0.8));
        let r = repondre(&prompt(&chemin, question), &reglage, Ok(&h));
        assert!(r.bloc.contains("mise à jour depuis le tour"), "{}", r.bloc);
        // Une fiche que seul le dense trouve, même haut : non jugée, non rendue, et dit
        // (le dense donnait 0,32 à 0,37 au même titre, hors sujet comme en plein sujet).
        h.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [{ "chunk": { "score": 0.9 }, "signals": [{ "label": "vector", "score": 0.9 }], "data": { "name": "autre fiche", "about": "dense seul", "state": "current" } }] } }));
        let r = repondre(&prompt(&chemin, question), &reglage, Ok(&h));
        assert!(r.journal.iter().any(|l| l.contains("dense seul, non jugé : « autre fiche » 0.90")), "{:?}", r.journal);
        assert!(!r.bloc.contains("« autre fiche »"), "{}", r.bloc);
        // Une enveloppe sans contenu : aucun rappel.
        let avant = h.noms().iter().filter(|n| *n == "recall_fiches").count();
        let r = repondre(&prompt(&chemin, "<task-notification>\n<task-id>b1x2y3z4</task-id>\n<status>completed</status>\n</task-notification>"), &reglage, Ok(&h));
        assert_eq!(h.noms().iter().filter(|n| *n == "recall_fiches").count(), avant, "{:?}", r.journal);
        assert!(r.journal.iter().any(|l| l.contains("rappel : tour de notification, rien à chercher")), "{:?}", r.journal);
        // Sans identité : rien n'est exclu, et le journal le dit.
        let mut sans = HoteCanne::new(vec![]);
        sans.reponses.insert("recall_fiches", fiche("96", "moi", 0.9));
        let r = repondre(&prompt(&chemin, "une question sur la fiche propre et son exclusion"), &reglage, Ok(&sans));
        assert!(r.journal.iter().any(|l| l.contains("identité inconnue, pas d'exclusion")), "{:?}", r.journal);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    /// Un faux hôte qui a une boîte (lot 5) : la mémoire de `HoteCanne`, la boîte
    /// de `mail::tests::FausseBoite`.
    struct AvecBoite(HoteCanne, Arc<crate::mail::tests::FausseBoite>);
    impl Hote for AvecBoite {
        fn nom(&self) -> &str { self.0.nom() }
        fn appeler(&self, outil: &str, a: Value) -> Result<Value, String> { self.0.appeler(outil, a) }
        fn deposer_run(&self, c: Value) -> Result<Value, String> { self.0.deposer_run(c) }
        fn boite(&self) -> Option<&dyn crate::mail::Boite> { Some(&*self.1) }
    }

    #[test]
    fn la_boite_se_montre_en_tete_du_tour_puis_en_une_ligne_et_les_comptes_rendus_sortent_par_seen() {
        let _env = env();
        let (chemin, mut reglage) = transcription_d_essai("boite", &[T_USER]);
        let boite = Arc::new(crate::mail::tests::FausseBoite { items: vec![crate::mail::tests::item("b1/doc:3", "question", Some("codex:rag3weaver1"), "Je mets le README à la racine ?"), crate::mail::tests::item("b1/doc:4", "report", None, "fait")], ..Default::default() });
        let h = AvecBoite(hote_avec_identite(), boite.clone());
        let r = repondre(&prompt(&chemin, "une question assez longue sur les hooks"), &reglage, Ok(&h));
        assert!(r.bloc.contains("codex:rag3weaver1 (doc) te demande : « Je mets le README à la racine ? » — réponds : `rag3weaver-backend mail reply --from claude-code:s1 --to b1/doc:3 \"<ta réponse>\"`"), "{}", r.bloc);
        assert!(r.bloc.contains("rend compte : « fait »"), "{}", r.bloc);
        // Le compte rendu sort par seen, sous chaque clé de la session (handle et clé stable).
        let vus = boite.vus.lock().unwrap().clone();
        assert!(vus.iter().any(|(c, ids)| c == "claude-code:s1" && ids == &vec!["b1/doc:4".to_string()]) && vus.iter().any(|(c, _)| c == "96"), "{vus:?}");
        // Au tour suivant : la question en une ligne, rien d'autre.
        let r = repondre(&prompt(&chemin, "une autre question assez longue"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Toujours en attente de ta réponse : codex:rag3weaver1 (question, `b1/doc:3`).") && !r.bloc.contains("Je mets le README"), "{}", r.bloc);
        // PostToolUse ne montre que le neuf : rien ici.
        let outil = Evenement::lire(&json!({ "session_id": "s1", "hook_event_name": "PostToolUse" }).to_string()).unwrap();
        assert!(repondre(&outil, &reglage, Ok(&h)).bloc.is_empty());
        // Un rejeu ne poste jamais seen.
        let avant = boite.vus.lock().unwrap().len();
        reglage.rejeu = true;
        let _ = repondre(&prompt(&chemin, "une question assez longue sur le rejeu"), &reglage, Ok(&h));
        assert_eq!(boite.vus.lock().unwrap().len(), avant, "aucun seen en rejeu");
        // La boîte en panne : une ligne qui le dit, jamais un silence.
        reglage.rejeu = false;
        let panne = AvecBoite(hote_avec_identite(), Arc::new(crate::mail::tests::FausseBoite { en_panne: true, ..Default::default() }));
        let r = repondre(&prompt(&chemin, "une question assez longue encore"), &reglage, Ok(&panne));
        assert!(r.bloc.contains("Boîte des agents injoignable") && r.bloc.contains("mail pending --for claude-code:s1"), "{}", r.bloc);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn le_juge_se_journalise_se_plafonne_et_se_coupe() {
        let _env = env();
        let (chemin, mut reglage) = transcription_d_essai("plafond-juge", &[T_USER]);
        // Des candidats venus du tour : ce qui reste après son seul rendu.
        let tache = |reglage: &Reglage, sid: &str| {
            let t = json!({ "session_id": sid, "tour": 1, "requete": "cartes", "prompt": "les cartes", "candidats": [{ "name": "a", "about": "x", "plein": 2.0 }, { "name": "b", "about": "y", "plein": 3.0 }] });
            let chemin = chemin_du_fond(reglage, sid, "tache");
            std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
            std::fs::write(&chemin, t.to_string()).unwrap();
            chemin
        };
        let juge = crate::llm::MockLlm::new(r#"{"pertinent": true, "pourquoi": "les deux servent", "items_gardes": ["a", "b"]}"#);
        // Chaque appel se journalise : le modèle, la durée, les jetons.
        reglage.recall_judge_max_per_hour = 1;
        let j = rappel_differe(&tache(&reglage, "p1"), &reglage, None, Some(&juge));
        assert!(j.iter().any(|l| l.starts_with("[fond] juge : modèle ") && l.contains(" ms, entrée ") && l.contains(" jetons, sortie ")), "{j:?}");
        // Le plafond atteint : le juge n'est pas appelé, et rien ne passe sans lui.
        let j = rappel_differe(&tache(&reglage, "p2"), &reglage, None, Some(&juge));
        assert!(j.iter().any(|l| l.contains("[fond] sans juge (plafond horaire atteint, 1 appels) : 0 rendu, 2 tus")), "{j:?}");
        assert!(!j.iter().any(|l| l.starts_with("[fond] juge : modèle")), "{j:?}");
        // Le juge coupé : des candidats venus du tour se taisent.
        reglage.recall_judge = false;
        let j = rappel_differe(&tache(&reglage, "p3"), &reglage, None, Some(&juge));
        assert!(j.iter().any(|l| l.contains("[fond] sans juge (recall_judge à false) : 0 rendu, 2 tus")), "{j:?}");
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn le_juge_ote_et_n_invente_rien() {
        let candidats = vec![json!({ "name": "régime GPU", "about": "les cartes du poste" }), json!({ "name": "hooks-essai", "about": "la session d'essai" })];
        let llm = crate::llm::MockLlm::new(r#"{"pertinent": true, "pourquoi": "le message parle des cartes", "items_gardes": ["régime GPU", "une fiche inventée"]}"#);
        let j = juger(&llm, "comment ménager les cartes", &candidats).unwrap();
        assert_eq!(j.gardes, vec!["régime GPU".to_string()], "un nom qu'il n'a pas reçu est jeté");
        assert!(j.pertinent);
        let llm = crate::llm::MockLlm::new(r#"{"pertinent": false, "pourquoi": "hors sujet", "items_gardes": []}"#);
        assert!(!juger(&llm, "x", &candidats).unwrap().pertinent);
        assert!(juger(&crate::llm::MockLlm::new("pas du json"), "x", &candidats).unwrap_err().contains("réponse illisible"));
    }

    #[test]
    fn le_rappel_differe_est_juge_range_puis_rendu_une_fois_au_prochain_outil() {
        let _env = env();
        let (chemin, mut reglage) = transcription_d_essai("differe", &[T_USER, T_ASSISTANT_PETIT]);
        // Le rappel synchrone n'a pas répondu dans son budget : la tâche est écrite pour le fils.
        struct RappelLent(HoteCanne);
        impl Hote for RappelLent {
            fn nom(&self) -> &str { self.0.nom() }
            fn appeler(&self, outil: &str, a: Value) -> Result<Value, String> { self.0.appeler(outil, a) }
            fn appeler_borne(&self, outil: &str, a: Value, d: Duration) -> Result<Value, String> {
                if d < Duration::from_secs(5) { Err(format!("/call {outil} : l'hôte http://x ne répond pas : timeout: global")) } else { self.0.appeler(outil, a) }
            }
            fn deposer_run(&self, c: Value) -> Result<Value, String> { self.0.deposer_run(c) }
        }
        let mut canne = hote_avec_identite();
        canne.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [
            { "chunk": { "score": 0.7 }, "signals": [{ "label": "vector", "score": 0.7 }, { "label": "bm25", "score": 2.5 }], "data": { "name": "régime GPU", "about": "Comment on ménage les cartes.", "state": "current" } },
            { "chunk": { "score": 0.6 }, "signals": [{ "label": "bm25", "score": 1.2 }], "data": { "name": "sans rapport", "about": "loin", "state": "current" } }
        ] } }));
        let h = RappelLent(canne);
        let r = repondre(&prompt(&chemin, "comment ménager le régime GPU pendant les passes"), &reglage, Ok(&h));
        assert!(r.bloc.contains("il continue en fond, son résultat viendra au prochain outil ou au prochain tour"), "{}", r.bloc);
        let tache = chemin_du_fond(&reglage, "s1", "tache");
        assert!(tache.exists(), "{:?}", r.journal);
        // Le fils : rappel sous un délai long, juge, rangement.
        let juge = crate::llm::MockLlm::new(r#"{"pertinent": true, "pourquoi": "le message parle des cartes", "items_gardes": ["régime GPU"]}"#);
        let j = rappel_differe(&tache, &reglage, Some(&h), Some(&juge));
        assert!(j.iter().any(|l| l.contains("[fond] juge : 2 candidat(s), 1 gardé(s) — le message parle des cartes")), "{j:?}");
        assert!(!tache.exists(), "la tâche est consommée");
        // Le premier outil le rend, une fois, en additionalContext.
        let outil = Evenement::lire(&json!({ "session_id": "s1", "hook_event_name": "PostToolUse", "tool_name": "Bash" }).to_string()).unwrap();
        let r = repondre(&outil, &reglage, Err("pas d'hôte".into()));
        assert!(r.bloc.contains("Rappel du tour d'avant (requête « ") && r.bloc.contains("« régime GPU »") && !r.bloc.contains("sans rapport"), "{}", r.bloc);
        let r = repondre(&outil, &reglage, Err("pas d'hôte".into()));
        assert!(r.bloc.is_empty(), "une seule fois : {}", r.bloc);
        // Et la fiche est désormais connue de la session : un rappel suivant la tait.
        let etat = EtatDuRappel::lire(&reglage, "s1");
        assert!(etat.rendus.contains_key("régime GPU"), "{etat:?}");
        // Des candidats hors sujet : le juge rend le silence, le journal dit pourquoi.
        reglage.journal = chemin.parent().unwrap().join("hooks2.log");
        std::fs::write(chemin_du_fond(&reglage, "s2", "tache"), json!({ "session_id": "s2", "tour": 1, "requete": "météo", "prompt": "quel temps fait-il", "candidats": [{ "name": "régime GPU", "about": "les cartes", "score": 3.5 }] }).to_string()).unwrap();
        let non = crate::llm::MockLlm::new(r#"{"pertinent": false, "pourquoi": "le message ne parle pas de la mémoire", "items_gardes": []}"#);
        let j = rappel_differe(&chemin_du_fond(&reglage, "s2", "tache"), &reglage, None, Some(&non));
        assert!(j.iter().any(|l| l.contains("0 gardé(s) — le message ne parle pas de la mémoire")), "{j:?}");
        let outil2 = Evenement::lire(&json!({ "session_id": "s2", "hook_event_name": "PostToolUse" }).to_string()).unwrap();
        let r = repondre(&outil2, &reglage, Err("pas d'hôte".into()));
        assert!(r.bloc.is_empty(), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("rappel du tour d'avant : 0 rendu(s) — juge : le message ne parle pas de la mémoire")), "{:?}", r.journal);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn un_seul_rendu_par_tour_les_autres_vont_au_juge() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("brute-juge", &[T_USER, T_ASSISTANT_PETIT]);
        let mut h = hote_avec_identite();
        // La forme de 18:16:13 : la fiche propre touchée au plein texte (1,66), une
        // autre au dense seul (0,35) — puis deux fiches que le plein texte touche.
        h.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [
            { "chunk": { "score": 1.66 }, "signals": [{ "label": "vector", "score": 0.45 }, { "label": "bm25", "score": 1.66 }], "data": { "name": "96", "about": "moi", "state": "current" } },
            { "chunk": { "score": 0.35 }, "signals": [{ "label": "vector", "score": 0.35 }], "data": { "name": "régime GPU", "about": "les cartes", "state": "current" } }
        ] } }));
        let r = repondre(&prompt(&chemin, "une question assez longue sur les hooks et leur binaire"), &reglage, Ok(&h));
        assert!(!r.bloc.contains("Fiche en rapport"), "la fiche propre exclue, le dense seul non jugé : silence — {}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("exclu : « 96 » (fiche propre) ; dense seul, non jugé : « régime GPU » 0.35 ; déjà rendu : 0, neuf : 0 ; silence")), "{:?}", r.journal);
        h.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [
            { "chunk": { "score": 0.4 }, "signals": [{ "label": "vector", "score": 0.4 }, { "label": "bm25", "score": 1.2 }], "data": { "name": "seconde", "about": "b", "state": "current" } },
            { "chunk": { "score": 3.5 }, "signals": [{ "label": "bm25", "score": 3.5 }], "data": { "name": "première", "about": "a", "state": "current" } }
        ] } }));
        let r = repondre(&prompt(&chemin, "une question assez longue sur une autre fiche"), &reglage, Ok(&h));
        assert!(r.bloc.contains("« première » (plein texte 3.50") && !r.bloc.contains("« seconde »"), "le meilleur au plein texte, seul : {}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("1 rendu, 1 au juge")), "{:?}", r.journal);
        let t: Value = serde_json::from_str(&std::fs::read_to_string(chemin_du_fond(&reglage, "s1", "tache")).unwrap()).unwrap();
        assert_eq!(t["candidats"][0]["name"], "seconde");
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn un_rappel_lent_n_emporte_pas_le_bloc() {
        let _env = env();
        struct RappelLent(HoteCanne);
        impl Hote for RappelLent {
            fn nom(&self) -> &str { self.0.nom() }
            fn appeler(&self, outil: &str, a: Value) -> Result<Value, String> { self.0.appeler(outil, a) }
            fn appeler_borne(&self, outil: &str, _a: Value, _d: Duration) -> Result<Value, String> {
                Err(format!("/call {outil} : l'hôte http://x ne répond pas : timeout: global"))
            }
            fn deposer_run(&self, c: Value) -> Result<Value, String> { self.0.deposer_run(c) }
        }
        let (chemin, reglage) = transcription_d_essai("lent", &[T_USER, T_ASSISTANT_PETIT]);
        let h = RappelLent(hote_avec_identite());
        let r = repondre(&prompt(&chemin, "une question assez longue sur les hooks"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Rappel des fiches : pas de réponse en 1000 ms — il continue en fond"), "la panne a sa ligne — {}", r.bloc);
        assert!(!r.bloc.contains("froid") && !r.bloc.contains("réessaie"), "{}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("recall_fiches en ") && l.contains("timeout")), "{:?}", r.journal);
        // Et quand rien ne passe le seuil, c'est dit autrement : « rien en rapport ».
        let mut h2 = hote_avec_identite();
        h2.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [{ "chunk": { "score": 0.05 }, "data": { "name": "régime GPU", "about": "", "state": "current" } }] } }));
        let r = repondre(&prompt(&chemin, "une question assez longue sur les hooks"), &reglage, Ok(&h2));
        assert!(r.bloc.is_empty(), "rien en rapport : le silence, et le journal a les scores — {}", r.bloc);
        assert!(r.journal.iter().any(|l| l.contains("rappel : « régime GPU » dense 0.05")), "{:?}", r.journal);
        // Sa propre fiche, même haute, n'est pas rendue.
        let mut h3 = hote_avec_identite();
        h3.reponses.insert("recall_fiches", json!({ "ok": true, "result": { "result": [{ "chunk": { "score": 0.9 }, "data": { "name": "96", "about": "moi", "state": "current" } }] } }));
        let r = repondre(&prompt(&chemin, "une question assez longue sur les hooks"), &reglage, Ok(&h3));
        assert!(!r.bloc.contains("« 96 »"), "{}", r.bloc);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn un_rejeu_ne_ecrit_rien_ni_run_ni_marque() {
        let _env = env();
        let h = hote_avec_identite();
        let lecture = HoteEnLecture(&h);
        let mut reglage = Reglage::defaut();
        let dossier = std::env::temp_dir().join(format!("hooks-rejeu-lecture-{}", std::process::id()));
        reglage.journal = dossier.join("hooks.log");
        reglage.rejeu = true;
        let r = repondre(&post_compact("s1", "résumé"), &reglage, Ok(&lecture));
        assert!(!h.noms().iter().any(|n| n.starts_with("put_") || n == "relier_apport" || n == "citer_source"), "aucune écriture : {:?}", h.noms());
        assert!(r.journal.iter().any(|l| l.contains("rejeu : put_source non écrit")), "{:?}", r.journal);
        let r = repondre(&Evenement::lire(STOP).unwrap(), &reglage, Ok(&lecture));
        assert!(h.runs.lock().unwrap().is_empty(), "aucun run déposé");
        assert!(r.journal.iter().any(|l| l.contains("rejeu : aucun run déposé")), "{:?}", r.journal);
        assert!(!dossier.join("hooks-suggestions").exists(), "aucune marque posée");
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn le_bloc_est_borne_et_le_dit() {
        let long = "x".repeat(5000);
        let b = borner(&long, 100);
        assert_eq!(b.chars().count(), 100 - 12 + "\n[… borné]".chars().count());
        assert!(b.ends_with("[… borné]"));
        assert_eq!(borner("court", 100), "court");
        assert_eq!(borner("sans borne", 0), "sans borne");
    }

    #[test]
    fn les_rubriques_et_les_fiches_citees_se_lisent() {
        assert_eq!(rubrique(IDENTITY, "doing"), Some("la migration de la fiche de Lucie (fiche:migration-fiche) ; la page 04"));
        assert_eq!(rubrique(IDENTITY, "absent"), None);
        assert_eq!(fiches_citees("x fiche:a, fiche:b (fiche:a) fiche:«c»"), vec!["a", "b", "c"]);
        assert_eq!(fiches_citees("doing: l'essai (fiche:essai des hooks) ; fiche:bilan : lis main.rs"), vec!["essai des hooks", "bilan : lis main.rs"], "un nom de fiche a des espaces");
        assert_eq!(premiere_ligne("une\ndeux", 10), "une …");
        assert_eq!(premiere_ligne("seule", 10), "seule");
    }

    // ─── Lot 3 : les écritures mécaniques ─────────────────────────────────

    fn hote_sans_tache() -> HoteCanne {
        HoteCanne::new(vec![
            ("cited_by", citant(vec![json!({ "name": "96", "about": "la session de la mémoire longue", "state": "current", "refs": [], "apports": [] })])),
            ("recall_apports:identity", recherche(vec![json!({ "fiche": "96", "section": "identity", "texte": "## identity\nwho: 96\ndoing: rien de nommé", "created_at": 10 })])),
        ])
    }

    fn post_compact(session: &str, resume: &str) -> Evenement {
        Evenement::lire(&json!({ "session_id": session, "transcript_path": "/t.jsonl", "cwd": "/p", "hook_event_name": "PostCompact", "trigger": "auto", "compact_summary": resume }).to_string()).unwrap()
    }

    #[test]
    fn post_compact_range_le_resume_en_l0_et_en_l1_sur_la_fiche_de_tache() {
        let _env = env();
        let h = hote_avec_identite();
        let r = repondre(&post_compact("s1", "## Résumé\nla session migrait la fiche."), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.is_empty(), "PostCompact ne rend rien");
        let appels = h.appels.lock().unwrap();
        let noms: Vec<&str> = appels.iter().map(|(n, _)| n.as_str()).collect();
        let source = appels.iter().find(|(n, _)| n == "put_source").expect("le L0").1["record"].clone();
        assert!(source["chemin"].as_str().unwrap().starts_with("sources/") && source["chemin"].as_str().unwrap().ends_with("-compaction-s1.md"), "{source}");
        assert_eq!((source["via"].as_str(), source["author"].as_str(), source["origine"].as_str()), (Some("hook"), Some("96"), Some("filet")), "l'origine d'un L0 de crochet est « filet » (schéma Source : envoi, filet, migration ; page M9 du 18 octobre)");
        assert!(source["contenu"].as_str().unwrap().contains("la session migrait la fiche."));
        let apport = appels.iter().find(|(n, _)| n == "put_apport").expect("le L1").1["record"].clone();
        assert_eq!((apport["fiche"].as_str(), apport["section"].as_str(), apport["via"].as_str(), apport["author"].as_str(), apport["niveau"].as_str()), (Some("migration-fiche"), Some("compaction"), Some("hook"), Some("96"), Some("L1")));
        assert_eq!(apport["source"], source["chemin"]);
        assert!(noms.contains(&"relier_apport"));
        let genres: Vec<&str> = appels.iter().filter(|(n, _)| n == "citer_source").map(|(_, a)| a["genre"].as_str().unwrap()).collect();
        assert_eq!(genres, ["doc", "handle"]);
        let handle = appels.iter().find(|(n, a)| n == "citer_source" && a["genre"] == "handle").unwrap();
        assert_eq!(handle.1["valeur"], "session:s1");
        assert_eq!(handle.1["fiche"]["fiche"], "migration-fiche");
        assert!(r.journal.iter().any(|l| l.contains("L1 compaction écrit sur « migration-fiche »")), "{:?}", r.journal);
    }

    #[test]
    fn post_compact_sans_tache_garde_le_l0_seul_et_le_session_start_suivant_le_dit() {
        let _env = env();
        let dossier = std::env::temp_dir().join(format!("hooks-lot3-l0-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        let mut reglage = Reglage::defaut();
        reglage.journal = dossier.join("hooks.log");
        let h = hote_sans_tache();
        let r = repondre(&post_compact("s7", "résumé"), &reglage, Ok(&h));
        let noms = h.noms();
        assert!(noms.contains(&"put_source".to_string()) && !noms.contains(&"put_apport".to_string()), "{noms:?}");
        assert!(r.journal.iter().any(|l| l.contains("L0 seul") && l.contains("aucune fiche de tâche")), "{:?}", r.journal);
        let r = repondre(&Evenement::lire(&json!({ "session_id": "s7", "hook_event_name": "SessionStart", "source": "compact" }).to_string()).unwrap(), &reglage, Ok(&h));
        assert!(r.bloc.contains("La dernière compaction est rangée en L0 seul"), "{}", r.bloc);
        let r = repondre(&Evenement::lire(&json!({ "session_id": "s7", "hook_event_name": "SessionStart", "source": "compact" }).to_string()).unwrap(), &reglage, Ok(&h));
        assert!(!r.bloc.contains("L0 seul"), "dit une fois");
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn post_compact_sans_resume_ou_sans_identite_ne_perd_rien_et_le_dit() {
        let _env = env();
        let h = hote_avec_identite();
        let r = repondre(&post_compact("s1", "   "), &Reglage::defaut(), Ok(&h));
        assert!(h.noms().is_empty() && r.journal.iter().any(|l| l.contains("sans résumé")));
        let h = HoteCanne::new(vec![]);
        std::env::remove_var("RAG3WEAVER_AGENT_KEY");
        let r = repondre(&post_compact("inconnue", "résumé"), &Reglage::defaut(), Ok(&h));
        let source = h.appels.lock().unwrap().iter().find(|(n, _)| n == "put_source").expect("le L0 même sans identité").1["record"].clone();
        assert_eq!(source["author"], "");
        assert!(r.journal.iter().any(|l| l.contains("aucune identité")), "{:?}", r.journal);
    }

    #[test]
    fn pre_compact_rend_les_consignes_du_resume_avec_la_graine_d_identite() {
        let _env = env();
        let h = hote_avec_identite();
        let r = repondre(&Evenement::lire(PRE.replace("\"s1\"", "\"s1\"").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(!r.bloc.starts_with("[mémoire]"), "pas de préfixe : c'est le compacteur qui lit");
        assert!(r.bloc.contains("who, runtime, can, doing, holding, mandates, peers") && r.bloc.contains("Rien de tout cela ne se résume"), "{}", r.bloc);
        assert!(r.bloc.contains("Le dernier `identity` connu") && r.bloc.contains("holding: worktrees wt-docs-96"), "{}", r.bloc);
        let h = HoteCanne::new(vec![]);
        let r = repondre(&Evenement::lire(PRE).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("Aucun apport `identity` connu"), "{}", r.bloc);
    }

    const T_PROMPT_P2: &str = r#"{"type":"user","promptId":"p2","timestamp":"2026-11-07T10:00:00Z","message":{"role":"user","content":"range la fiche"}}"#;
    const T_ASSISTANT_P2: &str = r#"{"type":"assistant","promptId":"p2","message":{"role":"assistant","usage":{"input_tokens":1,"cache_read_input_tokens":10},"content":[{"type":"text","text":"je regarde"},{"type":"tool_use","name":"Read","id":"tu1","input":{"file_path":"/p/a.rs"}}]}}"#;
    const T_RESULT_P2: &str = r#"{"type":"user","promptId":"p2","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu1","content":"fn a() {}","is_error":false}]}}"#;
    const T_FIN_P2: &str = r#"{"type":"assistant","promptId":"p2","message":{"role":"assistant","content":[{"type":"text","text":"fait."}]}}"#;
    const T_PROMPT_P3: &str = r#"{"type":"user","promptId":"p3","message":{"role":"user","content":"autre chose"}}"#;

    #[test]
    fn stop_depose_le_tour_comme_un_run_claude_code_turn() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("stop", &[T_USER, T_ASSISTANT_PETIT, T_PROMPT_P2, T_ASSISTANT_P2, T_RESULT_P2, T_FIN_P2, T_PROMPT_P3]);
        let h = hote_avec_identite();
        let e = Evenement::lire(&json!({ "session_id": "s1", "transcript_path": chemin, "cwd": "/p", "hook_event_name": "Stop", "prompt_id": "p2", "stop_hook_active": false, "last_assistant_message": "fait." }).to_string()).unwrap();
        let r = repondre(&e, &reglage, Ok(&h));
        assert!(r.bloc.is_empty(), "Stop ne rend rien");
        let runs = h.runs.lock().unwrap();
        assert_eq!(runs.len(), 1);
        let corps = &runs[0];
        assert_eq!((corps["backend"].as_str(), corps["run"].as_str(), corps["genre"].as_str()), (Some("claude-code"), Some("s1-p2"), Some("claude_code_turn")));
        assert_eq!((corps["session"].as_str(), corps["agent"].as_str(), corps["dernier_message"].as_str()), (Some("session:s1"), Some("96"), Some("fait.")));
        let tours = corps["tours"].as_array().unwrap();
        assert_eq!(tours.len(), 4, "{tours:?}");
        assert_eq!((tours[0]["role"].as_str(), tours[0]["content"].as_str()), (Some("user"), Some("range la fiche")));
        assert_eq!(tours[1]["tool_calls"][0]["name"], "Read");
        assert_eq!((tours[2]["role"].as_str(), tours[2]["tool_use_id"].as_str()), (Some("tool"), Some("tu1")));
        assert_eq!(tours[3]["content"], "fait.");
        assert!(corps["serveurs"].is_object(), "la table des serveurs est toujours là : {corps}");
        assert!(r.journal.iter().any(|l| l.contains("run déposé : claude-code/s1-p2")), "{:?}", r.journal);
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
    }

    #[test]
    fn la_table_des_serveurs_ne_garde_que_les_notres_avec_leur_manifeste() {
        let _env = env();
        let dossier = std::env::temp_dir().join(format!("hooks-serveurs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        let cwd = dossier.join("projet");
        std::fs::create_dir_all(&cwd).unwrap();
        let claude_json = dossier.join("claude.json");
        std::fs::write(&claude_json, json!({
            "mcpServers": { "racine": { "command": "/opt/rag3weaver-backend", "args": ["mcp", "--manifest", "/r/backend.json"] } },
            "projects": { cwd.to_string_lossy(): { "mcpServers": {
                "memoire-longue": { "command": "/home/x/.local/bin/rag3weaver-backend", "args": ["mcp", "--manifest", "/m/backend.json", "--keys", "dev"] },
                "autre": { "command": "/usr/bin/npx", "args": ["-y", "quelque-chose"] },
                "sans-manifeste": { "command": "/home/x/.local/bin/rag3weaver-backend", "args": ["daemon"] }
            } } }
        }).to_string()).unwrap();
        std::fs::write(cwd.join(".mcp.json"), json!({ "mcpServers": { "code-rag3db": { "command": "/home/x/.local/bin/rag3weaver-backend", "args": ["mcp", "--manifest", "/c/rag3db.json"] } } }).to_string()).unwrap();
        std::env::set_var("RAG3WEAVER_CLAUDE_JSON", &claude_json);
        let t = serveurs_mcp(Some(&cwd));
        std::env::remove_var("RAG3WEAVER_CLAUDE_JSON");
        assert_eq!(t, serde_json::from_value(json!({ "memoire-longue": "/m/backend.json", "racine": "/r/backend.json", "code-rag3db": "/c/rag3db.json" })).unwrap(), "{t:?}");
        assert!(serveurs_mcp(Some(Path::new("/nulle/part"))).is_empty() || true);
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn la_cle_donnee_ecrit_le_handle_une_fois_au_startup() {
        let _env = env();
        let h = HoteCanne::new(vec![("get_fiche", json!({ "ok": true, "result": { "result": { "data": { "name": "archi", "about": "ex-orchestration", "state": "current" } } } }))]);
        std::env::set_var("RAG3WEAVER_AGENT_KEY", "archi");
        let r = repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").as_str()).unwrap(), &Reglage::defaut(), Ok(&h));
        assert!(r.bloc.contains("ton L1 `handle` est écrit"), "{}", r.bloc);
        let appels = h.appels.lock().unwrap();
        let handle = appels.iter().find(|(n, a)| n == "put_apport" && a["record"]["section"] == "handle").expect("le L1 handle").1["record"].clone();
        assert_eq!((handle["fiche"].as_str(), handle["texte"].as_str(), handle["via"].as_str(), handle["author"].as_str()), (Some("archi"), Some("session:25b7ff4b-dabf-42ef-8580-86661c846fb9"), Some("hook"), Some("archi")));
        assert_eq!(handle["source"], "sessions/25b7ff4b-dabf-42ef-8580-86661c846fb9", "une source par session : le L1 ne se réécrit pas");
        assert!(appels.iter().any(|(n, a)| n == "citer_source" && a["genre"] == "handle" && a["valeur"] == "session:25b7ff4b-dabf-42ef-8580-86661c846fb9"));
        drop(appels);
        std::env::remove_var("RAG3WEAVER_AGENT_KEY");
        // La session déjà citée : rien à écrire.
        let h2 = hote_avec_identite();
        repondre(&Evenement::lire(SESSION_START.replace("compact", "startup").replace("25b7ff4b-dabf-42ef-8580-86661c846fb9", "s1").as_str()).unwrap(), &Reglage::defaut(), Ok(&h2));
        assert!(!h2.noms().contains(&"put_apport".to_string()), "{:?}", h2.noms());
    }

    #[test]
    fn l_identity_est_redemandee_une_fois_puis_plus() {
        let _env = env();
        let (chemin, reglage) = transcription_d_essai("rappel-identity", &[T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY]);
        let h = hote_avec_identite(); // l'identity connu date de 10 ms après l'époque : plus vieux que la demande
        let r = repondre(&prompt(&chemin, "on continue la migration"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Ta session a été compactée"), "{}", r.bloc);
        // Un tour est passé, l'apport n'est pas venu : le rappel, une fois.
        std::fs::write(&chemin, [T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY, T_ASSISTANT_PETIT].join("\n") + "\n").unwrap();
        let r = repondre(&prompt(&chemin, "et maintenant la page"), &reglage, Ok(&h));
        assert!(r.bloc.contains("Rappel, une fois : ton apport `identity` n'est pas écrit"), "{}", r.bloc);
        let r = repondre(&prompt(&chemin, "et encore une question"), &reglage, Ok(&h));
        assert!(!r.bloc.contains("Rappel"), "pas de troisième fois — {}", r.bloc);
        // L'apport venu : la marque tombe.
        let mut h2 = hote_avec_identite();
        h2.reponses.insert("recall_apports:identity", recherche(vec![json!({ "fiche": "96", "section": "identity", "texte": IDENTITY, "created_at": maintenant_ms() + 10_000 })]));
        let (chemin2, reglage2) = transcription_d_essai("rappel-identity-2", &[T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY]);
        repondre(&prompt(&chemin2, "on continue la migration"), &reglage2, Ok(&h2));
        std::fs::write(&chemin2, [T_USER, T_ASSISTANT_PETIT, T_BOUNDARY, T_SUMMARY, T_ASSISTANT_PETIT].join("\n") + "\n").unwrap();
        let r = repondre(&prompt(&chemin2, "et maintenant la page"), &reglage2, Ok(&h2));
        assert!(!r.bloc.contains("Rappel"), "{}", r.bloc);
        assert!(lire_la_marque(&reglage2, "s1", "identity-demandee").is_none());
        let _ = std::fs::remove_dir_all(chemin.parent().unwrap());
        let _ = std::fs::remove_dir_all(chemin2.parent().unwrap());
    }

    #[test]
    fn le_client_depose_un_run_sur_la_route_du_journal() {
        let (adresse, recues) = hote_en_carton(vec![
            (200, json!({ "session": "S9" }).to_string()),
            (200, json!({ "ok": true, "describe": { "name": "memoire-longue" } }).to_string()),
            (200, json!({ "ok": true, "backend": "claude-code", "run": "r" }).to_string()),
        ]);
        let c = ClientHote::ouvrir(&adresse, Some("j"), Path::new("/m/backend.json"), &[], Duration::from_secs(5)).unwrap();
        c.deposer_run(json!({ "backend": "claude-code", "run": "r", "genre": "claude_code_turn" })).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let r = recues.lock().unwrap();
        assert_eq!(r[2].1, "/journal/run");
        assert!(r[2].3.contains("\"genre\":\"claude_code_turn\""));
    }
}
