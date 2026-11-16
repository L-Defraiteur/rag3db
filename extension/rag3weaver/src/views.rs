//! **Les vues** (2b′) : une vue se nomme `(couche, rang)`, et le chargeur lit sa chaîne de générations dans
//! `_generation` (la génération, sa parente épinglée, et ainsi de suite jusqu'à la racine) pour en faire la
//! **pile de lecture** : une description, sans aucune occurrence chargée. La composition se fait en
//! prédicat aux points de lecture d'aujourd'hui. Page :
//! `docs/arbre-principal/15-11-2026/01-2b-prime-le-chargeur-qui-compose-la-pile.md`.

use std::collections::BTreeSet;

use serde_json::Value;

/// Une vue demandée : la couche (son id stable) et le rang ; sans rang, la dernière génération publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRequest {
    pub layer: String,
    pub rank: Option<i64>,
}

/// Un étage de la pile : une génération, et ce qu'elle retire de ses parentes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackLayer {
    pub layer: String,
    pub rank: i64,
    /// Les clés d'occurrence retirées par cet étage.
    pub removed_keys: BTreeSet<String>,
    /// Les fichiers retirés par cet étage.
    pub removed_files: BTreeSet<String>,
}

/// La pile de lecture d'une vue : la vue d'abord, la racine en dernier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadStack {
    pub layers: Vec<StackLayer>,
    /// La racine n'avait aucune parente écrite (une génération d'avant 2b′, ou une vraie racine) : dit.
    pub root_without_parent: bool,
}

/// Les réglages des vues, sous `tracked.views` du manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSettings {
    /// La profondeur au-delà de laquelle une pile se refuse.
    pub max_depth: usize,
    /// Le temps qu'un chargement attend la table `_generation` sous une lecture.
    pub load_timeout_ms: u64,
}

impl Default for ViewSettings {
    fn default() -> Self {
        ViewSettings { max_depth: 16, load_timeout_ms: 2000 }
    }
}

/// **Les réglages des vues** depuis `tracked` (la clé `views`) ; absents, les défauts. Toute clé inconnue
/// est un refus qui la nomme.
pub fn view_settings(tracked: Option<&Value>) -> Result<ViewSettings, String> {
    let mut r = ViewSettings::default();
    let Some(vues) = tracked.and_then(|t| t.get("views")) else { return Ok(r) };
    let objet = vues.as_object().ok_or_else(|| format!("tracked.views : un objet {{max_depth?, load_timeout_ms?}} attendu, pas {vues}"))?;
    for (cle, v) in objet {
        let entier = v.as_u64().filter(|n| *n > 0);
        match (cle.as_str(), entier) {
            ("max_depth", Some(n)) => r.max_depth = n as usize,
            ("load_timeout_ms", Some(n)) => r.load_timeout_ms = n,
            ("max_depth" | "load_timeout_ms", None) => return Err(format!("tracked.views.{cle} : un entier positif attendu, pas {v}")),
            _ => return Err(format!("tracked.views : clé inconnue « {cle} » (max_depth, load_timeout_ms)")),
        }
    }
    Ok(r)
}

/// Les tombstones d'une génération : `[]` ou une liste de clés (l'écriture de la génération 0), ou
/// `{"keys": [...], "files": [...]}`.
fn read_tombstones(couche: &str, rang: i64, texte: &str) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let refus = || format!("vue {couche}:{rang} : tombstones illisibles ({texte})");
    let liste = |v: Option<&Value>| -> Result<BTreeSet<String>, String> {
        match v {
            None => Ok(BTreeSet::new()),
            Some(Value::Array(a)) => a.iter().map(|x| x.as_str().map(str::to_string).ok_or_else(refus)).collect(),
            Some(_) => Err(refus()),
        }
    };
    let v: Value = serde_json::from_str(texte).map_err(|_| refus())?;
    match &v {
        Value::Array(_) => Ok((liste(Some(&v))?, BTreeSet::new())),
        Value::Object(o) if o.keys().all(|k| k == "keys" || k == "files") => Ok((liste(o.get("keys"))?, liste(o.get("files"))?)),
        _ => Err(refus()),
    }
}

/// **Charger la pile d'une vue** depuis `_generation`. Refus nommés : couche inconnue, rang absent, parente
/// manquante, cycle, profondeur au-delà du réglage.
pub fn load_stack(catalog: &crate::catalog::Catalog, vue: &ViewRequest, reglages: &ViewSettings) -> Result<ReadStack, String> {
    catalog.check_reopen().map_err(|e| format!("vues : {e}"))?;
    load_stack_on(catalog.conn(), catalog.dialect_arc().as_ref(), vue, reglages)
}

