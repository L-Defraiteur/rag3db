//! **Le régime de travail** : un nom pour une composition de choix.
//!
//! Trois réglages décident si le poste reste utilisable pendant qu'on
//! travaille — la carte, le rapport cyclique, la longueur d'une rafale. Ils
//! existent chacun de leur côté, chacun avec sa variable d'environnement, et
//! personne ne se rappelle les trois.
//!
//! Un régime les nomme ensemble :
//!
//! | | `confort` | `plein` (défaut) |
//! |---|---|---|
//! | carte de l'embarqueur | la moins chargée | celle du système |
//! | rapport cyclique | 60 % | 100 % |
//! | rafale | 2 048 caractères | 8 192 |
//!
//! # La précédence, la même que partout
//!
//! **Le code l'emporte sur la variable, qui l'emporte sur le régime, qui
//! l'emporte sur le défaut.** Un régime ne force rien : il fournit ce que
//! personne n'a dit. `RAG3WEAVER_GPU_DUTY=90` avec `RAG3WEAVER_REGIME=confort`
//! donne 90, et c'est ce qu'on attend d'un réglage explicite.
//!
//! # Le quatrième réglage : d'où vient l'inférence
//!
//! `confort` envoie aussi l'agentique vers un fournisseur distant, pour que
//! l'inférence ne prenne aucune carte locale. Ce n'est pas une méthode de plus
//! sur `Regime` — le module en avait refusé une, à raison, parce que le choix
//! du `Llm` se fait chez l'appelant et qu'une intention que personne ne lit
//! est un mécanisme construit-et-jamais-appelé. C'est [`modele_agentique`], et
//! elle est appelée par les cinq suites qui recopiaient le même motif.
//!
//! ## L'exception à la précédence, et pourquoi
//!
//! Partout ailleurs, une variable l'emporte sur le régime. Ici non, et c'est
//! délibéré : `RAG3WEAVER_LOCAL_LLM` **traîne dans un profil**, elle n'est pas
//! posée pour la passe en cours. Si elle gagnait, `confort` reprendrait la
//! carte qu'il vient de libérer — la promesse serait tenue à trois quarts, en
//! silence, et il faudrait se rappeler de la retirer. C'est exactement ce
//! qu'un régime existe pour éviter.
//!
//! La distinction n'est donc pas « variable contre régime » mais **« ce qui
//! traîne » contre « ce qu'on demande maintenant »**. [`VARIABLE_LLM`]
//! (`RAG3WEAVER_LLM=local`) dit une intention pour cette passe, et elle gagne.
//!
//! ## Aucun repli silencieux
//!
//! Si `confort` veut le nuage et que le jeton manque, [`modele_agentique`]
//! rend `None` **en disant laquelle des trois conditions a manqué**. Elle ne
//! retombe pas sur le local : ce serait reprendre la carte que le régime
//! voulait libérer, au moment précis où l'on croit avoir demandé le contraire.

use std::path::Path;

#[cfg(feature = "openai-llm")]
use crate::openai_llm::OpenAiLlm;

/// À quel régime on travaille.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Regime {
    /// La machine reste utilisable pendant qu'on travaille.
    Confort,
    /// On prend ce qu'il y a. **Le défaut** : un changement de comportement ne
    /// doit pas arriver parce qu'on a ajouté un module.
    #[default]
    Plein,
}

/// La variable qui choisit le régime.
pub const VARIABLE: &str = "RAG3WEAVER_REGIME";

/// La variable qui choisit d'où vient l'inférence, **pour cette passe** :
/// `local` ou `distant`. Elle l'emporte sur le régime ; `RAG3WEAVER_LOCAL_LLM`
/// non — voir l'en-tête du module.
pub const VARIABLE_LLM: &str = "RAG3WEAVER_LLM";

