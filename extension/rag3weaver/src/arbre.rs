//! **Un arbre de relations en texte** — la forme du « Dependency Graph » du
//! rendu des résultats (`templates/render/results.md.jinja`) et du « Graphe »
//! de `grep` : un titre, une branche par relation entre crochets, les voisins
//! dessous.
//!
//! ```text
//! titre
//! ├── [CONSUMES]
//! │   ├── voisin (function) @ a.rs:12
//! │   └── voisin
//! └── [CONSUMED_BY]
//!     └── voisin
//! ```
//!
//! Le gabarit jinja écrit la même forme de son côté ; `grep` et la section
//! Liens passent par ici.

/// L'arbre d'un titre et de ses groupes `(relation, voisins déjà formés)`.
pub fn arbre(titre: &str, groupes: &[(String, Vec<String>)]) -> String {
    let mut out = format!("{titre}\n");
    for (g, (relation, voisins)) in groupes.iter().enumerate() {
        let dernier = g + 1 == groupes.len();
        out.push_str(&format!("{}[{relation}]\n", if dernier { "└── " } else { "├── " }));
        let tuyau = if dernier { "    " } else { "│   " };
        for (i, v) in voisins.iter().enumerate() {
            out.push_str(&format!("{tuyau}{}{v}\n", if i + 1 == voisins.len() { "└── " } else { "├── " }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::arbre;

    #[test]
    fn la_forme_du_graphe_de_dependances() {
        let t = arbre("a", &[("CONSUMES".into(), vec!["b".into(), "c".into()]), ("CONSUMED_BY".into(), vec!["d".into()])]);
        assert_eq!(t, "a\n├── [CONSUMES]\n│   ├── b\n│   └── c\n└── [CONSUMED_BY]\n    └── d\n");
    }
}
