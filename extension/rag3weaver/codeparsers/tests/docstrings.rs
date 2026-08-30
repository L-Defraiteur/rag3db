//! **La documentation d'un scope, langage par langage.**
//!
//! Jusqu'au 29 août 2026 seul Rust remplissait `docstring` ; C, C++, C# et Go
//! rendaient `None` alors que le mécanisme existait et était générique. Un
//! champ vide ne se voit pas : la recherche perdait la seule phrase qui
//! explique ce qu'une fonction fait, et personne ne pouvait dire que c'était
//! un défaut plutôt qu'un corpus sans commentaires.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn docs(nom: &str, source: &str) -> HashMap<String, String> {
    let root = "/virtual";
    let chemin = format!("{root}/{nom}");
    let mut contenu = HashMap::new();
    contenu.insert(chemin.clone(), source.to_string());
    let resultat = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: root.to_string(),
            files: vec![chemin.clone()],
            content_map: Some(contenu),
            resolve_relationships: Some(false),
            resolver_options: None,
        });
    resultat
        .files
        .get(&chemin)
        .unwrap_or_else(|| panic!("{nom} n'a pas été parsé : {:?}", resultat.errors))
        .scopes
        .iter()
        .filter_map(|s| s.docstring.clone().map(|d| (s.name.clone(), d)))
        .collect()
}

#[test]
fn rust_documente_en_triple_barre() {
    let d = docs(
        "a.rs",
        "/// Rend la norme.\n/// Sur deux lignes.\n#[inline]\npub fn norm(x: i32) -> i32 { x.abs() }\n",
    );
    assert_eq!(d.get("norm").map(String::as_str), Some("Rend la norme.\nSur deux lignes."));
}

/// **Go n'a qu'une convention**, et c'est `//` collé à la déclaration.
#[test]
fn go_documente_en_double_barre() {
    let d = docs(
        "a.go",
        "package main\n\n// Norm rend la valeur absolue.\nfunc Norm(x int) int {\n\treturn x\n}\n",
    );
    assert_eq!(d.get("Norm").map(String::as_str), Some("Norm rend la valeur absolue."));
}

/// C# documente en `///` (XML) ; un `//` ordinaire n'est **pas** de la doc, et
/// le prendre pour telle remplirait le champ de notes de travail.
#[test]
fn csharp_documente_en_xml_et_pas_en_double_barre() {
    let d = docs(
        "a.cs",
        "class C {\n    /// <summary>Additionne.</summary>\n    public int Add(int a) { return a; }\n}\n",
    );
    assert_eq!(d.get("Add").map(String::as_str), Some("<summary>Additionne.</summary>"));

    let ordinaire = docs(
        "b.cs",
        "class C {\n    // note de travail, à refaire\n    public int Add(int a) { return a; }\n}\n",
    );
    assert_eq!(ordinaire.get("Add"), None, "un // en C# n'est pas de la documentation");
}

/// **C++ : les deux styles, dans le même fichier.** Le bloc Doxygen est la
/// convention canonique ; `//` est ce que la plupart des bases écrivent
/// vraiment — dont le moteur sur lequel celui-ci est bâti.
#[test]
fn cpp_attrape_le_bloc_doxygen_et_les_lignes() {
    let bloc = docs(
        "a.cpp",
        "/**\n * Additionne deux entiers.\n * Rien de plus.\n */\nint add(int a, int b) { return a + b; }\n",
    );
    assert_eq!(
        bloc.get("add").map(String::as_str),
        Some("Additionne deux entiers.\nRien de plus.")
    );

    let lignes = docs(
        "b.cpp",
        "// Additionne deux entiers.\nint add(int a, int b) { return a + b; }\n",
    );
    assert_eq!(lignes.get("add").map(String::as_str), Some("Additionne deux entiers."));
}

#[test]
fn c_attrape_les_deux_styles_aussi() {
    let bloc = docs(
        "a.c",
        "/*! Ouvre le fichier. */\nint ouvrir(const char* p) { return 0; }\n",
    );
    assert_eq!(bloc.get("ouvrir").map(String::as_str), Some("Ouvre le fichier."));

    let lignes = docs(
        "b.c",
        "/// Ouvre le fichier.\nint ouvrir(const char* p) { return 0; }\n",
    );
    assert_eq!(lignes.get("ouvrir").map(String::as_str), Some("Ouvre le fichier."));
}

/// **Un bloc ordinaire n'est pas de la documentation.** `/* … */` sans étoile
/// ni exclamation, c'est du code mis de côté ou une note d'implémentation ; le
/// mettre dans `docstring` mettrait du bruit dans l'index.
#[test]
fn un_bloc_ordinaire_n_est_pas_de_la_doc() {
    let d = docs(
        "a.cpp",
        "/* ancien prototype : int add(int); */\nint add(int a, int b) { return a + b; }\n",
    );
    assert_eq!(d.get("add"), None);
}

/// **Une ligne vide rompt le lien.** Une doc qui ne touche pas son élément
/// documente autre chose, ou plus rien.
#[test]
fn une_ligne_vide_rompt_le_lien() {
    let d = docs(
        "a.go",
        "package main\n\n// Ceci parle du fichier, pas de la fonction.\n\nfunc Norm(x int) int {\n\treturn x\n}\n",
    );
    assert_eq!(d.get("Norm"), None);
}
