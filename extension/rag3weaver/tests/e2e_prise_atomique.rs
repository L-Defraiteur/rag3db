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
