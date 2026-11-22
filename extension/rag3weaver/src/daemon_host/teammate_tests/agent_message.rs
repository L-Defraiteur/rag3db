//! **A message between agents is an event of the IDE's stream** (c7 for Lucie, 22 November 2026: « des agents qui se
//! parlent »): the witness of the lot `agents-message`, in a module of its own.

use super::*;

struct Capte(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Capte {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A teammate writes a message to another agent, and that agent answers it: each line is an `AgentMessage` on the
/// subject `agents`, its sender and its recipient as agent keys (the task's `teammate` said by its own key), with the
/// task, the line's id and its text. A request of the supervisor is not one.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_message_between_agents_is_an_event_of_the_stream() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "archi-x").expect("la tâche s'ouvre");
    let cle = host.tache(&h.id, "archi-x").unwrap().agent_key.expect("a key");
    let capte = Arc::new(Mutex::new(Vec::new()));
    host.ide_flux().abonner(Box::new(Capte(capte.clone())), Some(["agents".to_string()].into_iter().collect()), None);
    let files = host.teammate_files(&h.id, "archi-x").unwrap();
    files.append("supervisor", "request", None, "Demande à l'autre.").unwrap();
    let envoi = files.append_with("teammate", "message", None, "Tu as fini la porte ?", &json!({ "to": "agent:autre-123" })).unwrap();
    files.append_with("agent:autre-123", "answer", None, "Oui.", &json!({ "to": "teammate", "reply_to": envoi["id"] })).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    let texte = String::from_utf8_lossy(&capte.lock().unwrap()).into_owned();
    let evenements: Vec<Value> = texte
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim_start_matches("data: ")).ok())
        .map(|e| e["evenement"].clone())
        .filter(|e| e["kind"] == "AgentMessage")
        .collect();
    assert_eq!(evenements.len(), 2, "two messages between agents, not the supervisor's request: {texte}");
    assert_eq!((&evenements[0]["from"], &evenements[0]["to"]), (&json!(cle), &json!("agent:autre-123")), "{}", evenements[0]);
    assert_eq!((&evenements[1]["from"], &evenements[1]["to"]), (&json!("agent:autre-123"), &json!(cle)), "{}", evenements[1]);
    assert_eq!(evenements[0]["id"], envoi["id"]);
    assert_eq!(evenements[0]["text"], json!("Tu as fini la porte ?"));
    assert_eq!(evenements[0]["task"], json!("archi-x"));
}
