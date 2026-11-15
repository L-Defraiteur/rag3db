//! **Le modèle de langage d'une tâche** (lot tty-14, `/model` du terminal) : la liste des modèles que
//! déclare son manifeste (`models.llm`), le défaut et celui de la tâche marqués ; le choix d'un modèle
//! par son `name`, gardé dans le registre des tâches et suivi par le chat de la tâche ; un nom inconnu
//! refusé en nommant les modèles déclarés.

// Les modèles du témoin sont `compatible` (protocole openai) : sans `openai-llm`, l'hôte refuse à bon droit
// de rouvrir la tâche (« ce bâti n'a pas la feature `openai-llm` ») — vu rouge sous rag3db-native,code,daemon.
#[cfg(all(test, feature = "code", feature = "rag3db-native", feature = "openai-llm"))]
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
        let (_depot, manifeste) = depot_inscrit(d.path(), true);
        let modeles = json!([
            { "name": "rapide", "provider": "compatible", "protocol": "openai", "address": "http://127.0.0.1:9", "model": "m-rapide", "context_tokens": 1000 },
            { "name": "fin", "provider": "compatible", "protocol": "openai", "address": "http://127.0.0.1:9", "model": "m-fin", "context_tokens": 2000, "default": true }
        ]);
        let host = hote(d.path(), defaut()).with_modele_nomme(nomme());
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
        let t = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
        // Deux modèles déclarés dans le manifeste de la tâche, le second par défaut ; l'hôte relit un
        // manifeste changé à sa réouverture (le principal a les siens par sa propre copie).
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&t.manifest).unwrap()).unwrap();
        m["models"]["llm"] = modeles;
        std::fs::write(&t.manifest, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let session = host.ide_session();
        assert_eq!(host.open(Some(&session), &t.manifest)["ok"], true, "la tâche se rouvre sur son manifeste changé");

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

        // **Un choix qui ne désigne plus rien** (ED) : le manifeste renomme « rapide » en « vite ». La
        // liste dit l'ancien choix, inconnu ; le chat refuse le tour en disant quoi faire.
        let texte = std::fs::read_to_string(&t.manifest).unwrap().replace("\"rapide\"", "\"vite\"");
        std::fs::write(&t.manifest, texte).unwrap();
        assert_eq!(host.open(Some(&session), &t.manifest)["ok"], true);
        let liste = host.modeles_de_tache(&h.id, "doc").unwrap();
        assert_eq!((liste["current"].clone(), liste["current_unknown"].clone()), (json!("rapide"), json!(true)), "{liste}");
        let etat = host.chat_state(&t.id).unwrap();
        assert_eq!(etat["on"], false);
        let pourquoi = etat["why"].as_str().unwrap_or_default();
        assert!(pourquoi.contains("vite, fin") && pourquoi.contains("choisissez un modèle par /model"), "{pourquoi}");
    }
}
