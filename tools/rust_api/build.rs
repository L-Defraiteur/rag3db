use std::env;
use std::path::{Path, PathBuf};

fn link_mode() -> &'static str {
    if env::var("RAG3DB_SHARED").is_ok() {
        "dylib"
    } else {
        "static"
    }
}

fn get_target() -> String {
    env::var("PROFILE").unwrap()
}

fn link_libraries() {
    // This also needs to be set by any crates using it if they want to use extensions
    if !cfg!(windows) && link_mode() == "static" {
        println!("cargo:rustc-link-arg=-rdynamic");
    }
    if cfg!(windows) && link_mode() == "dylib" {
        println!("cargo:rustc-link-lib=dylib=rag3db_shared");
    } else if link_mode() == "dylib" {
        println!("cargo:rustc-link-lib={}=rag3db", link_mode());
    } else if rustversion::cfg!(since(1.82)) {
        println!("cargo:rustc-link-lib=static:+whole-archive=rag3db");
    } else {
        println!("cargo:rustc-link-lib=static=rag3db");
    }
    if link_mode() == "static" {
        if cfg!(windows) {
            println!("cargo:rustc-link-lib=dylib=msvcrt");
            println!("cargo:rustc-link-lib=dylib=shell32");
            println!("cargo:rustc-link-lib=dylib=ole32");
        } else if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=dylib=c++");
        } else {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }

        for lib in [
            "utf8proc",
            "antlr4_cypher",
            "antlr4_runtime",
            "re2",
            "fastpfor",
            "parquet",
            "thrift",
            "snappy",
            "zstd",
            "miniz",
            "mbedtls",
            "brotlidec",
            "brotlicommon",
            "lz4",
            "roaring_bitmap",
            "simsimd",
        ] {
            if rustversion::cfg!(since(1.82)) {
                println!("cargo:rustc-link-lib=static:+whole-archive={lib}");
            } else {
                println!("cargo:rustc-link-lib=static={lib}");
            }
        }
    }
}

fn build_bundled_cmake() -> Vec<PathBuf> {
    let rag3db_root = {
        let root = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("rag3db-src");
        if root.is_symlink() || root.is_dir() {
            root
        } else {
            // If the path is not directory, this is probably an in-source build on windows where the
            // symlink is unreadable.
            Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..")
        }
    };

    let mut build = cmake::Config::new(&rag3db_root);
    build
        .no_build_target(true)
        .define("BUILD_SHELL", "OFF")
        .define("BUILD_SINGLE_FILE_HEADER", "OFF")
        .define("AUTO_UPDATE_GRAMMAR", "OFF");
    // GCC 13+ a resserré les includes transitifs : <memory>, <string> etc. ne
    // tirent plus <cstdint>. Le cœur kuzu s'appuie dessus dans 613 fichiers, qui
    // utilisent uint32_t & co sans l'inclure — d'où « 'uint32_t' does not name a
    // type » sur toute chaîne récente.
    //
    // On force l'include plutôt que d'éditer 613 fichiers d'amont : c'est un
    // problème de chaîne de compilation, il se règle au niveau de la chaîne. Sans
    // effet sur les compilateurs plus anciens, qui l'incluaient déjà.
    if !cfg!(windows) {
        build.cxxflag("-include cstdint");
    }

    if cfg!(windows) {
        build.generator("Ninja");
        build.cxxflag("/EHsc");
        build.define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreadedDLL");
        build.define("CMAKE_POLICY_DEFAULT_CMP0091", "NEW");
    }
    if let Ok(jobs) = std::env::var("NUM_JOBS") {
        std::env::set_var("CMAKE_BUILD_PARALLEL_LEVEL", jobs);
    }
    let build_dir = build.build();

    let rag3db_lib_path = build_dir.join("build").join("src");
    println!("cargo:rustc-link-search=native={}", rag3db_lib_path.display());

    for dir in [
        "utf8proc",
        "antlr4_cypher",
        "antlr4_runtime",
        "re2",
        "brotli",
        "alp",
        "fastpfor",
        "parquet",
        "thrift",
        "snappy",
        "zstd",
        "miniz",
        "mbedtls",
        "lz4",
        "roaring_bitmap",
        "simsimd",
    ] {
        let lib_path = build_dir
            .join("build")
            .join("third_party")
            .join(dir)
            .canonicalize()
            .unwrap_or_else(|_| {
                panic!(
                    "Could not find {}/build/third_party/{}",
                    build_dir.display(),
                    dir
                )
            });
        println!("cargo:rustc-link-search=native={}", lib_path.display());
    }

    vec![
        rag3db_root.join("src/include"),
        build_dir.join("build/src"),
        build_dir.join("build/src/include"),
        rag3db_root.join("third_party/nlohmann_json"),
        rag3db_root.join("third_party/fastpfor"),
        rag3db_root.join("third_party/alp/include"),
    ]
}

