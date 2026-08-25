//! E2E : **Qwen2.5-0.5B-Instruct en local**, sur burn/wgpu, sans réseau.
//!
//! C'est le pendant local de `openai_llm_*` : le même trait, les mêmes
//! invariants, un modèle de 996 Mo posé sur la machine.
//!
//! Poser les artefacts (une fois) :
//!
//! ```sh
//! mkdir -p ~/.cache/rag3weaver/qwen2.5-0.5b-instruct
//! # model.bpk : voir generated/README.md (conversion ONNX -> burnpack)
//! curl -L -o ~/.cache/rag3weaver/qwen2.5-0.5b-instruct/tokenizer.json \
//!   https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/main/tokenizer.json
//! curl -L -o ~/.cache/rag3weaver/qwen2.5-0.5b-instruct/tokenizer_config.json \
//!   https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/main/tokenizer_config.json
//! ```
//!
//! Remplacer les chemins avec `RAG3WEAVER_QWEN_DIR`, ou
//! `RAG3WEAVER_QWEN_BPK` / `_TOKENIZER` / `_TOKENIZER_CONFIG`.
//!
//! Run with: cargo test --test e2e_burn_llm --features burn-llm -- --ignored --nocapture

#![cfg(feature = "burn-llm")]

use std::time::Instant;

use rag3weaver::burn_llm::BurnLlm;
use rag3weaver::llm::{
    CountingSink, FinishReason, GenOptions, Llm, StringSink, Turn,
};

fn model() -> BurnLlm {
    let dir = BurnLlm::default_dir();
    assert!(
        dir.join("model.bpk").exists() || std::env::var("RAG3WEAVER_QWEN_BPK").is_ok(),
        "poids absents : {}\nVoir l'en-tête de ce fichier.",
        dir.display()
    );
    let t = Instant::now();
    let m = BurnLlm::from_dir(&dir, Default::default()).expect("chargement");
    eprintln!("[burn-llm] chargement : {:?}", t.elapsed());
    m
}

