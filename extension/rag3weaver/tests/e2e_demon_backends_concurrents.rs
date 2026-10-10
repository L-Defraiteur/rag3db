//! **N backends sur une seule base, par le démon : leurs `initialize` se
//! marchent-ils dessus ?**
//!
//! C'est la question que j'ai nommée en concevant le mode `--demon` du serveur
//! MCP (`docs/10-octobre-2026-18h00/02`), et qui décide de sa forme.
//!
//! **Le raisonnement, en entier, pour qu'on puisse le contredire.** Claude Code
//! lance un processus MCP **par session**, et une base rag3db n'a qu'un seul
//! écrivain (`F_WRLCK`). Deux sessions sur la même mémoire seraient donc deux
//! écrivains — refusés par le moteur. La réponse retenue est que le processus
//! MCP **ne possède pas la base** : son backend reste local (ses outils et ses
//! schémas ne demandent aucune base) et sa **connexion** passe par
//! `rag3daemon`, le processus unique qui la tient. N serveurs, un écrivain,
//! aucun protocole à relayer, puisque `DaemonConnection` est une
//! `DbConnection` comme une autre.
//!
//! **Le trou de ce raisonnement, et c'est tout l'objet de ce test** : chacun de
//! ces N backends garde **son** `Catalog`, donc son `initialize` — qui repose
//! les DDL du schéma. N ouvertures concurrentes vont donc les reposer
//! ensemble. `IF NOT EXISTS` est partout, mais « partout » est une impression,
//! pas une mesure : ce test la remplace.
//!
//! **Si ce test est rouge, le mode `--demon` tel qu'il est écrit ne tient
//! pas**, et le pont (relayer `describe`/`call` vers un hôte unique) redevient
//! la bonne réponse — pour une raison mesurée au lieu d'une précaution. C'est
//! la condition que j'ai posée moi-même avant d'écrire une ligne de ce mode.
//!
//! **Pas de vecteurs, pas d'embarqueur, exprès.** Le manifeste est écrit ici,
//! minimal, une entité en BM25 seul : la question porte sur la concurrence des
//! DDL, pas sur la provenance des nombres. Un test qui exigerait un service
//! d'embarquement pour répondre à ça mélangerait deux sujets et ne se
//! rejouerait pas partout.
//!
//! # ÉTAT AU 11 OCTOBRE 2026, 00 h 45 : NON CONCLUANT, ET LES DÉFAUTS SONT LES MIENS
//!
//! Ce témoin n'a **pas** répondu à sa question. Trois passes, trois défauts de
//! ma main avant d'atteindre le sujet — un `schema` d'entité omis, une
//! déclaration d'embarquement omise (`load` l'exige quel que soit le signal),
//! puis une garde qui lisait la **première colonne** de `SHOW_TABLES` comme un
//! nom alors que c'est un identifiant : elle faisait **rougir un cas où les
//! quatre ouvertures avaient réussi**.
//!
//! **Ce qui reste à expliquer, et que je n'ai pas élucidé :**
//!
//! 1. la passe où les quatre ouvertures ont rendu `Ok` affichait `tables : []`.
//!    Soit les tables existaient et ma lecture était fausse (le plus probable),
//!    soit `initialize` n'a rien créé — et ce serait un défaut du produit. **Les
//!    deux hypothèses sont ouvertes** ; ce qui les sépare est un
//!    `MATCH (n:Note) RETURN count(n)` qui rende un compte ou un refus nommé ;
//! 2. ce `MATCH`, justement, **pend** (vingt minutes, arrêté à la main) sur une
//!    cinquième connexion au démon, après que les quatre backends ont été lâchés.
//!    Un lecteur qui bloque alors que les écrivains sont partis est soit un
//!    `Drop` qui ne rend pas la main, soit un verrou du démon — à instrumenter ;
//! 3. **le journal du démon annonce une autre adresse** que celle que le témoin
//!    affiche (41969 contre 37273) alors que `Attache::adresse` rend l'adresse
//!    *demandée*. Tant que cet écart n'est pas compris, je ne sais pas à quel
//!    démon les quatre ouvertures ont parlé — donc le « 4 réussies » n'est pas
//!    une preuve.
//!
//! **Pourquoi il est poussé ainsi** : parce qu'un témoin non concluant dont les
//! défauts sont nommés vaut mieux qu'un témoin supprimé, et parce que la
//! conclusion honnête est que **le mode `--demon` n'est ni validé ni réfuté**.
//! La condition que je m'étais fixée — rouge ⇒ le pont redevient la bonne
//! réponse — ne s'applique pas : ce rouge ne parle pas du produit.
//!
//! **La leçon de méthode**, et c'est la plus réutilisable : j'ai composé un
//! manifeste minimal **de zéro**, en devinant ce qui était obligatoire. Sept
//! fois aujourd'hui j'ai supposé une interface au lieu de la lire. La procédure
//! qui l'évite n'est pas « lire avant d'écrire » mais **copier un gabarit livré
//! et en retirer** : un gabarit est la liste exhaustive de ce qui est exigé, et
//! le filet `src/gabarits.rs` la vérifie à chaque passe de la lib.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_demon_backends_concurrents
//! ```

