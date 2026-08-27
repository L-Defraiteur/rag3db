//! **Le compteur** : ce qui a été consommé, en unités qui ne sont pas toutes
//! des jetons.
//!
//! Voir [doc 08](../docs/27-aout-2026-13h01/08-le-compteur.md).
//!
//! ## Quatre décisions, et elles tiennent tout le module
//!
//! **1. L'unité n'est pas le jeton.** Un compteur qui ne connaîtrait que les
//! jetons ne pourrait pas mesurer une synthèse vocale, facturée au caractère
//! ou à la seconde d'audio. La primitive est donc `(ressource, unité,
//! quantité)` — la même forme pour un LLM distant, un LLM local, un TTS et un
//! STT.
//!
//! **2. On enregistre des faits, jamais un prix.** Les tarifs changent ; un
//! prix rangé à côté d'un appel est un verdict qui survit à ses raisons
//! ([doc 05](../docs/27-aout-2026-13h01/05-la-reputation-des-abstractions.md)
//! §2.1). La tarification est une table remplaçable, appliquée au moment de
//! lire.
//!
//! **3. Le slug de ressource est le joint vers plus tard.** Le crédit — solde
//! = dotation − consommation — ne veut rien dire aujourd'hui et doit de toute
//! façon être autoritatif, donc ailleurs. Mais le slug, lui, coûte trois fois
//! rien maintenant et rend le reste possible : le jour où quelqu'un vend
//! quelque chose, il n'a qu'une table à écrire. **Ce qu'on ne peut pas
//! rattraper entre maintenant** ; le reste attendra.
//!
//! **4. Un compteur local est une mesure, jamais une autorité.** La
//! facturation du fournisseur est la vérité. Ce module dit ce qu'on a
//! demandé et ce que le fournisseur a rapporté — pas ce qu'on doit.
//!
//! ## Ce qu'il ne fait pas
//!
//! Il ne remonte pas l'arbre des runs. Un run parent « coûte » ce qu'il a
//! consommé plus ses enfants, et cet arbre vit dans le graphe (`CHILD_OF`) :
//! le total d'une branche est un parcours, c'est-à-dire notre métier, pas
//! celui d'un compteur en mémoire.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// En quoi se compte une consommation.
///
/// Volontairement petit et fermé : une unité de plus est un choix de produit,
/// pas une commodité. Ce qui varie d'un fournisseur à l'autre est le **prix**
/// d'une unité, pas la liste des unités.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unit {
    /// Jetons d'entrée facturés plein tarif.
    InputTokens,
    /// Jetons d'entrée **servis depuis le cache** du fournisseur. Séparés
    /// parce qu'ils coûtent environ dix fois moins : les confondre fausse le
    /// total d'un ordre de grandeur, et dans le sens qui flatte.
    CachedInputTokens,
    /// Jetons générés.
    OutputTokens,
    /// Secondes d'audio — STT en entrée, TTS en sortie.
    AudioSeconds,
    /// Caractères — certains TTS facturent ainsi.
    Characters,
    Images,
    /// L'appel lui-même, quand c'est lui qu'on compte.
    Requests,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputTokens => "input_tokens",
            Self::CachedInputTokens => "cached_input_tokens",
            Self::OutputTokens => "output_tokens",
            Self::AudioSeconds => "audio_seconds",
            Self::Characters => "characters",
            Self::Images => "images",
            Self::Requests => "requests",
        }
    }
}

/// Ce qu'un appel a consommé.
///
/// Un seul appel porte souvent plusieurs unités — entrée, entrée en cache,
/// sortie — d'où `units` plutôt qu'un enregistrement par unité : **un appel,
/// une ligne**, sinon les totaux se recomposent mal et l'attribution se
/// dédouble.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consumption {
    /// Slug stable et paramétrable : `llm.gemini-3.5-flash`,
    /// `tts.piper.fr`, `stt.whisper-large`. C'est lui qu'une table de prix
    /// résoudra un jour.
    pub resource: String,
    /// Qui a servi : `vertex`, `openai`, `local`… Un même modèle chez deux
    /// fournisseurs n'a ni le même prix ni la même disponibilité.
    pub provider: String,
    /// Le run qui a consommé. L'attribution est la partie qu'on ne peut pas
    /// rattraper après coup.
    pub run: String,
    pub agent: String,
    pub units: Vec<(Unit, u64)>,
}

