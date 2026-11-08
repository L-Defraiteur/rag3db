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

/// Les fichiers et les symboles de l'index d'un backend (`fts_counts` de
/// `Backend::index_states`), relus une fois si une écriture tient le catalogue ;
/// `None` plutôt que des zéros.
fn comptes_de_l_index(hote: &DaemonHost, hosted: &Arc<super::Hosted>) -> (Option<u64>, Option<u64>) {
    for essai in 0..2 {
        let etat = hote.with_backend(hosted, |b| b.index_states()).ok().and_then(|r| r.ok()).unwrap_or(Value::Null);
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
            Value::Object(entites) if !entites.is_empty() => {
                // Par entité, l'état du plein texte et des vecteurs (`ready`, `never`, …).
                let n = entites.len();
                let prets = |cle: &str| entites.values().filter(|e| e[cle] == "ready").count();
                let en_cours = entites.values().any(|e| e["text_percent"].is_number() || e["vectors_percent"].as_f64().is_some_and(|p| p > 0.0 && p < 100.0));
                format!(
                    "plein texte {}/{n} entités, vecteurs {}/{n}{}",
                    prets("text"),
                    prets("vectors"),
                    if en_cours { ", en cours" } else { "" }
                )
            }
            Value::Null | Value::Object(_) => "ouvert, pas encore indexé".into(),
            Value::String(s) => s.clone(),
            autre => autre.to_string(),
        },
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
        hote.with_backend(&h, |_| ())?;
        let depot = super::repo_of(&worktree);
        let branche = super::branch_of(&worktree);
        // Le premier index part en fond ; un worktree déjà attaché garde le sien.
        let index = if deja {
            None
        } else {
            Some(match hote.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))) {
                Ok(Ok(recu)) => {
                    // La fin se dira sur le flux (`RepoIndexed`) : la page la montre.
                    if let Some(journal) = journal_du_recu(&recu) {
                        let quoi = json!({ "worktree": worktree, "depot": depot, "branche": branche });
                        suivre_le_premier_index(Arc::downgrade(&hote), h.clone(), journal, quoi, BORNES_DU_SUIVI);
                    }
                    "son index est lancé en fond ; sa fin se dira sur le flux de l'IDE".to_string()
                }
                Ok(Err(e)) | Err(e) => format!("son index n'a pas pu partir : {e}"),
            })
        };
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
        let etat = hote.state();
        let mut lignes = vec![format!("{} worktree(s) attaché(s) :", manifestes.len())];
        for m in manifestes {
            let m = m.canonicalize().unwrap_or(m);
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
    use std::path::Path;
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
}
