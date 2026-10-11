fn main() {
    println!("cargo:rerun-if-env-changed=RAG3DB_SHARED");
    compiled_against();
    // Native extensions resolve core C++ symbols from the executable when rag3db
    // is statically linked. Link arguments of dependency build scripts are not
    // propagated to this crate's executables (see tools/rust_api/build.rs).
    if std::env::var_os("CARGO_FEATURE_RAG3DB_NATIVE").is_some()
        && std::env::var_os("RAG3DB_SHARED").is_none()
        && matches!(
            std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
            Ok("linux" | "freebsd")
        )
    {
        println!("cargo:rustc-link-arg=-rdynamic");
    }
}

/// **Contre quel bâti du moteur ce crate est compilé** : l'identifiant de bâti
/// lu dans l'en-tête généré du bâti (`<RAG3DB_INCLUDE_DIR>/include/common/build_id.h`),
/// jamais dans git (un arbre sale, un autre dossier de bâti mentiraient). Il
/// part dans la section `.rag3db_compiled_against` des binaires
/// (`build_check`). Sans en-tête (le moteur rebâti dans la crate), rien.
fn compiled_against() {
    println!("cargo:rerun-if-env-changed=RAG3DB_INCLUDE_DIR");
    let Ok(include) = std::env::var("RAG3DB_INCLUDE_DIR") else { return };
    let header = std::path::Path::new(&include).join("include/common/build_id.h");
    println!("cargo:rerun-if-changed={}", header.display());
    let Ok(text) = std::fs::read_to_string(&header) else { return };
    let id = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("#define RAG3DB_BUILD_ID"))
        .and_then(|rest| rest.trim().strip_prefix('"'))
        .and_then(|rest| rest.split('"').next());
    if let Some(id) = id {
        println!("cargo:rustc-env=RAG3DB_COMPILED_AGAINST={id}");
    }
}
