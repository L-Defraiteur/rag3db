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
//! # Ce que ce module ne fait pas encore
//!
//! Le régime `confort` devrait aussi envoyer l'agentique vers un fournisseur
//! distant, pour que l'inférence ne prenne aucune carte locale. Ce n'est
//! **pas** branché : le choix du `Llm` se fait chez l'appelant, et déclarer
//! ici une intention que personne ne lit produirait exactement le genre de
//! mécanisme construit-et-jamais-appelé qu'on passe la semaine à débusquer.
//! Voir l'issue 06.

use std::path::Path;

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

/// La variable qui choisit.
pub const VARIABLE: &str = "RAG3WEAVER_REGIME";

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
    pub fn carte_embedder(self) -> Option<String> {
        match self {
            Self::Plein => None,
            Self::Confort => carte_la_plus_libre(Path::new("/sys/class/drm")).map(|i| format!("gpu:{i}")),
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
}