/// **Le chargeur sur une connexion** : celle de l'instantané d'un appel d'outil, pour que la vue et
/// ses lignes viennent de la même lecture, sans le mutex du catalogue.
pub fn load_stack_on(
    conn: &dyn crate::connection::DbConnection,
    dialecte: &dyn crate::dialect::SchemaDialect,
    vue: &ViewRequest,
    reglages: &ViewSettings,
) -> Result<ReadStack, String> {
    use crate::connection::{CypherValue, QueryParam};
    let (Some(lire), Some(dernier)) = (dialecte.read_generation(), dialecte.latest_generation_rank()) else {
        return Err("vues : ce dialecte n'a pas de générations".into());
    };
    let debut = std::time::Instant::now();
    let texte = |nom: &str, v: &str| QueryParam::new(nom, CypherValue::String(v.to_string()));
    let executer = |cypher: &str, params: &[QueryParam]| conn.execute_with_params(cypher, params).map_err(|e| format!("vues : lecture de _generation : {e}"));
    let rang = match vue.rank {
        Some(r) => r,
        None => {
            let lu = executer(&dernier, &[texte("couche", &vue.layer)])?;
            match lu.rows.first().and_then(|r| r.first()) {
                Some(CypherValue::Int(r)) => *r,
                _ => return Err(format!("vue « {} » : couche inconnue (couche sans génération, 2a′-3)", vue.layer)),
            }
        }
    };
    let mut etages: Vec<StackLayer> = Vec::new();
    let mut vus = BTreeSet::new();
    let (mut couche, mut rang) = (vue.layer.clone(), rang);
    loop {
        if !vus.insert((couche.clone(), rang)) {
            return Err(format!("vue {} : cycle de parentes en {couche}:{rang}", vue.layer));
        }
        if etages.len() >= reglages.max_depth {
            return Err(format!("vue {} : profondeur au-delà de {} étages (tracked.views.max_depth)", vue.layer, reglages.max_depth));
        }
        if debut.elapsed().as_millis() as u64 > reglages.load_timeout_ms {
            return Err(format!("vue {} : chargement au-delà de {} ms (tracked.views.load_timeout_ms)", vue.layer, reglages.load_timeout_ms));
        }
        let lu = executer(&lire, &[texte("couche", &couche), QueryParam::new("rang", CypherValue::Int(rang))])?;
        let Some(ligne) = lu.rows.first() else {
            return Err(match etages.last() {
                None if vue.rank.is_some() => format!("vue {couche} : rang {rang} absent de _generation"),
                None => format!("vue « {couche} » : couche inconnue"),
                Some(enfant) => format!("vue {} : parente manquante {couche}:{rang} (de {}:{})", vue.layer, enfant.layer, enfant.rank),
            });
        };
        let tombstones = match ligne.get(2) {
            Some(CypherValue::String(t)) => t.clone(),
            _ => "[]".to_string(),
        };
        let (removed_keys, removed_files) = read_tombstones(&couche, rang, &tombstones)?;
        etages.push(StackLayer { layer: couche.clone(), rank: rang, removed_keys, removed_files });
        match (ligne.first(), ligne.get(1)) {
            (Some(CypherValue::String(pc)), Some(CypherValue::Int(pr))) if !pc.is_empty() => {
                couche = pc.clone();
                rang = *pr;
            }
            // Une génération sans parente écrite se relit comme racine (promesse 7).
            _ => return Ok(ReadStack { layers: etages, root_without_parent: true }),
        }
    }
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

    fn vue(couche: &str, rang: Option<i64>) -> ViewRequest {
        ViewRequest { layer: couche.into(), rank: rang }
    }

    fn etage(couche: &str, rang: i64, cles: &[&str], fichiers: &[&str]) -> StackLayer {
        StackLayer {
            layer: couche.into(),
            rank: rang,
            removed_keys: cles.iter().map(|s| s.to_string()).collect(),
            removed_files: fichiers.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// **Une pile d'un étage** : la génération 0 de `backend`, sans parente écrite, se relit comme racine ;
    /// un rang omis prend la dernière génération publiée.
    #[test]
    fn a_one_layer_stack_and_its_root_without_parent() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        let p = load_stack(&cat, &vue("backend", None), &ViewSettings::default()).unwrap();
        assert_eq!(p, ReadStack { layers: vec![etage("backend", 0, &[], &[])], root_without_parent: true });
        assert_eq!(load_stack(&cat, &vue("backend", Some(0)), &ViewSettings::default()).unwrap(), p);
    }

    /// **Une pile de deux étages** : l'enfant sur sa parente épinglée, ses tombstones lus ; un rang omis
    /// prend le dernier rang de l'enfant.
    #[test]
    fn a_two_layer_stack_with_its_tombstones() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "backend", 1, None, "[]");
        poser(&cat, "enfant", 1, Some(("backend", 0)), r#"{"keys": ["k-f"], "files": ["src/vieux.rs"]}"#);
        poser(&cat, "enfant", 2, Some(("backend", 0)), r#"{"keys": ["k-f", "k-g"], "files": []}"#);
        let p = load_stack(&cat, &vue("enfant", None), &ViewSettings::default()).unwrap();
        assert_eq!(
            p,
            ReadStack { layers: vec![etage("enfant", 2, &["k-f", "k-g"], &[]), etage("backend", 0, &[], &[])], root_without_parent: true }
        );
        let p1 = load_stack(&cat, &vue("enfant", Some(1)), &ViewSettings::default()).unwrap();
        assert_eq!(p1.layers[0], etage("enfant", 1, &["k-f"], &["src/vieux.rs"]));
        assert_eq!(p1.layers[1], etage("backend", 0, &[], &[]), "la parente reste épinglée à son rang, pas au dernier");
    }

    /// **La publication écrit une racine sans parente** (promesse 6, `backend` seul) : la génération 0
    /// posée par `poser_les_generations`, par le vrai `merge_generation`, se relit comme racine.
    #[test]
    fn generation_0_published_reads_back_as_a_root() {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let mut cat = crate::Catalog::without_embedding_model(Box::new(conn), crate::CatalogConfig::default());
        cat.initialize().unwrap();
        cat.poser_les_generations(&[], "backend").unwrap();
        let p = load_stack(&cat, &vue("backend", None), &ViewSettings::default()).unwrap();
        assert_eq!(p, ReadStack { layers: vec![etage("backend", 0, &[], &[])], root_without_parent: true });
        let lu = cat.execute_raw("MATCH (g:_generation) RETURN g.parente_couche IS NULL, g.parente_rang IS NULL").unwrap();
        assert_eq!(lu.rows.len(), 1);
        assert!(lu.rows[0].iter().all(|v| matches!(v, crate::connection::CypherValue::Bool(true))), "la racine porte une parente : {:?}", lu.rows);
    }

    /// **Le chargeur sur connexion** rend la même pile que par le catalogue (l'instantané d'un appel).
    #[test]
    fn the_loader_on_a_connection_gives_the_same_stack() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "enfant", 1, Some(("backend", 0)), r#"{"keys": ["k"], "files": []}"#);
        let r = ViewSettings::default();
        let par_le_catalogue = load_stack(&cat, &vue("enfant", None), &r).unwrap();
        let sur_connexion = load_stack_on(cat.conn(), cat.dialect_arc().as_ref(), &vue("enfant", None), &r).unwrap();
        assert_eq!(sur_connexion, par_le_catalogue);
    }

    /// **Les refus nommés** : une couche inconnue, un rang absent, une parente manquante, un cycle, une
    /// profondeur au-delà du réglage, des tombstones illisibles.
    #[test]
    fn the_loader_refusals_name_themselves() {
        let cat = catalogue();
        poser(&cat, "backend", 0, None, "[]");
        poser(&cat, "orpheline", 1, Some(("disparue", 3)), "[]");
        poser(&cat, "a", 1, Some(("b", 1)), "[]");
        poser(&cat, "b", 1, Some(("a", 1)), "[]");
        poser(&cat, "c1", 1, Some(("backend", 0)), "[]");
        poser(&cat, "c2", 1, Some(("c1", 1)), "[]");
        poser(&cat, "illisible", 1, None, "pas du json");
        let r = ViewSettings::default();
        for (v, mot) in [
            (vue("inconnue", None), "inconnue"),
            (vue("backend", Some(7)), "7"),
            (vue("orpheline", None), "disparue:3"),
            (vue("a", None), "cycle"),
            (vue("illisible", None), "tombstones"),
        ] {
            let refus = load_stack(&cat, &v, &r).expect_err(&format!("{v:?} se charge"));
            assert!(refus.contains(mot), "le refus de {v:?} ne nomme pas « {mot} » : {refus}");
        }
        let court = ViewSettings { max_depth: 2, ..r };
        let refus = load_stack(&cat, &vue("c2", None), &court).expect_err("trois étages sous une profondeur de 2");
        assert!(refus.contains("profondeur"), "{refus}");
        assert!(load_stack(&cat, &vue("c2", None), &ViewSettings::default()).is_ok());
    }

    /// **Les réglages des vues** : les défauts sans `views`, les valeurs déclarées, et une clé inconnue
    /// refusée en la nommant ; `tracked` accepte la clé `views`.
    #[test]
    fn the_view_settings_and_their_refusals() {
        assert_eq!(view_settings(None).unwrap(), ViewSettings::default());
        assert_eq!(view_settings(Some(&json!({"sources": []}))).unwrap(), ViewSettings::default());
        assert_eq!(
            view_settings(Some(&json!({"views": {"max_depth": 4, "load_timeout_ms": 500}}))).unwrap(),
            ViewSettings { max_depth: 4, load_timeout_ms: 500 }
        );
        for (mauvais, mot) in [(json!({"views": {"profondeur": 4}}), "profondeur"), (json!({"views": {"max_depth": 0}}), "max_depth"), (json!({"views": 3}), "views")] {
            let refus = view_settings(Some(&mauvais)).expect_err(&format!("{mauvais} est accepté"));
            assert!(refus.contains(mot), "le refus de {mauvais} ne nomme pas « {mot} » : {refus}");
        }
        crate::suivies::declaration_du_manifeste(Some(&json!({"views": {"max_depth": 4}, "sources": []})))
            .expect("tracked refuse la clé views");
    }
}
