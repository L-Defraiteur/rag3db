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
    /// `compose_restart` on the memory `memoire`: `{sources, identity, task_fiche,
    /// budget_tokens}`; returns `{text, tokens, …}`.
    fn compose_restart(&self, _memoire: &Path, _args: &Value) -> Result<Value, String> {
        Err("compose_restart: not offered by this memory".into())
    }
    /// The L1 that cite a slice's Source (`cited_by` on the memory): a list of apports.
    fn cited_by(&self, _memoire: &Path, _source: &str) -> Result<Value, String> {
        Err("cited_by: not offered by this memory".into())
    }
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
    /// The Sources of the slices filed without error, in order (`compose_restart` takes them).
    #[serde(default)]
    pub sources: Vec<String>,
    /// The turns a compaction took out of the session: the session's turn n is the run's turn
    /// `turn_offset + n`.
    #[serde(default)]
    pub turn_offset: usize,
    /// The last message written by `put_turn`, `(turn, seq)`: what follows goes at the next end of
    /// turn, a refused one included.
    #[serde(default)]
    pub written_through: Option<(usize, usize)>,
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
/// A message that a compaction put in the session (the restart context and its answer): not a
/// turn of the run.
pub(super) fn de_reprise(t: &crate::llm::Turn) -> bool {
    t.metadata.as_ref().is_some_and(|m| m["compaction"] == true)
}

pub(super) fn numeroter(turns: &[crate::llm::Turn], offset: usize) -> Vec<(usize, usize, &crate::llm::Turn)> {
    let (mut tour, mut seq) = (offset, 0usize);
    let mut out = Vec::new();
    for t in turns.iter().filter(|t| t.role != "system" && !de_reprise(t)) {
        if t.role == "user" {
            tour += 1;
            seq = 0;
        } else {
            seq += 1;
        }
        if tour > offset {
            out.push((tour, seq, t));
        }
    }
    out
}

/// The `put_turn` line of a message. A tool result carries its tool, the arguments of its call
/// (found on the assistant message) and its status.
pub(super) fn ligne_du_message(run: &str, tour: usize, seq: usize, t: &crate::llm::Turn, appels: &std::collections::HashMap<String, (String, String)>) -> Value {
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
    fn compose_restart(&self, memoire: &Path, args: &Value) -> Result<Value, String> {
        let h = self.hote.upgrade().ok_or("the host is gone")?;
        let v = h.call(None, memoire, "compose_restart", args.clone());
        if v["error"].is_null() { Ok(v.get("result").cloned().unwrap_or(v)) } else { Err(v["error"].to_string()) }
    }
    /// `cited_by {value: "doc:<source>", format: "json"}` returns fiches, each with its apports
    /// (form given by mémoire, 19 Nov 2026); flattened here to `{apport, fiche, section}`, the
    /// identity of an apport being (fiche, source, section).
    fn cited_by(&self, memoire: &Path, source: &str) -> Result<Value, String> {
        let h = self.hote.upgrade().ok_or("the host is gone")?;
        let v = h.call(None, memoire, "cited_by", serde_json::json!({ "value": format!("doc:{source}"), "format": "json", "limit": 200 }));
        if !v["error"].is_null() {
            return Err(v["error"].to_string());
        }
        let fiches: Value = match &v["result"] {
            Value::String(t) => serde_json::from_str(t).map_err(|e| format!("cited_by: unreadable result: {e}"))?,
            autre => autre.clone(),
        };
        let mut out = Vec::new();
        for f in fiches.as_array().cloned().unwrap_or_default() {
            let d = if f["data"].is_null() { f.clone() } else { f["data"].clone() };
            let nom = d["name"].as_str().unwrap_or_default().to_string();
            for a in d["apports"].as_array().cloned().unwrap_or_default() {
                let (src, section) = (a["source"].as_str().unwrap_or_default(), a["section"].as_str().unwrap_or_default());
                if !src.contains(source) {
                    continue;
                }
                out.push(serde_json::json!({ "apport": format!("{nom}|{src}|{section}"), "fiche": nom, "section": section }));
            }
        }
        Ok(Value::Array(out))
    }
}

