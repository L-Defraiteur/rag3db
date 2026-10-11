//! **L'annuaire de la session** : qui est là, sous quel run, et qui lit quel
//! fil (lot 3 des agents qui se parlent,
//! `docs/11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md` §2.5).
//!
//! On adressait par run id, et les tests posaient nom = run id. L'annuaire
//! associe un **nom** de participant à son run en cours, et un **fil** nommé à
//! ses participants : `send` écrit à un nom, à un run, ou à tout un fil. Rien
//! de neuf en base : la trace (`Participant`, `PARTICIPATES_IN`) se pose déjà
//! à partir des messages.
//!
//! Un participant inscrit a **sa boîte ouverte** ([`Directory::enroll`]) : un
//! message qui lui arrive avant son run n'est pas perdu (un curseur ne voit
//! que ce qui suit sa création).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// Qui tient un nom : l'hôte (`0`), ou un agent, par l'identifiant que lui
/// donne [`new_owner`].
pub type Owner = u64;

/// Un identifiant d'agent neuf, pour [`Directory::claim`].
pub fn new_owner() -> Owner {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Default)]
struct Book {
    /// Nom → (run en cours, qui tient le nom).
    runs: BTreeMap<String, (String, Owner)>,
    /// Fil → noms de ses participants.
    threads: BTreeMap<String, BTreeSet<String>>,
}

/// L'annuaire, partagé par les agents d'une session.
#[derive(Default)]
pub struct Directory {
    book: Mutex<Book>,
}

/// Un destinataire résolu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    pub name: String,
    pub run: String,
}

impl Directory {
    pub fn new() -> Self {
        Self::default()
    }

    fn book(&self) -> std::sync::MutexGuard<'_, Book> {
        self.book.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// **L'hôte** inscrit `name` sous `run` : il fait autorité, l'inscription
    /// remplace ce qui y était. Un agent qui prendra ce nom l'adoptera.
    pub fn register(&self, name: &str, run: &str) {
        self.book().runs.insert(name.to_string(), (run.to_string(), 0));
    }

    /// **Un agent** prend `name` pour `run`. Il peut reprendre son propre nom
    /// sous un run neuf, ou un nom posé par l'hôte ; un nom tenu par un **autre**
    /// agent est refusé — sans quoi deux agents du même nom (le nom par
    /// défaut, typiquement) se voleraient leur boîte en silence.
    pub fn claim(&self, name: &str, run: &str, owner: Owner) -> Result<(), String> {
        let mut book = self.book();
        if let Some((held, by)) = book.runs.get(name) {
            if *by != 0 && *by != owner {
                return Err(format!(
                    "le nom « {name} » est déjà tenu par un autre agent (run {held}) : nommez cet agent autrement"
                ));
            }
        }
        book.runs.insert(name.to_string(), (run.to_string(), owner));
        Ok(())
    }

    /// Inscrit `name` sous `run` **et ouvre sa boîte** sur `bus` : ce qui lui
    /// est écrit dès maintenant sera lu à son prochain tour.
    pub fn enroll(&self, bus: &crate::events::EventBus, name: &str, run: &str) {
        bus.cursor(&crate::events::inbox_topic(run), crate::agent::AGENT_INBOX_CURSOR);
        self.register(name, run);
    }

    /// [`Self::claim`], la boîte ouverte.
    pub fn claim_and_open(&self, bus: &crate::events::EventBus, name: &str, run: &str, owner: Owner) -> Result<(), String> {
        self.claim(name, run, owner)?;
        bus.cursor(&crate::events::inbox_topic(run), crate::agent::AGENT_INBOX_CURSOR);
        Ok(())
    }

    /// Les noms inscrits, dans l'ordre.
    pub fn names(&self) -> Vec<String> {
        self.book().runs.keys().cloned().collect()
    }

    /// Le run d'un participant, désigné par son nom ou par son run.
    pub fn resolve(&self, who: &str) -> Option<Recipient> {
        let book = self.book();
        if let Some((run, _)) = book.runs.get(who) {
            return Some(Recipient { name: who.to_string(), run: run.clone() });
        }
        book.runs
            .iter()
            .find(|(_, (run, _))| run.as_str() == who)
            .map(|(name, (run, _))| Recipient { name: name.clone(), run: run.clone() })
    }

    /// `name` lit désormais le fil `conversation`.
    pub fn join(&self, conversation: &str, name: &str) {
        self.book().threads.entry(conversation.to_string()).or_default().insert(name.to_string());
    }

    /// Les participants d'un fil, dans l'ordre.
    pub fn participants(&self, conversation: &str) -> Vec<String> {
        self.book().threads.get(conversation).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    /// **À qui part un envoi** de `from` : à `to` (un nom ou un run), ou, `to`
    /// vide, à tous les participants du fil `conversation` sauf `from`. Un fil
    /// nommé gagne l'expéditeur et le destinataire nommé. Les refus disent
    /// quoi faire : un nom inconnu liste les noms connus.
    pub fn recipients(&self, from: &str, to: &str, conversation: &str) -> Result<Vec<Recipient>, String> {
        let to = to.trim();
        let conversation = conversation.trim();
        let out = if to.is_empty() {
            if conversation.is_empty() {
                return Err("ni destinataire ('to') ni fil ('conversation')".to_string());
            }
            let others: Vec<Recipient> = self
                .participants(conversation)
                .into_iter()
                .filter(|n| n != from)
                .filter_map(|n| self.resolve(&n))
                .collect();
            if others.is_empty() {
                return Err(format!("personne d'autre ne lit le fil « {conversation} »"));
            }
            others
        } else {
            let Some(r) = self.resolve(to) else {
                return Err(format!("« {to} » n'est pas dans l'annuaire ; inscrits : {}", self.names().join(", ")));
            };
            if r.name == from {
                return Err("on ne s'écrit pas à soi-même".to_string());
            }
            vec![r]
        };
        if !conversation.is_empty() {
            self.join(conversation, from);
            for r in &out {
                self.join(conversation, &r.name);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
