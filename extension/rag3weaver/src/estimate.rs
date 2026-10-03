//! **Dire ce que ça va coûter avant d'indexer** — le verbe `estimate`.
//!
//! Première brique de l'entrée « indexer ce dépôt »
//! (`docs/3-octobre-2026-14h26/03-indexer-ce-depot.md`). Une lecture : elle
//! compte, choisit le modèle, prévoit une durée, et dit si une confirmation
//! sera demandée. Elle n'écrit rien.
//!
//! # Générique
//!
//! Rien ici ne sait ce qu'est du code. Une source est une liste de
//! `(chemin, taille)` ; ce qu'on en retient est décidé par une politique
//! passée en paramètre ([`Kept`]). La politique des dépôts de code
//! (`code::verdict`) en est une, [`code_policy`] l'adapte ; un dossier de
//! documents en passera une autre.
//!
//! # La durée se mesure, elle ne se tabule pas
//!
//! Pas de table de débits par modèle ni par carte : elle serait fausse au
//! premier poste différent. [`probe_rate`] embarque quelques morceaux par
//! l'embarqueur réellement attaché et rend un débit ; l'appelant le garde en
//! base et le relit (`Catalog::note_embedding_rate`). Sans débit connu,
//! l'estimation rend ses comptes et dit qu'elle ne sait pas la durée.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::burst::BurstSettings;
use crate::embedder::Embedder;
use crate::embedding_choice::{CardClass, Choice};

/// Au-delà de cette durée prévue de vecteurs, `index` demandera une
/// confirmation. **Provisoire** : proposé à Lucie le 3 octobre 2026, pas
/// encore tranché. En durée et pas en fichiers : un nombre de fichiers ne dit
/// rien d'un poste à l'autre.
pub const CONFIRM_ABOVE: Duration = Duration::from_secs(5 * 60);

/// Ce que la politique fait d'un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    /// Retenu, sous un genre que la politique nomme (`code`, `texte`…).
    Yes(&'static str),
    /// Écarté, avec la raison — jamais en silence.
    No(&'static str),
}

/// La politique des dépôts de code, telle que l'ingestion l'applique.
#[cfg(feature = "code")]
pub fn code_policy(path: &str, bytes: u64) -> Kept {
    match crate::code::verdict(path, bytes as usize) {
        crate::code::Verdict::Code => Kept::Yes("code"),
        crate::code::Verdict::Texte => Kept::Yes("texte"),
        crate::code::Verdict::Ecarte(reason) => Kept::No(reason),
    }
}

/// Ce qu'une source contient, vu par une politique.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Survey {
    /// Fichiers retenus.
    pub files: usize,
    /// Leurs octets — l'ordre de grandeur des caractères à embarquer.
    pub bytes: u64,
    /// Retenus, par genre : fichiers et octets.
    pub kept: BTreeMap<String, (usize, u64)>,
    /// Écartés, par raison : fichiers et octets.
    pub skipped: BTreeMap<String, (usize, u64)>,
}

impl Survey {
    pub fn skipped_files(&self) -> usize {
        self.skipped.values().map(|(n, _)| n).sum()
    }

    /// Ajoute ce que la source a écarté d'elle-même, avant la politique.
    pub fn with_exclusions<'a>(mut self, excluded: impl IntoIterator<Item = &'a (String, &'static str)>) -> Self {
        for (_, reason) in excluded {
            self.skipped.entry(reason.to_string()).or_insert((0, 0)).0 += 1;
        }
        self
    }
}

