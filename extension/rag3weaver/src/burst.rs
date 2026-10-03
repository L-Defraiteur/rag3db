//! **La rafale se règle en durée, pas en caractères** — pour une carte que
//! l'affichage partage.
//!
//! Lucie, le 3 octobre 2026, devant un écran qui se figeait pendant une passe
//! d'embarquement :
//!
//! > peut-être surtout des plus petits blocs, espacés de quelques centaines de
//! > millisecondes, pour laisser respirer l'écran
//!
//! Un appel `embed()` est une série de noyaux que rien ne préempte : ni wgpu
//! ni Vulkan n'exposent de priorité de file. Le compositeur attend la fin de
//! l'appel, donc **le gel dure ce que dure une rafale**. Le budget d'avant
//! était en caractères — 2 048, calibré le 27 août sur BGE-M3 et une R9700 —
//! et la même rafale durait 26 ms avec un modèle, 5 ms avec un autre, sur le
//! même poste. Ce qu'on veut tenir, c'est une durée.
//!
//! # La règle
//!
//! Deux réglages, et ce sont les seuls : **la durée visée d'une rafale** et
//! **la pause après elle**. Le premier lot part petit — c'est une sonde. Sa
//! durée mesurée donne un débit ; le lot suivant se dimensionne par règle de
//! trois pour durer ce qu'on vise. Aucune table par modèle ni par carte : le
//! poste se calibre en quelques lots, et le même code sert l'iGPU d'un
//! portable comme une carte dédiée qu'on partage.
//!
//! # Ce que le régulateur change, et ce qu'il ne change pas
//!
//! Il ne change que **la taille des lots** : chaque texte est embarqué une
//! fois, écrit une fois, avec son marqueur. Avec un embarqueur déterministe
//! les vecteurs sont identiques au bit près
//! (`le_regulateur_ne_change_que_la_taille_des_lots`).
//!
//! Avec un vrai modèle en Flex32, **la composition d'un lot déplace un
//! vecteur de quelques 1e-5** : le lot est rembourré à son texte le plus
//! long, et la demi-précision n'arrondit pas pareil selon ce rembourrage.
//! Mesuré le 3 octobre 2026 sur granite-278m, 64 textes : 4,4e-5 d'écart
//! absolu maximal entre un texte seul et le même dans un lot, zéro quand on
//! rejoue le même découpage. C'est l'ordre de grandeur de l'écart entre deux
//! cartes, et c'était déjà vrai de tout changement de budget de lot : ce
//! n'est pas un bug du régulateur.
//!
//! # Ce qui est pur ici, et ce qui ne l'est pas
//!
//! [`BurstPacer`] ne lit pas l'heure et ne dort pas : on lui **donne** la
//! durée d'un lot, il rend le budget du suivant et la pause à observer. C'est
//! ce qui le rend testable sans carte et sans attendre — un embarqueur factice
//! « dure » ce que le test décide. Le chronomètre et le sommeil restent chez
//! l'appelant, aux deux seuls endroits qui exécutent des lots.

use std::ops::Range;
use std::time::{Duration, Instant};

use crate::embedder::{lot_budget, lot_budget_si, stable_batches, stable_count, LotBudget, EMBED_CHAR_BUDGET};
use crate::regime::Regime;

/// La variable de la durée visée d'une rafale, en millisecondes.
pub const TARGET_VARIABLE: &str = "RAG3WEAVER_BURST_MS";
/// La variable de la pause après chaque rafale, en millisecondes.
pub const PAUSE_VARIABLE: &str = "RAG3WEAVER_BURST_PAUSE_MS";

/// Le premier lot, quand il n'y a pas encore de mesure : la valeur prudente
/// d'avant. **Il part petit exprès.** Le pire cas est une première rafale plus
/// courte que nécessaire, jamais un gel ; partir du conseil du modèle et
/// corriger ensuite, ce serait offrir une première rafale de plus d'une
/// seconde pour apprendre qu'elle était trop longue.
pub const PROBE_CHARS: usize = 2_048;

