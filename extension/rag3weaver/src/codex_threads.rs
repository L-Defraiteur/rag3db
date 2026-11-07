//! **Les fils Codex, vus d'ici** (lot 5 des hooks, page 07 du 7 novembre 2026) :
//! les découvrir, résoudre un nom en UUID, et livrer une ligne de la boîte par
//! `codex queue`.
//!
//! Tout se lit, rien ne s'écrit chez Codex : `~/.codex/session_index.jsonl`
//! (`{id, thread_name, updated_at}`, une ligne par renommage, la dernière fait
//! foi) et la première ligne des transcriptions
//! `~/.codex/sessions/AAAA/MM/JJ/rollout-…-<uuid>.jsonl` (`session_meta`, son
//! `cwd`). `CODEX_HOME` donne une autre racine, comme chez Codex.
//!
//! La livraison adresse **l'UUID** : le 7 novembre, `codex queue --thread <nom>`
//! a rendu « No active session found matching 'rag3weaver1' » sur un fil vivant,
//! l'UUID a passé (essai de c7). Le nom est la clé stable (`codex:<nom>`),
//! l'UUID le handle de la session courante. Jamais `--resume`.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// La racine de Codex : `CODEX_HOME`, sinon `~/.codex`.
pub fn racine() -> PathBuf {
    if let Some(r) = std::env::var_os("CODEX_HOME").filter(|r| !r.is_empty()) {
        return PathBuf::from(r);
    }
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(".codex")
}

/// Le binaire de Codex : `RAG3WEAVER_CODEX_BIN` (les témoins y mettent un faux),
/// sinon `codex` sur le PATH.
pub fn binaire() -> PathBuf {
    std::env::var_os("RAG3WEAVER_CODEX_BIN").filter(|b| !b.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("codex"))
}

/// **Un fil Codex**.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fil {
    pub id: String,
    pub nom: Option<String>,
    pub updated_at: Option<String>,
    pub cwd: Option<PathBuf>,
    pub transcription: Option<PathBuf>,
}

impl Fil {
    /// La clé stable : `codex:<nom>`, `codex:<uuid>` sans nom.
    pub fn cle(&self) -> String {
        format!("codex:{}", self.nom.as_deref().unwrap_or(&self.id))
    }
}

/// L'UUID qui finit le nom d'une transcription `rollout-<date>-<uuid>.jsonl`.
fn uuid_du_rollout(nom: &str) -> Option<&str> {
    let tige = nom.strip_prefix("rollout-")?.strip_suffix(".jsonl")?;
    let id = tige.get(tige.len().checked_sub(36)?..)?;
    (id.len() == 36 && id.chars().filter(|c| *c == '-').count() == 4).then_some(id)
}

fn les_transcriptions(dossier: &Path, out: &mut HashMap<String, PathBuf>) {
    let Ok(lecture) = std::fs::read_dir(dossier) else { return };
    for e in lecture.flatten() {
        let p = e.path();
        if p.is_dir() {
            les_transcriptions(&p, out);
        } else if let Some(id) = p.file_name().and_then(|n| n.to_str()).and_then(uuid_du_rollout) {
            out.insert(id.to_string(), p);
        }
    }
}

/// Le `cwd` de la première ligne (`session_meta`) d'une transcription.
fn cwd_de(transcription: &Path) -> Option<PathBuf> {
    use std::io::BufRead;
    let f = std::fs::File::open(transcription).ok()?;
    let mut premiere = String::new();
    std::io::BufReader::new(f).read_line(&mut premiere).ok()?;
    let v: Value = serde_json::from_str(&premiere).ok()?;
    v["payload"]["cwd"].as_str().map(PathBuf::from)
}

