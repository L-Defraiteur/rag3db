/// **Mesurer avant de construire.**
/// Répond aux questions ouvertes du cahier des charges du 30 août :
/// les scopes de premier niveau se recouvrent-ils déjà ? les offsets tiennent-ils
/// à la ligne ? combien de fichiers sont aujourd'hui hors index ?
///
/// Usage : cargo run --example couverture -- <racine> [<racine>...]
use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask, SupportedLanguage};
use codeparsers::parallel::project_parser::detect_language_from_path;

use std::collections::BTreeMap;
use std::path::Path;

const IGNORES: &[&str] = &["target", ".git", "node_modules", "dist", "build", "generated", ".venv"];

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORES.contains(&name.as_str()) { continue; }
        // `file_type()` de `DirEntry` ne suit pas les liens : `tools/rust_api/rag3db-src`
        // pointe sur la racine du dépôt, et le suivre boucle sans fin.
        let genre = match entry.file_type() { Ok(g) => g, Err(_) => continue };
        if genre.is_symlink() { continue; }
        if genre.is_dir() { walk(&path, out); } else { out.push(path); }
    }
}

fn grammaire(lang: &SupportedLanguage) -> tree_sitter::Language {
    match lang {
        SupportedLanguage::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        SupportedLanguage::Javascript => tree_sitter_typescript::LANGUAGE_TSX.into(),
        SupportedLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SupportedLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SupportedLanguage::Go => tree_sitter_go::LANGUAGE.into(),
        SupportedLanguage::C => tree_sitter_c::LANGUAGE.into(),
        SupportedLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        SupportedLanguage::Csharp => tree_sitter_c_sharp::LANGUAGE.into(),
    }
}

/// Nombre de nœuds ERROR, et si la racine se déclare en erreur.
fn erreurs(lang: &SupportedLanguage, contenu: &str) -> (bool, usize, usize) {
    let mut p = tree_sitter::Parser::new();
    if p.set_language(&grammaire(lang)).is_err() { return (false, 0, 0); }
    let arbre = match p.parse(contenu, None) { Some(t) => t, None => return (false, 0, 0) };
    let racine = arbre.root_node();
    let mut n = 0usize;
    let mut octets = 0usize;
    let mut pile = vec![racine];
    while let Some(noeud) = pile.pop() {
        if noeud.kind() == "ERROR" || noeud.is_missing() {
            n += 1;
            octets += noeud.end_byte().saturating_sub(noeud.start_byte());
            continue; // on ne descend pas sous une erreur : on compte la région
        }
        let mut c = noeud.walk();
        for enfant in noeud.children(&mut c) { pile.push(enfant); }
    }
    (racine.has_error(), n, octets)
}

/// Ligne et texte de la première erreur — pour savoir *pourquoi* ça casse.
fn premiere_erreur(lang: &SupportedLanguage, contenu: &str) -> (usize, String) {
    let mut p = tree_sitter::Parser::new();
    if p.set_language(&grammaire(lang)).is_err() { return (0, String::new()); }
    let arbre = match p.parse(contenu, None) { Some(t) => t, None => return (0, String::new()) };
    let mut pile = vec![arbre.root_node()];
    let mut meilleure: Option<tree_sitter::Node> = None;
    while let Some(noeud) = pile.pop() {
        if noeud.kind() == "ERROR" || noeud.is_missing() {
            if meilleure.map_or(true, |m| noeud.start_byte() < m.start_byte()) { meilleure = Some(noeud); }
            continue;
        }
        let mut c = noeud.walk();
        for enfant in noeud.children(&mut c) { pile.push(enfant); }
    }
    match meilleure {
        Some(n) => {
            let extrait: String = contenu[n.start_byte()..n.end_byte().min(n.start_byte()+120)]
                .replace('\n', "⏎");
            (n.start_position().row + 1, extrait)
        }
        None => (0, String::new()),
    }
}

