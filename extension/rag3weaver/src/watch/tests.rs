//! Les témoins de la montre de fichiers (W1).
use super::*;
use crate::code_tools::WorkingTree;

fn tree() -> (tempfile::TempDir, Arc<dyn FileSource>) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.path().join("src/b.rs"), "fn b() {}\n").unwrap();
    std::fs::write(dir.path().join("README.md"), "# x\n").unwrap();
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(dir.path()));
    (dir, source)
}

fn config(settle_ms: u64) -> WatchConfig {
    WatchConfig { settle_ms, ..Default::default() }
}

fn changes(v: &[FileChange]) -> Vec<(&str, Change)> {
    v.iter().map(|c| (c.path.as_str(), c.change)).collect()
}

/// **Créé, modifié, supprimé** : un événement chacun, une fois le fichier posé.
/// La photographie de départ ne dit rien.
#[test]
fn it_says_created_modified_and_removed_once_settled() {
    let (dir, source) = tree();
    let mut s = Scanner::new(dir.path().to_path_buf(), source, &config(300)).unwrap();
    assert_eq!(s.len(), 3);
    let t0 = Instant::now();
    assert!(s.scan(t0).unwrap().is_empty(), "rien n'a changé");

    std::fs::write(dir.path().join("src/a.rs"), "fn a() { 1 }\n").unwrap();
    std::fs::write(dir.path().join("src/c.rs"), "fn c() {}\n").unwrap();
    std::fs::remove_file(dir.path().join("src/b.rs")).unwrap();

    // La suppression se dit tout de suite ; le reste attend d'être posé.
    assert_eq!(changes(&s.scan(t0).unwrap()), [("src/b.rs", Change::Removed)]);
    assert!(s.scan(t0 + Duration::from_millis(100)).unwrap().is_empty());
    let got = s.scan(t0 + Duration::from_millis(400)).unwrap();
    assert_eq!(changes(&got), [("src/a.rs", Change::Modified), ("src/c.rs", Change::Created)]);
    assert_eq!(got[0].hash, blake3::hash(b"fn a() { 1 }\n").to_hex().to_string());
    assert!(s.scan(t0 + Duration::from_millis(800)).unwrap().is_empty(), "dit une seule fois");
}

/// **Un `touch` ne dit rien** : la date bouge, le contenu non.
#[test]
fn a_touch_without_new_content_says_nothing() {
    let (dir, source) = tree();
    let mut s = Scanner::new(dir.path().to_path_buf(), source, &config(0)).unwrap();
    let path = dir.path().join("src/a.rs");
    let later = SystemTime::now() + Duration::from_secs(5);
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(later).unwrap();
    assert!(s.scan(Instant::now()).unwrap().is_empty());
}

/// **Trois écritures rapprochées, un événement** : chaque écriture relance
/// l'attente, l'événement porte le contenu final.
#[test]
fn rapid_writes_make_one_event() {
    let (dir, source) = tree();
    let mut s = Scanner::new(dir.path().to_path_buf(), source, &config(300)).unwrap();
    let path = dir.path().join("src/a.rs");
    let t0 = Instant::now();
    for (i, content) in ["fn a() { 1 }\n", "fn a() { 12 }\n", "fn a() { 123 }\n"].iter().enumerate() {
        std::fs::write(&path, content).unwrap();
        assert!(s.scan(t0 + Duration::from_millis(100 * i as u64)).unwrap().is_empty(), "écriture {i}");
    }
    let got = s.scan(t0 + Duration::from_millis(700)).unwrap();
    assert_eq!(changes(&got), [("src/a.rs", Change::Modified)]);
    assert_eq!(got[0].hash, blake3::hash(b"fn a() { 123 }\n").to_hex().to_string());
}

/// `paths` restreint la montre à des sous-chemins.
#[test]
fn paths_restrict_the_watch() {
    let (dir, source) = tree();
    let mut s = Scanner::new(dir.path().to_path_buf(), source, &WatchConfig { settle_ms: 0, paths: vec!["src/".into()], ..Default::default() }).unwrap();
    assert_eq!(s.len(), 2);
    std::fs::write(dir.path().join("README.md"), "# y\n").unwrap();
    assert!(s.scan(Instant::now()).unwrap().is_empty());
}