/// What the archiving of a conversation needs: its settings, its memory, its way there, the file
/// of its mark.
pub(super) struct Rangement {
    pub reglage: crate::backend::SessionArchive,
    pub memoire: PathBuf,
    pub rangeur: Arc<dyn Rangeur>,
    pub chemin: PathBuf,
    pub verrou: Arc<std::sync::Mutex<()>>,
}

/// Whether a slice report says « filed »: the mark reaches `to_turn` and there is no error (the
/// report is flat; `report.*` is read too, the form of the first witnesses).
fn tranche_rangee(r: &Value, to: usize) -> bool {
    let errors = r["errors"].as_array().or(r["report"]["errors"].as_array()).map(|a| a.len()).unwrap_or(0);
    r["ok"] != false && r["report"]["ok"] != false && errors == 0 && r["archived_through_turn"] == json!(to)
}

impl Rangement {
    /// Read, change and write the state of the mark, one writer at a time.
    pub fn changer(&self, f: impl FnOnce(&mut EtatDuRangement)) -> EtatDuRangement {
        let _g = self.verrou.lock().unwrap_or_else(|p| p.into_inner());
        let mut etat = lire_l_etat(&self.chemin);
        f(&mut etat);
        let _ = ecrire_l_etat(&self.chemin, &etat);
        etat
    }

    pub fn etat(&self) -> EtatDuRangement {
        let _g = self.verrou.lock().unwrap_or_else(|p| p.into_inner());
        lire_l_etat(&self.chemin)
    }

    /// Every message after `written_through` goes to the memory, in order; the first refusal
    /// stops there and is said, and the next call starts from it again.
    pub fn envoyer_les_tours(&self, run: &str, turns: &[crate::llm::Turn]) -> Result<(), String> {
        let etat = self.etat();
        let numerotes = numeroter(turns, etat.turn_offset);
        let appels: std::collections::HashMap<String, (String, String)> = turns.iter().flat_map(|t| t.tool_calls.iter().map(|c| (c.id.clone(), (c.name.clone(), c.arguments.clone())))).collect();
        for (tour, seq, t) in numerotes {
            if etat.written_through.is_some_and(|w| (tour, seq) <= w) {
                continue;
            }
            if let Err(e) = self.rangeur.put_turn(&self.memoire, &ligne_du_message(run, tour, seq, t, &appels)) {
                let e = format!("put_turn {tour}.{seq}: {e}");
                self.changer(|s| s.last_error = Some(e.clone()));
                return Err(e);
            }
            self.changer(|s| s.written_through = Some((tour, seq)));
        }
        Ok(())
    }

    /// The open slice, from the mark to the last turn of `turns`: `(from, to, tokens)`.
    pub fn tranche_ouverte(&self, turns: &[crate::llm::Turn]) -> (usize, usize, usize) {
        let etat = self.etat();
        let numerotes = numeroter(turns, etat.turn_offset);
        let de = etat.archived_through_turn.unwrap_or(0) + 1;
        let jusqu_a = numerotes.last().map(|(n, _, _)| *n).unwrap_or(etat.turn_offset);
        let caracteres: usize = numerotes.iter().filter(|(n, _, _)| *n >= de).map(|(_, _, t)| t.content.chars().count() + t.tool_calls.iter().map(|c| c.arguments.chars().count()).sum::<usize>()).sum();
        (de, jusqu_a, caracteres.div_ceil(4))
    }