impl Consumption {
    pub fn new(resource: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            provider: provider.into(),
            run: String::new(),
            agent: String::new(),
            units: Vec::new(),
        }
    }

    pub fn by(mut self, run: impl Into<String>, agent: impl Into<String>) -> Self {
        self.run = run.into();
        self.agent = agent.into();
        self
    }

    /// Ajoute une quantité. **Zéro n'est pas enregistré** : une ligne à zéro
    /// encombre un relevé sans rien y apprendre.
    pub fn with(mut self, unit: Unit, amount: u64) -> Self {
        if amount > 0 {
            self.units.push((unit, amount));
        }
        self
    }

    pub fn amount(&self, unit: Unit) -> u64 {
        self.units.iter().find(|(u, _)| *u == unit).map(|(_, n)| *n).unwrap_or(0)
    }

    /// Rien de mesurable : ni à publier, ni à enregistrer.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }
}

/// Ce qui a été consommé pendant cette session.
///
/// Partagé par `Arc` et mutable derrière un `Mutex`, comme
/// [`crate::postures::Postures`] et [`crate::session::Session`] : plusieurs
/// mains y écrivent — la boucle d'agent, un TTS, un STT.
#[derive(Debug, Default)]
pub struct Meter {
    inner: Mutex<Vec<Consumption>>,
}

