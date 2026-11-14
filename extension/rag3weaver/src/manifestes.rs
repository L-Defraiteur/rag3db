//! **Les manifestes d'un dépôt, pour la provenance des imports** (14 novembre 2026, second lot de la
//! provenance) : les dépendances qu'ils déclarent et les paquets du dépôt lui-même (les crates d'un
//! workspace Cargo, les paquets d'un workspace npm). codeparsers en tire la provenance d'un nom importé
//! (`declared_dependencies` → `dependency`, `project_packages` → `project`) ; rien n'est deviné : un nom
//! que les manifestes ne disent pas reste `inconnue`.
//!
//! **Par langue** (14 novembre 2026) : chaque manifeste range ses noms sous la langue qu'il décrit
//! (`Cargo.*` → rust, `package.json` → javascript, `pyproject.toml` et `requirements*.txt` → python,
//! `vcpkg.json` → c et cpp, `go.mod` → go) ; codeparsers les normalise par les variantes de cette langue
//! (`declared_dependencies_by_language`), et un chemin de module Go garde sa casse.
//!
//! Lus : `Cargo.toml`, `Cargo.lock`, `package.json`, `pyproject.toml`, `requirements*.txt`,
//! `vcpkg.json`, `go.mod`, n'importe où dans la liste des fichiers de la source (un dépôt a souvent plusieurs
//! manifestes). Sans parseur TOML, comme `origin.rs` : des sections et des clés, ligne à ligne.

use std::collections::{BTreeMap, BTreeSet};

/// Ce que les manifestes d'un dépôt disent.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifestes {
    /// Les chemins des manifestes lus, tels que la source les liste (dits dans l'état d'index).
    pub lus: Vec<String>,
    /// Les noms des dépendances déclarées (hors paquets du dépôt), tels qu'écrits.
    pub dependances: BTreeSet<String>,
    /// Les noms des paquets du dépôt lui-même.
    pub paquets_du_projet: BTreeSet<String>,
    /// Les dépendances déclarées, par langue (la clé est le `langue` des règles d'import de codeparsers).
    #[serde(default)]
    pub dependances_par_langue: BTreeMap<String, BTreeSet<String>>,
    /// Les paquets du dépôt, par langue.
    #[serde(default)]
    pub paquets_par_langue: BTreeMap<String, BTreeSet<String>>,
    /// Les `replace` de `go.mod` (`ancien => nouveau`), notés sans être suivis.
    #[serde(default)]
    pub remplacements_go: BTreeSet<String>,
}

/// Un nom de fichier de manifeste reconnu.
pub fn est_un_manifeste(chemin: &str) -> bool {
    let nom = chemin.rsplit('/').next().unwrap_or(chemin);
    matches!(nom, "Cargo.toml" | "Cargo.lock" | "package.json" | "pyproject.toml" | "vcpkg.json" | "go.mod")
        || (nom.starts_with("requirements") && nom.ends_with(".txt"))
}

impl Manifestes {
    /// Lit les manifestes parmi `chemins` ; `lire` rend le texte d'un chemin (la source).
    pub fn lire(chemins: &[String], mut lire: impl FnMut(&str) -> Option<String>) -> Manifestes {
        let mut m = Manifestes::default();
        let mut tries: Vec<&String> = chemins.iter().filter(|c| est_un_manifeste(c)).collect();
        tries.sort();
        for chemin in tries {
            let Some(texte) = lire(chemin) else { continue };
            let nom = chemin.rsplit('/').next().unwrap_or(chemin);
            // Ce que ce manifeste déclare, rangé ensuite sous sa ou ses langues.
            let mut un = Manifestes::default();
            let langues: &[&str] = match nom {
                "Cargo.toml" => { un.lire_cargo_toml(&texte); &["rust"] }
                "Cargo.lock" => { un.dependances.extend(noms_de_section(&texte, |s| s == "[[package]]")); &["rust"] }
                "package.json" => { un.lire_package_json(&texte); &["javascript"] }
                "pyproject.toml" => { un.lire_pyproject(&texte); &["python"] }
                "vcpkg.json" => { un.lire_vcpkg(&texte); &["c", "cpp"] }
                "go.mod" => { un.lire_go_mod(&texte); &["go"] }
                _ => { un.dependances.extend(texte.lines().filter_map(nom_pep508)); &["python"] }
            };
            for langue in langues {
                m.dependances_par_langue.entry(langue.to_string()).or_default().extend(un.dependances.iter().cloned());
                m.paquets_par_langue.entry(langue.to_string()).or_default().extend(un.paquets_du_projet.iter().cloned());
            }
            m.dependances.extend(un.dependances);
            m.paquets_du_projet.extend(un.paquets_du_projet);
            m.remplacements_go.extend(un.remplacements_go);
            m.lus.push(chemin.clone());
        }
        // Une crate du workspace figure aussi dans le Cargo.lock : elle est du projet, pas une dépendance.
        let projet: BTreeSet<String> = m.paquets_du_projet.iter().map(|n| normaliser(n)).collect();
        m.dependances.retain(|d| !projet.contains(&normaliser(d)));
        for (langue, deps) in m.dependances_par_langue.iter_mut() {
            if let Some(paquets) = m.paquets_par_langue.get(langue) {
                // Go compare tel quel (sa casse compte) ; les autres langues, minuscules et tirets confondus.
                let p: BTreeSet<String> = paquets.iter().map(|n| if langue == "go" { n.clone() } else { normaliser(n) }).collect();
                deps.retain(|d| !p.contains(&if langue == "go" { d.clone() } else { normaliser(d) }));
            }
        }
        m
    }

