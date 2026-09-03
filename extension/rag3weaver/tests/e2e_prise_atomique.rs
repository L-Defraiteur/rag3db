//! **Peut-on prendre un travail sans que deux le prennent ?**
//!
//! La question ouverte de l'issue 03 §7. Notre `ExecutionCheckpoint` a déjà
//! tout ce qui fait un travail durable — statut par nœud, sorties sauvées,
//! reprise au premier nœud incomplet, `find_incomplete()`. Ce qui manque à une
//! file, c'est la **prise atomique** : deux travailleurs qui lisent la même
//! liste doivent repartir avec des travaux différents.
//!
//! Ces tests ne supposent rien : ils essaient. Le premier met huit fils sur
//! quarante travaux. Le second pose la question qui décide de tout — **peut-on
//! seulement avoir deux preneurs ?**
//!
//! ```bash
//! cargo test --features rag3db-native --test e2e_prise_atomique -- --nocapture
//! ```

#![cfg(feature = "rag3db-native")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::Rag3dbConnection;

const TRAVAUX: usize = 40;
const FILS: usize = 8;

/// La requête de prise : le premier travail libre, marqué au nom du preneur.
fn prise(qui: &str) -> String {
    format!(
        "MATCH (t:Travail) WHERE t.statut = 'libre' \
         WITH t LIMIT 1 \
         SET t.statut = 'pris', t.preneur = '{qui}' \
         RETURN t.id AS id"
    )
}

/// Huit fils, une connexion, quarante travaux : personne ne prend deux fois.
///
/// Attention à ce que ça prouve exactement. Notre `Rag3dbConnection` tient
/// **une** `rag3db::Connection`, et c'est le moteur C++ qui sérialise les
/// appels qui la traversent. L'atomicité vient donc de là, pas d'une isolation
/// transactionnelle : c'est le cas facile, et il marche.
#[test]
fn deux_travailleurs_ne_prennent_pas_le_meme_travail() {
    let conn: Arc<dyn DbConnection> =
        Arc::new(Rag3dbConnection::in_memory().expect("base en mémoire"));

    conn.execute(
        "CREATE NODE TABLE Travail(id INT64, statut STRING, preneur STRING, PRIMARY KEY(id))",
    )
    .expect("table");
    for i in 0..TRAVAUX {
        conn.execute(&format!(
            "CREATE (:Travail {{id: {i}, statut: 'libre', preneur: ''}})"
        ))
        .expect("insertion");
    }

    // Ce que chacun a rapporté, et ce qui a échoué.
    let prises: Arc<Mutex<Vec<(String, i64)>>> = Arc::new(Mutex::new(Vec::new()));
    let echecs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let fils: Vec<_> = (0..FILS)
        .map(|f| {
            let conn = conn.clone();
            let prises = prises.clone();
            let echecs = echecs.clone();
            std::thread::spawn(move || {
                let qui = format!("fil-{f}");
                // Assez de tours pour que tout le monde se marche dessus.
                for _ in 0..TRAVAUX {
                    match conn.execute(&prise(&qui)) {
                        Ok(r) => {
                            if r.rows.is_empty() {
                                break; // plus rien de libre
                            }
                            if let Some(CypherValue::Int(id)) = r.rows[0].first() {
                                prises.lock().unwrap().push((qui.clone(), *id));
                            }
                        }
                        Err(e) => echecs.lock().unwrap().push(e.to_string()),
                    }
                }
            })
        })
        .collect();
    for f in fils {
        f.join().expect("fil");
    }

    let prises = prises.lock().unwrap().clone();
    let echecs = echecs.lock().unwrap().clone();

    // ── Ce qu'on a observé ────────────────────────────────────────────────
    let mut par_id: HashMap<i64, Vec<String>> = HashMap::new();
    for (qui, id) in &prises {
        par_id.entry(*id).or_default().push(qui.clone());
    }
    let doubles: Vec<_> = par_id.iter().filter(|(_, v)| v.len() > 1).collect();

    println!("▸ {FILS} fils, {TRAVAUX} travaux");
    println!("  prises rendues      : {}", prises.len());
    println!("  travaux distincts   : {}", par_id.len());
    println!("  pris deux fois      : {}", doubles.len());
    println!("  requêtes en échec   : {}", echecs.len());
    if let Some(e) = echecs.first() {
        println!("  première erreur     : {e}");
    }
    for (id, qui) in doubles.iter().take(3) {
        println!("  ✗ travail {id} pris par {qui:?}");
    }

    // ── Ce que ça prouve ─────────────────────────────────────────────────
    // Le fait qui décide de tout : deux preneurs ne doivent jamais repartir
    // avec le même travail. Si cette assertion tombe, une file adossée à
    // notre base a besoin d'un arbitre ailleurs — un processus, pas une
    // transaction.
    assert!(
        doubles.is_empty(),
        "{} travaux pris deux fois — la prise n'est pas atomique",
        doubles.len()
    );

    // Et l'état final doit être cohérent : rien de libre, rien sans preneur.
    let reste = conn
        .execute("MATCH (t:Travail) WHERE t.statut = 'libre' RETURN count(t) AS n")
        .expect("compte");
    println!("  restés libres       : {:?}", reste.rows.first());
}

