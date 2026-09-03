//! E2E : **deux processus sur la même base**, ce qui était impossible il y a
//! une heure.
//!
//! `tests/e2e_prise_atomique.rs` a mesuré le mur : un second processus ne peut
//! pas ouvrir une base rag3db, le verrou `F_WRLCK` est posé en `F_SETLK` et
//! refuse tout de suite. Ce test-ci refait exactement la même scène, mais en
//! passant par rag3daemon — et il vérifie **les deux faits à la fois** : que
//! l'ouverture directe échoue toujours, et que par le démon les deux processus
//! travaillent sur les mêmes données sans se marcher dessus.
//!
//! ```bash
//! cargo test --features daemon,rag3db-native --test e2e_rag3daemon -- --nocapture
//! ```

#![cfg(all(feature = "daemon", feature = "rag3db-native"))]

use std::collections::HashSet;

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::daemon::DaemonConnection;
use rag3weaver::serveur::Fin;
use rag3weaver::Rag3dbConnection;

const TRAVAUX: i64 = 40;
const ADRESSE_ENFANT: &str = "RAG3WEAVER_ENFANT_ADRESSE";

fn port_libre() -> String {
    let e = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let a = e.local_addr().unwrap().to_string();
    drop(e);
    a
}

/// La requête de prise : le premier travail libre, marqué au nom du preneur.
fn prise(qui: &str) -> String {
    format!(
        "MATCH (t:Travail) WHERE t.statut = 'libre' \
         WITH t LIMIT 1 \
         SET t.statut = 'pris', t.preneur = '{qui}' \
         RETURN t.id AS id"
    )
}

/// Prendre jusqu'à ce qu'il n'y ait plus rien.
fn prendre_tout(conn: &dyn DbConnection, qui: &str) -> Vec<i64> {
    let mut pris = Vec::new();
    loop {
        match conn.execute(&prise(qui)) {
            Ok(r) if r.rows.is_empty() => break,
            Ok(r) => {
                if let Some(CypherValue::Int(id)) = r.rows[0].first() {
                    pris.push(*id);
                }
            }
            Err(e) => panic!("{qui} : {e}"),
        }
    }
    pris
}