impl Regime {
    /// Le régime courant, depuis l'environnement. Une valeur illisible n'est
    /// pas une raison de s'arrêter : on le dit et on prend le défaut.
    pub fn courant() -> Self {
        match std::env::var(VARIABLE) {
            Err(_) => Self::Plein,
            Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
                "" => Self::Plein,
                "confort" => Self::Confort,
                "plein" | "plein-regime" | "plein_regime" => Self::Plein,
                autre => {
                    eprintln!("[rag3weaver] {VARIABLE}='{autre}' inconnu (confort | plein) — plein");
                    Self::Plein
                }
            },
        }
    }

    /// Le rapport cyclique voulu, en pourcentage.
    pub fn duty(self) -> u32 {
        match self {
            Self::Confort => 60,
            Self::Plein => 100,
        }
    }

    /// La longueur d'une rafale, en caractères de texte par appel GPU.
    ///
    /// 2 048 en confort : le débit y perd un peu — l'optimum mesuré est vers
    /// 8 192 — mais une rafale quatre fois plus courte, c'est quatre fois plus
    /// d'occasions pour le compositeur de passer.
    pub fn budget_caracteres(self) -> usize {
        match self {
            Self::Confort => 2_048,
            Self::Plein => crate::embedder::EMBED_CHAR_BUDGET,
        }
    }

    /// La carte de l'embarqueur, sous la forme que `BurnDevice::parse` lit
    /// (`gpu:1`). `None` : rien à dire, on laisse le défaut.
    ///
    /// Alias historique de [`Self::carte_locale`], gardé parce que
    /// `burn_device.rs` le nommait ainsi.
    pub fn carte_embedder(self) -> Option<String> {
        self.carte_locale()
    }

    /// **La carte que tout modèle local devrait prendre.**
    ///
    /// Elle valait pour l'embarqueur seul, avec une raison d'efficacité : il
    /// tient le modèle des heures durant, tandis qu'un reranker ou un OCR
    /// prennent la carte le temps d'un appel.
    ///
    /// Cette raison reste vraie et cesse de décider sous `confort`, parce que
    /// le régime ne parle pas d'efficacité mais de **ne pas être dérangée** :
    /// un OCR qui prend la carte du compositeur le temps d'un appel fait
    /// exactement le tort que le régime existe pour éviter. Sous `plein`,
    /// personne n'est déplacé et l'arbitrage d'efficacité reprend.
    pub fn carte_locale(self) -> Option<String> {
        match self {
            Self::Plein => None,
            Self::Confort => carte_la_plus_libre(Path::new("/sys/class/drm")).map(|i| format!("gpu:{i}")),
        }
    }
}

/// D'où l'on veut que vienne l'inférence agentique.
///
/// Le choix se lit sans la feature `openai-llm` — c'est une **intention**, pas
/// un client ; seule la fabrique qui la matérialise en dépend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origine {
    /// Un serveur compatible OpenAI sur le poste — llama.cpp, Ollama.
    Locale,
    /// Vertex AI. Aucune carte locale prise.
    Distante,
}

impl Origine {
    /// Ce qu'on veut, en lisant l'environnement **et** le régime.
    ///
    /// L'ordre est celui de l'en-tête du module : l'intention de la passe
    /// (`RAG3WEAVER_LLM`) d'abord, le régime ensuite, et `RAG3WEAVER_LOCAL_LLM`
    /// seulement en dernier — parce qu'elle traîne dans un profil au lieu
    /// d'être posée pour ce qu'on fait maintenant.
    pub fn voulue(regime: Regime) -> Self {
        match std::env::var(VARIABLE_LLM) {
            Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
                "local" | "locale" => return Self::Locale,
                "distant" | "distante" | "cloud" | "vertex" => return Self::Distante,
                "" => {}
                autre => eprintln!(
                    "[rag3weaver] {VARIABLE_LLM}='{autre}' inconnu (local | distant) — on ignore"
                ),
            },
            Err(_) => {}
        }
        match regime {
            Regime::Confort => Self::Distante,
            Regime::Plein => {
                if std::env::var("RAG3WEAVER_LOCAL_LLM").is_ok() {
                    Self::Locale
                } else {
                    Self::Distante
                }
            }
        }
    }
}