    /// `go.mod` : `module X` est du projet ; `require X v` (sur une ligne ou dans un bloc `require ( … )`)
    /// est une dépendance ; `replace A => B` se note sans être suivi ; `exclude`, `retract` et les autres
    /// directives ne disent rien d'un import.
    fn lire_go_mod(&mut self, texte: &str) {
        let mut bloc: Option<String> = None;
        for ligne in texte.lines() {
            let l = ligne.split("//").next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }
            if let Some(directive) = &bloc {
                if l == ")" {
                    bloc = None;
                    continue;
                }
                let directive = directive.clone();
                self.directive_go(&directive, l);
                continue;
            }
            let (directive, reste) = l.split_once(char::is_whitespace).map_or((l, ""), |(d, r)| (d, r.trim()));
            if reste == "(" {
                bloc = Some(directive.to_string());
            } else {
                self.directive_go(directive, reste);
            }
        }
    }

    fn directive_go(&mut self, directive: &str, argument: &str) {
        let premier = argument.split_whitespace().next().unwrap_or("").trim_matches('"');
        if premier.is_empty() {
            return;
        }
        match directive {
            "module" => {
                self.paquets_du_projet.insert(premier.to_string());
            }
            "require" => {
                self.dependances.insert(premier.to_string());
            }
            "replace" => {
                self.remplacements_go.insert(argument.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            _ => {}
        }
    }

    fn lire_cargo_toml(&mut self, texte: &str) {
        self.paquets_du_projet.extend(noms_de_section(texte, |s| s == "[package]"));
        let mut section = String::new();
        for ligne in texte.lines() {
            let l = ligne.trim();
            if l.starts_with('[') {
                section = l.to_string();
                // `[dependencies.serde]` : la dépendance est dans le nom de la section.
                for prefixe in ["[dependencies.", "[dev-dependencies.", "[build-dependencies.", "[workspace.dependencies."] {
                    if let Some(n) = l.strip_prefix(prefixe).and_then(|r| r.strip_suffix(']')) {
                        self.dependances.insert(n.trim_matches('"').to_string());
                    }
                }
                continue;
            }
            let de_dependances = matches!(section.as_str(), "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]" | "[workspace.dependencies]")
                || (section.starts_with("[target.") && section.ends_with("dependencies]"));
            if de_dependances {
                if let Some(cle) = cle_toml(l) {
                    // `serde.workspace = true` : la clé est `serde`.
                    self.dependances.insert(cle.split('.').next().unwrap_or(&cle).to_string());
                }
            }
        }
    }

    fn lire_package_json(&mut self, texte: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(texte) else { return };
        if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
            self.paquets_du_projet.insert(n.to_string());
        }
        for champ in ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"] {
            if let Some(o) = v.get(champ).and_then(|o| o.as_object()) {
                self.dependances.extend(o.keys().cloned());
            }
        }
    }

    fn lire_pyproject(&mut self, texte: &str) {
        self.paquets_du_projet.extend(noms_de_section(texte, |s| s == "[project]" || s == "[tool.poetry]"));
        let mut section = String::new();
        let mut dans_une_liste = false;
        for ligne in texte.lines() {
            let l = ligne.trim();
            if l.starts_with('[') && !dans_une_liste {
                section = l.to_string();
                continue;
            }
            // `[project] dependencies = ["requests>=2", …]`, sur une ou plusieurs lignes ;
            // `[project.optional-dependencies] groupe = [ … ]`.
            let liste_ouverte = (section == "[project]" && cle_toml(l).as_deref() == Some("dependencies"))
                || (section == "[project.optional-dependencies]" && cle_toml(l).is_some());
            if liste_ouverte || dans_une_liste {
                let fragment = if liste_ouverte { l.split_once('=').map(|(_, v)| v).unwrap_or("") } else { l };
                for chaine in fragment.split(',') {
                    let c = chaine.trim().trim_start_matches('[').trim_end_matches(']').trim().trim_matches(|ch| ch == '"' || ch == '\'');
                    if let Some(n) = nom_pep508(c) {
                        self.dependances.insert(n);
                    }
                }
                // La liste se ferme sur un `]` seul ou juste après une chaîne (`"x"]`) — pas sur le `]` des
                // extras d'une exigence (`"requests[socks]>=2.0",`).
                let fin = fragment.trim().trim_end_matches(',').trim_end();
                let fermee = fin == "]" || fin.ends_with("\"]") || fin.ends_with("']");
                dans_une_liste = !fermee;
                continue;
            }
            // Poetry : `[tool.poetry.dependencies] requests = "^2"` ; `python` n'est pas un paquet.
            if section == "[tool.poetry.dependencies]" || section == "[tool.poetry.dev-dependencies]" || (section.starts_with("[tool.poetry.group.") && section.ends_with(".dependencies]")) {
                if let Some(cle) = cle_toml(l).filter(|c| c != "python") {
                    self.dependances.insert(cle);
                }
            }
        }
    }

    fn lire_vcpkg(&mut self, texte: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(texte) else { return };
        if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
            self.paquets_du_projet.insert(n.to_string());
        }
        for d in v.get("dependencies").and_then(|d| d.as_array()).into_iter().flatten() {
            if let Some(n) = d.as_str().or_else(|| d.get("name").and_then(|n| n.as_str())) {
                self.dependances.insert(n.to_string());
            }
        }
    }
}

