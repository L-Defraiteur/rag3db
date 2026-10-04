//! **Un arbre de relations en texte** — la forme du « Dependency Graph » du
//! rendu des résultats (`templates/render/results.md.jinja`) et du « Graphe »
//! de `grep` : un titre, une branche par relation entre tildes, les voisins
//! dessous.
//!
//! ```text
//! titre
//! ├── ~ Consumes ~
//! │   ├── voisin (function) @ a.rs:12
//! │   └── voisin
//! └── ~ Consumed by ~
//!     └── voisin
//! ```
//!
//! Le gabarit jinja écrit la même forme de son côté ; `grep` et la section
//! Liens passent par ici.

/// **Le libellé d'une relation** dans un arbre : son nom en casse de phrase,
/// les tirets bas en espaces — `CONSUMED_BY` se lit `Consumed by`. Le gabarit
/// jinja écrit le même, par `replace("_", " ") | capitalize`.
pub fn relation_label(relation: &str) -> String {
    let mut chars = relation.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect::<String>().replace('_', " "),
        None => String::new(),
    }
}

/// L'arbre d'un titre et de ses groupes `(relation, voisins déjà formés)`.
pub fn arbre(titre: &str, groupes: &[(String, Vec<String>)]) -> String {
    let mut out = format!("{titre}\n");
    for (g, (relation, voisins)) in groupes.iter().enumerate() {
        let dernier = g + 1 == groupes.len();
        out.push_str(&format!("{}~ {} ~\n", if dernier { "└── " } else { "├── " }, relation_label(relation)));
        let tuyau = if dernier { "    " } else { "│   " };
        for (i, v) in voisins.iter().enumerate() {
            out.push_str(&format!("{tuyau}{}{v}\n", if i + 1 == voisins.len() { "└── " } else { "├── " }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{arbre, relation_label};

    #[test]
    fn la_forme_du_graphe_de_dependances() {
        let t = arbre("a", &[("CONSUMES".into(), vec!["b".into(), "c".into()]), ("CONSUMED_BY".into(), vec!["d".into()])]);
        assert_eq!(t, "a\n├── ~ Consumes ~\n│   ├── b\n│   └── c\n└── ~ Consumed by ~\n    └── d\n");
    }

    #[test]
    fn le_libelle_est_en_casse_de_phrase() {
        assert_eq!(relation_label("DEFINED_IN"), "Defined in");
        assert_eq!(relation_label("HAS_VARIANT"), "Has variant");
        assert_eq!(relation_label("CONSUMES"), "Consumes");
        assert_eq!(relation_label(""), "");
    }
}