/// **Les fils connus**, les plus récents d'abord : l'index (le dernier nom
/// d'un id fait foi) et les transcriptions sans nom.
pub fn fils(racine: &Path) -> Vec<Fil> {
    let mut par_id: HashMap<String, Fil> = HashMap::new();
    if let Ok(texte) = std::fs::read_to_string(racine.join("session_index.jsonl")) {
        for l in texte.lines() {
            let Ok(v) = serde_json::from_str::<Value>(l) else { continue };
            let Some(id) = v["id"].as_str().filter(|i| !i.is_empty()) else { continue };
            let f = par_id.entry(id.to_string()).or_insert_with(|| Fil { id: id.to_string(), ..Default::default() });
            if let Some(n) = v["thread_name"].as_str().filter(|n| !n.trim().is_empty()) {
                f.nom = Some(n.trim().to_string());
            }
            if let Some(u) = v["updated_at"].as_str() {
                f.updated_at = Some(u.to_string());
            }
        }
    }
    let mut transcriptions = HashMap::new();
    les_transcriptions(&racine.join("sessions"), &mut transcriptions);
    for (id, chemin) in transcriptions {
        let f = par_id.entry(id.clone()).or_insert_with(|| Fil { id, ..Default::default() });
        f.cwd = cwd_de(&chemin);
        if f.updated_at.is_none() {
            f.updated_at = std::fs::metadata(&chemin).and_then(|m| m.modified()).ok().map(|t| {
                let s = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                format!("@{s}")
            });
        }
        f.transcription = Some(chemin);
    }
    let mut v: Vec<Fil> = par_id.into_values().collect();
    v.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
    v
}

/// Les fils dont le `cwd` est ce dossier ou l'un de ses sous-dossiers.
pub fn fils_du_dossier(racine: &Path, dossier: &Path) -> Vec<Fil> {
    fils(racine).into_iter().filter(|f| f.cwd.as_deref().is_some_and(|c| c.starts_with(dossier) || dossier.starts_with(c))).collect()
}

/// **Résout une clé `codex:<nom>` (ou `codex:<uuid>`) en UUID de fil.** Un
/// nom porté par plusieurs fils prend le plus récent. Rien de connu : la
/// raison, que la ligne garde en attente.
pub fn resoudre(racine: &Path, cle: &str) -> Result<String, String> {
    let nom = cle.strip_prefix("codex:").filter(|n| !n.is_empty()).ok_or_else(|| format!("« {cle} » n'est pas une clé codex:<nom>"))?;
    let tous = fils(racine);
    if let Some(f) = tous.iter().find(|f| f.id == nom) {
        return Ok(f.id.clone());
    }
    tous.iter()
        .find(|f| f.nom.as_deref() == Some(nom))
        .map(|f| f.id.clone())
        .ok_or_else(|| format!("aucun fil Codex nommé « {nom} » dans {} (ni transcription de cet id)", racine.join("session_index.jsonl").display()))
}

/// Le message déposé chez Codex : la ligne citée, son id, la commande de réponse.
pub fn message_de_livraison(ligne: &Value, cle_codex: &str) -> String {
    let id = ligne["id"].as_str().unwrap_or("?");
    let de = ligne["from"].as_str().unwrap_or("?");
    let tache = ligne["task"].as_str().unwrap_or("?");
    format!(
        "[boîte rag3weaver] {de} t'écrit (tâche {tache}, id {id}) : {}\nRéponds dans ton shell : rag3weaver-backend mail reply --from {cle_codex} --to {id} \"<ta réponse>\"",
        crate::mail::citer(ligne["text"].as_str().unwrap_or(""), 4_000)
    )
}

/// **Livre un message à un fil** : `codex queue --thread <uuid> --message <m>`,
/// sous un délai ; un code non nul ou un délai dépassé est la raison rendue.
pub fn livrer(binaire: &Path, uuid: &str, message: &str, delai: Duration) -> Result<(), String> {
    let mut fils = std::process::Command::new(binaire)
        .args(["queue", "--thread", uuid, "--message", message])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("{} queue : ne démarre pas : {e}", binaire.display()))?;
    let fin = Instant::now() + delai;
    loop {
        match fils.try_wait() {
            Ok(Some(statut)) => {
                let sortie = fils.wait_with_output().map_err(|e| e.to_string())?;
                if statut.success() {
                    return Ok(());
                }
                let err = String::from_utf8_lossy(&sortie.stderr);
                let out = String::from_utf8_lossy(&sortie.stdout);
                let dit = if err.trim().is_empty() { out.trim().to_string() } else { err.trim().to_string() };
                return Err(format!("codex queue : code {} — {}", statut.code().unwrap_or(-1), dit.lines().next().unwrap_or("")));
            }
            Ok(None) if Instant::now() < fin => std::thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = fils.kill();
                let _ = fils.wait();
                return Err(format!("codex queue : pas de réponse en {} s", delai.as_secs()));
            }
            Err(e) => return Err(format!("codex queue : {e}")),
        }
    }
}