/// **Le modèle agentique, en un seul endroit.**
///
/// Rend `None` avec une raison **écrite** plutôt qu'un silence : cinq suites
/// recopiaient ce motif, et deux d'entre elles avalaient leurs erreurs avec un
/// `.ok()?` nu — « pas de modèle configuré » y était indiscernable de « jeton
/// invalide », ce qui transforme une panne d'identifiants en suite ignorée.
///
/// `etiquette` préfixe les messages : c'est ce que la suite appelante mettait
/// à la main (`[cloud-agent]`, `[avis]`, `[mermaid]`, `[fil]`).
#[cfg(feature = "openai-llm")]
pub fn modele_agentique(etiquette: &str) -> Option<OpenAiLlm> {
    modele_agentique_nomme(etiquette).map(|(llm, _)| llm)
}

/// Comme [`modele_agentique`], mais rend aussi le nom du modèle — ce dont une
/// suite a besoin pour titrer son rapport.
#[cfg(feature = "openai-llm")]
pub fn modele_agentique_nomme(etiquette: &str) -> Option<(OpenAiLlm, String)> {
    match Origine::voulue(Regime::courant()) {
        Origine::Locale => {
            let base = std::env::var("RAG3WEAVER_LOCAL_LLM")
                .map_err(|_| {
                    eprintln!("[{etiquette}] origine locale demandée mais RAG3WEAVER_LOCAL_LLM n'est pas posée")
                })
                .ok()?;
            let nom = std::env::var("RAG3WEAVER_LOCAL_MODEL").unwrap_or_else(|_| "local".into());
            eprintln!("[{etiquette}] {nom} @ {base}");
            // Un modèle local n'a pas de quota mais il est lent : la politique
            // de réessai du nuage (60 s après un 429) n'a rien à faire ici.
            Some((OpenAiLlm::new(base, nom.clone()), nom))
        }
        Origine::Distante => {
            let projet = std::env::var("GOOGLE_CLOUD_PROJECT")
                .map_err(|_| eprintln!("[{etiquette}] GOOGLE_CLOUD_PROJECT n'est pas posée"))
                .ok()?;
            let source = crate::gcp_auth::TokenSource::from_env()
                .map_err(|e| eprintln!("[{etiquette}] identifiants Vertex introuvables : {e}"))
                .ok()?;
            let jeton = source
                .token()
                .map_err(|e| eprintln!("[{etiquette}] le jeton Vertex n'a pas pu être obtenu : {e}"))
                .ok()?;
            let lieu = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".into());
            let nom = std::env::var("VERTEX_MODEL").unwrap_or_else(|_| "google/gemini-3.5-flash".into());
            eprintln!("[{etiquette}] {nom} @ {lieu} · projet {projet}");
            Some((OpenAiLlm::vertex(&projet, &lieu, jeton, nom.clone()), nom))
        }
    }
}

