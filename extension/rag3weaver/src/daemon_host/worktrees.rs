//! **L'atelier : l'hôte attache les worktrees** (page
//! 07-novembre-2026-everything-declarative/03, lot 0 ; Lucie, 8 novembre 2026 :
//! « l'atelier oui absolument »). `attach_worktree` inscrit un worktree déjà sur le
//! disque depuis le gabarit de code livré (comme `code-tree add`), l'ouvre, lance son
//! premier index en fond et le publie sur le flux (`repos`, `RepoAttached`) ;
//! `list_worktrees` dit les worktrees inscrits. Aucune commande git n'est lancée, rien
//! n'est cloné. Les phrases parlent de dépôt, de worktree, de branche et d'index.
//!
//! **La fin du premier index se dit** (Lucie, 8 novembre 2026 à 16 h 08 : « il devrait
//! avoir notif quand indexation finie ? ») : un suiveur lit le journal de l'index jusqu'à
//! sa dernière ligne et publie `RepoIndexed` sur `repos` — fini (avec le reste dû des
//! vecteurs, s'il y en a), en échec ou arrêté —, avec les fichiers et les symboles de
//! l'index ; la page en tire une ligne dans le chat de l'atelier et le badge de la barre.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::DaemonHost;
use crate::dataflow::worktree_nodes::{WorktreeAnswer, WorktreeHost};

/// L'hôte des worktrees que l'hôte pose sur chaque backend qu'il ouvre.
pub struct HoteDesWorktrees {
    pub hote: Weak<DaemonHost>,
}

impl HoteDesWorktrees {
    fn hote(&self) -> Result<Arc<DaemonHost>, String> {
        self.hote.upgrade().ok_or_else(|| "l'hôte de rag3weaver s'arrête : réessayez quand il sera revenu".to_string())
    }
}

