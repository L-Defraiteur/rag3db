//! **Un écrivain tué net laisse-t-il une base que rag3weaver sait réouvrir ?**
//!
//! La session du banc a montré, dans le moteur seul et avec un seul écrivain,
//! qu'un processus qui insère dans une table indexée par HNSW puis meurt avant
//! le point de reprise suivant laisse une base qui **plante à l'ouverture**
//! (SIGSEGV au rejeu du journal). Reste la question qui décide pour nous :
//! **le chemin réel de rag3weaver est-il touché ?**
//!
//! Trois choix de méthode, chacun pour une raison.
//!
//! **Sur disque, et pour une raison plus faible que je ne le croyais.** Le
//! dossier temporaire de ce poste est un tmpfs de 61 Gio. J'ai d'abord écrit
//! qu'un journal y vivant en RAM rendait l'épreuve vaine : **c'est faux pour un
//! SIGKILL**, et la session d'orchestration l'a corrigé. Le cache du noyau
//! survit au processus, qu'il s'agisse d'un tmpfs ou d'un disque ; la
//! différence n'apparaît qu'à une coupure de courant. Les suites voisines qui
//! utilisent `std::env::temp_dir()` prouvent donc bien ce qu'elles prétendent,
//! pour cette classe de mort. On reste ici dans `~/.cache` — c'est sans effet
//! sur le résultat, et ça garde la base examinable après coup.
//!
//! **Deux processus fils, pas un.** L'écrivain est tué par SIGKILL ; le
//! relecteur est un **second** fils, pour qu'un plantage à l'ouverture ne
//! remonte pas tuer la suite. C'est la seule façon de distinguer « plante »
//! d'« erreur nommée » : la première tue son processus, la seconde rend un
//! message.
//!
//! **Le journal est vérifié non vide avant la mort.** C'est la faute que le
//! banc avait faite : tuer un processus dont le journal est déjà replié ne
//! teste rien, et rend un vert qui ne veut rien dire.
//!
//! Et l'embarquement est un `MockEmbedder` : le défaut cherché est dans le
//! journal et l'index HNSW, pas dans la provenance des nombres. Une suite qui
//! n'a besoin ni de carte ni de service se rejoue partout, et n'accuse pas le
//! service d'embarquement d'un défaut du moteur.
//!
//! # État : un rouge connu, et ce qu'il attend
//!
//! `une_mort_apres_insertion_vectorielle` est **rouge, et c'est voulu**. Il
//! attend un moteur postérieur à `fcd9a7882` (« une base ne plante plus à
//! l'ouverture quand le journal écrit dans une table dont l'index n'est pas
//! chargé »). Tant que la bibliothèque liée est d'avant ce commit, le SIGSEGV
//! décrit le moteur d'avant — d'où la ligne d'âge imprimée à chaque cas.
//!
//! Les deux autres sont verts et le resteront : ils établissent, avec la
//! colonne `libvector` du journal, que **l'enregistrement du chargement
//! d'extension présent au journal ⟺ la base s'ouvre**. Quatre cas mesurés, une
//! seule variable, aucune exception.
//!
//! Ce qui reste à écrire est **côté produit** et ne m'appartient pas : que
//! rag3weaver reconnaisse un index détaché à l'ouverture et le rebâtisse de
//! lui-même en le disant. La sonde est là (`CATALOGUE=…`) et n'attend qu'un
//! moteur à jour pour dire ce que le montage du catalogue fait aujourd'hui.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_arret_brutal
//! ```

#![cfg(feature = "rag3db-native")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ECRIVAIN: &str = "RAG3WEAVER_ARRET_ECRIVAIN";
const RELECTEUR: &str = "RAG3WEAVER_ARRET_RELECTEUR";
/// Ce que l'écrivain fait avant de se laisser tuer.
const SCENARIO: &str = "RAG3WEAVER_ARRET_SCENARIO";

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("racine du dépôt")
            .display()
            .to_string()
    })
}