/// Jamais moins : en dessous, un lot ne porte plus un chunk entier et le
/// régulateur piétinerait sur des lots d'un élément.
pub const FLOOR_CHARS: usize = 512;

/// D'un lot au suivant, pas plus d'un doublement ni d'une division par deux.
/// Un lot aberrant — un chunk énorme, une recompilation de noyaux — ne doit
/// pas faire osciller la suite.
const MAX_STEP: f64 = 2.0;

/// Les deux réglages d'une carte partagée avec l'affichage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurstSettings {
    /// Ce que doit durer une rafale. C'est le gel maximal qu'on accepte.
    pub target: Duration,
    /// Le trou après chaque rafale : le temps laissé au compositeur.
    pub pause: Duration,
}

impl BurstSettings {
    /// **Provisoire, en attendant le tableau mesuré sur le poste.** Une rafale
    /// de trois images et un trou de dix : l'intuition de Lucie, « petits
    /// blocs, espacés ». Le défaut définitif sera choisi par elle sur deux ou
    /// trois couples mesurés (débit, et ce que ça fait à l'écran).
    pub const DEFAULT: Self = Self { target: Duration::from_millis(50), pause: Duration::from_millis(150) };

    /// Les réglages lus dans l'environnement, sinon le défaut. Une valeur
    /// illisible n'est pas une raison de s'arrêter : on le dit et on prend le
    /// défaut — la même règle que pour le régime.
    pub fn from_env() -> Self {
        Self::from_values(std::env::var(TARGET_VARIABLE).ok().as_deref(), std::env::var(PAUSE_VARIABLE).ok().as_deref())
    }

    /// Pur, pour les tests : les deux valeurs telles qu'on les a lues.
    pub fn from_values(target_ms: Option<&str>, pause_ms: Option<&str>) -> Self {
        let read = |raw: Option<&str>, variable: &str, default: Duration, min_ms: u64| -> Duration {
            match raw.map(str::trim).filter(|v| !v.is_empty()) {
                None => default,
                Some(v) => match v.parse::<u64>() {
                    Ok(ms) => Duration::from_millis(ms.max(min_ms)),
                    Err(_) => {
                        eprintln!("[rag3weaver] {variable}='{v}' illisible (des millisecondes) — {} ms", default.as_millis());
                        default
                    }
                },
            }
        };
        Self {
            // Une rafale d'une milliseconde ne porte rien ; une pause peut être nulle.
            target: read(target_ms, TARGET_VARIABLE, Self::DEFAULT.target, 1),
            pause: read(pause_ms, PAUSE_VARIABLE, Self::DEFAULT.pause, 0),
        }
    }
}

/// **Le régulateur** : combien de caractères pour la prochaine rafale.
#[derive(Debug, Clone)]
pub struct BurstPacer {
    settings: BurstSettings,
    budget: usize,
    ceiling: usize,
}

impl BurstPacer {
    /// `ceiling` : jamais plus de caractères par rafale que ce plafond — ce que
    /// le modèle tient d'un coup. Le régulateur vise une durée, il ne dépasse
    /// pas pour autant ce que la carte sait recevoir.
    pub fn new(settings: BurstSettings, ceiling: usize) -> Self {
        let ceiling = ceiling.max(FLOOR_CHARS);
        Self { settings, budget: PROBE_CHARS.min(ceiling), ceiling }
    }

    /// Le budget de la prochaine rafale, en caractères.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// La pause à observer après chaque rafale.
    pub fn pause(&self) -> Duration {
        self.settings.pause
    }

    pub fn settings(&self) -> BurstSettings {
        self.settings
    }

    /// La pause due après une rafale qui a duré `took` : entière si la rafale
    /// a tenu sa durée, **au prorata** si elle a été plus courte. Une requête
    /// de recherche — un texte, quelques millisecondes — ne doit pas payer le
    /// trou d'une rafale d'ingestion.
    pub fn pause_after(&self, took: Duration) -> Duration {
        let target = self.settings.target;
        if took >= target {
            self.settings.pause
        } else {
            self.settings.pause.mul_f64(took.as_secs_f64() / target.as_secs_f64())
        }
    }

