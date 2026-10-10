//! **Lancer une commande, et attendre qu'un journal dise quelque chose.**
//!
//! Le verbe qui manquait à l'agent de code. Le modèle l'avait mis en première
//! place, deux fois : *« je code à l'aveugle »*, *« un agent qui ne peut pas
//! tester ses modifications est un agent qui produit du code cassé »*.
//!
//! Tout ce qui décide vit ailleurs, et c'est voulu :
//!
//! - [`crate::commande`] tient la porte, les modes et le verdict ;
//! - `codeparsers::shell` réduit la ligne en argv, ou refuse en le nommant ;
//! - ici, on branche les deux et on rend le résultat.
//!
//! # Un refus est un résultat, pas une erreur
//!
//! Les trois décisions — autorisé, demandé, refusé — rendent un **résultat
//! d'outil** que l'agent lit. Une erreur de nœud arrêterait le graphe et
//! priverait l'agent de ce qu'il a besoin de savoir : *ce qui* a bloqué et
//! *pourquoi*. Un agent qui reçoit « refusé, parce que `rm` est irréversible »
//! change de plan ; un agent qui reçoit une erreur relance.

use std::sync::Arc;

use crate::commande::{executer, lancer_en_fond, Atelier, Contexte, Decision, EnFond, Fin, Garde};

use super::node::{Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};

/// Le service qui porte la porte : `Arc<Garde>`.
pub const GARDE_SERVICE: &str = "garde";
/// Le bac à sable du workspace, si le manifeste en déclare un : posé sur
/// chaque exécution. Voir [`crate::commande::BacASable`].
pub const BAC_A_SABLE_SERVICE: &str = "bac_a_sable";

/// Plafond dur, quoi que demande l'appelant. Un agent qui met une heure
/// bloque un fil pendant une heure.
const DELAI_MAX_S: u64 = 1_800;

/// `run` : exécute une ligne de commande, si la porte le permet.
pub struct RunCommandNode {
    node_name: String,
    ligne: String,
    delai_s: u64,
    /// Le délai a-t-il été donné ? En fond, sans délai donné, la commande vit
    /// le temps du run.
    delai_donne: bool,
    max_sortie: usize,
    /// En fond : rendre tout de suite la poignée et les journaux.
    fond: bool,
}

impl RunCommandNode {
    pub fn new(name: &str, ligne: impl Into<String>) -> Self {
        Self { node_name: name.to_string(), ligne: ligne.into(), delai_s: 60, delai_donne: false, max_sortie: 20_000, fond: false }
    }
    pub fn with_delai(mut self, s: u64) -> Self {
        self.delai_s = s.clamp(1, DELAI_MAX_S);
        self.delai_donne = true;
        self
    }
    /// **En fond** : la commande part, la poignée et les chemins des journaux
    /// sont rendus tout de suite ; `tail` lit ce qu'elle écrit.
    pub fn en_fond(mut self, oui: bool) -> Self {
        self.fond = oui;
        self
    }
    pub fn with_max_sortie(mut self, n: usize) -> Self {
        self.max_sortie = n.max(200);
        self
    }
}

/// Où vivent les journaux d'un run. **Pas tout `/tmp`** : seulement les siens.
///
/// Donner au modèle l'accès à `/tmp` entier lui donnerait les fichiers
/// temporaires de tous les autres processus de la machine. Un dossier par
/// exécution, et il ne lit que ce qu'il a produit.
///
/// **Sous le cache de la plateforme, pas dans `/tmp`** (10 octobre 2026) :
/// `/tmp` est souvent de la mémoire vive, et un journal de compilation y
/// pèse. `RAG3WEAVER_JOURNAUX_DIR` le place ailleurs. Au premier usage, les
/// dossiers de run de plus de [`JOURS_GARDES`] jours sont effacés.
pub fn dossier_journaux() -> std::path::PathBuf {
    static DOSSIER: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DOSSIER
        .get_or_init(|| {
            let d = std::env::var_os("RAG3WEAVER_JOURNAUX_DIR")
                .map(std::path::PathBuf::from)
                .or_else(|| crate::commande::dossier_de_cache().map(|c| c.join("rag3weaver").join("commandes")))
                .unwrap_or_else(|| std::env::temp_dir().join("rag3weaver-commandes"));
            menager(&d, jours_gardes());
            d
        })
        .clone()
}

/// Le dossier des journaux de ce run. Un run sans identifiant (un nœud
/// exécuté hors de la boucle, en test) prend un dossier par processus : les
/// poignées `cmd-N` repartent de 1 dans chaque processus, et deux processus
/// ne doivent pas écrire dans le même journal.
fn dossier_du_run(ctx: &NodeContext) -> std::path::PathBuf {
    let run = ctx.run_id();
    if run.is_empty() {
        dossier_journaux().join(format!("processus-{}", std::process::id()))
    } else {
        dossier_journaux().join(run)
    }
}

/// Les journaux d'un run sont gardés 7 jours (défaut confirmé le 10 octobre
/// 2026), `RAG3WEAVER_JOURNAUX_JOURS` pour un autre nombre.
pub const JOURS_GARDES: u64 = 7;

