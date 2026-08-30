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

use crate::commande::{executer, Atelier, Contexte, Decision, Garde};

use super::node::{Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};

/// Le service qui porte la porte : `Arc<Garde>`.
pub const GARDE_SERVICE: &str = "garde";

/// Plafond dur, quoi que demande l'appelant. Un agent qui met une heure
/// bloque un fil pendant une heure.
const DELAI_MAX_S: u64 = 1_800;

/// `run` : exécute une ligne de commande, si la porte le permet.
pub struct RunCommandNode {
    node_name: String,
    ligne: String,
    delai_s: u64,
    max_sortie: usize,
}

impl RunCommandNode {
    pub fn new(name: &str, ligne: impl Into<String>) -> Self {
        Self { node_name: name.to_string(), ligne: ligne.into(), delai_s: 60, max_sortie: 20_000 }
    }
    pub fn with_delai(mut self, s: u64) -> Self {
        self.delai_s = s.clamp(1, DELAI_MAX_S);
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
pub fn dossier_journaux() -> std::path::PathBuf {
    std::env::temp_dir().join("rag3weaver-commandes")
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
        })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
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

        // Autorisée : chaque partie a son laissez-passer, on les exécute dans
        // l'ordre. `&&` s'arrête au premier échec, comme un shell le ferait.
        let atelier = Atelier::dans(&racine)
            .avec_delai(std::time::Duration::from_secs(self.delai_s))
            .avec_max_sortie(self.max_sortie)
            .avec_journaux(dossier_journaux().join(ctx.run_id()));

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
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let chemin = std::path::PathBuf::from(&self.journal);
        // **Seulement ses propres journaux.** Le reste de `/tmp` appartient aux
        // autres processus de la machine.
        if !chemin.starts_with(dossier_journaux()) {
            return Err(format!(
                "wait: `{}` n'est pas un journal de commande — on n'attend que sur ce qu'on a produit",
                chemin.display()
            ));
        }
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
    use std::sync::Arc;

    // **Rien de dangereux ne s'exécute ici.** Les seules commandes réellement
    // lancées sont `pwd` — en lecture seule et sans argument. Les commandes
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
