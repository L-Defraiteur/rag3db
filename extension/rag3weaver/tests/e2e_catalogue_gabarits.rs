//! **Le catalogue de gabarits, cherché comme le reste.**
//!
//! C'est tout l'argument du
//! [doc 04](../docs/vision_roadmap_09_2026/04-le-catalogue-comme-graphe.md) et
//! la cible du [doc 08](../docs/vision_roadmap_09_2026/08-des-catalogues-de-gabarits.md) :
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
//!
//! **`#[ignore]` sur chacun.** `run_e2e.sh` ne lance que les tests ignorés
//! (`-- --ignored`) : cinq de ces dix n'en avaient pas et étaient donc
//! « filtered out » à chaque passe complète — la moitié de la suite ne tournait
//! nulle part. Relevé le 3 septembre 2026.

mod common;

use std::sync::Arc;

use rag3weaver::embedder::DualEmbedder;
use rag3weaver::search::SearchOptions;
use rag3weaver::template::{
    builtin_root, register_template_schema, scan, Family, Origin, TEMPLATE_ENTITY,
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

    // **Le même embedder des deux côtés.** `Catalog::search` embarque la
    // requête avec l'embedder **du catalogue** (`catalog.rs:3812`), tandis que
    // `set_dual_embedder` ne change que ce qui indexe les documents. Passer un
    // `HashEmbedder` ici et BGE-M3 en dual compare deux espaces vectoriels sans
    // rapport : l'ordre devient quasi constant, les scores tournent autour de
    // zéro, et rien ne le dit. Mesuré le 29 août 2026 — voir
    // `docs/issues/29-08-2026/01`.
    let config = CatalogConfig { name: Some("gabarits".into()), embedding_dim: 1024, ..Default::default() };
    let bge: Arc<dyn DualEmbedder> = common::burn::BGE_M3.clone();
    let dense: Arc<dyn rag3weaver::embedder::Embedder> = common::burn::BGE_M3.clone();
    let mut catalog = Catalog::new(boxed, Box::new(dense), config);
    catalog.initialize().unwrap();
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

    let fiches = scan(&builtin_root(), Origin::Builtin).unwrap();
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

/// **Brique 3 : qu'est-ce qui est réellement découpé et embarqué ?**
///
/// `score = 1.0 - distance` avec une métrique cosinus devrait rendre la
/// similarité — donc ~0,60 pour `user` sur sa question (brique 1). On lit
/// 0,068. Avant d'accuser le calcul, on regarde **le texte** qui a été
/// embarqué : si ce n'est pas la description, il n'y a rien à corriger dans la
/// recherche.
#[test]
#[ignore]
fn brique_3_quel_texte_est_embarque() {
    let mut catalog = setup();
    let fiches = scan(&builtin_root(), Origin::Builtin).unwrap();
    catalog
        .ingest_entities(TEMPLATE_ENTITY, fiches.iter().map(|f| f.data()).collect())
        .unwrap();

    // Les tables que le catalogue a créées pour cette entité.
    for q in [
        "MATCH (c:Template_Chunk) RETURN count(c)",
        "MATCH (t:Template) RETURN count(t)",
    ] {
        eprintln!("[stock] {q} → {:?}", catalog.execute_raw(q).map(|r| r.rows));
    }

    // Le texte de chaque chunk, tronqué : c'est lui qui a été embarqué.
    let rows = catalog
        .execute_raw("MATCH (c:Template_Chunk) RETURN c._title, c._text LIMIT 12")
        .expect("read les chunks");
    for r in &rows.rows {
        let titre = r.first().and_then(|v| v.as_str()).unwrap_or("");
        let texte = r.get(1).and_then(|v| v.as_str()).unwrap_or("");
        let coupe: String = texte.chars().take(120).collect();
        eprintln!("[chunk] titre={titre:?} texte={coupe:?}");
    }
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
    let fiches = scan(&builtin_root(), Origin::Builtin).unwrap();
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
    let fiches = scan(&builtin_root(), Origin::Builtin).expect("read les gabarits fournis");
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
    // Ce qu'on veut read quand un classement surprend : le score, et **quel
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

/// **Poser, pas seulement trouver.**
///
/// Le catalogue se cherchait déjà comme un document ; savoir que `user` existe
/// ne l'installait pas. L'outil `place` ferme la boucle, et ce test vérifie la
/// seule chose qui compte : après l'appel, l'entité **est dans le schéma** et
/// on peut y écrire.
#[test]
#[ignore]
fn poser_un_gabarit_l_enregistre_vraiment() {
    use rag3weaver::template::{read, prepare_entity};

    let mut catalog = setup();
    let racine = builtin_root();

    // Ce que l'outil fait, avec les mêmes fonctions que le nœud.
    let contenu = read(&racine, Family::Entity, "user").expect("le gabarit user existe");
    let config = prepare_entity(&contenu, &[]).expect("configuration lisible");
    let champs: Vec<String> = {
        let mut c: Vec<String> = config.fields.keys().cloned().collect();
        c.sort_unstable();
        c
    };
    assert!(!champs.is_empty(), "un gabarit d'entité sans champ ne sert à rien");
    catalog.register_entity("Client", config).expect("enregistrement");

    // **La preuve n'est pas l'absence d'erreur, c'est qu'on puisse s'en
    // servir.** Une entité enregistrée dont la table n'existe pas passerait
    // l'appel et échouerait à la première écriture.
    let r = catalog
        .conn()
        .execute("MATCH (c:Client) RETURN count(c) AS n")
        .expect("la table Client existe");
    assert_eq!(r.rows[0][0], rag3weaver::connection::CypherValue::Int(0));
    println!("[place] Client posé depuis 'user' — champs : {}", champs.join(", "));
}

/// **Un nom inconnu doit dire ce qui existe.**
///
/// C'est la différence entre un agent qui se corrige en un tour et un agent qui
/// redemande la liste : « gabarit 'users' inconnu » l'envoie deviner, « inconnu
/// — il y a conversation, product, user » lui donne la réponse dans le refus.
#[test]
#[ignore]
fn un_gabarit_inconnu_nomme_ses_voisins() {
    use rag3weaver::template::read;
    let e = read(&builtin_root(), Family::Entity, "users").expect_err("'users' n'existe pas");
    assert!(e.contains("users"), "{e}");
    assert!(e.contains("user"), "le refus doit nommer les voisins : {e}");
}

/// **Les motifs s'appliquent avant l'enregistrement**, et ils ajoutent
/// vraiment quelque chose. Sans cette vérification, un motif silencieusement
/// ignoré produirait une entité qui a l'air correcte.
#[test]
#[ignore]
fn un_motif_ajoute_ses_champs_avant_l_enregistrement() {
    use rag3weaver::template::{read, prepare_entity};
    let racine = builtin_root();
    let contenu = read(&racine, Family::Entity, "user").expect("user");

    let nu = prepare_entity(&contenu, &[]).expect("nu");
    let motif = match read(&racine, Family::Pattern, "versioned") {
        Ok(m) => m,
        // Le motif est facultatif dans le catalogue fourni : si personne ne
        // l'a encore écrit, le test ne prétend pas le contraire.
        Err(_) => return,
    };
    let habille = prepare_entity(&contenu, &[&motif]).expect("habillé");
    assert!(
        habille.fields.len() > nu.fields.len(),
        "le motif n'a rien ajouté : {} champs contre {}",
        habille.fields.len(),
        nu.fields.len()
    );
}

/// **Adopter, puis reposer.** Le pendant de `place` : ce qui a marché une fois
/// se repose sans le réécrire. Le test fait l'aller-retour complet dans une
/// racine de projet jetable.
#[test]
#[ignore]
fn un_gabarit_adopte_se_repose() {
    use rag3weaver::template::{write_entity, read_in, prepare_entity, roots, Header};

    let projet = tempfile::tempdir().expect("tempdir");
    let racine = projet.path();

    // Une entité comme un agent en construirait une : partie d'un gabarit
    // fourni, elle a divergé.
    let base = read_in(&roots(None), Family::Entity, "user").expect("user fourni");
    let mut config = prepare_entity(&base, &[]).expect("config");
    config.fields.insert(
        "avatarUrl".into(),
        rag3weaver::config::SimpleFieldDef {
            field_type: rag3weaver::config::FieldType::String,
            ..Default::default()
        },
    );

    let header = Header {
        category: "auth".into(),
        description: "Un compte avec son portrait : de quoi afficher qui parle.".into(),
        note: "Adopté par un test.".into(),
        ..Header::default()
    };
    let chemin = write_entity(racine, "user_avec_portrait", &config, &header).expect("écriture");
    assert!(chemin.exists(), "le gabarit n'a pas été écrit : {}", chemin.display());

    // Et il se relit par les mêmes chemins que les gabarits fournis.
    let toutes = roots(Some(racine));
    let relu = read_in(&toutes, Family::Entity, "user_avec_portrait").expect("relecture");
    let config2 = prepare_entity(&relu, &[]).expect("config relue");
    assert!(config2.fields.contains_key("avatarUrl"), "le champ ajouté a survécu à l'aller-retour");
    assert_eq!(config2.fields.len(), config.fields.len());
}

/// **À nom égal, le projet gagne.** Un dépôt doit pouvoir remplacer un gabarit
/// fourni sans demander la permission, et sans que le nom change — sinon toute
/// la chaîne (recherche, `place`, documentation) parle d'autre chose.
#[test]
#[ignore]
fn un_gabarit_de_projet_masque_celui_de_la_bibliotheque() {
    use rag3weaver::template::{write_entity, read_in, prepare_entity, roots, scan_roots, Header};

    let projet = tempfile::tempdir().expect("tempdir");
    let racine = projet.path();

    let fourni = read_in(&roots(None), Family::Entity, "user").expect("user fourni");
    let mut config = prepare_entity(&fourni, &[]).expect("config");
    config.fields.clear();
    config.fields.insert(
        "seulementCeci".into(),
        rag3weaver::config::SimpleFieldDef {
            field_type: rag3weaver::config::FieldType::String,
            is_title: true,
            ..Default::default()
        },
    );
    write_entity(
        racine,
        "user",
        &config,
        &Header {
            category: "auth".into(),
            description: "La version de ce projet, volontairement différente.".into(),
            note: String::new(),
            ..Header::default()
        },
    )
    .expect("écriture");

    let toutes = roots(Some(racine));
    let relu = prepare_entity(&read_in(&toutes, Family::Entity, "user").unwrap(), &[]).unwrap();
    assert!(relu.fields.contains_key("seulementCeci"), "c'est celui du projet qui doit gagner");
    assert!(!relu.fields.contains_key("email"), "celui de la bibliothèque ne doit pas transparaître");

    // Et il n'apparaît qu'une fois dans le catalogue : masquer n'est pas
    // ajouter. Deux fiches de même nom rendraient la recherche ambiguë.
    let fiches = scan_roots(&toutes).unwrap();
    let users: Vec<_> = fiches.iter().filter(|f| f.family == Family::Entity && f.name == "user").collect();
    assert_eq!(users.len(), 1, "un seul 'user' visible, pas deux");
    assert_eq!(users[0].description, "La version de ce projet, volontairement différente.");
}

/// **Une description vide est refusée.** C'est le champ embarqué : sans lui, le
/// gabarit existe sur le disque et reste introuvable — un agent enterrerait son
/// propre travail sans le savoir.
#[test]
#[ignore]
fn un_gabarit_sans_description_est_refuse() {
    use rag3weaver::template::{write_entity, read_in, prepare_entity, roots, Header};
    let projet = tempfile::tempdir().expect("tempdir");
    let config = prepare_entity(&read_in(&roots(None), Family::Entity, "user").unwrap(), &[]).unwrap();
    let e = write_entity(projet.path(), "muet", &config, &Header::default())
        .expect_err("une description vide doit être refusée");
    assert!(e.contains("description"), "{e}");
}

// ─── La couche utilisateur (doc 6 septembre 2026, 14h43) ────────────────────

/// Les fiches visibles par la recherche, avec leur origine, pour un filtre.
fn fiches_par_origine(cat: &Arc<std::sync::Mutex<Catalog>>, origine: &str) -> Vec<(String, String)> {
    let mut o = SearchOptions { limit: 50, ..Default::default() };
    o.filters.insert(
        "origin".to_string(),
        rag3weaver::filter::FilterValue::Direct(rag3weaver::connection::CypherValue::String(origine.into())),
    );
    let out = Catalog::rechercher(cat, TEMPLATE_ENTITY, "gabarit", o).expect("recherche");
    let mut v: Vec<(String, String)> = out
        .results
        .iter()
        .map(|r| {
            let d = r.data.as_ref().expect("data");
            (
                d.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                r.uuid.clone(),
            )
        })
        .collect();
    v.sort();
    v
}

/// **La synchronisation alimente la couche du projet, par identité.**
///
/// Avant le 6 septembre, seuls les tests indexaient des fiches ; en production
/// `search(target='Template')` ne trouvait rien parce que rien n'existait.
/// Ici : la bibliothèque entre, une seconde passe ne touche rien, un gabarit
/// de projet **prend la place** de celui qu'il masque (même `_uuid`, origine
/// changée) — même quand seule l'origine change —, un gabarit propre au
/// projet s'ajoute puis disparaît avec son fichier, et « il a bougé depuis »
/// est signalé, pas migré.
#[test]
#[ignore]
fn la_synchronisation_alimente_la_couche_du_projet() {
    use rag3weaver::template::{prepare_entity, read, roots, write_entity, Header, Origin};

    let mut catalog = setup();
    let fournis = scan(&builtin_root(), Origin::Builtin).unwrap();

    // 1. La bibliothèque entre, une fois.
    let r1 = catalog.sync_templates(None).unwrap();
    assert_eq!(r1.added.len(), fournis.len(), "{r1:?}");
    assert!(r1.updated.is_empty() && r1.removed.is_empty() && r1.stale.is_empty(), "{r1:?}");
    let r2 = catalog.sync_templates(None).unwrap();
    assert_eq!(r2.added.len() + r2.updated.len() + r2.removed.len(), 0, "{r2:?}");
    assert_eq!(r2.unchanged, fournis.len());

    // 2. Un projet masque `user` : même identité, autre couche.
    let projet = tempfile::tempdir().unwrap();
    let racine = projet.path();
    let user_fourni = read(&builtin_root(), Family::Entity, "user").unwrap();
    let hash_user_fourni = fournis.iter().find(|f| f.name == "user" && f.family == Family::Entity).unwrap().content_hash.clone();
    let config = prepare_entity(&user_fourni, &[]).unwrap();
    write_entity(
        racine,
        "user",
        &config,
        &Header {
            category: "auth".into(),
            description: "Le compte de ce projet : de quoi savoir qui est connecté, avec un portrait.".into(),
            derived_from: hash_user_fourni.clone(),
            ..Header::default()
        },
    )
    .unwrap();
    let r3 = catalog.sync_templates(Some(racine)).unwrap();
    assert_eq!(r3.updated, vec!["entity/user".to_string()], "{r3:?}");
    assert!(r3.added.is_empty() && r3.removed.is_empty(), "{r3:?}");
    assert!(r3.stale.is_empty(), "l'empreinte est celle d'aujourd'hui : rien n'a bougé — {r3:?}");

    let cat = Arc::new(std::sync::Mutex::new(catalog));
    let projet_seul = fiches_par_origine(&cat, "project");
    assert_eq!(projet_seul.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["user"]);
    let uuid_user = projet_seul[0].1.clone();
    let fournis_visibles = fiches_par_origine(&cat, "builtin");
    assert!(!fournis_visibles.iter().any(|(n, _)| n == "user"), "masquer n'est pas ajouter");
    assert!(fournis_visibles.iter().any(|(n, _)| n == "product"));

    // 3. Seule l'origine change : un `product` copié tel quel dans le projet.
    let dir = racine.join(rag3weaver::template::PROJECT_DIR).join("entities");
    std::fs::copy(builtin_root().join("entities/product.json"), dir.join("product.json")).unwrap();
    let mut catalog = Arc::try_unwrap(cat).ok().unwrap().into_inner().unwrap();
    let r4 = catalog.sync_templates(Some(racine)).unwrap();
    assert_eq!(r4.updated, vec!["entity/product".to_string()], "{r4:?}");
    let cat = Arc::new(std::sync::Mutex::new(catalog));
    let noms: Vec<String> = fiches_par_origine(&cat, "project").into_iter().map(|(n, _)| n).collect();
    assert_eq!(noms, vec!["product", "user"], "la description n'a pas changé, l'origine si");

    // 4. Un gabarit propre au projet : il s'ajoute, puis part avec son fichier.
    let mut catalog = Arc::try_unwrap(cat).ok().unwrap().into_inner().unwrap();
    write_entity(
        racine,
        "pet",
        &config,
        &Header { description: "Un animal de compagnie et son maître.".into(), ..Header::default() },
    )
    .unwrap();
    let r5 = catalog.sync_templates(Some(racine)).unwrap();
    assert_eq!(r5.added, vec!["entity/pet".to_string()], "{r5:?}");
    std::fs::remove_file(dir.join("pet.json")).unwrap();
    let r6 = catalog.sync_templates(Some(racine)).unwrap();
    assert_eq!(r6.removed, vec!["entity/pet".to_string()], "{r6:?}");

    // 5. Le fichier du projet disparaît : la fiche fournie reprend sa place,
    //    sous le même uuid.
    std::fs::remove_file(dir.join("user.json")).unwrap();
    let r7 = catalog.sync_templates(Some(racine)).unwrap();
    assert_eq!(r7.updated, vec!["entity/user".to_string()], "{r7:?}");
    let cat = Arc::new(std::sync::Mutex::new(catalog));
    let fournis_visibles = fiches_par_origine(&cat, "builtin");
    let user = fournis_visibles.iter().find(|(n, _)| n == "user").expect("user revenu à la bibliothèque");
    assert_eq!(user.1, uuid_user, "même identité des deux côtés du masque");

    // 6. « Il a bougé depuis que tu l'as pris » : une empreinte d'hier.
    let mut catalog = Arc::try_unwrap(cat).ok().unwrap().into_inner().unwrap();
    write_entity(
        racine,
        "user",
        &config,
        &Header {
            description: "Le compte de ce projet, pris sur une vieille bibliothèque.".into(),
            derived_from: "empreinte-d-hier".into(),
            ..Header::default()
        },
    )
    .unwrap();
    let r8 = catalog.sync_templates(Some(racine)).unwrap();
    assert_eq!(r8.stale, vec!["entity/user".to_string()], "{r8:?}");
    assert_eq!(r8.updated, vec!["entity/user".to_string()]);
    let _ = roots(Some(racine));
}

/// **Un agent monté comme en production trouve, pose et adopte.**
///
/// Le montage est celui de `mount_agent_services_on` — le seul, depuis le
/// 6 septembre 2026 ; avant, chaque test refaisait le sien et aucun code de
/// production n'en avait. Par les outils, pas par les fonctions : `search`
/// avec la cible `Template` trouve `user` ; `place` le pose ; `adopt` écrit,
/// **réindexe**, et se souvient de ce qu'il masque.
#[test]
#[ignore]
fn un_agent_monte_comme_en_production_trouve_pose_et_adopte() {
    use rag3weaver::agent::mount_agent_services_on;
    use rag3weaver::code_tools::{FileSource, WorkingTree};
    use rag3weaver::dataflow::graph_tool::builtin_graph_tools;
    use rag3weaver::llm::ToolCall;
    use rag3weaver::template::{scan_roots, roots, Origin};

    let projet = tempfile::tempdir().unwrap();
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(projet.path()));
    let catalog = Arc::new(std::sync::Mutex::new(setup()));
    let services = Arc::new(mount_agent_services_on(&catalog, source).unwrap());
    let (nodes, tools) = builtin_graph_tools().unwrap();
    let call = |name: &str, args: serde_json::Value| -> String {
        let tc = ToolCall { id: "c1".into(), name: name.into(), arguments: args.to_string(), provider_extra: None };
        tools.call(&tc, &nodes, services.clone()).content.clone()
    };

    // 1. Chercher : la bibliothèque est là sans qu'on l'ait indexée à la main.
    let trouve = call("search", serde_json::json!({
        "target": "Template", "query": "de quoi savoir qui est connecté sur mon site", "limit": 3
    }));
    eprintln!("[agent] search → {trouve}");
    assert!(trouve.contains("user"), "{trouve}");

    // 2. Poser.
    let pose = call("place", serde_json::json!({ "template": "user", "as": "Member" }));
    eprintln!("[agent] place → {pose}");
    assert!(pose.contains("Member"), "{pose}");
    assert!(catalog.lock().unwrap().is_registered_entity("Member"));

    // 3. Adopter sous un nom neuf : indexé aussitôt, ne masque rien.
    let adopte = call("adopt", serde_json::json!({
        "entity": "Member", "as": "member",
        "description": "Un membre du club : son adhésion, ses cotisations, ce qu'il a le droit de faire.",
        "category": "auth"
    }));
    eprintln!("[agent] adopt → {adopte}");
    assert!(adopte.contains("indexé"), "{adopte}");
    assert!(!adopte.contains("masque"), "{adopte}");
    let trouve = call("search", serde_json::json!({
        "target": "Template", "query": "adhésion et cotisations d'un membre", "limit": 3
    }));
    eprintln!("[agent] search → {trouve}");
    assert!(trouve.contains("member"), "l'adoption doit se retrouver sans autre appel — {trouve}");

    // 4. Adopter sous le nom d'un gabarit fourni : masqué, et l'empreinte gardée.
    let adopte = call("adopt", serde_json::json!({
        "entity": "Member", "as": "user",
        "description": "Le compte de ce projet : un membre, avec son adhésion.",
        "category": "auth"
    }));
    eprintln!("[agent] adopt user → {adopte}");
    assert!(adopte.contains("masque le `user` fourni"), "{adopte}");
    let fiches = scan_roots(&roots(Some(projet.path()))).unwrap();
    let user = fiches.iter().find(|f| f.family == Family::Entity && f.name == "user").unwrap();
    assert_eq!(user.origin, Origin::Project);
    let fourni = scan(&builtin_root(), Origin::Builtin).unwrap().into_iter().find(|f| f.name == "user").unwrap();
    assert_eq!(user.derived_from, fourni.content_hash, "l'empreinte d'aujourd'hui, pour dire demain qu'elle a bougé");
    let n_projet = fiches.iter().filter(|f| f.origin == Origin::Project).count();
    assert_eq!(n_projet, 2, "member et user");
}