fn jours_gardes() -> u64 {
    std::env::var("RAG3WEAVER_JOURNAUX_JOURS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(JOURS_GARDES)
}

/// Effacer les dossiers de run plus vieux que `jours`.
fn menager(dossier: &std::path::Path, jours: u64) {
    let limite = std::time::Duration::from_secs(jours * 86_400);
    let Ok(entrees) = std::fs::read_dir(dossier) else { return };
    for e in entrees.flatten() {
        let vieux = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age > limite);
        if vieux && e.path().is_dir() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// **Les commandes en fond de ce processus**, par poignée : ce que `tail`
/// lit, et où il en était.
struct Suivie {
    en_fond: EnFond,
    ligne: String,
    /// Les octets déjà rendus par `tail since_last`, par flux.
    regard_out: std::sync::Mutex<u64>,
    regard_err: std::sync::Mutex<u64>,
}

fn suivies() -> &'static std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Suivie>>> {
    static S: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Suivie>>>> =
        std::sync::OnceLock::new();
    S.get_or_init(Default::default)
}

/// La poignée d'une commande en fond : celle de l'appel d'outil quand la
/// boucle la donne (`ctx.tool_handle()`, « #outil-N »), un compteur du
/// processus sinon. Le registre est du processus et la boucle numérote par
/// run : une poignée déjà prise par un autre run reçoit un suffixe.
fn poignee(ctx: &NodeContext) -> String {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let suivant = || N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let Some(appel) = ctx.tool_handle().map(|h| h.trim_start_matches('#').to_string()).filter(|h| !h.is_empty()) else {
        return format!("cmd-{}", suivant());
    };
    let prises = suivies().lock().unwrap_or_else(|p| p.into_inner());
    if prises.contains_key(&appel) {
        format!("{appel}-{}", suivant())
    } else {
        appel
    }
}

/// Ce qu'on dit de la fin d'une commande.
pub fn dire_la_fin(poignee: &str, f: &Fin) -> String {
    dire_la_fin_de(poignee, f, false)
}

/// `fin_du_run` : la mort vient de la portée du run qui se vide.
fn dire_la_fin_de(poignee: &str, f: &Fin, fin_du_run: bool) -> String {
    let comment = match (f.code, f.signal, f.tuee) {
        (_, _, true) if fin_du_run => "tuée à la fin du run".to_string(),
        (_, _, true) => "tuée".to_string(),
        (Some(c), _, _) => format!("code {c}"),
        (None, Some(s), _) => format!("tuée par le signal {s}"),
        (None, None, _) => "fin inconnue".to_string(),
    };
    let mut t = format!(
        "**`#{poignee}` a fini** : {comment} · {:.1?} · {} octets en sortie, {} en erreur.\n",
        f.duree, f.octets_stdout, f.octets_stderr
    );
    if !f.fin_stdout.trim().is_empty() {
        t.push_str(&format!("\nFin de la sortie :\n```\n{}\n```\n", f.fin_stdout.trim_end()));
    }
    if !f.fin_stderr.trim().is_empty() {
        t.push_str(&format!("\nFin des erreurs :\n```\n{}\n```\n", f.fin_stderr.trim_end()));
    }
    t
}

/// Vérifie qu'un chemin de journal reste **réellement** sous
/// [`dossier_journaux`], et pas seulement dans son texte.
///
/// `starts_with` compare composant par composant sans rien résoudre : il
/// laisse passer `<dossier>/../../etc/passwd` (le texte commence bien par le
/// dossier) et un lien symbolique posé sous le dossier qui pointe dehors. Deux
/// gardes, dans l'ordre : le texte (aucun `..`, le préfixe exigé), puis le
/// disque (le maillon existant le plus profond, canonisé, doit rester sous le
/// dossier canonisé — la canonisation résout toute la chaîne de liens d'un
/// coup). Un chemin dont rien n'existe encore ne détourne rien : il passe, et
/// l'attente dira « pas encore ».
fn journal_borne(journal: &str) -> Result<std::path::PathBuf, String> {
    let chemin = std::path::PathBuf::from(journal);
    let refus = || {
        format!(
            "`{journal}` n'est pas un journal de commande — on ne lit que ce qu'on a produit"
        )
    };
    if chemin.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(refus());
    }
    let base = dossier_journaux();
    if !chemin.starts_with(&base) {
        return Err(refus());
    }
    let base_canon = base.canonicalize().unwrap_or_else(|_| base.clone());
    let mut maillon = chemin.as_path();
    while maillon != base {
        if maillon.exists() {
            let canon = maillon.canonicalize().map_err(|e| format!("journal : {e}"))?;
            if !canon.starts_with(&base_canon) {
                return Err(refus());
            }
            break;
        }
        match maillon.parent() {
            Some(parent) => maillon = parent,
            None => break,
        }
    }
    Ok(chemin)
}

impl Node for RunCommandNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "RunCommandNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "command": self.ligne,
            "timeout_s": self.delai_s,
            "background": self.fond,
        })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::run_nodes::RunCommandNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // **Sans porte, on n'exécute rien.** Le défaut fermé n'est pas une
        // précaution : un montage qui oublie le service ne doit pas se
        // transformer en exécution libre.
        let garde = ctx
            .service::<Arc<Garde>>(GARDE_SERVICE)
            .cloned()
            .ok_or("run: le service 'garde' est absent — aucune commande ne s'exécute sans porte")?;

        let racine = racine_du_travail(ctx)
            .ok_or("run: pas de racine de travail (la source de fichiers est virtuelle)")?;

        let contexte = Contexte { accorde_par_l_utilisateur: false, domaine: Some(racine.clone()) };
        let verdict = match garde.juger_ligne(&self.ligne, &contexte) {
            Ok(v) => v,
            Err(refus) => {
                return rendre(ctx, format!("**Commande refusée.** {refus}"));
            }
        };

        if verdict.decision != Decision::Autorise {
            let mot = if verdict.decision == Decision::Refuse { "refusée" } else { "en attente" };
            let détail: Vec<String> = verdict
                .parties
                .iter()
                .map(|(c, v)| format!("- `{}` → {:?} : {}", c.lisible(), v.decision, v.motif))
                .collect();
            return rendre(
                ctx,
                format!(
                    "**Commande {mot}.** {}\n\n{}\n\n_Mode `{:?}`. Une commande refusée ne se \
                     relance pas telle quelle : changez-la, ou demandez à l'utilisateur._",
                    verdict.motif,
                    détail.join("\n"),
                    garde.mode()
                ),
            );
        }

        if self.fond {
            return self.lancer_en_fond(ctx, &garde, &contexte, &verdict.parties, &racine);
        }

        // Autorisée : chaque partie a son laissez-passer, on les exécute dans
        // l'ordre. `&&` s'arrête au premier échec, comme un shell le ferait.
        let mut atelier = Atelier::dans(&racine)
            .avec_delai(std::time::Duration::from_secs(self.delai_s))
            .avec_max_sortie(self.max_sortie)
            .avec_journaux(dossier_du_run(ctx));
        // Le bac à sable du manifeste, s'il y en a un : le noyau borne ce
        // que la commande voit — la garde refuse tôt et parle, le bac tient.
        if let Some(bac) = ctx.service::<Arc<crate::commande::BacASable>>(BAC_A_SABLE_SERVICE) {
            atelier = atelier.avec_bac_a_sable(bac.as_ref().clone());
        }

        let mut rapport = String::new();
        for (c, _) in &verdict.parties {
            let laissez = garde
                .autoriser(c, &contexte)
                .map_err(|v| format!("run: {} : {}", c.lisible(), v.motif))?;
            let s = executer(laissez, &atelier).map_err(|e| format!("run: {e}"))?;

            rapport.push_str(&format!(
                "### `{}`\n\ncode {} · {:.1?}{}\n\n",
                c.lisible(),
                s.code.map(|c| c.to_string()).unwrap_or_else(|| "tué".into()),
                s.duree,
                if s.expiree { format!(" · **délai de {} s dépassé**", self.delai_s) } else { String::new() }
            ));
            if !s.stdout.trim().is_empty() {
                rapport.push_str(&format!("```\n{}\n```\n\n", s.stdout.trim_end()));
            }
            if !s.stderr.trim().is_empty() {
                rapport.push_str(&format!("stderr :\n```\n{}\n```\n\n", s.stderr.trim_end()));
            }
            if let Some(j) = &s.journal_stdout {
                rapport.push_str(&format!(
                    "_Sortie entière : `{}` ({} octets). Lisible avec `read`, `grep`, ou une \
                     commande de lecture._\n\n",
                    j.display(),
                    s.octets_stdout
                ));
            }
            ctx.metric("exit_code", s.code.unwrap_or(-1) as f64);
            if !s.a_reussi() {
                rapport.push_str("_Arrêt : cette commande a échoué._\n");
                break;
            }
        }
        rendre(ctx, rapport)
    }
}

