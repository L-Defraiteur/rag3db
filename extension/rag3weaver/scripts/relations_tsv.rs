//! Écrit les relations résolues d'un ensemble de fichiers, une par ligne (TSV),
//! pour comparer deux révisions de l'analyseur.
//!
//! ```text
//! (copié dans examples/ d'un arbre de codeparsers par scripts/aretes_retirees.sh)
//! cargo run --release --example relations_tsv -- [--rag3weaver] <racine> <fichier>...
//! ```
//!
//! `--rag3weaver` prend les options que rag3weaver passe au résolveur (ni
//! références de niveau fichier, ni références des enfants).
//!
//! Colonnes : type, uuid source, uuid cible, nom source, nom cible, fichier
//! source, sites (`Genre@ligne`, séparés par des virgules).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use std::collections::HashMap;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let options_rag3weaver = args.first().is_some_and(|a| a == "--rag3weaver");
    if options_rag3weaver {
        args.remove(0);
    }
    let Some(racine) = args.first().cloned() else {
        eprintln!("usage : relations_tsv [--rag3weaver] <racine> <fichier>...");
        std::process::exit(2);
    };
    let fichiers: Vec<String> = args[1..].to_vec();
    let contenus: HashMap<String, String> = fichiers
        .iter()
        .map(|f| (f.clone(), std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{f} : {e}"))))
        .collect();
    let analyse = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: racine,
        files: fichiers,
        content_map: Some(contenus),
        resolve_relationships: Some(true),
        resolver_options: options_rag3weaver.then(|| RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            ..Default::default()
        }),
    });
    let Some(resolution) = analyse.relationships else { return };
    for r in &resolution.relationships {
        let sites = r
            .metadata
            .as_ref()
            .map(|m| m.sites.iter().map(|s| format!("{:?}@{}", s.usage, s.line.unwrap_or(0))).collect::<Vec<_>>().join(","))
            .unwrap_or_default();
        println!("{:?}\t{}\t{}\t{}\t{}\t{}\t{}", r.r#type, r.from_uuid, r.to_uuid, r.from_name, r.to_name, r.from_file, sites);
    }
}
