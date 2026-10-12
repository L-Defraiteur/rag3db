//! **Les tests par requêtes** (`workspace.queries_via`) : un test qui atteint
//! le code par une requête — une chaîne Cypher dans un test C++, un fichier
//! de cas `.test`, un `CALL fonction(…)` — et que le graphe d'appels ne voit
//! pas, puisqu'il s'arrête à l'analyse de la requête.
//!
//! Tout est déclaré par le dépôt, rien n'est en dur pour un langage :
//! - les **fichiers de test** par une expression sur leur chemin : leurs
//!   scopes `texte_brut` reçoivent la marque `suite` (certitude `declared`) ;
//! - les **noms enregistrés** par une expression lue dans le contenu des
//!   classes (`static constexpr const char* name = "STORAGE_INFO"`) :
//!   REGISTERS, de la classe vers le symbole du nom ;
//! - les **appels** et les **mots-clés** de la requête : QUERIES, du test vers
//!   le symbole du nom appelé (`storage_info(` → `STORAGE_INFO`), ou vers les
//!   symboles qu'un mot-clé déclaré désigne (`COPY` → `NodeBatchInsert`).
//!
//! `impact` relie ensuite un nœud atteint à ces tests par le chemin déclaré
//! au gabarit (`queried_by`) : le nœud ou sa classe, ce qu'ils définissent ou
//! enregistrent, et qui le demande.
//!
//! Ce module ne fait que le relevé, sur l'analyse d'un lot : une fonction
//! pure, sans catalogue.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use regex::Regex;

use crate::backend_code::QueriesVia;
use crate::code::ScopeRecord;

/// La valeur de `test_role` d'un fichier de test déclaré : un fichier de cas
/// en porte plusieurs.
pub const ROLE_SUITE: &str = "suite";
/// La certitude d'une marque posée par une déclaration du manifeste, ni par
/// la syntaxe (`certain`) ni par le seul nom (`convention`).
pub const CERTAINTY_DECLARED: &str = "declared";

/// Une déclaration compilée : ses expressions vérifiées une fois, au montage.
#[derive(Debug, Clone)]
pub struct CompiledQueriesVia {
    test_files: Option<Regex>,
    query_text: Regex,
    registered_name: Option<Regex>,
    registered_kinds: BTreeSet<String>,
    call_pattern: Regex,
    /// Le mot-clé (en majuscules), son expression, et les symboles visés.
    keywords: Vec<(String, Regex, Vec<String>)>,
}

impl CompiledQueriesVia {
    /// Compiler la déclaration ; une expression fautive est refusée en la
    /// nommant.
    pub fn compile(d: &QueriesVia) -> Result<Self, String> {
        let re = |nom: &str, motif: &str| Regex::new(motif).map_err(|e| format!("workspace.queries_via.{nom} : {e}"));
        let test_files = d.test_files.as_deref().map(|m| re("test_files", m)).transpose()?;
        let query_text = re("query_text", &d.query_text)?;
        let registered_name = d.registered_name.as_deref().map(|m| re("registered_name", m)).transpose()?;
        let call_pattern = re("call_pattern", &d.call_pattern)?;
        for (nom, r) in [("query_text", &query_text), ("call_pattern", &call_pattern)] {
            if r.captures_len() < 2 {
                return Err(format!("workspace.queries_via.{nom} : il faut un groupe de capture (le texte retenu)"));
            }
        }
        if registered_name.as_ref().is_some_and(|r| r.captures_len() < 2) {
            return Err("workspace.queries_via.registered_name : il faut un groupe de capture (le nom)".into());
        }
        let mut keywords = Vec::new();
        for (mot, symboles) in &d.keywords {
            let r = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(mot))).map_err(|e| format!("workspace.queries_via.keywords.{mot} : {e}"))?;
            keywords.push((mot.to_uppercase(), r, symboles.clone()));
        }
        Ok(Self {
            test_files,
            query_text,
            registered_name,
            registered_kinds: d.registered_kinds.iter().cloned().collect(),
            call_pattern,
            keywords,
        })
    }
}

/// Ce que le relevé rend pour un lot.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct QueryLinks {
    /// Les scopes à marquer : clé → `test_name` (le chemin du fichier).
    pub marks: HashMap<String, String>,
    /// QUERIES : (clé du scope de test, symbole, `via`).
    pub queries: Vec<(String, String, String)>,
    /// REGISTERS : (clé du scope qui enregistre, nom enregistré).
    pub registers: Vec<(String, String)>,
}

