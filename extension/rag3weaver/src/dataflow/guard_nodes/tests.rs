//! The witness of the lot `guard-chain` (23 November 2026): the guard chain's primitives, each one thing.
use super::*;

fn run(factory: &dyn NodeFactory, config: Value, inputs: &[(&str, (bool, &str))]) -> Result<Value, String> {
    let mut node = factory.create("n", &config)?;
    let mut ctx = NodeContext::new();
    for (port, (holds, why)) in inputs {
        ctx.set_input(port, verdict(*holds, *why));
    }
    node.execute(&mut ctx)?;
    let out = node.outputs()[0].name.to_string();
    let mut outputs = ctx.drain_outputs();
    Ok(outputs.remove(&out).and_then(|v| v.downcast::<Value>().cloned()).expect("an output"))
}

/// **`FirstMatch` gives the first rule that holds, in the declared order, else `otherwise`**; a rule whose input did
/// not come counts false; `Not`, `All` and `Any` combine verdicts and say why.
#[test]
fn the_first_rule_that_holds_wins_and_the_combinators_say_why() {
    let f = FirstMatchNodeFactory;
    let rules = json!({ "rules": "remaining|silent_turn|request_without_a_line", "otherwise": "end" });
    let v = run(&f, rules.clone(), &[("remaining", (false, "nothing left")), ("silent_turn", (true, "no text after the last tool")), ("request_without_a_line", (true, "asked"))]).unwrap();
    assert_eq!(v, json!({ "verdict": "silent_turn", "why": "no text after the last tool" }), "the first in the declared order");
    let v = run(&f, rules.clone(), &[("request_without_a_line", (false, "a line"))]).unwrap();
    assert_eq!(v["verdict"], json!("end"), "none holds, the others did not come");
    assert!(f.create("n", &json!({ "rules": "end", "otherwise": "end" })).is_err(), "otherwise is not a rule");
    assert!(f.create("n", &json!({ "rules": "a b" })).is_err(), "a rule is a port name");

    assert_eq!(run(&NotNodeFactory, json!({}), &[("holds", (true, "wrote"))]).unwrap(), json!({ "holds": false, "why": "not (wrote)" }));
    let all = JoinNodeFactory { all: true };
    let any = JoinNodeFactory { all: false };
    let two = json!({ "inputs": "asked|silent" });
    assert_eq!(run(&all, two.clone(), &[("asked", (true, "by a human")), ("silent", (true, "no text"))]).unwrap()["holds"], json!(true));
    assert_eq!(run(&all, two.clone(), &[("asked", (true, "by a human"))]).unwrap()["holds"], json!(false), "an input that did not come counts false");
    assert_eq!(run(&any, two.clone(), &[("silent", (true, "no text"))]).unwrap()["holds"], json!(true));
    let why = run(&all, two, &[("asked", (true, "by a human")), ("silent", (false, "a text"))]).unwrap();
    assert_eq!(why["why"], json!("all (asked: by a human; silent: a text)"));
}

/// **The schema follows the configuration**: `FirstMatch` has one input per rule, `All` one per named input, so that a
/// Mermaid edge `|silent_turn|` plugs in.
#[test]
fn the_ports_follow_the_configuration() {
    let s = FirstMatchNodeFactory.schema_for(&json!({ "rules": ["remaining", "silent_turn"] }));
    assert_eq!(s.inputs.iter().map(|p| p.name.to_string()).collect::<Vec<_>>(), ["remaining", "silent_turn"]);
    let s = JoinNodeFactory { all: true }.schema_for(&json!({ "inputs": "a|b|c" }));
    assert_eq!(s.inputs.len(), 3);
}
