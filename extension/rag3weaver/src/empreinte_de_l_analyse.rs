//! **Le garde-fou de l'empreinte de l'analyse** (c7, 20 novembre 2026). Un lot qui change le texte, le découpage, la
//! candidature ou les arêtes d'un scope monte `REGLES_DE_L_ANALYSE` dans le même commit : sans montée, une base reçoit
//! le changement par morceaux (seulement sur les fichiers qu'elle réanalyse), et le lot suivant qui monte les règles
//! en porte le chapeau. Le 14 novembre, 57bfc142bf a borné le contenu d'un scope à ses octets sans monter les règles ;
//! la repasse des règles 9 l'a appliqué d'un coup à l'arbre de luciepc, et la recherche y a perdu dix points.
//!
//! Le témoin analyse un corpus figé (`tests/fixtures/empreinte_analyse/`, un ou deux fichiers par langue, avec les
//! formes qui ont déjà bougé), en tire une empreinte stable (les scopes et leur texte, les relations, les rendez-vous,
//! les morceaux et la candidature de chaque scope, sans la version de l'analyse ni les chemins absolus) et la compare
//! à la table `tests/fixtures/empreinte_analyse.txt` (`<règles> <empreinte>`). Une empreinte qui change sans que le
//! numéro monte est rouge ; monter les règles, c'est écrire la ligne de la nouvelle empreinte, et ce geste se lit dans
//! le diff du lot. Un pointeur codeparsers qui change l'empreinte monte les règles aussi.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::chunker::{Chunker, ChunkerConfig};
use crate::code::{analyze, scope_chunking_with, CONTAINER_MIN_CHARS, REGLES_DE_L_ANALYSE};

fn racine() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/empreinte_analyse")
}

fn fichiers(d: &Path, v: &mut Vec<PathBuf>) {
    let Ok(lu) = std::fs::read_dir(d) else { return };
    for e in lu.flatten() {
        let p = e.path();
        if p.is_dir() {
            fichiers(&p, v)
        } else {
            v.push(p)
        }
    }
}

/// Le corpus figé, en chemins relatifs, trié.
fn corpus() -> Vec<(String, String)> {
    let r = racine();
    let mut v = Vec::new();
    fichiers(&r, &mut v);
    let mut sources: Vec<(String, String)> = v
        .iter()
        .map(|p| (p.strip_prefix(&r).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(p).unwrap().replace("\r\n", "\n")))
        .collect();
    sources.sort();
    sources
}

/// Ôte ce qui ne dit rien de l'analyse (la version, qui porte déjà les règles et le commit du parseur ; les chemins
/// absolus), et trie les listes : l'empreinte ne dépend que du contenu, pas de l'ordre d'un parcours.
fn canonique(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(m) => {
            for cle in ["root", "version_de_l_analyse", "absolute_path"] {
                m.remove(cle);
            }
            for x in m.values_mut() {
                canonique(x);
            }
        }
        serde_json::Value::Array(a) => {
            for x in a.iter_mut() {
                canonique(x);
            }
            a.sort_by_cached_key(|x| x.to_string());
        }
        _ => {}
    }
}

/// L'empreinte de l'analyse du corpus figé, en hexadécimal.
pub(crate) fn empreinte() -> String {
    let analyse = analyze("/empreinte", corpus());
    let mut v = serde_json::to_value(&analyse).expect("l'analyse se sérialise");
    canonique(&mut v);
    let decoupage = scope_chunking_with(CONTAINER_MIN_CHARS);
    let decoupeur = Chunker::new(ChunkerConfig::from(&decoupage));
    let mut morceaux: Vec<serde_json::Value> = analyse
        .scopes
        .iter()
        .map(|s| {
            let candidat = !decoupage.not_candidate.as_ref().is_some_and(|r| r.excludes(&s.data()));
            let textes: Vec<String> = if candidat { decoupeur.chunk(&s.content).into_iter().map(|c| c.text).collect() } else { Vec::new() };
            serde_json::json!({ "key": s.key, "candidat": candidat, "morceaux": textes })
        })
        .collect();
    morceaux.sort_by_cached_key(|x| x.to_string());
    let tout = serde_json::json!({ "analyse": v, "morceaux": morceaux });
    let mut h = Sha256::new();
    h.update(tout.to_string().as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// La table versionnée `règles → empreinte`.
fn table() -> Vec<(u32, String)> {
    let t = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/empreinte_analyse.txt")).unwrap_or_default();
    t.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (n, e) = l.split_once(char::is_whitespace)?;
            Some((n.parse().ok()?, e.trim().to_string()))
        })
        .collect()
}

#[test]
fn l_empreinte_de_l_analyse_ne_change_qu_avec_les_regles() {
    let calculee = empreinte();
    let table = table();
    match table.iter().find(|(n, _)| *n == REGLES_DE_L_ANALYSE) {
        None => panic!(
            "les règles de l'analyse sont à {REGLES_DE_L_ANALYSE}, sans empreinte notée : ajouter la ligne\n{REGLES_DE_L_ANALYSE} {calculee}\nà tests/fixtures/empreinte_analyse.txt"
        ),
        Some((_, notee)) => assert_eq!(
            notee, &calculee,
            "l'analyse change (empreinte {calculee}, notée {notee} pour les règles {REGLES_DE_L_ANALYSE}) sans que REGLES_DE_L_ANALYSE monte : \
             monter les règles dans code.rs, avec sa ligne d'historique, et noter la nouvelle empreinte"
        ),
    }
}

/// Deux calculs de suite donnent la même empreinte : elle ne dépend d'aucun ordre de parcours ni d'aucun état.
#[test]
fn l_empreinte_est_stable() {
    assert_eq!(empreinte(), empreinte());
}
