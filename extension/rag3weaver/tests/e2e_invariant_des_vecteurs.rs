//! E2E — **l'invariant des vecteurs** : toute ligne qui porte un vecteur est
//! joignable par son index, et sort première quand on cherche son propre
//! vecteur.
//!
//! Le fait du moteur, mesuré le 3 octobre 2026 par la session cœur C++ :
//! **remplacer un vecteur par un autre par `SET` perd des lignes dans l'index
//! HNSW** — 969 joignables sur 1 000 ligne à ligne, jusqu'à 114 sur 1 000 par
//! lots de 512, et pas le même compte d'une passe à l'autre. Sont sûrs : un `SET` depuis NULL, supprimer puis réinsérer la
//! ligne, retirer l'index, écrire, recréer l'index.
//!
//! Cette suite ne teste pas le moteur : elle passe par **les chemins du
//! produit** et dit à quel point le défaut nous touche. Après chaque suite
//! d'opérations, l'index vaut-il un index bâti à neuf sur les mêmes lignes ?
//!
//! # La mesure
//!
//! Un HNSW est approché : même neuf, il peut laisser un nœud hors d'atteinte.
//! On ne compare donc pas à « tout » mais à **l'index reconstruit** — retiré
//! puis recréé sur les mêmes lignes. Deux comptes, avant et après la
//! reconstruction : les lignes rendues par une recherche de `k` = toutes, et
//! celles qui sortent premières sur leur propre vecteur.
//!
//! Base en mémoire, extension vector, un embarqueur déterministe.
//!
//! Run with: ./run_e2e.sh --test e2e_invariant_des_vecteurs
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap, HashSet};

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::disponibilite::Disponibilites as D;
use rag3weaver::embedder::{EmbedError, Embedder, HashEmbedder};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection};

const DIM: usize = 32;
const LIGNES: usize = 600;
const TABLE: &str = "Fiche_Chunk";

/// Un embarqueur déterministe qui ne se déclare pas factice.
struct Hachage(HashEmbedder, &'static str);

impl Embedder for Hachage {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.0.embed(texts)
    }
    fn dim(&self) -> usize {
        self.0.dim()
    }
    fn is_mock(&self) -> bool {
        false
    }
    fn name(&self) -> &str {
        self.1
    }
}

/// **La géométrie qui fait perdre des lignes au moteur** : dimension 4, des
/// vecteurs alignés sur le numéro de la fiche — celle du témoin du cœur C++.
/// Avec des vecteurs dispersés de dimension 32, le moteur seul ne perd rien à
/// cette échelle (voir `le_moteur_seul_sous_des_set_de_vecteur`) : un cas du
/// produit n'y prouverait rien.
struct Aligne;

impl Embedder for Aligne {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| {
                let i: usize = t
                    .split("numéro ")
                    .nth(1)
                    .map(|reste| reste.chars().take_while(char::is_ascii_digit).collect::<String>())
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
                if t.contains("v2") {
                    vec![(i % 13) as f32 + 0.5, (i % 19) as f32, (i % 31) as f32, i as f32 + 0.25]
                } else {
                    vec![(i % 17) as f32, (i % 23) as f32, (i % 29) as f32, i as f32]
                }
            })
            .collect())
    }
    fn dim(&self) -> usize {
        4
    }
    fn is_mock(&self) -> bool {
        false
    }
    fn name(&self) -> &str {
        "aligne"
    }
}

fn extension() -> String {
    let root = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::Path::new(&manifest).parent().unwrap().parent().unwrap().to_string_lossy().to_string()
    });
    format!("{root}/extension/vector/build/libvector.rag3db_extension")
}

/// Une entité dont la clé fait l'identité : éditer son texte garde la ligne,
/// et ses morceaux.
fn fiche() -> EntityConfig {
    serde_json::from_value(serde_json::json!({
        "fields": {"cle": {"type": "string", "isTitle": true}, "texte": {"type": "string", "isContent": true}},
        "hashsafe": ["cle"],
        "signals": ["bm25", "vector"]
    }))
    .expect("configuration de l'entité")
}