    /// **Un lot vient de finir** : `chars` caractères ont pris `took`.
    ///
    /// Règle de trois sur le débit de ce lot, bornée : pas plus d'un
    /// doublement ni d'une division par deux d'un lot à l'autre, et toujours
    /// entre le plancher et le plafond. Un échantillon sans contenu ou sans
    /// durée ne dit rien et ne change rien.
    pub fn record(&mut self, chars: usize, took: Duration) {
        if chars == 0 || took.is_zero() {
            return;
        }
        let rate = chars as f64 / took.as_secs_f64();
        let ideal = rate * self.settings.target.as_secs_f64();
        let current = self.budget as f64;
        let stepped = ideal.clamp(current / MAX_STEP, current * MAX_STEP);
        self.budget = (stepped.round() as usize).clamp(FLOOR_CHARS.min(self.ceiling), self.ceiling);
    }
}

/// **Le régulateur et son horloge** — la seule partie de ce module qui lit
/// l'heure et qui dort.
///
/// La pause n'est pas dormie *après* la rafale mais **exigée avant la
/// suivante** : celui qui vient de finir rend sa réponse tout de suite, et le
/// trou est tenu quand même — y compris entre deux appelants différents, tant
/// qu'ils passent par la même porte.
#[derive(Debug)]
pub struct BurstGate {
    pacer: BurstPacer,
    not_before: Option<Instant>,
}

impl BurstGate {
    pub fn new(pacer: BurstPacer) -> Self {
        Self { pacer, not_before: None }
    }

    /// Le budget de la prochaine rafale, en caractères.
    pub fn budget(&self) -> usize {
        self.pacer.budget()
    }

    /// Attend la fin du trou laissé par la rafale précédente.
    pub fn wait(&self) {
        if let Some(wait) = self.not_before.and_then(|at| at.checked_duration_since(Instant::now())) {
            std::thread::sleep(wait);
        }
    }

    /// Une rafale de `chars` caractères vient de durer `took`.
    pub fn done(&mut self, chars: usize, took: Duration) {
        self.pacer.record(chars, took);
        self.not_before = Some(Instant::now() + self.pacer.pause_after(took));
    }
}

/// **Faut-il ménager l'écran ?** — pur, pour les tests.
///
/// - Un réglage explicite de l'ancien rythme (`RAG3WEAVER_GPU_DUTY`,
///   `RAG3WEAVER_EMBED_CHAR_BUDGET`) garde le dernier mot : c'est lui qui
///   s'applique, pas le régulateur.
/// - `plein` **écrit** veut dire plein : on prend la carte sans ménagement.
/// - `confort` ménage dès que la carte des modèles est celle du compositeur.
/// - **Rien d'écrit** : on ménage si la seule carte du poste porte
///   l'affichage (Lucie, 3 octobre 2026). Le reste de `confort` — l'origine
///   de l'inférence agentique — ne bouge pas : ce défaut ne change que le
///   rythme de la carte.
pub fn applies(explicit_regime: Option<Regime>, sole_card_drives_display: bool, shared_under_confort: bool, explicit_tuning: bool) -> bool {
    if explicit_tuning {
        return false;
    }
    match explicit_regime {
        Some(Regime::Plein) => false,
        Some(Regime::Confort) => shared_under_confort,
        None => sole_card_drives_display,
    }
}

/// Les réglages de rafale **si** ce processus doit ménager l'écran.
pub fn active() -> Option<BurstSettings> {
    let set = |v: &str| std::env::var(v).map(|v| !v.trim().is_empty()).unwrap_or(false);
    let explicit_tuning = set("RAG3WEAVER_GPU_DUTY") || set("RAG3WEAVER_EMBED_CHAR_BUDGET");
    let explicit = Regime::explicit();
    let shared_under_confort = explicit == Some(Regime::Confort) && Regime::Confort.carte_partagee();
    applies(explicit, crate::regime::sole_card_of_this_machine_drives_display(), shared_under_confort, explicit_tuning)
        .then(BurstSettings::from_env)
}

