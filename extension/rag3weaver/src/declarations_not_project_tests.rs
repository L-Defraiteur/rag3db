//! **list_declarations says what it lists** (Lucie's dry run, 22 Nov 2026: on an empty repository, list_files rendered
//! nothing, the agent called list_declarations and showed its own manifest's files as the project's). Its answer and its
//! description say these are the backend's declarations, not the project's files. A module of its own.

use super::reload_tests::toy;
use crate::agent::ToolBox;
use crate::llm::{ToolCall, Turn};
use serde_json::json;

struct Nothing;
impl ToolBox for Nothing {
    fn call(&self, call: &ToolCall) -> Turn {
        Turn::tool_result(&call.id, &call.name, "{}")
    }
}

#[test]
fn list_declarations_says_these_are_not_the_projects_files() {
    let (_dir, backend) = toy();
    let tools = crate::declare::DeclarationTools { backend: &backend, inner: &Nothing };
    let r = tools.call(&ToolCall::new("1", "list_declarations", json!({}).to_string()));
    assert!(r.content.contains("not the project's files"), "{}", r.content);
    let def = tools.tool_defs().into_iter().find(|d| d.name == "list_declarations").unwrap();
    assert!(def.description.contains("not the project's files"), "{}", def.description);
}