fn monter(conn: Rag3dbConnection, modele: &'static str) -> Catalog {
    monter_avec(conn, Box::new(Hachage(HashEmbedder::new(DIM), modele)))
}

fn monter_avec(conn: Rag3dbConnection, embarqueur: Box<dyn Embedder>) -> Catalog {
    conn.execute(&format!("LOAD EXTENSION '{}'", extension())).unwrap();
    let config = CatalogConfig { embedding_dim: embarqueur.dim(), ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), embarqueur, config);
    catalog.initialize().unwrap();
    catalog.register_entity("Fiche", fiche()).unwrap();
    catalog
}

fn catalogue() -> Catalog {
    monter(Rag3dbConnection::in_memory().expect("base en mémoire"), "hachage")
}

fn ligne(i: usize, version: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([
        ("cle".to_string(), CypherValue::String(format!("fiche-{i}"))),
        ("texte".to_string(), CypherValue::String(format!("Fiche numéro {i}, {version} : le contenu {} de la ligne {i}.", i * 7919 % 1009))),
    ])
}

fn lignes(plage: std::ops::Range<usize>, version: &str) -> Vec<BTreeMap<String, CypherValue>> {
    plage.map(|i| ligne(i, version)).collect()
}

/// Ce que l'index rend aujourd'hui pour les lignes qui portent un vecteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Etat {
    /// Les lignes qui portent un vecteur (marqueur posé).
    porteuses: usize,
    /// Celles qu'une recherche de `k` = toutes rend.
    joignables: usize,
    /// Celles qui sortent premières sur leur propre vecteur.
    premieres: usize,
    /// Celles dont le marqueur dit que le vecteur est celui du texte courant.
    a_jour: usize,
}

fn chaine(v: &CypherValue) -> String {
    match v {
        CypherValue::String(s) => s.clone(),
        autre => format!("{autre:?}"),
    }
}

fn mesurer(catalog: &Catalog) -> Etat {
    let s = catalog.vector_storage(TABLE).expect("stockage du modèle courant");
    let porteuses = catalog
        .execute_raw(&format!(
            "MATCH (c:{TABLE}) WHERE c.{m} IS NOT NULL AND c.{m} <> '' RETURN c._uuid, CAST(c.{col} AS STRING), c.{m} = c._text_hash",
            m = s.marker,
            col = s.column
        ))
        .expect("lecture des vecteurs");
    let k = porteuses.rows.len() + 64;
    let mut joignables = 0;
    let mut premieres = 0;
    let mut a_jour = 0;
    // Toutes les lignes qu'une recherche large rend, depuis un point quelconque.
    let rendues: HashSet<String> = match porteuses.rows.first() {
        None => HashSet::new(),
        Some(premiere) => catalog
            .execute_raw(&format!("CALL QUERY_VECTOR_INDEX('{TABLE}', '{}', {}, {k}) RETURN node._uuid", s.index, chaine(&premiere[1])))
            .expect("recherche large")
            .rows
            .iter()
            .map(|r| chaine(&r[0]))
            .collect(),
    };
    for row in &porteuses.rows {
        let uuid = chaine(&row[0]);
        if rendues.contains(&uuid) {
            joignables += 1;
        }
        if matches!(row[2], CypherValue::Bool(true)) {
            a_jour += 1;
        }
        let proche = catalog
            .execute_raw(&format!("CALL QUERY_VECTOR_INDEX('{TABLE}', '{}', {}, 1) RETURN node._uuid", s.index, chaine(&row[1])))
            .expect("recherche de soi");
        if proche.rows.first().map(|r| chaine(&r[0])).as_deref() == Some(uuid.as_str()) {
            premieres += 1;
        }
    }
    Etat { porteuses: porteuses.rows.len(), joignables, premieres, a_jour }
}

/// Le vecteur de chaque morceau, par uuid — pour dire ce qu'une édition a
/// fait : la même ligne avec un autre vecteur est un remplacement en place.
fn vecteurs(catalog: &Catalog) -> HashMap<String, String> {
    let s = catalog.vector_storage(TABLE).expect("stockage");
    catalog
        .execute_raw(&format!("MATCH (c:{TABLE}) WHERE c.{} IS NOT NULL RETURN c._uuid, CAST(c.{} AS STRING)", s.column, s.column))
        .expect("lecture des vecteurs")
        .rows
        .iter()
        .map(|r| (chaine(&r[0]), chaine(&r[1])))
        .collect()
}