#![cfg(all(feature = "daemon", feature = "rag3db-native"))]

use std::path::{Path, PathBuf};

use rag3weaver::backend::PreparedBackend;
use rag3weaver::connection::DbConnection;
use rag3weaver::daemon::DaemonConnection;
use rag3weaver::serveur::Fin;
use serde_json::json;

/// Combien de sessions simultanées on simule. Trois suffisent à faire se
/// croiser des DDL ; au-delà on mesurerait la machine, pas le défaut.
const SESSIONS: usize = 4;

fn port_libre() -> String {
    let e = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let a = e.local_addr().unwrap().to_string();
    drop(e);
    a
}

fn racine_moteur() -> PathBuf {
    std::env::var("RAG3DB_ROOT").map(PathBuf::from).unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("racine du dépôt")
    })
}

/// Un manifeste minimal : une entité, du plein texte, aucun outil.
///
/// `tools` est obligatoire au manifeste et vaut `{}` ici : ce test n'appelle
/// aucun verbe, il **ouvre**. C'est l'ouverture qui repose les DDL.
fn manifeste(ou: &Path, base: &Path) -> PathBuf {
    let ext = racine_moteur().join("extension/vector/build/libvector.rag3db_extension");
    assert!(
        ext.exists(),
        "l'extension vectorielle est absente : {}\nPosez RAG3DB_ROOT sur l'arbre où le \
         moteur est bâti (depuis un worktree, c'est l'arbre principal).",
        ext.display()
    );
    // **`schema` est obligatoire sur une entité de manifeste** — un chemin vers
    // un JSON Schema, pas un objet en ligne. Lu dans `BackendEntity` après
    // m'être fait refuser « missing field `schema` » : c'est la cinquième
    // interface que j'aurai supposée au lieu de la lire aujourd'hui, et la
    // cinquième passe perdue pour dix secondes de lecture.
    let schema = ou.join("note.json");
    std::fs::write(
        &schema,
        serde_json::to_vec_pretty(&json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": ["key", "text", "revision", "created_at", "updated_at"],
            "properties": {
                "key": {"type": "string", "minLength": 1},
                "text": {"type": "string"},
                "revision": {"type": "integer", "minimum": 1},
                "created_at": {"type": "integer"},
                "updated_at": {"type": "integer"}
            }
        }))
        .unwrap(),
    )
    .expect("écrire le schéma");

    let m = json!({
        "version": 1,
        "name": "concurrence",
        "database": base.to_string_lossy(),
        "vector_extension": ext.to_string_lossy(),
        // **`load` exige une déclaration d'embarquement, quel que soit le
        // signal** (`BackendManifest::embed_source` : « ce backend ne déclare
        // pas d'embarquement »). Lu dans la structure après m'être fait
        // refuser — septième interface supposée de la journée. Elle est
        // déclarée mais **jamais jointe** : l'entité n'a que du BM25, donc
        // `needs_embeddings()` est faux et `open` reçoit `None`.
        "embeddings": { "address": "", "model": "bge-m3", "dimensions": 1024 },
        "entities": {
            "Note": {
                "schema": schema.to_string_lossy(),
                "config": {
                    "fields": { "key": {"type":"string"}, "text": {"type":"string","isContent":true} },
                    "hashsafe": ["key"],
                    "returnFields": ["key","text"],
                    "signals": ["bm25"]
                }
            }
        },
        "tools": {}
    });
    let chemin = ou.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).expect("écrire le manifeste");
    chemin
}

