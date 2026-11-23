//! **The conditions of the end-of-step guards** (23 November 2026, Lucie: « tout déclaratif »): each node answers one
//! question about a step, from the facts the host gives it (`StepFacts`, on the input `step`), and gives a verdict
//! `{holds, why}` on `holds` ([`super::guard_nodes`]). None reads a file or a mailbox; the chains that combine them are
//! declared in the teammate's templates.
//!
//! The facts read here, in the `step` map: `text_since_last_tool`, `text_in_step`, `used_tools` (booleans of the step in
//! progress); `step_done` (the arguments of the step_done being called or that closed the step: `item`, `succeeded`,
//! `remaining`); `plan` (the active plan, its `items` with `id` and `status`, or null); `turn` (the mailbox's lines since
//! the turn's last `ended`, each with `from`, `kind`, `by`, `id`, `reply_to`).

use serde_json::Value;

use super::guard_nodes::{verdict, HOLDS};
use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType};

/// The input the facts come on.
pub const STEP: &str = "step";

/// One question about a step: its answer and why, from the facts and the node's configuration.
type Question = fn(step: &Value, config: &Value) -> Result<(bool, String), String>;

/// A condition: its node type, what it asks, its configuration, and the question.
pub struct Condition {
    node_type: &'static str,
    description: &'static str,
    params: fn() -> Vec<ConfigParam>,
    question: Question,
}

/// The conditions, each one question.
pub fn conditions() -> Vec<Condition> {
    vec![
        Condition { node_type: "TextAfterLastTool", description: "The step wrote text to the user after its last tool call.", params: no_params, question: text_after_last_tool },
        Condition { node_type: "WroteInStep", description: "The step wrote text to the user, anywhere in it.", params: no_params, question: wrote_in_step },
        Condition { node_type: "UsedTools", description: "The step called a tool of work (not step_done nor report).", params: no_params, question: used_tools },
        Condition { node_type: "TurnAskedBy", description: "The turn was opened by a request of `who` (human, agent or any).", params: who_param, question: turn_asked_by },
        Condition { node_type: "TurnHasTeammateLine", description: "The teammate wrote a line to the asker since the turn's request.", params: no_params, question: turn_has_teammate_line },
        Condition { node_type: "ClosingStepDone", description: "The step_done closes the turn: succeeded, nothing left, nothing else to do in the active plan.", params: no_params, question: closing_step_done },
        Condition { node_type: "OpenCard", description: "A card of the teammate's waits for an answer in the turn (`kind`: approval, question or any).", params: kind_param, question: open_card },
    ]
}

fn no_params() -> Vec<ConfigParam> {
    Vec::new()
}

fn choice(name: &'static str, default: &'static str, choices: &[&str], description: &'static str) -> Vec<ConfigParam> {
    vec![ConfigParam {
        name: name.into(),
        param_type: ConfigParamType::String,
        required: false,
        default: Some(Value::String(default.into())),
        description: description.into(),
        choices: Some(Choices::Fixed(choices.iter().map(|c| c.to_string()).collect())),
        json_schema: None,
    }]
}

fn who_param() -> Vec<ConfigParam> {
    choice("who", "any", &["human", "agent", "any"], "Whose request opened the turn.")
}

fn kind_param() -> Vec<ConfigParam> {
    choice("kind", "any", &["approval", "question", "any"], "The kind of card.")
}

fn text_after_last_tool(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}
fn wrote_in_step(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}
fn used_tools(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}
fn turn_asked_by(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}
fn turn_has_teammate_line(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}
fn closing_step_done(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}
fn open_card(_step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok((false, "not written yet".into()))
}

/// A condition's node: it reads `step`, asks its question, gives its verdict on `holds`.
pub struct ConditionNode {
    name: String,
    node_type: &'static str,
    question: Question,
    config: Value,
}

impl Node for ConditionNode {
    fn node_type(&self) -> &str {
        self.node_type
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value(STEP, PortType::Map, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value(HOLDS, PortType::Map, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let step = ctx.input(STEP).and_then(|v| v.downcast::<Value>().cloned()).ok_or_else(|| format!("{}: no facts on 'step'", self.node_type))?;
        let (holds, why) = (self.question)(&step, &self.config)?;
        ctx.set_output(HOLDS, verdict(holds, why));
        Ok(())
    }
}

/// The factory of one condition.
pub struct ConditionFactory(pub Condition);

impl NodeFactory for ConditionFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        for p in (self.0.params)() {
            if let (Some(v), Some(Choices::Fixed(choices))) = (config.get(p.name.as_ref()).and_then(Value::as_str), p.choices.as_ref()) {
                if !choices.iter().any(|c| c == v) {
                    return Err(format!("{}: '{}' is one of {}, not « {v} »", self.0.node_type, p.name, choices.join(", ")));
                }
            }
        }
        Ok(Box::new(ConditionNode { name: name.to_string(), node_type: self.0.node_type, question: self.0.question, config: config.clone() }))
    }
    fn node_type(&self) -> &str {
        self.0.node_type
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: self.0.node_type.into(),
            description: self.0.description.into(),
            inputs: vec![PortDef::value(STEP, PortType::Map, true)],
            outputs: vec![PortDef::value(HOLDS, PortType::Map, true)],
            config_params: (self.0.params)(),
        }
    }
}

#[cfg(test)]
#[path = "step_conditions/tests.rs"]
mod tests;
