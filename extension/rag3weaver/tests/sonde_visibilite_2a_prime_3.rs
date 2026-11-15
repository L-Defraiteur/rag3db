//! **La mesure de 2a′-3** (15 novembre 2026, demandée par c7) : la visibilité d'une ligne qui peut
//! réapparaître (un scope A → B → A retrouve sa ligne), sous trois formes, contre la forme de 2a′-2.
//! - **(2)** les deux colonnes `_depuis` et `_jusqu_a` sur la ligne : la forme d'aujourd'hui (le doublon
//!   au bord haut d'un trou de rattrapage, compté dans l'état) ;
//! - **(3a)** une table d'intervalles (ligne, couche, depuis, jusqu'à), une rangée par intervalle : le
//!   prédicat devient une jointure ;
//! - **(3a′)** les deux colonnes pour l'intervalle principal, et une annexe des seules réapparitions :
//!   le prédicat devient « colonnes OU annexe », vide d'abord, puis à 1 % des lignes.
//!
//! Deux sondes, qui mesurent sans échouer :
//! - `le_cout_des_formes_de_la_visibilite` : sur une **copie** d'une base de code réelle
//!   (`RAG3WEAVER_SONDE_BASE`, jamais la base en service), le jeu de requêtes de lecteur de
//!   `sonde_cout_du_predicat`, médiane de `RAG3WEAVER_SONDE_TOURS` jeux (10 par défaut), et le plan
//!   (`EXPLAIN`) des requêtes qui départagent ;
//! - `le_compte_des_reapparitions` : sur l'histoire git d'un dépôt (`RAG3WEAVER_SONDE_DEPOT`, lu
//!   seulement), les `RAG3WEAVER_SONDE_COMMITS` derniers commits de la première parente (50 par défaut) :
//!   pour chaque clé de scope, l'empreinte de son contenu (sa rangée sans la position) commit après
//!   commit, et le nombre de réapparitions (une empreinte qui revient après en avoir eu une autre).

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::Rag3dbConnection;
use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

fn scalaire(c: &Rag3dbConnection, q: &str) -> String {
    let r = c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
    r.rows.first().and_then(|l| l.first()).map(|v| format!("{v:?}")).unwrap_or_default()
}

fn texte(c: &Rag3dbConnection, q: &str) -> String {
    match c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}")).rows.first().and_then(|l| l.first()) {
        Some(CypherValue::String(s)) => s.clone(),
        v => panic!("{q} : {v:?}"),
    }
}