#[test]
#[ignore = "e2e : lance un démon et ouvre une base"]
fn quatre_backends_par_le_demon_ne_se_marchent_pas_dessus() {
    let tmp = std::env::temp_dir().join(format!("rag3weaver-demon-conc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("dossier temporaire");
    let base = tmp.join("base.rag3db");
    let chemin = manifeste(&tmp, &base);

    // Le démon : **le seul** à posséder la base.
    let adresse = port_libre();
    let journal = tmp.join("journal");
    let serveur = DaemonConnection::serveur(
        &adresse,
        env!("CARGO_BIN_EXE_rag3daemon"),
        base.to_string_lossy(),
    )
    .journal_dans(&journal)
    // Hermétique : ce démon-ci ne doit pas survivre au test.
    .fin(Fin::Arreter);
    let _amorce = DaemonConnection::assurer(&serveur).unwrap_or_else(|e| {
        panic!("assurer le démon : {e}\n  journal : {}", journal.join("rag3daemon.log").display())
    });
    println!("▸ démon sur {adresse}, base {}", base.display());

    // **Les N ouvertures, vraiment en même temps.** Une barrière, sinon la
    // première a fini de reposer ses DDL avant que la deuxième commence, et le
    // test rendrait un vert qui ne prouve rien — la même faute que tuer un
    // processus dont le journal est déjà replié.
    let barriere = std::sync::Arc::new(std::sync::Barrier::new(SESSIONS));
    let mut fils = Vec::new();
    for i in 0..SESSIONS {
        let chemin = chemin.clone();
        let adresse = adresse.clone();
        let barriere = barriere.clone();
        fils.push(std::thread::spawn(move || -> Result<usize, String> {
            // Le backend se prépare **sans base** : c'est la propriété sur
            // laquelle tout le mode `--demon` repose.
            let prepared = PreparedBackend::load(&chemin).map_err(|e| format!("load : {e}"))?;
            let conn = DaemonConnection::joindre(&adresse).map_err(|e| format!("joindre : {e}"))?;
            barriere.wait();
            let backend = prepared
                .open(Box::new(conn), None)
                .map_err(|e| format!("open (session {i}) : {e}"))?;
            // Une lecture, pour prouver que le catalogue monté est utilisable
            // et pas seulement construit.
            let r = backend
                .prepared
                .describe()
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or("describe sans nom")?;
            assert_eq!(r, "concurrence");
            Ok(i)
        }));
    }

    let mut echecs: Vec<String> = Vec::new();
    let mut reussites = 0usize;
    for (i, f) in fils.into_iter().enumerate() {
        match f.join() {
            Ok(Ok(_)) => reussites += 1,
            Ok(Err(e)) => echecs.push(e),
            Err(_) => echecs.push(format!("session {i} : panique")),
        }
    }
    println!("▸ ouvertures : {reussites} réussies, {} en échec", echecs.len());
    for e in &echecs {
        println!("▸   échec : {e}");
    }

    // **Tous les échecs sont rendus, pas le premier.** La leçon du filet des
    // gabarits et des bancs à corpus vivant : un test qui s'arrête au premier
    // cache les autres, et on les découvre un par un.
    assert!(
        echecs.is_empty(),
        "{} des {SESSIONS} ouvertures simultanées par le démon ont échoué — le mode `--demon` \
         du serveur MCP ne tient pas tel qu'il est écrit, et c'est le pont (relayer \
         describe/call vers un hôte unique) qui redevient la bonne réponse :\n  {}",
        echecs.len(),
        echecs.join("\n  ")
    );

    // **Et la table existe, et se lit.** Reposer un DDL `IF NOT EXISTS` à
    // quatre doit laisser une table interrogeable, pas une table à moitié
    // écrite.
    //
    // **La première version de cette garde lisait `SHOW_TABLES` et se
    // trompait** : elle prenait la PREMIÈRE colonne pour le nom, alors que
    // c'est un identifiant — donc elle ne voyait aucune table et faisait
    // rougir un cas où les quatre ouvertures avaient réussi. Un contrôle qui
    // masque un vert est plus coûteux qu'un contrôle absent, et celui-ci l'a
    // fait parce qu'il supposait la forme d'une réponse au lieu de la lire.
    //
    // La version qui tient ne dépend d'aucun ordre de colonnes : un `MATCH`
    // sur l'étiquette. Si la table n'existe pas, le moteur refuse en la
    // nommant ; si elle existe, il rend un compte. Et un nom de table est
    // unique dans une base par construction — le risque n'était jamais
    // « quatre tables », c'était « une table inutilisable ».
    let lecteur = DaemonConnection::joindre(&adresse).expect("joindre pour relire");
    let compte = lecteur
        .execute("MATCH (n:Note) RETURN count(n)")
        .unwrap_or_else(|e| {
            panic!(
                "l'entité Note n'est pas interrogeable après {SESSIONS} ouvertures \
                 simultanées ({e}) : les DDL reposés en concurrence ont laissé la base \
                 dans un état inutilisable"
            )
        });
    let lignes = compte.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64());
    println!("▸ Note interrogeable, {} ligne(s)", lignes.unwrap_or(-1));
    assert_eq!(lignes, Some(0), "la table est neuve et vide : {compte:?}");

    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("dossier temporaire");
    let base = tmp.join("base.rag3db");
    let chemin = manifeste(&tmp, &base);

    // Le démon : **le seul** à posséder la base.
    let adresse = port_libre();
    let journal = tmp.join("journal");
    let serveur = DaemonConnection::serveur(
        &adresse,
        env!("CARGO_BIN_EXE_rag3daemon"),
        base.to_string_lossy(),
    )
    .journal_dans(&journal)
    // Hermétique : ce démon-ci ne doit pas survivre au test.
    .fin(Fin::Arreter);
    let _amorce = DaemonConnection::assurer(&serveur).unwrap_or_else(|e| {
        panic!("assurer le démon : {e}\n  journal : {}", journal.join("rag3daemon.log").display())
    });
    println!("▸ démon sur {adresse}, base {}", base.display());

    // **Les N ouvertures, vraiment en même temps.** Une barrière, sinon la
    // première a fini de reposer ses DDL avant que la deuxième commence, et le
    // test rendrait un vert qui ne prouve rien — la même faute que tuer un
    // processus dont le journal est déjà replié.
    let barriere = std::sync::Arc::new(std::sync::Barrier::new(SESSIONS));
    let mut fils = Vec::new();
    for i in 0..SESSIONS {
        let chemin = chemin.clone();
        let adresse = adresse.clone();
        let barriere = barriere.clone();
        fils.push(std::thread::spawn(move || -> Result<usize, String> {
            // Le backend se prépare **sans base** : c'est la propriété sur
            // laquelle tout le mode `--demon` repose.
            let prepared = PreparedBackend::load(&chemin).map_err(|e| format!("load : {e}"))?;
            let conn = DaemonConnection::joindre(&adresse).map_err(|e| format!("joindre : {e}"))?;
            barriere.wait();
            let backend = prepared
                .open(Box::new(conn), None)
                .map_err(|e| format!("open (session {i}) : {e}"))?;
            // Une lecture, pour prouver que le catalogue monté est utilisable
            // et pas seulement construit.
            let r = backend
                .prepared
                .describe()
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or("describe sans nom")?;
            assert_eq!(r, "concurrence");
            Ok(i)
        }));
    }

    let mut echecs: Vec<String> = Vec::new();
    let mut reussites = 0usize;
    for (i, f) in fils.into_iter().enumerate() {
        match f.join() {
            Ok(Ok(_)) => reussites += 1,
            Ok(Err(e)) => echecs.push(e),
            Err(_) => echecs.push(format!("session {i} : panique")),
        }
    }
    println!("▸ ouvertures : {reussites} réussies, {} en échec", echecs.len());
    for e in &echecs {
        println!("▸   échec : {e}");
    }

    // **Tous les échecs sont rendus, pas le premier.** La leçon du filet des
    // gabarits et des bancs à corpus vivant : un test qui s'arrête au premier
    // cache les autres, et on les découvre un par un.
    assert!(
        echecs.is_empty(),
        "{} des {SESSIONS} ouvertures simultanées par le démon ont échoué — le mode `--demon` \
         du serveur MCP ne tient pas tel qu'il est écrit, et c'est le pont (relayer \
         describe/call vers un hôte unique) qui redevient la bonne réponse :\n  {}",
        echecs.len(),
        echecs.join("\n  ")
    );

    // **Et une seule fois chaque table.** Reposer un DDL `IF NOT EXISTS` à
    // quatre doit laisser une table, pas quatre entrées ni une table à moitié
    // écrite. Ce que `SHOW_TABLES` rend est la seule preuve qui ne dépende pas
    // du chemin par lequel on vient de passer.
    let lecteur = DaemonConnection::joindre(&adresse).expect("joindre pour relire");
    let tables = lecteur
        .execute("CALL SHOW_TABLES() RETURN *")
        .expect("SHOW_TABLES");
    let noms: Vec<String> = tables
        .rows
        .iter()
        .filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    println!("▸ tables : {noms:?}");
    let notes = noms.iter().filter(|n| n.as_str() == "Note").count();
    assert_eq!(
        notes, 1,
        "l'entité Note doit exister une fois et une seule après {SESSIONS} ouvertures \
         simultanées ; tables vues : {noms:?}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
