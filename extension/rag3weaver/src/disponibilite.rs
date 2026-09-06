//! **Ce qui doit être prêt** — le vocabulaire commun de l'écriture et de la
//! lecture.
//!
//! Lucie, le 5 septembre 2026 :
//!
//! > on peut autant à la lecture qu'à l'écriture attendre « data / textsearch /
//! > sparse / dense ».
//!
//! # Un ensemble, pas un niveau
//!
//! `Donnee` précède tout — on n'indexe pas ce qui n'est pas posé. Mais
//! `PleinTexte`, `Sparse` et `Dense` sont des **frères parallèles** : ils se
//! calculent et se commitent séparément, et rien n'oblige à les attendre
//! ensemble. Qui ne veut que du plein texte n'a pas à attendre l'embarquement
//! GPU, et c'est précisément ce qu'il faisait avant.
//!
//! D'où un **ensemble** et non un entier : on n'attend pas « jusqu'au niveau
//! N », on attend **les signaux qu'on nomme**.
//!
//! # Les deux bouts du même axe
//!
//! | | qui parle | ce qu'il dit |
//! |---|---|---|
//! | écriture | l'écrivain | *ce qu'il a rendu prêt* |
//! | lecture | le lecteur | *ce dont il a besoin d'être prêt* |
//!
//! C'est le même vocabulaire des deux côtés, et c'est voulu : la marque
//! d'ingestion qui traverse la frontière du processus doit pouvoir dire
//! *jusqu'où* un écrivain est allé, pas seulement qu'il a « du travail ».
//!
//! # Ce que le moteur sait tenir aujourd'hui, et ce qu'il approxime
//!
//! **La garantie est conservatrice : on ne dit jamais « prêt » quand ça ne
//! l'est pas.** En revanche on attend parfois plus que demandé, et il faut le
//! savoir :
//!
//! | demandé | ce qui se passe | exact ? |
//! |---|---|---|
//! | rien | rien | **oui** |
//! | `Donnee` | les entités en file sont posées (`flush_insertions`) | non — relations et agrégats peuvent rester |
//! | `PleinTexte` | le graphe est drainé **sans l'étage GPU** | **oui** |
//! | `Sparse` ou `Dense` | le graphe entier, étage GPU compris | non — les deux signaux GPU partent ensemble |
//!
//! La ligne `PleinTexte` est devenue exacte le 6 septembre 2026 : les nœuds
//! d'embarquement sont des **feuilles** du graphe de drain, donc les omettre ne
//! déséquilibre rien. Ce qui n'est pas embarqué devient une **dette dans la
//! base** — chunks dont `_embed_hash` ou `_sparse_hash` est vide — et non un
//! état en mémoire : elle survit à un processus qui meurt, elle s'interroge
//! (`SchemaDialect::count_marqueur_manquant`), et une recherche qui bute dessus
//! le dit au lieu de rendre zéro en silence.
//!
//! Les deux approximations restantes vont dans le sens sûr. Celle de `Donnee`
//! se resserrera quand `flush_insertions` saura poser aussi les relations ;
//! celle des deux signaux GPU, quand les nœuds d'embarquement sauront n'en
//! calculer qu'un — le schéma v3 sait déjà les distinguer par leurs marqueurs.

use serde::{Deserialize, Serialize};

/// Les quatre disponibilités, comme un ensemble de drapeaux.
///
/// Se combine avec `|` : `Disponibilites::DONNEE | Disponibilites::PLEIN_TEXTE`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Disponibilites(u8);

impl Disponibilites {
    /// N'attendre rien. Ce que le moteur voit, il le rend.
    pub const AUCUNE: Self = Self(0);
    /// Les lignes sont en base et lisibles.
    pub const DONNEE: Self = Self(0b0001);
    /// Le plein texte les trouve.
    pub const PLEIN_TEXTE: Self = Self(0b0010);
    /// L'index sparse les trouve.
    pub const SPARSE: Self = Self(0b0100);
    /// L'index vectoriel dense les trouve.
    pub const DENSE: Self = Self(0b1000);