fn mediane(c: &Rag3dbConnection, q: &str, tours: usize) -> (f64, usize) {
    let mut t = Vec::with_capacity(tours);
    let mut lignes = 0;
    for _ in 0..tours {
        let d = Instant::now();
        let r = c.execute(q).unwrap_or_else(|e| panic!("{q} : {e}"));
        t.push(d.elapsed().as_secs_f64() * 1000.0);
        lignes = r.rows.len();
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (t[t.len() / 2], lignes)
}

fn expliquer(c: &Rag3dbConnection, etiquette: &str, q: &str) {
    match c.execute(&format!("EXPLAIN {q}")) {
        Ok(r) => {
            for l in r.rows {
                for v in l {
                    if let CypherValue::String(s) = v {
                        for ligne in s.lines() {
                            eprintln!("[sonde] EXPLAIN {etiquette} | {ligne}");
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("[sonde] EXPLAIN {etiquette} refusé : {e}"),
    }
}

#[test]
#[ignore]
fn le_cout_des_formes_de_la_visibilite() {
    let Some(base) = std::env::var_os("RAG3WEAVER_SONDE_BASE") else {
        eprintln!("SAUTÉ le_cout_des_formes_de_la_visibilite : RAG3WEAVER_SONDE_BASE manque (une copie, jamais la base en service)");
        return;
    };
    let tours: usize = std::env::var("RAG3WEAVER_SONDE_TOURS").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let c = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let scopes: i64 = scalaire(&c, "MATCH (s:Scope) RETURN count(*)").trim_start_matches("Int(").trim_end_matches(')').parse().unwrap_or(0);
    eprintln!("[sonde] Scope {scopes} lignes, File {}, CONSUMES {}", scalaire(&c, "MATCH (f:File) RETURN count(*)"), scalaire(&c, "MATCH ()-[r:CONSUMES]->() RETURN count(*)"));
    // Les colonnes de 2a′-2, posées sur la copie si elle ne les a pas (une base à versions migrée par 1b).
    for t in ["Scope", "File"] {
        for (col, def) in [("_couche", "STRING DEFAULT 'backend'"), ("_depuis", "INT64 DEFAULT 0"), ("_jusqu_a", "INT64")] {
            match c.execute(&format!("ALTER TABLE {t} ADD {col} {def}")) {
                Ok(_) => eprintln!("[sonde] ALTER {t} ADD {col}"),
                Err(e) => eprintln!("[sonde] ALTER {t} {col} : déjà là ({e})"),
            }
        }
    }
    let fichier = texte(&c, "MATCH (s:Scope) RETURN s.file_path AS f, count(*) AS n ORDER BY n DESC LIMIT 1");
    let nom = texte(&c, "MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN b.name AS n, count(*) AS k ORDER BY k DESC LIMIT 1");
    let source = texte(&c, &format!("MATCH (s:Scope) WHERE s.file_path = '{fichier}' RETURN s.source LIMIT 1"));
    eprintln!("[sonde] fichier « {fichier} », nom « {nom} », source « {source} », {tours} tours");

    // (2) les colonnes, au rang 0 de la couche du backend.
    let col = |v: &str| format!("({v}._couche = 'backend' AND {v}._depuis <= 0 AND ({v}._jusqu_a IS NULL OR {v}._jusqu_a > 0))");
    // (3a) la table d'intervalles : une rangée par intervalle, reliée à sa ligne.
    let intervalle = |v: &str| {
        format!("EXISTS {{ MATCH (i:Intervalle_sonde)-[:INTERVALLE_DE]->({v}) WHERE i.couche = 'backend' AND i.depuis <= 0 AND (i.jusqu_a IS NULL OR i.jusqu_a > 0) }}")
    };
    // (3a′) les colonnes, ou une réapparition de l'annexe.
    let annexe = |v: &str| {
        format!("({} OR EXISTS {{ MATCH (a:Reapparition_sonde)-[:REAPPARITION_DE]->({v}) WHERE a.couche = 'backend' AND a.depuis <= 0 AND (a.jusqu_a IS NULL OR a.jusqu_a > 0) }})", col(v))
    };
    let jeu = |p: &dyn Fn(&str) -> String| -> Vec<(&'static str, String)> {
        vec![
            ("outline (un fichier)", format!("MATCH (s:Scope) WHERE s.file_path = '{fichier}' AND {} RETURN s.name, s.start_line ORDER BY s.start_line", p("s"))),
            ("usages (un saut)", format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = '{nom}' AND {} AND {} RETURN a._uuid", p("a"), p("b"))),
            ("impact (1 à 3, bouts)", format!("MATCH (a:Scope)-[:CONSUMES*1..3]->(b:Scope) WHERE b.name = '{nom}' AND {} AND {} RETURN count(DISTINCT a)", p("a"), p("b"))),
            ("racines d'une source", format!("MATCH (s:Scope) WHERE s.source = '{source}' AND {} RETURN s._uuid", p("s"))),
            ("compte des scopes", format!("MATCH (s:Scope) WHERE {} RETURN count(*)", p("s"))),
        ]
    };
    let jouer = |etiquette: &str, requetes: Vec<(&'static str, String)>| -> Vec<f64> {
        requetes
            .into_iter()
            .map(|(quoi, q)| {
                let (ms, n) = mediane(&c, &q, tours);
                eprintln!("[sonde] {etiquette:>14} | {quoi:<24} | {ms:>9.2} ms | {n} ligne(s)");
                ms
            })
            .collect()
    };

    let colonnes = jouer("(2) colonnes", jeu(&col));
    for (i, (quoi, q)) in jeu(&col).into_iter().enumerate() {
        if i == 1 || i == 4 {
            expliquer(&c, &format!("(2) {quoi}"), &q);
        }
    }

    // (3a) : la table d'intervalles, une rangée par ligne de Scope (pas encore de réapparition).
    c.execute("CREATE NODE TABLE Intervalle_sonde(id STRING, couche STRING, depuis INT64, jusqu_a INT64, PRIMARY KEY(id))").unwrap();
    c.execute("CREATE REL TABLE INTERVALLE_DE(FROM Intervalle_sonde TO Scope)").unwrap();
    let d = Instant::now();
    c.execute("MATCH (s:Scope) CREATE (i:Intervalle_sonde {id: s._uuid, couche: 'backend', depuis: 0})-[:INTERVALLE_DE]->(s)").unwrap();
    eprintln!("[sonde] (3a) {} intervalles posés en {:.0} ms", scalaire(&c, "MATCH (i:Intervalle_sonde) RETURN count(*)"), d.elapsed().as_secs_f64() * 1000.0);
    let intervalles = jouer("(3a) intervalles", jeu(&intervalle));
    for (i, (quoi, q)) in jeu(&intervalle).into_iter().enumerate() {
        if i == 1 || i == 4 {
            expliquer(&c, &format!("(3a) {quoi}"), &q);
        }
    }

    // (3a′) : l'annexe des réapparitions, vide.
    c.execute("CREATE NODE TABLE Reapparition_sonde(id STRING, couche STRING, depuis INT64, jusqu_a INT64, PRIMARY KEY(id))").unwrap();
    c.execute("CREATE REL TABLE REAPPARITION_DE(FROM Reapparition_sonde TO Scope)").unwrap();
    let vide = jouer("(3a′) vide", jeu(&annexe));
    for (i, (quoi, q)) in jeu(&annexe).into_iter().enumerate() {
        if i == 1 || i == 4 {
            expliquer(&c, &format!("(3a′ vide) {quoi}"), &q);
        }
    }
    // (3a′) : l'annexe à 1 % des lignes.
    let un_pour_cent = (scopes / 100).max(1);
    let d = Instant::now();
    c.execute(&format!(
        "MATCH (s:Scope) WITH s LIMIT {un_pour_cent} CREATE (a:Reapparition_sonde {{id: s._uuid, couche: 'backend', depuis: 0}})-[:REAPPARITION_DE]->(s)"
    ))
    .unwrap();
    eprintln!("[sonde] (3a′) {} réapparitions posées en {:.0} ms", scalaire(&c, "MATCH (a:Reapparition_sonde) RETURN count(*)"), d.elapsed().as_secs_f64() * 1000.0);
    let pleine = jouer("(3a′) 1 %", jeu(&annexe));
    for (i, (quoi, q)) in jeu(&annexe).into_iter().enumerate() {
        if i == 1 || i == 4 {
            expliquer(&c, &format!("(3a′ 1 %) {quoi}"), &q);
        }
    }

    let rapport = |etiquette: &str, v: &[f64]| {
        eprintln!(
            "[sonde] BILAN {etiquette}/colonnes (outline, usages, impact, racines, compte) : {}",
            colonnes.iter().zip(v).map(|(a, b)| format!("{:.2}", b / a.max(0.001))).collect::<Vec<_>>().join(" ")
        )
    };
    rapport("(3a)", &intervalles);
    rapport("(3a′ vide)", &vide);
    rapport("(3a′ 1 %)", &pleine);
}

/// L'empreinte d'un scope : sa rangée sans la position (la position sort de l'identité en 2a′-3).
fn empreinte(sc: &rag3weaver::code::ScopeRecord) -> String {
    let mut d = sc.data();
    for k in ["start_line", "end_line", "start_byte", "end_byte"] {
        d.remove(k);
    }
    blake3::hash(format!("{d:?}").as_bytes()).to_hex()[..16].to_string()
}

fn git(depot: &str, args: &[&str]) -> Option<String> {
    let o = std::process::Command::new("git").arg("-C").arg(depot).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

#[test]
#[ignore]
fn le_compte_des_reapparitions() {
    let Some(depot) = std::env::var("RAG3WEAVER_SONDE_DEPOT").ok() else {
        eprintln!("SAUTÉ le_compte_des_reapparitions : RAG3WEAVER_SONDE_DEPOT manque (un dépôt git, lu seulement)");
        return;
    };
    let n: usize = std::env::var("RAG3WEAVER_SONDE_COMMITS").ok().and_then(|v| v.parse().ok()).unwrap_or(50);
    let tete = std::env::var("RAG3WEAVER_SONDE_TETE").unwrap_or_else(|_| "HEAD".into());
    // Les commits de la première parente, du plus vieux au plus récent, et celui d'avant pour l'état de départ.
    // `RAG3WEAVER_SONDE_CODE_SEUL=1` : les commits qui touchent du code seulement (sous src/, extension/ ou
    // tools/), pour un échantillon qui ne soit pas fait de commits de docs.
    let code_seul = std::env::var("RAG3WEAVER_SONDE_CODE_SEUL").as_deref() == Ok("1");
    let mut args: Vec<String> = vec!["rev-list".into(), "--first-parent".into(), "-n".into(), (n + 1).to_string(), tete.clone()];
    if code_seul {
        args.extend(["--".into(), "src".into(), "extension".into(), "tools".into()]);
    }
    let liste = git(&depot, &args.iter().map(String::as_str).collect::<Vec<_>>()).expect("rev-list");
    let mut commits: Vec<String> = liste.lines().map(str::to_string).collect();
    commits.reverse();
    let admis = |p: &str| {
        let ext = p.rsplit('.').next().unwrap_or("");
        matches!(ext, "rs" | "cpp" | "cc" | "c" | "h" | "hpp" | "py" | "ts" | "tsx" | "js" | "go" | "java" | "cs") && !p.contains("third_party/")
    };
    // Les fichiers que la fenêtre touche : seuls eux peuvent faire réapparaître un scope.
    let mut touches: BTreeSet<String> = BTreeSet::new();
    let mut par_commit: Vec<Vec<String>> = Vec::new();
    for w in commits.windows(2) {
        let diff = git(&depot, &["diff", "--name-only", &w[0], &w[1]]).unwrap_or_default();
        let f: Vec<String> = diff.lines().filter(|p| admis(p)).map(str::to_string).collect();
        touches.extend(f.iter().cloned());
        par_commit.push(f);
    }
    eprintln!("[sonde] {} commits de {depot} ({}), dont {} touchent du code, {} fichiers de code touchés", commits.len() - 1, &tete, par_commit.iter().filter(|f| !f.is_empty()).count(), touches.len());
    // L'histoire de chaque clé : son empreinte à chaque commit (None : absente).
    let mut histoire: HashMap<String, Vec<Option<String>>> = HashMap::new();
    let mut etat: HashMap<String, Option<String>> = HashMap::new();
    let analyser = |commit: &str, chemins: &[String]| -> (HashMap<String, String>, BTreeSet<String>) {
        let sources: Vec<(String, String)> =
            chemins.iter().filter_map(|p| git(&depot, &["show", &format!("{commit}:{p}")]).map(|t| (p.clone(), t))).collect();
        let _ = commit;
        // Un curseur constant : la clé d'un scope porte sa source, qui ne doit pas changer de commit en commit.
        let a = rag3weaver::code::analyze_in_project("/", sources, "sonde", None);
        let fichiers: BTreeSet<String> = chemins.iter().cloned().collect();
        (a.scopes.iter().map(|sc| (sc.key.clone(), empreinte(sc))).collect(), fichiers)
    };
    let d = Instant::now();
    let touches_v: Vec<String> = touches.iter().cloned().collect();
    let (depart, _) = analyser(&commits[0], &touches_v);
    for (k, e) in depart {
        etat.insert(k, Some(e));
    }
    let fichier_de = |cle: &str| cle.split('#').nth(1).unwrap_or("").to_string();
    for (i, f) in par_commit.iter().enumerate() {
        let commit = &commits[i + 1];
        let (neufs, fichiers) = analyser(commit, f);
        // Les clés des fichiers relus : absentes sauf si l'analyse les rend.
        for (k, v) in etat.iter_mut() {
            if fichiers.contains(&fichier_de(k)) && !neufs.contains_key(k) {
                *v = None;
            }
        }
        for (k, e) in neufs {
            etat.insert(k, Some(e));
        }
        for (k, v) in &etat {
            histoire.entry(k.clone()).or_default().push(v.clone());
        }
    }
    // Compter : un changement (l'empreinte bouge), une réapparition (une empreinte, ou une présence après
    // une absence, revient à une valeur déjà vue avant la précédente).
    let (mut changements, mut reapparitions, mut cles_reapparues) = (0usize, 0usize, 0usize);
    let mut exemples = Vec::new();
    for (k, h) in &histoire {
        let mut vus: BTreeSet<Option<String>> = BTreeSet::new();
        let mut a_reapparu = false;
        let mut avant: Option<&Option<String>> = None;
        for v in h {
            if let Some(p) = avant {
                if p != v {
                    changements += 1;
                    if vus.contains(v) && v.is_some() {
                        reapparitions += 1;
                        a_reapparu = true;
                    }
                }
                vus.insert(p.clone());
            }
            avant = Some(v);
        }
        if a_reapparu {
            cles_reapparues += 1;
            if exemples.len() < 10 {
                exemples.push(k.clone());
            }
        }
    }
    eprintln!("[sonde] {} clés suivies, {changements} changements, {reapparitions} réapparitions sur {cles_reapparues} clés ({:.1} s)", histoire.len(), d.elapsed().as_secs_f64());
    for e in exemples {
        eprintln!("[sonde] réapparue : {e}");
    }
}
