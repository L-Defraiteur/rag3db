//! Private read-only diagnostic. Link only against the attested 927 rlib.
//! No Catalog, backend, connection, ingestion or source-content read is used.
use std::path::PathBuf;

fn hex(value: &str) -> String {
    value.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().expect("one source root required"));
    assert!(args.next().is_none(), "one source root only");
    assert!(root.is_absolute() && root.is_dir(), "existing absolute source root required");
    assert!(!root.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name == ".vault" || name.starts_with(".env")
    }), "secret root refused");
    let scanner = rag3weaver::code_tools::WorkingTree::new(&root);
    let (mut listed, mut excluded) = scanner.list_with_exclusions().expect("source listing failed");
    listed.sort();
    excluded.sort();
    // Hex UTF-8 fields keep tabs/newlines in file names unambiguous, without
    // another dependency. This is a listing, not manifest ingestion approval.
    println!("source-admission-v1\t{}", hex(&root.to_string_lossy()));
    for path in &listed {
        println!("listed\t{}", hex(path));
    }
    for (path, reason) in &excluded {
        println!("excluded\t{}\t{}", hex(path), hex(reason));
    }
    println!("counts\t{}\t{}", listed.len(), excluded.len());
}