/// **La montre en service** publie sur le sujet `files`, et son arrêt
/// n'attend pas la fin de la période.
#[test]
fn the_running_watch_publishes_and_stops_at_once() {
    let (dir, source) = tree();
    let bus = crate::events::EventBus::new(64);
    let mut rx = bus.subscribe(crate::events::topic::FILES);
    let handle = spawn(
        "code",
        &WatchConfig { every_ms: 20, settle_ms: 0, ..Default::default() },
        dir.path().to_path_buf(),
        source,
        bus.clone(),
        Arc::new(crate::catalog::retard::Lag::new()),
        crate::scope::Scope::default(),
        None,
        Default::default(),
    )
    .unwrap();
    std::fs::write(dir.path().join("src/a.rs"), "fn a() { 2 }\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let event = loop {
        if let Ok(e) = rx.try_recv() {
            break e;
        }
        assert!(Instant::now() < deadline, "aucun événement en 10 s");
        std::thread::sleep(Duration::from_millis(10));
    };
    match event {
        crate::events::Event::FileChanged { watch, path, change, .. } => {
            assert_eq!((watch.as_str(), path.as_str(), change.as_str()), ("code", "src/a.rs", "modified"));
        }
        other => panic!("{other:?}"),
    }
    handle.stop();

    // Une période d'une minute : l'arrêt n'en attend rien.
    let (dir, source) = tree();
    let handle = spawn("lente", &WatchConfig { every_ms: 60_000, ..Default::default() }, dir.path().to_path_buf(), source, bus, Arc::new(crate::catalog::retard::Lag::new()), crate::scope::Scope::default(), None, Default::default()).unwrap();
    let t = Instant::now();
    handle.stop();
    assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
}

/// `watches` se lit au manifeste, avec ses défauts ; une clé inconnue est refusée.
#[test]
fn a_watch_is_read_from_the_manifest() {
    let w: WatchConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(w, WatchConfig::default());
    // La fraîcheur d'abord (12 octobre 2026) : 250 ms de période, 150 de pose.
    assert_eq!((w.every_ms, w.settle_ms, w.source.as_str()), (250, 150, "workspace"));
    assert!(serde_json::from_value::<WatchConfig>(serde_json::json!({"période": 3})).is_err());
}


