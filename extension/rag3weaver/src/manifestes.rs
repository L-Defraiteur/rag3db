//! **Les manifestes d'un dépôt, pour la provenance des imports** (14 novembre 2026, second lot de la
//! provenance) : les dépendances qu'ils déclarent et les paquets du dépôt lui-même (les crates d'un
//! workspace Cargo, les paquets d'un workspace npm). codeparsers en tire la provenance d'un nom importé
//! (`declared_dependencies` → `dependency`, `project_packages` → `project`) ; rien n'est deviné : un nom
//! que les manifestes ne disent pas reste `inconnue`.
//!
//! Lus : `Cargo.toml`, `Cargo.lock`, `package.json`, `pyproject.toml`, `requirements*.txt`,
//! `vcpkg.json`, n'importe où dans la liste des fichiers de la source (un dépôt a souvent plusieurs
//! manifestes). Sans parseur TOML, comme `origin.rs` : des sections et des clés, ligne à ligne.

use std::collections::BTreeSet;

/// Ce que les manifestes d'un dépôt disent.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifestes {
    /// Les chemins des manifestes lus, tels que la source les liste (dits dans l'état d'index).
    pub lus: Vec<String>,
    /// Les noms des dépendances déclarées (hors paquets du dépôt), tels qu'écrits.
    pub dependances: BTreeSet<String>,
    /// Les noms des paquets du dépôt lui-même.
    pub paquets_du_projet: BTreeSet<String>,
}

/// Un nom de fichier de manifeste reconnu.
pub fn est_un_manifeste(chemin: &str) -> bool {
    let nom = chemin.rsplit('/').next().unwrap_or(chemin);
    matches!(nom, "Cargo.toml" | "Cargo.lock" | "package.json" | "pyproject.toml" | "vcpkg.json")
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
            match nom {
                "Cargo.toml" => m.lire_cargo_toml(&texte),
                "Cargo.lock" => m.dependances.extend(noms_de_section(&texte, |s| s == "[[package]]")),
                "package.json" => m.lire_package_json(&texte),
                "pyproject.toml" => m.lire_pyproject(&texte),
                "vcpkg.json" => m.lire_vcpkg(&texte),
                _ => m.dependances.extend(texte.lines().filter_map(nom_pep508)),
            }
            m.lus.push(chemin.clone());
        }
        // Une crate du workspace figure aussi dans le Cargo.lock : elle est du projet, pas une dépendance.
        let projet: BTreeSet<String> = m.paquets_du_projet.iter().map(|n| normaliser(n)).collect();
        m.dependances.retain(|d| !projet.contains(&normaliser(d)));
        m
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