/// Combien de lignes ont gardé leur uuid et changé de vecteur.
fn remplaces(avant: &HashMap<String, String>, apres: &HashMap<String, String>) -> usize {
    apres.iter().filter(|(uuid, v)| avant.get(*uuid).is_some_and(|ancien| ancien != *v)).count()
}

/// L'index retiré puis recréé sur les mêmes lignes : la référence.
fn reconstruire(catalog: &Catalog) {
    let s = catalog.vector_storage(TABLE).expect("stockage");
    catalog.execute_raw(&format!("CALL DROP_VECTOR_INDEX('{TABLE}', '{}')", s.index)).expect("retirer l'index");
    catalog
        .execute_raw(&format!("CALL CREATE_VECTOR_INDEX('{TABLE}', '{}', '{}', metric := 'cosine')", s.index, s.column))
        .expect("recréer l'index");
}

/// **L'invariant** : l'index vaut un index neuf, et chaque vecteur est celui
/// du texte courant.
fn verifier(catalog: &Catalog, quoi: &str) {
    let avant = mesurer(catalog);
    reconstruire(catalog);
    let neuf = mesurer(catalog);
    eprintln!("[invariant] {quoi} : {avant:?} ; index reconstruit : {neuf:?}");
    assert_eq!(avant.a_jour, avant.porteuses, "{quoi} : des vecteurs ne sont pas ceux du texte courant — {avant:?}");
    assert_eq!(
        (avant.joignables, avant.premieres),
        (neuf.joignables, neuf.premieres),
        "{quoi} : l'index ne vaut pas un index neuf sur les mêmes lignes — avant {avant:?}, reconstruit {neuf:?}"
    );
    assert_eq!(neuf.premieres, neuf.porteuses, "{quoi} : même reconstruit, des lignes ne sortent pas premières sur leur propre vecteur — {neuf:?}");
}

fn solder(catalog: &mut Catalog) {
    while catalog.embarquer_le_retard(D::TOUT, 512, None).expect("rattrapage") > 0 {}
}

/// Le témoin : une ingestion directe. Si l'invariant ne tient pas ici, c'est
/// la mesure qui est fausse, pas le produit.
#[test]
#[ignore]
fn une_ingestion_directe_tient_l_invariant() {
    let mut catalog = catalogue();
    catalog.ingest_entities("Fiche", lignes(0..LIGNES, "v1")).expect("ingestion");
    assert_eq!(mesurer(&catalog).porteuses, LIGNES);
    verifier(&catalog, "ingestion directe");
}

/// L'ingestion en deux temps : les lignes et le plein texte, puis la dette de
/// vecteurs — un `SET` depuis NULL, le chemin principal.
#[test]
#[ignore]
fn l_ingestion_en_deux_temps_tient_l_invariant() {
    let mut catalog = catalogue();
    catalog.ingest_entities_jusqu_a("Fiche", lignes(0..LIGNES, "v1"), D::RECHERCHE_TEXTE).expect("mots");
    assert_eq!(mesurer(&catalog).porteuses, 0, "aucun vecteur avant le rattrapage");
    solder(&mut catalog);
    assert_eq!(mesurer(&catalog).porteuses, LIGNES);
    verifier(&catalog, "deux temps");
}

