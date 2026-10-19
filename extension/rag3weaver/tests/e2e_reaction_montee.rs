//! E2E : **une réaction déclarée au manifeste change une ligne que personne
//! n'a demandé de changer.**
//!
//! Le témoin du montage (11 octobre 2026), et la seule chose qui prouve que le
//! lot est vivant *en service* et pas seulement en test. `e2e_reacteur` éprouve
//! déjà le chaînon des nœuds — mais en exécutant le graphe à la main, avec un
//! registre de services monté par le test. Ici, rien n'est monté par le test :
//! un manifeste déclare une réaction, `Backend::open` la monte, un appel
//! d'outil ordinaire (`put_sujet`) émet sur le bus, et la ligne ancrée change
//! d'état. **Aucun appel ne nomme la réaction, nulle part.**
//!
//! Ce qu'il éprouve, dans l'ordre :
//!
//! - la réaction du manifeste est montée à l'ouverture, sans que l'hôte
//!   l'appelle ;
//! - elle part avec ses liaisons (`target`, `relation`, `transition`,
//!   `source`) : un paramètre obligatoire non transmis ferait échouer
//!   l'instanciation, et l'échec paraîtrait dans `reaction_errors()` ;
//! - le filtre `source` tient : écrire la **cible** ne redéclenche pas la
//!   remontée (sinon une mémoire transitionnée relancerait la réaction sans
//!   fin) ;
//! - l'arrêt est attendu et rend les erreurs.
//!
//! Rien ici ne nomme une mémoire ni un sujet : une entité `Fiche` à machine à
//! états, une entité `Sujet`, une relation `ANCRE`. La fiche réactive, elle,
//! est **celle du gabarit livré** (`backends/memory/graphs/review_anchored.mmd`)
//! — un témoin qui écrirait sa propre copie éprouverait sa copie.
//!
//! Run with: ./run_e2e.sh --test e2e_reaction_montee

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::Rag3dbConnection;

/// Le schéma d'une entité à deux champs de texte et un état.
fn schema(etats: &[&str]) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        // `etat` est requis, **comme dans le gabarit livré** : la vue
        // `editable` ne retire que les champs générés (dates, révision), pas le
        // champ de cycle de vie, donc `put_memory` exige `state` de son
        // appelant. Le laisser facultatif a un prix mesuré ici le 11 octobre
        // 2026 : un `put` qui l'omet **réécrit l'état initial** et défait la
        // transition sans le dire (ticket
        // `2026-10-11-un-put-sans-le-champ-de-cycle-de-vie-reecrit-l-etat-initial`).
        "required": ["titre", "corps", "etat", "revision", "created_at", "updated_at"],
        "properties": {
            "titre": {"type": "string", "minLength": 1},
            "corps": {"type": "string", "minLength": 1},
            "etat": {"type": "string", "enum": etats},
            "revision": {"type": "integer", "minimum": 1},
            "created_at": {"type": "integer"},
            "updated_at": {"type": "integer"}
        }
    })
}

/// La configuration d'une entité. **Aucun signal** : ce témoin ne mesure pas
/// la recherche, et un signal vectoriel exigerait un service d'embarquement
/// pour rien (`needs_embeddings` resterait faux, c'est vérifié à l'ouverture).
fn entite(fichier: &str, cycle: Option<Value>) -> Value {
    let mut config = json!({
        "fields": {
            "titre": {"type": "string", "isTitle": true},
            "corps": {"type": "string", "isContent": true},
            "etat": {"type": "string"}
        },
        "hashsafe": ["titre"],
        "returnFields": ["titre", "corps", "etat", "revision"],
        "signals": []
    });
    if let Some(cycle) = cycle {
        config["lifecycle"] = cycle;
    }
    json!({
        "schema": fichier,
        "config": config,
        "writes": {
            "created_at": "created_at",
            "updated_at": "updated_at",
            "revision": "revision"
        }
    })
}

/// L'extension vectorielle de l'arbre où le moteur est bâti. Aucune entité ne
/// déclare de signal vecteur ici, mais le manifeste doit la **nommer** et le
/// moteur la charge à l'ouverture : `RAG3DB_ROOT` ne se dérive jamais du chemin
/// du code (piège 6 du journal — depuis un worktree, l'extension est ailleurs).
fn extension_vecteur() -> String {
    let chemin = crate::common::vector_extension();
    assert!(
        Path::new(&chemin).is_file(),
        "l'extension vectorielle manque : {chemin} — posez RAG3DB_ROOT sur l'arbre où le \
         moteur est bâti"
    );
    chemin
}