/// **La carte la moins chargée**, par son rang parmi les cartes — l'index que
/// `gpu:N` attend. `None` s'il n'y a rien à choisir.
///
/// # Pourquoi pas « la carte sans écran »
///
/// C'était le premier critère, et il ne marchait pas — pour une raison qui
/// mérite d'être écrite, parce qu'elle se représente chaque fois qu'on lit
/// sysfs.
///
/// **`status=connected` ne veut pas dire qu'il y a un écran.** Sur le poste de
/// développement, le 29 août 2026, `card0-HDMI-A-3` se déclarait `connected`,
/// `enabled`, `dpms=On` — avec **zéro octet d'EDID et un seul mode, 640x480**.
/// Rien n'était branché dessus. Un vrai écran se reconnaît à son EDID et à sa
/// liste de modes : les deux sorties de `card2` rendent 384 et 128 octets
/// d'EDID, pour 46 et 15 modes. Le connecteur fantôme, lui, ne rend rien.
///
/// On aurait donc pu sauver le critère en exigeant un EDID non vide. On ne le
/// fait pas, parce que la question n'est pas « y a-t-il un écran » mais
/// **« quelqu'un se sert-il de cette carte »** — un écran branché sur un siège
/// qui n'est pas lancé n'occupe personne. La charge répond aux deux : 0,09 Go
/// contre 1,95 Go au repos, parce que les tampons du compositeur vivent sur
/// une carte et pas sur l'autre.
///
/// # Ce que ça suppose, et ce que ça ne suppose pas
///
/// - L'ordre PCI est celui que wgpu énumère. Vérifié ici (bus 04 → `gpu:0`,
///   bus 07 → `gpu:1`, VRAM à l'appui) ; c'est ce que fait le chargeur Vulkan.
/// - La mesure est prise **une fois, au démarrage**. Une carte occupée à cet
///   instant par autre chose sera écartée à tort — c'est un défaut par défaut,
///   et `RAG3WEAVER_BURN_DEVICE_EMBEDDER` reprend la main.
/// - Une seule carte : rien à choisir, on ne dit rien.
pub fn carte_la_plus_libre(racine: &Path) -> Option<usize> {
    let mut cartes: Vec<(String, u64)> = Vec::new();
    for e in std::fs::read_dir(racine).ok()?.flatten() {
        let nom = e.file_name();
        let nom = nom.to_string_lossy();
        // `card0`, pas `card0-DP-1` : les connecteurs sont des entrées sœurs.
        if !nom.starts_with("card") || nom.contains('-') {
            continue;
        }
        let device = e.path().join("device");
        // Une vraie carte a un compteur d'occupation. Le reste est du décor.
        if !device.join("gpu_busy_percent").exists() {
            continue;
        }
        let vram = std::fs::read_to_string(device.join("mem_info_vram_used"))
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(u64::MAX);
        // L'adresse PCI, pour l'ordre : `device` est un lien vers elle
        // (`../../../0000:04:00.0`). `read_link` et pas `canonicalize` :
        // canonicaliser rendrait « device » pour toutes les cartes, et l'ordre
        // serait décidé par autre chose.
        let adresse = std::fs::read_link(&device)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| nom.to_string());
        cartes.push((adresse, vram));
    }
    if cartes.len() < 2 {
        return None;
    }
    // **Sur l'adresse seule.** Trier les paires ferait de la VRAM un second
    // critère et réordonnerait les cartes selon ce qu'on cherche justement à
    // mesurer.
    cartes.sort_by(|a, b| a.0.cmp(&b.0));
    let mini = cartes.iter().map(|(_, v)| *v).min()?;
    cartes.iter().position(|(_, v)| *v == mini)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_defaut_ne_change_rien() {
        let r = Regime::default();
        assert_eq!(r, Regime::Plein);
        assert_eq!(r.duty(), 100, "100 % : on n'attend pas");
        assert_eq!(r.carte_embedder(), None, "aucune carte imposée");
    }

    #[test]
    fn le_confort_laisse_des_trous_et_raccourcit_les_rafales() {
        let r = Regime::Confort;
        assert_eq!(r.duty(), 60);
        assert!(r.budget_caracteres() < Regime::Plein.budget_caracteres());
    }

    /// Un faux `/sys/class/drm` : des cartes, leur adresse PCI, leur VRAM
    /// occupée en octets.
    fn faux_sysfs(cartes: &[(&str, &str, u64)]) -> tempfile::TempDir {
        let d = tempfile::tempdir().expect("tempdir");
        for (carte, pci, vram) in cartes {
            // Le vrai `device` est un **lien** vers l'adresse PCI : on en
            // pose un, sinon le test ne dirait rien de l'ordre réel.
            let cible = d.path().join(pci);
            std::fs::create_dir_all(&cible).unwrap();
            std::fs::write(cible.join("gpu_busy_percent"), "0\n").unwrap();
            std::fs::write(cible.join("mem_info_vram_used"), format!("{vram}\n")).unwrap();
            std::fs::create_dir_all(d.path().join(carte)).unwrap();
            std::os::unix::fs::symlink(&cible, d.path().join(carte).join("device")).unwrap();
        }
        d
    }

    /// Le cas du poste : la seconde carte est presque vide, la première porte
    /// les tampons du compositeur.
    #[test]
    fn la_moins_chargee_est_celle_qui_a_le_moins_de_vram_prise() {
        let d = faux_sysfs(&[
            ("card0", "0000:04:00.0", 1_950_000_000),
            ("card2", "0000:07:00.0", 90_000_000),
        ]);
        assert_eq!(carte_la_plus_libre(d.path()), Some(1));
    }

    /// **L'index suit l'ordre PCI, pas l'ordre des noms.** `card2` avant
    /// `card10` alphabétiquement, mais c'est le bus qui décide.
    #[test]
    fn l_index_suit_l_adresse_pci() {
        let d = faux_sysfs(&[
            ("card10", "0000:04:00.0", 90_000_000),
            ("card2", "0000:07:00.0", 1_950_000_000),
        ]);
        assert_eq!(carte_la_plus_libre(d.path()), Some(0), "bus 04 est la première");
    }

    /// **Une seule carte : rien à choisir.** Rendre `Some(0)` reviendrait à
    /// imposer la carte du compositeur en croyant bien faire.
    #[test]
    fn une_seule_carte_ne_propose_rien() {
        let d = faux_sysfs(&[("card0", "0000:04:00.0", 1_950_000_000)]);
        assert_eq!(carte_la_plus_libre(d.path()), None);
    }

    #[test]
    fn un_dossier_qui_n_existe_pas_ne_panique_pas() {
        assert_eq!(carte_la_plus_libre(Path::new("/n/existe/pas")), None);
    }

    // ── Les quatre promesses ────────────────────────────────────────────
    //
    // `confort` en tenait trois sur quatre, et rien ne disait que la
    // quatrième manquait. Un régime dont une promesse s'évapore en silence
    // est pire qu'un régime absent : on croit avoir libéré le poste.

    /// **Les quatre, ensemble.** Ce test existe pour qu'une promesse ne puisse
    /// pas disparaître sans que quelque chose casse.
    #[test]
    fn confort_tient_ses_quatre_promesses() {
        let r = Regime::Confort;
        // 1 · le rapport cyclique laisse passer le compositeur
        assert_eq!(r.duty(), 60);
        // 2 · les rafales sont courtes, donc interruptibles souvent
        assert_eq!(r.budget_caracteres(), 2_048);
        // 3 · la carte : on ne peut pas exiger un index sans sysfs, mais on
        //     peut exiger que le régime *demande* à en choisir une — sous
        //     `plein` la réponse est `None` sans même regarder.
        assert!(
            Regime::Plein.carte_locale().is_none(),
            "plein n'impose aucune carte"
        );
        // 4 · l'inférence part au loin, et c'est celle qui manquait
        assert_eq!(
            Origine::voulue(Regime::Confort),
            Origine::Distante,
            "confort n'a pas de quatrième promesse s'il garde l'inférence locale"
        );
    }

    /// **La carte vaut pour les trois rôles, pas pour l'embarqueur seul.**
    /// L'ancien arbitrage — un OCR ne prend la carte que le temps d'un appel —
    /// est un raisonnement d'efficacité, et `confort` parle d'autre chose.
    #[test]
    fn la_carte_ne_distingue_plus_les_roles() {
        // Même source pour tous : `carte_embedder` n'est plus qu'un alias.
        assert_eq!(Regime::Confort.carte_embedder(), Regime::Confort.carte_locale());
        assert_eq!(Regime::Plein.carte_embedder(), Regime::Plein.carte_locale());
    }
}

