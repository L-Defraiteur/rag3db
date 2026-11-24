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
        Condition { node_type: "ClosingStepDone", description: "The step_done closes the turn: nothing left, nothing else to do in the active plan, and succeeded unless `succeeded` is `any`.", params: succeeded_param, question: closing_step_done },
        Condition { node_type: "UsedToolsAmong", description: "The step called one of `tools` (a list, `a|b|c`).", params: tools_param, question: used_tools_among },
        Condition { node_type: "StepFull", description: "The step's last model call passed the step's context bound.", params: no_params, question: step_full },
        Condition { node_type: "RequestExpects", description: "The turn's request declares what it expects (`expects`).", params: no_params, question: request_expects },
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

fn tools_param() -> Vec<ConfigParam> {
    vec![ConfigParam {
        name: "tools".into(),
        param_type: ConfigParamType::String,
        required: true,
        default: None,
        description: "The tools, `a|b|c`.".into(),
        choices: None,
        json_schema: None,
    }]
}

fn used_tools_among(step: &Value, config: &Value) -> Result<(bool, String), String> {
    let wanted: Vec<&str> = config["tools"].as_str().ok_or("UsedToolsAmong: `tools` is missing")?.split('|').map(str::trim).filter(|t| !t.is_empty()).collect();
    let used: Vec<&str> = step["tools"].as_array().into_iter().flatten().filter_map(Value::as_str).filter(|t| wanted.contains(t)).collect();
    Ok((!used.is_empty(), if used.is_empty() { "none of these tools in the step".into() } else { format!("the step called {}", used.join(", ")) }))
}

fn step_full(step: &Value, _config: &Value) -> Result<(bool, String), String> {
    let holds = step["context_full"].as_bool().unwrap_or(false);
    Ok((holds, if holds { "the step's context is full" } else { "the step has room" }.into()))
}

fn succeeded_param() -> Vec<ConfigParam> {
    choice("succeeded", "required", &["required", "any"], "Whether the step_done must say it succeeded.")
}

fn kind_param() -> Vec<ConfigParam> {
    choice("kind", "any", &["approval", "question", "any"], "The kind of card.")
}

fn flag(step: &Value, key: &str, yes: &str, no: &str) -> (bool, String) {
    let holds = step[key].as_bool().unwrap_or(false);
    (holds, if holds { yes } else { no }.to_string())
}

fn text_after_last_tool(step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok(flag(step, "text_since_last_tool", "a text after the last tool", "no text after the last tool"))
}

fn wrote_in_step(step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok(flag(step, "text_in_step", "the step wrote to the user", "the step wrote nothing to the user"))
}

fn used_tools(step: &Value, _config: &Value) -> Result<(bool, String), String> {
    Ok(flag(step, "used_tools", "the step used tools", "the step used no tool"))
}

/// The turn's lines (`turn`), none when absent.
fn turn(step: &Value) -> &[Value] {
    step["turn"].as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// The rank of the turn's last request or instruction that neither the teammate nor the host wrote.
fn request(lines: &[Value]) -> Option<usize> {
    lines.iter().rposition(|l| matches!(l["kind"].as_str(), Some("request" | "instruction")) && l["from"] != "teammate" && l["from"] != "host")
}

fn turn_asked_by(step: &Value, config: &Value) -> Result<(bool, String), String> {
    let lines = turn(step);
    let Some(n) = request(lines) else { return Ok((false, "no request in the turn".into())) };
    let human = lines[n]["by"]["kind"] == "human";
    let holds = match config["who"].as_str().unwrap_or("any") {
        "human" => human,
        "agent" => !human,
        _ => true,
    };
    Ok((holds, format!("the turn's request is {}'s", if human { "a human" } else { "an agent" })))
}

fn turn_has_teammate_line(step: &Value, _config: &Value) -> Result<(bool, String), String> {
    let lines = turn(step);
    let Some(n) = request(lines) else { return Ok((false, "no request in the turn".into())) };
    let holds = lines[n + 1..].iter().any(|l| l["from"] == "teammate" && matches!(l["kind"].as_str(), Some("report" | "question" | "approval" | "message" | "answer")));
    Ok((holds, if holds { "the teammate wrote to the asker" } else { "no line of the teammate since the request" }.into()))
}

fn closing_step_done(step: &Value, config: &Value) -> Result<(bool, String), String> {
    let d = &step["step_done"];
    if d.is_null() {
        return Ok((false, "no step_done".into()));
    }
    if d["succeeded"] != true && config["succeeded"].as_str() != Some("any") {
        return Ok((false, "the step_done did not succeed".into()));
    }
    if !d["remaining"].as_str().unwrap_or_default().trim().is_empty() {
        return Ok((false, "the step_done says something is left".into()));
    }
    let item = d["item"].as_str().unwrap_or_default();
    let reste = step["plan"]["items"].as_array().into_iter().flatten().any(|i| i["id"] != item && !matches!(i["status"].as_str(), Some("done" | "dropped")));
    Ok((!reste, if reste { "the active plan has items still to do" } else { "the step_done closes the turn" }.into()))
}

fn request_expects(step: &Value, _config: &Value) -> Result<(bool, String), String> {
    let lines = turn(step);
    let holds = request(lines).is_some_and(|n| !lines[n]["expects"].is_null());
    Ok((holds, if holds { "the request declares what it expects" } else { "the request expects nothing declared" }.into()))
}

fn open_card(step: &Value, config: &Value) -> Result<(bool, String), String> {
    let lines = turn(step);
    let kind = config["kind"].as_str().unwrap_or("any");
    let ouverte = lines.iter().find(|l| {
        l["from"] == "teammate"
            && match kind {
                "approval" => l["kind"] == "approval",
                "question" => l["kind"] == "question",
                _ => matches!(l["kind"].as_str(), Some("approval" | "question")),
            }
            && !lines.iter().any(|r| !l["id"].is_null() && r["reply_to"] == l["id"])
    });
    Ok(match ouverte {
        Some(l) => (true, format!("the card {} waits for an answer", l["id"].as_str().unwrap_or("?"))),
        None => (false, "no card waits".into()),
    })
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
