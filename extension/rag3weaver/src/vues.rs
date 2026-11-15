//! **Les vues** (2b′) : une vue se nomme `(couche, rang)`, et le chargeur lit sa chaîne de générations dans
//! `_generation` (la génération, sa parente épinglée, et ainsi de suite jusqu'à la racine) pour en faire la
//! **pile de lecture** : une description, sans aucune occurrence chargée. La composition se fait en
//! prédicat aux points de lecture d'aujourd'hui. Page :
//! `docs/arbre-principal/15-11-2026/01-2b-prime-le-chargeur-qui-compose-la-pile.md`.

use std::collections::BTreeSet;

use serde_json::Value;

/// Une vue demandée : la couche (son id stable) et le rang ; sans rang, la dernière génération publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VueDemandee {
    pub couche: String,
    pub rang: Option<i64>,
}

/// Un étage de la pile : une génération, et ce qu'elle retire de ses parentes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etage {
    pub couche: String,
    pub rang: i64,
    /// Les clés d'occurrence retirées par cet étage.
    pub cles_retirees: BTreeSet<String>,
    /// Les fichiers retirés par cet étage.
    pub fichiers_retires: BTreeSet<String>,
}

/// La pile de lecture d'une vue : la vue d'abord, la racine en dernier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PileDeLecture {
    pub etages: Vec<Etage>,
    /// La racine n'avait aucune parente écrite (une génération d'avant 2b′, ou une vraie racine) : dit.
    pub racine_sans_parente: bool,
}

/// Les réglages des vues, sous `tracked.views` du manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReglagesDesVues {
    /// La profondeur au-delà de laquelle une pile se refuse.
    pub max_depth: usize,
    /// Le temps qu'un chargement attend la table `_generation` sous une lecture.
    pub load_timeout_ms: u64,
}

impl Default for ReglagesDesVues {
    fn default() -> Self {
        ReglagesDesVues { max_depth: 16, load_timeout_ms: 2000 }
    }
}

/// **Les réglages des vues** depuis `tracked` (la clé `views`) ; absents, les défauts. Toute clé inconnue
/// est un refus qui la nomme.
pub fn reglages_des_vues(tracked: Option<&Value>) -> Result<ReglagesDesVues, String> {
    let _ = tracked;
    Err("les réglages des vues ne se lisent pas encore".into())
}

