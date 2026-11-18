//! **The session archived as it goes** (page 07 of 18 Nov 2026, rag3weaver-archi; Lucie: « the
//! archivist at each slice of tokens, and that's it; at compaction time, everything is already
//! filed but the last slice »). At the end of each turn the host writes the turn's messages to
//! the memory declared by `bilans.memoire` (`put_turn`); once the session holds more than
//! `memory.slice_tokens` since the last archived turn, the slice is filed in the background by
//! the archivist (`archive_slice`), one at a time per conversation, and the next turn never waits.
//! The mark « archived through turn T » moves only on a report without error, to the value the
//! memory returns; it lives in `chats/<backend>.rangement.json`, whose only writer is the
//! archiving thread.

use serde_json::Value;
use std::path::Path;

/// The two calls the host makes to the memory: a seam, so that a witness can play them.
pub trait Rangeur: Send + Sync {
    /// `put_turn` on the memory `memoire`: one message of a turn.
    fn put_turn(&self, memoire: &Path, line: &Value) -> Result<Value, String>;
    /// `archive_slice` on the memory `memoire`: `{run, from_turn, to_turn, context}`; returns
    /// `{report, archived_through_turn}`.
    fn archive_slice(&self, memoire: &Path, args: &Value) -> Result<Value, String>;
}

use super::DaemonHost;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// The mark and the slices of a conversation, in `chats/<backend>.rangement.json`.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct EtatDuRangement {
    /// The last turn filed by the archivist, after the commit.
    #[serde(default)]
    pub archived_through_turn: Option<usize>,
    /// Each slice asked: `{from_turn, to_turn, ok, error?}`.
    #[serde(default)]
    pub slices: Vec<Value>,
    /// What went wrong last (a refused slice, a memory without the tool), until the next success.
    #[serde(default)]
    pub last_error: Option<String>,
}

pub(super) fn chemin_de_l_etat(chats_root: &Path, backend_id: &str) -> PathBuf {
    chats_root.join(format!("{backend_id}.rangement.json"))
}

