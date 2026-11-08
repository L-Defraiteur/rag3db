//! **L'atelier : l'hôte attache les worktrees** (page
//! 07-novembre-2026-everything-declarative/03, lot 0 ; Lucie, 8 novembre 2026 :
//! « l'atelier oui absolument »). `attach_worktree` inscrit un worktree déjà sur le
//! disque depuis le gabarit de code livré (comme `code-tree add`), l'ouvre, lance son
//! premier index en fond et le publie sur le flux (`repos`, `RepoAttached`) ;
//! `list_worktrees` dit les worktrees inscrits. Aucune commande git n'est lancée, rien
//! n'est cloné. Les phrases parlent de dépôt, de worktree, de branche et d'index.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use serde_json::{json, Value};

use super::DaemonHost;
use crate::dataflow::worktree_nodes::WorktreeHost;

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
        let en_cours = vus
            .iter()
            .filter(|e| e[cle] != "ready")
            .filter_map(|e| e[pourcent].as_u64())
            .filter(|p| *p < 100)
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
fn marque_du_premier_index(manifeste: &Path) -> PathBuf {
    let mut p = manifeste.as_os_str().to_owned();
    p.push(".premier-index");
    PathBuf::from(p)
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
fn lancer_sur(b: &crate::backend::Backend, manifeste: &Path) -> Result<Option<PathBuf>, String> {
    if crate::dataflow::index_nodes::indexings_running(&b.catalog) > 0 {
        return Ok(None);
    }
    let v = b.call("index", json!({ "confirm": true }))?;
    let journal = v["result"].as_str().unwrap_or_default().lines().find_map(|l| l.strip_prefix("journal : ")).map(|j| PathBuf::from(j.trim()));
    if let Some(j) = &journal {
        let _ = std::fs::write(marque_du_premier_index(manifeste), j.display().to_string());
    }
    Ok(journal)
}

/// **À l'ouverture d'un backend par l'hôte** : son index repart en fond quand son
/// premier index a été coupé (le témoin `.premier-index`), ou quand le manifeste déclare
/// `embeddings.catch_up_on_open` et que des morceaux sont encore sans vecteur — la
/// réouverture reconstruit l'index HNSW mais ne solde aucune dette (c7, 8 novembre 2026 :
/// wt-react5 resté à 15 % de vecteurs 18 minutes après sa réouverture). Une base qui ne
/// déclare ni l'un ni l'autre n'est jamais indexée par là.
pub(super) fn a_l_ouverture(hosted: &super::Hosted, b: &crate::backend::Backend) {
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
    match lancer_sur(b, &hosted.manifest) {
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

/// Le backend de `h` est-il ouvert dans l'hôte (ou en train de l'être) ?
fn ouvert(h: &super::Hosted) -> bool {
    h.backend.try_read().map(|g| g.is_some()).unwrap_or(true)
}

/// **Lance l'index d'un worktree** et pose le témoin de son journal ; dit en mots ce
/// qui est parti.
fn lancer_l_index(hote: &super::DaemonHost, h: &super::Hosted, manifeste: &Path) -> String {
    // Un index déjà reparti à l'ouverture (`a_l_ouverture`) se dit de même.
    match hote.with_backend(h, |b| lancer_sur(b, manifeste)) {
        Ok(Ok(lance)) => {
            // Lancé par un outil : il tient la porte.
            if lance.is_some() {
                h.rattrapage.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            "son index est lancé en fond".to_string()
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
    fn attach(&self, path: &Path) -> Result<String, String> {
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
        let ou = match (&depot, &branche) {
            (Some(d), Some(b)) => format!("dépôt {d}, branche {b}"),
            (Some(d), None) => format!("dépôt {d}, tête détachée"),
            (None, _) => "hors d'un dépôt git".to_string(),
        };
        Ok(match index {
            Some(i) if deja => format!("Le worktree {} est déjà attaché ({ou}) ; il était fermé, il est rouvert, et {i}. list_worktrees dit où il en est.", worktree.display()),
            Some(i) => format!("Worktree attaché : {} ({ou}) ; {i}. list_worktrees dit où il en est.", worktree.display()),
            None => format!(
                "Le worktree {} est déjà attaché ({ou}) : il est ouvert ; son index : {}.",
                worktree.display(),
                etat_de_l_index(&hote.state(), &manifeste)
            ),
        })
    }

    fn open_branch(&self, path: &Path, branch: &str, from: Option<&str>) -> Result<String, String> {
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
        let attache = self.attach(&worktree)?;
        Ok(match commande {
            Some(c) => format!("Branche {branch} ouverte dans le worktree {} (commande : `{c}`). {attache}", worktree.display()),
            None => format!("La branche {branch} est déjà ouverte dans le worktree {} : aucune commande git. {attache}", worktree.display()),
        })
    }

    fn list(&self) -> Result<String, String> {
        let hote = self.hote()?;
        let manifestes = hote.racines_de_code()?.registered_manifests();
        if manifestes.is_empty() {
            return Ok("Aucun worktree attaché : attach_worktree en attache un.".into());
        }
        // Un worktree fermé dont le premier index n'a pas fini se rouvre et le relance :
        // « pas ouvert » ne se dit plus d'un index qui devrait tourner.
        let manifestes: Vec<PathBuf> = manifestes.into_iter().map(|m| m.canonicalize().unwrap_or(m)).collect();
        for m in &manifestes {
            let _ = reprendre_si_fini_pas(&hote, m);
        }
        let etat = hote.state();
        let mut lignes = vec![format!("{} worktree(s) attaché(s) :", manifestes.len())];
        for m in manifestes {
            let racine = super::espace_du_manifeste(&m);
            let (depot, branche) = racine.as_deref().map(|r| (super::repo_of(r), super::branch_of(r))).unwrap_or((None, None));
            lignes.push(format!(
                "- {} — dépôt {}, branche {} ; index : {}",
                racine.as_deref().map(|r| r.display().to_string()).unwrap_or_else(|| "?".into()),
                depot.as_deref().unwrap_or("hors git"),
                branche.as_deref().unwrap_or("détachée"),
                etat_de_l_index(&etat, &m)
            ));
        }
        Ok(lignes.join("\n"))
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
            let agent = crate::daemon::agent();
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
        let livre = atelier.with_extension("json.livre");
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
        let marque = PathBuf::from(format!("{}.premier-index", manifeste.display()));
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
        assert!(!liste.contains("pas ouvert"), "{liste}");
        let neuf = journal_de();
        assert!(!neuf.is_empty() && neuf != coupe.display().to_string(), "un journal neuf : {neuf}");
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
            std::thread::sleep(std::time::Duration::from_millis(5 * texts.len() as u64));
            Ok(texts.iter().map(|_| { let mut v = vec![0.0_f32; 768]; v[0] = 1.0; v }).collect())
        }
        fn dim(&self) -> usize {
            768
        }
    }

    fn service_granite() -> String {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let _ = crate::daemon::EmbedDaemon::new(Arc::new(Granite)).servir_lie(ecoute);
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
        // Assez de morceaux pour plusieurs passes de 512.
        for i in 0..700 {
            std::fs::write(arbre.join(format!("src/f{i}.rs")), format!("pub fn fonction_{i}() -> u32 {{\n    {i}\n}}\n")).unwrap();
        }
        let ecrire = |f: &dyn Fn(&mut Value)| {
            let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
            f(&mut m);
            std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        };
        // 1. Un index sans service : le plein texte, et tous les vecteurs dus.
        ecrire(&|m| {
            m["embeddings"]["catch_up_on_open"] = json!(false);
            m["embeddings"].as_object_mut().unwrap().remove("address");
        });
        let host = Arc::new(crate::daemon_host::tests::hote_isole());
        let h = host.hosted(&manifeste).unwrap();
        let fini = |j: &str| std::fs::read_to_string(j).ok().and_then(|t| t.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.starts_with(crate::dataflow::index_nodes::DONE_LINE) || l.starts_with(crate::dataflow::index_nodes::FAILED_PREFIX))).unwrap_or(false);
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
        while !fini(&journal) {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index n'a pas fini en 120 s");
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        attendre_l_index();
        let dus = || host.with_backend(&h, |b| b.catalog.lock().unwrap().index_progress().unwrap().dense_missing()).unwrap();
        assert!(dus() > 0, "sans service, des vecteurs restent dus");
        let fermer = || {
            let s = host.open_session("le témoin", None, Vec::new(), None);
            assert_eq!(host.close(Some(&s), &manifeste)["ok"], true);
        };
        fermer();

        // 2. Rouvert avec le service, sans le réglage : rien ne part.
        let adresse = service_granite();
        ecrire(&|m| m["embeddings"]["address"] = json!(adresse));
        assert_eq!(host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap(), 0, "sans le réglage, aucun index ne part à l'ouverture");
        let avant = dus();
        assert!(avant > 0);
        fermer();

        // 3. Rouvert avec le réglage : l'index repart de lui-même. Il ne tient pas la
        // porte : une fermeture l'arrête au paquet suivant, et la dette reste.
        ecrire(&|m| m["embeddings"]["catch_up_on_open"] = json!(true));
        host.with_backend(&h, |_| ()).unwrap();
        assert!(host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0, "avec le réglage, l'index repart à l'ouverture");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let debut = std::time::Instant::now();
        fermer();
        assert!(debut.elapsed() < std::time::Duration::from_secs(30), "la fermeture a attendu {:?}", debut.elapsed());
        ecrire(&|m| m["embeddings"]["catch_up_on_open"] = json!(false));
        let reste = dus();
        assert!(reste > 0, "la fermeture a coupé le rattrapage : la dette reste");
        fermer();

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
        assert!(dit.starts_with("plein texte prêt"), "{dit}");
        assert!(!dit.contains("en cours"), "{dit}");
    }

}