/// Compte une source. `files` : `(chemin, taille en octets)`.
pub fn survey<'a>(files: impl IntoIterator<Item = (&'a str, u64)>, policy: impl Fn(&str, u64) -> Kept) -> Survey {
    let mut s = Survey::default();
    for (path, bytes) in files {
        let (bucket, label) = match policy(path, bytes) {
            Kept::Yes(kind) => {
                s.files += 1;
                s.bytes += bytes;
                (&mut s.kept, kind)
            }
            Kept::No(reason) => (&mut s.skipped, reason),
        };
        let entry = bucket.entry(label.to_string()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += bytes;
    }
    s
}

/// Les chemins et les tailles d'un dossier, **sans lire les fichiers**, et ce
/// que la source a écarté avant toute politique — secrets probables, avec la
/// raison (`WorkingTree::list_with_exclusions` : la liste unique du crate).
/// Les fichiers ignorés par les règles du dossier n'y sont pas : ce n'est pas
/// le dépôt.
#[cfg(feature = "code")]
pub fn working_tree_files(tree: &crate::code_tools::WorkingTree) -> Result<(Vec<(String, u64)>, Vec<(String, &'static str)>), String> {
    let (paths, excluded) = tree.list_with_exclusions()?;
    let files = paths
        .into_iter()
        .map(|path| {
            let bytes = std::fs::metadata(tree.root().join(&path)).map(|m| m.len()).unwrap_or(0);
            (path, bytes)
        })
        .collect();
    Ok((files, excluded))
}

/// Les chemins et les tailles d'une source quelconque. Elle ne sait pas dire
/// une taille sans lire : on lit — c'est le prix d'une source qui n'est pas
/// un dossier.
#[cfg(feature = "code")]
pub fn source_files(source: &dyn crate::code_tools::FileSource) -> Result<Vec<(String, u64)>, String> {
    source
        .list()?
        .into_iter()
        .map(|path| {
            let bytes = source.read(&path)?.map(|c| c.len() as u64).unwrap_or(0);
            Ok((path, bytes))
        })
        .collect()
}

/// Un débit d'embarquement, en caractères par seconde.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    pub chars_per_second: f64,
}

impl Rate {
    /// `chars` caractères embarqués en `took`. `None` si l'échantillon ne dit
    /// rien (vide, ou trop bref pour être chronométré).
    pub fn from_sample(chars: usize, took: Duration) -> Option<Self> {
        (chars > 0 && !took.is_zero()).then(|| Self { chars_per_second: chars as f64 / took.as_secs_f64() })
    }

    /// Le débit **tenu dans la durée** quand le régulateur de rafale est
    /// actif : la carte ne travaille que `rafale / (rafale + pause)` du temps.
    /// Un embarqueur distant n'est pas cadencé d'ici : passer `None`.
    pub fn paced(self, pacing: Option<BurstSettings>) -> Self {
        match pacing {
            Some(p) if !p.pause.is_zero() => {
                let duty = p.target.as_secs_f64() / (p.target + p.pause).as_secs_f64();
                Self { chars_per_second: self.chars_per_second * duty }
            }
            _ => self,
        }
    }

    pub fn duration_for(self, chars: u64) -> Duration {
        Duration::from_secs_f64(chars as f64 / self.chars_per_second.max(1.0))
    }
}

/// **La sonde** : embarque `samples` une fois et rend le débit brut.
///
/// Un premier appel à vide chauffe les noyaux — sans lui la sonde mesurerait
/// la compilation des pipelines, pas le modèle.
pub fn probe_rate(embedder: &dyn Embedder, samples: &[String]) -> Result<Option<Rate>, String> {
    if samples.is_empty() {
        return Ok(None);
    }
    embedder.embed(&samples[..1]).map_err(|e| e.to_string())?;
    let chars: usize = samples.iter().map(|s| s.len()).sum();
    let t = std::time::Instant::now();
    embedder.embed(samples).map_err(|e| e.to_string())?;
    Ok(Rate::from_sample(chars, t.elapsed()))
}

/// La carte **de celui qui calcule**, pour l'heuristique du premier index :
/// un service d'embarquement attaché, sinon la carte de ce poste.
pub fn computing_card(service_attached: bool, local: CardClass) -> CardClass {
    if service_attached {
        CardClass::Service
    } else {
        local
    }
}

/// Ce que `estimate` rend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    pub survey: Survey,
    /// Le modèle du premier index, et pourquoi.
    pub model: String,
    pub model_reason: String,
    /// Durée prévue des vecteurs, en secondes. `None` : aucun débit connu
    /// pour cet embarqueur — l'estimation ne l'invente pas.
    pub vectors_seconds: Option<u64>,
    /// `index` demandera une confirmation. Sans durée connue, on ne bloque
    /// pas sur une ignorance : `false`.
    pub needs_confirmation: bool,
    pub confirm_above_seconds: u64,
}

impl Estimate {
    /// Pur : tout ce qui se lit sur le poste est passé en paramètre.
    pub fn new(survey: Survey, choice: Choice, rate: Option<Rate>, confirm_above: Duration) -> Self {
        let vectors = rate.map(|r| r.duration_for(survey.bytes));
        Self {
            survey,
            model: choice.model,
            model_reason: choice.reason,
            vectors_seconds: vectors.map(|d| d.as_secs()),
            needs_confirmation: vectors.is_some_and(|d| d > confirm_above),
            confirm_above_seconds: confirm_above.as_secs(),
        }
    }