fn ask(q: &str) -> Vec<Turn> {
    vec![
        Turn::system("Tu réponds en une phrase, en français."),
        Turn::user(q),
    ]
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Le modèle génère, et il génère la même chose deux fois
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn generation_is_real_and_deterministic() {
    let llm = model();
    let opts = GenOptions::default().with_max_tokens(48);

    let run = |llm: &BurnLlm| {
        let mut sink = StringSink::default();
        let t = Instant::now();
        let (finish, usage) = llm.generate(&ask("Quelle est la capitale de la France ?"), &opts, &mut sink).unwrap();
        let ms = t.elapsed().as_millis();
        eprintln!(
            "[burn-llm] {} jetons en {} ms ({:.1} j/s), fin={:?}\n  >>> {}",
            usage.completion_tokens,
            ms,
            usage.tokens_per_s(),
            finish.reason,
            sink.text
        );
        (sink.text, finish, usage)
    };

    let (a, finish, usage) = run(&llm);
    assert!(!a.trim().is_empty(), "le modèle n'a rien dit");
    assert!(usage.completion_tokens > 0);
    assert_eq!(usage.prompt_tokens > 0, true, "prompt_tokens exact, pas estimé");
    assert!(finish.tool_calls.is_empty());

    let (b, _, _) = run(&llm);
    assert_eq!(a, b, "température 0 : deux exécutions doivent coïncider");

    // Le contenu : un 0,5 B doit trouver Paris.
    assert!(
        a.to_lowercase().contains("paris"),
        "réponse inattendue : {a}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Les invariants du trait
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn flow_stop_cuts_the_decode_loop_immediately() {
    let llm = model();
    let opts = GenOptions::default().with_max_tokens(200);
    let mut sink = CountingSink::stopping_after(3);
    let t = Instant::now();
    let (finish, usage) = llm.generate(&ask("Raconte-moi une longue histoire."), &opts, &mut sink).unwrap();
    let ms = t.elapsed().as_millis();

    eprintln!("[burn-llm] annulé après {} fragments en {ms} ms", sink.tokens);
    assert_eq!(finish.reason, FinishReason::Cancelled);
    assert_eq!(sink.tokens, 3, "pas un fragment de plus");
    assert_eq!(sink.finished, Some(finish.clone()), "on_finish appelé, une fois");
    assert!(
        usage.completion_tokens < 30,
        "la boucle doit s'arrêter net, pas continuer à décoder : {} jetons",
        usage.completion_tokens
    );
}

#[test]
#[ignore]
fn max_tokens_is_honored_exactly() {
    let llm = model();
    let opts = GenOptions::default().with_max_tokens(12);
    let mut sink = StringSink::default();
    let (finish, usage) = llm.generate(&ask("Raconte-moi une longue histoire."), &opts, &mut sink).unwrap();
    eprintln!("[burn-llm] max_tokens=12 -> {} jetons : {}", usage.completion_tokens, sink.text);
    assert_eq!(usage.completion_tokens, 12);
    assert_eq!(finish.reason, FinishReason::MaxTokens);
    assert!(!finish.is_complete(), "tronqué par notre plafond");
}

#[test]
#[ignore]
fn a_stop_sequence_cuts_client_side() {
    let llm = model();
    // Le modèle compte ; on coupe sur « 3 ».
    let turns = vec![Turn::user("Compte de 1 à 9, séparé par des virgules, sans rien d'autre.")];
    let opts = GenOptions::default().with_max_tokens(60).with_stop(vec!["3".into()]);
    let mut sink = StringSink::default();
    let (finish, _) = llm.generate(&turns, &opts, &mut sink).unwrap();

    eprintln!("[burn-llm] stop sur \"3\" -> {:?} / {:?}", sink.text, finish.reason);
    match &finish.reason {
        FinishReason::Stop(s) => assert_eq!(s, "3"),
        // Un 0,5 B peut ne jamais produire « 3 » : on ne bâtit pas le test
        // sur son obéissance, seulement sur le fait que s'il le produit, on
        // coupe — et que la séquence n'est jamais poussée dans le puits.
        other => eprintln!("[burn-llm] le modèle n'a pas produit \"3\" ({other:?})"),
    }
    assert!(!sink.text.contains('3'), "la séquence d'arrêt ne doit jamais être émise : {:?}", sink.text);
}

#[test]
#[ignore]
fn context_overflow_is_reported_not_crashed() {
    let llm = model();
    let huge = "mot ".repeat(40_000);
    let err = llm
        .generate(&[Turn::user(huge)], &GenOptions::default(), &mut StringSink::default())
        .unwrap_err();
    eprintln!("[burn-llm] {err}");
    assert!(matches!(err, rag3weaver::llm::LlmError::ContextOverflow { .. }));
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Le prompt rendu — c'est le premier suspect quand un local déraille
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn the_chat_template_renders_qwen_markup_and_the_tools_block() {
    let llm = model();
    let opts = GenOptions::default();
    let p = llm.render_prompt(&ask("bonjour"), &opts).unwrap();
    eprintln!("[burn-llm] prompt nu :\n{p}");
    assert!(p.contains("<|im_start|>system"));
    assert!(p.contains("<|im_start|>user\nbonjour<|im_end|>"));
    assert!(p.ends_with("<|im_start|>assistant\n"), "l'amorce de génération");
    assert!(!p.contains("# Tools"), "sans outils, pas de bloc outils");

    // Avec outils : le bloc est natif, on ne l'injecte pas.
    let (nodes, _) = rag3weaver::dataflow::builtin_graph_tools().unwrap();
    let defs = rag3weaver::tools::tool_defs(&nodes);
    let with = llm.render_prompt(&ask("bonjour"), &opts.clone().with_tools(defs)).unwrap();
    assert!(with.contains("# Tools"), "bloc outils absent");
    assert!(with.contains("<tool_call>"), "consigne d'appel absente");
    assert!(with.contains("LlmNode"), "les outils du registre doivent y être");

    // Et un historique d'agent se rejoue : appel puis résultat.
    let call = rag3weaver::llm::ToolCall::new("call_1", "search", r#"{"query":"x"}"#);
    let replay = vec![
        Turn::user("cherche"),
        Turn::assistant_with_calls("", vec![call]),
        Turn::tool_result("call_1", "search", "[]"),
    ];
    let r = llm.render_prompt(&replay, &opts).unwrap();
    eprintln!("[burn-llm] historique rejoué :\n{r}");
    assert!(r.contains("<tool_call>"), "l'appel doit être rejoué");
    assert!(r.contains(r#""arguments": {"query": "x"}"#), "arguments en OBJET : {r}");
    assert!(r.contains("<tool_response>"), "le résultat doit être rejoué");
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Le cache KV est-il *correct* ? (et pas seulement rapide)
// ═════════════════════════════════════════════════════════════════════════════

/// Le test qui vaut tous les autres : le décodage incrémental avec cache doit
/// rendre **exactement** ce que rend le recalcul complet du contexte à chaque
/// pas. Un cache qui marche « à peu près » produit du texte plausible et faux ;
/// c'est ce qu'on ne veut pas découvrir en production.
#[test]
#[ignore]
fn the_kv_cache_matches_a_full_recompute() {
    let llm = model();
    let turns = ask("Quelle est la capitale de la France ?");
    let prompt = llm.render_prompt(&turns, &GenOptions::default()).unwrap();

    // Décodage glouton **sans cache** : chaque pas repart du contexte entier.
    let mut ids = llm.encode_for_test(&prompt);
    let start = ids.len();
    for _ in 0..12 {
        let logits = llm.prefill_logits_for_test(&ids);
        let mut best = 0usize;
        for (i, v) in logits.iter().enumerate() {
            if v > &logits[best] {
                best = i;
            }
        }
        ids.push(best as u32);
    }
    let without_cache = llm.decode_for_test(&ids[start..]);

    // Le même, par la boucle normale — donc avec cache.
    let mut sink = StringSink::default();
    llm.generate(&turns, &GenOptions::default().with_max_tokens(12), &mut sink).unwrap();

    eprintln!("[cache] sans cache : {without_cache:?}");
    eprintln!("[cache] avec cache : {:?}", sink.text);
    assert!(
        without_cache.starts_with(sink.text.trim_end()) || sink.text.trim_end() == without_cache.trim_end(),
        "le cache diverge du recalcul complet :\n  sans = {without_cache:?}\n  avec = {:?}",
        sink.text
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. Le canari numérique
// ═════════════════════════════════════════════════════════════════════════════

/// Trois complétions brutes, sans balisage de chat, dont la suite est
/// évidente. C'est le test qui a **attrapé l'export fp16 dégradé**
/// d'onnx-community : il rendait « The capital of France is » → « is is is »
/// là où le f32 rend « Paris ». Toute future bascule de quantification doit
/// repasser par ici.
#[test]
#[ignore]
fn raw_completions_are_numerically_sane() {
    let llm = model();
    let cases = [
        ("The capital of France is", "paris"),
        ("1, 2, 3, 4,", "5"),
        ("def add(a, b):\n    return", "a + b"),
    ];
    for (raw, expected) in cases {
        let mut ids = llm.encode_for_test(raw);
        let start = ids.len();
        for _ in 0..10 {
            let l = llm.prefill_logits_for_test(&ids);
            let mut b = 0usize;
            for (i, v) in l.iter().enumerate() {
                if v > &l[b] {
                    b = i;
                }
            }
            ids.push(b as u32);
        }
        let out = llm.decode_for_test(&ids[start..]);
        eprintln!("[sanity] {raw:?} >>> {out:?}");
        assert!(
            out.to_lowercase().contains(expected),
            "complétion dégradée : {raw:?} devait contenir {expected:?}, a rendu {out:?}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. Les appels d'outils, format de la famille Qwen
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn the_model_emits_well_formed_tool_calls() {
    let llm = model();
    let (_, graph_tools) = rag3weaver::dataflow::builtin_graph_tools().unwrap();
    let defs = rag3weaver::tools::graph_tool_defs(&graph_tools);
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    let opts = GenOptions::default().with_max_tokens(120).with_tools(defs.clone());

    let turns = vec![
        Turn::system("Tu es un agent. Utilise les outils pour répondre."),
        Turn::user("Cherche les produits qui parlent de programmation."),
    ];
    let mut sink = StringSink::default();
    let t = Instant::now();
    let (finish, usage) = llm.generate(&turns, &opts, &mut sink).unwrap();
    eprintln!(
        "[tools] {:?} — {} jetons en {:?} ({:.1} j/s)\n  texte visible = {:?}\n  appels = {:?}",
        finish.reason,
        usage.completion_tokens,
        t.elapsed(),
        usage.tokens_per_s(),
        sink.text,
        finish.tool_calls.iter().map(|c| (&c.name, &c.arguments)).collect::<Vec<_>>()
    );

    assert!(!finish.tool_calls.is_empty(), "aucun appel émis : {:?}", sink.text);
    assert_eq!(finish.reason, FinishReason::ToolCall);
    let call = &finish.tool_calls[0];
    assert!(names.contains(&call.name.as_str()), "outil inventé : {}", call.name);
    // Les arguments doivent être du JSON exploitable.
    let args: serde_json::Value = serde_json::from_str(&call.arguments)
        .unwrap_or_else(|e| panic!("arguments non JSON ({e}) : {}", call.arguments));
    assert!(args.is_object());
    // Et le balisage ne doit **jamais** fuir dans le puits.
    assert!(!sink.text.contains("<tool_call>"), "balise poussée en texte : {:?}", sink.text);
    assert!(!call.id.is_empty(), "un appel sans identifiant est irrejouable");
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. Débit — pas un seuil, une mesure qu'on veut voir passer dans le journal
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn throughput() {
    let llm = model();
    // Trois prompts de longueurs croissantes ; deux passes chacun, parce que
    // wgpu compile ses noyaux à la première forme rencontrée.
    for (label, prompt_words) in [("court", 10usize), ("moyen", 300), ("long", 1500)] {
        let filler = "mot ".repeat(prompt_words);
        let turns = vec![Turn::user(format!("{filler}\nRaconte une histoire."))];
        let opts = GenOptions::default().with_max_tokens(32);
        for pass in 0..2 {
            let mut sink = StringSink::default();
            let t = Instant::now();
            let (_, u) = llm.generate(&turns, &opts, &mut sink).unwrap();
            let ms = t.elapsed().as_millis() as f64;
            if pass == 1 {
                eprintln!(
                    "[débit] prompt {label:6} ({} jetons) : {} générés en {:.0} ms -> {:.1} j/s ({:.0} ms/jeton)",
                    u.prompt_tokens,
                    u.completion_tokens,
                    ms,
                    u.completion_tokens as f64 * 1000.0 / ms,
                    ms / u.completion_tokens.max(1) as f64
                );
            }
        }
    }
}