// `#[ignore]` comme partout ici : `run_e2e.sh` ne lance que les tests ignorés.
// Sans lui, celui-ci était « filtered out » à chaque passe complète — jamais
// joué, alors qu'il porte la raison d'être du démon.
#[test]
#[ignore]
fn deux_processus_partagent_la_base_par_le_demon() {
    // ── Rôle enfant : se brancher sur le démon et prendre sa part ──────────
    if let Ok(adresse) = std::env::var(ADRESSE_ENFANT) {
        let conn = DaemonConnection::joindre(&adresse).expect("l'enfant joint le démon");
        for id in prendre_tout(&conn, "enfant") {
            println!("{id}");
        }
        std::process::exit(0);
    }

    let adresse = port_libre();
    let dossier = std::env::temp_dir().join(format!(
        "rag3weaver-rag3daemon-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let journal = std::env::temp_dir().join("rag3weaver-e2e-rag3daemon");

    let serveur = DaemonConnection::serveur(
        &adresse,
        env!("CARGO_BIN_EXE_rag3daemon"),
        dossier.to_string_lossy(),
    )
    .journal_dans(&journal)
    // Hermétique : ce démon-ci ne doit pas survivre au test.
    .fin(Fin::Arreter);

    let parent = DaemonConnection::assurer(&serveur).unwrap_or_else(|e| {
        panic!(
            "assurer : {e}\n  journal : {}",
            journal.join("rag3daemon.log").display()
        )
    });
    assert_eq!(parent.identite().base, dossier.to_string_lossy());

    // ── Le mur est toujours là ────────────────────────────────────────────
    // Le démon tient le verrou : personne d'autre ne peut ouvrir la base en
    // direct. C'est ce qui rend le démon nécessaire, pas une commodité.
    let direct = Rag3dbConnection::new(&dossier);
    assert!(
        direct.is_err(),
        "la base s'est ouverte en direct alors que le démon la tient — \
         le verrou ne joue plus, et tout ce test ne prouve plus rien"
    );
    println!("▸ ouverture directe pendant que le démon tient la base : refusée ✓");

    // ── Les travaux ───────────────────────────────────────────────────────
    parent
        .execute("CREATE NODE TABLE Travail(id INT64, statut STRING, preneur STRING, PRIMARY KEY(id))")
        .expect("table");
    for i in 0..TRAVAUX {
        parent
            .execute(&format!(
                "CREATE (:Travail {{id: {i}, statut: 'libre', preneur: ''}})"
            ))
            .expect("insertion");
    }

    // ── Les deux processus prennent en même temps ─────────────────────────
    let enfant = std::process::Command::new(std::env::current_exe().expect("current_exe"))
        // `--ignored` : l'enfant relance ce test, qui porte désormais l'attribut.
        // Sans ça il ne joue rien, et le parent lit son silence comme « il n'a
        // rien pris » — un échec qui accuserait le partage au lieu du montage.
        .args(["--exact", "deux_processus_partagent_la_base_par_le_demon", "--nocapture", "--ignored"])
        .env(ADRESSE_ENFANT, &adresse)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("lancer l'enfant");

    let pris_parent = prendre_tout(&parent, "parent");
    let sortie = enfant.wait_with_output().expect("attendre l'enfant");
    assert!(sortie.status.success(), "l'enfant a échoué : {}", String::from_utf8_lossy(&sortie.stderr));

    let pris_enfant: Vec<i64> = String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .filter_map(|l| l.trim().parse::<i64>().ok())
        .collect();

    println!(
        "▸ parent : {} travaux   enfant : {} travaux",
        pris_parent.len(),
        pris_enfant.len()
    );

    // ── Ce que ça prouve ─────────────────────────────────────────────────
    let a: HashSet<i64> = pris_parent.iter().copied().collect();
    let b: HashSet<i64> = pris_enfant.iter().copied().collect();
    let doubles: Vec<_> = a.intersection(&b).collect();

    assert!(doubles.is_empty(), "travaux pris deux fois : {doubles:?}");
    assert_eq!(a.len(), pris_parent.len(), "le parent a pris deux fois le même");
    assert_eq!(b.len(), pris_enfant.len(), "l'enfant a pris deux fois le même");
    assert_eq!(
        a.len() + b.len(),
        TRAVAUX as usize,
        "il manque des travaux : {} + {} ≠ {TRAVAUX}",
        a.len(),
        b.len()
    );
    assert!(
        !pris_enfant.is_empty(),
        "l'enfant n'a rien pris — le parent a peut-être tout avalé avant qu'il démarre, \
         et alors on n'a rien prouvé sur le partage"
    );

    // Et la base est cohérente vue du démon.
    let reste = parent
        .execute("MATCH (t:Travail) WHERE t.statut = 'libre' RETURN count(t) AS n")
        .expect("compte");
    assert_eq!(reste.rows[0][0], CypherValue::Int(0));

    drop(parent); // arrête le démon (Fin::Arreter)
    let _ = std::fs::remove_dir_all(&dossier);
}

// ═══ Le choix du lecteur, et ce qu'il en dit ════════════════════════════════

/// **Un lecteur choisit son chemin, et sait lequel il a pris.**
///
/// Depuis le report de Vela, un lecteur peut ouvrir lui-même en lecture seule
/// pendant que le démon tient la base en écriture. Le relais devient donc
/// facultatif **pour lire** — facultatif, pas caduc : la bibliothèque liée peut
/// être antérieure au report.
///
/// Ce test tient la règle qui empêche cette souplesse de devenir un piège :
/// **jamais de repli silencieux**, ni vers le relais, ni vers le direct. Un
/// lecteur qui croit lire en direct alors qu'il passe par un relais ne peut
/// diagnostiquer ni sa latence ni sa fraîcheur.
#[test]
#[ignore]
fn un_lecteur_choisit_son_chemin_et_le_dit() {
    use rag3weaver::acces::{ouvrir_lecteur, Acces, Chemin};

    let adresse = port_libre();
    let dossier = std::env::temp_dir().join(format!(
        "rag3weaver-acces-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let journal = std::env::temp_dir().join("rag3weaver-e2e-acces");

    let serveur = DaemonConnection::serveur(
        &adresse,
        env!("CARGO_BIN_EXE_rag3daemon"),
        dossier.to_string_lossy(),
    )
    .journal_dans(&journal)
    .fin(Fin::Arreter);

    // Le démon tient la base et y écrit : c'est la scène qui compte.
    let ecrivain = DaemonConnection::assurer(&serveur)
        .unwrap_or_else(|e| panic!("assurer : {e}\n  journal : {}", journal.join("rag3daemon.log").display()));
    ecrivain
        .execute("CREATE NODE TABLE Note(id INT64, texte STRING, PRIMARY KEY(id))")
        .expect("table");
    for i in 0..5 {
        ecrivain
            .execute(&format!("CREATE (:Note {{id: {i}, texte: 'note {i}'}})"))
            .expect("insertion");
    }

    let compte = |l: &rag3weaver::acces::Lecteur| -> i64 {
        l.conn
            .execute("MATCH (n:Note) RETURN count(n) AS n")
            .expect("lire")
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_i64())
            .unwrap_or(-1)
    };

    // ── Le relais, explicitement demandé ──────────────────────────────────
    let par_relais = ouvrir_lecteur(&dossier, Acces::Demon, Some(&serveur)).expect("le relais");
    assert_eq!(par_relais.par, Chemin::Demon);
    assert!(par_relais.avertissements.is_empty(), "rien à signaler quand on demande le relais");
    assert_eq!(compte(&par_relais), 5, "le relais doit lire ce qui est en base");

    // ── Le direct, ou une erreur nommée ───────────────────────────────────
    match ouvrir_lecteur(&dossier, Acces::Direct, Some(&serveur)) {
        Ok(direct) => {
            assert_eq!(direct.par, Chemin::Direct);
            assert!(
                direct.avertissements.is_empty(),
                "un direct qui réussit n'a rien à signaler : {:?}",
                direct.avertissements
            );
            assert_eq!(compte(&direct), 5, "le direct doit lire la même chose que le relais");
            println!("▸ direct : ouvert pendant que le démon tient la base, 5 notes lues");

            // Et `Auto` doit alors prendre le direct, sans un mot.
            let auto = ouvrir_lecteur(&dossier, Acces::Auto, Some(&serveur)).expect("auto");
            assert_eq!(auto.par, Chemin::Direct, "Auto doit préférer le direct quand il marche");
            assert!(auto.avertissements.is_empty(), "{:?}", auto.avertissements);
        }
        Err(e) => {
            // Bibliothèque antérieure au report : le direct est refusé, et
            // l'erreur le **nomme** au lieu de rendre le message brut du cœur.
            assert!(e.contains("accès direct demandé et refusé"), "{e}");
            println!("▸ direct : refusé — bibliothèque antérieure au report de Vela");

            // Et `Auto` se rabat **en le disant** : c'est toute la règle.
            let auto = ouvrir_lecteur(&dossier, Acces::Auto, Some(&serveur)).expect("auto");
            assert_eq!(auto.par, Chemin::Demon, "Auto doit se rabattre");
            assert!(
                auto.avertissements.iter().any(|w| w.contains("repli sur le relais")),
                "le repli doit se dire, jamais se taire : {:?}",
                auto.avertissements
            );
            assert_eq!(compte(&auto), 5);
            println!("▸ auto   : replié sur le relais, et il le dit");
        }
    }

    let _ = std::fs::remove_dir_all(&dossier);
}