/// Le manifeste écrit dans `dir`, et son chemin.
///
/// Les graphes sont nommés en **absolu** vers `templates/` du dépôt : le
/// gabarit livré est ce qu'on veut éprouver, et un chemin relatif depuis un
/// dossier temporaire ne l'atteint pas.
fn manifeste(dir: &Path) -> PathBuf {
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let put = gabarits.join("backends/notebook/graphs/put.mmd");
    let lien = gabarits.join("tools/link.mmd");
    let reaction = gabarits.join("backends/memory/graphs/review_anchored.mmd");
    assert!(put.is_file(), "le graphe put du gabarit notebook : {}", put.display());
    assert!(
        reaction.is_file(),
        "la fiche réactive du gabarit memory : {}",
        reaction.display()
    );

    std::fs::write(
        dir.join("fiche.json"),
        serde_json::to_vec_pretty(&schema(&["courant", "a_revoir"])).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("sujet.json"),
        serde_json::to_vec_pretty(&schema(&["courant"])).unwrap(),
    )
    .unwrap();

    let cycle = json!({
        "field": "etat",
        "initial": "courant",
        "transitions": [{"name": "revoir", "from": "courant", "to": "a_revoir"}]
    });
    let manifest = json!({
        "version": 1,
        "name": "reaction-montee",
        "database": "base.rag3db",
        // Adresse injoignable et modèle `none` : aucune entité ne déclare de
        // signal, donc personne ne la compose. C'est la forme du gabarit
        // `proto/boutique`, qui s'ouvre sans embarqueur.
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": extension_vecteur(),
        "entities": {
            "Fiche": entite("fiche.json", Some(cycle)),
            "Sujet": entite("sujet.json", None)
        },
        "relations": {"ANCRE": {"from": "Fiche", "to": "Sujet"}},
        // `input_payloads` avec la vue `editable` : sans elle, le payload est
        // validé contre le schéma complet et l'écriture se refuse sur les
        // champs que le serveur écrit lui-même (`etat`, `revision`, les
        // dates). C'est la forme du gabarit `memory`.
        "tools": {
            "put_fiche": {
                "graph": put,
                "bindings": {"entity": "Fiche"},
                "input_payloads": {"record": {"entity": "Fiche", "view": "editable"}}
            },
            "put_sujet": {
                "graph": put,
                "bindings": {"entity": "Sujet"},
                "input_payloads": {"record": {"entity": "Sujet", "view": "editable"}}
            },
            // L'ancre se pose **par un outil**, comme un agent la poserait
            // (11 octobre 2026) : jusque-là ce témoin l'écrivait par
            // `Catalog::link`, et il éprouvait le réacteur sans éprouver la
            // surface exposée — c'est ainsi que le gabarit `memory` a vécu sans
            // aucun outil pour poser son ancre.
            "ancrer": {"graph": lien, "bindings": {"relation": "ANCRE"}}
        },
        // **La seule déclaration qui compte.** Rien d'autre dans ce manifeste
        // ne parle de réagir, et aucun appel plus bas ne nomme
        // « review_anchored ».
        "reactions": {
            "review_anchored": {
                "graph": reaction,
                "bindings": {
                    "target": "Fiche",
                    "relation": "ANCRE",
                    "transition": "revoir",
                    "source": "Sujet"
                }
            }
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    assert_eq!(
        prepared.reaction_graphs().keys().collect::<Vec<_>>(),
        vec!["review_anchored"],
        "la réaction est lue au chargement"
    );
    assert!(
        !prepared.needs_embeddings(),
        "aucun signal déclaré : ce témoin tourne sans service d'embarquement"
    );
    let base = prepared.path(&prepared.manifest.database);
    let conn = Rag3dbConnection::new(&base).expect("la base");
    prepared.open(Box::new(conn), None).expect("l'ouverture")
}

/// L'état d'une fiche, lu en base — pas dans la réponse d'un outil : ce qu'on
/// veut voir, c'est ce que la réaction a **écrit**.
fn etat(backend: &Backend, titre: &str) -> Option<String> {
    let cat = backend.catalog.lock().unwrap();
    cat.execute_raw_with_params(
        "MATCH (n:Fiche {titre: $t}) RETURN n.etat",
        &[QueryParam::new("t", CypherValue::String(titre.to_string()))],
    )
    .ok()?
    .rows
    .first()
    .and_then(|l| l.first())
    .and_then(|v| v.as_str())
    .map(str::to_string)
}

/// Attend que la fiche atteigne `vise`, au plus `limite`. Rend l'état vu.
///
/// Une attente, pas un `sleep` : la fiche réactive déclare
/// `%% policy: debounce 500`, donc la réaction part une demi-seconde après la
/// fin de la rafale — un délai fixe serait soit trop court, soit du temps
/// perdu à chaque passe.
fn attendre_l_etat(backend: &Backend, titre: &str, vise: &str, limite: Duration) -> Option<String> {
    let debut = Instant::now();
    loop {
        let vu = etat(backend, titre);
        if vu.as_deref() == Some(vise) || debut.elapsed() > limite {
            return vu;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore]
fn une_reaction_du_manifeste_transitionne_sans_que_personne_l_appelle() {
    let dir = tempfile::tempdir().expect("dossier");
    let chemin = manifeste(dir.path());
    let backend = ouvrir(&chemin);

    // ── 1. Montée à l'ouverture, sans que l'hôte l'appelle ───────────────
    assert_eq!(
        backend.reaction_runs(),
        0,
        "montée, mais rien n'est encore arrivé"
    );

    let fiche = json!({
        "titre": "duty cycle à 70",
        "corps": "Pour ne pas figer l'écran.",
        "etat": "courant"
    });
    let r = backend
        .call("put_fiche", json!({"record": fiche}))
        .expect("la fiche s'écrit");
    assert_eq!(
        r["result"]["data"]["etat"], "courant",
        "l'état de départ de la transition déclarée : {r}"
    );

    let sujet = json!({
        "titre": "régime GPU",
        "corps": "Comment on ménage les cartes.",
        "etat": "courant"
    });
    backend
        .call("put_sujet", json!({"record": sujet}))
        .expect("le sujet s'écrit");

    // ── 2. Rien n'est ancré : la réaction part et ne transitionne rien ───
    //
    // Ce passage vide aussi le curseur des naissances. Sans lui, on ne saurait
    // pas distinguer « la réaction a marché » de « l'écriture avait déjà tout
    // transitionné ».
    assert!(
        backend.wait_reaction_runs(1, Duration::from_secs(20)),
        "la réaction doit partir sur l'écriture du sujet, sans ancre — runs = {}, erreurs = {:?}",
        backend.reaction_runs(),
        backend.reaction_errors()
    );
    assert_eq!(
        etat(&backend, "duty cycle à 70").as_deref(),
        Some("courant"),
        "rien n'est ancré, donc rien ne transitionne"
    );

    // ── 3. L'ancre, puis le sujet change ─────────────────────────────────
    //
    // Le bus écouté dès ici, sur `catalog` (l'écriture de la réaction) et sur
    // `reactions` (ses runs) : le 3 ter relie les deux.
    let (mut du_catalogue, mut des_reactions) = {
        let bus = backend.catalog.lock().unwrap().event_bus();
        (bus.subscribe(rag3weaver::events::topic::CATALOG), bus.subscribe(rag3weaver::events::topic::REACTIONS))
    };
    backend
        .call(
            "ancrer",
            json!({"links": [{"from": {"titre": "duty cycle à 70"}, "to": {"titre": "régime GPU"}}]}),
        )
        .expect("l'ancre, par l'outil");

    let r = backend
        .call(
            "put_sujet",
            json!({
                "record": {
                    "titre": "régime GPU",
                    "corps": "Le budget de caractères compte aussi.",
                    "etat": "courant"
                },
                "expected_revision": 1
            }),
        )
        .expect("le sujet change");
    assert_eq!(r["result"]["data"]["revision"], 2, "la seconde écriture : {r}");

    // **Le cœur du témoin.** Personne n'a nommé la réaction.
    let vu = attendre_l_etat(&backend, "duty cycle à 70", "a_revoir", Duration::from_secs(25));
    assert_eq!(
        vu.as_deref(),
        Some("a_revoir"),
        "la fiche ancrée doit avoir transitionné — runs = {}, erreurs = {:?}",
        backend.reaction_runs(),
        backend.reaction_errors()
    );
    assert_eq!(
        backend.reaction_errors(),
        Vec::<String>::new(),
        "et sans erreur : une liaison perdue ferait échouer l'instanciation ici"
    );

    // ── 3 bis. **La réaction a écrit, et ne se relance pas sans fin** ─────
    //
    // Depuis le 19 octobre 2026, le catalogue publie aussi les transitions
    // (`EntitiesChanged`, `change: transitioned`) : l'écriture de la réaction
    // part sur `catalog`, la sonnette du réacteur l'entend, et la réaction
    // repasse **une fois** sur son propre événement — la fiche n'est pas sa
    // source, elle n'écrit rien, donc rien ne la relance. Le compte se stabilise.
    std::thread::sleep(Duration::from_millis(1500));
    let apres_l_ecriture = backend.reaction_runs();
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(
        backend.reaction_runs(),
        apres_l_ecriture,
        "une réaction qui écrit publie son écriture et ne se relance pas sans fin — runs = {}",
        backend.reaction_runs()
    );

    // ── 3 ter. **L'écriture dit quel run l'a faite, et ce run s'annonce** ─
    //
    // La transition publiée sur `catalog` porte le run du nœud qui l'a écrite
    // (la pièce 1 de la mémoire), et ce run est celui que le réacteur annonce
    // sur `reactions` par son `RunStarted`, cible la réaction (la publication
    // des runs, 5f9cf0983) : de l'écriture, on remonte à la réaction qui l'a
    // faite, par le bus seul.
    let vider = |rx: &mut async_broadcast::Receiver<rag3weaver::events::Event>| -> Vec<Value> {
        let mut v = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(e) => v.push(e.to_json()),
                // Un récepteur débordé perd les plus anciens et continue : le
                // dire plutôt que de s'arrêter là.
                Err(async_broadcast::TryRecvError::Overflowed(n)) => v.push(json!({ "debordement": n })),
                Err(_) => return v,
            }
        }
    };
    let ecrits = vider(&mut du_catalogue);
    let transition = ecrits
        .iter()
        .find(|e| e["change"] == "transitioned" && e["entity"] == "Fiche")
        .unwrap_or_else(|| panic!("aucune transition de Fiche sur catalog : {ecrits:?}"));
    let run = transition["run"].as_str().unwrap_or_else(|| panic!("la transition ne dit pas son run : {transition}"));
    let annonces = vider(&mut des_reactions);
    let debut = annonces
        .iter()
        .find(|e| e["kind"] == "RunStarted" && e["run"] == run)
        .unwrap_or_else(|| panic!("le run {run} de la transition n'est pas annoncé sur reactions : {annonces:?}"));
    assert_eq!(debut["target"], json!({ "reaction": "review_anchored" }), "le run qui a écrit est celui de la réaction : {debut}");

    // ── 4. Le filtre `source` est DANS le graphe, pas à la sonnette ──────
    //
    // **Relevé ici le 11 octobre 2026, et mon assertion disait le contraire.**
    // Écrire la CIBLE fait bien partir la réaction : la sonnette du réacteur
    // écoute le sujet `catalog` tout entier, et c'est `ReactTransitionNode`
    // qui écarte ensuite les événements d'une autre entité que la source
    // déclarée. Ce qu'on veut donc voir, ce n'est pas « elle ne part pas »,
    // c'est **« rien ne cascade »** : une exécution de plus, aucune écriture,
    // donc aucun événement, donc pas d'exécution suivante. Sans ce filtre, la
    // fiche que la réaction vient d'écrire rouvrirait une remontée.
    let avant = backend.reaction_runs();
    backend
        .call(
            "put_fiche",
            json!({
                // `etat` rendu tel qu'il est : omis, il serait réécrit à
                // l'initial et défairait la transition (voir le schéma).
                "record": {
                    "titre": "duty cycle à 70",
                    "corps": "Troisième écriture de la fiche.",
                    "etat": "a_revoir"
                },
                // 1 et non 2 : la transition a écrit l'état **sans** monter la
                // révision — le verbe de cycle de vie n'est pas une édition du
                // record. Une révision attendue protège des éditions
                // concurrentes, pas d'un changement d'état.
                "expected_revision": 1
            }),
        )
        .expect("la fiche change");
    assert!(
        backend.wait_reaction_runs(avant + 1, Duration::from_secs(20)),
        "la réaction part aussi sur un changement de la cible — runs = {}",
        backend.reaction_runs()
    );
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(
        backend.reaction_runs(),
        avant + 1,
        "une seule exécution de plus : elle n'a rien écrit, donc rien ne la relance"
    );
    assert_eq!(
        etat(&backend, "duty cycle à 70").as_deref(),
        Some("a_revoir"),
        "et elle n'a rien réécrit sur la cible"
    );
    assert_eq!(backend.reaction_errors(), Vec::<String>::new());

    // ── 5. L'arrêt est attendu, et il rend ce qu'il a vu ─────────────────
    assert_eq!(backend.stop_reactions(), Vec::<String>::new());
    assert_eq!(backend.reaction_runs(), 0, "plus rien en service");
}
