fn main() {
    println!("cargo:rerun-if-env-changed=RAG3DB_SHARED");
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