    /// Tout, y compris ce qui coûte une passe GPU.
    pub const TOUT: Self = Self(0b1111);
    /// Ce qu'une recherche par mots-clés exige, et rien de plus.
    pub const RECHERCHE_TEXTE: Self = Self(0b0011);

    pub const fn donnee(self) -> bool { self.0 & 0b0001 != 0 }
    pub const fn plein_texte(self) -> bool { self.0 & 0b0010 != 0 }
    pub const fn sparse(self) -> bool { self.0 & 0b0100 != 0 }
    pub const fn dense(self) -> bool { self.0 & 0b1000 != 0 }

    pub const fn est_vide(self) -> bool { self.0 == 0 }

    /// Y a-t-il autre chose que la donnée là-dedans ?
    ///
    /// C'est la question qui décide aujourd'hui entre « poser les entités » et
    /// « drainer tout le graphe » — voir la table d'approximation en tête de
    /// module.
    pub const fn exige_un_derive(self) -> bool { self.0 & 0b1110 != 0 }

    /// `true` si `self` contient tout ce que `autre` demande.
    pub const fn contient(self, autre: Self) -> bool {
        self.0 & autre.0 == autre.0
    }

    /// Ce que `autre` demande et que `self` n'a pas.
    pub const fn manque(self, autre: Self) -> Self {
        Self(autre.0 & !self.0)
    }

    /// Les noms, pour un message lisible par un humain ou un agent.
    pub fn noms(self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.donnee() { v.push("data"); }
        if self.plein_texte() { v.push("textsearch"); }
        if self.sparse() { v.push("sparse"); }
        if self.dense() { v.push("dense"); }
        v
    }

    /// Lit un ensemble écrit en toutes lettres : `"data"`,
    /// `"data,textsearch"`, `"tout"`, `"rien"`.
    ///
    /// **Un nom inconnu est refusé**, avec la liste des noms admis. Retomber en
    /// silence sur un défaut ferait croire à une attente qui n'aurait pas lieu
    /// — c'est le défaut que tout ce module existe pour supprimer.
    pub fn depuis_liste(texte: &str) -> Result<Self, String> {
        let mut acc = Self::AUCUNE;
        for brut in texte.split(',') {
            let mot = brut.trim().to_ascii_lowercase();
            if mot.is_empty() {
                continue;
            }
            acc |= match mot.as_str() {
                "rien" | "aucune" | "none" => Self::AUCUNE,
                "data" | "donnee" | "données" | "donnée" => Self::DONNEE,
                "textsearch" | "plein_texte" | "fulltext" => Self::PLEIN_TEXTE,
                "sparse" => Self::SPARSE,
                "dense" | "vector" => Self::DENSE,
                "tout" | "all" => Self::TOUT,
                autre => {
                    return Err(format!(
                        "disponibilité inconnue « {autre} » — attendu : rien, data, \
                         textsearch, sparse, dense, tout (séparées par des virgules)"
                    ))
                }
            };
        }
        Ok(acc)
    }
}

impl std::ops::BitOr for Disponibilites {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

impl std::ops::BitOrAssign for Disponibilites {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl std::fmt::Debug for Disponibilites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let noms = self.noms();
        if noms.is_empty() {
            write!(f, "Disponibilites(rien)")
        } else {
            write!(f, "Disponibilites({})", noms.join("+"))
        }
    }
}

impl std::fmt::Display for Disponibilites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let noms = self.noms();
        if noms.is_empty() {
            write!(f, "rien")
        } else {
            write!(f, "{}", noms.join(", "))
        }
    }
}

/// Sérialisée comme la liste de ses noms — `["data","textsearch"]` — et non
/// comme un entier : un nombre dans un JSON de configuration ne se relit pas.
impl Serialize for Disponibilites {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.noms().serialize(s)
    }
}

