//! **La mesure bornée des déplacés** (2a′-2, page
//! `docs/everything-declarative/14-11-2026/06-l-ecrivain-de-2a-prime-2-et-le-cache-par-rang.md`, § 7 ;
//! la condition d'arbre principal, page 03, § 4). Elle décide si la position d'un scope reste dans
//! l'identité de sa ligne.
//!
//! Sous des lignes immuables, un scope dont le texte ne change pas mais dont les lignes bougent prend
//! une ligne neuve : ses arêtes entrantes se recopient. La sonde compte, sur les
//! `RAG3WEAVER_SONDE_COMMITS` derniers commits de la branche `RAG3WEAVER_SONDE_REF` (master par défaut)
//! du dépôt `RAG3WEAVER_SONDE_DEPOT` :
//! - pour chaque fichier de code touché par le commit, ses scopes avant et après, analysés par
//!   `code::analyze` et appariés par leur clé : **changés** (le texte diffère), **déplacés seulement**
//!   (même texte, autre ligne de début), ajoutés, retirés ;
//! - les **arêtes entrantes** des changés et des déplacés, lues sur une copie d'une base de ce dépôt
//!   (`RAG3WEAVER_SONDE_BASE`), par (chemin, nom, genre).
//!
//! **Une approximation, dite** : les arêtes entrantes viennent d'une seule base, prise à un instant, et
//! non de chaque commit. Elles disent l'ordre de grandeur des recopies, pas leur compte exact.
//!
//! **Le seuil** (arbre principal) : si les déplacés font plus de la moitié des recopies, **et** qu'un
//! commit médian dépasse quelques milliers d'arêtes, la position sort de l'identité de la ligne.

use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::Rag3dbConnection;
use std::collections::BTreeMap;
use std::process::Command;