impl RunCommandNode {
    /// **En fond** : une seule commande (un enchaînement se lance par un
    /// script), partie dans son propre groupe ; la poignée et les journaux
    /// rendus tout de suite. Sa fin est rangée, et postée dans la boîte de
    /// l'agent quand la boucle donne son bus (`"agent_bus"`, `"agent_inbox"`).
    /// Sa mort est confiée à la portée du run (`"run_scope"`) : le run ne
    /// l'attend pas, il la tue en finissant.
    fn lancer_en_fond(
        &self,
        ctx: &mut NodeContext,
        garde: &Garde,
        contexte: &Contexte,
        parties: &[(crate::commande::Commande, crate::commande::Verdict)],
        racine: &std::path::Path,
    ) -> Result<(), String> {
        if parties.len() != 1 {
            return rendre(
                ctx,
                format!(
                    "**Commande refusée en fond.** En fond, une seule commande à la fois ({} ici) : \
                     lancez-les une par une, ou un script qui les enchaîne.",
                    parties.len()
                ),
            );
        }
        let (c, _) = &parties[0];
        let laissez = garde.autoriser(c, contexte).map_err(|v| format!("run: {} : {}", c.lisible(), v.motif))?;
        let poignee = poignee(ctx);
        let mut atelier = Atelier::dans(racine)
            .avec_max_sortie(self.max_sortie)
            .avec_journaux(dossier_du_run(ctx))
            .avec_nom_journaux(poignee.clone());
        if let Some(bac) = ctx.service::<Arc<crate::commande::BacASable>>(BAC_A_SABLE_SERVICE) {
            atelier = atelier.avec_bac_a_sable(bac.as_ref().clone());
        }
        let delai = self.delai_donne.then(|| std::time::Duration::from_secs(self.delai_s));
        // La fin, postée dans la boîte du run. Sans boîte (un graphe lancé
        // hors de la boucle), elle reste rangée et `tail` la dit.
        let par_la_portee = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let bus = ctx.service::<crate::events::EventBus>("agent_bus").cloned();
        let boite = ctx.service::<String>("agent_inbox").cloned();
        let a_la_fin: Box<dyn FnOnce(&Fin) + Send> = match (bus, boite) {
            (Some(bus), Some(boite)) => {
                let (nom, portee) = (poignee.clone(), par_la_portee.clone());
                Box::new(move |f: &Fin| {
                    let fin_du_run = portee.load(std::sync::atomic::Ordering::SeqCst);
                    bus.send_message(&boite, &format!("#{nom}"), &boite, &dire_la_fin_de(&nom, f, fin_du_run));
                })
            }
            _ => Box::new(|_| {}),
        };
        let en_fond = lancer_en_fond(laissez, &atelier, delai, a_la_fin).map_err(|e| format!("run: {e}"))?;
        let chemin = |p: &Option<std::path::PathBuf>| p.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        let texte = format!(
            "**En fond** : `#{poignee}` · `{}` (pid {}).\n\n- sortie : `{}`\n- erreurs : `{}`\n\n\
             _Lisez ce qu'elle écrit avec `tail` (la poignée, `lines` ou `since_last`) ; `wait` attend \
             une ligne précise. `tail` dit aussi si elle a fini, et avec quel code._\n",
            c.lisible(),
            en_fond.pid,
            chemin(&en_fond.journal_stdout),
            chemin(&en_fond.journal_stderr),
        );
        let suivie = Arc::new(Suivie { en_fond, ligne: c.lisible(), regard_out: Default::default(), regard_err: Default::default() });
        suivies().lock().unwrap_or_else(|p| p.into_inner()).insert(poignee, suivie.clone());
        // La mort, confiée à la portée du run. **Après** le registre, et en
        // tenant la commande elle-même : une portée déjà vidée (le run a fini
        // pendant que la commande démarrait) joue la fermeture ici, tout de
        // suite, et elle doit trouver de quoi tuer.
        if let Some(portee) = ctx.service::<Arc<crate::agent::RunScope>>("run_scope").cloned() {
            portee.defer(Box::new(move || {
                par_la_portee.store(true, std::sync::atomic::Ordering::SeqCst);
                suivie.en_fond.tuer(GRACE_EN_FIN_DE_RUN);
            }));
        }
        rendre(ctx, texte)
    }
}

/// Le délai entre SIGTERM et SIGKILL quand le run finit.
const GRACE_EN_FIN_DE_RUN: std::time::Duration = std::time::Duration::from_secs(5);

/// **Tuer une commande en fond** et tout son groupe (la fin du run).
pub fn tuer_en_fond(poignee: &str, grace: std::time::Duration) -> bool {
    let suivie = suivies().lock().unwrap_or_else(|p| p.into_inner()).get(poignee.trim_start_matches('#')).cloned();
    match suivie {
        Some(s) => {
            s.en_fond.tuer(grace);
            true
        }
        None => false,
    }
}

/// La fin d'une commande en fond, si elle est finie.
pub fn fin_en_fond(poignee: &str) -> Option<Fin> {
    suivies().lock().unwrap_or_else(|p| p.into_inner()).get(poignee.trim_start_matches('#')).and_then(|s| s.en_fond.fin())
}

