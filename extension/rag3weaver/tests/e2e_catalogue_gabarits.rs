//! **Le catalogue de gabarits, cherché comme le reste.**
//!
//! C'est tout l'argument du
//! [doc 04](../docs/vision_roadmap_08_2026/04-le-catalogue-comme-graphe.md) et
//! la cible du [doc 08](../docs/vision_roadmap_08_2026/08-des-catalogues-de-gabarits.md) :
//! un agent trouve ses **capacités** avec les moyens qu'il emploie pour
//! trouver un **document**. Pas une liste figée dans une invite, pas un second
//! mécanisme — la même recherche, sur une entité de plus.
//!
//! Ce que ce test vérifie, dans l'ordre où ça compte :
//!
//! 1. le catalogue s'indexe comme n'importe quoi d'autre ;
//! 2. on filtre par **catégorie** — l'axe thématique, ouvert ;
//! 3. on trouve **par le sens**, sans reprendre les mots du gabarit ;
//! 4. on **pose** ce qu'on a trouvé, éventuellement avec un motif.
//!
//! Run with: ./run_e2e.sh --test e2e_catalogue_gabarits
#![cfg(all(feature = "rag3db-native", feature = "burn-embedder"))]

mod common;

use std::sync::Arc;

use rag3weaver::embedder::{DualEmbedder, HashEmbedder};
use rag3weaver::search::SearchOptions;
use rag3weaver::template::{
    builtin_root, register_template_schema, scan, Family, TEMPLATE_ENTITY,
};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

/// Un catalogue avec le vrai embedder : trouver un gabarit **par le sens** est
/// précisément ce qu'on veut prouver, et un embedder factice le rendrait faux
/// — c'est l'erreur qu'on a payée le 27 août.
fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();

    let config = CatalogConfig { name: Some("gabarits".into()), embedding_dim: 1024, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(1024)), config);
    catalog.initialize().unwrap();
    let bge: Arc<dyn DualEmbedder> = common::burn::BGE_M3.clone();
    catalog.set_dual_embedder(bge);
    register_template_schema(&mut catalog).unwrap();
    catalog
}