impl Meter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, c: Consumption) {
        if c.is_empty() {
            return;
        }
        if let Ok(mut g) = self.inner.lock() {
            g.push(c);
        }
    }

    /// Tout, dans l'ordre où c'est arrivé.
    pub fn all(&self) -> Vec<Consumption> {
        self.inner.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Les totaux par `(ressource, unité)`. Triés : un relevé qui change
    /// d'ordre d'une fois sur l'autre ne se compare pas.
    pub fn totals(&self) -> BTreeMap<(String, Unit), u64> {
        let mut out: BTreeMap<(String, Unit), u64> = BTreeMap::new();
        for c in self.all() {
            for (u, n) in &c.units {
                *out.entry((c.resource.clone(), *u)).or_insert(0) += n;
            }
        }
        out
    }

    /// Le total d'une unité, toutes ressources confondues.
    pub fn total(&self, unit: Unit) -> u64 {
        self.all().iter().map(|c| c.amount(unit)).sum()
    }

    /// Le même relevé, borné à un run. **Ce run seul** — pas ses enfants :
    /// l'arbre vit dans le graphe, et l'y chercher ici serait deviner.
    pub fn totals_for_run(&self, run: &str) -> BTreeMap<(String, Unit), u64> {
        let mut out: BTreeMap<(String, Unit), u64> = BTreeMap::new();
        for c in self.all().iter().filter(|c| c.run == run) {
            for (u, n) in &c.units {
                *out.entry((c.resource.clone(), *u)).or_insert(0) += n;
            }
        }
        out
    }

    /// Un relevé lisible. **Vide quand rien n'a été consommé** — un tableau
    /// toujours présent, même vide, on cesse de le lire.
    pub fn describe(&self) -> String {
        let totals = self.totals();
        if totals.is_empty() {
            return String::new();
        }
        let mut par_ressource: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for ((res, unit), n) in totals {
            par_ressource.entry(res).or_default().push(format!("{n} {}", unit.as_str()));
        }
        par_ressource
            .into_iter()
            .map(|(res, parts)| format!("{res} : {}", parts.join(", ")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appel(res: &str, entree: u64, cache: u64, sortie: u64) -> Consumption {
        Consumption::new(res, "vertex")
            .by("run-1", "chercheur")
            .with(Unit::InputTokens, entree)
            .with(Unit::CachedInputTokens, cache)
            .with(Unit::OutputTokens, sortie)
    }

    #[test]
    fn un_appel_une_ligne() {
        // Plusieurs unités par appel : sinon les totaux se recomposent mal et
        // l'attribution se dédouble.
        let m = Meter::new();
        m.record(appel("llm.gemini-3.5-flash", 1200, 8000, 300));
        assert_eq!(m.all().len(), 1);
        assert_eq!(m.total(Unit::InputTokens), 1200);
        assert_eq!(m.total(Unit::CachedInputTokens), 8000);
    }

    #[test]
    fn zero_ne_s_enregistre_pas() {
        let c = appel("llm.local", 0, 0, 40);
        assert_eq!(c.units.len(), 1, "{:?}", c.units);
        // Et un appel qui n'a rien mesuré n'entre pas du tout.
        let m = Meter::new();
        m.record(Consumption::new("tts.piper.fr", "local"));
        assert!(m.all().is_empty());
    }

    /// L'entrée en cache coûte environ dix fois moins : les confondre fausse
    /// le total d'un ordre de grandeur, dans le sens qui flatte.
    #[test]
    fn le_cache_est_une_unite_a_part() {
        let m = Meter::new();
        m.record(appel("llm.gemini-3.5-flash", 1200, 8000, 300));
        let t = m.totals();
        assert_eq!(t[&("llm.gemini-3.5-flash".to_string(), Unit::InputTokens)], 1200);
        assert_eq!(t[&("llm.gemini-3.5-flash".to_string(), Unit::CachedInputTokens)], 8000);
        assert_ne!(Unit::InputTokens, Unit::CachedInputTokens);
    }

    /// La raison d'être de l'unité générique : un TTS ne se compte pas en
    /// jetons, et le compteur ne doit pas avoir à le savoir.
    #[test]
    fn un_tts_et_un_stt_entrent_dans_le_meme_compteur() {
        let m = Meter::new();
        m.record(
            Consumption::new("tts.piper.fr", "local")
                .by("run-1", "voix")
                .with(Unit::Characters, 1_840),
        );
        m.record(
            Consumption::new("stt.whisper-large", "local")
                .by("run-1", "oreille")
                .with(Unit::AudioSeconds, 73),
        );
        m.record(appel("llm.gemini-3.5-flash", 1200, 8000, 300));
        let releve = m.describe();
        assert!(releve.contains("tts.piper.fr : 1840 characters"), "{releve}");
        assert!(releve.contains("stt.whisper-large : 73 audio_seconds"), "{releve}");
        assert!(releve.contains("input_tokens"), "{releve}");
    }

    #[test]
    fn un_releve_vide_ne_dit_rien() {
        // Un tableau toujours présent, même vide, on cesse de le lire.
        assert_eq!(Meter::new().describe(), "");
    }

    #[test]
    fn l_attribution_borne_le_releve_a_un_run() {
        let m = Meter::new();
        m.record(appel("llm.a", 100, 0, 10));
        m.record(
            Consumption::new("llm.a", "vertex")
                .by("run-2", "autre")
                .with(Unit::InputTokens, 900),
        );
        let t = m.totals_for_run("run-1");
        assert_eq!(t[&("llm.a".to_string(), Unit::InputTokens)], 100, "run-2 n'y est pas");
        assert_eq!(m.total(Unit::InputTokens), 1000, "et le total les voit tous les deux");
    }

    #[test]
    fn le_releve_est_stable() {
        // Un relevé qui change d'ordre d'une fois sur l'autre ne se compare pas.
        let m = Meter::new();
        m.record(appel("llm.z", 1, 0, 1));
        m.record(appel("llm.a", 1, 0, 1));
        assert!(m.describe().starts_with("llm.a"), "{}", m.describe());
    }
}