/// Les valeurs `name = "…"` des sections que `garder` retient.
fn noms_de_section(texte: &str, garder: impl Fn(&str) -> bool) -> Vec<String> {
    let mut dedans = false;
    let mut noms = Vec::new();
    for ligne in texte.lines() {
        let l = ligne.trim();
        if l.starts_with('[') {
            dedans = garder(l);
            continue;
        }
        if dedans && cle_toml(l).as_deref() == Some("name") {
            if let Some((_, v)) = l.split_once('=') {
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                if !v.is_empty() {
                    noms.push(v.to_string());
                }
            }
        }
    }
    noms
}

/// La clé d'une ligne TOML `cle = valeur` (guillemets ôtés), ou `None` (commentaire, vide, suite).
fn cle_toml(l: &str) -> Option<String> {
    if l.is_empty() || l.starts_with('#') {
        return None;
    }
    let (cle, _) = l.split_once('=')?;
    let cle = cle.trim().trim_matches('"');
    (!cle.is_empty() && cle.chars().all(|c| c.is_alphanumeric() || "-_.@/".contains(c))).then(|| cle.to_string())
}

/// Le nom d'une exigence PEP 508 (`requests[socks]>=2.0 ; python_version<"3.8"` → `requests`) ou d'une
/// ligne de requirements.txt ; `None` pour un commentaire, une option (`-r`, `-e`) ou une URL.
fn nom_pep508(ligne: &str) -> Option<String> {
    let l = ligne.trim();
    if l.is_empty() || l.starts_with('#') || l.starts_with('-') || l.contains("://") {
        return None;
    }
    let fin = l.find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.')).unwrap_or(l.len());
    let nom = &l[..fin];
    (!nom.is_empty()).then(|| nom.to_string())
}