fn rendre(ctx: &mut NodeContext, texte: String) -> Result<(), String> {
    ctx.set_output("result", PortValue::new(serde_json::Value::String(texte)));
    Ok(())
}

/// La racine de la source de fichiers — le seul répertoire où l'on exécute.
fn racine_du_travail(ctx: &mut NodeContext) -> Option<std::path::PathBuf> {
    let source = ctx.service::<Arc<dyn crate::code_tools::FileSource>>(
        crate::code_tools::FILE_SOURCE_SERVICE,
    )?;
    source.cursor().strip_prefix("worktree:").map(std::path::PathBuf::from)
}

// ─── Attendre qu'un journal dise quelque chose ───────────────────────────────

/// `wait` : attend qu'un motif apparaisse dans un journal, ou que le délai
/// passe.
///
/// **Ça appartient à la famille `run`, pas à `grep`.** `grep` cherche dans des
/// sources : le fichier est là, entier, et la réponse est immédiate. Un journal
/// est un **flux** : il grandit pendant qu'on le lit, et « pas encore » n'est
/// pas « non ». Confondre les deux ferait qu'un agent conclurait à l'absence en
/// regardant trop tôt.
pub struct WaitOutputNode {
    node_name: String,
    journal: String,
    motif: String,
    delai_s: u64,
}

impl WaitOutputNode {
    pub fn new(name: &str, journal: impl Into<String>, motif: impl Into<String>) -> Self {
        Self { node_name: name.to_string(), journal: journal.into(), motif: motif.into(), delai_s: 60 }
    }
    pub fn with_delai(mut self, s: u64) -> Self {
        self.delai_s = s.clamp(1, DELAI_MAX_S);
        self
    }
}

impl Node for WaitOutputNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "WaitOutputNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "journal": self.journal,
            "pattern": self.motif,
            "timeout_s": self.delai_s,
        })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::run_nodes::WaitOutputNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // **Seulement ses propres journaux.** Le reste de `/tmp` appartient aux
        // autres processus de la machine — `..` et liens symboliques compris.
        let chemin = journal_borne(&self.journal)?;
        let motif = regex::Regex::new(&self.motif)
            .map_err(|e| format!("wait: motif invalide : {e}"))?;

        let debut = std::time::Instant::now();
        let delai = std::time::Duration::from_secs(self.delai_s);
        loop {
            if let Ok(contenu) = std::fs::read_to_string(&chemin) {
                if let Some(ligne) = contenu.lines().find(|l| motif.is_match(l)) {
                    ctx.metric("attente_ms", debut.elapsed().as_millis() as f64);
                    return rendre(
                        ctx,
                        format!(
                            "**Trouvé** après {:.1?} :\n\n```\n{ligne}\n```\n",
                            debut.elapsed()
                        ),
                    );
                }
            }
            if debut.elapsed() >= delai {
                // **« Pas encore » n'est pas « non ».** Le dire est ce qui
                // distingue une attente d'une recherche.
                return rendre(
                    ctx,
                    format!(
                        "**Pas encore.** `{}` n'a rien qui corresponde à `{}` après {} s. \
                         La commande tourne peut-être toujours : réessayez, ou lisez le journal.\n",
                        chemin.display(),
                        self.motif,
                        self.delai_s
                    ),
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}

// ─── Lire la fin d'un journal ────────────────────────────────────────────────

/// `tail` : la fin du journal d'une commande, ou ce qui s'y est ajouté depuis
/// le dernier regard — et si la commande a fini, avec quel code.
///
/// À côté de `wait` : `wait` attend une ligne précise, `tail` regarde où en
/// est la commande sans rien attendre.
pub struct TailNode {
    node_name: String,
    /// La poignée (`#cmd-3`) ou un chemin de journal.
    cible: String,
    lignes: usize,
    depuis_le_dernier_regard: bool,
    erreurs: bool,
}

impl TailNode {
    pub fn new(name: &str, cible: impl Into<String>) -> Self {
        Self { node_name: name.to_string(), cible: cible.into(), lignes: 50, depuis_le_dernier_regard: false, erreurs: false }
    }
    pub fn lignes(mut self, n: usize) -> Self {
        self.lignes = n.clamp(1, 10_000);
        self
    }
    pub fn depuis_le_dernier_regard(mut self, oui: bool) -> Self {
        self.depuis_le_dernier_regard = oui;
        self
    }
    /// Lire le flux d'erreurs plutôt que la sortie.
    pub fn erreurs(mut self, oui: bool) -> Self {
        self.erreurs = oui;
        self
    }
}

impl Node for TailNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "TailNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::run_nodes::TailNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let poignee = self.cible.trim().trim_start_matches('#').to_string();
        let suivie = suivies().lock().unwrap_or_else(|p| p.into_inner()).get(&poignee).cloned();
        let (chemin, etat) = match &suivie {
            Some(s) => {
                let p = if self.erreurs { &s.en_fond.journal_stderr } else { &s.en_fond.journal_stdout };
                let p = p.clone().ok_or("tail: cette commande n'a pas de journal")?;
                let etat = match s.en_fond.fin() {
                    None => format!("`#{poignee}` (`{}`) **tourne encore**.", s.ligne),
                    Some(f) => dire_la_fin(&poignee, &f).lines().next().unwrap_or_default().to_string(),
                };
                (p, etat)
            }
            // Un chemin : seulement nos journaux, comme `wait`.
            None => (journal_borne(&self.cible)?, String::new()),
        };
        let contenu = std::fs::read(&chemin).unwrap_or_default();
        let mut note = String::new();
        let morceau: Vec<u8> = if self.depuis_le_dernier_regard {
            let regard = suivie.as_ref().map(|s| if self.erreurs { &s.regard_err } else { &s.regard_out });
            let mut vu = regard.map(|r| *r.lock().unwrap_or_else(|p| p.into_inner())).unwrap_or(0);
            if vu > contenu.len() as u64 {
                // Le journal a tourné (anneau) : on reprend à son début, qui
                // dit ce qui est perdu.
                note = "_Le journal a tourné depuis le dernier regard : reprise à son début._\n\n".into();
                vu = 0;
            }
            if let Some(r) = regard {
                *r.lock().unwrap_or_else(|p| p.into_inner()) = contenu.len() as u64;
            } else {
                note = "_`since_last` demande une poignée : voici la fin._\n\n".into();
            }
            if regard.is_some() { contenu[vu as usize..].to_vec() } else { contenu.clone() }
        } else {
            contenu.clone()
        };
        let texte_brut = String::from_utf8_lossy(&morceau);
        let lignes: Vec<&str> = texte_brut.lines().collect();
        let n = if self.depuis_le_dernier_regard && suivie.is_some() { lignes.len().min(10_000) } else { self.lignes };
        let rendues = &lignes[lignes.len().saturating_sub(n)..];
        let mut t = String::new();
        if !etat.is_empty() {
            t.push_str(&format!("{etat}\n\n"));
        }
        t.push_str(&note);
        if rendues.is_empty() {
            t.push_str(if self.depuis_le_dernier_regard { "_Rien de neuf._\n" } else { "_Journal vide._\n" });
        } else {
            let tete = if lignes.len() > rendues.len() {
                format!("_{} dernières lignes sur {}, `{}`._\n", rendues.len(), lignes.len(), chemin.display())
            } else {
                format!("_`{}`._\n", chemin.display())
            };
            t.push_str(&tete);
            t.push_str(&format!("```\n{}\n```\n", rendues.join("\n")));
        }
        rendre(ctx, t)
    }
}