    /// Ce qu'un agent lit, en quelques lignes.
    pub fn text(&self) -> String {
        let mo = |b: u64| format!("{:.1} Mo", b as f64 / 1e6);
        let mut out = format!("{} fichiers retenus ({})", self.survey.files, mo(self.survey.bytes));
        let kinds: Vec<String> = self.survey.kept.iter().map(|(k, (n, _))| format!("{n} {k}")).collect();
        if !kinds.is_empty() {
            out.push_str(&format!(" : {}", kinds.join(", ")));
        }
        for (reason, (n, bytes)) in &self.survey.skipped {
            out.push_str(&format!("\n{n} écartés ({}) : {reason}", mo(*bytes)));
        }
        out.push_str(&format!("\nmodèle : {} — {}", self.model, self.model_reason));
        match self.vectors_seconds {
            Some(s) => out.push_str(&format!("\nvecteurs : environ {}", human(s))),
            None => out.push_str("\nvecteurs : durée inconnue (aucun débit mesuré pour cet embarqueur)"),
        }
        if self.needs_confirmation {
            out.push_str(&format!(" — au-delà de {}, confirmation demandée", human(self.confirm_above_seconds)));
        }
        out
    }
}

/// Un service d'embarquement est-il attaché à ce processus ?
pub fn service_attached() -> bool {
    #[cfg(feature = "daemon")]
    {
        crate::model_source::Capability::Embed.variables().iter().any(|v| std::env::var(v).is_ok_and(|v| !v.trim().is_empty()))
    }
    #[cfg(not(feature = "daemon"))]
    {
        false
    }
}