/// **Brique 1 : l'embedder nu.**
///
/// Avant d'accuser la recherche, on mesure le cosinus à la main entre une
/// question et chaque description. Si le vecteur dit la vérité ici, le défaut
/// est en aval — fusion, filtre, ou stockage. S'il ment déjà ici, c'est le
/// corpus ou le modèle.
///
/// Méthode de Lucie, 29 août : essayer chaque brique séparément.
#[test]
#[ignore]
fn brique_1_le_cosinus_nu_dit_il_la_verite() {
    use rag3weaver::embedder::Embedder;
    let emb: Arc<dyn Embedder> = common::burn::BGE_M3.clone();

    let fiches = scan(&builtin_root()).unwrap();
    let entites: Vec<_> = fiches.iter().filter(|f| f.family == Family::Entity).collect();
    assert_eq!(entites.len(), 3);

    let mut textes: Vec<String> = entites.iter().map(|f| f.description.clone()).collect();
    let questions = [
        ("vendre des articles avec un prix", "product"),
        ("de quoi savoir qui est connecté sur mon site", "user"),
        ("suivre un échange entre plusieurs personnes", "conversation"),
    ];
    for (q, _) in &questions {
        textes.push((*q).to_string());
    }
    let v = emb.embed(&textes).expect("embarquer");

    let cos = |a: &[f32], b: &[f32]| -> f32 {
        let (mut d, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..a.len() {
            d += a[i] * b[i];
            na += a[i] * a[i];
            nb += b[i] * b[i];
        }
        d / (na.sqrt() * nb.sqrt()).max(f32::EPSILON)
    };

    let mut justes = 0;
    for (i, (q, attendu)) in questions.iter().enumerate() {
        let qv = &v[entites.len() + i];
        let mut scores: Vec<(f32, &str)> = entites
            .iter()
            .enumerate()
            .map(|(j, f)| (cos(qv, &v[j]), f.name.as_str()))
            .collect();
        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let ligne: Vec<String> = scores.iter().map(|(s, n)| format!("{n} {s:.4}")).collect();
        let ok = scores[0].1 == *attendu;
        justes += usize::from(ok);
        eprintln!("[cosinus] {} « {q} » → {}", if ok { "✓" } else { "✗" }, ligne.join(" · "));
    }
    eprintln!("[cosinus] {justes}/3 questions rendent le bon gabarit en tête");

    // Ce que ce test fixe : **le cosinus nu est notre référence**. S'il est
    // juste et que la recherche ne l'est pas, le défaut est dans le moteur ;
    // s'il est faux, il n'y a rien à corriger en aval.
    assert_eq!(justes, 3, "l'embedder doit reconnaître ses propres descriptions");
}

/// **Brique 2 : le vecteur seul, à travers le moteur.**
///
/// Le cosinus nu est juste (brique 1). On refait les mêmes questions par
/// `Catalog::search`, en ne laissant qu'un signal à la fois — vecteur, puis
/// plein texte, puis les deux. Celui qui ment se nomme.
#[test]
#[ignore]
fn brique_2_quel_signal_ment() {
    use rag3weaver::search::SearchSignals;
    let mut catalog = setup();
    let fiches = scan(&builtin_root()).unwrap();
    let r = catalog
        .ingest_entities(TEMPLATE_ENTITY, fiches.iter().map(|f| f.data()).collect())
        .unwrap();
    assert_eq!(r.failed, 0);

    let noms = |r: &rag3weaver::search::SearchResponse| -> Vec<String> {
        r.results
            .iter()
            .filter_map(|x| match x.data.as_ref()?.get("name")? {
                rag3weaver::connection::CypherValue::String(s) => Some(format!("{s} {:.4}", x.score)),
                _ => None,
            })
            .collect()
    };

    let questions = [
        ("vendre des articles avec un prix", "product"),
        ("de quoi savoir qui est connecté sur mon site", "user"),
        ("suivre un échange entre plusieurs personnes", "conversation"),
    ];
    for (etiquette, signaux) in [
        ("vecteur seul", SearchSignals::VECTOR),
        ("plein texte seul", SearchSignals::BM25),
        ("les deux", SearchSignals::HYBRID),
    ] {
        let mut justes = 0;
        for (q, attendu) in &questions {
            let mut o = rag3weaver::search::SearchOptions { limit: 5, signals: Some(signaux), ..Default::default() };
            o.filters.insert(
                "family".to_string(),
                rag3weaver::filter::FilterValue::Direct(rag3weaver::connection::CypherValue::String("entity".into())),
            );
            let out = catalog.search(TEMPLATE_ENTITY, q, o).unwrap();
            let l = noms(&out);
            let ok = l.first().map(|s| s.starts_with(attendu)).unwrap_or(false);
            justes += usize::from(ok);
            eprintln!("[{etiquette}] {} « {q} » → {}", if ok { "✓" } else { "✗" }, l.join(" · "));
        }
        eprintln!("[{etiquette}] {justes}/3");
    }
}

#[test]
#[ignore]
fn un_agent_trouve_ses_gabarits_comme_il_trouve_un_document() {
    let mut catalog = setup();

    // ── 1. Le catalogue s'indexe ────────────────────────────────────────
    let fiches = scan(&builtin_root()).expect("lire les gabarits fournis");
    eprintln!("[catalogue] {} gabarits sur le disque", fiches.len());
    for f in &fiches {
        eprintln!("  {:<10} {:<14} {}", f.family.as_str(), f.name, f.category);
    }
    // `ingest_entities` porte son propre graphe : c'est **son** compteur qui
    // dit ce qui a été indexé, pas le `drain` qui suit et qui n'a plus rien à
    // faire.
    let report = catalog
        .ingest_entities(TEMPLATE_ENTITY, fiches.iter().map(|f| f.data()).collect())
        .expect("ingérer les fiches");
    eprintln!("[catalogue] indexé : {report:?}");
    assert_eq!(report.failed, 0, "{report:?}");
    assert_eq!(report.processed, fiches.len(), "chaque fiche est indexée");

    let opts = SearchOptions { limit: 10, ..Default::default() };
    let noms = |r: &rag3weaver::search::SearchResponse| -> Vec<String> {
        r.results
            .iter()
            .filter_map(|x| match x.data.as_ref()?.get("name")? {
                rag3weaver::connection::CypherValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    };
    // Ce qu'on veut lire quand un classement surprend : le score, et **quel
    // signal** l'a trouvé.
    let detail = |r: &rag3weaver::search::SearchResponse| -> String {
        r.results
            .iter()
            .take(4)
            .map(|x| {
                let n = x.data.as_ref().and_then(|d| d.get("name")).and_then(|v| match v {
                    rag3weaver::connection::CypherValue::String(s) => Some(s.clone()),
                    _ => None,
                }).unwrap_or_default();
                format!("{n} {:.4}", x.score)
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };

    // ── 2. Par catégorie — un **filtre**, pas une requête ───────────────
    //
    // Une catégorie est une facette : elle dit *de quoi ça parle*, elle ne se
    // cherche pas, elle **restreint**. Premier essai le 29 août : la passer en
    // requête (`search("auth")`) rendait `product` en tête et `user` en
    // quatrième — un mot qui n'est dans aucun champ de contenu ne matche rien,
    // et il ne restait que du bruit vectoriel. Deux questions, deux
    // mécanismes.
    //
    // La catégorie traverse les familles, et c'est tout son intérêt : le jour
    // où un écran de connexion sera dans le catalogue, `auth` rendra le schéma
    // **et** l'écran.
    let filtre = |champ: &str, valeur: &str| {
        let mut o = opts.clone();
        o.filters.insert(
            champ.to_string(),
            rag3weaver::filter::FilterValue::Direct(rag3weaver::connection::CypherValue::String(valeur.into())),
        );
        o
    };
    let auth = catalog.search(TEMPLATE_ENTITY, "", filtre("category", "auth")).unwrap();
    eprintln!("[catalogue] catégorie auth : {}", detail(&auth));
    assert_eq!(noms(&auth), vec!["user".to_string()], "{}", detail(&auth));

    // Et la famille filtre aussi — l'autre axe, structurel celui-là.
    let entites = catalog.search(TEMPLATE_ENTITY, "", filtre("family", "entity")).unwrap();
    let mut e = noms(&entites);
    e.sort();
    assert_eq!(e, vec!["conversation", "product", "user"], "{}", detail(&entites));

    // ── 3. Par le sens — sans reprendre les mots ────────────────────────
    //
    // Le test qui compte. Aucun de ces mots n'est dans la description de
    // `user` ; c'est le vecteur qui doit faire le lien, et c'est ce qu'un
    // BM25 seul ne peut pas faire.
    // **Les deux axes ensemble, et c'est comme ça qu'on s'en sert.** Un agent
    // qui cherche un schéma ne fouille pas les graphes-outils : il restreint la
    // famille, puis demande par le sens. Sur les dix gabarits mêlés, la requête
    // seule ne discriminait pas — dix descriptions courtes et hétérogènes,
    // scores tous à ~0,03. Le filtre n'est pas un raccourci, c'est la moitié de
    // la question.
    let qui = catalog
        .search(TEMPLATE_ENTITY, "de quoi savoir qui est connecté sur mon site", filtre("family", "entity"))
        .unwrap();
    eprintln!("[catalogue] qui est connecté : {}", detail(&qui));
    eprintln!("[catalogue] signaux : {:?}", qui.meta.signals);
    assert_eq!(noms(&qui).first().map(String::as_str), Some("user"), "{}", detail(&qui));

    // Diagnostic : la même question, avec et sans le filtre. Si l'ordre
    // s'inverse, ce n'est pas le sens qui décide, c'est le chemin filtré.
    let vente_nue = catalog.search(TEMPLATE_ENTITY, "vendre des articles avec un prix", opts.clone()).unwrap();
    eprintln!("[diag] vendre SANS filtre : {}", detail(&vente_nue));
    let vente = catalog
        .search(TEMPLATE_ENTITY, "vendre des articles avec un prix", filtre("family", "entity"))
        .unwrap();
    eprintln!("[diag] vendre AVEC filtre : {}", detail(&vente));
    eprintln!("[diag] meta filtré : vector={} bm25={} fused={}", vente.meta.vector_count, vente.meta.bm25_count, vente.meta.fused_count);
    assert_eq!(noms(&vente).first().map(String::as_str), Some("product"), "{}", detail(&vente));

    let fil = catalog
        .search(TEMPLATE_ENTITY, "suivre un échange entre plusieurs personnes", filtre("family", "entity"))
        .unwrap();
    eprintln!("[catalogue] un échange : {}", detail(&fil));
    assert_eq!(noms(&fil).first().map(String::as_str), Some("conversation"), "{}", detail(&fil));

    // ── 4. Poser ce qu'on a trouvé ──────────────────────────────────────
    //
    // Sous le nom qu'on veut : le nom appartient à qui adopte, comme pour les
    // outils.
    let produit = std::fs::read_to_string(builtin_root().join("entities/product.json")).unwrap();
    rag3weaver::template::place_entity_with(&mut catalog, &produit, &[], "Article")
        .expect("poser le gabarit");
    assert!(catalog.is_registered_entity("Article"));

    // Et avec un motif : une entité par révision au lieu d'une par chose.
    let motif = std::fs::read_to_string(builtin_root().join("patterns/versioned.json")).unwrap();
    rag3weaver::template::place_entity_with(&mut catalog, &produit, &[&motif], "ArticleVersionne")
        .expect("poser avec le motif");
    assert!(catalog.is_registered_entity("ArticleVersionne"));

    // Les deux vivent côte à côte, et n'ont pas la même identité — c'est tout
    // ce que « versionné » veut dire.
    let mut ligne = std::collections::BTreeMap::new();
    ligne.insert("sku".to_string(), rag3weaver::connection::CypherValue::String("K-1".into()));
    ligne.insert("name".to_string(), rag3weaver::connection::CypherValue::String("Couteau".into()));
    ligne.insert("description".to_string(), rag3weaver::connection::CypherValue::String("Un couteau de cuisine.".into()));
    let nu = catalog.entity_uuid("Article", &ligne).unwrap();

    ligne.insert("revision".to_string(), rag3weaver::connection::CypherValue::String("v1".into()));
    let v1 = catalog.entity_uuid("ArticleVersionne", &ligne).unwrap();
    ligne.insert("revision".to_string(), rag3weaver::connection::CypherValue::String("v2".into()));
    let v2 = catalog.entity_uuid("ArticleVersionne", &ligne).unwrap();

    eprintln!("[motif] nu={nu} v1={v1} v2={v2}");
    assert_ne!(v1, v2, "deux révisions, deux lignes — c'est tout le motif");
    assert_ne!(nu, v1, "l'entité nue et la versionnée n'ont pas la même identité");

    // ── Ce qu'on veut voir en le lisant ─────────────────────────────────
    let familles: std::collections::BTreeSet<&str> = fiches.iter().map(|f| f.family.as_str()).collect();
    eprintln!("[catalogue] familles présentes : {familles:?}");
    assert!(familles.contains(Family::Entity.as_str()));
    assert!(familles.contains(Family::Graph.as_str()), "les graphes-outils sont déjà des gabarits");
    assert!(familles.contains(Family::Pattern.as_str()));
}