// ─── Fabriques ───────────────────────────────────────────────────────────────

use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};

/// Fabrique de [`RunCommandNode`].
pub struct RunCommandNodeFactory;

impl NodeFactory for RunCommandNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let ligne = config
            .get("command")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("RunCommandNode: 'command' est obligatoire")?;
        let mut node = RunCommandNode::new(name, ligne);
        if let Some(t) = config.get("timeout_s").and_then(|v| v.as_u64()) {
            node = node.with_delai(t);
        }
        if let Some(m) = config.get("max_output").and_then(|v| v.as_u64()) {
            node = node.with_max_sortie(m as usize);
        }
        if let Some(b) = config.get("background").and_then(|v| v.as_bool()) {
            node = node.en_fond(b);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "RunCommandNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "RunCommandNode",
            description: "Exécute une ligne de commande dans la racine du projet, si la porte le permet.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "command",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "La ligne à exécuter (ex. 'cargo test --lib'). Enchaînements && ; || et tuyaux acceptés ; substitutions, redirections et jokers refusés.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "timeout_s",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(60)),
                    description: "Secondes avant de tuer (plafond 1800). Au-delà, préférez la variante de fond.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "max_output",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(20_000)),
                    description: "Caractères rendus par flux. Le reste n'est pas perdu : il est dans le journal, dont le chemin est donné.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "background",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "En fond : rend tout de suite la poignée et les journaux ; `tail` lit la suite. Sans plafond de durée, sauf timeout_s donné.",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

/// Fabrique de [`TailNode`].
pub struct TailNodeFactory;

impl NodeFactory for TailNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let cible = config
            .get("handle")
            .or_else(|| config.get("journal"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("TailNode: 'handle' (ou 'journal') est obligatoire")?;
        let mut node = TailNode::new(name, cible);
        if let Some(n) = config.get("lines").and_then(|v| v.as_u64()) {
            node = node.lignes(n as usize);
        }
        if let Some(b) = config.get("since_last").and_then(|v| v.as_bool()) {
            node = node.depuis_le_dernier_regard(b);
        }
        if config.get("stream").and_then(|v| v.as_str()) == Some("err") {
            node = node.erreurs(true);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "TailNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "TailNode",
            description: "La fin du journal d'une commande, ou ce qui s'y est ajouté depuis le dernier regard, et son état.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "handle",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "La poignée rendue par run en fond (ex. '#cmd-3'), ou un chemin de journal.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "lines",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(50)),
                    description: "Combien de dernières lignes rendre.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "since_last",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "Seulement ce qui s'est ajouté depuis le dernier tail de cette poignée.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "stream",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("out")),
                    description: "'out' (la sortie) ou 'err' (les erreurs).",
                    choices: Some(super::node_registry::Choices::fixed(["out", "err"])),
                    json_schema: None,
                },
            ],
        }
    }
}

/// Fabrique de [`WaitOutputNode`].
pub struct WaitOutputNodeFactory;