/// La même normalisation que codeparsers (`import_resolution::provenance::normaliser`).
fn normaliser(nom: &str) -> String {
    nom.trim().to_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lire(fichiers: &[(&str, &str)]) -> Manifestes {
        let chemins: Vec<String> = fichiers.iter().map(|(c, _)| c.to_string()).collect();
        Manifestes::lire(&chemins, |c| fichiers.iter().find(|(p, _)| *p == c).map(|(_, t)| t.to_string()))
    }

    #[test]
    fn un_workspace_cargo_et_son_cargo_lock() {
        let m = lire(&[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/ma-crate\"]\n\n[workspace.dependencies]\nserde = \"1\"\n"),
            ("crates/ma-crate/Cargo.toml", "[package]\nname = \"ma-crate\"\n\n[dependencies]\nserde.workspace = true\nserde_json = { version = \"1\" }\n\n[dependencies.anyhow]\nversion = \"1\"\n\n[dev-dependencies]\ntempfile = \"3\"\n"),
            ("Cargo.lock", "[[package]]\nname = \"ma-crate\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"itoa\"\nversion = \"1.0.0\"\n"),
            ("src/lib.rs", "pub fn f() {}"),
        ]);
        assert_eq!(m.lus, vec!["Cargo.lock", "Cargo.toml", "crates/ma-crate/Cargo.toml"]);
        assert_eq!(m.paquets_du_projet, BTreeSet::from(["ma-crate".to_string()]));
        // Le Cargo.lock donne aussi les dépendances transitives (itoa) ; la crate du workspace n'en est pas une.
        assert_eq!(m.dependances, BTreeSet::from(["anyhow", "itoa", "serde", "serde_json", "tempfile"].map(String::from)));
    }

    #[test]
    fn un_package_json_et_un_workspace_npm() {
        let m = lire(&[
            ("package.json", r#"{"name": "racine", "workspaces": ["paquets/*"], "dependencies": {"lodash": "^4"}, "devDependencies": {"@types/node": "^20"}}"#),
            ("paquets/outils/package.json", r#"{"name": "@moi/outils", "peerDependencies": {"react": "*"}}"#),
        ]);
        assert_eq!(m.paquets_du_projet, BTreeSet::from(["racine", "@moi/outils"].map(String::from)));
        assert_eq!(m.dependances, BTreeSet::from(["lodash", "@types/node", "react"].map(String::from)));
    }

    #[test]
    fn go_mod_module_require_en_bloc_et_en_ligne() {
        let m = lire(&[(
            "go.mod",
            "module example.com/moi/Outil\n\ngo 1.23\n\nrequire github.com/spf13/cobra v1.8.0\n\nrequire (\n\tgithub.com/Azure/azure-sdk-for-go/sdk/azcore v1.11.0\n\tgolang.org/x/sync v0.7.0 // indirect\n)\n\nreplace github.com/spf13/cobra => ../cobra\n\nexclude golang.org/x/net v0.1.0\n",
        )]);
        assert_eq!(m.lus, vec!["go.mod"]);
        let go = &m.dependances_par_langue["go"];
        assert_eq!(go.iter().map(String::as_str).collect::<Vec<_>>(), vec!["github.com/Azure/azure-sdk-for-go/sdk/azcore", "github.com/spf13/cobra", "golang.org/x/sync"]);
        assert_eq!(m.paquets_par_langue["go"].iter().map(String::as_str).collect::<Vec<_>>(), vec!["example.com/moi/Outil"], "la casse du module est gardée");
        assert_eq!(m.remplacements_go.iter().map(String::as_str).collect::<Vec<_>>(), vec!["github.com/spf13/cobra => ../cobra"]);
        assert!(!go.contains("golang.org/x/net"), "exclude n'est pas une dépendance");
    }

    #[test]
    fn chaque_manifeste_range_sous_sa_langue() {
        let m = lire(&[
            ("Cargo.lock", "[[package]]\nname = \"serde-json\"\n"),
            ("package.json", r#"{"name": "app", "dependencies": {"lodash": "^4"}}"#),
            ("requirements.txt", "requests==2\n"),
            ("vcpkg.json", r#"{"dependencies": ["fmt"]}"#),
        ]);
        assert!(m.dependances_par_langue["rust"].contains("serde-json"));
        assert!(m.dependances_par_langue["javascript"].contains("lodash"));
        assert!(m.dependances_par_langue["python"].contains("requests"));
        assert!(m.dependances_par_langue["c"].contains("fmt") && m.dependances_par_langue["cpp"].contains("fmt"));
        assert!(!m.dependances_par_langue["python"].contains("lodash"), "un paquet npm n'est pas une dépendance Python");
        assert!(m.paquets_par_langue["javascript"].contains("app"));
    }

    /// De bout en bout, par l'analyse : un import se compare aux manifestes de **sa** langue. Un nom que
    /// seul `package.json` déclare ne fait pas d'un import Rust une dépendance (les tables de toutes les
    /// langues le faisaient) ; le même nom dans le Cargo.lock, si. Le cas Go :
    /// `go_l_analyse_garde_la_casse_du_module_requis`.
    #[cfg(feature = "code")]
    #[test]
    fn l_analyse_compare_aux_manifestes_de_la_langue() {
        let source = "use serde_json::Value;\n\npub fn lire() -> Value {\n    Value::Null\n}\n";
        let provenance = |m: &Manifestes| {
            let a = crate::code::analyze_in_project_avec("/virtual", vec![("lib.rs".to_string(), source.to_string())], "", None, Some(m));
            a.libraries.iter().find(|l| l.name == "serde_json").map(|l| l.provenance.clone())
        };
        let npm = lire(&[("package.json", r#"{"name": "app", "dependencies": {"serde_json": "1"}}"#)]);
        assert_eq!(provenance(&npm), Some(Some("inconnue".to_string())), "un paquet npm n'est pas une crate");
        let cargo = lire(&[("Cargo.lock", "[[package]]\nname = \"serde-json\"\n")]);
        assert_eq!(provenance(&cargo), Some(Some("dependency".to_string())), "la crate du Cargo.lock, tiret compris");
    }

    /// De bout en bout, par l'analyse : un module Go requis par `go.mod` sous sa casse est `dependency`,
    /// le même chemin sous une autre casse est `inconnue`, et `fmt` est builtin.
    #[cfg(feature = "code")]
    #[test]
    fn go_l_analyse_garde_la_casse_du_module_requis() {
        let m = lire(&[("go.mod", "module example.com/moi/outil\n\nrequire github.com/Azure/azure-sdk-for-go/sdk/azcore v1.11.0\n")]);
        let source = "package main\n\nimport (\n\t\"fmt\"\n\t\"github.com/Azure/azure-sdk-for-go/sdk/azcore/policy\"\n\taz \"github.com/azure/azure-sdk-for-go/sdk/azcore\"\n)\n\nfunc lancer() {\n\tfmt.Println(\"a\")\n\tpolicy.Lancer()\n\taz.Autre()\n}\n";
        let a = crate::code::analyze_in_project_avec("/virtual", vec![("main.go".to_string(), source.to_string())], "", None, Some(&m));
        let provenance = |nom: &str| a.libraries.iter().find(|l| l.name == nom).map(|l| l.provenance.clone());
        assert_eq!(provenance("fmt"), Some(Some("builtin".to_string())), "{:?}", a.libraries);
        assert_eq!(provenance("github.com/Azure/azure-sdk-for-go/sdk/azcore/policy"), Some(Some("dependency".to_string())), "{:?}", a.libraries);
        assert_eq!(provenance("github.com/azure/azure-sdk-for-go/sdk/azcore"), Some(Some("inconnue".to_string())), "{:?}", a.libraries);
    }

    #[test]
    fn pyproject_poetry_requirements_et_vcpkg() {
        let m = lire(&[
            ("pyproject.toml", "[project]\nname = \"mon-outil\"\ndependencies = [\n  \"requests[socks]>=2.0\",\n  'PyYAML ; python_version > \"3.8\"',\n]\n\n[project.optional-dependencies]\ndev = [\"pytest\"]\n"),
            ("outils/pyproject.toml", "[tool.poetry]\nname = \"autre\"\n\n[tool.poetry.dependencies]\npython = \"^3.11\"\nnumpy = \"^1\"\n"),
            ("requirements-dev.txt", "# commentaire\n-r requirements.txt\nblack==24.1\nhttps://exemple/paquet.tar.gz\n"),
            ("vcpkg.json", r#"{"name": "moteur", "dependencies": ["fmt", {"name": "openssl", "features": ["tools"]}]}"#),
        ]);
        assert_eq!(m.paquets_du_projet, BTreeSet::from(["mon-outil", "autre", "moteur"].map(String::from)));
        assert_eq!(m.dependances, BTreeSet::from(["requests", "PyYAML", "pytest", "numpy", "black", "fmt", "openssl"].map(String::from)));
    }

    #[test]
    fn sans_manifeste_rien() {
        let m = lire(&[("src/lib.rs", "pub fn f() {}")]);
        assert_eq!(m, Manifestes::default());
    }
}
