//! **L'atelier lit et écrit** (Lucie, 8 novembre 2026 au soir ; page
//! `09-novembre-2026-atelier-lire-ecrire/01-conception.md`). Les outils de fichier de
//! l'atelier passent par l'hôte : un chemin dans un worktree attaché va au backend de code
//! de ce worktree (sa source, son index, sa réindexation), un autre à la racine de
//! l'atelier, sans index ; hors de la racine, le modèle déclare un espace ou demande
//! l'accord.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use super::{DaemonHost, Hosted};
use crate::dataflow::fichier_nodes::Edition;

/// **Une demande d'accord** (ask_permission) : le chemin, le mode, la raison du modèle, et
/// la décision quand elle est prise. Une **commande en attente** du chat de l'atelier (lot 7)
/// en est une aussi : sa ligne et le backend dont la porte l'accordera ; son chemin est le
/// domaine de travail.
#[derive(Debug, Clone)]
pub(super) struct Demande {
    pub chemin: PathBuf,
    pub mode: String,
    pub raison: String,
    pub decision: Option<bool>,
    /// La publication d'une même carte se sérialise sans retenir le registre
    /// pendant l'accès au backend (qui peut lui-même consulter les accords).
    publication: Arc<std::sync::Mutex<()>>,
    /// La ligne de commande, pour une commande en attente.
    pub commande: Option<String>,
    /// Le backend de la conversation qui l'a posée : sa porte accorde une commande, et la
    /// décision y relance le tour. Une demande posée hors d'un chat n'en a pas.
    pub backend: Option<String>,
}

/// Les demandes et les chemins ouverts de l'atelier.
#[derive(Debug, Default)]
pub(super) struct Accords {
    demandes: std::collections::HashMap<String, Demande>,
    /// Les chemins accordés et leur mode (`read` ou `write` ; `write` vaut aussi lecture).
    ouverts: Vec<(PathBuf, String)>,
    suivant: u64,
}

impl Accords {
    fn demande_a_decider(&self, id: &str) -> Result<Demande, String> {
        let d = self.demandes.get(id).ok_or_else(|| format!("aucune demande {id}"))?;
        if let Some(deja) = d.decision {
            return Err(format!("la demande {id} est déjà {}", if deja { "accordée" } else { "refusée" }));
        }
        Ok(d.clone())
    }
}

/// La publication vers la porte et l'enregistrement de la décision : la même
/// frontière pour l'hôte et les témoins à registre/Garde synthétiques.
fn publier_decision(
    accords: &std::sync::Mutex<Accords>,
    id: &str,
    d: Demande,
    accorder: bool,
    publier: impl FnOnce(&Demande) -> Result<(), String>,
) -> Result<Demande, String> {
    let publication = d.publication.clone();
    let _publication = publication.lock().unwrap_or_else(|p| p.into_inner());
    // Le snapshot a pu attendre un autre répondant : il n'autorise jamais
    // une publication avant cette relecture sous le verrou de la demande.
    let d = accords.lock().unwrap_or_else(|p| p.into_inner()).demande_a_decider(id)?;
    if accorder && d.commande.is_some() && d.backend.is_some() {
        publier(&d)?;
    }
    {
        let mut a = accords.lock().unwrap_or_else(|p| p.into_inner());
        let Some(e) = a.demandes.get_mut(id) else { return Err(format!("aucune demande {id}")) };
        if e.decision.is_some() {
            return Err(format!("la demande {id} a été décidée entre-temps"));
        }
        e.decision = Some(accorder);
        if accorder && d.commande.is_none() {
            a.ouverts.push((d.chemin.clone(), d.mode.clone()));
        }
    }
    Ok(d)
}

/// **Ce que le modèle lit après une décision sur la carte** : la commande ou le chemin entre
/// accents graves, et quoi faire.
fn message_de_la_carte(d: &Demande, accorde: bool) -> String {
    match (&d.commande, accorde) {
        (Some(c), true) => format!("(carte d'accord) La commande `{c}` est accordée : relance-la telle quelle."),
        (Some(c), false) => format!("(carte d'accord) La commande `{c}` est refusée : ne la relance pas ; fais autrement, ou demande-moi."),
        (None, true) => {
            let quoi = if d.mode == "write" { "en écriture (et en lecture)" } else { "en lecture" };
            format!("(carte d'accord) L'accès {quoi} à `{}` est accordé : réessaie.", d.chemin.display())
        }
        (None, false) => format!("(carte d'accord) L'accès à `{}` est refusé : ne réessaie pas ; fais autrement, ou demande-moi.", d.chemin.display()),
    }
}

/// Le worktree attaché qui contient un chemin : son backend, sa racine, son dépôt et sa
/// branche.
pub(super) struct Worktree {
    pub hosted: Arc<Hosted>,
    pub racine: PathBuf,
    pub depot: Option<String>,
    pub branche: Option<String>,
}

impl Worktree {
    fn nom(&self) -> String {
        match (&self.depot, &self.branche) {
            (Some(d), Some(b)) => format!("{d} · {b}"),
            (Some(d), None) => d.clone(),
            _ => self.racine.display().to_string(),
        }
    }
}

/// Un chemin résolu : absolu, dans la racine de l'atelier, et le worktree attaché qui
/// le contient s'il y en a un.
pub(super) struct Resolu {
    pub chemin: PathBuf,
    pub worktree: Option<Worktree>,
}

/// **La forme canonique d'un chemin qui peut ne pas exister encore** (un fichier à créer),
/// résolue composant par composant : un composant qui existe se canonise (liens symboliques
/// compris), un composant absent reste un nom, et `..` remonte depuis la forme déjà résolue.
/// Avant le 13 novembre 2026, un `..` qui suivait un dossier absent restait dans le chemin
/// (`racine/nouveau/../../x`), et le test de racine, lexical, le croyait dedans : l'atelier
/// écrivait hors de sa racine sans accord (relecture d'everything declarative).
fn canonique(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut forme = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => forme.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                forme.pop();
            }
            Component::Normal(n) => {
                let suivant = forme.join(n);
                forme = if std::fs::symlink_metadata(&suivant).is_ok() { suivant.canonicalize().unwrap_or(suivant) } else { suivant };
            }
        }
    }
    forme
}

impl DaemonHost {
    /// **Résoudre un chemin de l'atelier** : relatif à sa racine, ou absolu (`~/`
    /// s'étend) ; refusé hors de la racine, avec les deux suites que le modèle a.
    pub(super) fn atelier_resoudre(&self, racine: Option<&Path>, path: &str, mode: &str) -> Result<Resolu, String> {
        let racine = racine.ok_or("l'atelier n'a pas de racine de travail (`workspace.root`)")?;
        let racine = canonique(racine);
        let p = crate::backend_code::etendre_tilde(path.trim());
        let chemin = canonique(&if p.is_absolute() { p } else { racine.join(p) });
        if !chemin.starts_with(&racine) && !self.accorde(&chemin, mode) {
            return Err(format!(
                "{} est hors de la racine de l'atelier ({}). Deux suites : déclarer l'espace (attach_worktree ou open_branch sur le worktree qui le contient), ou demander l'accord de l'utilisateur (ask_permission, avec la raison).",
                chemin.display(),
                racine.display()
            ));
        }
        Ok(Resolu { worktree: self.worktree_qui_contient(&chemin)?, chemin })
    }

