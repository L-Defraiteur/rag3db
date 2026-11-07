//! **L'adaptateur de transcription de Claude Code** (page
//! `docs/07-novembre-2026-hooks-claude-code/01` §4) : le `.jsonl` d'une session,
//! lu pour les seules choses dont les hooks ont besoin.
//!
//! Le format est **interne à Claude Code et non documenté** ; relevé le 7
//! novembre 2026 sur les transcriptions de ce poste. L'adaptateur est petit et
//! tolérant : une ligne qu'il ne comprend pas est sautée et comptée, jamais une
//! raison de se taire ; un format qui bouge casse un témoin, pas une session.
//!
//! Ce qu'on lit :
//! - **le contexte réel du dernier tour** : la dernière ligne `assistant` porte
//!   `message.usage` (`input_tokens` + `cache_read_input_tokens` +
//!   `cache_creation_input_tokens`). C'est exact, là où la taille du fichier ne
//!   dit rien (mesuré : les tranches entre deux compactions vont de 76 Ko à 10 Mo,
//!   et Claude Code compacte à ~96 % du million) ;
//! - **les fichiers que la session a touchés** : les blocs `tool_use` des outils
//!   d'édition (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`), leur `file_path` ;
//! - **la compaction** : une ligne `system` de sous-type `compact_boundary`, puis
//!   une ligne `user` marquée `isCompactSummary` qui porte le résumé ; « premier
//!   tour après compaction » veut dire : aucun tour `assistant` depuis la dernière.

use serde_json::Value;
use std::io::BufRead;
use std::path::Path;

/// Les outils dont un `tool_use` touche un fichier.
pub const OUTILS_D_EDITION: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcription {
    /// Le contexte du dernier tour `assistant` qui en dit un, en jetons.
    pub dernier_usage: Option<u64>,
    /// Les fichiers édités par la session, dans l'ordre, sans doublon.
    pub fichiers_touches: Vec<String>,
    /// Le texte du dernier résumé de compaction, s'il y en a eu une.
    pub dernier_resume: Option<String>,
    /// Le `trigger` de la dernière compaction (`manual`, `auto`), relevé sur la frontière.
    pub derniere_compaction: Option<String>,
    /// Aucun tour `assistant` depuis la dernière compaction : c'est le premier.
    pub premier_tour_apres_compaction: bool,
    /// Les lignes lues, et celles qu'on n'a pas comprises.
    pub lignes: usize,
    pub lignes_illisibles: usize,
    /// La taille du fichier, le repli quand aucune ligne ne dit son usage.
    pub octets: u64,
}