/// **L'édition d'un texte** : la ligne garde sa clé, donc ses morceaux ; leur
/// vecteur est remplacé. C'est le chemin de `ingest_entities` sur une ligne
/// modifiée et de la réingestion d'un fichier édité.
#[test]
#[ignore]
fn l_edition_d_un_texte_tient_l_invariant() {
    let mut catalog = catalogue();
    catalog.ingest_entities("Fiche", lignes(0..LIGNES, "v1")).expect("ingestion");
    // Une ligne sur deux change de texte, d'un seul lot.
    let avant = vecteurs(&catalog);
    let editees: Vec<_> = (0..LIGNES).step_by(2).map(|i| ligne(i, "v2, réécrite")).collect();
    catalog.ingest_entities("Fiche", editees).expect("édition");
    assert_eq!(mesurer(&catalog).porteuses, LIGNES);
    // Sans cette garde le test serait vert pour une mauvaise raison : il faut
    // que l'édition ait bien remplacé des vecteurs sur des lignes gardées.
    assert_eq!(remplaces(&avant, &vecteurs(&catalog)), LIGNES / 2, "l'édition devait remplacer un vecteur sur deux, en place");
    verifier(&catalog, "édition par lot");
}

/// La même édition, ligne à ligne : ce que fait un agent qui édite.
#[test]
#[ignore]
fn des_editions_une_a_une_tiennent_l_invariant() {
    let mut catalog = catalogue();
    catalog.ingest_entities("Fiche", lignes(0..LIGNES, "v1")).expect("ingestion");
    let avant = vecteurs(&catalog);
    for i in (0..LIGNES).step_by(3) {
        catalog.ingest_entities("Fiche", vec![ligne(i, "v2, réécrite")]).expect("édition");
    }
    assert_eq!(remplaces(&avant, &vecteurs(&catalog)), LIGNES / 3, "chaque édition devait remplacer un vecteur, en place");
    verifier(&catalog, "éditions une à une");
}

/// L'édition en deux temps : le texte d'abord, le vecteur ensuite. Le vecteur
/// d'avant ne doit pas rester en place sous un texte qui a changé.
#[test]
#[ignore]
fn l_edition_en_deux_temps_reembarque_et_tient_l_invariant() {
    let mut catalog = catalogue();
    catalog.ingest_entities("Fiche", lignes(0..LIGNES, "v1")).expect("ingestion");
    let editees: Vec<_> = (0..LIGNES).step_by(2).map(|i| ligne(i, "v2, réécrite")).collect();
    let avant = vecteurs(&catalog);
    catalog.ingest_entities_jusqu_a("Fiche", editees, D::RECHERCHE_TEXTE).expect("édition, mots seulement");
    solder(&mut catalog);
    eprintln!("[invariant] édition en deux temps : {} vecteurs remplacés en place", remplaces(&avant, &vecteurs(&catalog)));
    verifier(&catalog, "édition en deux temps");
}

/// **Le cas le plus proche du défaut du moteur** : mille fiches aux vecteurs
/// alignés, toutes rééditées d'un coup ; toutes doivent rester joignables,
/// sans reconstruire.
///
/// Ce que ce cas ne prouve pas : joué une fois sans le passage par NULL de
/// `write_vectors` (4 octobre 2026), il rendait aussi 1000 sur 1000. Le
/// produit pose ses vecteurs par lots de 32, et le moteur perd surtout par
/// gros lots, d'une passe à l'autre différemment. C'est une garde contre une
/// régression du chemin, pas une démonstration du contournement.
#[test]
#[ignore]
fn une_reedition_massive_ne_sort_aucune_ligne_de_l_index() {
    const MILLE: usize = 1000;
    let mut catalog = monter_avec(Rag3dbConnection::in_memory().expect("base en mémoire"), Box::new(Aligne));
    catalog.ingest_entities("Fiche", lignes(0..MILLE, "v1")).expect("ingestion");
    let avant = vecteurs(&catalog);
    catalog.ingest_entities("Fiche", lignes(0..MILLE, "v2, réécrite")).expect("réédition");
    assert_eq!(remplaces(&avant, &vecteurs(&catalog)), MILLE, "chaque ligne devait changer de vecteur, en place");
    let etat = mesurer(&catalog);
    eprintln!("[invariant] réédition massive, vecteurs alignés : {etat:?}");
    assert_eq!((etat.porteuses, etat.joignables, etat.a_jour), (MILLE, MILLE, MILLE), "des lignes sont sorties de l'index — {etat:?}");
}