/// **La précédence de l'origine, isolée de l'environnement réel.**
///
/// Ces tests posent des variables de processus : ils vivent dans leur propre
/// module et s'exécutent en série, sinon deux d'entre eux se marchent dessus
/// sur `RAG3WEAVER_LLM` — un test qui échoue une fois sur trois est pire qu'un
/// test absent, parce qu'on apprend à l'ignorer.
#[cfg(test)]
mod tests_origine {
    use super::*;

    /// Pose les variables, appelle, remet tout comme c'était. Sérialisé par un
    /// verrou de module : `std::env::set_var` est global au processus.
    fn avec(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> Origine) -> Origine {
        static VERROU: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _g = VERROU.lock().unwrap_or_else(|e| e.into_inner());
        let anciens: Vec<(String, Option<String>)> =
            vars.iter().map(|(k, _)| (k.to_string(), std::env::var(k).ok())).collect();
        for (k, v) in vars {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        let r = f();
        for (k, v) in anciens {
            match v {
                Some(v) => std::env::set_var(&k, v),
                None => std::env::remove_var(&k),
            }
        }
        r
    }

    /// **Le cœur de la décision du 3 septembre.** `RAG3WEAVER_LOCAL_LLM` traîne
    /// dans un profil ; elle ne dit pas ce qu'on veut *maintenant*. Si elle
    /// gagnait, `confort` reprendrait la carte qu'il vient de libérer.
    #[test]
    fn sous_confort_une_variable_qui_traine_ne_reprend_pas_la_carte() {
        let o = avec(
            &[("RAG3WEAVER_LLM", None), ("RAG3WEAVER_LOCAL_LLM", Some("http://127.0.0.1:8080/v1"))],
            || Origine::voulue(Regime::Confort),
        );
        assert_eq!(o, Origine::Distante);
    }

    /// Mais une intention posée **pour cette passe** gagne, elle : c'est la
    /// règle du module, appliquée à la bonne variable.
    #[test]
    fn une_intention_explicite_reprend_la_main() {
        let o = avec(
            &[("RAG3WEAVER_LLM", Some("local")), ("RAG3WEAVER_LOCAL_LLM", Some("http://x/v1"))],
            || Origine::voulue(Regime::Confort),
        );
        assert_eq!(o, Origine::Locale, "RAG3WEAVER_LLM=local doit pouvoir reprendre la carte");
    }

    /// Sous `plein`, rien ne change pour qui que ce soit : c'est la condition
    /// pour que ce travail n'ait aucun effet de bord.
    #[test]
    fn sous_plein_la_variable_decide_comme_avant() {
        let avec_locale = avec(
            &[("RAG3WEAVER_LLM", None), ("RAG3WEAVER_LOCAL_LLM", Some("http://x/v1"))],
            || Origine::voulue(Regime::Plein),
        );
        assert_eq!(avec_locale, Origine::Locale);
        let sans = avec(
            &[("RAG3WEAVER_LLM", None), ("RAG3WEAVER_LOCAL_LLM", None)],
            || Origine::voulue(Regime::Plein),
        );
        assert_eq!(sans, Origine::Distante, "sans local, on va au nuage comme avant");
    }

    /// Une valeur inconnue se dit et ne décide pas — la même règle que pour le
    /// régime lui-même.
    #[test]
    fn une_valeur_illisible_ne_decide_rien() {
        let o = avec(
            &[("RAG3WEAVER_LLM", Some("nuageux")), ("RAG3WEAVER_LOCAL_LLM", Some("http://x/v1"))],
            || Origine::voulue(Regime::Plein),
        );
        assert_eq!(o, Origine::Locale, "on retombe sur la règle normale, pas sur un choix arbitraire");
    }
}