pub(super) fn lire_l_etat(chemin: &Path) -> EtatDuRangement {
    std::fs::read(chemin).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_l_etat(chemin: &Path, etat: &EtatDuRangement) -> Result<(), String> {
    if let Some(p) = chemin.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let neuf = chemin.with_extension("json.neuf");
    std::fs::write(&neuf, serde_json::to_vec_pretty(etat).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&neuf, chemin).map_err(|e| e.to_string())
}

/// An instant in ms since the epoch, as ISO 8601 UTC (`2026-11-18T21:04:05Z`).
pub(super) fn iso_utc(ms: u64) -> String {
    let s = ms / 1000;
    let (jours, reste) = ((s / 86_400) as i64, s % 86_400);
    // Days to civil date (Howard Hinnant's algorithm).
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let doe = z - ere * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + ere * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", reste / 3600, reste % 3600 / 60, reste % 60)
}

/// The messages of a session, numbered: `(turn, seq, message)`. A turn starts at each user
/// message; `seq` 0 is that message, then the assistant and each tool result, in order.
fn numeroter(turns: &[crate::llm::Turn]) -> Vec<(usize, usize, &crate::llm::Turn)> {
    let (mut tour, mut seq) = (0usize, 0usize);
    let mut out = Vec::new();
    for t in turns.iter().filter(|t| t.role != "system") {
        if t.role == "user" {
            tour += 1;
            seq = 0;
        } else {
            seq += 1;
        }
        if tour > 0 {
            out.push((tour, seq, t));
        }
    }
    out
}

/// The `put_turn` line of a message. A tool result carries its tool, the arguments of its call
/// (found on the assistant message) and its status.
fn ligne_du_message(run: &str, tour: usize, seq: usize, t: &crate::llm::Turn, appels: &std::collections::HashMap<String, (String, String)>) -> Value {
    let at = t.at_ms.map(iso_utc).unwrap_or_default();
    let mut l = json!({ "run": run, "turn": tour, "seq": seq, "at": at, "role": t.role, "text": t.content });
    if t.role == "tool" {
        let id = t.tool_call_id.clone().unwrap_or_default();
        let (nom, params) = appels.get(&id).cloned().unwrap_or_else(|| (t.tool_name.clone().unwrap_or_default(), String::new()));
        let v: Value = serde_json::from_str(&t.content).unwrap_or(Value::Null);
        let status = if v["ok"] == false || !v["error"].is_null() { "error" } else { "ok" };
        l["tool_name"] = json!(nom);
        l["params"] = json!(params);
        l["result"] = json!(t.content);
        l["status"] = json!(status);
    }
    l
}

/// The host's own way to the memory: the tools `put_turn` and `archive_slice` of the memory
/// backend, called by the host like any tool.
struct RangeurDeLHote {
    hote: std::sync::Weak<DaemonHost>,
}

impl Rangeur for RangeurDeLHote {
    fn put_turn(&self, memoire: &Path, line: &Value) -> Result<Value, String> {
        let h = self.hote.upgrade().ok_or("the host is gone")?;
        let v = h.call(None, memoire, "put_turn", line.clone());
        if v["error"].is_null() { Ok(v) } else { Err(v["error"].to_string()) }
    }
    fn archive_slice(&self, memoire: &Path, args: &Value) -> Result<Value, String> {
        let h = self.hote.upgrade().ok_or("the host is gone")?;
        let v = h.call(None, memoire, "archive_slice", args.clone());
        if v["error"].is_null() { Ok(v.get("result").cloned().unwrap_or(v)) } else { Err(v["error"].to_string()) }
    }
}

impl DaemonHost {
    /// **At the end of a chat turn**: its messages go to the memory (`put_turn`); past
    /// `memory.slice_tokens` since the mark, the slice goes to the archivist in the background,
    /// one at a time per conversation. Nothing here makes the turn fail: what goes wrong is
    /// written as `last_error` in the conversation's state.
    pub(super) fn ranger_apres_le_tour(&self, hosted: &Arc<super::Hosted>, backend_id: &str, depuis: usize) {
        let Ok((reglage, bilans)) = self.with_backend(hosted, |b| (b.prepared().manifest.memory.clone().unwrap_or_default(), b.prepared().manifest.bilans.clone())) else { return };
        let Some(bilans) = bilans else { return };
        if !reglage.index_turns {
            return;
        }
        let memoire = if bilans.memoire.is_absolute() { bilans.memoire.clone() } else { hosted.manifest.parent().map(|p| p.join(&bilans.memoire)).unwrap_or(bilans.memoire.clone()) };
        let Some(rangeur) = self.rangeur.clone().or_else(|| self.soi.get().map(|w| Arc::new(RangeurDeLHote { hote: w.clone() }) as Arc<dyn Rangeur>)) else { return };
        let turns = {
            let chats = self.chats.lock().unwrap_or_else(|p| p.into_inner());
            let Some(e) = chats.get(backend_id) else { return };
            e.session.turns.clone()
        };
        let run = format!("chat:{backend_id}");
        let numerotes = numeroter(&turns);
        let appels: std::collections::HashMap<String, (String, String)> = turns.iter().flat_map(|t| t.tool_calls.iter().map(|c| (c.id.clone(), (c.name.clone(), c.arguments.clone())))).collect();
        let chemin = chemin_de_l_etat(&self.chats_root, backend_id);
        // The new messages of this turn, to the memory.
        // `depuis` counts the session's messages (system aside) before this turn.
        let premier_neuf = depuis;
        let mut erreur = None;
        for (tour, seq, t) in numerotes.iter().skip(premier_neuf) {
            if let Err(e) = rangeur.put_turn(&memoire, &ligne_du_message(&run, *tour, *seq, t, &appels)) {
                erreur = Some(format!("put_turn: {e}"));
                break;
            }
        }
        if let Some(e) = erreur {
            let mut etat = lire_l_etat(&chemin);
            etat.last_error = Some(e);
            let _ = ecrire_l_etat(&chemin, &etat);
            return;
        }
        // The slice since the mark: is it full, and is no filing in flight?
        let etat = lire_l_etat(&chemin);
        let de = etat.archived_through_turn.unwrap_or(0) + 1;
        let jusqu_a = numerotes.last().map(|(n, _, _)| *n).unwrap_or(0);
        let caracteres: usize = numerotes.iter().filter(|(n, _, _)| *n >= de).map(|(_, _, t)| t.content.chars().count() + t.tool_calls.iter().map(|c| c.arguments.chars().count()).sum::<usize>()).sum();
        if jusqu_a < de || caracteres.div_ceil(4) <= reglage.slice_tokens {
            return;
        }
        if !self.rangements_en_vol.lock().unwrap_or_else(|p| p.into_inner()).insert(backend_id.to_string()) {
            return;
        }
        let en_vol = self.rangements_en_vol.clone();
        let id = backend_id.to_string();
        let args = json!({ "run": run, "from_turn": de, "to_turn": jusqu_a, "context": "" });
        std::thread::spawn(move || {
            let reponse = rangeur.archive_slice(&memoire, &args);
            let mut etat = lire_l_etat(&chemin);
            let errors = reponse.as_ref().map(|r| r["report"]["errors"].as_array().map(|a| a.len()).unwrap_or(0)).unwrap_or(0);
            let reussi = matches!(&reponse, Ok(r) if r["report"]["ok"] != false && errors == 0 && r["archived_through_turn"] == json!(jusqu_a));
            if reussi {
                etat.archived_through_turn = Some(jusqu_a);
                etat.last_error = None;
                etat.slices.push(json!({ "from_turn": de, "to_turn": jusqu_a, "ok": true }));
            } else {
                let pourquoi = match &reponse {
                    Err(e) => e.clone(),
                    Ok(r) => format!("the archivist did not file turns {de}-{jusqu_a}: {}", r["report"]["errors"]),
                };
                etat.slices.push(json!({ "from_turn": de, "to_turn": jusqu_a, "ok": false, "error": pourquoi }));
                etat.last_error = Some(pourquoi);
            }
            let _ = ecrire_l_etat(&chemin, &etat);
            en_vol.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_instant_reads_as_iso_utc() {
        assert_eq!(super::iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::iso_utc(1_794_431_045_000), "2026-11-11T21:04:05Z");
    }
}
