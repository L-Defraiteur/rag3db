//! **Par quel modèle on commence** — l'heuristique du premier index.
//!
//! Tranché par Lucie le 7 septembre 2026 : granite-278m par défaut, et
//! granite-107m au premier index si **l'un** de deux déclencheurs :
//!
//! - plus d'environ **50 000 documents** — des fichiers lus, pas des octets ;
//!   elle raisonne en documents. Les octets restent dans la raison écrite,
//!   parce que ce sont eux qui expliquent le temps ;
//! - **une carte faible ou absente** — aucune carte dédiée, ou une VRAM
//!   totale sous 4 Go.
//! - **la seule carte du poste porte l'affichage** (Lucie, 3 octobre 2026) —
//!   278m y tient, mais ses rafales passent devant le compositeur : au
//!   premier index, on prend celui qui libère la carte six fois plus vite.
//!
//! Ce n'est pas « quel modèle pour cet index » : depuis qu'un index porte
//! plusieurs modèles, c'est « par lequel on commence ». Un mauvais choix
//! coûte une passe de rattrapage, pas une réindexation. Voir
//! `docs/7-septembre-2026-22h51/01-l-heuristique-de-taille-de-depot.md`.
//!
//! # Pourquoi 107m n'est jamais « meilleur »
//!
//! Le banc de l'optimiseur (`docs/optimiseur/6-septembre-2026-16h30/04`) :
//! 107m perd 6 à 7 points de MRR et 9 de rappel à 1 face à 278m, et rien en
//! rappel à 5 sur du code commenté. Il est deux fois plus rapide. Il n'y a
//! donc pas de dépôt où il vaut mieux — il y a des dépôts où 278m est trop
//! long à attendre au premier index, et des cartes où son lot ne tient pas.
//!
//! # Le plancher de 4 Go
//!
//! Déduit des mesures de lot (`docs/issues/6-septembre-2026/03`, § flash
//! attention) : le lot conseillé de Granite, 128 séquences de 512 jetons,
//! prend 1,6 Go de scores d'attention — 3,2 Go à 256, refusé — plus ~1,1 Go
//! de poids f32 pour 278m. Sous 4 Go le lot doit rétrécir, et 278m n'est plus
//! deux fois plus lent que 107m mais cinq fois. **Déduit, pas mesuré sur une
//! petite carte** : le premier portable qui passe le dira.
//!
//! # La précédence, la même que partout
//!
//! Le code l'emporte sur la variable, qui l'emporte sur l'heuristique, qui
//! l'emporte sur le défaut. `RAG3WEAVER_EMBED_MODEL` posée gagne toujours ;
//! l'heuristique ne parle que dans le silence.

use serde::{Deserialize, Serialize};

/// Le modèle servi quand rien ne dit le contraire (`bb538cb0d`).
pub const DEFAULT_MODEL: &str = "granite-278m";
/// Le modèle du premier index quand un déclencheur tire.
pub const FAST_MODEL: &str = "granite-107m";
/// La variable qui, posée, gagne sur tout le reste.
pub const MODEL_VARIABLE: &str = "RAG3WEAVER_EMBED_MODEL";

/// Au-delà, le premier index part en 107m. Le seuil de Lucie ; il tombe là
/// où 278m dépasse la dizaine de minutes (~1 ms par chunk, ~12 chunks par
/// fichier sur le cœur), et le noyau Linux — l'exemple qu'elle avait en tête
/// — bascule.
pub const FILES_THRESHOLD: usize = 50_000;
/// En dessous, la carte est « faible » : le lot de 278m n'y tient pas entier.
pub const VRAM_FLOOR_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Ce qu'on sait de la carte, résumé en cinq cas. La lecture de
/// `/sys/class/drm` est ailleurs (`regime::card_class`) : ici c'est un
/// paramètre, donc l'heuristique se teste sans carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardClass {
    /// Une carte dédiée, avec au moins le plancher de VRAM.
    Dedicated { vram_bytes: u64 },
    /// Une carte dédiée sous le plancher.
    Weak { vram_bytes: u64 },
    /// Une carte intégrée, **parce que le pilote le dit**. Sa « VRAM » est une
    /// réservation du BIOS — 512 Mo ici, 4 096 là — et sa mémoire réelle est
    /// celle du système : le plancher ne s'y applique pas.
    Integrated,
    /// Aucune carte dédiée : le calcul partira sur le processeur.
    None,
    /// **Ce n'est pas ce poste qui calcule** : un service d'embarquement est
    /// attaché (`RAG3WEAVER_EMBED_SERVICE`). Les déclencheurs qui parlent de
    /// la carte d'ici — faible, absente, seule à porter l'affichage — n'ont
    /// alors rien à dire ; seul celui de la taille reste.
    Service,
}