fn build_ffi(
    bridge_file: &str,
    out_name: &str,
    source_file: &str,
    bundled: bool,
    include_paths: &Vec<PathBuf>,
) {
    let mut build = cxx_build::bridge(bridge_file);
    build.file(source_file);

    if bundled {
        build.define("RAG3DB_BUNDLED", None);
    }
    if get_target() == "debug" || get_target() == "relwithdebinfo" {
        build.define("ENABLE_RUNTIME_CHECKS", "1");
    }
    if link_mode() == "static" {
        build.define("RAG3DB_STATIC_DEFINE", None);
    }

    build.includes(include_paths);

    println!("cargo:rerun-if-env-changed=RAG3DB_SHARED");

    println!("cargo:rerun-if-changed=include/rag3db_rs.h");
    println!("cargo:rerun-if-changed=src/rag3db_rs.cpp");
    // Note that this should match the rag3db-src/* entries in the package.include list in Cargo.toml
    // Unfortunately they appear to need to be specified individually since the symlink is
    // considered to be changed each time.
    println!("cargo:rerun-if-changed=rag3db-src/src");
    println!("cargo:rerun-if-changed=rag3db-src/cmake");
    println!("cargo:rerun-if-changed=rag3db-src/third_party");
    println!("cargo:rerun-if-changed=rag3db-src/CMakeLists.txt");
    println!("cargo:rerun-if-changed=rag3db-src/tools/CMakeLists.txt");

    if cfg!(windows) {
        build.flag("/std:c++20");
        build.flag("/MD");
    } else {
        build.flag("-std=c++2a");
    }
    build.compile(out_name);
}

fn main() {
    if env::var("DOCS_RS").is_ok() {
        // Do nothing; we're just building docs and don't need the C++ library
        return;
    }

    let mut bundled = false;
    let mut include_paths =
        vec![Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("include")];

    if let (Ok(rag3db_lib_dir), Ok(rag3db_include)) =
        (env::var("RAG3DB_LIBRARY_DIR"), env::var("RAG3DB_INCLUDE_DIR"))
    {
        println!("cargo:rustc-link-search=native={rag3db_lib_dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{rag3db_lib_dir}");
        include_paths.push(Path::new(&rag3db_include).to_path_buf());
    } else {
        include_paths.extend(build_bundled_cmake());
        bundled = true;
    }
    if link_mode() == "static" {
        link_libraries();
    }
    build_ffi(
        "src/ffi.rs",
        "rag3db_rs",
        "src/rag3db_rs.cpp",
        bundled,
        &include_paths,
    );

    if cfg!(feature = "arrow") {
        build_ffi(
            "src/ffi/arrow.rs",
            "rag3db_arrow_rs",
            "src/rag3db_arrow.cpp",
            bundled,
            &include_paths,
        );
    }
    if link_mode() == "dylib" {
        link_libraries();
    }
}