    /// The slice `de..=jusqu_a` to the archivist, waiting; the mark, the Source and the error
    /// written from the answer. `Ok` only when the slice is filed.
    pub fn ranger(&self, run: &str, de: usize, jusqu_a: usize) -> Result<(), String> {
        let reponse = self.rangeur.archive_slice(&self.memoire, &json!({ "run": run, "from_turn": de, "to_turn": jusqu_a, "context": "" }));
        let mut resultat = Ok(());
        self.changer(|etat| match &reponse {
            Ok(r) if tranche_rangee(r, jusqu_a) => {
                etat.archived_through_turn = Some(jusqu_a);
                etat.last_error = None;
                if let Some(source) = r["source"].as_str() {
                    etat.sources.push(source.to_string());
                }
                etat.slices.push(json!({ "from_turn": de, "to_turn": jusqu_a, "ok": true, "source": r["source"] }));
            }
            _ => {
                let pourquoi = match &reponse {
                    Err(e) => e.clone(),
                    Ok(r) => format!("the archivist did not file turns {de}-{jusqu_a}: {}", if r["errors"].is_null() { r["report"]["errors"].clone() } else { r["errors"].clone() }),
                };
                etat.slices.push(json!({ "from_turn": de, "to_turn": jusqu_a, "ok": false, "error": pourquoi }));
                etat.last_error = Some(pourquoi.clone());
                resultat = Err(pourquoi);
            }
        });
        resultat
    }
}

impl DaemonHost {
    /// The backend of a run: `chat:<backend>`, or `<task>:<parent>` for a teammate's task.
    pub(super) fn backend_du_run(&self, run: &str) -> Result<String, String> {
        if let Some(id) = run.strip_prefix("chat:") {
            return Ok(id.to_string());
        }
        let (tache, parent) = run.split_once(':').ok_or_else(|| format!("{run}: not a run (chat:<backend> or <task>:<parent>)"))?;
        self.tache(parent, tache).map(|t| t.id).ok_or_else(|| format!("{run}: no task {tache} on {parent}"))
    }

    /// The archiving of a conversation, if its backend declares a memory and indexes its turns.
    pub(super) fn rangement_de(&self, hosted: &Arc<super::Hosted>, backend_id: &str) -> Option<Rangement> {
        let (reglage, bilans) = self.with_backend(hosted, |b| (b.prepared().manifest.memory.clone().unwrap_or_default(), b.prepared().manifest.bilans.clone())).ok()?;
        let bilans = bilans?;
        if !reglage.index_turns {
            return None;
        }
        let memoire = if bilans.memoire.is_absolute() { bilans.memoire.clone() } else { hosted.manifest.parent().map(|p| p.join(&bilans.memoire)).unwrap_or(bilans.memoire.clone()) };
        let rangeur = self.rangeur.clone().or_else(|| self.soi.get().map(|w| Arc::new(RangeurDeLHote { hote: w.clone() }) as Arc<dyn Rangeur>))?;
        Some(Rangement { reglage, memoire, rangeur, chemin: chemin_de_l_etat(&self.chats_root, backend_id), verrou: self.rangement_verrou.clone() })
    }

    /// **At the end of a turn**: the messages not yet written go to the memory; past
    /// `memory.slice_tokens` since the mark, the slice goes to the archivist in the background,
    /// one at a time per conversation. Nothing here makes the turn fail.
    pub(super) fn ranger_apres_le_tour(&self, hosted: &Arc<super::Hosted>, backend_id: &str, run: &str, turns: Vec<crate::llm::Turn>) {
        let Some(r) = self.rangement_de(hosted, backend_id) else { return };
        if r.envoyer_les_tours(run, &turns).is_err() {
            return;
        }
        let (de, jusqu_a, jetons) = r.tranche_ouverte(&turns);
        if jusqu_a < de || jetons <= r.reglage.slice_tokens {
            return;
        }
        if !self.rangements_en_vol.lock().unwrap_or_else(|p| p.into_inner()).insert(backend_id.to_string()) {
            return;
        }
        let en_vol = self.rangements_en_vol.clone();
        let (id, run) = (backend_id.to_string(), run.to_string());
        std::thread::spawn(move || {
            let _ = r.ranger(&run, de, jusqu_a);
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