impl Transcription {
    pub fn lire(chemin: &Path) -> Result<Self, String> {
        let f = std::fs::File::open(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let octets = f.metadata().map(|m| m.len()).unwrap_or(0);
        let mut t = Self::depuis_lignes(std::io::BufReader::new(f).lines().map_while(Result::ok));
        t.octets = octets;
        Ok(t)
    }

    /// Le même, sur des lignes déjà lues (les témoins).
    pub fn depuis_lignes<I: IntoIterator<Item = String>>(lignes: I) -> Self {
        let mut t = Self::default();
        // Un tour assistant vu depuis la dernière compaction ?
        let mut assistant_depuis_compaction = false;
        let mut compaction_vue = false;
        for ligne in lignes {
            if ligne.trim().is_empty() {
                continue;
            }
            t.lignes += 1;
            let Ok(v) = serde_json::from_str::<Value>(&ligne) else {
                t.lignes_illisibles += 1;
                continue;
            };
            let Some(genre) = v["type"].as_str() else {
                t.lignes_illisibles += 1;
                continue;
            };
            match genre {
                "assistant" => {
                    assistant_depuis_compaction = true;
                    let u = &v["message"]["usage"];
                    if u.is_object() {
                        let total = ["input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"]
                            .iter()
                            .filter_map(|k| u[k].as_u64())
                            .sum::<u64>();
                        if total > 0 {
                            t.dernier_usage = Some(total);
                        }
                    }
                    if let Some(blocs) = v["message"]["content"].as_array() {
                        for b in blocs {
                            if b["type"] == "tool_use" && OUTILS_D_EDITION.contains(&b["name"].as_str().unwrap_or("")) {
                                if let Some(p) = b["input"]["file_path"].as_str() {
                                    if !t.fichiers_touches.iter().any(|x| x == p) {
                                        t.fichiers_touches.push(p.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                "system" if v["subtype"] == "compact_boundary" => {
                    compaction_vue = true;
                    assistant_depuis_compaction = false;
                    t.derniere_compaction = v["compactMetadata"]["trigger"].as_str().map(str::to_string);
                }
                "user" if v["isCompactSummary"] == true => {
                    compaction_vue = true;
                    assistant_depuis_compaction = false;
                    t.dernier_resume = texte_du_message(&v["message"]["content"]);
                }
                // `user`, `attachment`, `queue-operation`, `last-prompt`… : rien à lire.
                _ => {}
            }
        }
        t.premier_tour_apres_compaction = compaction_vue && !assistant_depuis_compaction;
        t
    }

    /// Le contexte en jetons : l'usage lu, sinon le repli par les octets (÷ 4),
    /// avec un mot qui dit lequel.
    pub fn contexte_estime(&self) -> (u64, &'static str) {
        match self.dernier_usage {
            Some(n) => (n, "usage du dernier tour"),
            None => (self.octets / 4, "octets du fichier ÷ 4, estimation grossière"),
        }
    }
}

/// Le texte d'un `content` : une chaîne, ou des blocs `text` joints.
fn texte_du_message(c: &Value) -> Option<String> {
    match c {
        Value::String(s) => Some(s.clone()),
        Value::Array(blocs) => {
            let t: Vec<&str> = blocs.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect();
            if t.is_empty() { None } else { Some(t.join("\n")) }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Les formes relevées le 7 novembre 2026, raccourcies.
    const USER: &str = r#"{"type":"user","promptId":"p1","message":{"role":"user","content":"salut"},"timestamp":"2026-09-25T14:34:42.189Z"}"#;
    const ASSISTANT_1: &str = r#"{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":2,"cache_creation_input_tokens":8642,"cache_read_input_tokens":31926,"output_tokens":352,"output_tokens_details":{"thinking_tokens":195}},"content":[{"type":"text","text":"ok"}]},"timestamp":"2026-09-25T14:34:45.504Z","uuid":"u1"}"#;
    const ASSISTANT_EDIT: &str = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Write","id":"toolu_1","input":{"file_path":"/p/src/a.rs","content":"…"}},{"type":"tool_use","name":"Edit","id":"toolu_2","input":{"file_path":"/p/src/b.rs","old_string":"x","new_string":"y"}},{"type":"tool_use","name":"Read","id":"toolu_3","input":{"file_path":"/p/src/c.rs"}}]}}"#;
    const ASSISTANT_2: &str = r#"{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":900000,"output_tokens":10},"content":[{"type":"tool_use","name":"Edit","id":"toolu_4","input":{"file_path":"/p/src/a.rs","old_string":"1","new_string":"2"}}]}}"#;
    const BOUNDARY: &str = r#"{"isSidechain":false,"type":"system","subtype":"compact_boundary","content":"Conversation compacted","level":"info","compactMetadata":{"trigger":"auto","preTokens":967349,"postTokens":12338}}"#;
    const SUMMARY: &str = r#"{"type":"user","isCompactSummary":true,"message":{"role":"user","content":"This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\nSummary…"},"promptId":"p9"}"#;
    const BRUIT: &str = r#"{"type":"queue-operation","operation":"enqueue","sessionId":"s"}"#;

    fn lignes(v: &[&str]) -> Transcription {
        Transcription::depuis_lignes(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn l_usage_du_dernier_tour_est_la_somme_des_trois_comptes() {
        let t = lignes(&[USER, ASSISTANT_1]);
        assert_eq!(t.dernier_usage, Some(2 + 8642 + 31926));
        assert_eq!(t.contexte_estime(), (40570, "usage du dernier tour"));
        let t = lignes(&[USER, ASSISTANT_1, ASSISTANT_EDIT, ASSISTANT_2]);
        assert_eq!(t.dernier_usage, Some(900105), "le dernier tour qui dit un usage ; un tour sans usage ne l'efface pas");
    }

    #[test]
    fn les_fichiers_touches_sont_ceux_des_outils_d_edition_sans_doublon() {
        let t = lignes(&[USER, ASSISTANT_EDIT, ASSISTANT_2]);
        assert_eq!(t.fichiers_touches, vec!["/p/src/a.rs", "/p/src/b.rs"], "Read n'édite pas ; a.rs une seule fois");
    }

    #[test]
    fn la_compaction_se_voit_et_le_premier_tour_apres_elle_aussi() {
        let avant = lignes(&[USER, ASSISTANT_1]);
        assert!(!avant.premier_tour_apres_compaction && avant.dernier_resume.is_none());
        let juste_apres = lignes(&[USER, ASSISTANT_1, BOUNDARY, SUMMARY]);
        assert!(juste_apres.premier_tour_apres_compaction);
        assert_eq!(juste_apres.derniere_compaction.as_deref(), Some("auto"));
        assert!(juste_apres.dernier_resume.unwrap().starts_with("This session is being continued"));
        let plus_tard = lignes(&[USER, ASSISTANT_1, BOUNDARY, SUMMARY, ASSISTANT_2]);
        assert!(!plus_tard.premier_tour_apres_compaction, "un tour assistant est passé");
        assert_eq!(plus_tard.dernier_usage, Some(900105));
    }

    #[test]
    fn une_ligne_illisible_est_comptee_jamais_une_erreur() {
        let t = lignes(&[USER, "pas du json", r#"{"sans":"type"}"#, BRUIT, ASSISTANT_1, ""]);
        assert_eq!((t.lignes, t.lignes_illisibles), (5, 2));
        assert_eq!(t.dernier_usage, Some(40570));
    }

    #[test]
    fn sans_usage_le_repli_est_dit() {
        let mut t = lignes(&[USER, BRUIT]);
        t.octets = 4000;
        assert_eq!(t.contexte_estime(), (1000, "octets du fichier ÷ 4, estimation grossière"));
    }

    #[test]
    fn un_fichier_se_lit_et_un_absent_est_une_erreur_dite() {
        let dossier = std::env::temp_dir().join(format!("transcription-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("s.jsonl");
        std::fs::write(&chemin, format!("{USER}\n{ASSISTANT_1}\n{BOUNDARY}\n{SUMMARY}\n")).unwrap();
        let t = Transcription::lire(&chemin).unwrap();
        assert!(t.premier_tour_apres_compaction && t.octets > 0 && t.lignes == 4);
        assert!(Transcription::lire(&dossier.join("absent.jsonl")).unwrap_err().contains("absent.jsonl"));
        let _ = std::fs::remove_dir_all(&dossier);
    }
}
