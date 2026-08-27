//! **Les postures d'une session** : qui s'est tu, envers qui, et pourquoi.
//!
//! Voir [doc 12](../docs/26-aout-2026-20h29/12-conversations-a-plusieurs.md).
//!
//! Une posture est de l'**état d'interaction**, pas de la connaissance : elle
//! vit le temps d'une session et ne va pas au catalogue. Ce qui va au
//! catalogue, ce sont les **événements** — la pause a été prononcée, elle est
//! tracée, elle est rejouable.
//!
//! ## Ce que ce module existe pour attraper
//!
//! Une seule chose, et c'est la seule situation qui ne peut pas se résoudre
//! toute seule :
//!
//! > A s'est tu **en attendant B** ; B s'est tu **en attendant A**.
//!
//! Personne ne parle, personne n'est en faute, et rien ne se passe. Un
//! plafond de tours l'aurait cassé au hasard — c'est ce qu'on avait proposé
//! d'abord, et c'était traiter le symptôme. Ici on détecte **exactement**, en
//! regardant un graphe : c'est notre métier.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use crate::agent::PauseKind;

/// Ce qu'un participant a décidé, et pourquoi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posture {
    /// Envers qui. Vide : envers le fil entier.
    pub with: String,
    pub kind: PauseKind,
    pub reason: String,
}

/// Les postures en cours, par participant.
///
/// **Un participant a au plus une posture** : se taire est un état, pas une
/// pile. Parler la lève.
#[derive(Debug, Default)]
pub struct Postures {
    inner: Mutex<BTreeMap<String, Posture>>,
}

impl Postures {
    pub fn new() -> Self {
        Self::default()
    }

    /// Il s'est tu.
    pub fn record(&self, who: &str, posture: Posture) {
        if let Ok(mut g) = self.inner.lock() {
            g.insert(who.to_string(), posture);
        }
    }

    /// Il a reparlé — **on ne peut pas être en pause et parler en même
    /// temps**, c'est ce qui permet à un pair de réengager sans cérémonie
    /// (doc 12 §2.1).
    pub fn speak(&self, who: &str) -> bool {
        self.inner.lock().map(|mut g| g.remove(who).is_some()).unwrap_or(false)
    }

    pub fn get(&self, who: &str) -> Option<Posture> {
        self.inner.lock().ok()?.get(who).cloned()
    }

