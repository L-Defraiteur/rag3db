//! **Le modèle de langage d'une tâche** (lot tty-14, `/model` du terminal) : la liste des modèles que
//! déclare son manifeste (`models.llm`), le défaut et celui de la tâche marqués ; le choix d'un modèle
//! par son `name`, gardé dans le registre des tâches et suivi par le chat de la tâche ; un nom inconnu
//! refusé en nommant les modèles déclarés.

#[cfg(all(test, feature = "code", feature = "rag3db-native"))]
mod tests {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::{depot_inscrit, hote};
    use crate::llm::{Llm, MockLlm};
    use serde_json::{json, Value};
    use std::sync::Arc;

    /// Le chat par défaut : la fenêtre du modèle par défaut (2000).
    fn defaut() -> ChatModel {
        Arc::new(|_| Ok(Arc::new(MockLlm::new("ok").with_context_len(2000)) as Arc<dyn Llm>))
    }

    /// Le modèle nommé : sa fenêtre est celle que déclare sa source.
    fn nomme() -> super::super::chat::ModeleNomme {
        Arc::new(|s: &crate::model_source::ModelSource| Ok(Arc::new(MockLlm::new("ok").with_context_len(s.context_tokens.unwrap_or(0))) as Arc<dyn Llm>))
    }

    #[test]
    fn a_task_lists_its_declared_models_chooses_one_by_name_and_its_chat_follows() {
        let d = tempfile::tempdir().unwrap();
        let (depot, manifeste) = depot_inscrit(d.path(), true);
        // Deux modèles déclarés, le second par défaut, commités : la tâche part du commit du dépôt, une
        // modification non commitée du manifeste n'y entre pas.
        let modeles = json!([
            { "name": "rapide", "provider": "compatible", "protocol": "openai", "address": "http://127.0.0.1:9", "model": "m-rapide", "context_tokens": 1000 },
            { "name": "fin", "provider": "compatible", "protocol": "openai", "address": "http://127.0.0.1:9", "model": "m-fin", "context_tokens": 2000, "default": true }
        ]);
        // Le principal lit sa copie du dossier de configuration ; la tâche part du manifeste du
        // dépôt, au commit d'où part sa branche : les deux portent les modèles, celui du dépôt commité.
        let du_depot = depot.join("templates/backends/code/backend.json");
        assert!(du_depot.is_file(), "le manifeste du dépôt : {}", du_depot.display());
        for f in [&manifeste, &du_depot] {
            let mut m: Value = serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
            m["models"]["llm"] = modeles.clone();
            std::fs::write(f, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        }
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "deux modèles déclarés"]] {
            let o = std::process::Command::new("git").current_dir(&depot).args(args).output().unwrap();
            assert!(o.status.success(), "git {args:?} : {}{} (manifeste {})", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr), manifeste.display());
        }
        let host = hote(d.path(), defaut()).with_modele_nomme(nomme());
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");

        let liste = host.modeles_de_tache(&h.id, "doc").expect("la liste des modèles");
        let vus: Vec<(String, String, bool, bool)> = liste["models"].as_array().unwrap().iter().map(|x| (x["name"].as_str().unwrap().into(), x["model"].as_str().unwrap().into(), x["default"] == true, x["current"] == true)).collect();
        assert_eq!(vus, vec![("rapide".into(), "m-rapide".into(), false, false), ("fin".into(), "m-fin".into(), true, true)], "{liste}");
        assert_eq!(liste["current"], "fin", "sans choix, la tâche suit le défaut");
        assert_eq!(host.chat_state(&t.id).unwrap()["context"]["max"], 2000);

        let choisi = host.choisir_le_modele(&h.id, "doc", "rapide").expect("le choix d'un modèle déclaré");
        assert_eq!(choisi["current"], "rapide");
        assert_eq!(host.modeles_de_tache(&h.id, "doc").unwrap()["current"], "rapide");
        assert_eq!(host.chat_state(&t.id).unwrap()["context"]["max"], 1000, "le chat de la tâche suit le modèle choisi");
        let registre = std::fs::read_to_string(d.path().join("taches/registre.json")).unwrap();
        assert!(registre.contains("\"modele\": \"rapide\""), "le choix se garde avec la tâche : {registre}");

        let refus = host.choisir_le_modele(&h.id, "doc", "inconnu").unwrap_err();
        assert!(refus.contains("inconnu") && refus.contains("rapide, fin"), "un nom inconnu se refuse en nommant la liste : {refus}");
        assert_eq!(host.modeles_de_tache(&h.id, "doc").unwrap()["current"], "rapide", "un refus ne change rien");
        assert!(host.modeles_de_tache(&h.id, "absente").is_err(), "une tâche absente se refuse");
    }
}