#[derive(Default)]
struct Bilan {
    fichiers: usize,
    octets: usize,
    octets_couverts: usize,
    fichiers_arbre_en_erreur: usize,
    fichiers_ast_valid_faux: usize,
    fichiers_zero_scope: usize,
    fichiers_avec_recouvrement: usize,
    fichiers_avec_trou: usize,
    octets_trous: usize,
    octets_recouvrements: usize,
    octets_erreur: usize,
    scopes_premier_niveau: usize,
    scopes_total: usize,
    debut_hors_ligne: usize,
}

fn main() {
    let racines: Vec<String> = std::env::args().skip(1).collect();
    if racines.is_empty() { eprintln!("usage: couverture <racine>..."); std::process::exit(2); }

    let mut fichiers = Vec::new();
    for r in &racines { walk(Path::new(r), &mut fichiers); }
    fichiers.sort();

    let mut hors_index: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut bilans: BTreeMap<String, Bilan> = BTreeMap::new();

    for chemin in &fichiers {
        let ext = chemin.extension().and_then(|e| e.to_str()).unwrap_or("«sans»").to_lowercase();
        let brut = match std::fs::read(chemin) { Ok(b) => b, Err(_) => continue };
        let binaire = brut.iter().take(8192).any(|&b| b == 0);
        let contenu = match String::from_utf8(brut) {
            Ok(s) => s,
            Err(e) => {
                let n = e.as_bytes().len();
                let entree = hors_index.entry(format!("{ext} (non-UTF8)")).or_default();
                entree.0 += 1; entree.1 += n;
                continue;
            }
        };
        let langue = detect_language_from_path(&chemin.to_string_lossy());
        let langue = match langue {
            Some(l) if !binaire => match std::env::var("FORCER").as_deref() {
                Ok("cpp") if matches!(l, SupportedLanguage::C) => SupportedLanguage::Cpp,
                _ => l,
            },
            _ => {
                let clef = if binaire { format!("{ext} (binaire)") } else { ext.clone() };
                let entree = hors_index.entry(clef).or_default();
                entree.0 += 1; entree.1 += contenu.len();
                continue;
            }
        };

        let analyse = parse_file(&ParseFileTask {
            file_path: chemin.to_string_lossy().to_string(),
            content: contenu.clone(),
            language: langue.clone(),
        });
        let (racine_en_erreur, n_err, octets_err) = erreurs(&langue, &contenu);
        if std::env::var("DETAIL").is_ok() && octets_err > 0 {
            let (l, c) = premiere_erreur(&langue, &contenu);
            println!("<!-- {} : {} nœuds ERROR, {} octets, 1re à L{} : {} -->",
                chemin.display(), n_err, octets_err, l, c);
        }

        let b = bilans.entry(ext.clone()).or_default();
        b.fichiers += 1;
        b.octets += contenu.len();
        b.scopes_total += analyse.scopes.len();
        if analyse.scopes.is_empty() { b.fichiers_zero_scope += 1; }
        if racine_en_erreur { b.fichiers_arbre_en_erreur += 1; }
        if !analyse.ast_valid { b.fichiers_ast_valid_faux += 1; }
        b.octets_erreur += octets_err;

        // scopes de premier niveau : profondeur 0 et sans parent
        let mut premier: Vec<(usize, usize)> = analyse.scopes.iter()
            .filter(|s| s.depth == 0 && s.parent.is_none())
            .map(|s| (s.scope_start_byte, s.scope_end_byte))
            .collect();
        premier.sort();
        b.scopes_premier_niveau += premier.len();

        // le début d'un scope tombe-t-il en début de ligne ?
        for s in analyse.scopes.iter().filter(|s| s.depth == 0 && s.parent.is_none()) {
            let d = s.scope_start_byte;
            if d != 0 && contenu.as_bytes().get(d.wrapping_sub(1)) != Some(&b'\n') {
                b.debut_hors_ligne += 1;
            }
        }

        let mut fin = 0usize;
        let mut trou = 0usize;
        let mut recouvrement = 0usize;
        let mut couvert = 0usize;
        for (d, f) in &premier {
            if *d > fin { trou += d - fin; }
            if *d < fin { recouvrement += (fin - d).min(f.saturating_sub(*d)); }
            couvert += f.saturating_sub((*d).max(fin));
            fin = fin.max(*f);
        }
        if fin < contenu.len() { trou += contenu.len() - fin; }
        if trou > 0 { b.fichiers_avec_trou += 1; }
        if recouvrement > 0 { b.fichiers_avec_recouvrement += 1; }
        b.octets_trous += trou;
        b.octets_recouvrements += recouvrement;
        b.octets_couverts += couvert;
    }

    println!("## Ce que les parseurs couvrent aujourd'hui\n");
    println!("| ext | fich. | octets | couverts | trous | recouvr. | arbre err. | ast_valid faux | 0 scope | scopes 1er niv. | scopes | oct. ERROR |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    let mut t = Bilan::default();
    for (ext, b) in &bilans {
        let pct = if b.octets > 0 { 100.0 * b.octets_couverts as f64 / b.octets as f64 } else { 0.0 };
        println!("| {ext} | {} | {} | {:.1} % | {} ({}f) | {} ({}f) | {} | {} | {} | {} | {} | {} |",
            b.fichiers, b.octets, pct, b.octets_trous, b.fichiers_avec_trou,
            b.octets_recouvrements, b.fichiers_avec_recouvrement,
            b.fichiers_arbre_en_erreur, b.fichiers_ast_valid_faux, b.fichiers_zero_scope,
            b.scopes_premier_niveau, b.scopes_total,
            format!("{} ({:.0} %)", b.octets_erreur, 100.0 * b.octets_erreur as f64 / b.octets.max(1) as f64));
        t.fichiers += b.fichiers; t.octets += b.octets; t.octets_couverts += b.octets_couverts;
        t.octets_trous += b.octets_trous; t.octets_recouvrements += b.octets_recouvrements;
        t.fichiers_arbre_en_erreur += b.fichiers_arbre_en_erreur;
        t.fichiers_ast_valid_faux += b.fichiers_ast_valid_faux;
        t.fichiers_zero_scope += b.fichiers_zero_scope;
        t.fichiers_avec_trou += b.fichiers_avec_trou;
        t.fichiers_avec_recouvrement += b.fichiers_avec_recouvrement;
        t.scopes_premier_niveau += b.scopes_premier_niveau; t.scopes_total += b.scopes_total;
        t.octets_erreur += b.octets_erreur; t.debut_hors_ligne += b.debut_hors_ligne;
    }
    let pct = if t.octets > 0 { 100.0 * t.octets_couverts as f64 / t.octets as f64 } else { 0.0 };
    println!("| **total** | {} | {} | {:.1} % | {} ({}f) | {} ({}f) | {} | {} | {} | {} | {} | {} |",
        t.fichiers, t.octets, pct, t.octets_trous, t.fichiers_avec_trou,
        t.octets_recouvrements, t.fichiers_avec_recouvrement,
        t.fichiers_arbre_en_erreur, t.fichiers_ast_valid_faux, t.fichiers_zero_scope,
        t.scopes_premier_niveau, t.scopes_total,
        format!("{} ({:.0} %)", t.octets_erreur, 100.0 * t.octets_erreur as f64 / t.octets.max(1) as f64));
    println!("\nOctets sous un nœud ERROR (mesure tree-sitter directe) : {}", t.octets_erreur);

    println!("\n## Ce qui n'entre pas dans l'index\n");
    println!("| ext | fichiers | octets |");
    println!("|---|---:|---:|");
    let mut hf = 0usize; let mut ho = 0usize;
    let mut lignes: Vec<_> = hors_index.iter().collect();
    lignes.sort_by_key(|(_, (_, o))| std::cmp::Reverse(*o));
    for (ext, (f, o)) in lignes.iter().take(25) {
        println!("| {ext} | {f} | {o} |");
    }
    for (_, (f, o)) in hors_index.iter() { hf += f; ho += o; }
    println!("| **total** | {hf} | {ho} |");
    println!("\nDans l'index : {} fichiers, {} octets. Hors index : {hf} fichiers, {ho} octets ({:.0} % des octets).",
        t.fichiers, t.octets, 100.0 * ho as f64 / (ho + t.octets).max(1) as f64);
}
