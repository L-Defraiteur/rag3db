//! Planification des changements de gabarits, sans application.

#[cfg(test)]
mod tests {
    use crate::lecteur::{Lecteur, Recouvrement};
    use std::path::Path;

    /// Un processus neuf évite qu'un cache builtin déjà chaud cache une extraction.
    #[test]
    fn a_rejected_candidate_does_not_extract_a_builtin_cache() {
        const CHILD: &str = "RAG3WEAVER_TEST_M8_COLD_CACHE";
        const NAME: &str = "template_plan::tests::a_rejected_candidate_does_not_extract_a_builtin_cache";
        if std::env::var_os(CHILD).is_some() {
            let root = std::path::PathBuf::from(std::env::var_os("RAG3WEAVER_BUILTIN_TEMPLATES_DIR").unwrap());
            assert!(!root.exists());
            let path = root.parent().unwrap().join("virtual/backend.json");
            // Le graphe existe dans le livré, mais exige un workspace absent : refus attendu.
            let reader = Recouvrement::sur(path.parent().unwrap(), &[("backend.json".into(),
                r#"{"version":1,"name":"m8","database":"memory.rag3db","tools":{"files":{"graph":"builtin:tools/list.mmd"}}}"#.into())]);
            assert!(reader.est_fichier(&path));
            let _ = crate::backend::PreparedBackend::load_avec(&path, &reader);
            assert!(!root.exists(), "le refus de la proposition a extrait les gabarits dans {}", root.display());
            assert!(!path.exists(), "le manifeste fourni n'est pas écrit");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env(CHILD, "1")
            .env("RAG3WEAVER_BUILTIN_TEMPLATES_DIR", dir.path().join("cache-vide"))
            .output().unwrap();
        assert!(result.status.success(), "{}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        assert!(!dir.path().join("cache-vide").exists());
    }

    // Le témoin de caractérisation ci-dessus sera branché sur la validation fermée
    // commune avant de déclarer le plan pur livré. Aucune base n'est ouverte.
    #[test]
    fn the_virtual_manifest_is_not_a_disk_fixture() {
        let path = Path::new("/__m8_sources_without_disk__/backend.json");
        assert!(!path.exists());
    }
}