impl CardClass {
    /// Classe une carte par sa VRAM totale ; `None` si l'appelant n'en a
    /// trouvé aucune.
    pub fn from_vram(vram_bytes: Option<u64>) -> Self {
        match vram_bytes {
            None => Self::None,
            Some(v) if v < VRAM_FLOOR_BYTES => Self::Weak { vram_bytes: v },
            Some(v) => Self::Dedicated { vram_bytes: v },
        }
    }
}

/// Le choix, et **pourquoi**. La raison est une phrase : un choix qui ne se
/// dit pas se conteste, et dans six mois quelqu'un se demandera pourquoi cet
/// index est en 107m.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub model: String,
    pub reason: String,
}

fn mib(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}
fn gib_1(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// **L'heuristique seule**, sans regarder l'environnement.
///
/// `sole_card_drives_display` : la seule carte du poste porte l'affichage
/// (`regime::sole_card_drives_display`).
pub fn recommended_model(files: usize, source_bytes: u64, card: CardClass, sole_card_drives_display: bool) -> Choice {
    if files > FILES_THRESHOLD {
        return Choice {
            model: FAST_MODEL.into(),
            reason: format!(
                "{files} fichiers ({} Mo de source), au-delà de {FILES_THRESHOLD} : {FAST_MODEL} pour le premier index ; {DEFAULT_MODEL} viendra en retard si on l'exige",
                mib(source_bytes)
            ),
        };
    }
    match card {
        CardClass::Service => Choice {
            model: DEFAULT_MODEL.into(),
            reason: format!(
                "{files} fichiers ({} Mo de source), calcul par un service d'embarquement : {DEFAULT_MODEL}, le meilleur des deux",
                mib(source_bytes)
            ),
        },
        CardClass::None => Choice {
            model: FAST_MODEL.into(),
            reason: format!("aucune carte dédiée, calcul sur le processeur : {FAST_MODEL} pour le premier index"),
        },
        CardClass::Weak { vram_bytes } => Choice {
            model: FAST_MODEL.into(),
            reason: format!(
                "carte de {} Go, sous {} Go : le lot de {DEFAULT_MODEL} n'y tient pas entier, {FAST_MODEL} pour le premier index",
                gib_1(vram_bytes),
                gib_1(VRAM_FLOOR_BYTES)
            ),
        },
        CardClass::Dedicated { .. } | CardClass::Integrated if sole_card_drives_display => Choice {
            model: FAST_MODEL.into(),
            reason: format!(
                "la seule carte du poste porte l'affichage : {FAST_MODEL} pour le premier index, ses rafales libèrent l'écran plus vite ; {DEFAULT_MODEL} viendra en retard si on l'exige"
            ),
        },
        CardClass::Integrated => Choice {
            model: DEFAULT_MODEL.into(),
            reason: format!(
                "{files} fichiers ({} Mo de source), carte intégrée qui n'affiche rien : {DEFAULT_MODEL}, le meilleur des deux",
                mib(source_bytes)
            ),
        },
        CardClass::Dedicated { vram_bytes } => Choice {
            model: DEFAULT_MODEL.into(),
            reason: format!(
                "{files} fichiers ({} Mo de source), carte de {} Go : {DEFAULT_MODEL}, le meilleur des deux",
                mib(source_bytes),
                gib_1(vram_bytes)
            ),
        },
    }
}

/// **Le choix complet** : la variable si elle est posée, l'heuristique sinon.
///
/// `explicit` est la valeur de [`MODEL_VARIABLE`] telle que l'appelant l'a
/// lue — passée en paramètre pour rester testable sans toucher à
/// l'environnement du processus.
pub fn choose(files: usize, source_bytes: u64, card: CardClass, sole_card_drives_display: bool, explicit: Option<&str>) -> Choice {
    match explicit.map(str::trim).filter(|v| !v.is_empty()) {
        Some(model) => Choice {
            model: model.to_string(),
            reason: format!("{MODEL_VARIABLE}={model} posée : elle gagne sur l'heuristique"),
        },
        None => recommended_model(files, source_bytes, card, sole_card_drives_display),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;

    /// Le cœur C++ de rag3db, sur cette machine : rien ne déclenche.
    #[test]
    fn le_coeur_sur_une_vraie_carte_reste_en_278m() {
        let c = recommended_model(1_642, 10 * MIB, CardClass::Dedicated { vram_bytes: 31 * GIB }, false);
        assert_eq!(c.model, DEFAULT_MODEL);
        assert!(c.reason.contains("1642 fichiers") && c.reason.contains("31.0 Go"), "{}", c.reason);
    }

    #[test]
    fn au_dela_de_50_000_documents_le_premier_index_part_en_107m() {
        let c = recommended_model(62_000, 31 * MIB, CardClass::Dedicated { vram_bytes: 31 * GIB }, false);
        assert_eq!(c.model, FAST_MODEL);
        assert!(c.reason.contains("62000 fichiers") && c.reason.contains("au-delà de 50000"), "{}", c.reason);
        // Les octets sont dans la raison : ce sont eux qui expliquent le temps.
        assert!(c.reason.contains("31 Mo"), "{}", c.reason);
        // Le seuil est strict : 50 000 exactement reste en 278m.
        assert_eq!(recommended_model(FILES_THRESHOLD, 0, CardClass::Dedicated { vram_bytes: 31 * GIB }, false).model, DEFAULT_MODEL);
    }

    #[test]
    fn une_carte_faible_ou_absente_declenche_aussi() {
        let faible = recommended_model(1_642, 10 * MIB, CardClass::Weak { vram_bytes: 2 * GIB }, false);
        assert_eq!(faible.model, FAST_MODEL);
        assert!(faible.reason.contains("2.0 Go") && faible.reason.contains("sous 4.0 Go"), "{}", faible.reason);
        let sans = recommended_model(1_642, 10 * MIB, CardClass::None, false);
        assert_eq!(sans.model, FAST_MODEL);
        assert!(sans.reason.contains("aucune carte"), "{}", sans.reason);
    }

    #[test]
    fn la_classe_de_carte_suit_le_plancher() {
        assert_eq!(CardClass::from_vram(None), CardClass::None);
        assert_eq!(CardClass::from_vram(Some(4 * GIB - 1)), CardClass::Weak { vram_bytes: 4 * GIB - 1 });
        assert_eq!(CardClass::from_vram(Some(4 * GIB)), CardClass::Dedicated { vram_bytes: 4 * GIB });
    }

    /// La variable posée gagne sur les deux déclencheurs, et le dit.
    #[test]
    fn la_variable_posee_gagne_sur_l_heuristique() {
        let c = choose(62_000, 31 * MIB, CardClass::None, false, Some("granite-278m"));
        assert_eq!(c.model, "granite-278m");
        assert!(c.reason.contains("RAG3WEAVER_EMBED_MODEL=granite-278m posée"), "{}", c.reason);
        // Vide ou blanche, elle n'est pas posée.
        assert_eq!(choose(1_642, 0, CardClass::Dedicated { vram_bytes: 31 * GIB }, false, Some("  ")).model, DEFAULT_MODEL);
        assert_eq!(choose(1_642, 0, CardClass::Dedicated { vram_bytes: 31 * GIB }, false, None).model, DEFAULT_MODEL);
    }

    /// Le troisième déclencheur : le Z13, une carte intégrée et deux écrans.
    #[test]
    fn la_seule_carte_qui_porte_l_affichage_declenche_107m() {
        let z13 = recommended_model(1_642, 10 * MIB, CardClass::Integrated, true);
        assert_eq!(z13.model, FAST_MODEL);
        assert!(z13.reason.contains("la seule carte du poste porte l'affichage"), "{}", z13.reason);
        // Une tour à une seule carte dédiée, écran dessus : le même cas.
        assert_eq!(recommended_model(1_642, 10 * MIB, CardClass::Dedicated { vram_bytes: 16 * GIB }, true).model, FAST_MODEL);
        // Une carte intégrée qui n'affiche rien n'est pas « faible » : sa
        // VRAM annoncée ne dit rien, on ne la juge pas dessus.
        let serveur = recommended_model(1_642, 10 * MIB, CardClass::Integrated, false);
        assert_eq!(serveur.model, DEFAULT_MODEL);
        assert!(serveur.reason.contains("carte intégrée"), "{}", serveur.reason);
        // La variable posée gagne, ici comme ailleurs.
        assert_eq!(choose(1_642, 0, CardClass::Integrated, true, Some("granite-278m")).model, "granite-278m");
    }
}
