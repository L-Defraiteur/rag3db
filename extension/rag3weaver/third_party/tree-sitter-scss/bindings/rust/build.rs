fn main() {
    let src_dir = std::path::Path::new("src");

    let mut c_config = cc::Build::new();
    // `flag_if_supported` et non `flag` : MSVC (cl) refuse ce drapeau GCC
    // (« D8021 invalid numeric argument ») et c'était le seul obstacle au
    // bâti Windows de rag3weaver. Seule retouche à la crate publiée.
    c_config.flag_if_supported("-Wno-unused-parameter");
    c_config.std("c11").include(src_dir);

    let parser_path = src_dir.join("parser.c");
    c_config.file(&parser_path);
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    let scanner_path = src_dir.join("scanner.c");
    c_config.file(&scanner_path);
    println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());

    c_config.compile("tree-sitter-scss");
}