impl<'de> Deserialize<'de> for Disponibilites {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Forme {
            Liste(Vec<String>),
            Texte(String),
        }
        let brut = Forme::deserialize(d)?;
        let texte = match brut {
            Forme::Liste(v) => v.join(","),
            Forme::Texte(t) => t,
        };
        Self::depuis_liste(&texte).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_ensemble_se_compose_et_se_lit() {
        let d = Disponibilites::DONNEE | Disponibilites::PLEIN_TEXTE;
        assert!(d.donnee() && d.plein_texte());
        assert!(!d.dense() && !d.sparse());
        assert_eq!(d, Disponibilites::RECHERCHE_TEXTE);
        assert_eq!(d.to_string(), "data, textsearch");
    }

    /// `contient` est la question du lecteur : « ce qui est prêt couvre-t-il ce
    /// dont j'ai besoin ? ». Elle n'est pas symétrique, et c'est le point.
    #[test]
    fn contenir_n_est_pas_egaler() {
        let pret = Disponibilites::DONNEE | Disponibilites::PLEIN_TEXTE;
        assert!(pret.contient(Disponibilites::DONNEE));
        assert!(!pret.contient(Disponibilites::DENSE));
        assert!(Disponibilites::TOUT.contient(pret));
        assert!(!pret.contient(Disponibilites::TOUT));
    }

    /// `manque` sert aux messages : dire *ce qui* n'est pas prêt, pas seulement
    /// que quelque chose ne l'est pas.
    #[test]
    fn ce_qui_manque_se_nomme() {
        let pret = Disponibilites::DONNEE;
        let manque = pret.manque(Disponibilites::TOUT);
        assert_eq!(manque.noms(), vec!["textsearch", "sparse", "dense"]);
    }

    #[test]
    fn la_donnee_seule_n_exige_aucun_derive() {
        assert!(!Disponibilites::DONNEE.exige_un_derive());
        assert!(!Disponibilites::AUCUNE.exige_un_derive());
        assert!(Disponibilites::RECHERCHE_TEXTE.exige_un_derive());
        assert!(Disponibilites::DENSE.exige_un_derive());
    }

    #[test]
    fn une_liste_se_lit_dans_les_deux_langues() {
        assert_eq!(
            Disponibilites::depuis_liste("data, textsearch").unwrap(),
            Disponibilites::RECHERCHE_TEXTE
        );
        assert_eq!(
            Disponibilites::depuis_liste("donnee,plein_texte").unwrap(),
            Disponibilites::RECHERCHE_TEXTE
        );
        assert_eq!(Disponibilites::depuis_liste("tout").unwrap(), Disponibilites::TOUT);
        assert_eq!(Disponibilites::depuis_liste("rien").unwrap(), Disponibilites::AUCUNE);
        assert_eq!(Disponibilites::depuis_liste("").unwrap(), Disponibilites::AUCUNE);
    }

    /// Un nom mal tapé est **refusé et nommé**. S'il retombait sur le défaut,
    /// l'appelant croirait attendre et n'attendrait rien — le mensonge exact
    /// que ce module supprime.
    #[test]
    fn un_nom_inconnu_est_refuse_et_nomme() {
        let e = Disponibilites::depuis_liste("data,dnese").unwrap_err();
        assert!(e.contains("dnese"), "{e}");
        assert!(e.contains("dense"), "la liste des noms admis doit être là : {e}");
    }

    /// Elle voyage en **noms**, pas en entier : une configuration se relit.
    #[test]
    fn la_serialisation_garde_les_noms() {
        let d = Disponibilites::DONNEE | Disponibilites::DENSE;
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#"["data","dense"]"#);
        let relu: Disponibilites = serde_json::from_str(&json).unwrap();
        assert_eq!(relu, d);
        // Et une chaîne simple se relit aussi : c'est ce qu'une fiche d'outil
        // substitue.
        let depuis_texte: Disponibilites = serde_json::from_str(r#""data,dense""#).unwrap();
        assert_eq!(depuis_texte, d);
    }
}