fn git(depot: &str, args: &[&str]) -> Option<String> {
    let o = Command::new("git").arg("-C").arg(depot).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Les scopes d'une version d'un fichier : clé → (texte, ligne de début, nom, genre).
fn scopes(chemin: &str, texte: &str) -> BTreeMap<String, (String, usize, String, String)> {
    let a = rag3weaver::code::analyze("/sonde", vec![(chemin.to_string(), texte.to_string())]);
    a.scopes.into_iter().map(|s| (s.key, (s.content, s.start_line, s.name, s.scope_type))).collect()
}

fn mediane(mut v: Vec<usize>) -> usize {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[v.len() / 2]
}

#[test]
#[ignore]
fn la_mesure_bornee_des_deplaces() {
    let (Some(depot), Some(base)) = (std::env::var("RAG3WEAVER_SONDE_DEPOT").ok(), std::env::var_os("RAG3WEAVER_SONDE_BASE")) else {
        eprintln!("SAUTÉ la_mesure_bornee_des_deplaces : RAG3WEAVER_SONDE_DEPOT ou RAG3WEAVER_SONDE_BASE manque (une copie, jamais la base en service)");
        return;
    };
    let n: usize = std::env::var("RAG3WEAVER_SONDE_COMMITS").ok().and_then(|v| v.parse().ok()).unwrap_or(50);
    let reference = std::env::var("RAG3WEAVER_SONDE_REF").unwrap_or_else(|_| "origin/master".into());
    let conn = Rag3dbConnection::new(&base).expect("ouvrir la copie");
    let commits: Vec<String> = git(&depot, &["log", "--first-parent", "--format=%H", "-n", &n.to_string(), &reference])
        .expect("git log")
        .lines()
        .map(str::to_string)
        .collect();
    eprintln!("[sonde] {} commits de {reference} dans {depot}", commits.len());

    // Les arêtes entrantes par (chemin, nom, genre), lues une fois par fichier.
    let mut entrantes: BTreeMap<String, BTreeMap<(String, String), usize>> = BTreeMap::new();
    let mut entrantes_de = |chemin: &str| -> BTreeMap<(String, String), usize> {
        entrantes
            .entry(chemin.to_string())
            .or_insert_with(|| {
                let r = conn
                    .execute_with_params(
                        "MATCH ()-[r]->(s:Scope) WHERE s.file_path = $f RETURN s.name, s.scope_type, count(r)",
                        &[QueryParam::new("f", CypherValue::String(chemin.to_string()))],
                    )
                    .map(|r| r.rows)
                    .unwrap_or_default();
                r.into_iter()
                    .filter_map(|l| {
                        let nom = l.first()?.as_str()?.to_string();
                        let genre = l.get(1)?.as_str()?.to_string();
                        let k = l.get(2)?.as_i64()? as usize;
                        Some(((nom, genre), k))
                    })
                    .collect()
            })
            .clone()
    };

    let (mut par_commit, mut des_deplaces, mut des_changes) = (Vec::new(), 0usize, 0usize);
    for c in &commits {
        let Some(fichiers) = git(&depot, &["diff", "--name-only", &format!("{c}^"), c]) else { continue };
        let (mut changes, mut deplaces, mut ajoutes, mut retires, mut a_changes, mut a_deplaces, mut nf) = (0, 0, 0, 0, 0usize, 0usize, 0);
        for f in fichiers.lines() {
            let avant = git(&depot, &["show", &format!("{c}^:{f}")]).unwrap_or_default();
            let apres = git(&depot, &["show", &format!("{c}:{f}")]).unwrap_or_default();
            let (sa, sb) = (scopes(f, &avant), scopes(f, &apres));
            if sa.is_empty() && sb.is_empty() {
                continue;
            }
            nf += 1;
            let e = entrantes_de(f);
            for (cle, (texte, debut, nom, genre)) in &sb {
                match sa.get(cle) {
                    None => ajoutes += 1,
                    Some((t0, d0, _, _)) if t0 != texte => {
                        changes += 1;
                        a_changes += e.get(&(nom.clone(), genre.clone())).copied().unwrap_or(0);
                    }
                    Some((_, d0, _, _)) if d0 != debut => {
                        deplaces += 1;
                        a_deplaces += e.get(&(nom.clone(), genre.clone())).copied().unwrap_or(0);
                    }
                    _ => {}
                }
            }
            retires += sa.keys().filter(|k| !sb.contains_key(*k)).count();
        }
        let titre = git(&depot, &["log", "-1", "--format=%s", c]).unwrap_or_default();
        eprintln!(
            "[sonde] {} {nf:>3} fichiers de code ; scopes changés {changes:>4}, déplacés {deplaces:>4}, ajoutés {ajoutes:>4}, retirés {retires:>4} ; arêtes recopiées : changés {a_changes:>6}, déplacés {a_deplaces:>6} — {}",
            &c[..9],
            titre.trim().chars().take(60).collect::<String>()
        );
        par_commit.push(a_changes + a_deplaces);
        des_deplaces += a_deplaces;
        des_changes += a_changes;
    }
    let total = des_deplaces + des_changes;
    let part = if total == 0 { 0.0 } else { des_deplaces as f64 / total as f64 };
    let med = mediane(par_commit.clone());
    let avec_code: Vec<usize> = par_commit.iter().copied().filter(|k| *k > 0).collect();
    eprintln!(
        "[sonde] RÉSUMÉ {} commits : {total} arêtes recopiées, dont {des_deplaces} par les seuls déplacés ({:.0} %) ; médiane par commit {med}, médiane des commits qui recopient {} ; max {}",
        commits.len(),
        part * 100.0,
        mediane(avec_code),
        par_commit.iter().max().copied().unwrap_or(0)
    );
    eprintln!(
        "[sonde] SEUIL : déplacés > 50 % ({}) ET médiane > quelques milliers ({}) → {}",
        if part > 0.5 { "oui" } else { "non" },
        med,
        if part > 0.5 && med > 2_000 { "la position sort de l'identité" } else { "la position reste dans l'identité" }
    );
    let _ = conn.execute("RETURN 1");
}