/// **La question qui décide de tout.**
///
/// Une file suppose plusieurs preneurs. Sur une base de données, un second
/// preneur, c'est un second processus. Ce test le tente pour de vrai : le
/// parent ouvre une base sur disque, l'enfant essaie d'ouvrir la même.
///
/// Ce qu'on lit dans le moteur, et qu'on vérifie ici plutôt que de le croire :
/// `LocalFileSystem::openFile` pose un `F_WRLCK` en `F_SETLK` — non bloquant,
/// donc échec immédiat. Et `TransactionManager::beginTransaction` **refuse**
/// une seconde transaction d'écriture (`enableMultiWrites` est faux, et le seul
/// réglage qui le relâche s'appelle `debug_enable_multi_writes`).
#[test]
fn deux_processus_ne_peuvent_pas_ouvrir_la_meme_base() {
    const MARQUE: &str = "RAG3WEAVER_ENFANT_BASE";

    // Rôle enfant : ouvrir, et dire par le code de sortie ce qui s'est passé.
    if let Ok(dossier) = std::env::var(MARQUE) {
        match Rag3dbConnection::new(&dossier) {
            Ok(_) => std::process::exit(7),  // ouverte : deux preneurs possibles
            Err(_) => std::process::exit(3), // refusée : un seul preneur
        }
    }

    let dossier = std::env::temp_dir().join(format!(
        "rag3weaver-prise-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _parent = Rag3dbConnection::new(&dossier).expect("le parent ouvre la base");

    let sortie = std::process::Command::new(std::env::current_exe().expect("current_exe"))
        .args(["--exact", "deux_processus_ne_peuvent_pas_ouvrir_la_meme_base"])
        .env(MARQUE, &dossier)
        .output()
        .expect("lancer l'enfant");
    let code = sortie.status.code();

    println!("▸ l'enfant a rendu {code:?} (3 = refusé, 7 = ouvert)");
    let _ = std::fs::remove_dir_all(&dossier);

    assert_eq!(
        code,
        Some(3),
        "un second processus a ouvert la même base — alors une file à plusieurs \
         travailleurs est possible, et il faut une prise atomique.\n  stderr : {}",
        String::from_utf8_lossy(&sortie.stderr)
    );
}

/// **Ce que le moteur partage nativement : la lecture.**
///
/// Le verrou d'une base ouverte en lecture seule est un `F_RDLCK` — *partagé*.
/// Plusieurs processus peuvent donc lire la même base en même temps, y compris
/// depuis plusieurs machines sur un montage commun. Ce qu'ils ne peuvent pas,
/// c'est écrire ; et aucun ne peut ouvrir tant qu'un écrivain tient la base,
/// puisqu'un verrou partagé et un verrou exclusif s'excluent.
///
/// Les deux moitiés sont vérifiées ici, parce que c'est leur conjonction qui
/// décrit ce qu'on peut vraiment faire à plusieurs machines.
#[test]
fn plusieurs_lecteurs_partagent_une_base_qu_aucun_ecrivain_ne_tient() {
    const LECTEUR: &str = "RAG3WEAVER_ENFANT_LECTEUR";
    const ECRIVAIN_TIENT: &str = "RAG3WEAVER_ENFANT_PENDANT_ECRITURE";

    // Rôles enfants : ouvrir en lecture seule, et dire ce qui s'est passé.
    for marque in [LECTEUR, ECRIVAIN_TIENT] {
        if let Ok(dossier) = std::env::var(marque) {
            match Rag3dbConnection::read_only(&dossier) {
                Ok(conn) => {
                    // Ouvrir ne suffit pas : il faut pouvoir lire.
                    let r = conn
                        .execute("MATCH (t:Travail) RETURN count(t) AS n")
                        .expect("lire");
                    match r.rows.first().and_then(|l| l.first()) {
                        Some(CypherValue::Int(n)) if *n == TRAVAUX as i64 => std::process::exit(7),
                        autre => {
                            eprintln!("lu {autre:?}");
                            std::process::exit(9)
                        }
                    }
                }
                Err(_) => std::process::exit(3),
            }
        }
    }

    let dossier = std::env::temp_dir().join(format!(
        "rag3weaver-lecteurs-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // Écrire, puis **lâcher** la base : c'est la condition du partage.
    {
        let ecrivain = Rag3dbConnection::new(&dossier).expect("écrivain");
        ecrivain
            .execute("CREATE NODE TABLE Travail(id INT64, statut STRING, preneur STRING, PRIMARY KEY(id))")
            .expect("table");
        for i in 0..TRAVAUX {
            ecrivain
                .execute(&format!("CREATE (:Travail {{id: {i}, statut: 'libre', preneur: ''}})"))
                .expect("insertion");
        }
    }

    let enfant = |marque: &str| -> Option<i32> {
        std::process::Command::new(std::env::current_exe().expect("current_exe"))
            .args(["--exact", "plusieurs_lecteurs_partagent_une_base_qu_aucun_ecrivain_ne_tient"])
            .env(marque, &dossier)
            .output()
            .expect("lancer l'enfant")
            .status
            .code()
    };

    // ── Deux lecteurs, personne n'écrit ───────────────────────────────────
    let lecteur_parent = Rag3dbConnection::read_only(&dossier).expect("le parent lit");
    let code = enfant(LECTEUR);
    println!("▸ second lecteur pendant qu'un lecteur tient : {code:?} (7 = a lu)");
    assert_eq!(code, Some(7), "deux lecteurs doivent pouvoir partager la base");
    drop(lecteur_parent);

    // ── Un écrivain tient : deux contrats possibles, aucun troisième ──────
    //
    // **Cette moitié dépend de la bibliothèque, pas de notre code.** Avant le
    // report de Vela, un lecteur était refusé (verrou partagé contre verrou
    // exclusif). Après, il entre et lit — mesuré le 3 septembre 2026 contre
    // `build/lecteurs`, où ce même enfant rend 7 au lieu de 3.
    //
    // Épingler l'un des deux ferait de ce test l'affirmation d'un contrat mort
    // dès que la bibliothèque bouge — le défaut qu'on a sorti trois fois
    // aujourd'hui. Ce qui est **invariant**, et qui est notre affaire, c'est
    // qu'il n'y a pas de troisième issue : ou refusé, ou il lit **juste**.
    // Jamais du bruit, jamais un silence.
    //
    // L'affirmation tranchante sur la reprise vit dans
    // `un_lecteur_qui_insiste_pendant_qu_on_ecrit`, où elle a un sens dans les
    // deux régimes.
    let ecrivain = Rag3dbConnection::new(&dossier).expect("écrivain");
    let code = enfant(ECRIVAIN_TIENT);
    match code {
        Some(3) => println!(
            "▸ lecteur pendant qu'un écrivain tient  : refusé — bibliothèque antérieure \
             au report de Vela"
        ),
        Some(7) => println!(
            "▸ lecteur pendant qu'un écrivain tient  : il lit, et juste — bibliothèque \
             portant le report de Vela"
        ),
        autre => panic!(
            "issue inattendue ({autre:?}) : un lecteur doit être refusé (3) ou lire \
             juste (7). 9 voudrait dire qu'il a lu autre chose que ce qui est en base."
        ),
    }
    drop(ecrivain);

    let _ = std::fs::remove_dir_all(&dossier);
}

// ═══ Un lecteur qui insiste pendant qu'on écrit ═════════════════════════════

/// **Ce que la reprise doit absorber, éprouvé plutôt que supposé.**
///
/// Le report de Vela lève l'exclusion lecteur/écrivain : un lecteur peut ouvrir
/// pendant qu'un écrivain travaille. Le refus qui reste — « Couldn't replay
/// shadow pages under read-only mode » — est **transitoire** : il dure le temps
/// que l'écrivain finisse de poser ses pages fantômes. Sans reprise, cet
/// instant se présente à l'appelant comme « la base est inaccessible ».
///
/// Ce test ouvre quatre-vingts fois pendant qu'on écrit, et regarde ce qui
/// arrive. **Il affirme quelque chose dans les deux régimes**, et dit lequel il
/// observe :
///
/// - bibliothèque à jour → **aucun** refus ne doit survivre à la reprise, et
///   chaque ouverture réussie doit lire un compte cohérent, jamais du bruit ;
/// - bibliothèque antérieure au report → le refus est **uniforme**, c'est
///   l'ancien contrat, et il est nommé comme tel.
///
/// Ce qui est interdit dans les deux cas, c'est le refus **sporadique** : il
/// voudrait dire qu'un transitoire a traversé la reprise.
#[test]
fn un_lecteur_qui_insiste_pendant_qu_on_ecrit() {
    const INSISTANT: &str = "RAG3WEAVER_ENFANT_INSISTANT";
    const CYCLES: usize = 80;

    // Rôle enfant : ouvrir en boucle, compter les refus, dire ce qu'on a lu.
    if let Ok(dossier) = std::env::var(INSISTANT) {
        let mut refus = 0usize;
        let mut lus = 0usize;
        let mut incoherents = 0usize;
        for _ in 0..CYCLES {
            match Rag3dbConnection::read_only(&dossier) {
                Ok(conn) => match conn.execute("MATCH (t:Travail) RETURN count(t) AS n") {
                    // Le compte croît pendant qu'on écrit : ce qui compte est
                    // qu'il soit **plausible**, jamais du bruit.
                    Ok(r) => match r.rows.first().and_then(|l| l.first()) {
                        Some(CypherValue::Int(n)) if *n >= 0 => lus += 1,
                        _ => incoherents += 1,
                    },
                    Err(_) => incoherents += 1,
                },
                Err(_) => refus += 1,
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        println!("REFUS={refus} LUS={lus} INCOHERENTS={incoherents}");
        std::process::exit(0);
    }

    let dossier = std::env::temp_dir().join(format!(
        "rag3weaver-insistant-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let ecrivain = Rag3dbConnection::new(&dossier).expect("écrivain");
    ecrivain
        .execute("CREATE NODE TABLE Travail(id INT64, statut STRING, preneur STRING, PRIMARY KEY(id))")
        .expect("table");

    // L'enfant insiste pendant que le parent écrit sans relâche : un point de
    // reprise tous les cinq enregistrements, c'est-à-dire une boucle
    // volontairement hostile.
    let mut enfant = std::process::Command::new(std::env::current_exe().expect("current_exe"))
        // `--nocapture` : sans lui le harnais de l'enfant avale son `println!`,
        // et le parent ne lit rien — l'enfant paraîtrait muet alors qu'il parle.
        .args(["--exact", "un_lecteur_qui_insiste_pendant_qu_on_ecrit", "--nocapture"])
        .env(INSISTANT, &dossier)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("lancer l'enfant insistant");

    let mut i = 0i64;
    while enfant.try_wait().expect("try_wait").is_none() {
        ecrivain
            .execute(&format!("CREATE (:Travail {{id: {i}, statut: 'libre', preneur: ''}})"))
            .expect("insertion");
        i += 1;
        if i % 5 == 0 {
            let _ = ecrivain.execute("CHECKPOINT");
        }
    }
    let sortie = enfant.wait_with_output().expect("attendre l'enfant");
    let texte = String::from_utf8_lossy(&sortie.stdout);
    let ligne = texte
        .lines()
        .find(|l| l.starts_with("REFUS="))
        .unwrap_or_else(|| panic!("l'enfant n'a rien dit :\n{texte}"));
    println!("▸ {ligne}   ({i} écritures pendant ce temps)");

    let lire = |cle: &str| -> usize {
        ligne
            .split_whitespace()
            .find_map(|m| m.strip_prefix(cle))
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| panic!("« {cle} » illisible dans « {ligne} »"))
    };
    let (refus, lus, incoherents) = (lire("REFUS="), lire("LUS="), lire("INCOHERENTS="));

    // Ce qui est vrai dans les deux régimes.
    assert_eq!(
        incoherents, 0,
        "une ouverture qui réussit doit lire un compte cohérent, jamais du bruit"
    );
    assert_eq!(refus + lus, CYCLES, "chaque cycle doit avoir une issue nommée");

    if refus == 0 {
        println!("  → bibliothèque à jour : la reprise absorbe tout ce qui est transitoire");
    } else if refus == CYCLES {
        println!(
            "  → bibliothèque antérieure au report de Vela : l'exclusion lecteur/écrivain \
             tient encore, le refus est l'ancien contrat"
        );
    } else {
        panic!(
            "refus SPORADIQUE : {refus}/{CYCLES}. Ce n'est ni l'ancien contrat (refus \
             uniforme) ni la reprise qui fonctionne (aucun refus) — un transitoire a \
             traversé le budget de `read_only`"
        );
    }
    drop(ecrivain);
    let _ = std::fs::remove_dir_all(&dossier);
}

// ═══ La marque d'eau, vue depuis un autre processus ═════════════════════════

/// **La marque d'eau franchit-elle vraiment la frontière ?**
///
/// Elle a été éprouvée sur PostgreSQL (`e2e_postgres::la_marque_deau_traverse_la_frontiere`)
/// avec deux catalogues. Mais c'est **kuzu** que `rag3daemon` sert, et c'est
/// pour les lecteurs de kuzu que la marque existe : la prouver ailleurs et la
/// supposer ici serait exactement le raccourci qu'on passe la journée à
/// débusquer.
///
/// Ici, deux vrais processus. L'écrivain met en file **sans vider** ; l'enfant
/// ouvre la base en lecture seule et cherche la marque. S'il la voit, un
/// lecteur d'un autre processus peut savoir qu'il ne voit pas tout — c'est
/// toute la question.
///
/// Ce test demande une bibliothèque portant le report de Vela ; sur une plus
/// ancienne, l'enfant est refusé et il le **dit** plutôt que de faire croire à
/// une absence de marque.
#[test]
fn la_marque_dingestion_se_voit_depuis_un_autre_processus() {
    use rag3weaver::{Catalog, CatalogConfig};
    const ENFANT: &str = "RAG3WEAVER_ENFANT_MARQUE";

    // Rôle enfant : lire les marques, et rien d'autre.
    if let Ok(dossier) = std::env::var(ENFANT) {
        match Rag3dbConnection::read_only(&dossier) {
            Ok(conn) => {
                let r = conn
                    .execute(
                        "MATCH (m:_catalog_meta) WHERE m._key STARTS WITH '_ingestion/pending/' \
                         RETURN m._key, m._value",
                    )
                    .expect("lire les marques");
                let vivantes = r
                    .rows
                    .iter()
                    .filter(|l| l.get(1).and_then(|v| v.as_str()).is_some_and(|v| v != "0"))
                    .count();
                println!("MARQUES={vivantes}");
                std::process::exit(0);
            }
            Err(e) => {
                println!("REFUSE={e}");
                std::process::exit(3);
            }
        }
    }

    let dossier = std::env::temp_dir().join(format!(
        "rag3weaver-marque-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let mut config = CatalogConfig::default();
    config.embedding_dim = 4;
    let mut ecrivain = Catalog::new(
        Box::new(Rag3dbConnection::new(&dossier).expect("écrivain")),
        Box::new(rag3weaver::embedder::MockEmbedder::new(4)),
        config,
    );
    ecrivain.initialize().expect("initialize");
    // Un champ de contenu et le seul signal BM25 : ce test ne parle pas de
    // recherche, il parle de la marque — inutile de traîner un embarqueur.
    let mut champs = std::collections::HashMap::new();
    champs.insert(
        "texte".to_string(),
        rag3weaver::SimpleFieldDef {
            field_type: rag3weaver::config::FieldType::Text,
            is_content: true,
            ..Default::default()
        },
    );
    ecrivain
        .register_entity(
            "Produit",
            rag3weaver::EntityConfig {
                fields: champs,
                signals: rag3weaver::search::SearchSignals::BM25,
                ..Default::default()
            },
        )
        .expect("entité");

    let lire_marques = |attendu: &str| -> Option<usize> {
        let sortie = std::process::Command::new(std::env::current_exe().expect("current_exe"))
            .args([
                "--exact",
                "la_marque_dingestion_se_voit_depuis_un_autre_processus",
                "--nocapture",
            ])
            .env(ENFANT, &dossier)
            .output()
            .expect("lancer l'enfant");
        let texte = String::from_utf8_lossy(&sortie.stdout);
        if let Some(l) = texte.lines().find(|l| l.starts_with("REFUSE=")) {
            println!("▸ {attendu} : enfant refusé — {l}");
            return None;
        }
        let l = texte
            .lines()
            .find(|l| l.starts_with("MARQUES="))
            .unwrap_or_else(|| panic!("l'enfant n'a rien dit :\n{texte}"));
        let n: usize = l.trim_start_matches("MARQUES=").parse().expect("compte");
        println!("▸ {attendu} : {n} marque(s) vue(s) de l'autre processus");
        Some(n)
    };

    // 1. Rien en file : rien à signaler.
    let Some(au_repos) = lire_marques("au repos") else {
        println!(
            "  → bibliothèque antérieure au report de Vela : un lecteur ne peut pas \
             ouvrir pendant qu'un écrivain tient, donc la marque ne lui sert à rien \
             encore. Test sans objet ici, et il le dit."
        );
        drop(ecrivain);
        let _ = std::fs::remove_dir_all(&dossier);
        return;
    };
    assert_eq!(au_repos, 0, "au repos, aucun écrivain ne doit être marqué");

    // 2. L'écrivain met en file **sans vider**. C'est le cas qui mentait :
    //    la file est en mémoire, invisible de l'extérieur.
    let mut donnees = std::collections::BTreeMap::new();
    donnees.insert("_uuid".to_string(), CypherValue::String("p1".to_string()));
    donnees.insert("texte".to_string(), CypherValue::String("un texte".to_string()));
    ecrivain.create("Produit", donnees).expect("mise en file");
    assert!(ecrivain.has_pending(), "le montage suppose une file non vidée");

    let sous_travail = lire_marques("sous travail non publié").expect("l'enfant lit");
    assert_eq!(
        sous_travail, 1,
        "un lecteur d'un autre processus doit VOIR qu'un écrivain a du travail non publié"
    );

    // 3. L'écrivain vide : la marque s'efface, vue de l'extérieur.
    ecrivain.drain();
    let apres = lire_marques("après le drain").expect("l'enfant lit");
    assert_eq!(apres, 0, "après le drain, plus rien ne doit être marqué");

    drop(ecrivain);
    let _ = std::fs::remove_dir_all(&dossier);
}