/// Le gabarit de code livré, copié sous un dossier `templates/` à soi : `code-tree
/// add` prend son gabarit sous un tel dossier. Rendu avec le dossier à effacer.
fn gabarit_livre(sous: &Path) -> Result<(PathBuf, PathBuf), String> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dossier = sous.join(format!(".atelier-gabarit-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let templates = dossier.join("templates");
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    crate::code_tree::copy(&crate::builtin_templates::root()?, &templates, false)?;
    Ok((templates.join("backends/code/backend.json"), dossier))
}

/// Les bornes d'un premier index suivi : `plafond`, au-delà duquel la fin se dit en
/// échec (le journal n'a pas fini) plutôt que de suivre sans fin ; `naissance`, le temps
/// laissé au journal pour paraître (une passe morte avant d'écrire, des journaux
/// nettoyés : l'échec se dit tout de suite après, pas au plafond).
#[derive(Clone, Copy)]
pub(super) struct Bornes {
    pub plafond: Duration,
    pub naissance: Duration,
}

/// Les bornes du service.
pub(super) const BORNES_DU_SUIVI: Bornes = Bornes { plafond: Duration::from_secs(6 * 3600), naissance: Duration::from_secs(60) };

/// Ce qu'on garde de la fin du journal pour y lire sa dernière ligne.
const QUEUE_DU_JOURNAL: usize = 4096;

/// **La lecture d'un journal d'index par sa fin** (relecture d'everything declarative : un
/// premier index fait des Mo de journal en heures, le relire entier quatre fois par
/// seconde en ferait des Go) : seul ce qui s'est ajouté depuis la lecture précédente se
/// lit, et les derniers octets se gardent pour la dernière ligne.
#[derive(Default)]
struct LectureDuJournal {
    offset: u64,
    queue: Vec<u8>,
    /// La ligne commencée, pas encore finie par son retour.
    entamee: Vec<u8>,
    /// Les lignes complètes lues à la dernière lecture.
    neuves: Vec<String>,
}

impl LectureDuJournal {
    /// Lit l'ajout : `None` si le journal n'existe pas ; sinon, si l'ajout (cousu à la
    /// queue d'avant) dit « plein texte prêt », et la fin du journal si sa dernière ligne
    /// la dit (le contrat de `index_nodes` : `DONE_LINE`, éventuellement « — <reste dû> »,
    /// ou une ligne d'échec ou d'arrêt) : `(fini, la ligne)`.
    fn lire(&mut self, journal: &Path) -> Option<(bool, Option<(bool, String)>)> {
        use crate::dataflow::index_nodes::{DONE_LINE, FAILED_PREFIX, STOPPED_PREFIX};
        use std::io::{Read, Seek, SeekFrom};
        let mut f = std::fs::File::open(journal).ok()?;
        let longueur = f.metadata().map(|m| m.len()).unwrap_or(0);
        if longueur < self.offset {
            // Réécrit depuis : tout relire.
            self.offset = 0;
            self.queue.clear();
        }
        let mut ajout = Vec::new();
        if f.seek(SeekFrom::Start(self.offset)).is_ok() {
            let _ = f.read_to_end(&mut ajout);
        }
        self.offset += ajout.len() as u64;
        // Les lignes complètes de l'ajout (la dernière, sans retour, attend la suite).
        let mut lignes = std::mem::take(&mut self.entamee);
        lignes.extend_from_slice(&ajout);
        let fin_complete = lignes.iter().rposition(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
        self.entamee = lignes[fin_complete..].to_vec();
        self.neuves = String::from_utf8_lossy(&lignes[..fin_complete]).lines().map(str::to_string).collect();
        let mut cousu = std::mem::take(&mut self.queue);
        cousu.extend_from_slice(&ajout);
        let texte_pret = !ajout.is_empty() && String::from_utf8_lossy(&cousu).contains("plein texte prêt");
        let debut = cousu.len().saturating_sub(QUEUE_DU_JOURNAL);
        self.queue = cousu[debut..].to_vec();
        let queue = String::from_utf8_lossy(&self.queue);
        let fin = queue.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim().to_string()).and_then(|derniere| {
            if derniere.starts_with(DONE_LINE) {
                Some((true, derniere))
            } else if derniere.starts_with(FAILED_PREFIX) || derniere.starts_with(STOPPED_PREFIX) {
                Some((false, derniere))
            } else {
                None
            }
        });
        Some((texte_pret, fin))
    }
}

/// **L'avancement d'un index, lu dans une ligne de son journal** (Lucie, 8 novembre 2026 à
/// 16 h 30 : une barre qui avance, son pourcentage ; « par SSE, pas par polling ») : la
/// phase, et ce qui est fait sur le total. Les lignes de `index_nodes` et de
/// `catalog::progress` : « plein texte : N fichiers sur M (…) », « … · vecteurs N sur C
/// (P %, modèle) … », « … · vecteurs prêts (C morceaux, modèle) ».
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Avancement {
    pub phase: &'static str,
    pub fait: u64,
    pub total: u64,
}

pub(super) fn avancement_de_la_ligne(ligne: &str) -> Option<Avancement> {
    let nombre = |s: &str| s.trim().split(|c: char| !c.is_ascii_digit()).next().and_then(|n| n.parse::<u64>().ok());
    if let Some(i) = ligne.find("vecteurs prêts (") {
        let c = nombre(&ligne[i + "vecteurs prêts (".len()..])?;
        return Some(Avancement { phase: "vecteurs", fait: c, total: c });
    }
    // Le dernier « vecteurs » de la ligne (relecture d'everything declarative : une ligne
    // qui en porterait deux, « sans vecteurs … · vecteurs N sur C », se lit par le second).
    if let Some(i) = ligne.rfind("vecteurs ") {
        let reste = &ligne[i + "vecteurs ".len()..];
        if let Some((fait, apres)) = reste.split_once(" sur ") {
            if let (Some(fait), Some(total)) = (nombre(fait), nombre(apres)) {
                return Some(Avancement { phase: "vecteurs", fait, total });
            }
        }
    }
    if let Some(reste) = ligne.strip_prefix("plein texte : ") {
        if let Some((fait, apres)) = reste.split_once(" fichiers sur ") {
            if let (Some(fait), Some(total)) = (nombre(fait), nombre(apres)) {
                return Some(Avancement { phase: "plein texte", fait, total });
            }
        }
    }
    None
}

/// La cadence des avancements publiés : au plus un par seconde et par dépôt, plus un à
/// chaque changement de phase.
const CADENCE_DE_L_AVANCEMENT: Duration = Duration::from_secs(1);

/// Les fichiers et les symboles de l'index d'un backend (`fts_counts` de
/// `Backend::index_states`), relus une fois si une écriture tient le catalogue ;
/// `None` plutôt que des zéros.
fn comptes_de_l_index(_hote: &DaemonHost, hosted: &Arc<super::Hosted>) -> (Option<u64>, Option<u64>) {
    for essai in 0..2 {
        // **Sans jamais rouvrir le dépôt** : un dépôt fermé depuis la fin de l'index (une
        // fermeture, un hôte qui lâche) ne se rouvre pas pour un compte — la réouverture
        // tenait le slot pendant que la reprise le croyait ouvert (passe de la pile de
        // l'atelier, 8 novembre 2026). Fermé : pas de comptes.
        let etat = match hosted.backend.try_read() {
            Ok(g) => g.as_ref().and_then(|b| b.index_states().ok()).unwrap_or(Value::Null),
            Err(_) => Value::Null,
        };
        if etat["busy"] == true && essai == 0 {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        let compte = |entite: &str| etat["fts_counts"][entite]["rows"].as_u64();
        return (compte("File"), compte("Symbol"));
    }
    (None, None)
}

/// **Suivre le premier index d'un dépôt** jusqu'à la dernière ligne de son journal, puis
/// publier `RepoIndexed` sur `repos`. Un fil nommé, borné à la vie de l'hôte (le `Weak` :
/// l'hôte parti, il sort sans rien dire) et par `bornes` (un journal qui ne paraît pas, un
/// journal qui ne finit pas : un échec dit tel). `quoi` porte le worktree, le dépôt et la
/// branche, repris dans l'événement.
pub(super) fn suivre_le_premier_index(hote: Weak<DaemonHost>, hosted: Arc<super::Hosted>, journal: PathBuf, quoi: Value, bornes: Bornes) {
    let nom = format!("index-suivi:{}", hosted.id);
    let _ = std::thread::Builder::new().name(nom).spawn(move || {
        let debut = Instant::now();
        // Le plein texte prêt, quand le journal le dit (« plein texte prêt · vecteurs … ») :
        // la recherche par mots répond dès ce moment.
        let mut texte_ms: Option<u64> = None;
        let mut lecture = LectureDuJournal::default();
        // Le dernier avancement publié, l'instant et ce qui était fait : le débit se mesure
        // entre deux publications.
        let mut publie: Option<(Avancement, Instant)> = None;
        let mut dernier: Option<Avancement> = None;
        let cle = hosted.manifest.display().to_string();
        let (ok, ligne) = loop {
            match lecture.lire(&journal) {
                None if debut.elapsed() > bornes.naissance => {
                    break (false, format!("le journal de l'index n'existe pas : {}", journal.display()));
                }
                None => {}
                Some((texte_pret, fin)) => {
                    if texte_pret && texte_ms.is_none() {
                        texte_ms = Some(debut.elapsed().as_millis() as u64);
                    }
                    for l in &lecture.neuves {
                        if let Some(a) = avancement_de_la_ligne(l) {
                            dernier = Some(a);
                        }
                    }
                    if let Some(a) = dernier.clone() {
                        let change_de_phase = publie.as_ref().is_none_or(|(p, _)| p.phase != a.phase);
                        let a_bouge = publie.as_ref().is_none_or(|(p, _)| p != &a);
                        let a_temps = publie.as_ref().is_none_or(|(_, t)| t.elapsed() >= CADENCE_DE_L_AVANCEMENT);
                        if a_bouge && (change_de_phase || a_temps) {
                            if let Some(h) = hote.upgrade() {
                                let debit = publie.as_ref().filter(|(p, _)| p.phase == a.phase && a.fait >= p.fait).map(|(p, t)| {
                                    let s = t.elapsed().as_secs_f64().max(0.001);
                                    (((a.fait - p.fait) as f64 / s) * 10.0).round() / 10.0
                                });
                                let mut e = json!({
                                    "kind": "IndexProgress",
                                    "phase": a.phase,
                                    "fait": a.fait,
                                    "total": a.total,
                                    "pourcent": if a.total > 0 { (a.fait * 100 / a.total).min(100) } else { 0 },
                                    "debit": debit,
                                    "unite": if a.phase == "vecteurs" { "morceaux" } else { "fichiers" },
                                    "at_ms": crate::events::maintenant_ms(),
                                });
                                if let (Some(o), Some(q)) = (e.as_object_mut(), quoi.as_object()) {
                                    for (k, v) in q {
                                        o.insert(k.clone(), v.clone());
                                    }
                                }
                                h.ide_flux().publier("repos", &cle, e);
                            }
                            publie = Some((a, Instant::now()));
                        }
                    }
                    if let Some(fin) = fin {
                        break fin;
                    }
                }
            }
            if debut.elapsed() > bornes.plafond {
                break (false, format!("le journal de l'index n'a pas fini en {} min ({})", bornes.plafond.as_secs() / 60, journal.display()));
            }
            if hote.strong_count() == 0 {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        };
        let Some(h) = hote.upgrade() else { return };
        let (fichiers, symboles) = if ok { comptes_de_l_index(&h, &hosted) } else { (None, None) };
        let reste = ligne.split_once(" — ").map(|(_, r)| r.to_string()).filter(|_| ok);
        let mut e = json!({
            "kind": "RepoIndexed",
            "ok": ok,
            "fichiers": fichiers,
            "symboles": symboles,
            "ms": debut.elapsed().as_millis() as u64,
            "texte_ms": texte_ms,
            "reste": reste,
            "erreur": (!ok).then_some(ligne),
            "journal": journal,
        });
        if let (Some(o), Some(q)) = (e.as_object_mut(), quoi.as_object()) {
            for (k, v) in q {
                o.insert(k.clone(), v.clone());
            }
        }
        h.ide_flux().publier("repos", &hosted.manifest.display().to_string(), e);
        h.publier_l_etat_de_l_hote();
    });
}

/// Le chemin du journal dans le reçu d'`index`, où qu'il soit : la ligne
/// `journal : <chemin>` du texte (le format de l'outil), ou un champ `journal`.
fn journal_du_recu(v: &Value) -> Option<PathBuf> {
    match v {
        Value::String(s) => s.lines().find_map(|l| l.trim().strip_prefix("journal : ")).map(|p| PathBuf::from(p.trim())),
        Value::Object(o) => o.get("journal").and_then(Value::as_str).map(PathBuf::from).or_else(|| o.values().find_map(journal_du_recu)),
        Value::Array(a) => a.iter().find_map(journal_du_recu),
        _ => None,
    }
}

/// L'état de l'index d'un manifeste hébergé, dit en mots, d'après `/etat`.
fn etat_de_l_index(etat: &Value, manifeste: &Path) -> String {
    let b = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(manifeste)));
    match b {
        None => "pas ouvert".into(),
        Some(b) if b["open"] != true => "pas ouvert".into(),
        Some(b) => match &b["index"] {
            Value::Object(index) if index.values().any(est_une_entite) => phrase_de_l_index(index),
            Value::Object(index) if index.get("busy") == Some(&Value::Bool(true)) => "indexation en cours : une écriture tient l'index, ses comptes se diront au paquet suivant".into(),
            Value::Object(index) if index.get("indexings").and_then(Value::as_u64).is_some_and(|n| n > 0) => "indexation en cours, rien de cherchable encore".into(),
            Value::Null | Value::Object(_) => "ouvert, pas encore indexé".into(),
            Value::String(s) => s.clone(),
            autre => autre.to_string(),
        },
    }
}

/// Une clé de `index` qui est une entité : elle porte `text` et `vectors`. Les autres
/// (`fts_counts`, `reads`, `watches`, `embedding`, …) disent l'état, pas une entité.
fn est_une_entite(v: &Value) -> bool {
    v["text"].is_string() && v["vectors"].is_string()
}

/// **L'index en mots, sans faux « en cours »** (c7, 8 novembre 2026, vu sur le poste de
/// Lucie : « plein texte : 4/7, vecteurs : 3/7 » sur un index prêt, lu « indexation en
/// cours »). On compte les entités seules, et pour les vecteurs les entités qui en
/// déclarent seulement. « prêt » quand tout l'est ; « en cours, N % » seulement quand un
/// pourcentage le dit.
fn phrase_de_l_index(index: &serde_json::Map<String, Value>) -> String {
    let entites: Vec<&Value> = index.values().filter(|v| est_une_entite(v)).collect();
    // « en cours » seulement quand une indexation vit (c7, 8 novembre 2026 : une note
    // d'avant un arrêt n'est pas un travail qui avance).
    let vit = index.get("indexings").and_then(Value::as_u64).is_some_and(|n| n > 0);
    let signal = |cle: &str, pourcent: &str, nom: &str, prets_mot: &str, declares: &str| -> String {
        let vus: Vec<&&Value> = entites.iter().filter(|e| e[cle] != "not_declared").collect();
        let n = vus.len();
        if n == 0 {
            return format!("sans {nom} déclarés");
        }
        let prets = vus.iter().filter(|e| e[cle] == "ready").count();
        let echecs = vus.iter().filter(|e| e[cle] == "failed").count();
        let compte = format!("{prets}/{n}{declares}");
        if prets == n {
            return format!("{nom} {prets_mot} ({compte})");
        }
        // Un pourcentage d'une entité en cours seulement : « jamais » à 0 % n'avance pas.
        let en_cours = vus
            .iter()
            .filter(|e| e[cle] == "running")
            .filter_map(|e| e[pourcent].as_u64())
            .filter(|p| *p > 0 && *p < 100)
            .min();
        let mut dit = format!("{nom} {compte}");
        if let Some(p) = en_cours {
            dit.push_str(&if vit { format!(", en cours, {p} %") } else { format!(", {p} % faits, rien ne tourne") });
        }
        if echecs > 0 {
            dit.push_str(&format!(", {echecs} en échec"));
        }
        dit
    };
    let dit = format!(
        "{}, {}",
        signal("text", "text_percent", "plein texte", "prêt", ""),
        signal("vectors", "vectors_percent", "vecteurs", "prêts", " déclarés")
    );
    // Une indexation qui tourne se dit, même quand aucun pourcentage ne l'a encore écrit.
    match index.get("indexings").and_then(Value::as_u64).filter(|n| *n > 0) {
        Some(_) if !dit.contains("en cours") => format!("{dit} ; indexation en cours"),
        _ => dit,
    }
}

/// Une commande git dans `dossier`, sa sortie ; l'erreur dit la commande et ce que
/// git a écrit.
fn git(dossier: &Path, args: &[&str]) -> Result<String, String> {
    let sortie = std::process::Command::new("git")
        .arg("-C")
        .arg(dossier)
        .args(args)
        .output()
        .map_err(|e| format!("git {} : {e}", args.join(" ")))?;
    if !sortie.status.success() {
        return Err(format!("git {} : {}", args.join(" "), String::from_utf8_lossy(&sortie.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&sortie.stdout).trim().to_string())
}

/// Le nom d'un dossier frère pour une branche : `/` devient `-`, le reste qui n'est
/// pas sûr dans un nom de dossier devient `_`.
fn dossier_de_branche(branche: &str) -> String {
    branche.chars().map(|c| match c {
        '/' => '-',
        c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') => c,
        _ => '_',
    }).collect()
}

/// **Le témoin du premier index d'un worktree attaché** : à côté de son manifeste, le
/// chemin du journal de l'index lancé par l'attache. Ôté quand le journal finit par
/// [`DONE_LINE`](crate::dataflow::index_nodes::DONE_LINE) ; tant qu'il reste, un hôte
/// qui ouvre le worktree fermé relance l'index (c7, 8 novembre 2026, 16 h 09 : un
/// worktree attaché à 16 h 04, l'hôte reposé à 16 h 08, l'index n'avait plus repris).
pub(crate) fn marque_du_premier_index(manifeste: &Path) -> PathBuf {
    // Hors de l'arbre des déclarations : une tâche y verrait une déclaration inconnue.
    crate::builtin_templates::state_dir_of(manifeste).join("premier-index")
}

/// Le premier index d'un worktree reste-t-il à finir ? Ôte le témoin d'un index fini.
fn premier_index_a_finir(manifeste: &Path) -> bool {
    let marque = marque_du_premier_index(manifeste);
    let Ok(journal) = std::fs::read_to_string(&marque) else { return false };
    let fini = std::fs::read_to_string(journal.trim())
        .ok()
        .and_then(|t| t.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.starts_with(crate::dataflow::index_nodes::DONE_LINE)))
        .unwrap_or(false);
    if fini {
        let _ = std::fs::remove_file(&marque);
    }
    !fini
}

/// **Lance l'index d'un backend ouvert** et pose le témoin de son journal ; rend le
/// journal. Un index qui tourne déjà n'en lance pas un second.
/// Le suiveur d'archi s'y pose (`suivre_le_premier_index`) : la fin, l'avancement et la
/// reprise se disent sur le flux, quel que soit le chemin qui a lancé l'index (l'attache,
/// la reprise, le rattrapage à l'ouverture).
fn lancer_sur(b: &crate::backend::Backend, manifeste: &Path, suivi: Option<(Weak<DaemonHost>, Arc<super::Hosted>)>) -> Result<Option<PathBuf>, String> {
    if crate::dataflow::index_nodes::indexings_running(&b.catalog) > 0 {
        return Ok(None);
    }
    let v = b.call("index", json!({ "confirm": true }))?;
    let journal = journal_du_recu(&v);
    if let Some(j) = &journal {
        let marque = marque_du_premier_index(manifeste);
        if let Some(d) = marque.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(marque, j.display().to_string());
        if let Some((hote, hosted)) = suivi {
            let racine = super::espace_du_manifeste(manifeste);
            let (depot, branche) = racine.as_deref().map(|r| (super::repo_of(r), super::branch_of(r))).unwrap_or((None, None));
            let quoi = json!({ "worktree": racine, "depot": depot, "branche": branche });
            suivre_le_premier_index(hote, hosted, j.clone(), quoi, BORNES_DU_SUIVI);
        }
    }
    Ok(journal)
}

/// **À l'ouverture d'un backend par l'hôte** : son index repart en fond quand son
/// premier index a été coupé (le témoin `.premier-index`), ou quand le manifeste déclare
/// `embeddings.catch_up_on_open` et que des morceaux sont encore sans vecteur — la
/// réouverture reconstruit l'index HNSW mais ne solde aucune dette (c7, 8 novembre 2026 :
/// wt-react5 resté à 15 % de vecteurs 18 minutes après sa réouverture). Une base qui ne
/// déclare ni l'un ni l'autre n'est jamais indexée par là.
pub(super) fn a_l_ouverture(hote: &DaemonHost, hosted: &super::Hosted, b: &crate::backend::Backend) {
    if !b.prepared().manifest.tools.contains_key("index") {
        return;
    }
    let coupe = premier_index_a_finir(&hosted.manifest);
    let dette = if coupe || !b.prepared().manifest.embeddings.as_ref().is_some_and(|e| e.catch_up_on_open) {
        0
    } else {
        b.catalog.lock().unwrap_or_else(|p| p.into_inner()).index_progress().map(|p| p.dense_missing() + p.sparse_missing()).unwrap_or(0)
    };
    if !coupe && dette == 0 {
        return;
    }
    let pourquoi = if coupe { "son premier index avait été coupé".to_string() } else { format!("{dette} morceaux encore sans vecteur") };
    let suivi = hote.soi.get().cloned().zip(hote.hosted_existing(&hosted.manifest));
    match lancer_sur(b, &hosted.manifest, suivi) {
        Ok(j) => {
            hosted.rattrapage.store(j.is_some(), std::sync::atomic::Ordering::SeqCst);
            eprintln!(
            "[hôte] {} : {pourquoi} — l'index repart en fond{}",
            hosted.manifest.display(),
            j.map(|j| format!(" (journal : {})", j.display())).unwrap_or_default()
            )
        }
        Err(e) => eprintln!("[hôte] {} : {pourquoi}, mais l'index n'a pas pu repartir : {e}", hosted.manifest.display()),
    }
}

/// **La carte de l'avancement d'un dépôt dont l'index tourne**, que la page rend sous le
/// tour et nourrit des `IndexProgress` du flux (Lucie, 8 novembre 2026, 16 h 30 : « par
/// SSE, pas par polling »). Lu sans ouvrir le dépôt : un dépôt fermé n'avance pas.
fn carte_si_avance(hote: &super::DaemonHost, manifeste: &Path) -> Option<Value> {
    let h = hote.hosted_existing(manifeste)?;
    let avance = h.backend.try_read().ok().is_some_and(|g| g.as_ref().is_some_and(|b| crate::dataflow::index_nodes::indexings_running(&b.catalog) > 0));
    if !avance {
        return None;
    }
    // « dépôt · branche », le titre de la carte (archi : sinon elle montre l'id).
    let racine = super::espace_du_manifeste(manifeste);
    let titre = racine.as_deref().and_then(|r| {
        let depot = super::repo_of(r)?;
        Some(match super::branch_of(r) {
            Some(b) => format!("{depot} · {b}"),
            None => depot,
        })
    });
    Some(json!({ "kind": "index_progress", "sujets": ["repos"], "repo": h.id, "manifest": h.manifest, "titre": titre }))
}

/// Le backend de `h` est-il ouvert dans l'hôte (ou en train de l'être) ?
fn ouvert(h: &super::Hosted) -> bool {
    h.backend.try_read().map(|g| g.is_some()).unwrap_or(true)
}

/// **Lance l'index d'un worktree** et pose le témoin de son journal ; dit en mots ce
/// qui est parti.
fn lancer_l_index(hote: &super::DaemonHost, h: &super::Hosted, manifeste: &Path) -> String {
    // Un index déjà reparti à l'ouverture (`a_l_ouverture`) se dit de même.
    let suivi = hote.soi.get().cloned().zip(hote.hosted_existing(manifeste));
    match hote.with_backend(h, |b| lancer_sur(b, manifeste, suivi)) {
        Ok(Ok(_)) => {
            // **Un outil l'attend : il tient la porte**, qu'il l'ait lancé ou trouvé en cours
            // (relancé à l'ouverture par `a_l_ouverture`, dans ce même appel). Sinon la
            // réouverture d'un service d'embarquement revenu, ou une fermeture, l'arrêterait
            // sous l'outil qui l'a demandé (archi, 8 novembre 2026 : « index … en échec :
            // indexation arrêtée entre deux passes de vecteurs », 1 fois sur 3).
            h.rattrapage.store(false, std::sync::atomic::Ordering::SeqCst);
            "son index est lancé en fond ; sa fin se dira sur le flux de l'IDE".to_string()
        }
        Ok(Err(e)) | Err(e) => format!("son index n'a pas pu partir : {e}"),
    }
}

/// **Rouvre un worktree fermé dont le premier index n'a pas fini** : l'ouverture le
/// relance (`a_l_ouverture`). Rend ce qui est parti, ou `None` s'il n'y avait rien à
/// reprendre.
fn reprendre_si_fini_pas(hote: &super::DaemonHost, manifeste: &Path) -> Option<String> {
    if !premier_index_a_finir(manifeste) {
        return None;
    }
    let h = hote.hosted(manifeste).ok()?;
    if ouvert(&h) {
        return None;
    }
    Some(lancer_l_index(hote, &h, manifeste))
}

impl super::DaemonHost {
    /// **Au démarrage de l'hôte, les premiers index des worktrees attachés reprennent**
    /// (c7, 8 novembre 2026, 16 h 09). Seuls les worktrees dont le témoin dit un index pas
    /// fini sont ouverts : un dépôt inscrit sans témoin (la base principale de Lucie)
    /// n'est jamais ouvert ni indexé par là.
    pub fn reprendre_les_premiers_index(self: &Arc<Self>) -> Vec<String> {
        let Ok(racines) = self.racines_de_code() else { return Vec::new() };
        racines
            .registered_manifests()
            .into_iter()
            .filter_map(|m| {
                let m = m.canonicalize().unwrap_or(m);
                reprendre_si_fini_pas(self, &m).map(|dit| format!("{} : {dit}", m.display()))
            })
            .collect()
    }
}

impl WorktreeHost for HoteDesWorktrees {
    fn attach(&self, path: &Path) -> Result<WorktreeAnswer, String> {
        let worktree = path
            .canonicalize()
            .map_err(|e| format!("le worktree {} n'existe pas ({e}) : donnez le chemin d'un dossier sur ce poste", path.display()))?;
        if !worktree.is_dir() {
            return Err(format!("{} n'est pas un dossier : un worktree est un dossier", worktree.display()));
        }
        let hote = self.hote()?;
        let racines = hote.racines_de_code()?;
        let nom = crate::code_tree::name_of_tree(&worktree)?;
        let manifeste = racines.manifest(&nom);
        let deja = manifeste.exists();
        if !deja {
            std::fs::create_dir_all(&racines.data).map_err(|e| format!("{} : {e}", racines.data.display()))?;
            let (source, a_effacer) = gabarit_livre(&racines.data)?;
            let inscrit = crate::code_tree::add_files(&racines, &worktree, &source.display().to_string(), Some(&nom));
            let _ = std::fs::remove_dir_all(&a_effacer);
            inscrit?;
        }
        // Comparé à `/etat` par son chemin canonique, comme l'hôte le tient.
        let manifeste = manifeste.canonicalize().unwrap_or(manifeste);
        let h = hote.hosted(&manifeste)?;
        let etait_ferme = !ouvert(&h);
        hote.with_backend(&h, |_| ())?;
        let depot = super::repo_of(&worktree);
        let branche = super::branch_of(&worktree);
        // Le premier index part en fond. Un worktree déjà attaché mais fermé (un hôte
        // redémarré depuis) se rouvre et relance le sien, qui reprend ce qui manque ; un
        // worktree ouvert garde le sien (c7, 8 novembre 2026, 16 h 09).
        let index = if deja && !etait_ferme { None } else { Some(lancer_l_index(&hote, &h, &manifeste)) };
        hote.ide_flux().publier(
            "repos",
            &manifeste.display().to_string(),
            json!({ "kind": "RepoAttached", "worktree": worktree, "depot": depot, "branche": branche, "deja": deja }),
        );
        hote.publier_l_etat_de_l_hote();
        let ou = match (&depot, &branche) {
            (Some(d), Some(b)) => format!("dépôt {d}, branche {b}"),
            (Some(d), None) => format!("dépôt {d}, tête détachée"),
            (None, _) => "hors d'un dépôt git".to_string(),
        };
        let ui: Vec<Value> = carte_si_avance(&hote, &manifeste).into_iter().collect();
        let text = match index {
            Some(i) if deja => format!("Le worktree {} est déjà attaché ({ou}) ; il était fermé, il est rouvert, et {i}. list_worktrees dit où il en est.", worktree.display()),
            Some(i) => format!("Worktree attaché : {} ({ou}) ; {i}. list_worktrees dit où il en est.", worktree.display()),
            None => format!(
                "Le worktree {} est déjà attaché ({ou}) : il est ouvert ; son index : {}.",
                worktree.display(),
                etat_de_l_index(&hote.state(), &manifeste)
            ),
        };
        Ok(WorktreeAnswer { text, ui })
    }

    fn open_branch(&self, path: &Path, branch: &str, from: Option<&str>) -> Result<WorktreeAnswer, String> {
        let lieu = path
            .canonicalize()
            .map_err(|e| format!("{} n'existe pas ({e}) : donnez un worktree ou un dépôt de ce poste", path.display()))?;
        let depot = PathBuf::from(git(&lieu, &["rev-parse", "--show-toplevel"]).map_err(|_| format!("{} n'est dans aucun dépôt git : open_branch ouvre une branche d'un dépôt", lieu.display()))?);
        // Le dépôt principal (pas un worktree lié) : ses worktrees frères vivent à côté de lui.
        let commun = git(&depot, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let principal = Path::new(&commun).parent().map(Path::to_path_buf).unwrap_or_else(|| depot.clone());
        git(&principal, &["check-ref-format", "--branch", branch]).map_err(|_| format!("« {branch} » n'est pas un nom de branche valide"))?;
        let nom = principal.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "depot".into());
        let parent = principal.parent().ok_or_else(|| format!("{} n'a pas de dossier parent", principal.display()))?;
        let frere = parent.join(format!("{nom}-{}", dossier_de_branche(branch)));
        // La branche déjà ouverte dans un worktree : on l'attache, sans git.
        let ouverts = git(&principal, &["worktree", "list", "--porcelain"])?;
        let mut deja: Option<PathBuf> = None;
        let mut courant: Option<PathBuf> = None;
        for l in ouverts.lines() {
            if let Some(p) = l.strip_prefix("worktree ") {
                courant = Some(PathBuf::from(p));
            } else if l.strip_prefix("branch ").is_some_and(|b| b == format!("refs/heads/{branch}")) {
                deja = courant.clone();
            }
        }
        let (worktree, commande) = match deja {
            Some(w) => (w, None),
            None => {
                if frere.exists() {
                    return Err(format!("{} existe déjà et ne tient pas la branche {branch} : choisissez une autre branche, ou attachez ce dossier", frere.display()));
                }
                let existe = git(&principal, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
                let frere_txt = frere.display().to_string();
                let depart = from.unwrap_or("HEAD");
                let args: Vec<&str> = if existe { vec!["worktree", "add", &frere_txt, branch] } else { vec!["worktree", "add", "-b", branch, &frere_txt, depart] };
                git(&principal, &args)?;
                (frere.clone(), Some(format!("git -C {} {}", principal.display(), args.join(" "))))
            }
        };
        let WorktreeAnswer { text: attache, ui } = self.attach(&worktree)?;
        // **Né de l'atelier** : un worktree que cette commande a créé se marque, une fois inscrit (décision de
        // c7, 9 novembre 2026) ; l'atelier l'écrira au nom de l'utilisatrice. Une branche
        // déjà ouverte ailleurs ne l'est pas : c'est l'arbre de quelqu'un.
        if commande.is_some() {
            let racines = self.hote()?.racines_de_code()?;
            let nom = crate::code_tree::name_of_tree(&worktree.canonicalize().unwrap_or(worktree.clone()))?;
            let marque = racines.marque_de_l_atelier(&nom);
            if let Some(d) = marque.parent() {
                std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
            }
            std::fs::write(&marque, "open_branch\n").map_err(|e| format!("{} : {e}", marque.display()))?;
        }
        let text = match commande {
            Some(c) => format!("Branche {branch} ouverte dans le worktree {} (commande : `{c}`). {attache}", worktree.display()),
            None => format!("La branche {branch} est déjà ouverte dans le worktree {} : aucune commande git. {attache}", worktree.display()),
        };
        Ok(WorktreeAnswer { text, ui })
    }

    fn progress(&self, repo: Option<&str>) -> Result<WorktreeAnswer, String> {
        let hote = self.hote()?;
        let mut lignes = Vec::new();
        let mut ui = Vec::new();
        let etat = hote.state();
        for m in hote.racines_de_code()?.registered_manifests() {
            let m = m.canonicalize().unwrap_or(m);
            let racine = super::espace_du_manifeste(&m);
            let (depot, branche) = racine.as_deref().map(|r| (super::repo_of(r), super::branch_of(r))).unwrap_or((None, None));
            let carte = carte_si_avance(&hote, &m);
            let vise = match repo {
                // Un dépôt nommé : son nom, sa branche, « dépôt · branche » ou un bout du chemin.
                Some(r) => {
                    let r = r.trim();
                    depot.as_deref() == Some(r)
                        || branche.as_deref() == Some(r)
                        || depot.as_deref().zip(branche.as_deref()).is_some_and(|(d, b)| format!("{d} · {b}") == r)
                        || racine.as_deref().is_some_and(|p| p.to_string_lossy().contains(r))
                }
                None => carte.is_some(),
            };
            if !vise {
                continue;
            }
            let cible = hote.hosted(&m)?;
            lignes.push(format!(
                "- {} — dépôt {}, branche {} ; index : {} ; contexte : {}",
                racine.as_deref().map(|r| r.display().to_string()).unwrap_or_else(|| "?".into()),
                depot.as_deref().unwrap_or("hors git"),
                branche.as_deref().unwrap_or("détachée"),
                etat_de_l_index(&etat, &m),
                json!({"target":{"kind":"backend","id":cible.id}})
            ));
            ui.extend(carte);
        }
        let text = match (repo, lignes.is_empty()) {
            (Some(r), true) => return Err(format!("aucun worktree attaché ne répond à « {r} » : list_worktrees les dit")),
            (None, true) => "Aucun index n'avance en ce moment.".to_string(),
            (_, false) if ui.is_empty() => lignes.join("\n"),
            (_, false) => format!("{}\nLa carte sous ce message suit l'avancement en direct.", lignes.join("\n")),
        };
        Ok(WorktreeAnswer { text, ui })
    }

    fn read(&self, racine: Option<&Path>, path: &str, offset: usize, length: usize) -> Result<WorktreeAnswer, String> {
        self.hote()?.atelier_lire(racine, path, offset, length).map(WorktreeAnswer::from)
    }

    fn read_scope(&self, racine: Option<&Path>, name: &str, path: &str) -> Result<WorktreeAnswer, String> {
        self.hote()?.atelier_lire_scope(racine, name, path).map(WorktreeAnswer::from)
    }

    fn ask(&self, racine: Option<&Path>, path: &str, mode: &str, reason: &str) -> Result<WorktreeAnswer, String> {
        self.hote()?.atelier_demander(racine, path, mode, reason)
    }

    fn edit(&self, racine: Option<&Path>, path: &str, edition: crate::dataflow::fichier_nodes::Edition) -> Result<WorktreeAnswer, String> {
        self.hote()?.atelier_ecrire(racine, path, edition).map(WorktreeAnswer::from)
    }

    fn list(&self) -> Result<WorktreeAnswer, String> {
        let hote = self.hote()?;
        let manifestes = hote.racines_de_code()?.registered_manifests();
        if manifestes.is_empty() {
            return Ok(WorktreeAnswer::from("Aucun worktree attaché : attach_worktree en attache un.".to_string()));
        }
        // Un worktree fermé dont le premier index n'a pas fini se rouvre et le relance :
        // « pas ouvert » ne se dit plus d'un index qui devrait tourner.
        let manifestes: Vec<PathBuf> = manifestes.into_iter().map(|m| m.canonicalize().unwrap_or(m)).collect();
        for m in &manifestes {
            let _ = reprendre_si_fini_pas(&hote, m);
        }
        let etat = hote.state();
        let mut lignes = vec![format!("{} worktree(s) attaché(s) :", manifestes.len())];
        let ui: Vec<Value> = manifestes.iter().filter_map(|m| carte_si_avance(&hote, m)).collect();
        for m in manifestes {
            let racine = super::espace_du_manifeste(&m);
            let (depot, branche) = racine.as_deref().map(|r| (super::repo_of(r), super::branch_of(r))).unwrap_or((None, None));
            let cible = hote.hosted(&m)?;
            lignes.push(format!(
                "- {} — dépôt {}, branche {} ; index : {} ; contexte : {}",
                racine.as_deref().map(|r| r.display().to_string()).unwrap_or_else(|| "?".into()),
                depot.as_deref().unwrap_or("hors git"),
                branche.as_deref().unwrap_or("détachée"),
                etat_de_l_index(&etat, &m),
                json!({"target":{"kind":"backend","id":cible.id}})
            ));
        }
        Ok(WorktreeAnswer { text: lignes.join("\n"), ui })
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::hote;
    use crate::llm::{CallbackLlm, Llm, MockLlm, Turn};
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    fn git(dir: &Path, args: &[&str]) {
        assert!(std::process::Command::new("git").args(args).current_dir(dir).status().unwrap().success(), "git {args:?}");
    }

    /// Un écrivain qui garde ce que le flux lui envoie.
    struct Capte(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Capte {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// **L'atelier ouvre un worktree existant par un mot** (page 03 du 7 novembre 2026,
    /// lot 0 ; Lucie, 8 novembre : « l'atelier oui absolument »). Dans le chat de l'atelier,
    /// le modèle appelle `attach_worktree` avec le chemin d'un petit dépôt git : le
    /// manifeste du worktree est posé, il paraît dans `/etat`, `RepoAttached` passe sur le
    /// flux, son index part en fond, `list_worktrees` le dit ; un second appel dit « déjà
    /// attaché ». Aucune commande git n'est lancée par l'atelier.
    #[test]
    fn the_workshop_attaches_an_existing_worktree_from_the_chat() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        // L'atelier : le gabarit livré, copié comme un manifeste de l'utilisatrice.
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };

        let vus = Arc::new(Mutex::new(Vec::<String>::new()));
        let chemin = depot.display().to_string();
        let model: ChatModel = {
            let vus = vus.clone();
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (vus, chemin) = (vus.clone(), chemin.clone());
                Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                    let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                    let resultats: Vec<String> = turns[depuis..].iter().filter(|t| t.role == "tool").map(|t| t.content.clone()).collect();
                    match resultats.last() {
                        None => MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink),
                        Some(r) => {
                            vus.lock().unwrap().push(r.clone());
                            MockLlm::new("C'est fait.").generate(turns, opts, sink)
                        }
                    }
                })))
            })
        };
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines.clone()).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&atelier).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");

        host.chat_turn(&h.id, &format!("Ouvre le worktree {}", depot.display())).expect("le tour");
        let vu = vus.lock().unwrap().first().cloned().unwrap_or_default();
        assert!(vu.contains("Worktree attaché") && vu.contains("dépôt rag3db-essai, branche essai") && vu.contains("son index est lancé en fond"), "{vu}");
        assert!(!vu.contains("backend"), "le vocabulaire de l'utilisatrice : {vu}");
        // 1. Le manifeste posé.
        let manifeste = racines.manifest("code-rag3db-essai");
        assert!(manifeste.is_file(), "{}", manifeste.display());
        // 2. Le dépôt dans /etat.
        let etat = host.state();
        let canon = manifeste.canonicalize().unwrap();
        assert!(etat["backends"].as_array().unwrap().iter().any(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()) && b["open"] == true), "{etat:#}");
        // 3. RepoAttached sur le flux.
        let capte = String::from_utf8_lossy(&flux.lock().unwrap()).into_owned();
        assert!(capte.contains("RepoAttached") && capte.contains("rag3db-essai"), "{capte}");
        // 4. L'index lancé, que list_worktrees dit ; le second appel dit « déjà attaché ».
        let liste = host.with_backend(&h, |b| b.call("list_worktrees", json!({}))).unwrap().unwrap();
        let liste = liste["result"].as_str().unwrap_or_default().to_string();
        assert!(liste.contains("1 worktree(s) attaché(s)") && liste.contains(&depot.canonicalize().unwrap().display().to_string()) && liste.contains("branche essai"), "{liste}");
        let encore = host.with_backend(&h, |b| b.call("attach_worktree", json!({ "path": depot }))).unwrap().unwrap();
        let encore = encore["result"].as_str().unwrap_or_default().to_string();
        assert!(encore.contains("déjà attaché") && encore.contains("il est ouvert"), "{encore}");
        let _: Value = etat;
    }

    /// **Un worktree attaché paraît sous « Dépôts » sans recharger la page** (c7, 8
    /// novembre 2026, la suite de la barre). Dans le chat de l'atelier, l'utilisatrice
    /// demande d'ouvrir un worktree ; le modèle appelle `attach_worktree` ; la barre, en
    /// vue par dépôt, montre le dépôt et sa branche dès `RepoAttached`, la page jamais
    /// rechargée.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn un_worktree_attache_parait_sous_depots_sans_recharger_la_page() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        const TEMOIN: &str = "un_worktree_attache_parait_sous_depots_sans_recharger_la_page";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let chemin = depot.display().to_string();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let chemin = chemin.clone();
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                if turns[depuis..].iter().any(|t| t.role == "tool") {
                    MockLlm::new("C'est fait.").generate(turns, opts, sink)
                } else {
                    MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink)
                }
            })))
        });
        let (runs, chats) = (d.path().join("runs"), d.path().join("chats"));
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_chat_model(model));
        let racine = format!("http://{adresse}");
        let ide = |methode: &str, route: &str, corps: Value| -> Value {
            let agent = crate::daemon::agent_de_temoin();
            let mut r = if methode == "GET" {
                agent.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap()
            } else {
                agent.post(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap()
            };
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        assert_eq!(ide("POST", "/ide/describe", json!({ "manifest": atelier }))["ok"], true);
        let etat = ide("GET", "/ide/etat", Value::Null);
        let canon = atelier.canonicalize().unwrap();
        let id = etat["backends"]
            .as_array()
            .and_then(|a| a.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path())))
            .and_then(|b| b["id"].as_str())
            .unwrap_or_else(|| panic!("l'atelier dans l'état : {etat:#}"))
            .to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
        n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.aller(
            &format!("{racine}/ide/code/{id}/chat"),
            "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'",
        );
        n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'repo'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
        n.attendre("!!document.querySelector('#arbre-depots')", "la vue par dépôt");
        let sel = "#arbre-depots .depot[data-depot=\"rag3db-essai\"]";
        assert_eq!(n.evaluer(&format!("!!document.querySelector('{sel}')")), json!(false), "le dépôt paraît avant d'être attaché");
        // Une marque dans la page : un rechargement l'effacerait.
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('#message').value = 'Ouvre le worktree {}'; document.querySelector('#send').click()", depot.display()));
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour de l'atelier");
        n.attendre(&format!("!!document.querySelector('{sel} .espace')"), "le dépôt attaché sous « Dépôts »");
        let espace = n.evaluer(&format!("document.querySelector('{sel} .espace').textContent"));
        assert!(espace.as_str().unwrap_or_default().contains("rag3db-essai · essai"), "le dépôt et sa branche : {espace}");
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **La fin de l'index se dit dans le chat de l'atelier et dans la barre** (Lucie, 8
    /// novembre 2026 à 16 h 08 : « il devrait avoir notif quand indexation finie ? ») : après
    /// « Ouvre le worktree … », une ligne de la page dit « index de <dépôt · branche> terminé :
    /// N fichiers, M symboles, en T s » (pas un tour : le modèle ne la voit pas), et le dépôt
    /// passe à « rag indexé » sous « Dépôts », la page jamais rechargée.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn la_fin_de_l_index_se_dit_dans_le_chat_de_l_atelier_et_la_barre() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        const TEMOIN: &str = "la_fin_de_l_index_se_dit_dans_le_chat_de_l_atelier_et_la_barre";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let chemin = depot.display().to_string();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let chemin = chemin.clone();
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                if turns[depuis..].iter().any(|t| t.role == "tool") {
                    MockLlm::new("C'est fait.").generate(turns, opts, sink)
                } else {
                    MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink)
                }
            })))
        });
        let (runs, chats) = (d.path().join("runs"), d.path().join("chats"));
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_chat_model(model));
        let racine = format!("http://{adresse}");
        let ide = |methode: &str, route: &str, corps: Value| -> Value {
            let agent = crate::daemon::agent_de_temoin();
            let mut r = if methode == "GET" {
                agent.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap()
            } else {
                agent.post(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap()
            };
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        assert_eq!(ide("POST", "/ide/describe", json!({ "manifest": atelier }))["ok"], true);
        let etat = ide("GET", "/ide/etat", Value::Null);
        let canon = atelier.canonicalize().unwrap();
        let id = etat["backends"]
            .as_array()
            .and_then(|a| a.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path())))
            .and_then(|b| b["id"].as_str())
            .unwrap_or_else(|| panic!("l'atelier dans l'état : {etat:#}"))
            .to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
        n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.aller(
            &format!("{racine}/ide/code/{id}/chat"),
            "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'",
        );
        n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'repo'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
        n.attendre("!!document.querySelector('#arbre-depots')", "la vue par dépôt");
        let sel = "#arbre-depots .depot[data-depot=\"rag3db-essai\"]";
        assert_eq!(n.evaluer(&format!("!!document.querySelector('{sel}')")), json!(false), "le dépôt paraît avant d'être attaché");
        // Une marque dans la page : un rechargement l'effacerait.
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('#message').value = 'Ouvre le worktree {}'; document.querySelector('#send').click()", depot.display()));
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour de l'atelier");
        n.attendre(&format!("!!document.querySelector('{sel} .espace')"), "le dépôt attaché sous « Dépôts »");
        let espace = n.evaluer(&format!("document.querySelector('{sel} .espace').textContent"));
        assert!(espace.as_str().unwrap_or_default().contains("rag3db-essai · essai"), "le dépôt et sa branche : {espace}");
        // La fin du premier index : une ligne de la page dans le chat de l'atelier, et le
        // rag du dépôt indexé dans la barre, sans recharger.
        n.attendre("!!document.querySelector('.fin-d-index')", "la ligne de fin d'index dans le chat de l'atelier");
        let ligne = n.evaluer("document.querySelector('.fin-d-index').textContent");
        let ligne = ligne.as_str().unwrap_or_default().to_string();
        assert!(ligne.starts_with("index de rag3db-essai · essai terminé") && ligne.contains("fichiers") && ligne.contains("symboles") && ligne.contains(" s"), "{ligne}");
        assert_eq!(n.evaluer("document.querySelectorAll('.message.assistant').length"), json!(1), "la ligne n'est pas un tour de l'agent");
        n.attendre(&format!("document.querySelector('{sel} .espace')?.textContent.includes('rag indexé')"), "le rag du dépôt indexé dans la barre");
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **La carte de l'avancement glisse sous l'appel** (Lucie, 8 novembre 2026 à 16 h 30 : « que
    /// ça glisse une UI dans le chat ») : attach_worktree porte un `ui` `index_progress` que le
    /// modèle ne voit pas ; la page rend la carte sous l'appel, titrée « dépôt · branche »,
    /// branchée sur le flux, jusqu'à « terminé » ; rechargée, la carte reparaît par le tour relu.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn la_carte_de_l_avancement_glisse_sous_l_appel_et_reparait_au_rechargement() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        const TEMOIN: &str = "la_carte_de_l_avancement_glisse_sous_l_appel_et_reparait_au_rechargement";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let chemin = depot.display().to_string();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let chemin = chemin.clone();
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                if turns[depuis..].iter().any(|t| t.role == "tool") {
                    MockLlm::new("C'est fait.").generate(turns, opts, sink)
                } else {
                    MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink)
                }
            })))
        });
        let (runs, chats) = (d.path().join("runs"), d.path().join("chats"));
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_chat_model(model));
        let racine = format!("http://{adresse}");
        let ide = |methode: &str, route: &str, corps: Value| -> Value {
            let agent = crate::daemon::agent_de_temoin();
            let mut r = if methode == "GET" {
                agent.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap()
            } else {
                agent.post(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap()
            };
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        assert_eq!(ide("POST", "/ide/describe", json!({ "manifest": atelier }))["ok"], true);
        let etat = ide("GET", "/ide/etat", Value::Null);
        let canon = atelier.canonicalize().unwrap();
        let id = etat["backends"]
            .as_array()
            .and_then(|a| a.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path())))
            .and_then(|b| b["id"].as_str())
            .unwrap_or_else(|| panic!("l'atelier dans l'état : {etat:#}"))
            .to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.commande("Emulation.setDeviceMetricsOverride", json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }));
        n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.aller(
            &format!("{racine}/ide/code/{id}/chat"),
            "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'",
        );
        n.evaluer("(() => { const s = document.querySelector('#vue-barre'); const set = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set; set.call(s, 'repo'); s.dispatchEvent(new Event('change', { bubbles: true })); })()");
        n.attendre("!!document.querySelector('#arbre-depots')", "la vue par dépôt");
        let sel = "#arbre-depots .depot[data-depot=\"rag3db-essai\"]";
        assert_eq!(n.evaluer(&format!("!!document.querySelector('{sel}')")), json!(false), "le dépôt paraît avant d'être attaché");
        // Une marque dans la page : un rechargement l'effacerait.
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('#message').value = 'Ouvre le worktree {}'; document.querySelector('#send').click()", depot.display()));
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour de l'atelier");
        n.attendre(&format!("!!document.querySelector('{sel} .espace')"), "le dépôt attaché sous « Dépôts »");
        let espace = n.evaluer(&format!("document.querySelector('{sel} .espace').textContent"));
        assert!(espace.as_str().unwrap_or_default().contains("rag3db-essai · essai"), "le dépôt et sa branche : {espace}");
        // La fin du premier index : une ligne de la page dans le chat de l'atelier, et le
        // rag du dépôt indexé dans la barre, sans recharger.
        n.attendre("!!document.querySelector('.fin-d-index')", "la ligne de fin d'index dans le chat de l'atelier");
        let ligne = n.evaluer("document.querySelector('.fin-d-index').textContent");
        let ligne = ligne.as_str().unwrap_or_default().to_string();
        assert!(ligne.starts_with("index de rag3db-essai · essai terminé") && ligne.contains("fichiers") && ligne.contains("symboles") && ligne.contains(" s"), "{ligne}");
        assert_eq!(n.evaluer("document.querySelectorAll('.message.assistant').length"), json!(1), "la ligne n'est pas un tour de l'agent");
        n.attendre(&format!("document.querySelector('{sel} .espace')?.textContent.includes('rag indexé')"), "le rag du dépôt indexé dans la barre");
        // La carte sous l'appel d'attach_worktree : son titre, sa fin.
        let carte = "details.outil[data-outil=\"attach_worktree\"] + .ui-du-tour .carte-avancement";
        n.attendre(&format!("!!document.querySelector('{carte}')"), "la carte sous l'appel d'attach_worktree");
        assert!(n.evaluer(&format!("document.querySelector('{carte}').textContent")).as_str().unwrap_or_default().contains("rag3db-essai · essai"), "le titre de la carte");
        n.attendre(&format!("document.querySelector('{carte}')?.dataset.etat === 'fini' && document.querySelector('{carte}').textContent.includes('terminé')"), "la carte dit la fin de l'index");

        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");
        // Rechargée : la carte reparaît, relue dans le tour (turn.ui), et dit l'index prêt.
        n.aller(&format!("{racine}/ide/code/{id}/chat"), "document.getElementById('lien')?.textContent === 'flux : ouvert'");
        n.attendre(&format!("document.querySelector('{carte}')?.dataset.etat === 'fini' && document.querySelector('{carte}').textContent.includes('index prêt')"), "la carte relue après le rechargement");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **L'atelier ouvre une branche dans un worktree frère** (page 03, le pas d'après
    /// attach_worktree ; c7 : « aucun git caché : la commande git est dite dans la
    /// réponse »). Une branche neuve naît dans `<dépôt>-<branche>` à côté du dépôt, le
    /// worktree est attaché, la réponse dit la commande lancée ; rouvrir la même branche
    /// ne lance aucune commande ; un nom invalide est refusé.
    #[test]
    fn the_workshop_opens_a_branch_in_a_sibling_worktree() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new("rien"))) });
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines.clone()).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let h = host.hosted(&atelier).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");
        let appel = |args: Value| -> Result<String, String> {
            host.with_backend(&h, |b| b.call("open_branch", args)).unwrap().map(|r| r["result"].as_str().unwrap_or_default().to_string())
        };

        let dit = appel(json!({ "path": depot, "branch": "lot/neuve" })).expect("la branche s'ouvre");
        let frere = d.path().join("rag3db-essai-lot-neuve");
        assert!(frere.is_dir(), "le worktree frère : {dit}");
        let tete = std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).current_dir(&frere).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&tete.stdout).trim(), "lot/neuve");
        assert!(dit.contains("Branche lot/neuve ouverte dans le worktree") && dit.contains("worktree add -b lot/neuve") && dit.contains("(commande : `git -C "), "la commande est dite : {dit}");
        assert!(dit.contains("Worktree attaché") && racines.manifest("code-rag3db-essai-lot-neuve").is_file(), "le worktree est attaché : {dit}");
        assert!(!dit.contains("backend"), "le vocabulaire de l'utilisatrice : {dit}");

        let encore = appel(json!({ "path": depot, "branch": "lot/neuve" })).expect("rouvrir");
        assert!(encore.contains("déjà ouverte") && encore.contains("aucune commande git"), "{encore}");
        let refus = appel(json!({ "path": depot, "branch": "a..b" })).expect_err("un nom invalide");
        assert!(refus.contains("pas un nom de branche valide"), "{refus}");
    }

    /// Les `RepoIndexed` captés sur le flux (l'événement de chaque trame).
    fn indexes_captes(flux: &Arc<Mutex<Vec<u8>>>) -> Vec<Value> {
        let texte = String::from_utf8_lossy(&flux.lock().unwrap()).into_owned();
        texte
            .lines()
            .filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")))
            .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
            .map(|v| v.get("evenement").cloned().unwrap_or(v))
            .filter(|e| e["kind"] == "RepoIndexed")
            .collect()
    }

    /// Un petit dépôt git d'un fichier, sur sa branche `essai`.
    fn petit_depot(d: &Path) -> std::path::PathBuf {
        let depot = d.join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n\npub fn deux() -> u32 { 2 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        depot
    }

    /// **La fin du premier index se dit sur le flux** (Lucie, 8 novembre 2026 à 16 h 08 :
    /// « il devrait avoir notif quand indexation finie ? »). Un worktree attaché par
    /// l'atelier : quand la dernière ligne du journal de son index arrive, `RepoIndexed`
    /// passe sur `repos`, fini, avec le dépôt, la branche, les fichiers et les symboles de
    /// l'index et sa durée. Un second appel (« déjà attaché ») ne relance rien.
    #[test]
    fn the_end_of_the_first_index_of_an_attached_worktree_is_published() {
        let d = tempfile::tempdir().unwrap();
        let depot = petit_depot(d.path());
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&atelier).unwrap();
        let dit = host.with_backend(&h, |b| b.call("attach_worktree", json!({ "path": depot }))).unwrap().unwrap();
        assert!(dit["result"].as_str().unwrap_or_default().contains("sa fin se dira sur le flux de l'IDE"), "{dit}");

        let debut = std::time::Instant::now();
        let fin = loop {
            if let Some(e) = indexes_captes(&flux).into_iter().next() {
                break e;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(60), "aucun RepoIndexed en 60 s : {}", String::from_utf8_lossy(&flux.lock().unwrap()));
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        assert_eq!(fin["ok"], true, "{fin}");
        assert_eq!(fin["depot"], "rag3db-essai", "{fin}");
        assert_eq!(fin["branche"], "essai", "{fin}");
        assert!(fin["fichiers"].as_u64().is_some_and(|n| n >= 1), "les fichiers de l'index : {fin}");
        assert!(fin["symboles"].as_u64().is_some_and(|n| n >= 2), "les symboles de l'index : {fin}");
        assert!(fin["ms"].is_u64() && fin["erreur"].is_null(), "{fin}");

        // Déjà attaché : pas de second index, pas de second RepoIndexed.
        host.with_backend(&h, |b| b.call("attach_worktree", json!({ "path": depot }))).unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(800));
        assert_eq!(indexes_captes(&flux).len(), 1, "un worktree déjà attaché ne relance pas d'index");
    }

    /// **Le suiveur tient le contrat du journal d'index** : un échec et un arrêt se disent
    /// `ok: false` avec leur ligne ; un index fini avec un reste dû des vecteurs est fini
    /// (`ok: true`, `reste`) ; un journal qui ne finit pas se dit au plafond ; l'hôte parti,
    /// rien n'est publié.
    #[test]
    fn the_index_follower_keeps_the_contract_of_the_index_journal() {
        use crate::dataflow::index_nodes::{DONE_LINE, FAILED_PREFIX, STOPPED_PREFIX};
        let d = tempfile::tempdir().unwrap();
        let depot = petit_depot(d.path());
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        std::fs::create_dir_all(d.path().join("base")).unwrap();
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &d.path().join("base")).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(depot.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        let host = Arc::new(hote(d.path(), model));
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&manifeste).unwrap();
        let quoi = |n: &str| json!({ "worktree": format!("/w/{n}"), "depot": "rag3db-essai", "branche": n });
        let bornes = |plafond_ms: u64| super::Bornes { plafond: std::time::Duration::from_millis(plafond_ms), naissance: std::time::Duration::from_millis(400) };
        let suivre = |n: &str, lignes: &[String], plafond_ms: u64| {
            let journal = d.path().join(format!("{n}.log"));
            std::fs::write(&journal, lignes.join("\n") + "\n").unwrap();
            super::suivre_le_premier_index(Arc::downgrade(&host), h.clone(), journal, quoi(n), bornes(plafond_ms));
        };
        let long = 60_000;
        suivre("echec", &["plein texte : 1/1 fichiers".into(), format!("{FAILED_PREFIX} : la racine a disparu")], long);
        suivre("arret", &[format!("{STOPPED_PREFIX} : l'hôte s'arrête")], long);
        suivre("reste", &[format!("{DONE_LINE} — 12 morceau(x) sans vecteur, l'embarqueur n'a pas répondu")], long);
        suivre("plafond", &["plein texte : 0/1 fichiers".into()], 300);
        // Un journal qui ne paraît jamais : l'échec se dit après la naissance, pas au plafond.
        super::suivre_le_premier_index(Arc::downgrade(&host), h.clone(), d.path().join("absent.log"), quoi("absent"), bornes(long));
        // Un gros journal (plus que la queue gardée) : le plein texte prêt au milieu, la fin
        // tout au bout — lus par l'ajout, sans relire le début.
        let mut gros: Vec<String> = (0..2000).map(|i| format!("vecteurs {i} sur 2000 (granite-278m)")).collect();
        gros.insert(10, "plein texte prêt · relations 4/4".into());
        gros.push(DONE_LINE.into());
        suivre("gros", &gros, long);

        let debut = std::time::Instant::now();
        let vus = loop {
            let vus = indexes_captes(&flux);
            if vus.len() >= 6 {
                break vus;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(20), "six RepoIndexed attendus : {vus:#?}");
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        let de = |n: &str| vus.iter().find(|e| e["branche"] == n).cloned().unwrap_or_else(|| panic!("{n} : {vus:#?}"));
        let e = de("echec");
        assert_eq!((e["ok"].clone(), e["erreur"].as_str().map(|s| s.starts_with(FAILED_PREFIX))), (json!(false), Some(true)), "{e}");
        let e = de("arret");
        assert_eq!((e["ok"].clone(), e["erreur"].as_str().map(|s| s.starts_with(STOPPED_PREFIX))), (json!(false), Some(true)), "{e}");
        let e = de("reste");
        assert_eq!(e["ok"], true, "un reste dû des vecteurs n'est pas un échec : {e}");
        assert!(e["reste"].as_str().is_some_and(|r| r.contains("12 morceau(x)")) && e["erreur"].is_null(), "{e}");
        let e = de("plafond");
        assert!(e["ok"] == false && e["erreur"].as_str().is_some_and(|s| s.contains("n'a pas fini")), "{e}");
        let e = de("absent");
        assert!(e["ok"] == false && e["erreur"].as_str().is_some_and(|s| s.contains("n'existe pas")), "{e}");
        let e = de("gros");
        assert!(e["ok"] == true && e["texte_ms"].is_u64(), "le plein texte prêt lu au milieu d'un gros journal : {e}");

        // L'hôte parti, le suiveur sort sans rien publier.
        let journal = d.path().join("parti.log");
        std::fs::write(&journal, "plein texte : 0/1 fichiers\n").unwrap();
        let faible = Arc::downgrade(&host);
        super::suivre_le_premier_index(faible.clone(), h.clone(), journal.clone(), quoi("parti"), bornes(long));
        let flux2 = flux.clone();
        drop(host);
        std::thread::sleep(std::time::Duration::from_millis(600));
        assert!(faible.upgrade().is_none(), "l'hôte est parti");
        std::fs::write(&journal, format!("{DONE_LINE}\n")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        assert!(!indexes_captes(&flux2).iter().any(|e| e["branche"] == "parti"), "un hôte parti ne publie rien");
    }

    /// **L'avancement se lit dans les lignes du journal**, aux formes que `index_nodes` et
    /// `catalog::progress` écrivent.
    #[test]
    fn the_progress_of_an_index_reads_from_its_journal_lines() {
        use super::{avancement_de_la_ligne, Avancement};
        assert_eq!(avancement_de_la_ligne("plein texte : 12 fichiers sur 40 (88 scopes) · 3 liens en file"), Some(Avancement { phase: "plein texte", fait: 12, total: 40 }));
        assert_eq!(
            avancement_de_la_ligne("plein texte prêt · vecteurs 8200 sur 20100 (40 %, granite-278m) · reste environ 3 min"),
            Some(Avancement { phase: "vecteurs", fait: 8200, total: 20100 })
        );
        assert_eq!(avancement_de_la_ligne("plein texte prêt · vecteurs prêts (20100 morceaux, granite-278m)"), Some(Avancement { phase: "vecteurs", fait: 20100, total: 20100 }));
        assert_eq!(avancement_de_la_ligne("sans vecteurs (repris) · vecteurs 3 sur 9 (33 %, granite-278m)"), Some(Avancement { phase: "vecteurs", fait: 3, total: 9 }));
        assert_eq!(avancement_de_la_ligne("indexation lancée : 40 fichiers, granite-278m"), None);
        assert_eq!(avancement_de_la_ligne("plein texte prêt : 40 fichiers, 88 scopes, 12 relations posées"), None);
    }

    /// **L'avancement d'un index passe sur le flux, sans polling** (Lucie, 8 novembre 2026 à
    /// 16 h 30 et 16 h 31) : le suiveur publie `IndexProgress` à mesure que le journal
    /// avance — un changement de phase tout de suite, sinon au plus un par seconde —, avec
    /// le pourcentage et le débit, puis `RepoIndexed` à la fin.
    #[test]
    fn the_progress_of_an_index_is_published_on_the_flux_as_its_journal_grows() {
        use crate::dataflow::index_nodes::DONE_LINE;
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let depot = petit_depot(d.path());
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        std::fs::create_dir_all(d.path().join("base")).unwrap();
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &d.path().join("base")).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(depot.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        let host = Arc::new(hote(d.path(), model));
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&manifeste).unwrap();
        let journal = d.path().join("index.log");
        std::fs::write(&journal, "indexation lancée : 4 fichiers, granite-278m\n").unwrap();
        let bornes = super::Bornes { plafond: std::time::Duration::from_secs(60), naissance: std::time::Duration::from_secs(5) };
        super::suivre_le_premier_index(Arc::downgrade(&host), h.clone(), journal.clone(), json!({ "worktree": "/w", "depot": "rag3db-essai", "branche": "essai" }), bornes);
        let ecrire = |l: &str| {
            let mut f = std::fs::OpenOptions::new().append(true).open(&journal).unwrap();
            writeln!(f, "{l}").unwrap();
        };
        let pause = |ms: u64| std::thread::sleep(std::time::Duration::from_millis(ms));
        // Le plein texte : dix lignes en 300 ms — la cadence n'en publie pas dix.
        for i in 1..=10 {
            ecrire(&format!("plein texte : {i} fichiers sur 10 (0 scopes)"));
            pause(30);
        }
        pause(1300);
        // Les vecteurs : un changement de phase, publié tout de suite, puis l'avance.
        ecrire("plein texte prêt · vecteurs 100 sur 1000 (10 %, granite-278m)");
        pause(1200);
        ecrire("plein texte prêt · vecteurs 600 sur 1000 (60 %, granite-278m) · reste environ 30 s");
        pause(1200);
        ecrire(DONE_LINE);
        let evenements = || -> Vec<Value> {
            let texte = String::from_utf8_lossy(&flux.lock().unwrap()).into_owned();
            texte.lines().filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:"))).filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok()).map(|v| v.get("evenement").cloned().unwrap_or(v)).collect()
        };
        let debut = std::time::Instant::now();
        while !evenements().iter().any(|e| e["kind"] == "RepoIndexed") {
            assert!(debut.elapsed() < std::time::Duration::from_secs(10), "pas de RepoIndexed : {:#?}", evenements());
            pause(100);
        }
        let tout = evenements();
        let progres: Vec<&Value> = tout.iter().filter(|e| e["kind"] == "IndexProgress").collect();
        let texte: Vec<&&Value> = progres.iter().filter(|e| e["phase"] == "plein texte").collect();
        let vecteurs: Vec<&&Value> = progres.iter().filter(|e| e["phase"] == "vecteurs").collect();
        assert!(!texte.is_empty() && texte.len() <= 3, "la cadence borne le plein texte (dix lignes en 300 ms) : {texte:#?}");
        assert_eq!(texte.last().unwrap()["fait"], 10, "le dernier état du plein texte est publié : {texte:#?}");
        assert_eq!(vecteurs.len(), 2, "le changement de phase, puis l'avance : {vecteurs:#?}");
        assert_eq!((vecteurs[0]["fait"].clone(), vecteurs[0]["total"].clone(), vecteurs[0]["pourcent"].clone()), (json!(100), json!(1000), json!(10)));
        assert_eq!(vecteurs[1]["pourcent"], 60);
        assert!(vecteurs[1]["debit"].as_f64().is_some_and(|d| d > 100.0 && d < 1000.0), "500 morceaux en ~1,2 s : {}", vecteurs[1]);
        assert!(vecteurs[0]["debit"].is_null(), "pas de débit sur un changement de phase : {}", vecteurs[0]);
        assert_eq!((vecteurs[1]["depot"].clone(), vecteurs[1]["unite"].clone()), (json!("rag3db-essai"), json!("morceaux")));
        let i_fin = tout.iter().position(|e| e["kind"] == "RepoIndexed").unwrap();
        assert!(tout.iter().rposition(|e| e["kind"] == "IndexProgress").unwrap() < i_fin, "RepoIndexed vient après les avancements");
    }

    /// **L'avancement de l'index se lit dans la barre, par le flux** (Lucie, 8 novembre 2026
    /// à 16 h 30 et 16 h 31) : la ligne du dépôt dit la phase, le pourcentage et le débit à
    /// mesure que le journal avance, « figé depuis » quand rien ne bouge, et s'efface à la
    /// fin — la page jamais rechargée, aucune relecture de l'hôte.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn l_avancement_de_l_index_se_lit_dans_la_barre_par_le_flux() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        use crate::dataflow::index_nodes::DONE_LINE;
        use std::io::Write;
        const TEMOIN: &str = "l_avancement_de_l_index_se_lit_dans_la_barre_par_le_flux";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let depot = petit_depot(d.path());
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        std::fs::create_dir_all(d.path().join("base")).unwrap();
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &d.path().join("base")).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(depot.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new("ok"))) });
        let host = Arc::new(crate::daemon_host::tests::hote_isole().with_chats_root(d.path().join("chats")).with_racine_des_taches(d.path().join("taches")).with_chat_model(model));
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let racine = format!("http://{}", ecoute.local_addr().unwrap());
        let serveur = tiny_http::Server::from_listener(ecoute, None).unwrap();
        let servi = host.clone();
        let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(serveur, 2).serve(servi, Some("j".to_string())));
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{racine}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste }).to_string())
            .unwrap();
        let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(v["ok"], true, "{v}");
        let h = host.hosted(&manifeste.canonicalize().unwrap()).unwrap();

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.aller(&format!("{racine}/ide/code/{}/chat", h.id), "!!document.querySelector('#barre') && document.getElementById('lien')?.textContent === 'flux : ouvert'");
        n.evaluer("window.__ideFigeMs = 1500");
        let journal = d.path().join("index.log");
        std::fs::write(&journal, "indexation lancée : 1 fichiers, granite-278m\n").unwrap();
        let bornes = super::Bornes { plafond: std::time::Duration::from_secs(60), naissance: std::time::Duration::from_secs(5) };
        super::suivre_le_premier_index(Arc::downgrade(&host), h.clone(), journal.clone(), json!({ "worktree": depot, "depot": "rag3db-essai", "branche": "essai" }), bornes);
        let ecrire = |l: &str| {
            let mut f = std::fs::OpenOptions::new().append(true).open(&journal).unwrap();
            writeln!(f, "{l}").unwrap();
        };
        ecrire("plein texte prêt · vecteurs 100 sur 1000 (10 %, granite-278m)");
        n.attendre("document.querySelector('.avancement')?.dataset.pourcent === '10'", "l'avancement à 10 % dans la barre");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        ecrire("plein texte prêt · vecteurs 600 sur 1000 (60 %, granite-278m) · reste environ 30 s");
        n.attendre("document.querySelector('.avancement')?.dataset.pourcent === '60' && document.querySelector('.avancement').textContent.includes('morceaux/s')", "60 % et le débit");
        let texte = n.evaluer("document.querySelector('.avancement').textContent");
        assert!(texte.as_str().is_some_and(|t| t.contains("vecteurs 60 % (600/1000 morceaux)")), "{texte}");
        n.attendre("document.querySelector('.avancement')?.textContent.includes('figé depuis')", "« figé depuis » quand rien ne bouge");
        ecrire(DONE_LINE);
        n.attendre("!document.querySelector('.avancement')", "l'avancement s'efface à la fin de l'index");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **La phrase de l'index ne compte que les entités, et les vecteurs déclarés** (c7, 8
    /// novembre 2026, 16 h 12 : sur le poste de Lucie, un index prêt était dit « plein
    /// texte : 4/7, vecteurs : 3/7 », et le modèle en a fait « indexation en cours »). L'état
    /// exact de `/etat` : quatre entités prêtes, Symbol sans vecteurs déclarés, trois clés
    /// d'état qui ne sont pas des entités.
    #[test]
    fn the_index_phrase_counts_entities_and_declared_vectors_only() {
        let pret = |vecteurs: &str| json!({ "text": "ready", "vectors": vecteurs, "relations": "ready", "vectors_percent": if vecteurs == "ready" { 100 } else { 0 }, "updated_ms": 1 });
        let manifeste = Path::new("/x/backend.json");
        let etat = |index: Value| json!({ "backends": [{ "manifest": "/x/backend.json", "open": true, "index": index }] });
        let index = json!({
            "File": pret("ready"), "Library": pret("ready"), "Scope": pret("ready"), "Symbol": pret("not_declared"),
            "fts_counts": { "File": 10 }, "reads": { "n": 3 }, "watches": {},
        });
        assert_eq!(super::etat_de_l_index(&etat(index), manifeste), "plein texte prêt (4/4), vecteurs prêts (3/3 déclarés)");
        // En cours seulement quand un pourcentage le dit.
        let mut scope = pret("running");
        scope["vectors_percent"] = json!(42);
        let index = json!({ "File": pret("ready"), "Scope": scope.clone(), "Symbol": pret("not_declared"), "indexings": 1 });
        assert_eq!(super::etat_de_l_index(&etat(index), manifeste), "plein texte prêt (3/3), vecteurs 1/2 déclarés, en cours, 42 %");
        // La même note sans indexation vivante (wt-react5, 16 h 30) : pas « en cours ».
        let index = json!({ "File": pret("ready"), "Scope": scope, "Symbol": pret("not_declared") });
        assert_eq!(super::etat_de_l_index(&etat(index), manifeste), "plein texte prêt (3/3), vecteurs 1/2 déclarés, 42 % faits, rien ne tourne");
        // Une écriture qui tient le verrou : ni zéro ni « pas indexé ».
        let dit = super::etat_de_l_index(&etat(json!({ "busy": true, "watches": {} })), manifeste);
        assert!(dit.starts_with("indexation en cours"), "{dit}");
        // Le premier index d'un worktree : les entités encore jamais écrites, une
        // indexation qui tourne (c7, 16 h 30 : « plein texte 0/4, vecteurs 0/4 »).
        let jamais = json!({ "text": "never", "vectors": "never", "vectors_percent": 0, "updated_ms": 1 });
        let index = json!({ "File": jamais, "Scope": jamais, "indexings": 1 });
        assert_eq!(super::etat_de_l_index(&etat(index), manifeste), "plein texte 0/2, vecteurs 0/2 déclarés ; indexation en cours");
        assert_eq!(super::etat_de_l_index(&etat(json!({ "indexings": 1, "watches": {} })), manifeste), "indexation en cours, rien de cherchable encore");
    }

    /// L'atelier posé comme le pose `rag3weaver-backend atelier` : le gabarit livré
    /// copié sous `<config>/atelier/templates`, la racine de ses commandes donnée.
    fn atelier_pose(d: &Path, racine: &Path) -> (PathBuf, crate::code_tree::Roots) {
        let racines = crate::code_tree::Roots { config: d.join("config"), data: d.join("data") };
        let gabarits = racines.config.join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let atelier = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&atelier).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.display().to_string());
        std::fs::write(&atelier, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        (atelier, racines)
    }

    /// **Un atelier posé à neuf ne liste aucun worktree** (c7, 8 novembre 2026 : chez
    /// Lucie, list_worktrees rendait `…/atelier/templates/backends/code/workspace — dépôt
    /// hors git`, le gabarit de code copié avec l'atelier pris pour un arbre inscrit).
    #[test]
    fn a_fresh_workshop_lists_no_worktree() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("poste");
        std::fs::create_dir_all(&racine).unwrap();
        let (atelier, racines) = atelier_pose(d.path(), &racine);
        assert!(racines.registered_manifests().is_empty(), "{:?}", racines.registered_manifests());
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new("rien"))) });
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let h = host.hosted(&atelier).unwrap();
        let liste = host.with_backend(&h, |b| b.call("list_worktrees", json!({}))).unwrap().unwrap();
        assert_eq!(liste["result"].as_str(), Some("Aucun worktree attaché : attach_worktree en attache un."), "{liste}");
    }

    /// **Le chat de l'atelier lance une commande hors de toute tâche, par sa porte
    /// déclarée** (Lucie, 8 novembre 2026, 16 h 02 : « qu'il ait commande shell »). Le
    /// gabarit livré déclare `commands: "approval"` et `commands_outside_tasks` : `ls` (la
    /// liste sûre) passe et rend la sortie ; le message système dit l'ordre des outils et
    /// qu'on ne renvoie jamais au terminal. Un dépôt de code, sans la clé, garde ses
    /// commandes dans ses tâches (`chat::tests`).
    #[test]
    fn the_workshop_chat_runs_a_command_outside_a_task() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("poste");
        std::fs::create_dir_all(&racine).unwrap();
        std::fs::write(racine.join("temoin-de-la-porte.txt"), "là\n").unwrap();
        let (atelier, racines) = atelier_pose(d.path(), &racine);
        let (vus, systeme) = (Arc::new(Mutex::new(Vec::<String>::new())), Arc::new(Mutex::new(String::new())));
        let model: ChatModel = {
            let (vus, systeme) = (vus.clone(), systeme.clone());
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (vus, systeme) = (vus.clone(), systeme.clone());
                Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                    if let Some(t) = turns.first().filter(|t| t.role == "system") {
                        *systeme.lock().unwrap() = t.content.clone();
                    }
                    let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                    match turns[depuis..].iter().filter(|t| t.role == "tool").last() {
                        None => MockLlm::new("").with_tool_calls(vec![("run_command".to_string(), json!({ "command": "ls" }).to_string())]).generate(turns, opts, sink),
                        Some(r) => {
                            vus.lock().unwrap().push(r.content.clone());
                            MockLlm::new("Voilà.").generate(turns, opts, sink)
                        }
                    }
                })))
            })
        };
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let h = host.hosted(&atelier).unwrap();
        host.chat_turn(&h.id, "Liste ce qu'il y a ici").expect("le tour");
        let vu = vus.lock().unwrap().first().cloned().unwrap_or_default();
        assert!(!vu.contains("le chat n'en lance pas"), "la porte de l'atelier est fermée hors tâche : {vu}");
        assert!(vu.contains("temoin-de-la-porte.txt"), "la sortie de ls : {vu}");
        let systeme = systeme.lock().unwrap().clone();
        assert!(systeme.contains("run_command") && systeme.contains("list_worktrees") && systeme.contains("faites-le dans votre terminal"), "la consigne de l'atelier : {systeme}");
    }

    /// **L'atelier posé par un binaire d'avant a les outils du binaire qui l'ouvre** (Lucie,
    /// 8 novembre 2026, 16 h 12 : « faut que ce soit dynamique »). Posé sans open_branch,
    /// resté tel quel : l'hôte qui l'ouvre le remet au gabarit livré avant de le lire, le
    /// backend déclare open_branch, et `TemplateRefreshed` passe sur le flux.
    #[test]
    fn a_workshop_posed_before_gets_the_tools_of_the_binary_that_opens_it() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("poste");
        std::fs::create_dir_all(&racine).unwrap();
        let (atelier, racines) = atelier_pose(d.path(), &racine);
        let mut m: Value = serde_json::from_slice(&std::fs::read(&atelier).unwrap()).unwrap();
        m["tools"].as_object_mut().unwrap().remove("open_branch");
        std::fs::write(&atelier, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        crate::builtin_templates::record(&atelier, "backends/atelier/backend.json").unwrap();
        let livre = crate::builtin_templates::livre_of(&atelier);
        let mut l: Value = serde_json::from_str(&std::fs::read_to_string(&livre).unwrap()).unwrap();
        l["content"]["tools"].as_object_mut().unwrap().remove("open_branch");
        std::fs::write(&livre, l.to_string()).unwrap();

        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new("rien"))) });
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&atelier).unwrap();
        let outils: Vec<String> = host.with_backend(&h, |b| b.prepared().manifest.tools.keys().cloned().collect()).unwrap();
        assert!(outils.iter().any(|o| o == "open_branch"), "{outils:?}");
        let m: Value = serde_json::from_slice(&std::fs::read(&atelier).unwrap()).unwrap();
        assert_eq!(m["workspace"]["root"], json!(racine.display().to_string()), "le réglage de la pose reste");
        let capte = String::from_utf8_lossy(&flux.lock().unwrap()).into_owned();
        assert!(capte.contains("TemplateRefreshed") && capte.contains("open_branch"), "{capte}");
    }


    /// **Un worktree attaché dont le premier index a été coupé le reprend** (c7, 8
    /// novembre 2026, 16 h 09 : attaché à 16 h 04, l'hôte reposé à 16 h 08, list_worktrees
    /// disait « pas ouvert » et rien ne reprenait). L'attache pose le témoin du journal ;
    /// l'index coupé (dernière ligne « indexation arrêtée »), le worktree fermé :
    /// list_worktrees le rouvre et relance l'index, qui finit ; au démarrage de l'hôte, la
    /// reprise fait de même ; attach_worktree sur le worktree fermé le rouvre et relance.
    #[test]
    fn a_worktree_whose_first_index_was_cut_resumes_it() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 { 1 }\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let racine = d.path().join("poste");
        std::fs::create_dir_all(&racine).unwrap();
        let (atelier, racines) = atelier_pose(d.path(), &racine);
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new("rien"))) });
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines.clone()).with_runs_root(d.path().join("runs")));
        host.se_connaitre();
        let h = host.hosted(&atelier).unwrap();
        let outil = |nom: &str, args: Value| -> String {
            host.with_backend(&h, |b| b.call(nom, args)).unwrap().map(|r| r["result"].as_str().unwrap_or_default().to_string()).unwrap_or_else(|e| e)
        };
        outil("attach_worktree", json!({ "path": depot }));
        let manifeste = racines.manifest("code-rag3db-essai").canonicalize().unwrap();
        let marque = super::marque_du_premier_index(&manifeste);
        assert!(!marque.starts_with(manifeste.parent().unwrap().parent().unwrap().parent().unwrap()), "le témoin vit hors de l'arbre des déclarations : {}", marque.display());
        let journal_de = || std::fs::read_to_string(&marque).map(|j| j.trim().to_string()).unwrap_or_default();
        assert!(marque.is_file(), "l'attache pose le témoin du journal de son index");
        let fini = |journal: &str| std::fs::read_to_string(journal).ok().and_then(|t| t.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.starts_with(crate::dataflow::index_nodes::DONE_LINE))).unwrap_or(false);
        let attendre = |journal: &str| {
            let debut = std::time::Instant::now();
            while !fini(journal) {
                assert!(debut.elapsed() < std::time::Duration::from_secs(60), "l'index n'a pas fini en 60 s : {journal}");
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        };
        attendre(&journal_de());
        // L'index coupé, le worktree fermé : ce qu'un hôte reposé laisse.
        let coupe = d.path().join("coupe.out");
        let couper = || {
            std::fs::write(&coupe, format!("indexation lancée\n{} : l'hôte s'arrête\n", crate::dataflow::index_nodes::STOPPED_PREFIX)).unwrap();
            std::fs::write(&marque, coupe.display().to_string()).unwrap();
            let s = host.open_session("le témoin", None, Vec::new(), None);
            assert_eq!(host.close(Some(&s), &manifeste)["ok"], true);
        };

        // 1. list_worktrees le rouvre et relance l'index.
        couper();
        let liste = outil("list_worktrees", json!({}));
        assert!(liste.contains("worktree(s) attaché(s)") && !liste.contains("pas ouvert"), "{liste}");
        // Le journal neuf : l'ouverture le pose en relançant l'index.
        let debut = std::time::Instant::now();
        let neuf = loop {
            let j = journal_de();
            if !j.is_empty() && j != coupe.display().to_string() {
                break j;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(60), "pas de journal neuf en 60 s : {j} ; list_worktrees : {liste}");
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        attendre(&neuf);

        // 2. Au démarrage de l'hôte, la reprise.
        couper();
        let repris = host.reprendre_les_premiers_index();
        assert_eq!(repris.len(), 1, "{repris:?}");
        attendre(&journal_de());

        // 3. attach_worktree sur le worktree fermé le rouvre et relance.
        couper();
        let dit = outil("attach_worktree", json!({ "path": depot }));
        assert!(dit.contains("il est rouvert") && dit.contains("son index est lancé en fond"), "{dit}");
        // L'index relancé à l'ouverture, l'outil l'attend : il n'est plus un rattrapage qui
        // cède à une fermeture.
        let hote_du_depot = host.hosted_existing(&manifeste).expect("le dépôt est hébergé");
        assert!(!hote_du_depot.rattrapage.load(std::sync::atomic::Ordering::SeqCst), "l'index qu'attend attach_worktree reste marqué rattrapage");
        attendre(&journal_de());
        // Un index fini : plus rien à reprendre, le témoin s'ôte.
        assert!(host.reprendre_les_premiers_index().is_empty());
        assert!(!marque.exists(), "le témoin d'un index fini s'ôte");
    }


    /// Un faux service d'embarquement qui sert granite-278m (768) : des vecteurs
    /// unitaires, pour que la dette se solde sans modèle.
    struct Granite;
    impl crate::embedder::Embedder for Granite {
        fn name(&self) -> &str {
            "granite-278m"
        }
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> {
            // Lent exprès : un rattrapage dure assez pour qu'une fermeture le croise.
            std::thread::sleep(std::time::Duration::from_millis(2 * texts.len() as u64));
            Ok(texts.iter().map(|_| { let mut v = vec![0.0_f32; 768]; v[0] = 1.0; v }).collect())
        }
        fn dim(&self) -> usize {
            768
        }
    }

    fn service_granite() -> String {
        servir(Arc::new(Granite))
    }

    fn servir(modele: Arc<dyn crate::embedder::Embedder>) -> String {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let _ = crate::daemon::EmbedDaemon::new(modele).servir_lie(ecoute);
        });
        let debut = std::time::Instant::now();
        while crate::daemon::DaemonEmbedder::joindre(&adresse).is_err() {
            assert!(debut.elapsed() < std::time::Duration::from_secs(10), "le service factice ne répond pas");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        adresse
    }

    /// **Des vecteurs dus se soldent à la réouverture quand le manifeste le déclare**
    /// (c7, 8 novembre 2026 : wt-react5, rouvert après la coupure de 16 h 08, restait à 15 %
    /// de vecteurs ; la forme (b), `embeddings.catch_up_on_open`). Un index sans service
    /// d'embarquement laisse tous ses vecteurs dus ; fermé, puis rouvert avec le service
    /// et sans le réglage : rien ne part. Rouvert avec le réglage : l'index repart et la
    /// dette dense tombe à zéro.
    #[test]
    fn owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it() {
        let d = tempfile::tempdir().unwrap();
        let (manifeste, arbre) = crate::daemon_host::symboles_cites::tests::petit_depot(d.path());
        let manifeste = PathBuf::from(manifeste);
        // Assez de morceaux pour que, coupé après au plus une passe de 512 à l'étape 1, il
        // en reste plusieurs à l'étape 3 (la CI, plus lente, coupait après la première).
        for i in 0..1500 {
            std::fs::write(arbre.join(format!("src/f{i}.rs")), format!("pub fn fonction_{i}() -> u32 {{\n    {i}\n}}\n")).unwrap();
        }
        let ecrire = |f: &dyn Fn(&mut Value)| {
            let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
            f(&mut m);
            std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        };
        // 1. Un index coupé entre deux passes de vecteurs, comme par l'arrêt de l'hôte : le
        // plein texte, et des vecteurs dus (wt-react5, 8 novembre 2026). Le service lent est
        // déclaré au manifeste : l'environnement de la passe n'y change rien (archi), et
        // aucun morceau n'est refusé.
        let adresse = service_granite();
        ecrire(&|m| {
            m["embeddings"]["catch_up_on_open"] = json!(false);
            m["embeddings"]["address"] = json!(adresse);
        });
        let host = Arc::new(crate::daemon_host::tests::hote_isole());
        let h = host.hosted(&manifeste).unwrap();
        let attendre_l_index = || {
            let debut = std::time::Instant::now();
            while host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
                assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index n'a pas fini en 120 s");
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        };
        let recu = host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
        let journal = recu["result"].as_str().unwrap_or_default().lines().find_map(|l| l.strip_prefix("journal : ")).unwrap().trim().to_string();
        let debut = std::time::Instant::now();
        while !std::fs::read_to_string(&journal).is_ok_and(|t| t.contains("plein texte prêt")) {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "le plein texte n'est pas prêt en 120 s : {journal}");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // L'arrêt de l'hôte : au paquet suivant, la dette laissée.
        let catalogue = host.with_backend(&h, |b| b.catalog.clone()).unwrap();
        crate::dataflow::index_nodes::stop_indexings_of(&catalogue);
        drop(catalogue);
        attendre_l_index();
        let dus = || host.with_backend(&h, |b| b.catalog.lock().unwrap().index_progress().unwrap().dense_missing()).unwrap();
        assert!(dus() > 0, "l'index coupé laisse des vecteurs dus");
        let fermer = || {
            let s = host.open_session("le témoin", None, Vec::new(), None);
            assert_eq!(host.close(Some(&s), &manifeste)["ok"], true);
        };
        fermer();

        // 2. Rouvert, sans le réglage : rien ne part.
        assert_eq!(host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap(), 0, "sans le réglage, aucun index ne part à l'ouverture");
        let avant = dus();
        assert!(avant > 0);
        fermer();

        // 3. Rouvert avec le réglage : l'index repart de lui-même. Il ne tient pas la
        // porte : une fermeture l'arrête au paquet suivant, et la dette reste.
        ecrire(&|m| m["embeddings"]["catch_up_on_open"] = json!(true));
        host.with_backend(&h, |_| ()).unwrap();
        assert!(host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0, "avec le réglage, l'index repart à l'ouverture");
        // Son journal, par le témoin du premier index que l'ouverture a posé.
        let journal_du_rattrapage = std::fs::read_to_string(super::marque_du_premier_index(&manifeste)).expect("le rattrapage a posé son témoin");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let debut = std::time::Instant::now();
        fermer();
        assert!(debut.elapsed() < std::time::Duration::from_secs(30), "la fermeture a attendu {:?}", debut.elapsed());
        // La coupure se lit à la dernière ligne du journal, sans rouvrir le dépôt : une
        // réouverture verrait le témoin du premier index et relancerait l'index, qui
        // solderait la dette sous la mesure (la CI de master, 9 novembre 2026).
        let derniere = std::fs::read_to_string(journal_du_rattrapage.trim()).unwrap_or_default().lines().rev().find(|l| !l.trim().is_empty()).unwrap_or_default().to_string();
        assert!(derniere.starts_with(crate::dataflow::index_nodes::STOPPED_PREFIX), "la fermeture a coupé le rattrapage, et la dette reste : {derniere}");

        // 4. Rouvert avec le réglage, laissé faire : la dette se solde.
        ecrire(&|m| m["embeddings"]["catch_up_on_open"] = json!(true));
        let debut = std::time::Instant::now();
        host.with_backend(&h, |_| ()).unwrap();
        while dus() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "la dette n'est pas soldée en 120 s : {} dus", dus());
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        attendre_l_index();
    }


    /// **Une note « en cours » d'avant un arrêt ne se lit pas comme un index qui tourne**
    /// (c7, 8 novembre 2026 : code-essais-c7 rendait `text: running` noté le 25 octobre à
    /// 23 h 21, ses comptes complets, rien ne tournant ; list_worktrees disait « plein texte
    /// 0/7 »). Un index fini, puis la note du mois dernier posée sur chaque entité : l'état
    /// dit prêt d'après les comptes, et la phrase « plein texte prêt ».
    #[test]
    fn a_stale_running_note_without_a_live_index_reads_ready() {
        let d = tempfile::tempdir().unwrap();
        let (manifeste, _) = crate::daemon_host::symboles_cites::tests::petit_depot(d.path());
        let manifeste = PathBuf::from(manifeste);
        let host = Arc::new(crate::daemon_host::tests::hote_isole());
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
        let debut = std::time::Instant::now();
        while host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index n'a pas fini en 120 s");
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let entites = [crate::code::FILE, crate::code::SCOPE, crate::code::LIBRARY, crate::code::SYMBOL];
        host.with_backend(&h, |b| {
            let cat = b.catalog.lock().unwrap();
            for e in entites {
                let mut note = cat.index_state_for(e).unwrap();
                note.text = crate::catalog::Level::Running;
                note.text_percent = None;
                note.updated_ms = 1_792_966_868_365;
                cat.note_index_state_for(e, note).unwrap();
            }
        })
        .unwrap();
        let etats = host.with_backend(&h, |b| b.index_states()).unwrap().unwrap();
        assert!(etats["fts_counts"].is_object(), "les comptes du dernier index : {etats:#}");
        for e in [crate::code::FILE, crate::code::SCOPE] {
            assert_eq!(etats[e]["text"], "ready", "{e} : {etats:#}");
        }
        let etat = json!({ "backends": [{ "manifest": manifeste, "open": true, "index": etats }] });
        let dit = super::etat_de_l_index(&etat, &manifeste);
        assert!(dit.starts_with("plein texte prêt"), "{dit} ; l'état : {etats:#}");
        assert!(!dit.contains("en cours"), "{dit}");
    }


    /// **La carte de l'avancement passe à la page, jamais au modèle** (Lucie, 8 novembre
    /// 2026, 16 h 30 et 16 h 31). Dans le chat de l'atelier, le modèle attache un worktree
    /// dont l'index part : le tour `tool` de la session porte `ui` (`index_progress`, l'id
    /// du dépôt), `ToolCallFinished` aussi sur le flux, et aucun tour que voit le modèle
    /// ne dit « index_progress ». `show_progress` rend la même carte à la demande.
    #[test]
    fn a_progress_card_reaches_the_page_and_never_the_model() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        for i in 0..200 {
            std::fs::write(depot.join(format!("src/f{i}.rs")), format!("pub fn f{i}() -> u32 {{ {i} }}\n")).unwrap();
        }
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        let racine = d.path().join("poste");
        std::fs::create_dir_all(&racine).unwrap();
        let (atelier, racines) = atelier_pose(d.path(), &racine);
        // Ticket activation/résidence, rouge d'union : 200 fichiers ne sont
        // pas une barrière. Garder un vrai index actif jusqu'aux deux reçus.
        struct Libere(Arc<(Mutex<bool>, std::sync::Condvar)>);
        impl Drop for Libere {
            fn drop(&mut self) {
                let (verrou, reveil) = &*self.0;
                *verrou.lock().unwrap_or_else(|p| p.into_inner()) = true;
                reveil.notify_all();
            }
        }
        struct GraniteControle {
            porte: Arc<(Mutex<bool>, std::sync::Condvar)>,
            entre: std::sync::mpsc::Sender<()>,
        }
        impl crate::embedder::Embedder for GraniteControle {
            fn name(&self) -> &str { "granite-278m" }
            fn dim(&self) -> usize { 768 }
            fn embed(&self, textes: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> {
                let _ = self.entre.send(());
                let (verrou, reveil) = &*self.porte;
                let debut = std::time::Instant::now();
                let mut libre = verrou.lock().unwrap_or_else(|p| p.into_inner());
                while !*libre {
                    let reste = std::time::Duration::from_secs(30).saturating_sub(debut.elapsed());
                    if reste.is_zero() { return Err(crate::embedder::EmbedError::Timeout); }
                    libre = reveil.wait_timeout(libre, reste).unwrap_or_else(|p| p.into_inner()).0;
                }
                Ok(textes.iter().map(|_| { let mut v = vec![0.0; 768]; v[0] = 1.0; v }).collect())
            }
        }
        let porte = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let (entre, entre_rx) = std::sync::mpsc::channel();
        let adresse = servir(Arc::new(GraniteControle { porte: porte.clone(), entre }));
        let source = racines.config.join("atelier/templates/backends/code/backend.json");
        let inscrit = crate::code_tree::add_files(&racines, &depot, &source.display().to_string(), Some("code-rag3db-essai")).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&inscrit.manifest).unwrap()).unwrap();
        m["embeddings"]["address"] = json!(adresse);
        m["embeddings"]["catch_up_on_open"] = json!(false);
        std::fs::write(&inscrit.manifest, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let vus = Arc::new(Mutex::new(String::new()));
        let chemin = depot.display().to_string();
        let model: ChatModel = {
            let vus = vus.clone();
            Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
                let (vus, chemin) = (vus.clone(), chemin.clone());
                Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                    for t in turns {
                        vus.lock().unwrap().push_str(&t.content);
                    }
                    let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                    match turns[depuis..].iter().filter(|t| t.role == "tool").count() {
                        0 => MockLlm::new("").with_tool_calls(vec![("attach_worktree".to_string(), json!({ "path": chemin }).to_string())]).generate(turns, opts, sink),
                        1 => MockLlm::new("").with_tool_calls(vec![("show_progress".to_string(), json!({ "repo": "rag3db-essai" }).to_string())]).generate(turns, opts, sink),
                        _ => MockLlm::new("C'est parti.").generate(turns, opts, sink),
                    }
                })))
            })
        };
        let host = Arc::new(hote(d.path(), model).with_racines_de_code(racines).with_runs_root(d.path().join("runs")));
        // La sonde de débit est synchrone, avant le lancement de l'index :
        // un débit de fixture noté évite de bloquer attach sur cette sonde.
        let lie = host.hosted(&inscrit.manifest).unwrap();
        host.with_backend(&lie, |b| b.catalog.lock().unwrap().note_measured_embedding_rate(
            crate::estimate::Rate::from_sample(100_000, std::time::Duration::from_secs(1)).unwrap(),
        ).unwrap()).unwrap();
        let session = host.open_session("préparation fixture carte", None, Vec::new(), None);
        assert_eq!(host.close(Some(&session), &inscrit.manifest)["ok"], true);
        host.se_connaitre();
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), None, None);
        let h = host.hosted(&atelier).unwrap();
        // Déclaré après l'hôte : une panique libère avant son arrêt/join.
        let liberation = Libere(porte);
        host.chat_turn(&h.id, &format!("Ouvre le worktree {}", depot.display())).expect("le tour");
        entre_rx.recv_timeout(std::time::Duration::from_secs(10)).expect("la vraie passe de vecteurs a atteint la barrière");
        assert!(host.with_backend(&lie, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0);

        let etat = host.chat_state(&h.id).unwrap();
        let outils: Vec<&Value> = etat["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "tool").collect();
        assert_eq!(outils.len(), 2, "{etat:#}");
        let manifeste = PathBuf::from(format!("{}/code-rag3db-essai/templates/backends/code/backend.json", d.path().join("config").display())).canonicalize().unwrap();
        let id_du_depot = host.hosted_existing(&manifeste).expect("le dépôt attaché est hébergé").id.clone();
        for t in &outils {
            let carte = &t["ui"][0];
            assert_eq!(carte["kind"], "index_progress", "{t:#}");
            assert_eq!(carte["repo"], id_du_depot, "{t:#}");
            assert_eq!(carte["titre"], "rag3db-essai · essai", "{t:#}");
        }
        assert!(!vus.lock().unwrap().contains("index_progress"), "le modèle a vu la carte");
        let capte = String::from_utf8_lossy(&flux.lock().unwrap()).into_owned();
        assert!(capte.lines().any(|l| l.contains("ToolCallFinished") && l.contains("index_progress")), "ToolCallFinished porte la carte sur le flux");
        drop(liberation);
        let debut = std::time::Instant::now();
        while host.with_backend(&lie, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(10), "l'index doit finir après libération");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        host.chat_turn(&h.id, "Montre l'état du dépôt dont l'index est terminé.").unwrap();
        let fini = host.chat_state(&h.id).unwrap();
        let dernier = fini["turns"].as_array().unwrap().iter().rev().find(|t| t["role"] == "tool" && t["tool_name"] == "show_progress").unwrap();
        assert!(dernier["ui"].as_array().is_none_or(|u| u.is_empty()), "pas de carte d'index ACTIF une fois terminé : {dernier}");
        assert!(dernier["content"].as_str().unwrap().contains("plein texte prêt"), "l'état terminé réel reste dit : {dernier}");
        assert!(!vus.lock().unwrap().contains("index_progress"), "la reprise ne transmet aucune carte au modèle");
    }

    /// **Un dépôt fermé pendant son index n'est pas rouvert pour ses comptes** (passe de la
    /// pile de l'atelier, 8 novembre 2026 : à DONE_LINE, le suiveur lisait les comptes par
    /// `with_backend`, qui rouvrait un dépôt fermé entre-temps ; la réouverture tenait le
    /// slot pendant que la reprise le croyait ouvert). Le dépôt fermé avant la fin du journal :
    /// `RepoIndexed` part fini, sans `fichiers` ni `symboles`, et le dépôt reste fermé.
    #[test]
    fn a_repo_closed_during_its_index_is_not_reopened_for_its_counts() {
        use crate::dataflow::index_nodes::DONE_LINE;
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let depot = petit_depot(d.path());
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        std::fs::create_dir_all(d.path().join("base")).unwrap();
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &d.path().join("base")).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(depot.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let manifeste = manifeste.canonicalize().unwrap();
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        let host = Arc::new(hote(d.path(), model));
        let flux = Arc::new(Mutex::new(Vec::new()));
        host.ide_flux().abonner(Box::new(Capte(flux.clone())), Some(["repos".to_string()].into_iter().collect()), None);
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le dépôt s'ouvre");
        let journal = d.path().join("index.log");
        std::fs::write(&journal, "plein texte : 0 fichiers sur 1 (0 scopes)\n").unwrap();
        let bornes = super::Bornes { plafond: std::time::Duration::from_secs(60), naissance: std::time::Duration::from_secs(5) };
        super::suivre_le_premier_index(Arc::downgrade(&host), h.clone(), journal.clone(), json!({ "worktree": depot, "depot": "rag3db-essai", "branche": "essai" }), bornes);

        // Fermé avant la fin du journal.
        let session = host.open_session("le témoin", None, vec![], None);
        let ferme = host.close(Some(&session), &manifeste);
        assert_eq!(ferme["ok"], true, "{ferme}");
        assert!(h.backend.try_read().is_ok_and(|g| g.is_none()), "le dépôt est fermé");
        writeln!(std::fs::OpenOptions::new().append(true).open(&journal).unwrap(), "{DONE_LINE}").unwrap();

        let debut = std::time::Instant::now();
        let fin = loop {
            if let Some(e) = indexes_captes(&flux).into_iter().next() {
                break e;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(10), "aucun RepoIndexed en 10 s");
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        assert_eq!(fin["ok"], true, "{fin}");
        assert!(fin["fichiers"].is_null() && fin["symboles"].is_null(), "un dépôt fermé ne donne pas de comptes (ni des zéros) : {fin}");
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(h.backend.try_read().is_ok_and(|g| g.is_none()), "le suiveur a rouvert le dépôt pour ses comptes");
        let etat = host.state();
        let ouvert = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(manifeste.as_path()))).map(|b| b["open"].clone());
        assert_eq!(ouvert, Some(json!(false)), "le dépôt est encore fermé dans l'état de l'hôte : {etat:#}");
    }

    /// **Des morceaux refusés par le service se reprennent à l'index suivant, même rouvert**
    /// (ticket « des morceaux refusés par le service ne sont jamais repris », 8 novembre
    /// 2026 ; la cause, lue le 9 : les réclamations de la passe restaient au nom de son
    /// écrivain, que le catalogue rouvert, autre écrivain, sautait une minute). Un index dont
    /// le service refuse tout, la base fermée et rouverte, puis un index avec un service
    /// sain, dans la minute : la dette tombe à zéro.
    #[test]
    fn refused_chunks_are_retaken_by_the_next_index_after_a_reopen() {
        struct Refuse;
        impl crate::embedder::Embedder for Refuse {
            fn name(&self) -> &str {
                "granite-278m"
            }
            fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> {
                Err(crate::embedder::EmbedError::ProviderError("le service du témoin refuse tout".into()))
            }
            fn dim(&self) -> usize {
                768
            }
        }
        let d = tempfile::tempdir().unwrap();
        let (manifeste, _) = crate::daemon_host::symboles_cites::tests::petit_depot(d.path());
        let manifeste = PathBuf::from(manifeste);
        let ecrire = |f: &dyn Fn(&mut Value)| {
            let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
            f(&mut m);
            std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        };
        let refuse = servir(Arc::new(Refuse));
        ecrire(&|m| {
            m["embeddings"]["catch_up_on_open"] = json!(false);
            m["embeddings"]["address"] = json!(refuse);
        });
        let host = Arc::new(crate::daemon_host::tests::hote_isole());
        let h = host.hosted(&manifeste).unwrap();
        let indexer = || {
            let recu = host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
            let journal = recu["result"].as_str().unwrap_or_default().lines().find_map(|l| l.strip_prefix("journal : ")).unwrap().trim().to_string();
            let debut = std::time::Instant::now();
            while host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
                assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index n'a pas fini en 120 s : {journal}");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            journal
        };
        // La sonde d'`index` refuserait de partir : un débit noté d'avance.
        host.with_backend(&h, |b| b.catalog.lock().unwrap().note_measured_embedding_rate(crate::estimate::Rate::from_sample(100_000, std::time::Duration::from_secs(1)).unwrap()).unwrap()).unwrap();
        indexer();
        let dus = || host.with_backend(&h, |b| b.catalog.lock().unwrap().index_progress().unwrap().dense_missing()).unwrap();
        assert!(dus() > 0, "le service a tout refusé : des vecteurs restent dus");
        let s = host.open_session("le témoin", None, Vec::new(), None);
        assert_eq!(host.close(Some(&s), &manifeste)["ok"], true);
        // Un service sain, la base rouverte (un autre écrivain), dans la minute.
        let sain = service_granite();
        ecrire(&|m| m["embeddings"]["address"] = json!(sain));
        let journal = indexer();
        assert_eq!(dus(), 0, "les morceaux refusés sont repris par l'index suivant : {}", std::fs::read_to_string(&journal).unwrap_or_default());
    }

}