/// **Charger la pile d'une vue** depuis `_generation`. Refus nommés : couche inconnue, rang absent, parente
/// manquante, cycle, profondeur au-delà du réglage.
pub fn charger_la_pile(catalog: &crate::catalog::Catalog, vue: &VueDemandee, reglages: &ReglagesDesVues) -> Result<PileDeLecture, String> {
    let _ = (catalog, vue, reglages);
    Err("le chargeur des vues n'existe pas encore".into())
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;
    use serde_json::json;

    fn catalogue() -> crate::Catalog {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = crate::Catalog::without_embedding_model(Box::new(conn), crate::CatalogConfig::default());
        cat.initialize().unwrap();
        let ddl = cat.dialect_arc().create_generation_table().expect("le dialecte sait les générations");
        cat.execute_raw(&ddl).unwrap();
        cat
    }

    fn poser(cat: &crate::Catalog, couche: &str, rang: i64, parente: Option<(&str, i64)>, tombstones: &str) {
        let (pc, pr) = match parente {
            Some((c, r)) => (format!("'{c}'"), r.to_string()),
            None => ("NULL".to_string(), "NULL".to_string()),
        };
        let t = tombstones.replace('\'', "\\'");
        cat.execute_raw(&format!(
            "CREATE (:_generation {{_key: '{couche}:{rang}', couche: '{couche}', rang: {rang}, commit: '', base: '', recette: '', \
             parente_couche: {pc}, parente_rang: {pr}, tombstones: '{t}', publiee_le: 0}})"
        ))
        .unwrap();
    }

    fn vue(couche: &str, rang: Option<i64>) -> VueDemandee {
        VueDemandee { couche: couche.into(), rang }
    }

    fn etage(couche: &str, rang: i64, cles: &[&str], fichiers: &[&str]) -> Etage {
        Etage {
            couche: couche.into(),
            rang,
            cles_retirees: cles.iter().map(|s| s.to_string()).collect(),
            fichiers_retires: fichiers.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// **Une pile d'un étage** : la génération 0 de `backend`, sans parente écrite, se relit comme racine ;
    /// un rang omis prend la dernière génération publiée.
    #[test]
    fn une_pile_d_un_etage_et_sa_racine_sans_parente() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        let p = charger_la_pile(&cat, &vue("backend", None), &ReglagesDesVues::default()).unwrap();
        assert_eq!(p, PileDeLecture { etages: vec![etage("backend", 0, &[], &[])], racine_sans_parente: true });
        assert_eq!(charger_la_pile(&cat, &vue("backend", Some(0)), &ReglagesDesVues::default()).unwrap(), p);
    }

    /// **Une pile de deux étages** : l'enfant sur sa parente épinglée, ses tombstones lus ; un rang omis
    /// prend le dernier rang de l'enfant.
    #[test]
    fn une_pile_de_deux_etages_avec_ses_tombstones() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "backend", 1, None, "[]");
        poser(&cat, "enfant", 1, Some(("backend", 0)), r#"{"keys": ["k-f"], "files": ["src/vieux.rs"]}"#);
        poser(&cat, "enfant", 2, Some(("backend", 0)), r#"{"keys": ["k-f", "k-g"], "files": []}"#);
        let p = charger_la_pile(&cat, &vue("enfant", None), &ReglagesDesVues::default()).unwrap();
        assert_eq!(
            p,
            PileDeLecture { etages: vec![etage("enfant", 2, &["k-f", "k-g"], &[]), etage("backend", 0, &[], &[])], racine_sans_parente: true }
        );
        let p1 = charger_la_pile(&cat, &vue("enfant", Some(1)), &ReglagesDesVues::default()).unwrap();
        assert_eq!(p1.etages[0], etage("enfant", 1, &["k-f"], &["src/vieux.rs"]));
        assert_eq!(p1.etages[1], etage("backend", 0, &[], &[]), "la parente reste épinglée à son rang, pas au dernier");
    }

    /// **Les refus nommés** : une couche inconnue, un rang absent, une parente manquante, un cycle, une
    /// profondeur au-delà du réglage, des tombstones illisibles.
    #[test]
    fn les_refus_du_chargeur_se_nomment() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "orpheline", 1, Some(("disparue", 3)), "[]");
        poser(&cat, "a", 1, Some(("b", 1)), "[]");
        poser(&cat, "b", 1, Some(("a", 1)), "[]");
        poser(&cat, "c1", 1, Some(("backend", 0)), "[]");
        poser(&cat, "c2", 1, Some(("c1", 1)), "[]");
        poser(&cat, "illisible", 1, None, "pas du json");
        let r = ReglagesDesVues::default();
        for (v, mot) in [
            (vue("inconnue", None), "inconnue"),
            (vue("backend", Some(7)), "7"),
            (vue("orpheline", None), "disparue:3"),
            (vue("a", None), "cycle"),
            (vue("illisible", None), "tombstones"),
        ] {
            let refus = charger_la_pile(&cat, &v, &r).expect_err(&format!("{v:?} se charge"));
            assert!(refus.contains(mot), "le refus de {v:?} ne nomme pas « {mot} » : {refus}");
        }
        let court = ReglagesDesVues { max_depth: 2, ..r };
        let refus = charger_la_pile(&cat, &vue("c2", None), &court).expect_err("trois étages sous une profondeur de 2");
        assert!(refus.contains("profondeur"), "{refus}");
        assert!(charger_la_pile(&cat, &vue("c2", None), &ReglagesDesVues::default()).is_ok());
    }

    /// **Les réglages des vues** : les défauts sans `views`, les valeurs déclarées, et une clé inconnue
    /// refusée en la nommant ; `tracked` accepte la clé `views`.
    #[test]
    fn les_reglages_des_vues_et_leurs_refus() {
        assert_eq!(reglages_des_vues(None).unwrap(), ReglagesDesVues::default());
        assert_eq!(reglages_des_vues(Some(&json!({"sources": []}))).unwrap(), ReglagesDesVues::default());
        assert_eq!(
            reglages_des_vues(Some(&json!({"views": {"max_depth": 4, "load_timeout_ms": 500}}))).unwrap(),
            ReglagesDesVues { max_depth: 4, load_timeout_ms: 500 }
        );
        for (mauvais, mot) in [(json!({"views": {"profondeur": 4}}), "profondeur"), (json!({"views": {"max_depth": 0}}), "max_depth"), (json!({"views": 3}), "views")] {
            let refus = reglages_des_vues(Some(&mauvais)).expect_err(&format!("{mauvais} est accepté"));
            assert!(refus.contains(mot), "le refus de {mauvais} ne nomme pas « {mot} » : {refus}");
        }
        crate::suivies::declaration_du_manifeste(Some(&json!({"views": {"max_depth": 4}, "sources": []})))
            .expect("tracked refuse la clé views");
    }
}