/// Le changement de modèle : un second modèle remplit sa propre colonne,
/// depuis NULL ; celle du premier ne bouge pas.
#[test]
#[ignore]
fn un_second_modele_tient_l_invariant_sans_toucher_au_premier() {
    let dossier = tempfile::tempdir().expect("dossier");
    let base = dossier.path().join("vecteurs.rag3db");
    {
        let mut a = monter(Rag3dbConnection::new(&base).expect("base"), "modele-a");
        a.ingest_entities("Fiche", lignes(0..LIGNES, "v1")).expect("ingestion");
        verifier(&a, "premier modèle");
    }
    let mut b = monter(Rag3dbConnection::new(&base).expect("base rouverte"), "modele-b");
    // Rien à mesurer avant : le second modèle n'a sa colonne et son index
    // qu'une fois enregistré, ce que fait le premier rattrapage.
    solder(&mut b);
    assert_eq!(mesurer(&b).porteuses, LIGNES);
    verifier(&b, "second modèle");
}

/// **Le moteur seul, sans le produit** : une table nue, un index, puis des
/// `SET` d'un vecteur vers un autre. Ne juge rien — il dit ce que le moteur
/// de cette passe fait, pour lire les cas du produit à côté. Deux géométries :
/// celle de la suite (dimension 32, vecteurs dispersés, cosinus) et celle du
/// témoin du cœur C++ (dimension 4, vecteurs alignés sur l'identifiant, l2,
/// mille lignes — `vector_set_reachability`).
#[test]
#[ignore]
fn le_moteur_seul_sous_des_set_de_vecteur() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", extension())).unwrap();
    let disperse = |i: usize, tour: usize| {
        let graine = i + tour * 100_000;
        let v: Vec<String> = (0..DIM).map(|d| format!("{:.4}", (((graine + 1) * (d + 3) * 2654435761) % 1000) as f32 / 1000.0 + 0.001)).collect();
        format!("[{}]", v.join(","))
    };
    let aligne = |i: usize, tour: usize| match tour {
        0 => format!("[{}.0, {}.0, {}.0, {i}.0]", i % 17, i % 23, i % 29),
        _ => format!("[{}.5, {}.0, {}.0, {i}.25]", i % 13, i % 19, i % 31),
    };
    type Vecteur<'a> = &'a dyn Fn(usize, usize) -> String;
    let cas: [(&str, usize, &str, usize, Vecteur); 2] =
        [("dimension 32, dispersés, cosinus", DIM, "cosine", LIGNES, &disperse), ("dimension 4, alignés, l2", 4, "l2", 1000, &aligne)];
    for (nom, dim, metrique, lignes, vecteur) in cas {
        for lot in [1usize, 32, 512] {
            conn.execute(&format!("CREATE NODE TABLE Nu(id INT64 PRIMARY KEY, vec FLOAT[{dim}])")).unwrap();
            for i in 0..lignes {
                conn.execute(&format!("CREATE (:Nu {{id: {i}, vec: {}}})", vecteur(i, 0))).unwrap();
            }
            conn.execute(&format!("CALL CREATE_VECTOR_INDEX('Nu', 'nu_index', 'vec', metric := '{metrique}')")).unwrap();
            for debut in (0..lignes).step_by(lot) {
                let items: Vec<String> = (debut..(debut + lot).min(lignes)).map(|i| format!("{{id: {i}, emb: {}}}", vecteur(i, 1))).collect();
                conn.execute(&format!("UNWIND [{}] AS item MATCH (n:Nu {{id: item.id}}) SET n.vec = item.emb", items.join(","))).unwrap();
            }
            let rendues = conn
                .execute(&format!("CALL QUERY_VECTOR_INDEX('Nu', 'nu_index', {}, {k}, efs := {k}) RETURN node.id", vecteur(7, 1), k = lignes + 64))
                .unwrap()
                .rows
                .len();
            eprintln!("[moteur seul] {nom} — SET vecteur → vecteur par lots de {lot} : {rendues} joignables sur {lignes}");
            conn.execute("CALL DROP_VECTOR_INDEX('Nu', 'nu_index')").unwrap();
            conn.execute("DROP TABLE Nu").unwrap();
        }
    }
}