/// **Traite une ligne de la boîte** adressée à `codex:` : résout, livre, et rend
/// la ligne `delivery` à écrire dans la boîte (`to` = l'expéditeur, pour qu'il
/// voie l'attente dans son `pending`). `None` si la ligne n'est pas pour Codex.
pub fn traiter(ligne: &Value, racine: &Path, binaire: &Path, delai: Duration) -> Option<Value> {
    let to = ligne["to"].as_str().filter(|t| t.starts_with("codex:"))?;
    let id = ligne["id"].as_str().unwrap_or("");
    let (livre, texte, fil) = match resoudre(racine, to) {
        Err(raison) => (false, format!("en attente : {raison}"), None),
        Ok(uuid) => match livrer(binaire, &uuid, &message_de_livraison(ligne, to), delai) {
            Ok(()) => (true, format!("livré au fil {uuid}"), Some(uuid)),
            Err(raison) => (false, format!("en attente : {raison}"), Some(uuid)),
        },
    };
    let mut d = json!({ "from": "host", "kind": "delivery", "reply_to": id, "delivered": livre, "text": texte });
    if let Some(f) = ligne["from"].as_str() {
        d["to"] = json!(f);
    }
    if let Some(u) = fil {
        d["thread"] = json!(u);
    }
    for k in ["parent", "task", "backend"] {
        if let Some(v) = ligne.get(k) {
            d[k] = v.clone();
        }
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const U1: &str = "01a1b6af-70d1-7b93-a9aa-0811cba4fc62";
    const U2: &str = "01a1b6b0-fced-71a1-bc96-8740d2db55ca";

    fn racine_de_test(d: &Path) -> PathBuf {
        let r = d.join("codex");
        std::fs::create_dir_all(r.join("sessions/2026/11/07")).unwrap();
        std::fs::write(
            r.join("session_index.jsonl"),
            format!(
                "{{\"id\":\"{U1}\",\"thread_name\":\"ancien nom\",\"updated_at\":\"2026-11-07T15:00:00Z\"}}\n{{\"id\":\"{U1}\",\"thread_name\":\"rag3weaver1\",\"updated_at\":\"2026-11-07T15:45:23Z\"}}\nligne illisible\n"
            ),
        )
        .unwrap();
        for (u, cwd) in [(U1, "/depot/rag3db"), (U2, "/ailleurs")] {
            std::fs::write(r.join(format!("sessions/2026/11/07/rollout-2026-11-07T16-45-13-{u}.jsonl")), format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{u}\",\"cwd\":\"{cwd}\"}}}}\n{{\"type\":\"autre\"}}\n")).unwrap();
        }
        r
    }

    #[test]
    fn les_fils_viennent_de_l_index_et_des_transcriptions_le_dernier_nom_fait_foi() {
        let d = tempfile::tempdir().unwrap();
        let r = racine_de_test(d.path());
        let f = fils(&r);
        assert_eq!(f.len(), 2, "{f:?}");
        let un = f.iter().find(|f| f.id == U1).unwrap();
        assert_eq!((un.nom.as_deref(), un.cwd.as_deref()), (Some("rag3weaver1"), Some(Path::new("/depot/rag3db"))));
        assert_eq!(un.cle(), "codex:rag3weaver1");
        assert_eq!(f.iter().find(|f| f.id == U2).unwrap().cle(), format!("codex:{U2}"), "sans nom, la clé est l'UUID");
        assert_eq!(fils_du_dossier(&r, Path::new("/depot/rag3db/extension")).iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), vec![U1]);
    }

    #[test]
    fn un_nom_se_resout_en_uuid_un_inconnu_rend_sa_raison() {
        let d = tempfile::tempdir().unwrap();
        let r = racine_de_test(d.path());
        assert_eq!(resoudre(&r, "codex:rag3weaver1").unwrap(), U1);
        assert_eq!(resoudre(&r, &format!("codex:{U2}")).unwrap(), U2);
        assert!(resoudre(&r, "codex:ancien nom").is_err(), "un nom remplacé ne résout plus");
        let e = resoudre(&r, "codex:personne").unwrap_err();
        assert!(e.contains("aucun fil Codex nommé « personne »"), "{e}");
        assert!(resoudre(&r, "claude-code:x").is_err());
    }

    /// Un faux `codex` : il range ses arguments, un par ligne, et sort selon `CODE`.
    fn faux_codex(d: &Path, code: i32) -> (PathBuf, PathBuf) {
        let trace = d.join("trace");
        let bin = d.join("faux-codex");
        std::fs::write(&bin, format!("#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> {}; done\necho 'No active session found' >&2\nexit {code}\n", trace.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        (bin, trace)
    }

    #[test]
    fn une_ligne_pour_codex_est_livree_par_queue_a_l_uuid_et_dit_sa_livraison() {
        let d = tempfile::tempdir().unwrap();
        let r = racine_de_test(d.path());
        let (bin, trace) = faux_codex(d.path(), 0);
        let ligne = json!({ "id": "b1/doc:4", "parent": "b1", "task": "doc", "from": "claude-code:s1", "to": "codex:rag3weaver1", "kind": "message", "text": "relis le diff" });
        let l = traiter(&ligne, &r, &bin, Duration::from_secs(5)).unwrap();
        assert_eq!((l["kind"].as_str(), l["delivered"].as_bool(), l["reply_to"].as_str(), l["to"].as_str(), l["thread"].as_str()), (Some("delivery"), Some(true), Some("b1/doc:4"), Some("claude-code:s1"), Some(U1)));
        let args: Vec<String> = std::fs::read_to_string(&trace).unwrap().lines().map(str::to_string).collect();
        assert_eq!(&args[..4], &["queue", "--thread", U1, "--message"], "l'UUID, jamais le nom ni --resume : {args:?}");
        let m = args[4..].join("\n");
        assert!(m.contains("claude-code:s1 t'écrit (tâche doc, id b1/doc:4) : « relis le diff »") && m.contains("mail reply --from codex:rag3weaver1 --to b1/doc:4"), "{m}");
        assert!(!args.iter().any(|a| a == "--resume"));
        assert!(traiter(&json!({ "to": "claude-code:s1" }), &r, &bin, Duration::from_secs(1)).is_none(), "une ligne pour un autre n'est pas à nous");
    }

    #[test]
    fn un_echec_ou_un_nom_inconnu_laisse_la_ligne_en_attente_avec_sa_raison() {
        let d = tempfile::tempdir().unwrap();
        let r = racine_de_test(d.path());
        let (bin, trace) = faux_codex(d.path(), 1);
        let ligne = json!({ "id": "b1/doc:4", "from": "claude-code:s1", "to": "codex:rag3weaver1", "text": "x" });
        let l = traiter(&ligne, &r, &bin, Duration::from_secs(5)).unwrap();
        assert_eq!(l["delivered"], false);
        assert!(l["text"].as_str().unwrap().contains("en attente : codex queue : code 1 — No active session found"), "{l}");
        std::fs::remove_file(&trace).unwrap();
        let l = traiter(&json!({ "id": "b1/doc:5", "from": "claude-code:s1", "to": "codex:personne", "text": "x" }), &r, &bin, Duration::from_secs(5)).unwrap();
        assert_eq!(l["delivered"], false);
        assert!(l["text"].as_str().unwrap().contains("aucun fil Codex nommé « personne »"), "{l}");
        assert!(!trace.exists(), "un nom inconnu n'appelle pas codex");
    }

    #[test]
    fn un_codex_qui_ne_rend_pas_la_main_est_coupe_au_delai() {
        let d = tempfile::tempdir().unwrap();
        let bin = d.path().join("lent");
        std::fs::write(&bin, "#!/bin/sh\nsleep 30\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let debut = Instant::now();
        let e = livrer(&bin, U1, "x", Duration::from_millis(300)).unwrap_err();
        assert!(e.contains("pas de réponse en 0 s") && debut.elapsed() < Duration::from_secs(5), "{e}");
    }
}