    /// Le worktree attaché (un arbre inscrit) dont la racine contient `chemin` : le plus
    /// profond s'il y en a plusieurs.
    fn worktree_qui_contient(&self, chemin: &Path) -> Result<Option<Worktree>, String> {
        let Ok(racines) = self.racines_de_code() else { return Ok(None) };
        let mut meilleur: Option<(PathBuf, PathBuf)> = None;
        for m in racines.registered_manifests() {
            let Some(v) = std::fs::read_to_string(&m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { continue };
            let Some(root) = v["workspace"]["root"].as_str().map(PathBuf::from) else { continue };
            let root = canonique(&if root.is_absolute() { root } else { m.parent().unwrap_or(Path::new("/")).join(root) });
            if chemin.starts_with(&root) && meilleur.as_ref().is_none_or(|(r, _)| root.components().count() > r.components().count()) {
                meilleur = Some((root, m));
            }
        }
        let Some((racine, m)) = meilleur else { return Ok(None) };
        let m = m.canonicalize().unwrap_or(m);
        let hosted = self.hosted(&m)?;
        Ok(Some(Worktree { hosted, depot: super::repo_of(&racine), branche: super::branch_of(&racine), racine }))
    }

    /// **L'atelier peut-il écrire ce worktree ?** Oui s'il est né de l'atelier (ouvert par
    /// `open_branch`, marqué à côté de son manifeste), ou si l'utilisatrice l'a accordé
    /// (ask_permission) ; sinon non (décision de c7, 9 novembre 2026 : `task_writes` garde
    /// son sens pour l'arbre qu'elle tient).
    fn atelier_peut_ecrire(&self, w: &Worktree, chemin: &Path) -> bool {
        if self.accorde(chemin, "write") {
            return true;
        }
        let Ok(racines) = self.racines_de_code() else { return false };
        crate::code_tree::name_of_tree(&w.racine).is_ok_and(|nom| racines.marque_de_l_atelier(&nom).is_file())
    }

    /// Un accord couvre-t-il ce chemin pour ce mode ? (`write` vaut aussi lecture.)
    fn accorde(&self, chemin: &Path, mode: &str) -> bool {
        let a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
        a.ouverts.iter().any(|(p, m)| chemin.starts_with(p) && (m == "write" || m == mode))
    }

    /// **Demander l'accord** (`ask_permission`) : la demande gardée, sa carte rendue.
    pub(super) fn atelier_demander(&self, racine: Option<&Path>, path: &str, mode: &str, raison: &str) -> Result<crate::dataflow::worktree_nodes::WorktreeAnswer, String> {
        if !matches!(mode, "read" | "write") {
            return Err(format!("ask_permission : le mode est « read » ou « write » (reçu « {mode} »)"));
        }
        if raison.trim().is_empty() {
            return Err("ask_permission : donnez la raison (`reason`) : l'utilisatrice décide sur elle".into());
        }
        let p = crate::backend_code::etendre_tilde(path.trim());
        let chemin = canonique(&match (p.is_absolute(), racine) {
            (false, Some(r)) => r.join(p),
            _ => p,
        });
        let id = {
            let mut a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
            a.suivant += 1;
            let id = format!("accord-{}-{}", crate::events::maintenant_ms(), a.suivant);
            a.demandes.insert(id.clone(), Demande { chemin: chemin.clone(), mode: mode.to_string(), raison: raison.trim().to_string(), publication: Arc::default(), decision: None, commande: None, backend: None });
            id
        };
        let mot = if mode == "write" { "écrire" } else { "lire" };
        Ok(crate::dataflow::worktree_nodes::WorktreeAnswer {
            text: format!(
                "Demande {id} posée : {mot} {} ({}). L'utilisatrice accorde ou refuse sur la carte sous ce message ; attendez sa réponse, puis réessayez.",
                chemin.display(),
                raison.trim()
            ),
            ui: vec![json!({ "kind": "accord", "sujets": ["accords"], "id": id, "chemin": chemin, "mode": mode, "raison": raison.trim() })],
        })
    }

    /// **Une commande en attente devient une demande d'accord** (lot 7 ; c7, 9 novembre
    /// 2026) : le chat de l'atelier, hors d'une tâche, a vu la porte mettre `ligne` en
    /// attente ; la demande est gardée avec le backend dont la porte l'accordera, et la
    /// carte rendue (genre `commande`). Rend le texte à ajouter au résultat de l'outil, et
    /// la carte.
    pub(super) fn atelier_demander_commande(&self, backend: &str, ligne: &str, domaine: Option<&Path>) -> (String, Value) {
        let chemin = domaine.map(Path::to_path_buf).unwrap_or_default();
        let id = {
            let mut a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
            a.suivant += 1;
            let id = format!("accord-{}-{}", crate::events::maintenant_ms(), a.suivant);
            a.demandes.insert(
                id.clone(),
                Demande { chemin: chemin.clone(), mode: "command".into(), raison: String::new(), publication: Arc::default(), decision: None, commande: Some(ligne.to_string()), backend: Some(backend.to_string()) },
            );
            id
        };
        (
            format!(
                "\n\n_La demande {id} est sur la carte d'accord, sous cet appel : l'utilisatrice accorde ou refuse. Accordée, cette même commande passe telle quelle ; attendez sa réponse, puis relancez-la._"
            ),
            json!({ "kind": "accord", "sujets": ["accords"], "id": id, "genre": "commande", "commande": ligne, "chemin": chemin }),
        )
    }

    /// **Décider une demande** : un oui ouvre le chemin pour ce mode, ou, pour une commande,
    /// l'accorde à la porte de son backend (comme le oui du superviseur d'une tâche) ; la
    /// décision va au journal des décisions (`<racine des runs>/decisions.jsonl`, une ligne)
    /// et au flux (`accords`).
    pub fn decider_accord(&self, id: &str, accorder: bool, qui: &str) -> Result<Value, String> {
        let d = self.accords.lock().unwrap_or_else(|p| p.into_inner()).demande_a_decider(id)?;
        let d = publier_decision(&self.accords, id, d, accorder, |d| {
            let (Some(commande), Some(backend)) = (&d.commande, &d.backend) else { return Ok(()) };
            let domaine = Some(d.chemin.clone()).filter(|c| !c.as_os_str().is_empty());
            self.with_backend_id(backend, |b| match b.garde() {
                Some(g) => g.accorder_ligne(commande, &crate::commande::Contexte { accorde_par_l_utilisateur: true, domaine }),
                None => Err("ce backend n'a pas de porte des commandes (workspace.commands)".into()),
            })
            .and_then(|r| r)
            .map(|_| ())
            .map_err(|e| format!("l'accord de « {commande} » n'a pas pu se poser : {e}"))
        })?;
        let mut ligne = json!({
            "id": id,
            "genre": if d.commande.is_some() { "commande" } else { "chemin" },
            "chemin": d.chemin,
            "mode": d.mode,
            "raison": d.raison,
            "decision": if accorder { "accordé" } else { "refusé" },
            "qui": qui,
            "quand_ms": crate::events::maintenant_ms(),
        });
        if let Some(commande) = &d.commande {
            ligne["commande"] = json!(commande);
        }
        let journal = self.journal.racine().join("decisions.jsonl");
        if let Some(dir) = journal.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        }
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&journal).map_err(|e| format!("{} : {e}", journal.display()))?;
        writeln!(f, "{ligne}").map_err(|e| format!("{} : {e}", journal.display()))?;
        self.ide_flux().publier("accords", "*", json!({ "kind": "AccordDecide", "id": id, "decision": ligne["decision"], "chemin": d.chemin, "mode": d.mode, "commande": ligne["commande"] }));
        let mut rendu = ligne;
        rendu["relance"] = json!(d.backend.as_deref().is_some_and(|b| self.relancer_apres_la_carte(b, &message_de_la_carte(&d, accorder))));
        Ok(rendu)
    }

    /// **Rattacher une demande à la conversation qui l'a posée** : le nœud d'`ask_permission`
    /// ne connaît pas son backend ; le chat, qui voit la carte passer sous l'appel, l'y lie,
    /// pour que la décision relance ce tour-là. Une demande déjà rattachée le reste.
    pub(super) fn rattacher_accord(&self, id: &str, backend: &str) {
        let mut a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(d) = a.demandes.get_mut(id) {
            d.backend.get_or_insert_with(|| backend.to_string());
        }
    }

    /// **La décision relance le tour d'elle-même** (c7, 9 novembre 2026 : que Lucie n'ait
    /// pas à retaper) : elle entre dans la conversation comme un message que le modèle lit,
    /// et un tour repart, dans un fil, sans que l'utilisatrice écrive. Rend vrai si le tour
    /// est parti ; un tour déjà en cours, ou un hôte qui ne se connaît pas, le laisse à
    /// l'utilisatrice, et le journal de l'hôte le dit.
    fn relancer_apres_la_carte(&self, backend: &str, message: &str) -> bool {
        let message = message.to_string();
        let Some(hote) = self.soi.get().and_then(std::sync::Weak::upgrade) else { return false };
        let backend = backend.to_string();
        std::thread::spawn(move || {
            if let Err(e) = hote.chat_turn(&backend, &message) {
                eprintln!("[démon] la relance après la carte d'accord n'est pas partie ({backend}) : {e}");
            }
        });
        true
    }

    /// L'état d'une demande (la carte relue après un rechargement).
    pub fn etat_accord(&self, id: &str) -> Option<Value> {
        let a = self.accords.lock().unwrap_or_else(|p| p.into_inner());
        a.demandes.get(id).map(|d| {
            json!({
                "id": id,
                "genre": if d.commande.is_some() { "commande" } else { "chemin" },
                "commande": d.commande,
                "chemin": d.chemin,
                "mode": d.mode,
                "raison": d.raison,
                "decision": d.decision.map(|x| if x { "accordé" } else { "refusé" }),
            })
        })
    }

    /// **Lire un fichier de l'atelier** (`read_file`) : 200 lignes au plus, la suite dite.
    pub(super) fn atelier_lire(&self, racine: Option<&Path>, path: &str, offset: usize, length: usize) -> Result<String, String> {
        let r = self.atelier_resoudre(racine, path, "read")?;
        if let Some(w) = &r.worktree {
            let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
            let v = self.with_backend(&w.hosted, |b| b.call("read_file", json!({ "path": rel, "offset": offset, "limit": length }))).and_then(|x| x)?;
            let texte = v["presentation"].as_str().or_else(|| v["result"].as_str()).map(str::to_string).unwrap_or_else(|| v["result"].to_string());
            return Ok(format!("> lu dans le dépôt {} (son index le connaît)\n{texte}", w.nom()));
        }
        let contenu = std::fs::read_to_string(&r.chemin).map_err(|e| format!("{} : {e}", r.chemin.display()))?;
        let lignes: Vec<&str> = contenu.lines().collect();
        let total = lignes.len();
        let debut = offset.max(1);
        if total > 0 && debut > total {
            return Err(format!("{} a {total} lignes : offset {debut} est au-delà", r.chemin.display()));
        }
        let fin = (debut + length).saturating_sub(1).min(total);
        let mut out = format!("{} — lignes {debut}-{fin} sur {total}\n", r.chemin.display());
        for (i, l) in lignes.iter().enumerate().take(fin).skip(debut - 1) {
            out.push_str(&format!("{:05}| {l}\n", i + 1));
        }
        if fin < total {
            out.push_str(&format!("… {} ligne(s) de plus : read_file(path, offset={})\n", total - fin, fin + 1));
        }
        Ok(out)
    }

    /// Tous les worktrees attachés (les arbres inscrits).
    fn worktrees_attaches(&self) -> Vec<Worktree> {
        let Ok(racines) = self.racines_de_code() else { return Vec::new() };
        racines
            .registered_manifests()
            .into_iter()
            .filter_map(|m| {
                let v = std::fs::read_to_string(&m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())?;
                let root = PathBuf::from(v["workspace"]["root"].as_str()?);
                let racine = canonique(&if root.is_absolute() { root } else { m.parent()?.join(root) });
                let hosted = self.hosted(&m.canonicalize().unwrap_or(m)).ok()?;
                Some(Worktree { hosted, depot: super::repo_of(&racine), branche: super::branch_of(&racine), racine })
            })
            .collect()
    }

    /// **Lire un scope** (`read_scope`) : par `usages` (où il est défini) et `outline` (ses
    /// bornes) dans le worktree attaché ; son texte lu entre ses bornes (200 lignes au
    /// plus, la suite dite).
    pub(super) fn atelier_lire_scope(&self, racine: Option<&Path>, name: &str, path: &str) -> Result<String, String> {
        let (worktrees, rel_voulu): (Vec<Worktree>, Option<String>) = if path.trim().is_empty() {
            (self.worktrees_attaches(), None)
        } else {
            let r = self.atelier_resoudre(racine, path, "read")?;
            let Some(w) = r.worktree else {
                return Err(format!(
                    "{} est sous la racine de l'atelier, sans index : read_scope ne connaît que les worktrees attachés ; lisez-le par read_file",
                    r.chemin.display()
                ));
            };
            let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
            (vec![w], Some(rel))
        };
        if worktrees.is_empty() {
            return Err("aucun worktree attaché : read_scope lit l'index d'un worktree attaché (attach_worktree, open_branch)".into());
        }
        let json_de = |v: &Value| -> Value {
            let r = &v["result"];
            match r.as_str() {
                Some(s) => serde_json::from_str(s).unwrap_or(Value::Null),
                None => r.get("result").cloned().unwrap_or_else(|| r.clone()),
            }
        };
        // Les définitions du nom, par worktree : (worktree, fichier relatif, ligne de début).
        let mut trouves: Vec<(&Worktree, String, u64)> = Vec::new();
        for w in &worktrees {
            let args = json!({ "name": name, "path": rel_voulu.clone().unwrap_or_default(), "limit": 1, "format": "json" });
            // Une panne ne prouve ni l'absence ni l'unicité parmi les autres arbres.
            // Ticket : docs/everything-declarative/tickets/2026-11-13-read-scope-masque-les-erreurs-de-lecture-des-worktrees.md.
            let contexte = format!(
                "read_scope « {name} » : recherche incomplète ; usages du worktree {} (backend {}, manifeste {})",
                w.racine.display(), w.hosted.id, w.hosted.manifest.display()
            );
            let v = self.with_backend(&w.hosted, |b| b.call("usages", args))
                .map_err(|e| format!("{contexte} : accès au backend en échec : {e}"))?
                .map_err(|e| format!("{contexte} : appel usages en échec : {e}"))?;
            for d in json_de(&v)["definitions"].as_array().into_iter().flatten() {
                if let (Some(p), Some(l)) = (d["path"].as_str(), d["line"].as_u64()) {
                    trouves.push((w, p.to_string(), l));
                }
            }
        }
        let (w, rel, ligne) = match trouves.len() {
            0 => return Err(format!("« {name} » : aucun scope de ce nom dans l'index des worktrees attachés")),
            1 => trouves.remove(0),
            n => {
                let liste: Vec<String> = trouves.iter().map(|(w, p, l)| format!("{}:{l} ({})", w.racine.join(p).display(), w.nom())).collect();
                return Err(format!("« {name} » est défini {n} fois : donnez `path` pour choisir — {}", liste.join(" ; ")));
            }
        };
        // Les bornes, par le plan du fichier.
        let v = self.with_backend(&w.hosted, |b| b.call("outline", json!({ "path": rel, "format": "json" }))).and_then(|x| x)?;
        let plan = json_de(&v);
        let scope = plan["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|s| s["name"].as_str() == Some(name) && s["start_line"].as_u64() == Some(ligne))
            .cloned()
            .ok_or_else(|| format!("« {name} » : défini à {rel}:{ligne}, mais le plan du fichier ne le borne pas (l'index est-il à jour ?)"))?;
        let debut = scope["start_line"].as_u64().unwrap_or(ligne) as usize;
        let fin = scope["end_line"].as_u64().unwrap_or(ligne) as usize;
        let chemin = w.racine.join(&rel);
        let contenu = std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let lignes: Vec<&str> = contenu.lines().collect();
        let fin_lue = fin.min(debut + crate::dataflow::fichier_nodes::LIGNES_MAX - 1).min(lignes.len());
        let mut out = format!(
            "{} ({}) — {} lignes {debut}-{fin}, dépôt {}\n",
            name,
            scope["scope_type"].as_str().unwrap_or("scope"),
            chemin.display(),
            w.nom()
        );
        for (i, l) in lignes.iter().enumerate().take(fin_lue).skip(debut.saturating_sub(1)) {
            out.push_str(&format!("{:05}| {l}\n", i + 1));
        }
        if fin_lue < fin {
            out.push_str(&format!("… {} ligne(s) de plus : read_file(path, offset={})\n", fin - fin_lue, fin_lue + 1));
        }
        Ok(out)
    }

    /// **Écrire un fichier** (`replace_text`, `replace_lines`, `insert_lines`) : le texte
    /// neuf calculé, écrit (par le dépôt s'il est dans un worktree attaché, qui le
    /// réindexe), et le diff unifié rendu.
    pub(super) fn atelier_ecrire(&self, racine: Option<&Path>, path: &str, edition: Edition) -> Result<String, String> {
        let r = self.atelier_resoudre(racine, path, "write")?;
        // L'empreinte de ce qui a été lu : l'écriture refuse un fichier changé entre-temps
        // (ticket d'ED, l'écriture sans contrôle de concurrence). `None` : un fichier à créer.
        let (avant, lu) = match std::fs::read_to_string(&r.chemin) {
            Ok(t) => {
                let h = crate::hash::content_hash(&t);
                (t, Some(h))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && matches!(edition, Edition::Insertion { avant: 1, .. }) => (String::new(), None),
            Err(e) => return Err(format!("{} : {e}", r.chemin.display())),
        };
        #[cfg(test)]
        tests::apres_lecture(&r.chemin);
        let apres = texte_apres(&avant, &edition).map_err(|e| format!("{} : {e}", r.chemin.display()))?;
        let mut dit = Vec::new();
        if let Some(w) = &r.worktree {
            if !self.atelier_peut_ecrire(w, &r.chemin) {
                return Err(format!(
                    "{} est dans le dépôt {}, que l'atelier n'a pas ouvert lui-même (l'arbre que tient l'utilisatrice) : demandez son accord par ask_permission (path, mode « write », la raison), puis réessayez ; ou ouvrez une branche à vous (open_branch) et écrivez-y",
                    r.chemin.display(),
                    w.nom()
                ));
            }
        }
        match &r.worktree {
            Some(w) => {
                let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
                let mut args = json!({ "path": rel, "content": apres });
                if let Some(h) = &lu {
                    args["expected_hash"] = json!(h);
                }
                let v = self.with_backend(&w.hosted, |b| b.call_humain("edit_file", args)).and_then(|x| x)?;
                let rendu = v["presentation"].as_str().or_else(|| v["result"].as_str()).unwrap_or_default().to_string();
                let index = etat_de_la_reindexation(&rendu);
                dit.push(format!("écrit dans le dépôt {} ; {index}", w.nom()));
            }
            None => {
                ecrire_atomique(&r.chemin, &apres, lu.as_deref())?;
                dit.push("écrit sous la racine de l'atelier (sans index)".to_string());
            }
        }
        Ok(format!("{}\n{}", dit.join("\n"), diff_unifie(&r.chemin.display().to_string(), &avant, &apres)))
    }
}

/// **Ce que l'édition d'un dépôt dit de son index** (le rendu de `EditResult::to_markdown`).
fn etat_de_la_reindexation(rendu: &str) -> String {
    if let Some(reste) = rendu.split("Index updated: ").nth(1) {
        let n = |mot: &str| reste.split(mot).next().and_then(|x| x.rsplit(|c: char| !c.is_ascii_digit()).find(|m| !m.is_empty())).unwrap_or("0").to_string();
        format!("réindexé ({} scope(s) écrits, {} retirés)", n(" scopes upserted"), n(" removed"))
    } else if rendu.contains("index will pick it up") || rendu.contains("indexing in progress will read it") {
        "un index tourne : il prendra ce fichier".to_string()
    } else if let Some(e) = rendu.split("the index did not follow (").nth(1) {
        format!("sa réindexation n'a pas suivi ({})", e.split(") —").next().unwrap_or(e))
    } else {
        "sans index".to_string()
    }
}

/// Les lignes d'un texte, chacune avec sa fin de ligne.
fn lignes(t: &str) -> Vec<&str> {
    t.split_inclusive('\n').collect()
}

/// Un texte écrit en lignes : sa dernière ligne finie par un retour.
fn en_lignes(t: &str) -> String {
    if t.is_empty() || t.ends_with('\n') { t.to_string() } else { format!("{t}\n") }
}

/// **Le texte après une édition**, ou le refus qui dit pourquoi.
pub(super) fn texte_apres(avant: &str, edition: &Edition) -> Result<String, String> {
    match edition {
        Edition::Texte { ancien, nouveau } => match avant.matches(ancien.as_str()).count() {
            0 => Err("l'ancien texte n'y est pas : relisez le fichier (read_file) et recopiez-le exactement".into()),
            1 => Ok(avant.replacen(ancien.as_str(), nouveau, 1)),
            n => Err(format!("l'ancien texte y est {n} occurrences : donnez plus de contexte pour qu'il soit unique")),
        },
        Edition::Lignes { de, a, texte } => {
            let l = lignes(avant);
            if *de < 1 || *a < *de || *a > l.len() {
                return Err(format!("le fichier a {} lignes : les lignes {de}..{a} n'y sont pas", l.len()));
            }
            Ok(format!("{}{}{}", l[..de - 1].concat(), en_lignes(texte), l[*a..].concat()))
        }
        Edition::Insertion { avant: at, texte } => {
            let l = lignes(avant);
            if *at < 1 || *at > l.len() + 1 {
                return Err(format!("le fichier a {} lignes : on insère avant une ligne de 1 à {}", l.len(), l.len() + 1));
            }
            // Un fichier sans retour final : la ligne d'avant reçoit le sien avant l'ajout.
            let tete = l[..at - 1].concat();
            let tete = if *at - 1 == l.len() { en_lignes(&tete) } else { tete };
            Ok(format!("{tete}{}{}", en_lignes(texte), l[at - 1..].concat()))
        }
    }
}

/// **Écrire un fichier d'un coup** : un voisin au nom unique, écrit et rendu durable
/// (`fsync`), qui prend le mode de l'ancien fichier, puis renommé sur la cible. `lu` est
/// l'empreinte du contenu lu avant l'édition : si le fichier a changé depuis, rien n'est écrit
/// et le refus le dit. Sur une erreur, le voisin est effacé (tickets d'ED du 13 novembre 2026 :
/// le mode perdu, le temporaire au nom fixe, l'écriture sans contrôle de concurrence).
fn ecrire_atomique(chemin: &Path, texte: &str, lu: Option<&str>) -> Result<(), String> {
    use std::io::Write;
    static SUIVANT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dossier = chemin.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    let nom = chemin.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let n = SUIVANT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = dossier.join(format!(".{nom}.atelier-{}-{}-{n}.tmp", std::process::id(), crate::events::maintenant_ms()));
    let ecrire = || -> Result<(), String> {
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("{} : {e}", tmp.display()))?;
        f.write_all(texte.as_bytes()).map_err(|e| format!("{} : {e}", tmp.display()))?;
        if let Ok(ancien) = std::fs::metadata(chemin) {
            std::fs::set_permissions(&tmp, ancien.permissions()).map_err(|e| format!("{} : {e}", tmp.display()))?;
        }
        f.sync_all().map_err(|e| format!("{} : {e}", tmp.display()))?;
        // Le contenu de la cible est-il encore celui qui a été lu ?
        let maintenant = match std::fs::read_to_string(chemin) {
            Ok(t) => Some(crate::hash::content_hash(&t)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{} : {e}", chemin.display())),
        };
        if maintenant.as_deref() != lu {
            return Err(format!(
                "{} a changé depuis sa lecture : rien n'est écrit ; relisez-le (read_file), puis réessayez sur ce qu'il contient maintenant",
                chemin.display()
            ));
        }
        std::fs::rename(&tmp, chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        if let Ok(d) = std::fs::File::open(dossier) {
            let _ = d.sync_all();
        }
        Ok(())
    };
    let r = ecrire();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// **Le diff unifié d'une édition** : une région changée (les trois écritures n'en
/// changent qu'une), entre le préfixe et le suffixe communs, avec trois lignes de
/// contexte.
pub(super) fn diff_unifie(nom: &str, avant: &str, apres: &str) -> String {
    const CONTEXTE: usize = 3;
    let a: Vec<&str> = avant.lines().collect();
    let b: Vec<&str> = apres.lines().collect();
    let prefixe = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let max_suffixe = a.len().min(b.len()) - prefixe;
    let suffixe = a.iter().rev().zip(b.iter().rev()).take(max_suffixe).take_while(|(x, y)| x == y).count();
    if prefixe == a.len() && prefixe == b.len() {
        return format!("--- a/{}\n+++ b/{}\n(aucun changement)\n", nom.trim_start_matches('/'), nom.trim_start_matches('/'));
    }
    let debut = prefixe.saturating_sub(CONTEXTE);
    let fin_a = (a.len() - suffixe + CONTEXTE).min(a.len());
    let fin_b = (b.len() - suffixe + CONTEXTE).min(b.len());
    let nom = nom.trim_start_matches('/');
    let mut out = format!("--- a/{nom}\n+++ b/{nom}\n@@ -{},{} +{},{} @@\n", debut + 1, fin_a - debut, debut + 1, fin_b - debut);
    for l in &a[debut..prefixe] {
        out.push_str(&format!(" {l}\n"));
    }
    for l in &a[prefixe..a.len() - suffixe] {
        out.push_str(&format!("-{l}\n"));
    }
    for l in &b[prefixe..b.len() - suffixe] {
        out.push_str(&format!("+{l}\n"));
    }
    for l in &a[a.len() - suffixe..fin_a] {
        out.push_str(&format!(" {l}\n"));
    }
    out
}

#[cfg(all(test, feature = "rag3db-native"))]
pub(super) mod tests {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::hote;
    use crate::llm::{Llm, MockLlm};
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    pub(crate) fn git(dir: &Path, args: &[&str]) {
        assert!(std::process::Command::new("git").args(args).current_dir(dir).status().unwrap().success(), "git {args:?}");
    }

    /// L'atelier d'un témoin : le gabarit livré copié, sa racine posée sur `racine`, un hôte
    /// qui se connaît (les outils passent par lui), ses racines de code sous `d`.
    pub(crate) struct Atelier {
        pub host: Arc<super::super::DaemonHost>,
        pub h: Arc<super::super::Hosted>,
        pub racine: PathBuf,
    }

    impl Atelier {
        pub fn appeler(&self, outil: &str, args: Value) -> Result<Value, String> {
            self.host.with_backend(&self.h, |b| b.call(outil, args)).and_then(|r| r)
        }
        /// Le texte que lit le modèle : `result` s'il est une chaîne, sinon le JSON.
        pub fn texte(v: &Value) -> String {
            v["result"].as_str().map(str::to_string).unwrap_or_else(|| v["result"].to_string())
        }
    }

    pub(crate) fn atelier(d: &Path) -> Atelier {
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        atelier_avec(d, model)
    }

    pub(crate) fn atelier_avec(d: &Path, model: ChatModel) -> Atelier {
        atelier_avec_configuration(d, model, |_| {})
    }

    /// La déclaration du témoin est complète avant l'ouverture du backend.
    pub(crate) fn atelier_avec_configuration(d: &Path, model: ChatModel, configurer: impl FnOnce(&mut Value)) -> Atelier {
        let racine = d.join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let gabarits = d.join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        configurer(&mut m);
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let racines = crate::code_tree::Roots { config: d.join("config"), data: d.join("data") };
        let host = Arc::new(hote(d, model).with_racines_de_code(racines).with_runs_root(d.join("runs")));
        host.se_connaitre();
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");
        Atelier { host, h, racine }
    }

    /// Attendre que plus aucun index ne tourne sur le worktree `depot` (attaché) : un
    /// témoin qui finit pendant un index laisse le moteur couper sous lui (SIGSEGV à la
    /// sortie du processus).
    pub(crate) fn attendre_l_index(a: &Atelier, depot: &Path) {
        let racines = a.host.racines_de_code().unwrap();
        let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let m = racines.manifest(&nom).canonicalize().unwrap();
        let h = a.host.hosted(&m).unwrap();
        let debut = std::time::Instant::now();
        while a.host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index du worktree n'a pas fini en 120 s");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// Un petit dépôt git sous `dans`, sur sa branche `essai`.
    pub(crate) fn petit_depot(dans: &Path) -> PathBuf {
        let depot = dans.join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n\npub fn deux() -> u32 {\n    2\n}\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        depot
    }

    /// **Un `..` après un dossier absent ne sort pas de la racine** (relecture d'ED, 13 novembre
    /// 2026 ; ticket `2026-11-13-…-chemin-…-hors-de-la-racine`). `nouveau` n'existe pas :
    /// `nouveau/../../x.txt` se résout au-dessus de la racine et doit être refusé avant toute
    /// écriture, le fichier du dehors intact ; `nouveau/../notes.txt` vise le vrai `notes.txt`,
    /// qu'une insertion complète au lieu de l'écraser comme une création.
    #[test]
    fn a_dot_dot_after_a_missing_folder_does_not_leave_the_workshop_root() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let dehors = d.path().join("x.txt");
        std::fs::write(&dehors, "le fichier du dehors\n").unwrap();
        assert!(!a.racine.join("nouveau").exists());

        for outil in ["insert_lines", "replace_lines"] {
            let args = if outil == "insert_lines" { json!({ "path": "nouveau/../../x.txt", "at": 1, "text": "écrasé" }) } else { json!({ "path": "nouveau/../../x.txt", "from": 1, "to": 1, "text": "écrasé" }) };
            let r = a.appeler(outil, args);
            let dit = match &r { Ok(v) => Atelier::texte(v), Err(e) => e.clone() };
            assert!(dit.contains("hors de la racine"), "{outil} sur nouveau/../../x.txt est refusé : {dit}");
            assert_eq!(std::fs::read_to_string(&dehors).unwrap(), "le fichier du dehors\n", "{outil} : le fichier du dehors est intact");
            assert!(!a.racine.join("nouveau").exists(), "{outil} : rien n'est créé dans la racine");
        }
        let r = a.appeler("read_file", json!({ "path": "nouveau/../../x.txt" }));
        let dit = match &r { Ok(v) => Atelier::texte(v), Err(e) => e.clone() };
        assert!(dit.contains("hors de la racine") && !dit.contains("le fichier du dehors"), "la lecture aussi est refusée : {dit}");

        std::fs::write(a.racine.join("notes.txt"), "un\ndeux\n").unwrap();
        a.appeler("insert_lines", json!({ "path": "nouveau/../notes.txt", "at": 3, "text": "trois" })).expect("dans la racine, le vrai notes.txt");
        assert_eq!(std::fs::read_to_string(a.racine.join("notes.txt")).unwrap(), "un\ndeux\ntrois\n", "notes.txt complété, pas écrasé");
        assert!(!a.racine.join("nouveau").exists(), "le dossier absent n'est pas créé");
    }

    /// **Le crochet d'un témoin entre la lecture d'un fichier et son écriture** : un changement
    /// fait ailleurs à ce moment-là (une autre conversation, un `sed`, l'utilisatrice).
    #[allow(clippy::type_complexity)]
    static CROCHETS: std::sync::Mutex<Vec<(PathBuf, Box<dyn Fn() + Send>)>> = std::sync::Mutex::new(Vec::new());

    pub(crate) fn apres_lecture(chemin: &Path) {
        let crochets = CROCHETS.lock().unwrap_or_else(|p| p.into_inner());
        for (p, f) in crochets.iter() {
            if p == chemin {
                f();
            }
        }
    }

    /// **Une écriture garde le mode du fichier** (ticket d'ED,
    /// `2026-11-13-une-ecriture-de-l-atelier-perd-le-mode-du-fichier.md`) : un script `0755` et
    /// un fichier `0600`, réécrits par `replace_text`, gardent leur mode.
    #[test]
    fn a_workshop_write_keeps_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        for (nom, mode) in [("lancer.sh", 0o755), ("secret.txt", 0o600)] {
            let f = a.racine.join(nom);
            std::fs::write(&f, "un\ndeux\n").unwrap();
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(mode)).unwrap();
            a.appeler("replace_text", json!({ "path": nom, "old": "deux", "new": "DEUX" })).expect("replace_text");
            assert_eq!(std::fs::read_to_string(&f).unwrap(), "un\nDEUX\n");
            let garde = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
            assert_eq!(garde, mode, "{nom} garde son mode {mode:o} (il a {garde:o})");
        }
        let restes: Vec<_> = std::fs::read_dir(&a.racine).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.contains("atelier-tmp")).collect();
        assert!(restes.is_empty(), "aucun temporaire ne reste : {restes:?}");
    }

    /// **Une écriture dans un worktree garde aussi le mode du fichier** (le même ticket, « même
    /// vérification côté worktree ») : l'écriture passe alors par le backend de code du worktree
    /// (`edit_file`, la source du disque de `code_tools`).
    #[test]
    fn a_workshop_write_in_a_worktree_keeps_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        std::fs::write(depot.join("lancer.sh"), "echo un\n").unwrap();
        std::fs::set_permissions(depot.join("lancer.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
        git(&depot, &["add", "-A"]);
        git(&depot, &["commit", "-q", "-m", "le script"]);
        a.appeler("open_branch", json!({ "path": depot, "branch": "atelier/mode" })).expect("open_branch");
        let frere = a.racine.join("rag3db-essai-atelier-mode");
        attendre_l_index(&a, &frere);
        let f = frere.join("lancer.sh");
        assert_eq!(std::fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o755, "le worktree frère a le script exécutable");
        let dit = a.appeler("replace_text", json!({ "path": f, "old": "echo un", "new": "echo deux" })).expect("replace_text dans le worktree");
        assert!(Atelier::texte(&dit).contains("+echo deux"), "{dit}");
        let garde = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
        assert_eq!(garde, 0o755, "le script garde son mode 755 (il a {garde:o})");
        attendre_l_index(&a, &frere);
    }

    /// **Une écriture refuse un fichier changé depuis sa lecture** (ticket d'ED,
    /// `2026-11-13-une-ecriture-de-l-atelier-ecrase-sans-controle-de-concurrence.md`) : le
    /// fichier change entre la lecture d'`avant` et l'écriture ; l'atelier refuse en le
    /// disant, et le changement fait ailleurs reste.
    #[test]
    fn a_workshop_write_refuses_a_file_changed_since_it_was_read() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let f = a.racine.join("notes.txt");
        std::fs::write(&f, "un\ndeux\n").unwrap();
        let canon = f.canonicalize().unwrap();
        let ailleurs = canon.clone();
        CROCHETS.lock().unwrap().push((canon.clone(), Box::new(move || std::fs::write(&ailleurs, "changé ailleurs\n").unwrap())));
        let r = a.appeler("replace_text", json!({ "path": "notes.txt", "old": "deux", "new": "DEUX" }));
        CROCHETS.lock().unwrap().retain(|(p, _)| *p != canon);
        let dit = match &r { Ok(v) => Atelier::texte(v), Err(e) => e.clone() };
        assert!(dit.contains("a changé depuis sa lecture"), "refusé en le disant : {dit}");
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "changé ailleurs\n", "le changement fait ailleurs reste");
    }

    /// **Une réindexation en échec se dit en échec** (ticket d'ED,
    /// `2026-11-13-une-reindexation-de-l-atelier-en-echec-se-dit-reussie.md`) : le compte
    /// d'échecs du rendu de `EditResult` se lit et se dit ; un rendu non reconnu se dit tel,
    /// au lieu de « sans index ».
    #[test]
    fn a_failed_reindex_says_it_failed() {
        let dit = super::etat_de_la_reindexation("**src/a.rs** edited (3 → 4 lines, abcdef012345)\nIndex updated: 3 scopes upserted, 1 removed, 7 relations, 2 failed");
        assert!(dit.contains("2 en échec"), "le compte d'échecs est dit : {dit}");
        let dit = super::etat_de_la_reindexation("**src/a.rs** edited (3 → 4 lines, abcdef012345)\nIndex updated: 3 scopes upserted, 1 removed, 7 relations");
        assert!(dit.contains("réindexé") && !dit.contains("échec"), "{dit}");
        let dit = super::etat_de_la_reindexation("quelque chose d'inattendu");
        assert!(dit.contains("rendu non reconnu"), "un rendu non reconnu se dit tel : {dit}");
    }

    /// **Une carte décidée pendant son propre tour relance après lui** (ticket d'ED,
    /// `2026-11-13-la-relance-d-une-carte-de-l-atelier-se-dit-partie-quand-le-tour-la-refuse.md`) :
    /// le tour qui a posé la carte n'est pas fini quand l'utilisatrice accorde ; la décision
    /// doit arriver au modèle au tour suivant, une fois celui-là fini, au lieu d'être perdue sur
    /// « un tour est déjà en cours ».
    #[test]
    fn a_card_decided_during_its_own_turn_relaunches_after_it() {
        use crate::llm::{CallbackLlm, Turn};
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let fait = turns[depuis..].iter().any(|t| t.role == "tool");
                let commande = match message.strip_prefix("lance ") {
                    Some(c) => Some(c.to_string()),
                    None if message.contains("est accordée : relance-la") => message.split('`').nth(1).map(str::to_string),
                    None => None,
                };
                match commande.filter(|_| !fait) {
                    Some(c) => MockLlm::new("").with_tool_calls(vec![("run_command".to_string(), json!({ "command": c }).to_string())]).generate(turns, opts, sink),
                    None => {
                        // La réponse du premier tour tarde : la carte se décide pendant ce temps.
                        if message.starts_with("lance ") {
                            std::thread::sleep(std::time::Duration::from_millis(1500));
                        }
                        MockLlm::new("fait").generate(turns, opts, sink)
                    }
                }
            })))
        });
        let d = tempfile::tempdir().unwrap();
        let a = atelier_avec(d.path(), model);
        let (host, id_backend) = (a.host.clone(), a.h.id.clone());
        let premier = std::thread::spawn(move || host.chat_turn(&id_backend, "lance mkdir faits"));
        let debut = std::time::Instant::now();
        let id = loop {
            let trouve = a.host.accords.lock().unwrap().demandes.iter().find(|(_, d)| d.commande.as_deref() == Some("mkdir faits")).map(|(k, _)| k.clone());
            if let Some(id) = trouve {
                break id;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(20), "la carte n'est pas posée");
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert!(!premier.is_finished(), "la carte se décide pendant le tour qui l'a posée");
        a.host.decider_accord(&id, true, "le témoin").expect("accorder");
        premier.join().unwrap().expect("le premier tour");
        let debut = std::time::Instant::now();
        while !a.racine.join("faits").is_dir() {
            assert!(debut.elapsed() < std::time::Duration::from_secs(20), "la décision n'est jamais arrivée au modèle : {:#}", a.host.chat_state(&a.h.id).unwrap()["turns"]);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// **Un journal des décisions en échec n'ouvre aucun accord** (le même ticket) : la décision
    /// s'écrit avant d'ouvrir le chemin ; si elle ne peut pas s'écrire, ni accord en vigueur, ni
    /// `Ok`.
    #[test]
    fn a_decision_that_cannot_be_journaled_opens_nothing() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let dehors = d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("note.txt"), "la note\n").unwrap();
        let carte = a.appeler("ask_permission", json!({ "path": dehors.join("note.txt"), "mode": "read", "reason": "lire la note" })).expect("ask_permission");
        let id = carte["ui"][0]["id"].as_str().map(str::to_string).or_else(|| a.host.accords.lock().unwrap().demandes.keys().next().cloned()).expect("une demande");
        // Le journal ne peut pas s'écrire : un dossier à sa place.
        std::fs::create_dir_all(d.path().join("runs/decisions.jsonl")).unwrap();
        let r = a.host.decider_accord(&id, true, "le témoin");
        assert!(r.is_err(), "pas d'Ok sans journal : {r:?}");
        let lu = a.appeler("read_file", json!({ "path": dehors.join("note.txt") }));
        let dit = match &lu { Ok(v) => Atelier::texte(v), Err(e) => e.clone() };
        assert!(dit.contains("hors de la racine") && !dit.contains("la note"), "aucun accord en vigueur : {dit}");
    }

    /// **`read_file` dans l'atelier** : jamais plus de 200 lignes d'un coup (Lucie), la
    /// suite dite ; un fichier sous la racine se lit sans index ; un fichier d'un worktree
    /// attaché se lit par le backend de ce worktree.
    #[test]
    fn the_workshop_reads_a_file_two_hundred_lines_at_a_time() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let long: String = (1..=450).map(|i| format!("ligne {i}\n")).collect();
        std::fs::write(a.racine.join("notes.txt"), long).unwrap();

        let r = a.appeler("read_file", json!({ "path": "notes.txt", "length": 500 })).expect("read_file dans l'atelier");
        let t = Atelier::texte(&r);
        assert!(t.contains("ligne 200") && !t.contains("ligne 201\n"), "200 lignes au plus : {t}");
        assert!(t.contains("201"), "la suite est dite (offset 201) : {t}");
        let r = a.appeler("read_file", json!({ "path": "notes.txt", "offset": 401 })).unwrap();
        let t = Atelier::texte(&r);
        assert!(t.contains("ligne 401") && t.contains("ligne 450"), "{t}");

        // Un worktree attaché sous la racine : lu par son backend de code.
        let depot = petit_depot(&a.racine);
        let dit = a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        assert!(Atelier::texte(&dit).contains("Worktree attaché"), "{dit}");
        let r = a.appeler("read_file", json!({ "path": depot.join("src/lib.rs") })).unwrap();
        let t = Atelier::texte(&r);
        assert!(t.contains("pub fn un()") && t.contains("pub fn deux()"), "{t}");
        assert!(t.contains("rag3db-essai"), "la lecture dit le dépôt qui l'a servie : {t}");
        attendre_l_index(&a, &depot);
    }

    /// **`replace_text`** : un ancien texte unique, sinon un refus qui dit le nombre
    /// d'occurrences ; le diff unifié rendu ; dans un worktree attaché, l'index suit (le
    /// nom neuf se trouve par `usages`).
    #[test]
    fn the_workshop_replaces_a_unique_text_and_says_the_diff() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        std::fs::write(a.racine.join("conf.txt"), "a = 1\nb = 1\nc = 2\n").unwrap();
        let refus = a.appeler("replace_text", json!({ "path": "conf.txt", "old": "= 1", "new": "= 9" })).unwrap_err();
        assert!(refus.contains("2 occurrences"), "le refus dit le nombre d'occurrences : {refus}");
        let r = a.appeler("replace_text", json!({ "path": "conf.txt", "old": "c = 2", "new": "c = 3" })).expect("replace_text");
        let t = Atelier::texte(&r);
        assert!(t.contains("--- ") && t.contains("+++ ") && t.contains("@@") && t.contains("-c = 2") && t.contains("+c = 3"), "le diff unifié : {t}");
        assert_eq!(std::fs::read_to_string(a.racine.join("conf.txt")).unwrap(), "a = 1\nb = 1\nc = 3\n");

        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        // Un worktree que l'atelier n'a pas ouvert lui-même (attaché d'un dossier existant :
        // l'arbre que tient l'utilisatrice) : l'écrire demande son accord (décision de c7,
        // 9 novembre 2026), et rien n'est écrit.
        let refus = a.appeler("replace_text", json!({ "path": depot.join("src/lib.rs"), "old": "pub fn deux()", "new": "pub fn trois()" })).unwrap_err();
        assert!(refus.contains("ask_permission") && refus.contains("rag3db-essai"), "un arbre tenu par l'utilisatrice demande son accord : {refus}");
        assert!(std::fs::read_to_string(depot.join("src/lib.rs")).unwrap().contains("pub fn deux()"), "rien n'est écrit sans accord");
        attendre_l_index(&a, &depot);
    }

    /// **`replace_lines` et `insert_lines`** (par le graphe ; l'empreinte de lecture est
    /// tenue par le chat, son témoin est à part) : les lignes remplacées ou insérées, le
    /// diff unifié, les bornes refusées en le disant.
    #[test]
    fn the_workshop_replaces_and_inserts_lines_with_a_diff() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        std::fs::write(a.racine.join("liste.txt"), "un\ndeux\ntrois\nquatre\ncinq\n").unwrap();
        let r = a.appeler("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 3, "text": "DEUX\nTROIS" })).expect("replace_lines");
        let t = Atelier::texte(&r);
        assert!(t.contains("-deux") && t.contains("-trois") && t.contains("+DEUX") && t.contains("+TROIS"), "{t}");
        a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 1, "text": "zéro" })).expect("insert_lines au début");
        let r = a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 7, "text": "six" })).expect("insert_lines à la fin");
        assert!(Atelier::texte(&r).contains("+six"), "{r}");
        assert_eq!(std::fs::read_to_string(a.racine.join("liste.txt")).unwrap(), "zéro\nun\nDEUX\nTROIS\nquatre\ncinq\nsix\n");
        let refus = a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 10, "text": "x" })).unwrap_err();
        assert!(refus.contains("7 lignes"), "{refus}");
        let refus = a.appeler("replace_lines", json!({ "path": "liste.txt", "from": 6, "to": 9, "text": "x" })).unwrap_err();
        assert!(refus.contains("7 lignes"), "{refus}");
    }

    /// **`read_scope`** : un scope de l'index d'un worktree attaché, par son nom (et son
    /// fichier pour lever une ambiguïté) : son texte et ses bornes ; un nom inconnu, ou
    /// un fichier sous la racine sans index, refusés en disant pourquoi.
    #[test]
    fn the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        let r = a.appeler("read_scope", json!({ "name": "deux" })).expect("read_scope par le nom");
        let t = Atelier::texte(&r);
        assert!(t.contains("pub fn deux()") && t.contains("2") && !t.contains("pub fn un()"), "le texte du scope seul : {t}");
        assert!(t.contains("lignes 5-7") && t.contains("src/lib.rs") && t.contains("rag3db-essai"), "ses bornes, son fichier, son dépôt : {t}");
        let r = a.appeler("read_scope", json!({ "name": "un", "path": depot.join("src/lib.rs") })).expect("read_scope par le nom et le fichier");
        assert!(Atelier::texte(&r).contains("lignes 1-3"), "{r}");
        let refus = a.appeler("read_scope", json!({ "name": "inconnu_xyz" })).unwrap_err();
        assert!(refus.contains("inconnu_xyz") && refus.contains("aucun"), "{refus}");
        std::fs::write(a.racine.join("notes.txt"), "rien\n").unwrap();
        let refus = a.appeler("read_scope", json!({ "name": "rien", "path": "notes.txt" })).unwrap_err();
        assert!(refus.contains("sans index") && refus.contains("read_file"), "sous la racine, read_scope dit pourquoi : {refus}");
        attendre_l_index(&a, &depot);
    }

    /// Inscrire une déclaration complète dans les racines privées du vrai hôte,
    /// sans lancer d'index. Les outils choisis déterminent la panne observée.
    fn manifeste_de_scope_en_panne(a: &Atelier, nom: &str, outils: Value) -> PathBuf {
        let roots = a.host.racines_de_code().unwrap();
        let racine = a.racine.join(nom);
        let manifeste = roots.manifest(nom);
        std::fs::create_dir_all(&racine).unwrap();
        std::fs::create_dir_all(&roots.data).unwrap();
        std::fs::create_dir_all(manifeste.parent().unwrap()).unwrap();
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&json!({
            "version": 1, "kind": "code", "name": nom,
            "database": roots.data.join(format!("{nom}.rag3db")),
            "entities": {}, "tools": outils,
            "models": { "embed": { "provider": "service", "model": "granite-278m", "dimensions": 768 } },
            "workspace": { "source": "working_tree", "root": racine }
        })).unwrap()).unwrap();
        manifeste
    }

    #[test]
    fn read_scope_preserves_a_backend_error_instead_of_claiming_absence() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let manifeste = manifeste_de_scope_en_panne(&a, "ouverture-en-panne", json!({
            "usages": { "graph": "read-scope-ouverture-introuvable.mmd" }
        }));
        assert_eq!(a.host.worktrees_attaches().len(), 1);
        let erreur = a.appeler("read_scope", json!({ "name": "deux" })).unwrap_err();
        assert!(erreur.contains("read-scope-ouverture-introuvable.mmd"), "l'erreur initiale reste lisible : {erreur}");
        assert!(erreur.contains("accès au backend en échec") && erreur.contains("recherche incomplète"), "{erreur}");
        assert!(erreur.contains("deux") && erreur.contains("usages") && erreur.contains(manifeste.to_str().unwrap()), "{erreur}");
        assert!(!erreur.contains("aucun scope"), "une panne n'est pas une absence : {erreur}");
    }

    #[test]
    fn read_scope_preserves_a_usages_error_instead_of_claiming_absence() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let manifeste = manifeste_de_scope_en_panne(&a, "usages-non-expose", json!({}));
        assert_eq!(a.host.worktrees_attaches().len(), 1);
        let erreur = a.appeler("read_scope", json!({ "name": "deux" })).unwrap_err();
        assert!(erreur.contains("unexposed tool usages") && erreur.contains("appel usages en échec"), "{erreur}");
        assert!(erreur.contains("recherche incomplète") && erreur.contains("deux") && erreur.contains(manifeste.to_str().unwrap()), "{erreur}");
        assert!(!erreur.contains("aucun scope"), "{erreur}");
    }

    #[test]
    fn read_scope_refuses_unproven_uniqueness_after_a_real_definition() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        let trouve = a.appeler("read_scope", json!({ "name": "deux" })).unwrap();
        assert!(Atelier::texte(&trouve).contains("pub fn deux()"), "le premier index répond réellement : {trouve}");

        let manifeste = manifeste_de_scope_en_panne(&a, "zz-usages-indisponible", json!({}));
        let arbres = a.host.worktrees_attaches();
        assert_eq!(arbres.len(), 2);
        let premier = arbres.iter().position(|w| w.racine == depot.canonicalize().unwrap()).unwrap();
        let dernier = arbres.iter().position(|w| w.hosted.manifest == manifeste.canonicalize().unwrap()).unwrap();
        assert!(premier < dernier, "la définition valide est accumulée AVANT la panne");
        let erreur = a.appeler("read_scope", json!({ "name": "deux" })).unwrap_err();
        assert!(erreur.contains("recherche incomplète") && erreur.contains("unexposed tool usages"), "{erreur}");
        assert!(erreur.contains(manifeste.to_str().unwrap()), "le worktree en panne est nommé : {erreur}");
        assert!(!erreur.contains("aucun scope"), "{erreur}");
        attendre_l_index(&a, &depot);
    }

    #[test]
    fn read_scope_keeps_real_absence_and_ambiguity_after_complete_queries() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let premier = petit_depot(&a.racine.join("a"));
        // L'inscription nomme les arbres par leur dernier segment : deux noms distincts.
        let second = a.racine.join("autre-depot");
        std::fs::rename(petit_depot(&a.racine.join("b")), &second).unwrap();
        for depot in [&premier, &second] {
            a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
            attendre_l_index(&a, depot);
        }
        let ambigu = a.appeler("read_scope", json!({ "name": "deux" })).unwrap_err();
        assert!(ambigu.contains("défini 2 fois") && ambigu.contains("donnez `path`"), "{ambigu}");
        assert!(!ambigu.contains("recherche incomplète"), "{ambigu}");
        let absent = a.appeler("read_scope", json!({ "name": "absent_des_deux_arbres" })).unwrap_err();
        assert!(absent.contains("aucun scope") && absent.contains("absent_des_deux_arbres"), "{absent}");
        assert!(!absent.contains("recherche incomplète"), "{absent}");
        for depot in [&premier, &second] { attendre_l_index(&a, depot); }
    }

    /// **L'empreinte de lecture** (Lucie : replace_lines et insert_lines exigent d'avoir lu
    /// ces lignes dans ce tour ou le précédent) : par de vrais tours de chat dans l'atelier,
    /// un faux modèle qui suit un script par message.
    #[test]
    fn the_workshop_chat_writes_lines_only_after_reading_them_recently() {
        use crate::llm::{CallbackLlm, Turn};
        use std::sync::Mutex;
        // Le script : pour chaque message, les appels dans l'ordre, puis « fait ».
        let script: Vec<(&str, Vec<(&str, Value)>)> = vec![
            ("t1", vec![
                ("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 2, "text": "DEUX" })),
                ("read_file", json!({ "path": "liste.txt" })),
                ("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 2, "text": "DEUX" })),
            ]),
            ("t2", vec![("insert_lines", json!({ "path": "liste.txt", "at": 2, "text": "un et demi" }))]),
            ("t3", vec![("read_file", json!({ "path": "liste.txt", "offset": 1, "length": 3 }))]),
            ("t4", vec![("insert_lines", json!({ "path": "liste.txt", "at": 2, "text": "un et demi" }))]),
            ("t5", vec![]),
            ("t6", vec![("replace_lines", json!({ "path": "liste.txt", "from": 1, "to": 1, "text": "UN" }))]),
        ];
        let resultats: Arc<Mutex<Vec<(String, String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let vus = resultats.clone();
        let script = Arc::new(script.into_iter().map(|(m, c)| (m.to_string(), c.into_iter().map(|(n, a)| (n.to_string(), a)).collect::<Vec<_>>())).collect::<Vec<_>>());
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (script, vus) = (script.clone(), vus.clone());
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let faits: Vec<&Turn> = turns[depuis..].iter().filter(|t| t.role == "tool").collect();
                if let Some(dernier) = faits.last() {
                    vus.lock().unwrap().push((message.clone(), dernier.tool_name.clone().unwrap_or_default(), dernier.content.clone()));
                }
                let appels = script.iter().find(|(m, _)| *m == message).map(|(_, c)| c.clone()).unwrap_or_default();
                match appels.get(faits.len()) {
                    Some((nom, args)) => MockLlm::new("").with_tool_calls(vec![(nom.clone(), args.to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let d = tempfile::tempdir().unwrap();
        let a = atelier_avec(d.path(), model);
        std::fs::write(a.racine.join("liste.txt"), "un\ndeux\ntrois\nquatre\ncinq\n").unwrap();
        for m in ["t1", "t2", "t3", "t4", "t5", "t6"] {
            a.host.chat_turn(&a.h.id, m).unwrap_or_else(|e| panic!("le tour {m} : {e}"));
        }
        let r = resultats.lock().unwrap().clone();
        let de = |m: &str, outil: &str| -> Vec<String> { r.iter().filter(|(mm, o, _)| mm == m && o == outil).map(|(_, _, c)| c.clone()).collect() };
        let t1 = de("t1", "replace_lines");
        assert_eq!(t1.len(), 2, "{r:#?}");
        assert!(t1[0].contains("relis d'abord") && t1[0].contains("2-2"), "sans lecture, refusé : {}", t1[0]);
        assert!(t1[1].contains("+DEUX"), "après lecture, écrit : {}", t1[1]);
        assert!(de("t2", "insert_lines")[0].contains("relis d'abord"), "l'écriture du tour 1 a déplacé les lignes : {r:#?}");
        assert!(de("t4", "insert_lines")[0].contains("+un et demi"), "lu au tour précédent : écrit : {r:#?}");
        assert!(de("t6", "replace_lines")[0].contains("relis d'abord"), "lu deux tours plus tôt, puis écrit : refusé : {r:#?}");
        assert_eq!(std::fs::read_to_string(a.racine.join("liste.txt")).unwrap(), "un\nun et demi\nDEUX\ntrois\nquatre\ncinq\n");
    }

    /// **Un worktree né de l'atelier s'écrit sans question** (décision de c7, 9 novembre
    /// 2026) : open_branch crée la branche dans un worktree frère et le marque
    /// (`opened_by = "atelier"`) ; l'atelier l'écrit au nom de l'utilisatrice, son index
    /// suit. Le dépôt d'où elle part, que tient l'utilisatrice, demande toujours son accord.
    #[test]
    fn a_worktree_opened_by_the_workshop_is_written_without_asking() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        let dit = a.appeler("open_branch", json!({ "path": depot, "branch": "atelier/essai" })).expect("open_branch");
        let dit = Atelier::texte(&dit);
        assert!(dit.contains("atelier/essai"), "{dit}");
        let frere = a.racine.join("rag3db-essai-atelier-essai");
        assert!(frere.join("src/lib.rs").is_file(), "le worktree frère : {dit}");
        attendre_l_index(&a, &frere);
        let racines = a.host.racines_de_code().unwrap();
        let nom = crate::code_tree::name_of_tree(&frere.canonicalize().unwrap()).unwrap();
        let m = racines.manifest(&nom);
        assert!(racines.marque_de_l_atelier(&nom).is_file(), "la marque à côté du manifeste inscrit : {dit}");

        let r = a.appeler("replace_text", json!({ "path": frere.join("src/lib.rs"), "old": "pub fn deux()", "new": "pub fn trois()" })).expect("écrire le worktree né de l'atelier");
        let t = Atelier::texte(&r);
        assert!(t.contains("-pub fn deux()") && t.contains("+pub fn trois()") && t.contains("réindexé"), "{t}");
        let h = a.host.hosted(&m.canonicalize().unwrap()).unwrap();
        let u = a.host.with_backend(&h, |b| b.call("usages", json!({ "name": "trois", "format": "json" }))).unwrap().unwrap();
        assert!(u["result"]["result"]["definitions"].as_array().is_some_and(|d| !d.is_empty()), "trois est défini dans l'index : {u}");
        // Le dépôt d'origine n'est pas né de l'atelier : il n'est pas attaché ici, il est
        // sous la racine et s'écrit sans index ; attaché, il demanderait l'accord (témoin
        // the_workshop_replaces_a_unique_text_and_says_the_diff).
        attendre_l_index(&a, &frere);
    }

    /// **Une commande en attente se décide sur la carte d'accord** (lot 7 de la page
    /// `09-novembre-2026-atelier-lire-ecrire/01-conception.md`, c7, 9 novembre 2026) : dans
    /// le chat de l'atelier, une commande que la porte met en attente (`mkdir` écrit) rend la
    /// carte d'accord sous l'appel. La décision relance le tour d'elle-même : un oui l'accorde
    /// à la porte de l'atelier et le modèle, qui le lit, relance la même commande, qui passe ;
    /// un non le lui dit, et rien ne passe. Chaque décision va au journal des décisions.
    #[test]
    fn a_command_on_hold_in_the_workshop_chat_is_decided_on_the_permission_card() {
        use crate::llm::{CallbackLlm, Turn};
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let fait = turns[depuis..].iter().any(|t| t.role == "tool");
                // Un ordre (`mkdir …`), ou l'accord dit par la carte : la commande entre
                // accents graves, relancée telle quelle.
                let commande = match message.strip_prefix("lance ") {
                    Some(c) => Some(c.to_string()),
                    None if message.contains("est accordée : relance-la") => message.split('`').nth(1).map(str::to_string),
                    None => None,
                };
                match commande.filter(|_| !fait) {
                    Some(c) => MockLlm::new("").with_tool_calls(vec![("run_command".to_string(), json!({ "command": c }).to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let d = tempfile::tempdir().unwrap();
        let a = atelier_avec(d.path(), model);
        let etat = |a: &Atelier| a.host.chat_state(&a.h.id).unwrap();
        // Le dernier résultat de run_command de la conversation.
        let dernier = |a: &Atelier| -> Value { etat(a)["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "tool").last().cloned().unwrap_or(Value::Null) };
        // Le tour relancé par la carte : `n` messages de l'utilisatrice, et plus aucun tour en vol.
        let attendre_le_tour = |a: &Atelier, n: usize| {
            let debut = std::time::Instant::now();
            loop {
                let e = etat(a);
                let messages = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").count();
                if messages >= n && e["run"].is_null() {
                    return e;
                }
                assert!(debut.elapsed() < std::time::Duration::from_secs(30), "la carte n'a pas relancé de tour : {e:#}");
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        };

        a.host.chat_turn(&a.h.id, "lance mkdir faits").expect("le premier tour");
        let t = dernier(&a);
        assert!(t["content"].as_str().unwrap_or_default().contains("Commande en attente"), "mkdir attend l'accord : {t:#}");
        assert!(!a.racine.join("faits").exists(), "rien ne s'est exécuté");
        let carte = t["ui"][0].clone();
        assert_eq!(carte["kind"], "accord", "la carte d'accord sous l'appel : {t:#}");
        assert_eq!(carte["genre"], "commande", "{carte:#}");
        assert_eq!(carte["commande"], "mkdir faits", "{carte:#}");
        let id = carte["id"].as_str().expect("l'id de la demande").to_string();
        assert_eq!(a.host.etat_accord(&id).unwrap()["commande"], "mkdir faits");

        let rendu = a.host.decider_accord(&id, true, "le témoin").expect("accorder");
        assert_eq!((rendu["genre"].as_str(), rendu["relance"].as_bool()), (Some("commande"), Some(true)), "{rendu}");
        let e = attendre_le_tour(&a, 2);
        let dit = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").last().unwrap()["content"].clone();
        assert!(dit.as_str().is_some_and(|d| d.contains("`mkdir faits` est accordée")), "le modèle lit l'accord : {dit}");
        assert!(a.racine.join("faits").is_dir(), "accordée et relancée, la commande passe : {:#}", dernier(&a));

        a.host.chat_turn(&a.h.id, "lance mkdir autres").expect("le troisième tour");
        let id = dernier(&a)["ui"][0]["id"].as_str().expect("une seconde carte").to_string();
        a.host.decider_accord(&id, false, "le témoin").expect("refuser");
        let e = attendre_le_tour(&a, 4);
        let dit = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").last().unwrap()["content"].clone();
        assert!(dit.as_str().is_some_and(|d| d.contains("`mkdir autres` est refusée")), "le modèle lit le refus : {dit}");
        assert!(!a.racine.join("autres").exists(), "refusée, rien ne passe");

        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        let lignes: Vec<Value> = journal.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lignes.len(), 2, "{journal}");
        assert_eq!((lignes[0]["commande"].as_str(), lignes[0]["decision"].as_str()), (Some("mkdir faits"), Some("accordé")), "{journal}");
        assert_eq!((lignes[1]["commande"].as_str(), lignes[1]["decision"].as_str()), (Some("mkdir autres"), Some("refusé")), "{journal}");
    }

    /// **La carte d'un chemin relance son tour, elle aussi** (c7, 9 novembre 2026, le lot
    /// d'après celui des commandes) : un oui sur la carte d'`ask_permission` ouvre le chemin
    /// et le dit au modèle dans un tour qui repart seul ; il relit, et la lecture passe. Un non
    /// le lui dit aussi, et rien ne s'ouvre.
    #[test]
    fn a_path_card_decision_starts_the_turn_again_by_itself() {
        use crate::llm::{CallbackLlm, Turn};
        let d = tempfile::tempdir().unwrap();
        let dehors = d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("note.txt"), "la note du dehors\n").unwrap();
        std::fs::write(dehors.join("autre.txt"), "l'autre note\n").unwrap();
        let (note, autre) = (dehors.join("note.txt").display().to_string(), dehors.join("autre.txt").display().to_string());
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (note, autre) = (note.clone(), autre.clone());
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let fait = turns[depuis..].iter().any(|t| t.role == "tool");
                let appel = match message.as_str() {
                    "lis la note" => Some(("ask_permission", json!({ "path": note, "mode": "read", "reason": "lire la note du dehors" }))),
                    "lis l'autre" => Some(("ask_permission", json!({ "path": autre, "mode": "read", "reason": "lire l'autre note" }))),
                    m if m.contains("est accordé") => m.split('`').nth(1).map(|p| ("read_file", json!({ "path": p }))),
                    _ => None,
                };
                match appel.filter(|_| !fait) {
                    Some((nom, args)) => MockLlm::new("").with_tool_calls(vec![(nom.to_string(), args.to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let a = atelier_avec(d.path(), model);
        let etat = |a: &Atelier| a.host.chat_state(&a.h.id).unwrap();
        let dernier = |a: &Atelier| -> Value { etat(a)["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "tool").last().cloned().unwrap_or(Value::Null) };
        let attendre_le_tour = |a: &Atelier, n: usize| {
            let debut = std::time::Instant::now();
            loop {
                let e = etat(a);
                let messages = e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").count();
                if messages >= n && e["run"].is_null() {
                    return e;
                }
                assert!(debut.elapsed() < std::time::Duration::from_secs(30), "la carte n'a pas relancé de tour : {e:#}");
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        };
        let dernier_message = |e: &Value| e["turns"].as_array().unwrap().iter().filter(|t| t["role"] == "user").last().unwrap()["content"].as_str().unwrap_or_default().to_string();

        a.host.chat_turn(&a.h.id, "lis la note").expect("le premier tour");
        let id = dernier(&a)["ui"][0]["id"].as_str().expect("la carte d'accord").to_string();
        let rendu = a.host.decider_accord(&id, true, "le témoin").expect("accorder");
        assert_eq!(rendu["relance"], true, "la décision relance le tour : {rendu}");
        let e = attendre_le_tour(&a, 2);
        assert!(dernier_message(&e).contains("note.txt") && dernier_message(&e).contains("est accordé"), "le modèle lit l'accord : {}", dernier_message(&e));
        let lu = dernier(&a);
        assert_eq!(lu["tool_name"], "read_file", "{lu:#}");
        assert!(lu["content"].as_str().unwrap_or_default().contains("la note du dehors"), "la lecture passe : {lu:#}");

        a.host.chat_turn(&a.h.id, "lis l'autre").expect("le troisième tour");
        let id = dernier(&a)["ui"][0]["id"].as_str().expect("une seconde carte").to_string();
        let rendu = a.host.decider_accord(&id, false, "le témoin").expect("refuser");
        assert_eq!(rendu["relance"], true, "{rendu}");
        let e = attendre_le_tour(&a, 4);
        assert!(dernier_message(&e).contains("autre.txt") && dernier_message(&e).contains("refusé"), "le modèle lit le refus : {}", dernier_message(&e));
        assert_eq!(dernier(&a)["tool_name"], "ask_permission", "refusé, rien n'est relu");
    }

    /// **`ask_permission`** (décision de c7, 9 novembre 2026) : hors de la racine, ou sur un
    /// arbre que tient l'utilisatrice, le modèle demande l'accord ; la demande rend une carte
    /// d'accord (`ui`), l'utilisatrice accorde ou refuse ; un oui ouvre ce chemin pour ce
    /// mode, un non le garde fermé ; chaque décision va au journal des décisions (une ligne,
    /// à côté des runs), avec le chemin, le mode, la raison du modèle, qui, quand.
    #[test]
    fn a_permission_asked_by_the_workshop_opens_the_path_only_when_granted() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let dehors = d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("a.txt"), "hors de la racine\n").unwrap();
        std::fs::write(dehors.join("b.txt"), "aussi\n").unwrap();
        let refus = a.appeler("read_file", json!({ "path": dehors.join("a.txt") })).unwrap_err();
        assert!(refus.contains("hors de la racine") && refus.contains("ask_permission"), "{refus}");

        let demande = a.appeler("ask_permission", json!({ "path": dehors.join("a.txt"), "mode": "read", "reason": "lire la note posée hors de l'atelier" })).expect("ask_permission");
        let carte = demande["ui"][0].clone();
        assert_eq!(carte["kind"], "accord", "la carte d'accord sous l'appel : {demande}");
        let id = carte["id"].as_str().expect("l'id de la demande").to_string();
        assert!(Atelier::texte(&demande).contains(&id), "le modèle a l'id : {demande}");
        // Rien n'est ouvert avant la décision.
        assert!(a.appeler("read_file", json!({ "path": dehors.join("a.txt") })).is_err());
        a.host.decider_accord(&id, true, "le témoin").expect("accorder");
        let r = a.appeler("read_file", json!({ "path": dehors.join("a.txt") })).expect("lu après l'accord");
        assert!(Atelier::texte(&r).contains("hors de la racine"), "{r}");
        // Un accord de lecture n'ouvre pas l'écriture.
        assert!(a.appeler("replace_text", json!({ "path": dehors.join("a.txt"), "old": "hors", "new": "dans" })).is_err());

        let demande = a.appeler("ask_permission", json!({ "path": dehors.join("b.txt"), "mode": "write", "reason": "corriger b" })).unwrap();
        let id2 = demande["ui"][0]["id"].as_str().unwrap().to_string();
        a.host.decider_accord(&id2, false, "le témoin").expect("refuser");
        assert!(a.appeler("replace_text", json!({ "path": dehors.join("b.txt"), "old": "aussi", "new": "non" })).is_err(), "un refus garde fermé");
        assert_eq!(std::fs::read_to_string(dehors.join("b.txt")).unwrap(), "aussi\n");

        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        let lignes: Vec<Value> = journal.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lignes.len(), 2, "{journal}");
        assert_eq!((lignes[0]["decision"].clone(), lignes[0]["mode"].clone()), (json!("accordé"), json!("read")), "{journal}");
        assert_eq!(lignes[0]["raison"], "lire la note posée hors de l'atelier");
        assert!(lignes[0]["chemin"].as_str().is_some_and(|c| c.ends_with("dehors/a.txt")) && lignes[0]["qui"] == "le témoin" && lignes[0]["quand_ms"].is_u64(), "{journal}");
        assert_eq!(lignes[1]["decision"], "refusé");
    }

    /// **La carte d'accord glisse sous l'appel** (décision de c7, 9 novembre 2026) : dans le
    /// chat de l'atelier, le modèle demande à lire hors de la racine ; la carte paraît sous
    /// l'appel (le chemin, le mode, la raison), « accorder » la décide sans recharger la page,
    /// la décision va au journal ; le tour repart de lui-même, et la lecture passe.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn the_permission_card_slides_under_the_call_and_grants_the_read() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        use crate::llm::{CallbackLlm, Turn};
        const TEMOIN: &str = "the_permission_card_slides_under_the_call_and_grants_the_read";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let dehors = d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("note.txt"), "la note du dehors\n").unwrap();
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let note = dehors.join("note.txt").display().to_string();
        let vus: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
        let releve = vus.clone();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (note, vus) = (note.clone(), releve.clone());
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let faits: Vec<&Turn> = turns[depuis..].iter().filter(|t| t.role == "tool").collect();
                if let Some(t) = faits.last() {
                    vus.lock().unwrap().push(t.content.clone());
                }
                match (message.as_str(), faits.len()) {
                    ("lis la note", 0) => MockLlm::new("").with_tool_calls(vec![("ask_permission".to_string(), json!({ "path": note, "mode": "read", "reason": "lire la note posée hors de l'atelier" }).to_string())]).generate(turns, opts, sink),
                    (m, 0) if m.contains("est accordé") => MockLlm::new("").with_tool_calls(vec![("read_file".to_string(), json!({ "path": note }).to_string())]).generate(turns, opts, sink),
                    _ => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
            h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model)
        });
        let racine_url = format!("http://{adresse}");
        let poster = |route: &str, corps: Value| -> Value {
            let mut r = crate::daemon::agent_de_temoin().post(format!("{racine_url}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(corps.to_string()).unwrap();
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        assert_eq!(poster("/ide/describe", json!({ "manifest": manifeste }))["ok"], true);
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        let canon = manifeste.canonicalize().unwrap();
        let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
        n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
        n.evaluer("document.querySelector('#message').value = 'lis la note'; document.querySelector('#send').click()");
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le premier tour");
        let carte = "details.outil[data-outil=\"ask_permission\"] + .ui-du-tour .carte-accord";
        n.attendre(&format!("!!document.querySelector('{carte} button.accorder')"), "la carte d'accord sous l'appel");
        let texte = n.evaluer(&format!("document.querySelector('{carte}').textContent"));
        assert!(texte.as_str().is_some_and(|t| t.contains("lire") && t.contains("note.txt") && t.contains("lire la note posée hors de l'atelier")), "{texte}");
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('{carte} button.accorder').click()"));
        n.attendre(&format!("document.querySelector('{carte}')?.dataset.decision === 'accordé'"), "la carte dit la décision");
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");
        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        assert!(journal.contains("accordé") && journal.contains("l'utilisatrice, dans l'IDE") && journal.contains("note.txt"), "{journal}");

        // La décision relance le tour d'elle-même : rien n'est tapé.
        n.attendre(
            "[...document.querySelectorAll('.message.user')].some(m => m.textContent.includes('est accordé')) && document.querySelectorAll('.message.assistant').length === 2 && document.querySelector('#status')?.textContent === 'fini'",
            "le tour relancé par la carte",
        );
        assert!(vus.lock().unwrap().iter().any(|c| c.contains("la note du dehors")), "la lecture passe après l'accord : {:?}", vus.lock().unwrap());

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **La carte d'une commande, dans un vrai navigateur** (lot 7, c7, 9 novembre 2026) :
    /// `mkdir` attend l'accord, la carte glisse sous l'appel et dit la commande ; « accorder »
    /// la décide sans recharger la page, et le tour repart de lui-même : la page montre le
    /// message de la carte et la réponse, sans que rien ne soit tapé.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn the_command_card_grants_the_command_and_the_turn_starts_again_by_itself() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        use crate::llm::{CallbackLlm, Turn};
        const TEMOIN: &str = "the_command_card_grants_the_command_and_the_turn_starts_again_by_itself";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 8192, move |turns: &[Turn], opts, sink| {
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let message = turns[depuis].content.clone();
                let fait = turns[depuis..].iter().any(|t| t.role == "tool");
                let commande = match message.strip_prefix("lance ") {
                    Some(c) => Some(c.to_string()),
                    None if message.contains("est accordée : relance-la") => message.split('`').nth(1).map(str::to_string),
                    None => None,
                };
                match commande.filter(|_| !fait) {
                    Some(c) => MockLlm::new("").with_tool_calls(vec![("run_command".to_string(), json!({ "command": c }).to_string())]).generate(turns, opts, sink),
                    None => MockLlm::new("fait").generate(turns, opts, sink),
                }
            })))
        });
        let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| {
            h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model)
        });
        let racine_url = format!("http://{adresse}");
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{racine_url}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste }).to_string())
            .unwrap();
        let decrit: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(decrit["ok"], true, "{decrit}");
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        let canon = manifeste.canonicalize().unwrap();
        let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
        n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
        n.evaluer("document.querySelector('#message').value = 'lance mkdir faits'; document.querySelector('#send').click()");
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le premier tour");
        let carte = "details.outil[data-outil=\"run_command\"] + .ui-du-tour .carte-accord";
        n.attendre(&format!("!!document.querySelector('{carte} button.accorder')"), "la carte d'accord sous l'appel");
        let texte = n.evaluer(&format!("document.querySelector('{carte}').textContent"));
        assert!(texte.as_str().is_some_and(|t| t.contains("lancer") && t.contains("mkdir faits")), "{texte}");
        assert!(!racine.join("faits").exists(), "rien ne s'est exécuté avant l'accord");
        n.evaluer("window.__pas_rechargee = 1");
        n.evaluer(&format!("document.querySelector('{carte} button.accorder').click()"));
        n.attendre(&format!("document.querySelector('{carte}')?.dataset.decision === 'accordé'"), "la carte dit la décision");
        n.attendre(
            "[...document.querySelectorAll('.message.user')].some(m => m.textContent.includes('est accordée')) && document.querySelectorAll('.message.assistant').length === 2 && document.querySelector('#status')?.textContent === 'fini'",
            "le tour relancé par la carte, dans la page",
        );
        assert_eq!(n.evaluer("window.__pas_rechargee ?? null"), json!(1), "la page s'est rechargée");
        assert!(racine.join("faits").is_dir(), "accordée et relancée, la commande passe");
        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).expect("le journal des décisions");
        assert!(journal.contains("mkdir faits") && journal.contains("l'utilisatrice, dans l'IDE"), "{journal}");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **La jauge du contexte se voit avancer** (Lucie, 8 novembre 2026 au soir : « la barre de
    /// progression du contexte, je la vois pas beaucoup avancer, ni le pourcentage » ; c7, 9
    /// novembre). Sous un modèle d'un million de jetons, comme le sien, trois `read_file` de
    /// 200 lignes : la jauge suit chaque appel au modèle pendant le tour, écrit son pourcentage,
    /// garde une barre visible, et retrouve sa valeur après un redémarrage de l'hôte.
    #[test]
    #[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
    fn the_context_gauge_follows_each_model_call_says_its_percentage_and_survives_a_restart() {
        use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
        use crate::llm::{CallbackLlm, Turn};
        const TEMOIN: &str = "the_context_gauge_follows_each_model_call_says_its_percentage_and_survives_a_restart";
        let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        for f in ["a", "b", "c"] {
            let texte: String = (1..=200).map(|i| format!("ligne {i} du fichier {f}, quelques mots de plus pour peser dans le contexte\n")).collect();
            std::fs::write(racine.join(format!("{f}.txt")), texte).unwrap();
        }
        let gabarits = d.path().join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        // Le plafond de Lucie (gemini-3.8-flash) ; un appel lent, pour que la page ait le
        // temps de rendre chacun.
        let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            Ok(Arc::new(CallbackLlm::new("faux-atelier", 1_000_000, move |turns: &[Turn], opts, sink| {
                std::thread::sleep(std::time::Duration::from_millis(400));
                let depuis = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let faits = turns[depuis..].iter().filter(|t| t.role == "tool").count();
                match ["a.txt", "b.txt", "c.txt"].get(faits) {
                    Some(f) => MockLlm { context_len: 1_000_000, ..MockLlm::new("") }.with_tool_calls(vec![("read_file".to_string(), json!({ "path": f }).to_string())]).generate(turns, opts, sink),
                    None => MockLlm { context_len: 1_000_000, ..MockLlm::new("lu") }.generate(turns, opts, sink),
                }
            })))
        });
        let (runs, chats, taches) = (d.path().join("runs"), d.path().join("chats"), d.path().join("taches"));
        let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
        let lancer = || {
            let (runs, chats, taches, racines, model) = (runs.clone(), chats.clone(), taches.clone(), racines.clone(), model.clone());
            crate::daemon_host::tests::hote_garde_avec("j", move |h| {
                h.with_racines_de_code(racines).with_runs_root(runs).with_chats_root(chats).with_racine_des_taches(taches).with_chat_model(model).with_chats_reread()
            })
        };
        let (adresse, fil) = lancer();
        let racine_url = format!("http://{adresse}");
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{racine_url}/ide/describe"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifeste }).to_string())
            .unwrap();
        let decrit: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(decrit["ok"], true, "{decrit}");
        let mut r = crate::daemon::agent_de_temoin().get(format!("{racine_url}/ide/etat")).header("Cookie", "rag3weaver_ide=j").call().unwrap();
        let etat: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        let canon = manifeste.canonicalize().unwrap();
        let id = etat["backends"].as_array().and_then(|bs| bs.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(canon.as_path()))).and_then(|b| b["id"].as_str()).unwrap().to_string();
        let jauge = format!(".conversation[data-backend=\"{id}\"] .usage");

        let mut n = Navigateur::lancer(&binaire);
        n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        let prete = "document.querySelector('#send') && !document.querySelector('#send').disabled && document.getElementById('lien')?.textContent === 'flux : ouvert'";
        n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
        n.attendre(&format!("!!document.querySelector('{jauge}')"), "la jauge de la conversation");
        // Chaque valeur que la jauge prend pendant le tour.
        n.evaluer(&format!(
            "window.__jauge = []; new MutationObserver(() => {{ const u = document.querySelector('{jauge}')?.dataset.used; if (u && window.__jauge.at(-1) !== u) window.__jauge.push(u); }}).observe(document.body, {{ subtree: true, attributes: true, childList: true }})"
        ));
        n.evaluer("document.querySelector('#message').value = 'lis les trois'; document.querySelector('#send').click()");
        n.attendre("document.querySelector('#status')?.textContent === 'fini'", "le tour des trois lectures");
        n.attendre(&format!("Number(document.querySelector('{jauge}')?.dataset.used) > 0"), "la jauge du dernier appel");
        let vus = n.evaluer("window.__jauge");
        assert!(vus.as_array().is_some_and(|v| v.len() >= 3), "la jauge suit chaque appel au modèle pendant le tour, pas seulement la fin : {vus}");
        let texte = n.evaluer(&format!("document.querySelector('{jauge} .contexte').textContent"));
        assert!(texte.as_str().is_some_and(|t| t.contains('%')), "la jauge écrit son pourcentage : {texte}");
        let largeur = n.evaluer(&format!("document.querySelector('{jauge} [role=meter] > div').getBoundingClientRect().width"));
        assert!(largeur.as_f64().is_some_and(|l| l >= 2.0), "une barre visible, même à moins de 1 % : {largeur}");
        let avant = n.evaluer(&format!("document.querySelector('{jauge}').dataset.used"));
        let titre = n.evaluer(&format!("document.querySelector('{jauge}').title"));
        assert!(titre.as_str().is_some_and(|t| t.contains("1 000 000") || t.contains("1 M")), "au survol, le plafond : {titre}");

        // L'hôte redémarre : la jauge garde le dernier appel.
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
        let (adresse, fil) = lancer();
        let racine_url = format!("http://{adresse}");
        n.aller(&format!("{racine_url}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
        n.aller(&format!("{racine_url}/ide/code/{id}/chat"), prete);
        n.attendre(&format!("document.querySelector('{jauge}')?.dataset.used === {avant}"), "la jauge relue après le redémarrage");

        assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
        drop(n);
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine_url}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }
}


#[cfg(test)]
mod publication_accord_tests {
    use super::*;
    use std::sync::{Barrier, Mutex};
    use crate::commande::{Contexte, Garde, Mode};

    const ID: &str = "accord-synthetique";
    // Une chaîne jugée et des laissez-passer détruits : jamais exécutés.
    const LIGNE: &str = "rm ../accord-fixture-jamais-execute";

    struct Banc {
        domaine: tempfile::TempDir,
        registre: Arc<Mutex<Accords>>,
        garde: Arc<Garde>,
    }
    impl Banc {
        fn new() -> Self {
            let domaine = tempfile::tempdir().unwrap();
            let mut a = Accords::default();
            a.demandes.insert(ID.into(), Demande {
                chemin: domaine.path().to_path_buf(), mode: "command".into(), raison: "fixture".into(),
                publication: Arc::default(), decision: None, commande: Some(LIGNE.into()), backend: Some("backend-synthetique".into()),
            });
            Self { domaine, registre: Arc::new(Mutex::new(a)), garde: Arc::new(Garde::new(Mode::Approbation)) }
        }
        fn snapshot(&self) -> Demande { self.registre.lock().unwrap().demande_a_decider(ID).unwrap() }
        fn contexte(&self) -> Contexte { Contexte { accorde_par_l_utilisateur: false, domaine: Some(self.domaine.path().to_path_buf()) } }
        fn decider(&self, d: Demande, oui: bool) -> Result<Demande, String> {
            publier_decision(&self.registre, ID, d, oui, |_| self.garde.accorder_ligne(LIGNE, &self.contexte()).map(|_| ()))
        }
    }

    fn deux_reponses(oui_gagnant: bool) {
        let b = Banc::new();
        // Les deux réponses ont réellement franchi la lecture None, avant A.
        let (a, tardive) = (b.snapshot(), b.snapshot());
        let depart_tardif = Arc::new(Barrier::new(2));
        let (registre, garde, ctx, depart) = (b.registre.clone(), b.garde.clone(), b.contexte(), depart_tardif.clone());
        let seconde = std::thread::spawn(move || {
            depart.wait();
            publier_decision(&registre, ID, tardive, true, |_| garde.accorder_ligne(LIGNE, &ctx).map(|_| ()))
        });
        b.decider(a, oui_gagnant).unwrap();
        if oui_gagnant {
            assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_ok(), "premier usage disponible");
            assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_err(), "premier usage consommé");
        }
        depart_tardif.wait();
        assert!(seconde.join().unwrap().is_err(), "la réponse tardive n'est pas un second reçu");
        assert_eq!(b.registre.lock().unwrap().demandes[ID].decision, Some(oui_gagnant));
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_err(), "une réponse perdante ne crée ni ne réarme un usage CetteFois");
    }

    #[test]
    fn deux_oui_ne_rearment_pas_un_usage_deja_consomme() { deux_reponses(true); }

    #[test]
    fn un_refus_gagnant_ne_laisse_pas_le_droit_du_oui_perdant() { deux_reponses(false); }

    #[test]
    fn la_publication_serait_partagee_mais_pas_le_verrou_du_registre() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let b = Banc::new();
        let (a, tardive) = (b.snapshot(), b.snapshot());
        let publication = tardive.publication.clone();
        let publie = Arc::new(Barrier::new(2));
        let continuer = Arc::new(Barrier::new(2));
        let (registre, garde, ctx, fait, suite) = (b.registre.clone(), b.garde.clone(), b.contexte(), publie.clone(), continuer.clone());
        let premiere = std::thread::spawn(move || publier_decision(&registre, ID, a, true, |_| {
            garde.accorder_ligne(LIGNE, &ctx)?;
            // A est retenu après publication, avant commit ; même primitive que l'API.
            fait.wait();
            suite.wait();
            Ok(())
        }));
        publie.wait();
        let demande_verrouillee = publication.try_lock().is_err();
        let registre_libre = b.registre.try_lock().is_ok();
        let appels_perdant = Arc::new(AtomicUsize::new(0));
        let (registre, garde, ctx, appels) = (b.registre.clone(), b.garde.clone(), b.contexte(), appels_perdant.clone());
        let (annonce, attendu) = std::sync::mpsc::sync_channel(1);
        let seconde = std::thread::spawn(move || {
            annonce.send(()).unwrap();
            publier_decision(&registre, ID, tardive, true, |_| {
                appels.fetch_add(1, Ordering::SeqCst);
                garde.accorder_ligne(LIGNE, &ctx).map(|_| ())
            })
        });
        let b_annonce = attendu.recv_timeout(std::time::Duration::from_secs(5)).is_ok();
        // Toujours libérer et joindre avant les assertions, même si un invariant casse.
        continuer.wait();
        let (premier, second) = (premiere.join().unwrap(), seconde.join().unwrap());
        assert!(demande_verrouillee && registre_libre && b_annonce);
        assert!(premier.is_ok() && second.is_err());
        assert_eq!(appels_perdant.load(Ordering::SeqCst), 0);
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_ok());
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_err());
    }

    #[test]
    fn une_erreur_de_publication_laisse_la_demande_retentable() {
        let b = Banc::new();
        let e = publier_decision(&b.registre, ID, b.snapshot(), true, |_| Err("porte synthétique indisponible".into())).unwrap_err();
        assert!(e.contains("indisponible"));
        assert_eq!(b.registre.lock().unwrap().demandes[ID].decision, None);
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_err());
        b.decider(b.snapshot(), true).unwrap();
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_ok());
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_err());
    }

    #[test]
    fn refuser_une_carte_deja_accordee_n_est_pas_une_api_de_revocation() {
        let b = Banc::new();
        b.decider(b.snapshot(), true).unwrap();
        assert!(b.registre.lock().unwrap().demande_a_decider(ID).unwrap_err().contains("déjà accordée"));
        // Le contrat actuel n'expose aucune révocation : le premier usage demeure.
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_ok());
        assert!(b.garde.autoriser_ligne(LIGNE, &b.contexte()).is_err());
    }

    #[test]
    fn recharger_la_carte_conserve_la_decision_redemarrer_ne_la_rejoue_pas() {
        let d = tempfile::tempdir().unwrap();
        let host = DaemonHost::new().with_runs_root(d.path().join("runs"));
        let p = d.path().join("fixture");
        let answer = host.atelier_demander(None, p.to_str().unwrap(), "read", "fixture").unwrap();
        let id = answer.ui[0]["id"].as_str().unwrap();
        let recu = host.decider_accord(id, true, "fixture").unwrap();
        assert_eq!(recu["relance"], false);
        assert_eq!(host.etat_accord(id).unwrap()["decision"], "accordé");
        assert!(host.decider_accord(id, false, "fixture").is_err());
        let journal = std::fs::read_to_string(d.path().join("runs/decisions.jsonl")).unwrap();
        assert_eq!(journal.lines().count(), 1, "un perdant ne publie pas un second reçu");
        let nouveau = DaemonHost::new().with_runs_root(d.path().join("runs"));
        assert!(nouveau.etat_accord(id).is_none());
        assert!(!nouveau.accorde(&p, "read"));
        assert!(host.accorde(&p, "read"));
    }

    #[test]
    fn erreur_du_journal_apres_commit_ne_signifie_pas_absence_d_effet() {
        let d = tempfile::tempdir().unwrap();
        let host = DaemonHost::new().with_runs_root(d.path().join("runs"));
        std::fs::create_dir_all(d.path().join("runs/decisions.jsonl")).unwrap();
        let p = d.path().join("fixture");
        let answer = host.atelier_demander(None, p.to_str().unwrap(), "read", "fixture").unwrap();
        let id = answer.ui[0]["id"].as_str().unwrap();
        assert!(host.decider_accord(id, true, "fixture").is_err());
        assert_eq!(host.etat_accord(id).unwrap()["decision"], "accordé");
        assert!(host.accorde(&p, "read"), "qualification du contrat actuel, pas correction du journal");
        assert!(host.decider_accord(id, true, "fixture").unwrap_err().contains("déjà"));
    }
}