/// **L'estimation sur ce poste** : ce que la machine sait d'elle-même est lu
/// ici — le modèle écrit dans l'environnement, le service attaché, la carte,
/// le régulateur de rafale — et le reste est [`Estimate::new`], pur.
///
/// `rate` : le débit **brut** de l'embarqueur (noté en base, ou tout juste
/// sondé) ; `remote` : il vit ailleurs (`Embedder::distant`), le régulateur
/// d'ici ne le ralentit pas.
pub fn estimate_here(
    files: &[(String, u64)],
    excluded: &[(String, &'static str)],
    policy: impl Fn(&str, u64) -> Kept,
    rate: Option<Rate>,
    remote: bool,
) -> Estimate {
    let survey = survey(files.iter().map(|(p, b)| (p.as_str(), *b)), policy).with_exclusions(excluded);
    let service = service_attached();
    let card = computing_card(service, crate::regime::card_class_of_this_machine());
    let sole = !service && crate::regime::sole_card_of_this_machine_drives_display();
    let explicit = std::env::var(crate::embedding_choice::MODEL_VARIABLE).ok();
    let choice = crate::embedding_choice::choose(survey.files, survey.bytes, card, sole, explicit.as_deref());
    let pacing = if remote { None } else { crate::burst::active() };
    Estimate::new(survey, choice, rate.map(|r| r.paced(pacing)), CONFIRM_ABOVE)
}

fn human(seconds: u64) -> String {
    match seconds {
        0..=89 => format!("{seconds} s"),
        90..=5_399 => format!("{} min", (seconds + 30) / 60),
        _ => format!("{} h {:02}", seconds / 3_600, (seconds % 3_600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding_choice::{recommended_model, DEFAULT_MODEL, FAST_MODEL};

    const MO: u64 = 1_000_000;

    /// Un dépôt comme celui-ci : du code, un texte, des données qu'on écarte.
    #[cfg(feature = "code")]
    #[test]
    fn un_depot_de_code_se_compte_par_la_politique_d_ingestion() {
        let files = [("src/lib.rs", 40_000), ("README.md", 3_000), ("data/users.csv", 9 * MO), ("app.min.js", 80_000)];
        let s = survey(files, code_policy);
        assert_eq!((s.files, s.bytes), (2, 43_000));
        assert_eq!(s.kept["code"], (1, 40_000));
        assert_eq!(s.kept["texte"], (1, 3_000));
        assert_eq!(s.skipped_files(), 2);
        assert!(s.skipped.values().any(|(_, b)| *b == 9 * MO), "les données écartées gardent leur poids : {s:?}");
        // Ce que la source a écarté d'elle-même s'ajoute, avec sa raison.
        let excluded = vec![(".env".to_string(), "secret probable"), ("id_rsa".to_string(), "secret probable")];
        let s = s.with_exclusions(&excluded);
        assert_eq!(s.skipped["secret probable"].0, 2);
        assert_eq!(s.skipped_files(), 4);
    }

    /// **Rien de propre au code** : un dossier de documents, sa politique.
    #[test]
    fn un_dossier_de_documents_s_estime_avec_sa_propre_politique() {
        let policy = |path: &str, bytes: u64| match path.rsplit('.').next() {
            Some("md" | "txt") if bytes <= 200_000 => Kept::Yes("note"),
            Some("md" | "txt") => Kept::No("trop long pour une note"),
            _ => Kept::No("pas un document"),
        };
        let s = survey([("cr/2026-09-18.md", 12_000), ("cr/annexe.txt", 900_000), ("photo.jpg", 4 * MO)], policy);
        assert_eq!((s.files, s.bytes), (1, 12_000));
        assert_eq!(s.skipped["trop long pour une note"], (1, 900_000));
        assert_eq!(s.skipped["pas un document"], (1, 4 * MO));
    }

    #[test]
    fn le_debit_donne_la_duree_et_le_regulateur_la_rallonge() {
        let r = Rate::from_sample(150_000, Duration::from_secs(1)).unwrap();
        assert_eq!(r.duration_for(35 * MO).as_secs(), 233);
        // Rafales de 50 ms, 150 ms entre deux : un quart du temps sur la carte.
        let paced = r.paced(Some(BurstSettings { target: Duration::from_millis(50), pause: Duration::from_millis(150) }));
        assert_eq!(paced.duration_for(35 * MO).as_secs(), 933);
        // Sans régulateur, ou sans pause, rien ne change.
        assert_eq!(r.paced(None), r);
        assert_eq!(r.paced(Some(BurstSettings { target: Duration::from_millis(50), pause: Duration::ZERO })), r);
        // Un échantillon vide ou sans durée ne dit rien.
        assert_eq!(Rate::from_sample(0, Duration::from_secs(1)), None);
        assert_eq!(Rate::from_sample(10, Duration::ZERO), None);
    }

    /// La sonde, sur un embarqueur factice qui « dure » ce qu'on décide.
    #[test]
    fn la_sonde_mesure_l_embarqueur_attache() {
        #[derive(Debug)]
        struct Lent;
        impl Embedder for Lent {
            fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, crate::embedder::EmbedError> {
                let chars: u64 = texts.iter().map(|t| t.len() as u64).sum();
                std::thread::sleep(Duration::from_micros(10 * chars));
                Ok(texts.iter().map(|_| vec![0.0]).collect())
            }
            fn dim(&self) -> usize {
                1
            }
            fn name(&self) -> &str {
                "lent"
            }
        }
        let samples: Vec<String> = (0..8).map(|_| "x".repeat(500)).collect();
        let r = probe_rate(&Lent, &samples).unwrap().expect("un débit");
        // 10 µs par caractère : 100 000 caractères par seconde, à la louche.
        assert!((40_000.0..110_000.0).contains(&r.chars_per_second), "{r:?}");
        assert_eq!(probe_rate(&Lent, &[]).unwrap(), None);
    }

    #[cfg(feature = "code")]
    #[test]
    fn l_estimation_dit_le_modele_la_duree_et_la_confirmation() {
        let s = survey([("a.rs", 35 * MO)], code_policy);
        let choice = recommended_model(s.files, s.bytes, CardClass::Dedicated { vram_bytes: 31 << 30 }, false);
        let rate = Rate { chars_per_second: 150_000.0 };

        let rapide = Estimate::new(s.clone(), choice.clone(), Some(rate), CONFIRM_ABOVE);
        assert_eq!((rapide.model.as_str(), rapide.vectors_seconds, rapide.needs_confirmation), (DEFAULT_MODEL, Some(233), false));

        let lente = Estimate::new(s.clone(), choice.clone(), Some(Rate { chars_per_second: 20_000.0 }), CONFIRM_ABOVE);
        assert_eq!((lente.vectors_seconds, lente.needs_confirmation), (Some(1_750), true));
        assert!(lente.text().contains("environ 29 min") && lente.text().contains("confirmation demandée"), "{}", lente.text());

        // Sans débit connu : on ne l'invente pas, et on ne bloque pas dessus.
        let inconnue = Estimate::new(s, choice, None, CONFIRM_ABOVE);
        assert_eq!((inconnue.vectors_seconds, inconnue.needs_confirmation), (None, false));
        assert!(inconnue.text().contains("durée inconnue"), "{}", inconnue.text());
    }

    /// **Un service attaché fait taire les déclencheurs de la carte d'ici.**
    /// Le Z13 — carte intégrée, seule à porter l'affichage — partirait en
    /// 107m ; quand c'est un service qui calcule, il n'y a pas de raison.
    #[test]
    fn un_service_attache_fait_taire_les_declencheurs_de_la_carte_d_ici() {
        let ici = recommended_model(1_642, 10 * MO, computing_card(false, CardClass::Integrated), true);
        assert_eq!(ici.model, FAST_MODEL);
        let service = recommended_model(1_642, 10 * MO, computing_card(true, CardClass::Integrated), true);
        assert_eq!(service.model, DEFAULT_MODEL);
        assert!(service.reason.contains("service d'embarquement"), "{}", service.reason);
        // Le déclencheur de taille, lui, ne dépend pas de la carte.
        assert_eq!(recommended_model(62_000, 31 * MO, CardClass::Service, false).model, FAST_MODEL);
        // Sans carte du tout mais avec un service : 278m aussi.
        assert_eq!(recommended_model(100, MO, computing_card(true, CardClass::None), false).model, DEFAULT_MODEL);
    }
}
