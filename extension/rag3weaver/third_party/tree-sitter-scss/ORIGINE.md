# tree-sitter-scss 1.0.0, copie locale

Copie de la crate `tree-sitter-scss` 1.0.0 publiée sur crates.io (licence MIT,
dépôt https://github.com/tree-sitter-grammars/tree-sitter-scss), tirée par
codeparsers. Une seule retouche, dans `bindings/rust/build.rs` :
`flag_if_supported("-Wno-unused-parameter")` au lieu de `flag(...)`, parce que
MSVC refuse ce drapeau GCC et que c'était le seul obstacle au bâti Windows de
rag3weaver (10 octobre 2026). Branchée par `[patch.crates-io]` dans le
`Cargo.toml` de rag3weaver ; à retirer le jour où une version publiée le fait.