/// Comment une passe d'embarquement se découpe.
#[derive(Debug, Clone)]
pub enum Batches {
    /// Tout est découpé d'avance : la carte est à soi, ou quelqu'un d'autre
    /// la tient (un démon, une API).
    Fixed(Vec<Range<usize>>),
    /// Une rafale après l'autre, chacune dimensionnée par la précédente.
    /// `budget` est **ce que le modèle tient d'un coup** : le plafond.
    Paced { lens: Vec<usize>, budget: LotBudget, settings: BurstSettings },
}

/// Le découpage d'une passe sur des longueurs **triées**.
///
/// `holds_card` : cet appelant soumet lui-même à la carte (l'embarqueur
/// n'est pas `distant`). Sinon c'est celui qui la tient qui règle le rythme.
pub fn plan(lens: &[usize], advice: Option<(usize, usize)>, default_items: usize, holds_card: bool) -> Batches {
    if !holds_card {
        let explicit = std::env::var("RAG3WEAVER_EMBED_CHAR_BUDGET").ok().and_then(|v| v.trim().parse::<usize>().ok()).filter(|v| *v > 0);
        return plan_remote(lens, advice, default_items, explicit);
    }
    plan_with(lens, advice, default_items, active())
}

/// **Le découpage pour un embarqueur distant** : le lot que son modèle
/// conseille, quelle que soit la carte d'ici.
///
/// Rétrécir les lots ménage la carte *de celui qui calcule* ; quand c'est un
/// service, c'est à lui de la protéger, et il le fait (`EmbedDaemon::par_lots`
/// redécoupe ce qu'il reçoit). Mesuré le 3 octobre 2026 : sous `confort`, le
/// client envoyait au service d'un autre poste des lots de 2 048 caractères —
/// deux morceaux par requête — et la carte distante attendait à 28 %.
/// Un `RAG3WEAVER_EMBED_CHAR_BUDGET` écrit garde le dernier mot.
pub fn plan_remote(lens: &[usize], advice: Option<(usize, usize)>, default_items: usize, explicit: Option<usize>) -> Batches {
    Batches::Fixed(stable_batches(lens, lot_budget_si(advice, default_items, explicit, false, EMBED_CHAR_BUDGET)))
}

/// [`plan`], les réglages en paramètre — pour les tests.
pub fn plan_with(lens: &[usize], advice: Option<(usize, usize)>, default_items: usize, settings: Option<BurstSettings>) -> Batches {
    match settings {
        Some(settings) => Batches::Paced { lens: lens.to_vec(), budget: ceiling(advice, default_items), settings },
        None => Batches::Fixed(stable_batches(lens, lot_budget(advice, default_items))),
    }
}

/// Ce que le modèle tient d'un coup, sans égard pour l'écran : le plafond
/// d'une rafale.
pub fn ceiling(advice: Option<(usize, usize)>, default_items: usize) -> LotBudget {
    lot_budget_si(advice, default_items, None, false, EMBED_CHAR_BUDGET)
}

/// [`next_batch`] sous un plafond de modèle et le budget courant d'une porte.
/// Les comptes restent « stables » quand le modèle le demande : les formes de
/// lot se répètent et l'autotune ne recompile pas à chaque rafale.
pub fn next_paced_batch(lens: &[usize], start: usize, budget: LotBudget, chars: usize) -> Range<usize> {
    let lot = next_batch(lens, start, budget.max_items, chars.min(budget.max_chars), budget.max_area);
    if budget.stable && !lot.is_empty() {
        lot.start..lot.start + stable_count(lot.len())
    } else {
        lot
    }
}