impl NodeFactory for WaitOutputNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let journal = config
            .get("journal")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("WaitOutputNode: 'journal' est obligatoire")?;
        let motif = config
            .get("pattern")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("WaitOutputNode: 'pattern' est obligatoire")?;
        let mut node = WaitOutputNode::new(name, journal, motif);
        if let Some(t) = config.get("timeout_s").and_then(|v| v.as_u64()) {
            node = node.with_delai(t);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "WaitOutputNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "WaitOutputNode",
            description: "Attend qu'un motif apparaisse dans le journal d'une commande, ou que le délai passe.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "journal",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Le chemin de journal rendu par une commande précédente.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "pattern",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Expression régulière cherchée ligne par ligne.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "timeout_s",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(60)),
                    description: "Secondes d'attente avant de rendre « pas encore » (plafond 1800).",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_tools::{FileSource, WorkingTree, FILE_SOURCE_SERVICE};
    use crate::commande::Mode;
    use crate::dataflow::services::ServiceRegistry;
    use std::io::Write as _;
    use std::sync::Arc;

    // **Rien de dangereux ne s'exécute ici.** Les seules commandes réellement
    // lancées sont `pwd`, `ls` et `tail` — en lecture seule. Les commandes
    // destructrices n'apparaissent que dans les cas de refus, qui s'arrêtent
    // à la porte : `executer` ne prend qu'une `Autorisee`, et un refus n'en
    // produit pas.

    fn contexte(mode: Mode, racine: &std::path::Path) -> NodeContext {
        let mut services = ServiceRegistry::new();
        services.register(GARDE_SERVICE, Arc::new(Garde::new(mode)));
        let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(racine));
        services.register(FILE_SOURCE_SERVICE, source);
        NodeContext::with_services(Arc::new(services))
    }

    /// Le contexte que la boucle pose pour un appel d'outil : la poignée, le
    /// bus et la boîte du run, la portée du run.
    fn contexte_de_boucle(
        racine: &std::path::Path,
        bus: &crate::events::EventBus,
        boite: &str,
        portee: &Arc<crate::agent::RunScope>,
        appel: &str,
    ) -> NodeContext {
        let mut services = ServiceRegistry::new();
        services.register(GARDE_SERVICE, Arc::new(Garde::new(Mode::Auto)));
        let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(racine));
        services.register(FILE_SOURCE_SERVICE, source);
        services.register("tool_handle", appel.to_string());
        services.register("agent_bus", bus.clone());
        services.register("agent_inbox", boite.to_string());
        services.register("run_scope", portee.clone());
        NodeContext::with_services(Arc::new(services))
    }

    /// Les messages de la boîte, attendus jusqu'à en avoir `n` (5 s au plus).
    fn messages(rx: &mut async_broadcast::Receiver<crate::events::Event>, n: usize) -> Vec<(String, String)> {
        let mut dits = Vec::new();
        let debut = std::time::Instant::now();
        while dits.len() < n && debut.elapsed() < std::time::Duration::from_secs(5) {
            match rx.try_recv() {
                Ok(crate::events::Event::Message { from, content, .. }) => dits.push((from, content)),
                Ok(_) => {}
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        dits
    }

    fn texte(ctx: &mut NodeContext) -> String {
        ctx.drain_outputs()
            .remove("result")
            .and_then(super::super::port::take_or_clone::<serde_json::Value>)
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    }

    /// **Une commande en lecture seule passe et rend son résultat.** C'est la
    /// boucle que le modèle réclamait : lancer, lire, décider.
    #[test]
    fn une_commande_de_lecture_s_execute_et_rapporte() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        RunCommandNode::new("run", "pwd").execute(&mut ctx).expect("exécution");
        let t = texte(&mut ctx);
        assert!(t.contains("code 0"), "{t}");
        assert!(t.contains("`pwd`"), "{t}");
        // Le journal est nommé, pour qu'on puisse y revenir sans relancer.
        assert!(t.contains("Sortie entière"), "{t}");
    }

    /// **Un refus est un résultat, pas une erreur.** Le graphe ne s'arrête pas,
    /// et l'agent apprend *ce qui* a bloqué — sans quoi il relancerait.
    #[test]
    fn une_commande_destructrice_est_refusee_sans_rien_executer() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        RunCommandNode::new("run", "pwd && rm -rf /")
            .execute(&mut ctx)
            .expect("un refus n'est pas une erreur de nœud");
        let t = texte(&mut ctx);
        assert!(t.contains("refusée") || t.contains("attente"), "{t}");
        assert!(t.contains("rm"), "le refus doit nommer le coupable : {t}");
        assert!(!t.contains("code 0"), "rien ne doit s'être exécuté : {t}");
    }

    /// Ce qu'on n'a pas su réduire est refusé avec sa raison.
    #[test]
    fn une_ligne_non_reductible_est_refusee_avec_sa_raison() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        RunCommandNode::new("run", "cat $(cat cible)").execute(&mut ctx).unwrap();
        let t = texte(&mut ctx);
        assert!(t.contains("substitution"), "{t}");
    }

    /// **Sans porte, rien.** Un montage qui oublie le service ne doit pas se
    /// transformer en exécution libre : c'est une erreur franche.
    #[test]
    fn sans_garde_aucune_execution() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut services = ServiceRegistry::new();
        let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(dossier.path()));
        services.register(FILE_SOURCE_SERVICE, source);
        let mut ctx = NodeContext::with_services(Arc::new(services));
        let e = RunCommandNode::new("run", "pwd").execute(&mut ctx).expect_err("pas de porte");
        assert!(e.contains("garde"), "{e}");
    }

    /// **On n'attend que sur ses propres journaux.** Le reste du dossier
    /// temporaire appartient aux autres programmes de la machine.
    #[test]
    fn on_n_attend_pas_sur_un_fichier_qui_n_est_pas_a_nous() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        let etranger = dossier.path().join("pas-a-nous.log");
        std::fs::write(&etranger, "peu importe").unwrap();
        let e = WaitOutputNode::new("wait", etranger.to_string_lossy(), "x")
            .execute(&mut ctx)
            .expect_err("hors de nos journaux");
        assert!(e.contains("journal de commande"), "{e}");
    }

    /// **Un `..` dans le chemin est un mensonge sur la destination.** Le texte
    /// commence sous nos journaux, la cible est ailleurs : on refuse avant de
    /// lire quoi que ce soit.
    #[test]
    fn on_n_attend_pas_a_travers_des_points_points() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        // Un fichier à nous, hors du dossier des journaux, atteint en le
        // traversant : même contrôlé, il doit rester hors de portée.
        let secret = dossier.path().join("secret.out");
        std::fs::write(&secret, "fini\n").unwrap();
        let traverse = dossier_journaux().join("essai-traverse").join("..").join("..").join(
            secret.strip_prefix(std::env::temp_dir()).expect("sous temp_dir"),
        );
        let e = WaitOutputNode::new("wait", traverse.to_string_lossy(), "fini")
            .with_delai(1)
            .execute(&mut ctx)
            .expect_err("une traversée n'est pas un journal");
        assert!(e.contains("journal de commande"), "{e}");
    }

    /// **Un lien symbolique sous le dossier peut pointer dehors.** Le texte du
    /// chemin reste sous nos journaux, le disque dit autre chose : c'est le
    /// disque qui compte.
    #[cfg(unix)]
    #[test]
    fn on_n_attend_pas_a_travers_un_lien_qui_sort() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        let secret = dossier.path().join("secret.out");
        std::fs::write(&secret, "fini\n").unwrap();
        let repaire = dossier_journaux().join("essai-lien");
        std::fs::create_dir_all(&repaire).unwrap();
        let lien = repaire.join("sortie");
        let _ = std::fs::remove_file(&lien);
        std::os::unix::fs::symlink(dossier.path(), &lien).unwrap();
        let e = WaitOutputNode::new("wait", lien.join("secret.out").to_string_lossy(), "fini")
            .with_delai(1)
            .execute(&mut ctx)
            .expect_err("un lien qui sort n'est pas un journal");
        assert!(e.contains("journal de commande"), "{e}");
        let _ = std::fs::remove_dir_all(&repaire);
    }

    /// **« Pas encore » n'est pas « non ».** C'est ce qui distingue une attente
    /// d'une recherche : la commande tourne peut-être toujours.
    #[test]
    fn une_attente_qui_expire_dit_pas_encore() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        let journal = dossier_journaux().join("essai").join("rien.out");
        std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
        std::fs::write(&journal, "une ligne sans rapport\n").unwrap();
        WaitOutputNode::new("wait", journal.to_string_lossy(), "introuvable")
            .with_delai(1)
            .execute(&mut ctx)
            .expect("une attente qui expire n'est pas une erreur");
        let t = texte(&mut ctx);
        assert!(t.contains("Pas encore"), "{t}");
        let _ = std::fs::remove_file(&journal);
    }

    // ── En fond, et `tail` (10 octobre 2026) ────────────────────────────

    fn poignee_de(t: &str) -> String {
        let i = t.find("**En fond** : `#").expect("une poignée") + "**En fond** : `".len();
        t[i..].split('`').next().unwrap().to_string()
    }

    /// **`run` en fond rend tout de suite la poignée et des journaux qui
    /// existent ; `tail` lit les dernières lignes, puis seulement le neuf.**
    /// La commande est `tail -f` sur un fichier qu'on fait grandir : elle ne
    /// finit jamais d'elle-même, et la garde la laisse passer.
    #[test]
    fn en_fond_la_main_tout_de_suite_puis_tail_lit_le_neuf() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let suivi = dossier.path().join("suivi.log");
        std::fs::write(&suivi, "l1\nl2\nl3\nl4\n").unwrap();
        let mut ctx = contexte(Mode::Auto, dossier.path());
        let t0 = std::time::Instant::now();
        RunCommandNode::new("run", format!("tail -f -n +1 {}", suivi.display()))
            .en_fond(true)
            .execute(&mut ctx)
            .expect("lancée");
        assert!(t0.elapsed() < std::time::Duration::from_secs(1), "la main tout de suite : {:?}", t0.elapsed());
        let t = texte(&mut ctx);
        assert!(t.contains("**En fond**"), "{t}");
        let p = poignee_de(&t);
        // Attendre que la commande ait recopié les quatre lignes.
        let lire = |ctx: &mut NodeContext, node: TailNode| {
            let mut node = node;
            node.execute(ctx).expect("tail");
            texte(ctx)
        };
        let debut = std::time::Instant::now();
        loop {
            let t = lire(&mut ctx, TailNode::new("tail", &p).lignes(3));
            if t.contains("l4") {
                assert!(t.contains("l2") && !t.contains("l1"), "les trois dernières : {t}");
                assert!(t.contains("tourne encore"), "{t}");
                break;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(5), "rien recopié : {t}");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let t = lire(&mut ctx, TailNode::new("tail", &p).depuis_le_dernier_regard(true));
        assert!(t.contains("l1") && t.contains("l4"), "le premier regard rend tout : {t}");
        let t = lire(&mut ctx, TailNode::new("tail", &p).depuis_le_dernier_regard(true));
        assert!(t.contains("Rien de neuf"), "{t}");
        std::fs::OpenOptions::new().append(true).open(&suivi).unwrap().write_all(b"l5\n").unwrap();
        let debut = std::time::Instant::now();
        loop {
            let t = lire(&mut ctx, TailNode::new("tail", &p).depuis_le_dernier_regard(true));
            if t.contains("l5") {
                assert!(!t.contains("l4"), "seulement le neuf : {t}");
                break;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(5), "l5 n'arrive pas : {t}");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(tuer_en_fond(&p, std::time::Duration::from_secs(2)), "la poignée est connue");
        let t = lire(&mut ctx, TailNode::new("tail", &p).lignes(1));
        assert!(t.contains("a fini") && t.contains("tuée"), "{t}");
    }

    /// **La fin arrive dans la boîte**, sous la poignée de l'appel, avec son
    /// code : 0, puis 2, puis tuée.
    #[test]
    fn la_fin_arrive_dans_la_boite_avec_son_code() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let suivi = dossier.path().join("suivi.log");
        std::fs::write(&suivi, "l1\n").unwrap();
        let bus = crate::events::EventBus::new(64);
        let boite = "run-boite";
        let mut rx = bus.subscribe(&crate::events::inbox_topic(boite));
        let portee = Arc::new(crate::agent::RunScope::default());
        let lancer = |appel: &str, ligne: String| {
            let mut ctx = contexte_de_boucle(dossier.path(), &bus, boite, &portee, appel);
            RunCommandNode::new("run", ligne).en_fond(true).execute(&mut ctx).expect("lancée");
            poignee_de(&texte(&mut ctx))
        };
        let ok = lancer("#essai-boite-1", "ls".into());
        assert_eq!(ok, "#essai-boite-1", "la poignée est celle de l'appel");
        let dits = messages(&mut rx, 1);
        assert!(dits.len() == 1 && dits[0].0 == "#essai-boite-1" && dits[0].1.contains("code 0"), "{dits:?}");
        lancer("#essai-boite-2", "ls n-existe-pas-du-tout".into());
        let dits = messages(&mut rx, 1);
        assert!(dits.len() == 1 && dits[0].1.contains("code 2"), "{dits:?}");
        let long = lancer("#essai-boite-3", format!("tail -f {}", suivi.display()));
        assert!(tuer_en_fond(&long, std::time::Duration::from_secs(2)));
        let dits = messages(&mut rx, 1);
        assert!(dits.len() == 1 && dits[0].1.contains("tuée") && !dits[0].1.contains("fin du run"), "{dits:?}");
    }

    /// **Le run fini tue la commande** : la portée vidée l'arrête, son
    /// processus n'existe plus, et la boîte reçoit « tuée à la fin du run ».
    #[test]
    fn la_portee_videe_tue_la_commande_et_le_dit() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let suivi = dossier.path().join("suivi.log");
        std::fs::write(&suivi, "l1\n").unwrap();
        let bus = crate::events::EventBus::new(64);
        let boite = "run-portee";
        let mut rx = bus.subscribe(&crate::events::inbox_topic(boite));
        let portee = Arc::new(crate::agent::RunScope::default());
        let mut ctx = contexte_de_boucle(dossier.path(), &bus, boite, &portee, "#essai-portee-1");
        RunCommandNode::new("run", format!("tail -f {}", suivi.display())).en_fond(true).execute(&mut ctx).unwrap();
        let t = texte(&mut ctx);
        let pid: u32 = t.split("(pid ").nth(1).and_then(|r| r.split(')').next()).and_then(|p| p.parse().ok()).expect("le pid");
        assert!(messages(&mut rx, 1).is_empty(), "elle tourne : rien dans la boîte");
        let t0 = std::time::Instant::now();
        portee.vider();
        println!("▸ portée vidée en {:?}", t0.elapsed());
        let dits = messages(&mut rx, 1);
        assert!(dits.len() == 1 && dits[0].1.contains("tuée à la fin du run"), "{dits:?}");
        let vivant = std::process::Command::new("kill").args(["-0", &pid.to_string()]).status().unwrap().success();
        assert!(!vivant, "le processus {pid} est mort");
        portee.vider();
        assert!(messages(&mut rx, 1).is_empty(), "une seconde vidange ne redit rien");
    }

    /// **Un run qui finit pendant que la commande démarre** : la portée est
    /// déjà vidée quand la commande y enregistre sa mort ; la fermeture joue
    /// aussitôt (portée scellée, session recherche), le processus ne survit
    /// pas au run, et la boîte reçoit quand même « tuée à la fin du run ».
    #[test]
    fn une_commande_lancee_apres_la_fin_du_run_est_tuee_aussitot() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let suivi = dossier.path().join("suivi.log");
        std::fs::write(&suivi, "l1\n").unwrap();
        let bus = crate::events::EventBus::new(64);
        let boite = "run-tard";
        let mut rx = bus.subscribe(&crate::events::inbox_topic(boite));
        let portee = Arc::new(crate::agent::RunScope::default());
        portee.vider();
        let mut ctx = contexte_de_boucle(dossier.path(), &bus, boite, &portee, "#essai-tard-1");
        RunCommandNode::new("run", format!("tail -f {}", suivi.display())).en_fond(true).execute(&mut ctx).unwrap();
        let t = texte(&mut ctx);
        let pid: u32 = t.split("(pid ").nth(1).and_then(|r| r.split(')').next()).and_then(|p| p.parse().ok()).expect("le pid");
        let dits = messages(&mut rx, 1);
        assert!(dits.len() == 1 && dits[0].1.contains("tuée à la fin du run"), "{dits:?}");
        let vivant = std::process::Command::new("kill").args(["-0", &pid.to_string()]).status().unwrap().success();
        assert!(!vivant, "le processus {pid} n'a pas survécu au run");
    }

    /// **Deux commandes du même programme dans le même run** : deux poignées,
    /// deux journaux, deux fins livrées. Une poignée déjà prise reçoit un
    /// suffixe.
    #[test]
    fn deux_commandes_du_meme_programme_ont_deux_journaux_et_deux_fins() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let bus = crate::events::EventBus::new(64);
        let boite = "run-deux";
        let mut rx = bus.subscribe(&crate::events::inbox_topic(boite));
        let portee = Arc::new(crate::agent::RunScope::default());
        let mut journaux = Vec::new();
        let mut poignees = Vec::new();
        for appel in ["#essai-deux-1", "#essai-deux-2", "#essai-deux-1"] {
            let mut ctx = contexte_de_boucle(dossier.path(), &bus, boite, &portee, appel);
            RunCommandNode::new("run", "ls").en_fond(true).execute(&mut ctx).unwrap();
            let t = texte(&mut ctx);
            poignees.push(poignee_de(&t).trim_start_matches('#').to_string());
            let sortie = t.split("- sortie : `").nth(1).and_then(|r| r.split('`').next()).unwrap_or_default().to_string();
            journaux.push(sortie);
        }
        assert_eq!(&poignees[..2], ["essai-deux-1", "essai-deux-2"]);
        assert!(poignees[2].starts_with("essai-deux-1-"), "la poignée reprise reçoit un suffixe : {poignees:?}");
        let distincts: std::collections::HashSet<&String> = journaux.iter().collect();
        assert_eq!(distincts.len(), 3, "trois journaux : {journaux:?}");
        let dits = messages(&mut rx, 3);
        let de: std::collections::HashSet<String> = dits.iter().map(|(f, _)| f.trim_start_matches('#').to_string()).collect();
        assert_eq!(de, poignees.iter().cloned().collect(), "les trois fins livrées : {dits:?}");
    }

    /// **`tail` dit la fin et son code.**
    #[test]
    fn tail_dit_la_fin_et_son_code() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        RunCommandNode::new("run", "ls n-existe-pas-du-tout").en_fond(true).execute(&mut ctx).unwrap();
        let p = poignee_de(&texte(&mut ctx));
        let debut = std::time::Instant::now();
        while fin_en_fond(&p).is_none() {
            assert!(debut.elapsed() < std::time::Duration::from_secs(5), "pas de fin");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        TailNode::new("tail", &p).erreurs(true).execute(&mut ctx).unwrap();
        let t = texte(&mut ctx);
        assert!(t.contains("a fini") && t.contains("code 2"), "{t}");
        assert!(t.contains("n-existe-pas-du-tout"), "le message d'erreur de ls : {t}");
    }

    /// En fond, un enchaînement est refusé avec sa raison : une seule commande.
    #[test]
    fn en_fond_un_enchainement_est_refuse_avec_sa_raison() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        RunCommandNode::new("run", "pwd && pwd").en_fond(true).execute(&mut ctx).unwrap();
        let t = texte(&mut ctx);
        assert!(t.contains("refusée en fond") && t.contains("une seule commande"), "{t}");
    }

    /// **`tail` ne lit que nos journaux.**
    #[test]
    fn tail_ne_lit_pas_un_fichier_qui_n_est_pas_a_nous() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        let etranger = dossier.path().join("pas-a-nous.log");
        std::fs::write(&etranger, "secret").unwrap();
        let e = TailNode::new("tail", etranger.to_string_lossy()).execute(&mut ctx).expect_err("hors de nos journaux");
        assert!(e.contains("journal de commande"), "{e}");
    }

    #[test]
    fn une_attente_trouve_ce_qui_est_deja_la() {
        let dossier = tempfile::tempdir().expect("tempdir");
        let mut ctx = contexte(Mode::Auto, dossier.path());
        let journal = dossier_journaux().join("essai2").join("plein.out");
        std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
        std::fs::write(&journal, "démarrage\nListening on 7878\n").unwrap();
        WaitOutputNode::new("wait", journal.to_string_lossy(), "Listening on \\d+")
            .with_delai(5)
            .execute(&mut ctx)
            .expect("trouvé");
        let t = texte(&mut ctx);
        assert!(t.contains("Trouvé"), "{t}");
        assert!(t.contains("7878"), "{t}");
        let _ = std::fs::remove_file(&journal);
    }
}
