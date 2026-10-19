fn main() {
    println!("cargo:rerun-if-env-changed=RAG3DB_SHARED");
    compiled_against();
    builtin_templates();
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

/// **Les gabarits livrés, embarqués dans le binaire** (`src/builtin_templates.rs`) :
/// chaque fichier de `templates/`, par son chemin relatif à `templates/`. Un
/// manifeste les désigne par `builtin:<chemin>` ; ils suivent donc chaque bâti.
fn builtin_templates() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    println!("cargo:rerun-if-changed={}", racine.display());
    fn parcourir(dossier: &std::path::Path, racine: &std::path::Path, out: &mut Vec<(String, std::path::PathBuf)>) {
        let Ok(entrees) = std::fs::read_dir(dossier) else { return };
        for e in entrees.flatten() {
            let p = e.path();
            if p.is_dir() {
                parcourir(&p, racine, out);
            } else if p.is_file() {
                let rel = p.strip_prefix(racine).unwrap().components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
                out.push((rel, p));
            }
        }
    }
    let mut fichiers = Vec::new();
    parcourir(&racine, &racine, &mut fichiers);
    fichiers.sort();
    let mut code = String::from("pub static BUILTIN: &[(&str, &[u8])] = &[\n");
    for (rel, p) in &fichiers {
        code.push_str(&format!("    ({rel:?}, include_bytes!({:?})),\n", p.display().to_string()));
    }
    code.push_str("];\n");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("builtin_templates.rs");
    std::fs::write(out, code).unwrap();
}