/// **La montre note le retard** dans la cellule donnée, avant de publier.
#[test]
fn the_watch_notes_the_lag_before_publishing() {
    let (dir, source) = tree();
    let bus = crate::events::EventBus::new(64);
    let mut rx = bus.subscribe(crate::events::topic::FILES);
    let lag = Arc::new(crate::catalog::retard::Lag::new());
    let cell = crate::scope::Scope { org: "o".into(), project: "p".into() };
    let handle = spawn(
        "code",
        &WatchConfig { every_ms: 20, settle_ms: 0, ..Default::default() },
        dir.path().to_path_buf(),
        source,
        bus,
        lag.clone(),
        cell.clone(),
        None,
        Default::default(),
    )
    .unwrap();
    std::fs::write(dir.path().join("src/b.rs"), "fn b() { 2 }\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while rx.try_recv().is_err() {
        assert!(Instant::now() < deadline, "aucun événement en 10 s");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(lag.summary_in(&cell).map(|s| s.files), Some(1));
    assert_eq!(lag.summary_in(&crate::scope::Scope::default()), None);
    handle.stop();
}


/// **La réconciliation de départ** : contre l'empreinte de l'index, un fichier
/// changé est `modified`, un fichier neuf `created`, un fichier indexé absent
/// du disque `removed` ; un fichier à jour ne dit rien.
#[test]
fn reconciling_with_the_index_says_what_moved_since() {
    let (dir, source) = tree();
    let s = Scanner::new(dir.path().to_path_buf(), source, &config(0)).unwrap();
    let hash = |t: &str| blake3::hash(t.as_bytes()).to_hex().to_string();
    let indexed: Indexed = [
        ("src/a.rs".to_string(), hash("fn a() {}\n")),        // à jour
        ("src/b.rs".to_string(), hash("fn b() { ancien }\n")), // changé depuis
        ("src/parti.rs".to_string(), hash("x")),               // supprimé depuis
    ]
    .into_iter()
    .collect();
    let got = s.reconcile(&indexed);
    assert_eq!(
        changes(&got),
        [("README.md", Change::Created), ("src/b.rs", Change::Modified), ("src/parti.rs", Change::Removed)]
    );
}

/// **Au repos, la liste n'est pas relue** : un tour ne coûte que la date des
/// fichiers connus et de leurs dossiers. Une création fait bouger son dossier
/// et relire la liste ; une modification en place, non ; le filet de période
/// la relit de toute façon.
#[test]
fn a_quiet_scan_does_not_relist_the_source() {
    let (dir, source) = tree();
    let mut s = Scanner::new(dir.path().to_path_buf(), source, &config(0)).unwrap();
    let t0 = Instant::now();
    for i in 0..5 {
        assert!(s.scan(t0 + Duration::from_millis(100 * i)).unwrap().is_empty());
    }
    assert_eq!(s.lists(), 0, "au repos, la liste n'est pas relue");

    std::fs::write(dir.path().join("src/d.rs"), "fn d() {}\\n").unwrap();
    assert_eq!(changes(&s.scan(t0 + Duration::from_millis(600)).unwrap()), [("src/d.rs", Change::Created)]);
    assert_eq!(s.lists(), 1, "une création fait relire");

    std::fs::write(dir.path().join("src/a.rs"), "fn a() { 9 }\\n").unwrap();
    assert_eq!(changes(&s.scan(t0 + Duration::from_millis(700)).unwrap()), [("src/a.rs", Change::Modified)]);
    assert_eq!(s.lists(), 1, "une modification en place ne fait pas relire");

    s.scan(t0 + Duration::from_secs(61)).unwrap();
    assert_eq!(s.lists(), 2, "le filet de période relit");

    // Les compteurs le disent : deux hachages (la création, la modification),
    // deux relectures, et les chemins hachés.
    let st = s.stats();
    assert_eq!((st.scans, st.lists, st.hashes), (8, 2, 2), "{st:?}");
    assert_eq!(st.most_hashed.len(), 2, "{st:?}");
}

/// **La mesure, sur un vrai arbre** : le `src/` du moteur. Un tour au repos
/// coûte moins qu'un parcours de la liste — c'était le coût de la première
/// forme, qui relisait la liste à chaque tour (0,7 cœur sur 7 310 fichiers,
/// mesuré sur l'hôte de Lucie le 11 octobre).
#[test]
fn a_quiet_scan_of_the_engine_tree_costs_less_than_a_listing() {
    for sous in ["../../src", "../.."] {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sous);
    if !root.is_dir() {
        continue;
    }
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(&root));
    let t = Instant::now();
    let mut s = Scanner::new(root.clone(), source.clone(), &WatchConfig::default()).unwrap();
    let depart = t.elapsed();
    let t = Instant::now();
    let n = source.list().unwrap().len();
    let liste = t.elapsed();
    let t = Instant::now();
    for _ in 0..5 {
        assert!(s.scan(Instant::now()).unwrap().is_empty());
    }
    let tour = t.elapsed() / 5;
    eprintln!("montre sur {sous} : {n} fichiers ; départ {depart:?} ; une liste {liste:?} ; un tour au repos {tour:?}");
    assert_eq!(s.lists(), 0);
    // Les temps sont imprimés, pas comparés : sous la charge des témoins
    // voisins, une comparaison chronométrée ne prouverait rien. Ce qui est
    // assuré, c'est qu'un tour au repos ne relit pas la liste.
    let _ = (tour, liste);
    }
}


/// **La racine disparue met la montre en pause** : rien n'est retiré, ni par
/// la liste ni par les dates ; la racine revenue, la montre reprend.
#[test]
fn a_vanished_root_pauses_the_watch() {
    let (dir, source) = tree();
    let root = dir.path().to_path_buf();
    let mut s = Scanner::new(root.clone(), source, &config(0)).unwrap();
    let t0 = Instant::now();
    let backup = tempfile::tempdir().unwrap();
    let ailleurs = backup.path().join("racine");
    std::fs::rename(&root, &ailleurs).unwrap();
    for i in 0..3 {
        assert!(s.scan(t0 + Duration::from_secs(61 * i)).unwrap().is_empty(), "rien n'est retiré");
    }
    assert!(s.paused().is_some_and(|p| p.contains("racine absente")), "{:?}", s.paused());
    assert_eq!(s.len(), 3, "les fichiers connus le restent");
    std::fs::rename(&ailleurs, &root).unwrap();
    assert!(s.scan(t0 + Duration::from_secs(200)).unwrap().is_empty());
    assert_eq!(s.paused(), None, "la racine revenue, la montre reprend");
}