/// **Sur disque.** Voir l'en-tête : `/tmp` est de la mémoire vive ici.
fn dossier_sur_disque(cas: &str) -> PathBuf {
    let base = std::env::var("HOME").expect("HOME");
    let d = PathBuf::from(base).join(".cache/rag3weaver-build/arret-brutal").join(format!(
        "{cas}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).expect("créer le dossier");
    d
}

/// Le chemin de la base : un **fichier** dans le dossier. Le moteur refuse un
/// répertoire (« Database path cannot be a directory »), et le journal se pose
/// à côté — d'où un dossier à nous, pour pouvoir le regarder.
fn base_dans(dossier: &Path) -> PathBuf {
    dossier.join("base.rag3db")
}

/// **L'âge de la bibliothèque du moteur, dit à voix haute.**
///
/// Cette suite éprouve des correctifs **C++**. Elle se lie contre une
/// bibliothèque bâtie ailleurs, et un `SIGSEGV` contre un moteur d'avant le
/// correctif ressemble trait pour trait à une régression. Le 3 octobre au soir,
/// la garde 1 était commitée à 23h11 et la bibliothèque liée datait de 21h33 :
/// le rouge parlait du passé.
///
/// Donc on imprime la date. Un résultat qui ne dit pas contre quoi il a été
/// obtenu n'est pas un résultat.
fn age_du_moteur() {
    for chemin in [
        format!("{}/build/lecteurs-csv/src/librag3db.so", racine_moteur()),
        format!("{}/extension/vector/build/libvector.rag3db_extension", racine_moteur()),
    ] {
        match std::fs::metadata(&chemin).and_then(|m| m.modified()) {
            Ok(t) => {
                let age = std::time::SystemTime::now()
                    .duration_since(t)
                    .map(|d| format!("{} min", d.as_secs() / 60))
                    .unwrap_or_else(|_| "à venir".into());
                println!("▸ moteur lié : {chemin} (bâti il y a {age})");
            }
            Err(e) => println!("▸ moteur lié : {chemin} — illisible ({e})"),
        }
    }
}

fn charger_le_vecteur(conn: &dyn rag3weaver::connection::DbConnection) {
    let chemin = format!("{}/extension/vector/build/libvector.rag3db_extension", racine_moteur());
    assert!(Path::new(&chemin).exists(), "extension vector absente : {chemin}");
    conn.execute(&format!("LOAD EXTENSION '{chemin}'")).expect("charger vector");
}

fn fiche(avec_vecteur: bool) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("titre".to_string(), SimpleFieldDef {
        field_type: FieldType::String, is_title: true, ..Default::default()
    });
    fields.insert("corps".to_string(), SimpleFieldDef {
        field_type: FieldType::Text, is_content: true, ..Default::default()
    });
    EntityConfig {
        fields,
        signals: if avec_vecteur {
            SearchSignals::BM25 | SearchSignals::VECTOR
        } else {
            SearchSignals::BM25
        },
        hashsafe: Some(vec!["titre".into()]),
        ..Default::default()
    }
}

fn ligne(n: usize) -> std::collections::BTreeMap<String, CypherValue> {
    let mut d = std::collections::BTreeMap::new();
    d.insert("titre".into(), CypherValue::String(format!("note {n}")));
    d.insert("corps".into(), CypherValue::String(format!(
        "Un corps assez long pour faire un chunk et porter un vecteur, numéro {n}."
    )));
    d
}

fn catalogue_sur(dossier: &Path) -> Catalog {
    let conn = Rag3dbConnection::new(&base_dans(dossier).display().to_string())
        .expect("base sur disque");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    charger_le_vecteur(boxed.as_ref());
    let config = CatalogConfig {
        name: Some("arret-brutal".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().expect("initialize");
    catalog
}

/// Le journal d'écriture anticipée, et sa taille. `None` : aucun fichier de
/// journal trouvé.
fn journal(dossier: &Path) -> Option<(PathBuf, u64)> {
    let mut trouves: Vec<(PathBuf, u64)> = Vec::new();
    // Le journal se pose à côté de la base, et le moteur fait parfois de la
    // base un répertoire : on regarde les deux plutôt que de supposer.
    for racine in [dossier.to_path_buf(), base_dans(dossier)] {
        let Ok(lecture) = std::fs::read_dir(&racine) else { continue };
        for entree in lecture.flatten() {
            let chemin = entree.path();
            let nom = chemin.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if nom.contains("wal") {
                let taille = entree.metadata().map(|m| m.len()).unwrap_or(0);
                trouves.push((chemin, taille));
            }
        }
    }
    trouves.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    trouves.into_iter().next()
}

/// **Un point de reprise a-t-il eu lieu ?** On ne peut pas le demander, donc
/// on le regarde : la taille du journal ne retombe jamais d'elle-même. Un
/// échantillon toutes les deux millisecondes, et on retient si une baisse a été
/// vue depuis un maximum non nul.
///
/// Sans cet instrument, « sans point de reprise » reste une intention, et une
/// intention n'est pas une mesure — c'est l'erreur qui a rendu mon premier
/// témoin muet.
struct Guetteur {
    repli: std::sync::Arc<std::sync::atomic::AtomicBool>,
    arret: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Guetteur {
    fn repli_vu(&self) -> bool {
        self.arret.store(true, std::sync::atomic::Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.repli.load(std::sync::atomic::Ordering::Relaxed)
    }
}

fn surveiller_le_journal(dossier: &Path) -> Guetteur {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let repli = Arc::new(AtomicBool::new(false));
    let arret = Arc::new(AtomicBool::new(false));
    let (r, a, d) = (repli.clone(), arret.clone(), dossier.to_path_buf());
    std::thread::spawn(move || {
        let mut max = 0u64;
        while !a.load(Ordering::Relaxed) {
            if let Some((_, taille)) = journal(&d) {
                if taille > max {
                    max = taille;
                } else if max > 0 && taille < max {
                    r.store(true, Ordering::Relaxed);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    });
    Guetteur { repli, arret }
}

/// Ce qu'une réouverture a donné.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// S'ouvre, et le contenu attendu est là.
    OuvreEtJuste { lignes: usize },
    /// S'ouvre, mais l'index ne rend pas ce qu'il devrait.
    OuvreIndexFaux { lignes: usize, trouves: usize },
    /// S'ouvre, les lignes sont là, et **l'index se déclare en retard** —
    /// « is behind its table ». C'est ce que la garde 1 promet.
    OuvreIndexEnRetard { lignes: usize },
    /// S'ouvre, et **l'index était détaché puis a été rebâti par le chemin du
    /// produit** — la recherche vectorielle refusait avant le montage et rend
    /// des lignes après. C'est le seul vert qui prouve la réparation ; un
    /// [`Verdict::OuvreEtJuste`] sur ce cas-là veut dire que le témoin n'a pas
    /// atteint l'état qu'il prétend éprouver.
    OuvreIndexRebati { lignes: usize },
    /// Une erreur, nommée, rendue proprement.
    ErreurNommee(String),
    /// Le processus est mort — signal ou code non nul sans message.
    Plantage(String),
}

fn relire(dossier: &Path, cas: &str, attendu: usize) -> Verdict {
    let sortie = std::process::Command::new(std::env::current_exe().expect("current_exe"))
        .args(["--exact", cas, "--nocapture", "--ignored"])
        .env(RELECTEUR, dossier.display().to_string())
        .env("RAG3WEAVER_ARRET_ATTENDU", attendu.to_string())
        .output()
        .expect("lancer le relecteur");
    let texte = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );
    // **Ce que le relecteur a vu doit remonter.** Il imprime ses sondes dans un
    // processus fils dont la sortie est capturée ici ; sans ce relais elles
    // n'existent que dans une chaîne que personne ne lit — exactement le
    // défaut qu'on a passé la soirée à nommer, « une information existe et
    // rien ne la consulte ». La première passe avec les sondes ne m'a rien
    // appris pour cette seule raison.
    for l in texte.lines().filter(|l| {
        [
            "INDEX_VUS=",
            "INDEX_CIBLE=",
            "VECTEUR=",
            "SONDE_CREATE=",
            "CATALOGUE=",
            "VECTEUR_APRES=",
        ]
            .iter()
            .any(|p| l.starts_with(p))
    }) {
        println!("\u{25b8} [sonde] {l}");
    }

    if let Some(l) = texte.lines().find(|l| l.starts_with("VERDICT=")) {
        let corps = l.trim_start_matches("VERDICT=");
        if let Some(reste) = corps.strip_prefix("JUSTE ") {
            return Verdict::OuvreEtJuste { lignes: reste.trim().parse().unwrap_or(0) };
        }
        if let Some(reste) = corps.strip_prefix("REBATI ") {
            return Verdict::OuvreIndexRebati { lignes: reste.trim().parse().unwrap_or(0) };
        }
        if let Some(reste) = corps.strip_prefix("EN_RETARD ") {
            return Verdict::OuvreIndexEnRetard { lignes: reste.trim().parse().unwrap_or(0) };
        }
        if let Some(reste) = corps.strip_prefix("INDEX_FAUX ") {
            let mut it = reste.split_whitespace();
            return Verdict::OuvreIndexFaux {
                lignes: it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
                trouves: it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            };
        }
        if let Some(reste) = corps.strip_prefix("ERREUR ") {
            return Verdict::ErreurNommee(reste.trim().to_string());
        }
    }
    // Pas de verdict : le relecteur est mort avant de parler.
    Verdict::Plantage(format!(
        "statut {:?} — dernières lignes :\n{}",
        sortie.status,
        texte.lines().rev().take(6).collect::<Vec<_>>().join("\n")
    ))
}

/// Le rôle du relecteur, joué dans un second fils.
fn jouer_le_relecteur() -> Option<()> {
    let dossier = std::env::var(RELECTEUR).ok()?;
    let attendu: usize = std::env::var("RAG3WEAVER_ARRET_ATTENDU")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // Tout ce qui suit peut mourir : c'est l'objet du test.
    let dossier = PathBuf::from(dossier);
    let conn = match Rag3dbConnection::new(&base_dans(&dossier).display().to_string()) {
        Ok(c) => c,
        Err(e) => {
            println!("VERDICT=ERREUR ouverture : {e}");
            std::process::exit(0);
        }
    };
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    charger_le_vecteur(boxed.as_ref());
    let lignes = boxed
        .execute("MATCH (n:Fiche) RETURN count(n)")
        .ok()
        .and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()))
        .unwrap_or(-1);
    if lignes < 0 {
        println!("VERDICT=ERREUR la table Fiche est illisible");
        std::process::exit(0);
    }
    let lignes = lignes as usize;

    // ── L'état brut de l'index, avant toute redéclaration ────────────────
    //
    // Trois questions **séparées**, parce que leurs réponses demandent des
    // remèdes différents et qu'un « ça marche » global les confond : l'index
    // est-il encore là ? peut-on y chercher ? peut-on le recréer ?
    //
    // **L'ordre n'est pas libre.** La sonde `CREATE … skip_if_exists` *répare*
    // un index absent ; elle doit donc venir après les deux observations,
    // sinon elle efface ce qu'elles allaient voir. C'est aussi pourquoi elle
    // est imprimée : une sonde qui modifie ce qu'elle mesure doit le dire.
    //
    // Pourquoi d'abord en Cypher brut et pas par le catalogue : le montage
    // redéclare le schéma, et son `CREATE_VECTOR_INDEX` recrée l'index. Le
    // 3 octobre au soir, c'est ce qui a rendu un vert illisible — il ne
    // distinguait pas « la garde a réparé » de « je l'ai recréé moi-même ».
    let lignes_index: Vec<Vec<String>> = match boxed.execute("CALL SHOW_INDEXES() RETURN *") {
        Ok(r) => {
            let v: Vec<Vec<String>> = r
                .rows
                .iter()
                .map(|l| {
                    l.iter()
                        .map(|c| c.as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("{c:?}")))
                        .collect()
                })
                .collect();
            println!(
                "INDEX_VUS={}",
                v.iter().map(|l| l.join("/")).collect::<Vec<_>>().join(" | ")
            );
            v
        }
        Err(e) => {
            println!("INDEX_VUS=refus: {e}");
            Vec::new()
        }
    };

    // La cible : la ligne qui parle de `Fiche_Chunk`, et dans cette ligne la
    // cellule qui n'est ni le nom de table ni un type connu. Heuristique
    // assumée — et c'est pour ça qu'elle est imprimée : si elle se trompe,
    // `INDEX_VUS` juste au-dessus permet de le voir sans relancer.
    let cible = lignes_index
        .iter()
        .find(|l| l.iter().any(|c| c == "Fiche_Chunk"))
        .and_then(|l| {
            l.iter()
                .find(|c| {
                    !c.is_empty()
                        && c.as_str() != "Fiche_Chunk"
                        && !c.eq_ignore_ascii_case("HNSW")
                        && !c.eq_ignore_ascii_case("VECTOR")
                        && !c.eq_ignore_ascii_case("FTS")
                })
                .cloned()
        });
    match &cible {
        Some(nom) => println!("INDEX_CIBLE={nom}"),
        None => println!(
            "INDEX_CIBLE=inconnu — les deux sondes suivantes ne sont pas jouées \
             (ce n'est pas « elles passent » : c'est « on ne sait pas »)"
        ),
    }

    let mut vecteur_avant = "non jouée (index cible inconnu)".to_string();
    if let Some(nom) = &cible {
        // 1. Chercher dedans. C'est la question de l'agent : une recherche
        //    vectorielle rend-elle quelque chose, ou refuse-t-elle ?
        //
        //    Et on **garde** sa réponse : c'est elle qui dira, à la fin, si ce
        //    témoin a bien atteint l'état qu'il prétend éprouver. Un test qui
        //    imprime sa sonde sans la relire juge sur autre chose qu'elle.
        vecteur_avant = match boxed.execute(&format!(
            "CALL QUERY_VECTOR_INDEX('Fiche_Chunk', '{nom}', [0.1, 0.2, 0.3, 0.4], 3) \
             RETURN node._uuid"
        )) {
            Ok(r) => format!("ok {} lignes", r.rows.len()),
            Err(e) => format!("refus: {e}"),
        };
        println!("VECTEUR={vecteur_avant}");

        // 2. Le recréer sans l'avoir retiré. C'est l'appel que fait l'étape 4
        //    d'`initialize`, et celui que le moteur refuse sur un index
        //    détaché — `skip_if_exists` ou non.
        let recreation = boxed.execute(&format!(
            "CALL CREATE_VECTOR_INDEX('Fiche_Chunk', '{nom}', 'embedding__mockembedder', \
             metric := 'cosine', skip_if_exists := true)"
        ));
        match recreation {
            Ok(_) => println!("SONDE_CREATE=ok (l'index était sain, ou absent et vient d'être recréé)"),
            Err(e) => println!("SONDE_CREATE=refus: {e}"),
        }
    }

    // **La garde 1 promet trois choses** : la base s'ouvre, la table a ses
    // lignes, et l'index se déclare en retard au lieu de planter. Les deux
    // premières sont dites ci-dessus ; la troisième se sonde ici.
    //
    // Et on la sonde **par le chemin du produit**, pas en Cypher brut : la
    // question posée est « rag3weaver est-il touché », et c'est son montage de
    // catalogue — qui crée ses index à l'initialisation — qui rencontrera
    // l'index détaché. Ce que ce montage fait ou ne fait pas est précisément ce
    // qu'il reste à écrire côté produit.
    let cible_apres = cible.clone();
    let montage = std::panic::catch_unwind(|| {
        let conn = Rag3dbConnection::new(&base_dans(&dossier).display().to_string())
            .map_err(|e| e.to_string())?;
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        charger_le_vecteur(boxed.as_ref());
        let config = CatalogConfig {
            name: Some("arret-brutal".into()),
            entities: HashMap::new(),
            relations: HashMap::new(),
            embedding_dim: 4,
            ..Default::default()
        };
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().map_err(|e| e.to_string())?;
        catalog.register_entity("Fiche", fiche(true)).map_err(|e| e.to_string())?;
        // **Le verdict utile : peut-on chercher APRÈS le montage ?**
        //
        // Les sondes d'avant disaient l'état *trouvé* ; celle-ci dit l'état
        // *laissé*. Et c'est elle qui manquait : le 4 octobre 2026, le montage
        // annonçait `CATALOGUE=ok` dans la même seconde où une recherche
        // vectorielle refusait par « is behind its table ». Un montage qui
        // passe ne prouve pas qu'on peut chercher — il prouve seulement que
        // personne n'a regardé.
        //
        // Sur la connexion du catalogue, pas sur la nôtre : la réparation a
        // lieu dans celle-là, et une autre connexion pourrait en avoir une vue
        // en retard.
        let apres = match &cible_apres {
            Some(nom) => match catalog.execute_raw(&format!(
                "CALL QUERY_VECTOR_INDEX('Fiche_Chunk', '{nom}', [0.1, 0.2, 0.3, 0.4], 3) \
                 RETURN node._uuid"
            )) {
                Ok(r) => format!("ok {} lignes", r.rows.len()),
                Err(e) => format!("refus: {e}"),
            },
            None => "non jouée (index cible inconnu)".to_string(),
        };
        Ok::<String, String>(apres)
    });
    let (montage, vecteur_apres) = match montage {
        Ok(Ok(apres)) => ("ok".to_string(), apres),
        Ok(Err(e)) => (format!("erreur: {e}"), "non jouée (le montage a échoué)".to_string()),
        Err(_) => ("panique".to_string(), "non jouée (le montage a paniqué)".to_string()),
    };
    println!("CATALOGUE={montage}");
    println!("VECTEUR_APRES={vecteur_apres}");
    // **L'état laissé, pas l'état trouvé.** Le montage pouvait annoncer `ok`
    // sur une base dont la recherche refusait : juger sur lui seul, c'était
    // juger sur ce que personne ne regardait. On garde les deux sources — un
    // montage qui remonte le refus, et une recherche qui refuse encore après —
    // parce qu'un verdict doit se tromper du côté sévère.
    let en_retard = montage.contains("is behind its table")
        || vecteur_apres.contains("is behind its table");

    // **« Juste » et « rebâti » ne sont pas le même vert.** Le premier dit que
    // rien n'était cassé — donc que le témoin n'a pas atteint son état. Le
    // second dit que l'index était détaché et que le chemin du produit l'a
    // réparé : c'est le seul qui prouve quelque chose.
    let etait_detache = vecteur_avant.contains("is behind its table");
    let repare = etait_detache && vecteur_apres.starts_with("ok");

    if lignes != attendu {
        println!("VERDICT=INDEX_FAUX {lignes} {attendu}");
    } else if en_retard {
        println!("VERDICT=EN_RETARD {lignes}");
    } else if repare {
        println!("VERDICT=REBATI {lignes}");
    } else {
        println!("VERDICT=JUSTE {lignes}");
    }
    std::process::exit(0);
}

/// Le rôle de l'écrivain : écrire, se signaler prêt, puis attendre la mort.
fn jouer_l_ecrivain() -> Option<()> {
    let dossier = std::env::var(ECRIVAIN).ok()?;
    let scenario = std::env::var(SCENARIO).unwrap_or_else(|_| "insertion".into());
    let dossier = PathBuf::from(dossier);
    let avec_vecteur = scenario != "bm25";

    // **Le témoin demande deux sessions, et c'est tout son objet.** Mon
    // premier essai créait la table *et son index* dans la session qui meurt :
    // or `CREATE_VECTOR_INDEX` écrit son propre point de reprise, et la
    // première ingestion passe par un `COPY` qui en pose un aussi. « Sans
    // `CHECKPOINT` explicite » n'est donc pas « sans point de reprise », et ce
    // témoin-là ne prouvait rien — il a failli me faire annoncer que la cause
    // était réfutée.
    //
    // Ici : session 1 crée tout, ingère, **ferme proprement** ; session 2
    // ouvre, ingère, meurt. Et une surveillance du journal dit si un point de
    // reprise a eu lieu dans la seconde — sans quoi on recommencerait la même
    // erreur en la croyant corrigée.
    if scenario == "sans_reprise" {
        let mut session1 = catalogue_sur(&dossier);
        session1.register_entity("Fiche", fiche(true)).expect("register");
        session1.ingest_entities("Fiche", (0..3).map(ligne).collect()).expect("session 1");
        session1.execute_raw("CHECKPOINT").expect("replier avant de fermer");
        drop(session1);

        let mut session2 = catalogue_sur(&dossier);
        let guetteur = surveiller_le_journal(&dossier);
        session2.ingest_entities("Fiche", (3..6).map(ligne).collect()).expect("session 2");
        let repli = guetteur.repli_vu();
        println!("REPLI={}", if repli { "oui" } else { "non" });
        println!("PRET");
        use std::io::Write;
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(120));
        std::process::exit(3);
    }

    let avec_reprise = true;
    let mut catalog = catalogue_sur(&dossier);
    catalog.register_entity("Fiche", fiche(avec_vecteur)).expect("register");

    // Premier lot, puis un point de reprise : ce qui précède est donc replié,
    // et ce qui suit est ce que la mort doit mettre en jeu.
    //
    // **Le point de reprise est le cœur de la cause**, trouvée par la session
    // cœur C++ : il vide le journal, l'enregistrement `LOAD EXTENSION` compris.
    // Le rejeu d'après rencontre alors un index HNSW connu mais pas chargé, et
    // suit un pointeur nul. D'où le témoin `sans_reprise`, qui doit survivre.
    catalog.ingest_entities("Fiche", (0..3).map(ligne).collect()).expect("premier lot");
    if avec_reprise {
        catalog.execute_raw("CHECKPOINT").expect("point de reprise");
    }

    // Second lot, **sans** point de reprise : il ne vit que dans le journal.
    match scenario.as_str() {
        "insertion" | "bm25" | "sans_reprise" => {
            catalog.ingest_entities("Fiche", (3..6).map(ligne).collect()).expect("second lot");
        }
        autre => panic!("scénario inconnu : {autre}"),
    }

    println!("PRET");
    use std::io::Write;
    std::io::stdout().flush().ok();
    // On attend d'être tué. Une borne, pour qu'un parent mort ne laisse pas
    // un orphelin écrire indéfiniment.
    std::thread::sleep(std::time::Duration::from_secs(120));
    std::process::exit(3);
}

/// Joue un cas : lance l'écrivain, attend qu'il soit prêt, vérifie que le
/// journal porte quelque chose, le tue, et relit dans un second fils.
///
/// Rendu séparément du jugement : chaque cas décide lui-même de ce qu'il
/// attend, et un cas qui attend un plantage est aussi légitime qu'un cas qui
/// attend un succès — tant qu'il le dit.
fn jouer(cas: &str, scenario: &str) -> (Verdict, String) {
    age_du_moteur();
    let dossier = dossier_sur_disque(cas);
    let mut enfant = std::process::Command::new(std::env::current_exe().expect("current_exe"))
        .args(["--exact", cas, "--nocapture", "--ignored"])
        .env(ECRIVAIN, dossier.display().to_string())
        .env(SCENARIO, scenario)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("lancer l'écrivain");

    let mut lu = String::new();
    {
        use std::io::{BufRead, BufReader};
        let sortie = enfant.stdout.take().expect("stdout de l'écrivain");
        let mut lecteur = BufReader::new(sortie);
        let debut = std::time::Instant::now();
        while debut.elapsed() < std::time::Duration::from_secs(90) {
            let mut ligne = String::new();
            if lecteur.read_line(&mut ligne).unwrap_or(0) == 0 {
                break;
            }
            lu.push_str(&ligne);
            if ligne.trim() == "PRET" {
                break;
            }
        }
    }
    assert!(lu.contains("PRET"), "l'écrivain n'a jamais été prêt :\n{lu}");
    // **Ce que l'écrivain a dit avant de mourir est rendu, pas jeté.** La
    // première version lisait sa sortie pour y guetter « PRET » et perdait le
    // reste — dont `REPLI`, c'est-à-dire la preuve que la mesure valait
    // quelque chose. Un instrument dont personne ne lit le cadran n'est pas un
    // instrument.
    for ligne in lu.lines().filter(|l| !l.trim().is_empty() && l.trim() != "PRET") {
        println!("▸ [{scenario}] l'écrivain dit : {}", ligne.trim());
    }

    // **La vérification qui donne son sens au test** : le journal doit porter
    // quelque chose au moment de la mort. Le replier d'abord, c'est tuer un
    // processus qui n'a plus rien à perdre, et appeler ça une épreuve.
    let avant = journal(&dossier);
    let (chemin_wal, taille) = avant.clone().unwrap_or((PathBuf::new(), 0));
    println!("▸ [{scenario}] journal avant la mort : {taille} octets ({})", chemin_wal.display());

    let _ = enfant.kill();
    let _ = enfant.wait();

    assert!(
        avant.is_some() && taille > 0,
        "journal vide ou absent au moment de la mort : le test ne prouverait rien (fichiers : {:?})",
        [dossier.to_path_buf(), base_dans(&dossier)]
            .iter()
            .flat_map(|r| std::fs::read_dir(r).into_iter().flatten().flatten())
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    let verdict = relire(&dossier, cas, 6);
    println!("▸ [{scenario}] verdict : {verdict:?}   (base conservée : {})", dossier.display());
    (verdict, lu)
}

/// **Le témoin : sans point de reprise entre le chargement de l'extension et
/// l'écriture, la base doit survivre.**
///
/// C'est la contre-épreuve de la cause trouvée par la session cœur C++ — le
/// point de reprise vide le journal, enregistrement `LOAD EXTENSION` compris.
/// Sans lui, l'enregistrement est encore là au rejeu, l'extension est chargée,
/// et il n'y a pas de pointeur nul à suivre. **Vert attendu dès aujourd'hui** :
/// si ce cas est rouge, la cause n'est pas celle-là.
#[test]
#[ignore]
fn un_temoin_sans_point_de_reprise_survit() {
    if jouer_le_relecteur().is_some() || jouer_l_ecrivain().is_some() {
        return;
    }
    let (verdict, dit) = jouer("un_temoin_sans_point_de_reprise_survit", "sans_reprise");
    // **La validité d'abord.** Un témoin qui s'appelle « sans point de
    // reprise » et n'en vérifie rien est une affirmation déguisée en mesure :
    // mon premier essai créait l'index dans la session qui meurt, et
    // `CREATE_VECTOR_INDEX` pose son propre point de reprise. Le verdict ne se
    // lit qu'après cette ligne.
    assert!(
        dit.contains("REPLI=non"),
        "un point de reprise a eu lieu dans la session qui meurt : ce témoin ne prouve \
         rien, quel que soit son verdict. L'écrivain a dit :\n{dit}"
    );
    match verdict {
        Verdict::OuvreEtJuste { lignes } => assert_eq!(lignes, 6),
        autre => panic!(
            "le témoin devait survivre : sans point de reprise, l'enregistrement du \
             chargement d'extension est encore au journal. Si ce cas tombe, la cause \
             n'est pas le repli du journal. Reçu : {autre:?}"
        ),
    }
}

/// **La contre-épreuve : une entité sans vecteur survit-elle ?**
///
/// Si oui, le défaut est propre à l'**index HNSW** et non au journal en
/// général — ce qui est une affirmation bien plus utile que « le WAL casse sur
/// arrêt brutal », que la mémoire du dépôt portait sans la préciser. Si non,
/// c'est le journal, et il faut le dire aussi.
#[test]
#[ignore]
fn une_mort_sans_vecteur_est_elle_epargnee() {
    if jouer_le_relecteur().is_some() || jouer_l_ecrivain().is_some() {
        return;
    }
    match jouer("une_mort_sans_vecteur_est_elle_epargnee", "bm25").0 {
        Verdict::OuvreEtJuste { lignes } => assert_eq!(lignes, 6),
        autre => panic!(
            "une entité en BM25 seul ne survit pas non plus : le défaut n'est donc pas \
             propre à l'index vectoriel, c'est le journal. Reçu : {autre:?}"
        ),
    }
}

/// **Mort après une insertion vectorielle, le journal non replié.**
///
/// Le cas que la session du banc a vu casser dans le moteur seul. Ici par le
/// chemin de rag3weaver : son catalogue, son chargement d'extension, son
/// ingestion.
#[test]
#[ignore]
fn une_mort_apres_insertion_vectorielle() {
    if jouer_le_relecteur().is_some() || jouer_l_ecrivain().is_some() {
        return;
    }
    let cas = "une_mort_apres_insertion_vectorielle";
    let dossier = dossier_sur_disque(cas);

    let mut enfant = std::process::Command::new(std::env::current_exe().expect("current_exe"))
        .args(["--exact", cas, "--nocapture", "--ignored"])
        .env(ECRIVAIN, dossier.display().to_string())
        .env(SCENARIO, "insertion")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("lancer l'écrivain");

    // Attendre « PRET » sur sa sortie : sans ça on tuerait au hasard.
    let mut lu = String::new();
    {
        use std::io::{BufRead, BufReader};
        let sortie = enfant.stdout.take().expect("stdout de l'écrivain");
        let mut lecteur = BufReader::new(sortie);
        let debut = std::time::Instant::now();
        while debut.elapsed() < std::time::Duration::from_secs(90) {
            let mut ligne = String::new();
            if lecteur.read_line(&mut ligne).unwrap_or(0) == 0 {
                break;
            }
            lu.push_str(&ligne);
            if ligne.trim() == "PRET" {
                break;
            }
        }
    }
    assert!(lu.contains("PRET"), "l'écrivain n'a jamais été prêt :\n{lu}");

    // **La vérification qui donne son sens au test** : le journal doit porter
    // quelque chose au moment de la mort. Le replier d'abord, c'est tuer un
    // processus qui n'a plus rien à perdre, et appeler ça une épreuve.
    let avant = journal(&dossier);
    let (chemin_wal, taille) = avant.clone().unwrap_or((PathBuf::new(), 0));
    println!("▸ journal avant la mort : {} octets ({})", taille, chemin_wal.display());

    let _ = enfant.kill();
    let _ = enfant.wait();

    assert!(
        avant.is_some() && taille > 0,
        "journal vide ou absent au moment de la mort : le test ne prouverait rien \
         (fichiers : {:?})",
        [dossier.to_path_buf(), base_dans(&dossier)]
            .iter()
            .flat_map(|r| std::fs::read_dir(r).into_iter().flatten().flatten())
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    let verdict = relire(&dossier, cas, 6);
    println!("▸ verdict : {verdict:?}");

    // **Ce que la garde 1 promet**, et rien de plus : la base s'ouvre, la table
    // a ses six lignes, et l'index se **déclare** en retard au lieu de planter.
    //
    // L'index juste sans rien rebâtir est l'attendu de la garde 2 : il n'est
    // pas écrit ici, parce que sa cause vient d'être reformulée et qu'un
    // attendu écrit pour un remède qu'on ne connaît pas encore est un attendu
    // qu'il faudra réécrire.
    match verdict {
        // **Le seul vert qui prouve quelque chose.** L'index était détaché —
        // la recherche vectorielle refusait par « is behind its table » avant
        // le montage — et il rend des lignes après. Mesuré le 4 octobre 2026,
        // les deux pôles dans la même exécution.
        Verdict::OuvreIndexRebati { lignes } => {
            assert_eq!(lignes, 6, "les six lignes doivent être là");
        }
        // **Pas un succès : le témoin n'a pas atteint son état.** La base
        // s'ouvre et les lignes sont là, mais l'index n'était pas détaché,
        // donc rien de la réparation n'a été éprouvé. Taire ce cas, c'est
        // rendre un vert qui parle d'autre chose que de la question posée —
        // ce qui est précisément arrivé la veille.
        //
        // **Et ce rouge deviendra l'attendu**, le jour où la garde 2 du cœur
        // C++ arrivera (le rejeu charge l'extension avant de rejouer) : l'index
        // sera juste dès la réouverture, sans rien rebâtir. Annoncé par la
        // session cœur C++ le 4 octobre 2026.
        //
        // **Mais ce ne sera pas une simple inversion**, et c'est le piège à
        // éviter ce jour-là. La garde 2 ne peut que demander au rejeu de
        // charger ce qui est noté : si le chargement échoue — un déploiement
        // qui oublie le fichier d'extension, un chemin qui bouge, une version
        // qui ne se charge plus —, le rejeu continue sans elle et c'est la
        // garde 1 qui joue. Donc l'attendu devient **l'un ou l'autre selon que
        // l'extension se charge**, et ce témoin-ci devra être joué en deux
        // variantes : fichier d'extension en place (sain d'emblée) et fichier
        // déplacé avant la réouverture (détaché puis rebâti).
        //
        // Ce cas-là n'est pas écrit aujourd'hui, exprès : un attendu écrit pour
        // un remède qu'on n'a pas vu est un attendu qu'il faudra réécrire. Ce
        // qui est écrit, c'est qu'il **manque**.
        Verdict::OuvreEtJuste { lignes } => panic!(
            "le témoin n'a pas atteint l'état qu'il éprouve : l'index n'était pas détaché \
             ({lignes} lignes, base ouverte, recherche vectorielle déjà saine avant le \
             montage). La réparation n'est donc **pas** mesurée par cette exécution. \
             Vérifier que le journal a bien été replié avant la mort (le `CHECKPOINT` du \
             scénario « insertion »). SI LA GARDE 2 DU CŒUR C++ EST ARRIVÉE, ce n'est \
             pas un échec : l'attendu de ce cas devient `OuvreEtJuste`, et c'est cette \
             branche qu'il faut inverser.\ndossier conservé : {}",
            dossier.display()
        ),
        // L'index est resté détaché après le montage : la réparation
        // d'ouverture n'a pas joué. C'est le rouge que ce test existe pour
        // rendre, et il nomme le coupable.
        Verdict::OuvreIndexEnRetard { lignes } => panic!(
            "l'index est resté détaché après le chemin du produit ({lignes} lignes) : \
             `Catalog::ensure_vector_index` n'a pas reconnu le refus, ou le `DROP` devant \
             le `CREATE` n'a pas eu lieu.\ndossier conservé : {}",
            dossier.display()
        ),
        autre => panic!(
            "attendu : une base qui s'ouvre, six lignes, et un index détaché puis rebâti ; \
             reçu : {autre:?}\ndossier conservé pour examen : {}",
            dossier.display()
        ),
    }
}
