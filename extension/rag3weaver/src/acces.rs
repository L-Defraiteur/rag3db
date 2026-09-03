//! **Par où un lecteur atteint la base.**
//!
//! Une base embarquée se tient par un seul processus : c'est pour ça que
//! `rag3daemon` existe — mettre le processus qui tient le verrou derrière une
//! adresse. Depuis le report de Vela sur le cœur, un lecteur peut aussi
//! **ouvrir lui-même** en lecture seule pendant qu'un écrivain travaille, ce
//! qui rend le relais facultatif pour lire.
//!
//! Facultatif, pas caduc : la bibliothèque liée peut être antérieure au report,
//! auquel cas le direct est refusé. Il faut donc **choisir**, et le choix est
//! exactement là où les défauts se cachent.
//!
//! # La règle : jamais de repli silencieux
//!
//! Ni vers le démon, ni vers le direct. Un lecteur qui croit lire la base en
//! direct alors qu'il passe par un relais — ou l'inverse — ne peut rien
//! diagnostiquer, et c'est la famille de défaut qu'on passe nos journées à
//! sortir : le silence, pas l'erreur.
//!
//! D'où la même forme que [`crate::search_backend::MoteurTexte`], qui a déjà
//! fait ses preuves : trois valeurs, dont une qui décide et **dit ce qu'elle a
//! décidé**.

use std::path::Path;

use crate::connection::DbConnection;
use crate::daemon::DaemonConnection;
use crate::rag3db_connection::Rag3dbConnection;
use crate::serveur::Serveur;

/// Ce qu'un lecteur demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Acces {
    /// Le lecteur ouvre lui-même, en lecture seule.
    ///
    /// Un refus est une **erreur nommée**, jamais un détour muet par le relais.
    /// C'est le pendant de `MoteurTexte::Natif` : forcer, et échouer clairement
    /// si ce n'est pas possible.
    Direct,
    /// Le relais `rag3daemon`, comme avant le report de Vela.
    Demon,
    /// Le direct s'il est possible, le relais sinon — **et on le dit**.
    #[default]
    Auto,
}

/// Par où on est effectivement passé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chemin {
    Direct,
    Demon,
}

/// Une connexion de lecture, et l'histoire de son ouverture.
pub struct Lecteur {
    pub conn: Box<dyn DbConnection>,
    /// Par où, **en clair**. Un appelant qui mesure des latences ou lit un
    /// journal doit pouvoir le savoir sans le deviner.
    pub par: Chemin,
    /// Pourquoi on n'est pas passé en direct, quand c'est le cas.
    ///
    /// Vide sur `Direct` et sur un `Auto` qui a réussi du premier coup. Non
    /// vide, il porte la raison du repli — c'est ce qui empêche `Auto` d'être
    /// un silence.
    pub avertissements: Vec<String>,
}

/// **Ouvrir un lecteur, en disant par où.**
///
/// `serveur` décrit le `rag3daemon` à joindre ou à lancer ; il n'est consulté
/// que si le chemin retenu est le relais. `None` avec `Acces::Demon` est une
/// erreur, pas un repli.
pub fn ouvrir_lecteur(
    base: &Path,
    acces: Acces,
    serveur: Option<&Serveur>,
) -> Result<Lecteur, String> {
    let direct = |avertissements: Vec<String>| -> Result<Lecteur, String> {
        Rag3dbConnection::read_only(base)
            .map(|c| Lecteur { conn: Box::new(c), par: Chemin::Direct, avertissements })
            .map_err(|e| e.to_string())
    };
    let relais = |avertissements: Vec<String>| -> Result<Lecteur, String> {
        let Some(s) = serveur else {
            return Err(
                "accès par le relais demandé sans description de serveur : rien à \
                 joindre ni à lancer"
                    .to_string(),
            );
        };
        DaemonConnection::assurer(s)
            .map(|c| Lecteur { conn: Box::new(c), par: Chemin::Demon, avertissements })
            .map_err(|e| e.to_string())
    };

    match acces {
        Acces::Direct => direct(vec![]).map_err(|e| {
            format!(
                "accès direct demandé et refusé : {e}. La bibliothèque liée est \
                 peut-être antérieure au report qui autorise un lecteur pendant \
                 qu'un écrivain tient la base ; `Acces::Auto` se rabattrait sur le \
                 relais en le disant."
            )
        }),
        Acces::Demon => relais(vec![]),
        Acces::Auto => match direct(vec![]) {
            Ok(l) => Ok(l),
            Err(refus) => {
                // **Le repli se dit.** Sans cette phrase, un lecteur croirait
                // lire la base en direct alors qu'il passe par un relais — et
                // ne pourrait diagnostiquer ni sa latence ni sa fraîcheur.
                let dit = format!(
                    "accès direct refusé ({refus}) — repli sur le relais rag3daemon"
                );
                relais(vec![dit.clone()]).map_err(|e| {
                    format!("ni en direct ni par le relais. Direct : {refus}. Relais : {e}")
                })
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absent() -> std::path::PathBuf {
        std::env::temp_dir().join("rag3weaver-acces-base-absente")
    }

    /// **`Direct` ne se rabat pas**, et son erreur nomme la cause probable
    /// plutôt que de rendre le message brut du cœur C++.
    #[test]
    fn direct_refuse_ne_se_rabat_pas() {
        let _ = std::fs::remove_dir_all(absent());
        let e = ouvrir_lecteur(&absent(), Acces::Direct, None)
            .err()
            .expect("une base absente ne s'ouvre pas");
        assert!(e.contains("accès direct demandé et refusé"), "{e}");
        assert!(
            e.contains("Acces::Auto"),
            "l'erreur doit dire ce qu'un appelant peut faire d'autre : {e}"
        );
    }

    /// **`Demon` sans serveur est une erreur**, pas un repli sur le direct.
    /// L'inverse rendrait `Acces` décoratif.
    #[test]
    fn le_relais_sans_serveur_est_une_erreur() {
        let e = ouvrir_lecteur(&absent(), Acces::Demon, None)
            .err()
            .expect("pas de serveur, pas de relais");
        assert!(e.contains("sans description de serveur"), "{e}");
    }

    /// **`Auto` sans relais possible porte les deux raisons.** Une seule
    /// laisserait chercher du mauvais côté.
    #[test]
    fn auto_sans_relais_dit_les_deux_raisons() {
        let _ = std::fs::remove_dir_all(absent());
        let e = ouvrir_lecteur(&absent(), Acces::Auto, None)
            .err()
            .expect("ni l'un ni l'autre");
        assert!(e.contains("Direct :"), "{e}");
        assert!(e.contains("Relais :"), "{e}");
    }

    /// Le défaut est `Auto` : un appelant qui ne dit rien obtient le chemin qui
    /// s'explique, pas celui qui se tait.
    #[test]
    fn le_defaut_est_auto() {
        assert_eq!(Acces::default(), Acces::Auto);
    }
}