/// **Le relevé d'un lot** : les marques des fichiers de test déclarés, les
/// noms enregistrés, et ce que chaque test demande.
pub fn link_queries(c: &CompiledQueriesVia, scopes: &[ScopeRecord]) -> QueryLinks {
    let mut out = QueryLinks::default();
    let mut vus: BTreeSet<(String, String)> = BTreeSet::new();
    for s in scopes {
        // Les noms enregistrés : dans le contenu des genres déclarés.
        if let Some(r) = &c.registered_name {
            if c.registered_kinds.contains(&s.scope_type) {
                let noms: BTreeSet<String> = r.captures_iter(&s.content).filter_map(|m| m.get(1)).map(|m| m.as_str().to_uppercase()).collect();
                out.registers.extend(noms.into_iter().map(|n| (s.key.clone(), n)));
            }
        }
        // Le test : marqué par une grammaire, ou un fichier de test déclaré
        // (seulement un `texte_brut` sans marque : une marque de grammaire
        // l'emporte).
        let declare = s.scope_type == "texte_brut"
            && s.test_role.is_empty()
            && c.test_files.as_ref().is_some_and(|r| r.is_match(&s.file_path));
        if declare {
            out.marks.insert(s.key.clone(), s.file_path.clone());
        }
        if !declare && !matches!(s.test_role.as_str(), "case" | "suite") {
            continue;
        }
        // Le texte des requêtes : tout le fichier de cas, ou les chaînes du
        // test (`conn->query("…")`).
        let textes: Vec<&str> = if declare {
            vec![s.content.as_str()]
        } else {
            c.query_text.captures_iter(&s.content).filter_map(|m| m.get(1)).map(|m| m.as_str()).collect()
        };
        let mut ajouter = |symbole: String, via: String| {
            if vus.insert((s.key.clone(), symbole.clone())) {
                out.queries.push((s.key.clone(), symbole, via));
            }
        };
        for t in textes {
            if c.registered_name.is_some() {
                for m in c.call_pattern.captures_iter(t).filter_map(|m| m.get(1)) {
                    ajouter(m.as_str().to_uppercase(), "nom".to_string());
                }
            }
            for (mot, r, symboles) in &c.keywords {
                if r.is_match(t) {
                    for sym in symboles {
                        ajouter(sym.clone(), format!("mot:{mot}"));
                    }
                }
            }
        }
    }
    out.registers.sort();
    out.registers.dedup();
    out
}

/// Combien de mots-clés la table couvre, et lesquels : pour la page des
/// manques (ce qu'impact ne voit pas, c'est ce qui n'y est pas).
pub fn keyword_count(d: &QueriesVia) -> (usize, BTreeMap<String, usize>) {
    (d.keywords.len(), d.keywords.iter().map(|(k, v)| (k.clone(), v.len())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(key: &str, file: &str, kind: &str, role: &str, content: &str) -> ScopeRecord {
        ScopeRecord {
            key: key.into(),
            file_path: file.into(),
            scope_type: kind.into(),
            test_role: role.into(),
            content: content.into(),
            ..Default::default()
        }
    }

    fn declaration() -> QueriesVia {
        QueriesVia {
            test_files: Some(r"^test/test_files/.*\.test$".into()),
            registered_name: Some(r#"static constexpr const char\* name = "([A-Za-z_][A-Za-z0-9_]*)""#.into()),
            keywords: [("COPY".to_string(), vec!["NodeBatchInsert".to_string()]), ("SET".to_string(), vec!["SetNodeProperty".to_string()])]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    /// Un fichier de cas déclaré est marqué et lu en entier ; un test marqué
    /// n'est lu que dans ses chaînes ; une classe enregistre son nom ; un
    /// scope ordinaire ne demande rien, même s'il écrit `COPY`.
    #[test]
    fn le_releve_d_un_lot() {
        let c = CompiledQueriesVia::compile(&declaration()).unwrap();
        let scopes = vec![
            scope("cas", "test/test_files/set.test", "texte_brut", "", "-CASE A\n-STATEMENT MATCH (n:T) SET n.x = 1\n---- ok\n"),
            scope("autre", "docs/notes.txt", "texte_brut", "", "SET partout"),
            scope("gtest", "test/a_test.cpp", "function", "case", "TEST_F(A, B) {\n  conn->query(\"CALL storage_info('T') RETURN *\");\n  int copy = 1;\n}"),
            scope("info", "src/info.h", "struct", "", "struct StorageInfoFunction {\n  static constexpr const char* name = \"STORAGE_INFO\";\n};"),
            scope("code", "src/x.cpp", "function", "", "void f() { COPY(); }"),
        ];
        let l = link_queries(&c, &scopes);
        assert_eq!(l.marks, [("cas".to_string(), "test/test_files/set.test".to_string())].into_iter().collect());
        assert_eq!(l.registers, vec![("info".to_string(), "STORAGE_INFO".to_string())]);
        assert!(l.queries.contains(&("cas".into(), "SetNodeProperty".into(), "mot:SET".into())), "{:?}", l.queries);
        assert!(l.queries.contains(&("gtest".into(), "STORAGE_INFO".into(), "nom".into())), "{:?}", l.queries);
        assert!(!l.queries.iter().any(|(k, s, _)| k == "gtest" && s == "NodeBatchInsert"), "`int copy` hors chaîne : {:?}", l.queries);
        assert!(!l.queries.iter().any(|(k, _, _)| k == "autre" || k == "code" || k == "info"), "{:?}", l.queries);
    }

    /// Une expression sans groupe de capture est refusée en la nommant.
    #[test]
    fn une_expression_sans_groupe_est_refusee() {
        let d = QueriesVia { registered_name: Some("name = \"[A-Z]+\"".into()), ..declaration() };
        let e = CompiledQueriesVia::compile(&d).unwrap_err();
        assert!(e.contains("registered_name"), "{e}");
    }
}