/// **Le prochain lot**, à partir de `start`, dans des longueurs triées ou non.
///
/// Les mêmes bornes que le découpage d'avance (`embedder::stable_batches`) —
/// le nombre d'éléments, les caractères, et la surface d'attention
/// (éléments × jetons du plus long², à trois caractères par jeton) — mais pour
/// **un seul** lot : sur une carte partagée le budget change d'un lot à
/// l'autre, on ne peut donc plus tout découper avant le premier appel.
///
/// Un élément plus gros que le budget forme un lot à lui seul : un chunk ne se
/// coupe pas en deux appels.
pub fn next_batch(lens: &[usize], start: usize, max_items: usize, max_chars: usize, max_area: usize) -> Range<usize> {
    if start >= lens.len() {
        return lens.len()..lens.len();
    }
    let max_items = max_items.max(1);
    let mut end = start;
    let mut chars = 0usize;
    let mut longest_tokens = 0usize;
    while end < lens.len() {
        let len = lens[end];
        let count = end - start;
        if count >= max_items {
            break;
        }
        if count > 0 {
            if chars + len > max_chars {
                break;
            }
            let tokens = longest_tokens.max(len / 3).max(1);
            if (count + 1).saturating_mul(tokens).saturating_mul(tokens) > max_area {
                break;
            }
        }
        chars += len;
        longest_tokens = longest_tokens.max(len / 3);
        end += 1;
    }
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    /// **L'embarqueur factice** : il « dure » tant de microsecondes par
    /// caractère, plus un coût fixe par appel. Rien ne dort, rien ne touche à
    /// une carte : le test décide de la durée, le régulateur en tire le lot
    /// suivant.
    struct FakeEmbedder {
        micros_per_char: f64,
        fixed_micros: f64,
    }

    impl FakeEmbedder {
        fn took(&self, chars: usize) -> Duration {
            Duration::from_secs_f64((self.fixed_micros + self.micros_per_char * chars as f64) / 1e6)
        }
    }

    /// Fait tourner le régulateur sur `texts` textes de `len` caractères et
    /// rend, lot par lot, `(caractères, durée)`.
    fn run(pacer: &mut BurstPacer, embedder: &FakeEmbedder, texts: usize, len: usize) -> Vec<(usize, Duration)> {
        let lens = vec![len; texts];
        let mut pos = 0;
        let mut bursts = Vec::new();
        while pos < lens.len() {
            let range = next_batch(&lens, pos, usize::MAX, pacer.budget(), usize::MAX);
            let chars: usize = lens[range.clone()].iter().sum();
            let took = embedder.took(chars);
            pacer.record(chars, took);
            bursts.push((chars, took));
            pos = range.end;
        }
        bursts
    }

    fn settings(target_ms: u64, pause_ms: u64) -> BurstSettings {
        BurstSettings { target: MS(target_ms), pause: MS(pause_ms) }
    }

    /// Le premier lot est la sonde, quel que soit le modèle : petit, donc
    /// jamais un gel.
    #[test]
    fn le_premier_lot_part_petit() {
        let pacer = BurstPacer::new(settings(50, 150), 200_000);
        assert_eq!(pacer.budget(), PROBE_CHARS);
        // Un plafond plus bas que la sonde la borne.
        assert_eq!(BurstPacer::new(settings(50, 150), 1_000).budget(), 1_000);
    }

    /// **La convergence** : en quelques lots, la rafale dure ce qu'on vise.
    /// 12,8 µs par caractère — granite-278m sur ce poste, à trois caractères
    /// par jeton et 26 000 jetons par seconde.
    #[test]
    fn les_rafales_convergent_vers_la_duree_visee() {
        let mut pacer = BurstPacer::new(settings(50, 150), 200_000);
        let slow = FakeEmbedder { micros_per_char: 12.8, fixed_micros: 0.0 };
        let bursts = run(&mut pacer, &slow, 400, 300);
        // La sonde : 2 048 caractères → 1 800 pleins (six textes de 300), 23 ms.
        assert!(bursts[0].1 < MS(30), "la sonde est courte : {:?}", bursts[0]);
        // Passé les quatre premiers, chaque rafale tient dans ±15 % de la cible.
        for (chars, took) in &bursts[4..bursts.len() - 1] {
            let ms = took.as_secs_f64() * 1e3;
            assert!((42.0..=58.0).contains(&ms), "rafale de {chars} caractères : {ms:.1} ms");
        }
        // Et jamais une rafale ne dépasse nettement la cible, même en montant.
        assert!(bursts.iter().all(|(_, t)| *t <= MS(60)), "{bursts:?}");
    }

    /// **Aucune table par modèle** : un modèle six fois plus rapide reçoit des
    /// lots six fois plus gros, pour la même durée. 2,25 µs par caractère —
    /// granite-107m ici, 148 000 jetons par seconde.
    #[test]
    fn un_modele_plus_rapide_recoit_des_lots_plus_gros_pour_la_meme_duree() {
        let run_with = |micros_per_char: f64| {
            let mut pacer = BurstPacer::new(settings(50, 150), 400_000);
            let e = FakeEmbedder { micros_per_char, fixed_micros: 0.0 };
            run(&mut pacer, &e, 2_000, 300);
            pacer.budget()
        };
        let slow = run_with(12.8);
        let fast = run_with(2.25);
        let ratio = fast as f64 / slow as f64;
        assert!((5.0..=6.4).contains(&ratio), "lents {slow}, rapides {fast}, rapport {ratio:.2}");
    }

    /// **Un lot aberrant ne fait pas osciller la suite.** Une recompilation de
    /// noyaux fait durer un lot vingt fois trop : le suivant est divisé par
    /// deux, pas par vingt, et le régulateur revient en deux lots.
    #[test]
    fn un_lot_aberrant_ne_fait_pas_osciller() {
        let mut pacer = BurstPacer::new(settings(50, 150), 200_000);
        let e = FakeEmbedder { micros_per_char: 12.8, fixed_micros: 0.0 };
        run(&mut pacer, &e, 300, 300);
        let stable = pacer.budget();
        pacer.record(stable, MS(1_000));
        assert_eq!(pacer.budget(), (stable as f64 / 2.0).round() as usize, "une division par deux, pas plus");
        pacer.record(pacer.budget(), e.took(pacer.budget()));
        pacer.record(pacer.budget(), e.took(pacer.budget()));
        let back = pacer.budget() as f64 / stable as f64;
        assert!((0.9..=1.1).contains(&back), "revenu à {back:.2} du régime stable");
        // Et dans l'autre sens : un lot miraculeusement court ne fait que doubler.
        let before = pacer.budget();
        pacer.record(before, Duration::from_micros(10));
        assert_eq!(pacer.budget(), before * 2);
    }

    /// Les bornes dures : jamais sous le plancher, jamais au-dessus du plafond,
    /// et un échantillon vide ne change rien.
    #[test]
    fn les_bornes_tiennent() {
        let mut pacer = BurstPacer::new(settings(50, 150), 6_000);
        // Une carte très rapide : le plafond arrête la montée.
        for _ in 0..10 {
            pacer.record(pacer.budget(), Duration::from_micros(100));
        }
        assert_eq!(pacer.budget(), 6_000);
        // Une carte très lente : le plancher arrête la descente.
        for _ in 0..10 {
            pacer.record(pacer.budget(), MS(5_000));
        }
        assert_eq!(pacer.budget(), FLOOR_CHARS);
        let before = pacer.budget();
        pacer.record(0, MS(10));
        pacer.record(100, Duration::ZERO);
        assert_eq!(pacer.budget(), before, "un échantillon sans contenu ou sans durée ne dit rien");
    }

    /// Un coût fixe par appel — le lancement des noyaux — ne trompe pas la
    /// règle de trois durablement : elle converge quand même sous la cible.
    #[test]
    fn un_cout_fixe_par_appel_ne_fait_pas_depasser_la_cible() {
        let mut pacer = BurstPacer::new(settings(50, 150), 200_000);
        let e = FakeEmbedder { micros_per_char: 12.8, fixed_micros: 8_000.0 };
        let bursts = run(&mut pacer, &e, 600, 300);
        for (chars, took) in &bursts[5..bursts.len() - 1] {
            let ms = took.as_secs_f64() * 1e3;
            assert!((40.0..=58.0).contains(&ms), "rafale de {chars} caractères : {ms:.1} ms");
        }
    }

    // ── Le découpage d'un lot ───────────────────────────────────────────

    #[test]
    fn le_lot_se_ferme_sur_les_caracteres() {
        let lens = [300, 300, 300, 300];
        assert_eq!(next_batch(&lens, 0, usize::MAX, 700, usize::MAX), 0..2);
        assert_eq!(next_batch(&lens, 2, usize::MAX, 700, usize::MAX), 2..4);
        assert_eq!(next_batch(&lens, 4, usize::MAX, 700, usize::MAX), 4..4, "plus rien");
    }

    /// Un chunk ne se coupe pas en deux appels : plus gros que le budget, il
    /// forme son lot — et le régulateur apprendra de sa durée.
    #[test]
    fn un_element_trop_gros_forme_son_lot() {
        let lens = [5_000, 100, 100];
        assert_eq!(next_batch(&lens, 0, usize::MAX, 1_000, usize::MAX), 0..1);
        assert_eq!(next_batch(&lens, 1, usize::MAX, 1_000, usize::MAX), 1..3);
    }

    #[test]
    fn le_nombre_et_la_surface_bornent_aussi() {
        let lens = [300; 10];
        assert_eq!(next_batch(&lens, 0, 4, usize::MAX, usize::MAX), 0..4, "le nombre d'éléments");
        // 300 caractères = 100 jetons ; surface (n) × 100² ≤ 35 000 → 3 éléments.
        assert_eq!(next_batch(&lens, 0, usize::MAX, usize::MAX, 35_000), 0..3, "la surface d'attention");
    }

    /// Les lots successifs couvrent tout, sans trou ni recouvrement, quel que
    /// soit le budget qui change entre deux.
    #[test]
    fn les_lots_successifs_couvrent_tout() {
        let lens: Vec<usize> = (0..200).map(|i| 50 + (i * 37) % 900).collect();
        let mut pos = 0;
        let mut budget = 512;
        let mut covered = 0;
        while pos < lens.len() {
            let r = next_batch(&lens, pos, 32, budget, usize::MAX);
            assert_eq!(r.start, pos);
            assert!(r.end > r.start, "un lot avance toujours");
            covered += r.len();
            pos = r.end;
            budget = if budget > 4_000 { 512 } else { budget * 2 };
        }
        assert_eq!(covered, lens.len());
    }

    // ── Les réglages ────────────────────────────────────────────────────

    #[test]
    fn les_reglages_se_lisent_ou_prennent_le_defaut() {
        assert_eq!(BurstSettings::from_values(None, None), BurstSettings::DEFAULT);
        assert_eq!(BurstSettings::from_values(Some("30"), Some("300")), settings(30, 300));
        // Une pause nulle est un réglage ; une rafale nulle n'en est pas un.
        assert_eq!(BurstSettings::from_values(Some("0"), Some("0")), settings(1, 0));
        // Illisible : on le dit et on prend le défaut.
        assert_eq!(BurstSettings::from_values(Some("vite"), Some("")), BurstSettings::DEFAULT);
    }

    #[test]
    fn une_rafale_courte_ne_paie_qu_une_part_de_la_pause() {
        let p = BurstPacer::new(BurstSettings { target: Duration::from_millis(50), pause: Duration::from_millis(200) }, 100_000);
        assert_eq!(p.pause_after(Duration::from_millis(50)), Duration::from_millis(200));
        assert_eq!(p.pause_after(Duration::from_millis(400)), Duration::from_millis(200), "jamais plus que la pause");
        assert_eq!(p.pause_after(Duration::from_millis(5)), Duration::from_millis(20), "une requête ne paie pas le trou d'une rafale");
    }

    #[test]
    fn on_menage_l_ecran_par_defaut_quand_la_seule_carte_le_porte() {
        // Rien d'écrit : la carte décide.
        assert!(applies(None, true, false, false));
        assert!(!applies(None, false, false, false), "une carte à soi ne se ménage pas");
        // `plein` écrit veut dire plein, même sur ce poste.
        assert!(!applies(Some(Regime::Plein), true, false, false));
        // `confort` suit sa propre question : la carte est-elle partagée ?
        assert!(applies(Some(Regime::Confort), false, true, false));
        assert!(!applies(Some(Regime::Confort), true, false, false));
        // Un réglage explicite de l'ancien rythme garde le dernier mot.
        assert!(!applies(None, true, false, true));
        assert!(!applies(Some(Regime::Confort), true, true, true));
    }

    #[test]
    fn la_porte_tient_le_trou_avant_la_rafale_suivante() {
        let settings = BurstSettings { target: Duration::from_millis(10), pause: Duration::from_millis(40) };
        let mut gate = BurstGate::new(BurstPacer::new(settings, 100_000));
        let t = Instant::now();
        gate.wait();
        assert!(t.elapsed() < Duration::from_millis(20), "la première rafale n'attend rien");
        gate.done(2_048, Duration::from_millis(10));
        let t = Instant::now();
        gate.wait();
        assert!(t.elapsed() >= Duration::from_millis(35), "le trou est tenu : {:?}", t.elapsed());
    }

    #[test]
    fn le_plan_suit_les_reglages() {
        let lens = vec![100usize; 400];
        // Sans réglage : le découpage d'avance, comme avant.
        // Et c'est **exactement** celui d'avant : une carte à soi, ou `plein`
        // écrit, ne voient aucune différence.
        // (Les deux côtés lisent le même environnement : l'égalité tient
        // quels que soient le régime et les réglages posés.)
        let Batches::Fixed(lots) = plan_with(&lens, Some((128, 512)), 32, None) else { panic!("un plan d'avance attendu") };
        assert_eq!(lots, stable_batches(&lens, lot_budget(Some((128, 512)), 32)));
        // Avec : une rafale après l'autre, sous le plafond du modèle — pas
        // sous le budget prudent d'une carte partagée.
        let Batches::Paced { budget, .. } = plan_with(&lens, Some((128, 512)), 32, Some(BurstSettings::DEFAULT)) else {
            panic!("un plan cadencé attendu")
        };
        assert_eq!(budget.max_items, 128);
        assert_eq!(budget.max_chars, 128 * 512 * 3);
        // Sans conseil du modèle, l'optimum mesuré d'avant sert de plafond.
        assert_eq!(ceiling(None, 32).max_chars, EMBED_CHAR_BUDGET);
    }

    #[test]
    fn les_lots_cadences_gardent_des_comptes_stables() {
        let lens = vec![100usize; 400];
        let budget = ceiling(Some((128, 512)), 32);
        // 2 048 caractères de textes de 100 : 20 tiendraient, 16 est stable.
        assert_eq!(next_paced_batch(&lens, 0, budget, 2_048), 0..16);
        // Et tout est couvert, sans trou ni doublon.
        let mut start = 0;
        while start < lens.len() {
            let lot = next_paced_batch(&lens, start, budget, 2_048);
            assert_eq!(lot.start, start);
            assert!(!lot.is_empty());
            start = lot.end;
        }
        assert_eq!(start, lens.len());
    }

    /// Un embarqueur distant reçoit le lot que son modèle conseille : la
    /// carte d'ici, partagée ou non, n'a rien à y voir.
    #[test]
    fn un_embarqueur_distant_recoit_le_lot_du_modele() {
        let lens = vec![1_000usize; 300];
        let Batches::Fixed(lots) = plan_remote(&lens, Some((32, 512)), 32, None) else { panic!("découpé d'avance") };
        assert_eq!(lots[0], 0..32, "32 séquences, pas deux : {:?}", &lots[..2]);
        // Sans conseil, l'optimum mesuré — pas le budget prudent du régime.
        let Batches::Fixed(lots) = plan_remote(&lens, None, 32, None) else { panic!("découpé d'avance") };
        assert_eq!(lots[0], 0..8);
        // Un budget écrit garde le dernier mot.
        let Batches::Fixed(lots) = plan_remote(&lens, Some((32, 512)), 32, Some(2_048)) else { panic!("découpé d'avance") };
        assert_eq!(lots[0], 0..2);
    }
}