    /// Tout le monde, dans l'ordre.
    pub fn all(&self) -> Vec<(String, Posture)> {
        self.inner
            .lock()
            .map(|g| g.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    /// **Qui attend de vous** — la matière du bloc d'attentes (doc 12 §9).
    ///
    /// Chacun ne voit que ce qui attend **de lui** : montrer à quelqu'un que
    /// A attend B ne fait que l'encombrer.
    pub fn awaiting(&self, who: &str) -> Vec<(String, Posture)> {
        self.all()
            .into_iter()
            .filter(|(_, p)| p.kind.awaited() == Some(who))
            .collect()
    }

    /// **Les blocages** : les cycles du graphe « qui attend qui ».
    ///
    /// Un participant a au plus une posture, donc au plus une arête sortante :
    /// le graphe est *fonctionnel*, et un cycle se trouve en suivant la
    /// chaîne. Pas besoin de Tarjan pour ça — et surtout, aucun jugement :
    /// c'est de la structure, pas de l'interprétation.
    ///
    /// Chaque cycle est rendu **une seule fois**, dans un ordre stable.
    pub fn deadlocks(&self) -> Vec<Vec<String>> {
        let edges: BTreeMap<String, String> = self
            .all()
            .into_iter()
            .filter_map(|(who, p)| p.kind.awaited().filter(|_| p.kind.waits_on_someone()).map(|a| (who, a.to_string())))
            .collect();

        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut out: Vec<Vec<String>> = Vec::new();
        for start in edges.keys() {
            if seen.contains(start) {
                continue;
            }
            // On suit la chaîne en notant l'ordre de visite ; si on retombe
            // sur un nœud de *ce* parcours, la boucle commence là.
            let mut path: Vec<String> = Vec::new();
            let mut at = start.clone();
            loop {
                if let Some(i) = path.iter().position(|n| *n == at) {
                    let mut cycle = path[i..].to_vec();
                    // Ordre stable : on démarre au plus petit nom, pour que
                    // le même blocage soit toujours écrit pareil.
                    if let Some(k) = cycle.iter().enumerate().min_by_key(|(_, n)| n.as_str()).map(|(k, _)| k) {
                        cycle.rotate_left(k);
                    }
                    out.push(cycle);
                    break;
                }
                if seen.contains(&at) {
                    break; // déjà exploré par un parcours précédent
                }
                path.push(at.clone());
                match edges.get(&at) {
                    Some(next) => at = next.clone(),
                    None => break, // il attend quelqu'un qui, lui, ne se tait pas
                }
            }
            seen.extend(path);
        }
        out
    }

    /// Une phrase pour le rendu : ce qui attend de vous, et les blocages.
    /// **Vide quand il n'y a rien** — un bloc toujours présent apprend au
    /// modèle à ne plus le lire (doc 12 §9.4).
    pub fn describe_for(&self, who: &str) -> String {
        let mut lines: Vec<String> = self
            .awaiting(who)
            .into_iter()
            .map(|(qui, p)| format!("  · {qui} — {} , « {} »", kind_label(&p.kind), p.reason))
            .collect();
        if !lines.is_empty() {
            lines.insert(0, "en attente de vous :".to_string());
        }
        for cycle in self.deadlocks() {
            if cycle.iter().any(|n| n == who) || cycle.len() > 1 {
                lines.push(format!("blocage : {} s'attendent mutuellement", cycle.join(" → ")));
            }
        }
        lines.join("\n")
    }
}

fn kind_label(kind: &PauseKind) -> &'static str {
    match kind {
        PauseKind::Finished => "a fini",
        PauseKind::WaitingForRun(_) => "attend un résultat",
        PauseKind::WaitingForPeer(_) => "vous attend",
        PauseKind::WaitingForInstruction => "attend une consigne",
        PauseKind::Blocked => "est bloqué",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiting_for(who: &str) -> Posture {
        Posture { with: who.into(), kind: PauseKind::WaitingForPeer(who.into()), reason: format!("j'attends {who}") }
    }

    /// **Le seul cas qui ne peut pas se résoudre seul** : A attend B qui
    /// attend A.
    #[test]
    fn two_agents_waiting_on_each_other_is_a_deadlock() {
        let p = Postures::new();
        p.record("a", waiting_for("b"));
        p.record("b", waiting_for("a"));

        let cycles = p.deadlocks();
        eprintln!("[blocages] {cycles:?}");
        assert_eq!(cycles, vec![vec!["a".to_string(), "b".to_string()]]);
        // Et c'est dit, plutôt que deviné : un blocage annoncé est un
        // problème, un blocage silencieux est une panne.
        assert!(p.describe_for("a").contains("blocage"), "{}", p.describe_for("a"));
    }

    /// Une chaîne n'est pas un cycle. C, qui ne se tait pas, débloquera tout.
    #[test]
    fn a_chain_that_ends_somewhere_is_not_a_deadlock() {
        let p = Postures::new();
        p.record("a", waiting_for("b"));
        p.record("b", waiting_for("c")); // c ne s'est pas tu
        assert!(p.deadlocks().is_empty(), "{:?}", p.deadlocks());
    }

    /// **Attendre une consigne n'est pas attendre quelqu'un.** Sans cette
    /// distinction, deux agents qui attendent tous deux l'humain
    /// ressembleraient à un blocage — de faux blocages tous les quarts
    /// d'heure.
    #[test]
    fn waiting_for_an_instruction_never_deadlocks() {
        let p = Postures::new();
        let waiting = |r: &str| Posture { with: String::new(), kind: PauseKind::WaitingForInstruction, reason: r.into() };
        p.record("a", waiting("j'attends Lucie"));
        p.record("b", waiting("moi aussi"));
        assert!(p.deadlocks().is_empty());

        // Et « fini » non plus, évidemment.
        p.record("c", Posture { with: "d".into(), kind: PauseKind::Finished, reason: "tout dit".into() });
        assert!(p.deadlocks().is_empty());
    }

    /// Parler lève sa propre pause — c'est ce qui permet de réengager sans
    /// cérémonie, et ce qui défait un blocage.
    #[test]
    fn speaking_lifts_ones_own_pause_and_breaks_the_cycle() {
        let p = Postures::new();
        p.record("a", waiting_for("b"));
        p.record("b", waiting_for("a"));
        assert_eq!(p.deadlocks().len(), 1);

        assert!(p.speak("b"), "b avait bien une posture");
        assert!(p.deadlocks().is_empty(), "un seul qui reparle suffit");
        assert!(!p.speak("b"), "et parler deux fois ne lève rien de plus");
    }

    /// Chacun ne voit que ce qui attend **de lui**.
    #[test]
    fn what_awaits_you_is_addressed_to_you() {
        let p = Postures::new();
        p.record("a", waiting_for("lucie"));
        p.record("b", waiting_for("c"));

        let vu = p.describe_for("lucie");
        eprintln!("[pour lucie]\n{vu}");
        assert!(vu.contains("en attente de vous") && vu.contains('a'), "{vu}");
        assert!(!vu.contains("· b"), "ce que b attend de c ne la regarde pas : {vu}");

        // Rien à dire : rien du tout. Un bloc toujours présent, même vide,
        // apprend au modèle à ne plus le lire.
        assert!(p.describe_for("personne").is_empty());
    }

    /// Un blocage à trois se trouve aussi, et s'écrit toujours pareil.
    #[test]
    fn a_three_way_deadlock_is_written_the_same_way_every_time() {
        let p = Postures::new();
        p.record("b", waiting_for("c"));
        p.record("c", waiting_for("a"));
        p.record("a", waiting_for("b"));
        assert_eq!(p.deadlocks(), vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]]);
    }
}
