//! Qwen2.5-0.5B-Instruct sur burn — **génération locale**, sans Python, sans
//! serveur, sans réseau. C'est le pendant de [`crate::openai_llm`] : le même
//! trait [`Llm`], les mêmes invariants, un modèle de 996 Mo posé sur la
//! machine à la place d'un fournisseur.
//!
//! # Ce que ce module fait vraiment
//!
//! Sur le cloud, le fournisseur rend le chat template, décide de la fin de
//! génération et garantit la forme des appels d'outils. **Ici, c'est nous le
//! serveur** : les trois sont à notre charge (doc 50).
//!
//! 1. **Rendu du template** — via `hf-chat-template`, alimenté par le
//!    `tokenizer_config.json` du modèle. Le bloc `tools` de Qwen2.5 est natif :
//!    on ne l'injecte pas, on le remplit.
//! 2. **Boucle de décodage** — préremplissage puis pas à pas avec cache KV. Le
//!    cache n'est pas une optimisation confortable : sans lui chaque jeton
//!    recalcule tout le contexte (mesuré : ~400 ms pour 1 024 jetons contre
//!    ~40 ms pour un pas caché, **facteur 30**).
//! 3. **Parsing** — `<tool_call>{"name":…,"arguments":…}</tool_call>`, format
//!    de la famille Qwen (doc 50 §3), avec les tolérances de llama.cpp.
//!
//! # Ce qu'il ne fait pas — et il faut le savoir avant de s'en servir
//!
//! Une famille, un modèle. Pas de décodage contraint, pas de parsing
//! multi-familles, aucune injection d'outils dans un template qui n'en a pas.
//!
//! **Trois champs de [`GenOptions`] sont ignorés ici**, et un réglage ignoré
//! en silence est le genre de chose qui se paie trois semaines plus tard :
//!
//! | champ | état | pourquoi |
//! |---|---|---|
//! | `tool_choice` | **ignoré** | seul `Auto` est réalisable sans décodage contraint. `Required` et `Function` demandent une grammaire (`llguidance`), hors périmètre de cette passe. Le modèle reste libre de ne pas appeler d'outil. |
//! | `response_format` | **ignoré** | même raison : c'est une grammaire. |
//! | `reasoning` | **sans objet** | Qwen2.5 n'a pas de mode de réflexion ; il n'y a rien à borner. |
//!
//! [`Usage::retries`] vaut toujours `0` : il n'y a pas de transport, donc pas
//! de réessai — c'est exact, pas un oubli.
//! [`QwenConfig`] paramètre la géométrie pour qu'un second modèle de même
//! signature (Luciole-1B, 24 couches lui aussi) soit une option et pas une
//! réécriture — mais le **graphe** généré, lui, reste propre à un modèle.
//!
//! # Poids
//!
//! Jamais dans git. `model.bpk` (1,99 Go, **f32**) et `tokenizer.json` /
//! `tokenizer_config.json` se posent dans
//! `~/.cache/rag3weaver/qwen2.5-0.5b-instruct/` — voir `generated/README.md`
//! pour la recette et les empreintes. `RAG3WEAVER_QWEN_BPK` et
//! `RAG3WEAVER_QWEN_TOKENIZER` remplacent les chemins par défaut, comme pour
//! les six modèles précédents.
//!
//! ```ignore
//! let llm = BurnLlm::from_dir(BurnLlm::default_dir(), Default::default())?;
//! let out = generate_to_string(&llm, &[Turn::user("Bonjour")], &GenOptions::default())?;
//! ```

use std::path::{Path, PathBuf};

use burn::prelude::*;
use hf_chat_template::{ChatTemplate, Message, RenderInput, TokenizerConfig};
use serde_json::{json, Value};
use tokenizers::Tokenizer;

use crate::burn_device::BurnDevice;
use crate::llm::{
    emit, first_stop, holdback, Finish, FinishReason, GenOptions, Llm, LlmError, TokenSink,
    ToolCall, Turn, Usage,
};
use crate::qwen2_5_0_5b_onnx::Model as QwenGraph;

// ─── Configuration du modèle ─────────────────────────────────────────────────

/// Géométrie et jetons spéciaux d'un décodeur de la famille Qwen.
///
/// Paramétré et non codé en dur : la boucle, le cache, le parsing et le rendu
/// ne dépendent que de ces champs. Un modèle de même signature se branche en
/// ajoutant un constructeur et son graphe généré ; seul le graphe est
/// spécifique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QwenConfig {
    /// Nom lisible, rendu par [`Llm::name`].
    pub name: String,
    /// Nombre de couches. Le cache en porte `2 × n_layers` tenseurs.
    pub n_layers: usize,
    /// Têtes clé/valeur (GQA). Qwen2.5-0.5B n'en a que **2**, ce qui est
    /// exactement ce qui rend son cache minuscule : 96 Mio à 8 k jetons, là
    /// où Qwen3-0.6B (8 têtes KV × 128) en demande 896.
    pub n_kv_heads: usize,
    pub head_dim: usize,
    pub vocab_size: usize,
    /// Fenêtre utilisable, en jetons.
    pub context_len: usize,
    /// Jetons qui terminent un tour. Pour Qwen2.5 : `<|im_end|>` (fin de tour)
    /// **et** `<|endoftext|>` — un modèle instruct émet parfois le second.
    pub eos_ids: Vec<u32>,
    /// Balise ouvrante d'un appel d'outil.
    pub tool_call_open: String,
    /// Balise fermante.
    pub tool_call_close: String,
}

impl Default for QwenConfig {
    fn default() -> Self {
        Self::qwen2_5_0_5b_instruct()
    }
}

impl QwenConfig {
    /// `Qwen/Qwen2.5-0.5B-Instruct` — les valeurs viennent de son
    /// `config.json` (24 couches, hidden 896, 14 têtes Q, 2 têtes KV,
    /// head_dim 64, vocab 151 936, contexte 32 768).
    pub fn qwen2_5_0_5b_instruct() -> Self {
        Self {
            name: "Qwen/Qwen2.5-0.5B-Instruct (burn, fp16)".into(),
            n_layers: 24,
            n_kv_heads: 2,
            head_dim: 64,
            vocab_size: 151_936,
            context_len: 32_768,
            // <|im_end|>, <|endoftext|>
            eos_ids: vec![151_645, 151_643],
            tool_call_open: "<tool_call>".into(),
            tool_call_close: "</tool_call>".into(),
        }
    }

    /// Nombre de tenseurs de cache : une clé et une valeur par couche.
    pub fn n_past(&self) -> usize {
        2 * self.n_layers
    }

    /// Empreinte du cache KV pour `seq` jetons, en octets (fp16, lot de 1).
    pub fn kv_cache_bytes(&self, seq: usize) -> usize {
        2 * self.n_layers * self.n_kv_heads * self.head_dim * seq * 2
    }
}

// ─── Réglages d'échantillonnage propres au backend ───────────────────────────

/// Ce que [`GenOptions`] ne porte pas, parce que c'est du vocabulaire de
/// décodeur local et non du vocabulaire partagé avec les fournisseurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingConfig {
    /// `0` = désactivé. Appliqué **avant** top-p.
    pub top_k: usize,
    /// Graine du générateur. Fixe par défaut : à température non nulle, deux
    /// exécutions du même prompt restent reproductibles, ce qui est la seule
    /// façon de tester un échantillonnage.
    pub seed: u64,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self { top_k: 40, seed: 0x5EED_1234_ABCD_0001 }
    }
}

// ─── Le modèle ───────────────────────────────────────────────────────────────

/// Qwen2.5-0.5B-Instruct sur burn/wgpu. Implémente [`Llm`].
pub struct BurnLlm {
    graph: QwenGraph,
    tokenizer: Tokenizer,
    template: ChatTemplate,
    config: QwenConfig,
    sampling: SamplingConfig,
    device: Device,
}

impl BurnLlm {
    /// Dossier par défaut : `$RAG3WEAVER_QWEN_DIR`, sinon
    /// `~/.cache/rag3weaver/qwen2.5-0.5b-instruct`.
    pub fn default_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("RAG3WEAVER_QWEN_DIR") {
            return PathBuf::from(dir);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".cache/rag3weaver/qwen2.5-0.5b-instruct")
    }

    /// Charge depuis un dossier contenant `model.bpk`, `tokenizer.json` et
    /// `tokenizer_config.json`. Chaque chemin peut être remplacé par sa
    /// variable d'environnement.
    pub fn from_dir(dir: impl AsRef<Path>, device: BurnDevice) -> Result<Self, LlmError> {
        let dir = dir.as_ref();
        let pick = |var: &str, file: &str| -> PathBuf {
            std::env::var(var).map(PathBuf::from).unwrap_or_else(|_| dir.join(file))
        };
        Self::from_files(
            pick("RAG3WEAVER_QWEN_BPK", "model.bpk"),
            pick("RAG3WEAVER_QWEN_TOKENIZER", "tokenizer.json"),
            pick("RAG3WEAVER_QWEN_TOKENIZER_CONFIG", "tokenizer_config.json"),
            QwenConfig::default(),
            device,
        )
    }

    /// Charge en nommant chaque fichier.
    pub fn from_files(
        weights: impl AsRef<Path>,
        tokenizer: impl AsRef<Path>,
        tokenizer_config: impl AsRef<Path>,
        config: QwenConfig,
        device: BurnDevice,
    ) -> Result<Self, LlmError> {
        let read = |p: &Path| {
            std::fs::read_to_string(p)
                .map_err(|e| LlmError::Model(format!("read {}: {e}", p.display())))
        };
        let tok = Tokenizer::from_file(tokenizer.as_ref())
            .map_err(|e| LlmError::Model(format!("tokenizer: {e}")))?;
        let tok_config: TokenizerConfig = serde_json::from_str(&read(tokenizer_config.as_ref())?)
            .map_err(|e| LlmError::Model(format!("tokenizer_config.json: {e}")))?;
        let template = ChatTemplate::from_tokenizer_config(&tok_config)
            .map_err(|e| LlmError::Model(format!("chat template: {e}")))?;

        let bytes = std::fs::read(weights.as_ref())
            .map_err(|e| LlmError::Model(format!("read {}: {e}", weights.as_ref().display())))?;
        Self::from_bytes(&bytes, tok, template, config, device)
    }

    /// Construit depuis des octets burnpack déjà en mémoire — c'est le chemin
    /// du navigateur, où JS fournit les octets.
    pub fn from_bytes(
        weights: &[u8],
        tokenizer: Tokenizer,
        template: ChatTemplate,
        config: QwenConfig,
        device: BurnDevice,
    ) -> Result<Self, LlmError> {
        let mut device = device.or_role(crate::burn_device::BurnRole::Llm).resolve();
        // Précision de **calcul** (les poids, eux, sont fp16 dans le .bpk).
        // `configure` verrouille le périphérique à la première allocation ;
        // s'il est déjà pris, on n'y peut plus rien et ce n'est pas fatal.
        // **f32 par défaut, et ce n'est pas un oubli.** L'export fp16
        // d'onnx-community pour ce modèle est numériquement dégradé : mesuré,
        // il complète « The capital of France is » par « is is is » quand le
        // f32 rend « Paris. It is the largest city in Europe ». Le fp16 double
        // pourtant le débit et halve la mémoire — d'où la variable, pour le
        // jour où un export fp16 sain existera.
        if let Ok("f16") = std::env::var("RAG3WEAVER_QWEN_DTYPE").as_deref() {
            let _ = device.configure(
                burn::tensor::DeviceConfig::default()
                    .float_dtype(burn::tensor::FloatDType::F16),
            );
        }

        let graph = QwenGraph::from_bytes(
            burn::tensor::Bytes::from_bytes_vec(weights.to_vec()),
            &device,
        );
        Ok(Self { graph, tokenizer, template, config, sampling: SamplingConfig::default(), device })
    }

    /// Remplace les réglages d'échantillonnage propres au backend.
    pub fn with_sampling(mut self, sampling: SamplingConfig) -> Self {
        self.sampling = sampling;
        self
    }

    pub fn config(&self) -> &QwenConfig {
        &self.config
    }

    /// Rend le chat template. Public parce que c'est le premier endroit qu'on
    /// veut inspecter quand un modèle local se comporte mal.
    pub fn render_prompt(&self, turns: &[Turn], opts: &GenOptions) -> Result<String, LlmError> {
        let input = RenderInput {
            messages: turns.iter().map(to_message).collect(),
            tools: opts.tools.iter().map(|t| t.to_openai_json()).collect(),
            add_generation_prompt: true,
            ..Default::default()
        };
        self.template
            .render(&input)
            .map_err(|e| LlmError::Prompt(format!("chat template: {e}")))
    }

    /// Un pas de graphe : `ids` entre, les logits du **dernier** jeton et le
    /// cache mis à jour sortent.
    fn step(
        &self,
        ids: &[u32],
        past: Vec<Tensor<4>>,
        ctx: usize,
    ) -> Result<(Tensor<2>, Vec<Tensor<4>>), LlmError> {
        let n = ids.len();
        let input = Tensor::<2, Int>::from_data(
            TensorData::new(ids.iter().map(|&i| i as i32).collect::<Vec<_>>(), [1, n]),
            &self.device,
        );
        let mask = Tensor::<2, Int>::ones([1, ctx + n], &self.device);
        let pos: Vec<i32> = (0..n).map(|i| (ctx + i) as i32).collect();
        let pids =
            Tensor::<2, Int>::from_data(TensorData::new(pos, [1, n]), &self.device);

        // `forward` prend son cache en **arguments positionnels** : c'est la
        // forme que burn-onnx génère, et un `Vec` ne s'y déplie pas. On draine
        // l'itérateur — l'ordre d'évaluation des arguments est de gauche à
        // droite, donc l'appariement couche/tenseur est celui du vecteur.
        let mut p = past.into_iter();
        let o = self.graph.forward(
            input,
            mask,
            pids,
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
            p.next().unwrap(),
        );
        debug_assert!(p.next().is_none(), "cache plus long que ce que le graphe attend");

        // Les logits restent **sur le GPU** : les rapatrier coûte 151 936
        // flottants et une synchronisation par jeton. En glouton — notre
        // défaut — un seul entier suffit (cf. `pick`).
        let last = o.0.slice(s![.., (n - 1)..n, ..]).squeeze_dim::<2>(1);
        Ok((last, vec![o.1, o.2, o.3, o.4, o.5, o.6, o.7, o.8, o.9, o.10, o.11, o.12, o.13, o.14, o.15, o.16, o.17, o.18, o.19, o.20, o.21, o.22, o.23, o.24, o.25, o.26, o.27, o.28, o.29, o.30, o.31, o.32, o.33, o.34, o.35, o.36, o.37, o.38, o.39, o.40, o.41, o.42, o.43, o.44, o.45, o.46, o.47, o.48]))
    }

    #[doc(hidden)]
    pub fn encode_for_test(&self, prompt: &str) -> Vec<u32> {
        self.tokenizer.encode(prompt, false).unwrap().get_ids().to_vec()
    }
    #[doc(hidden)]
    pub fn decode_for_test(&self, ids: &[u32]) -> String {
        self.tokenizer.decode(ids, false).unwrap()
    }
    #[doc(hidden)]
    pub fn prefill_logits_for_test(&self, ids: &[u32]) -> Vec<f32> {
        let (l, _) = self.step(ids, self.empty_cache(), 0).unwrap();
        self.logits_vec(&l).unwrap()
    }

    /// Choisit le prochain jeton à partir des logits **restés sur le GPU**.
    ///
    /// En glouton, l'argmax se fait côté GPU et un seul entier traverse le
    /// bus. Dès qu'on échantillonne, il faut la distribution entière : on
    /// paie le rapatriement, mais seulement là.
    fn pick(&self, logits: &Tensor<2>, opts: &GenOptions, rng: &mut u64) -> Result<u32, LlmError> {
        if opts.temperature <= 0.0 {
            let id = logits
                .clone()
                .argmax(1)
                .into_data()
                .to_vec::<i32>()
                .map_err(|e| LlmError::Model(format!("argmax: {e:?}")))?[0];
            return Ok(id as u32);
        }
        let v = self.logits_vec(logits)?;
        Ok(sample(&v, opts.temperature, opts.top_p, self.sampling.top_k, rng))
    }

    fn logits_vec(&self, logits: &Tensor<2>) -> Result<Vec<f32>, LlmError> {
        logits
            .clone()
            .into_data()
            .convert::<f32>()
            .to_vec::<f32>()
            .map_err(|e| LlmError::Model(format!("logits: {e:?}")))
    }

    fn empty_cache(&self) -> Vec<Tensor<4>> {
        (0..self.config.n_past())
            .map(|_| {
                Tensor::<4>::zeros(
                    [1, self.config.n_kv_heads, 0, self.config.head_dim],
                    &self.device,
                )
            })
            .collect()
    }
}

// ─── Turn → Message ──────────────────────────────────────────────────────────

/// Traduit un tour vers le type de message du moteur de template.
///
/// `tool_calls` et `tool_call_id` passent : c'est ce qui rend un historique
/// d'agent **rejouable**. Le template de Qwen2.5 fait
/// `tool_call.function.arguments | tojson`, donc `arguments` doit être un
/// **objet** JSON et non une chaîne — sinon `tojson` rendrait la chaîne
/// échappée et le modèle relirait `"{\"a\":1}"`. Si les arguments bruts ne
/// parsent pas (appel tronqué), on passe la chaîne telle quelle : le rendu est
/// dégradé, il n'échoue pas.
fn to_message(t: &Turn) -> Message {
    let mut m = Message::new(&t.role, &t.content);
    if !t.tool_calls.is_empty() {
        m.tool_calls = t
            .tool_calls
            .iter()
            .map(|c| {
                let args: Value = serde_json::from_str(&c.arguments)
                    .unwrap_or_else(|_| Value::String(c.arguments.clone()));
                json!({
                    "id": c.id,
                    "type": "function",
                    "function": { "name": c.name, "arguments": args },
                })
            })
            .collect();
    }
    if let Some(id) = &t.tool_call_id {
        m.extra.insert("tool_call_id".into(), json!(id));
    }
    if let Some(name) = &t.tool_name {
        m.extra.insert("name".into(), json!(name));
    }
    m
}

// ─── Parsing des appels d'outils (famille Qwen) ──────────────────────────────

/// Balises acceptées à l'ouverture. La première est celle du template ; les
/// autres sont les dérapages que llama.cpp tolère et qu'un 0,5 B produit
/// réellement.
const OPEN_TAGS: [&str; 3] = ["<tool_call>", "<tool call>", "<toolcall>"];
const CLOSE_TAGS: [&str; 3] = ["</tool_call>", "</tool call>", "</toolcall>"];

/// Retire une clôture markdown autour d'un bloc JSON (```` ```json … ``` ````).
fn strip_fence(s: &str) -> &str {
    let t = s.trim();
    let Some(rest) = t.strip_prefix("```") else { return t };
    // ```json\n… ou ```\n…
    let rest = rest.strip_prefix("json").unwrap_or(rest);
    let rest = rest.trim_start_matches(['\r', '\n']);
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

/// Extrait `name` et `arguments` d'un bloc, **même s'il est tronqué**.
///
/// Un appel coupé par `max_tokens` produit du JSON invalide, et il faut
/// pourtant pouvoir le refermer : son `id` doit exister, sinon l'appel reste
/// orphelin et la conversation n'est plus rejouable. On tente donc le parse
/// complet, puis à défaut une extraction textuelle. Sans `name`, ce n'est pas
/// un appel du tout et le texte reste visible.
fn parse_call_body(body: &str) -> Option<(String, String)> {
    let body = strip_fence(body);
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(body) {
        let name = map.get("name")?.as_str()?.to_string();
        let args = match map.get("arguments") {
            None | Some(Value::Null) => "{}".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
        };
        return Some((name, args));
    }
    // Tronqué : récupérer ce qui est récupérable.
    let key = body.find("\"name\"")?;
    let after = &body[key + 6..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let name = rest[..end].to_string();
    let args = match body.find("\"arguments\"") {
        Some(i) => {
            let a = &body[i + 11..];
            a.find(':').map(|c| a[c + 1..].trim().to_string()).unwrap_or_default()
        }
        None => String::new(),
    };
    Some((name, args))
}

/// Sépare la sortie brute en (texte visible, appels d'outils).
///
/// L'ordre des appels est celui du modèle — c'est ce que le trait promet.
pub fn parse_tool_calls(raw: &str) -> (String, Vec<(String, String)>) {
    let mut visible = String::new();
    let mut calls = Vec::new();
    let mut rest = raw;

    loop {
        let Some((at, open)) = OPEN_TAGS
            .iter()
            .filter_map(|t| rest.find(t).map(|i| (i, *t)))
            .min_by_key(|(i, _)| *i)
        else {
            visible.push_str(rest);
            break;
        };
        let head = &rest[..at];
        let after = &rest[at + open.len()..];
        let (body, tail) = match CLOSE_TAGS
            .iter()
            .filter_map(|t| after.find(t).map(|i| (i, t.len())))
            .min_by_key(|(i, _)| *i)
        {
            Some((end, len)) => (&after[..end], &after[end + len..]),
            // Pas de fermeture : tronqué, tout le reste est le corps.
            None => (after, ""),
        };
        match parse_call_body(body) {
            Some(call) => {
                visible.push_str(head);
                calls.push(call);
            }
            // Pas un appel : la balise reste du texte, verbatim.
            None => {
                visible.push_str(head);
                visible.push_str(open);
                visible.push_str(body);
            }
        }
        if tail.is_empty() {
            break;
        }
        rest = tail;
    }
    (visible.trim().to_string(), calls)
}

// ─── Échantillonnage ─────────────────────────────────────────────────────────

/// xorshift64*. Six lignes, déterministe, aucune dépendance — et il n'a
/// besoin d'être ni cryptographique ni brillant : il choisit un indice dans
/// une distribution déjà tronquée par top-k et top-p.
fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// Choisit le prochain jeton.
///
/// `temperature == 0` (notre défaut) rend l'argmax : reproductible, et c'est
/// la propriété qui compte dans un pipeline RAG. Au-delà, on applique
/// température, puis top-k, puis top-p, dans cet ordre — celui de llama.cpp.
pub fn sample(logits: &[f32], temperature: f32, top_p: f32, top_k: usize, rng: &mut u64) -> u32 {
    if temperature <= 0.0 {
        let mut best = 0usize;
        for (i, v) in logits.iter().enumerate() {
            if v > &logits[best] {
                best = i;
            }
        }
        return best as u32;
    }

    let mut idx: Vec<u32> = (0..logits.len() as u32).collect();
    idx.sort_unstable_by(|&a, &b| {
        logits[b as usize]
            .partial_cmp(&logits[a as usize])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if top_k > 0 && top_k < idx.len() {
        idx.truncate(top_k);
    }

    let max = logits[idx[0] as usize];
    let mut probs: Vec<f32> =
        idx.iter().map(|&i| ((logits[i as usize] - max) / temperature).exp()).collect();
    let sum: f32 = probs.iter().sum();
    for p in probs.iter_mut() {
        *p /= sum;
    }

    // top-p : garder le plus petit préfixe dont la masse atteint `top_p`.
    if (0.0..1.0).contains(&top_p) {
        let mut acc = 0.0f32;
        let mut keep = probs.len();
        for (i, p) in probs.iter().enumerate() {
            acc += *p;
            if acc >= top_p {
                keep = i + 1;
                break;
            }
        }
        probs.truncate(keep);
        idx.truncate(keep);
    }

    let total: f32 = probs.iter().sum();
    let mut r = (next_u64(rng) >> 11) as f32 / (1u64 << 53) as f32 * total;
    for (i, p) in probs.iter().enumerate() {
        r -= *p;
        if r <= 0.0 {
            return idx[i];
        }
    }
    idx[idx.len() - 1]
}

// ─── Le trait ────────────────────────────────────────────────────────────────

impl Llm for BurnLlm {
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(Finish, Usage), LlmError> {
        if turns.is_empty() {
            return Err(LlmError::Prompt("no turns".into()));
        }
        if let Some(t) = turns.iter().find(|t| t.role.is_empty()) {
            return Err(LlmError::Prompt(format!("turn with empty role: {:?}", t.content)));
        }
        let started = std::time::Instant::now();

        let prompt = self.render_prompt(turns, opts)?;
        let encoded = self
            .tokenizer
            .encode(prompt.as_str(), false)
            .map_err(|e| LlmError::Model(format!("encode: {e}")))?;
        let prompt_ids: Vec<u32> = encoded.get_ids().to_vec();
        if prompt_ids.len() >= self.config.context_len {
            return Err(LlmError::ContextOverflow {
                max: self.config.context_len,
                got: prompt_ids.len(),
            });
        }

        // Ce qui coupe l'émission : les séquences demandées **et** la balise
        // d'appel d'outil, qui ne doit jamais être poussée comme du texte.
        let mut cutters: Vec<String> = opts.stop.clone();
        if !opts.tools.is_empty() {
            cutters.extend(OPEN_TAGS.iter().map(|t| t.to_string()));
        }

        // ── Préremplissage ──────────────────────────────────────────────
        let (mut logits, mut past) = self.step(&prompt_ids, self.empty_cache(), 0)?;
        let mut ctx = prompt_ids.len();

        let mut rng = self.sampling.seed;
        let mut gen_ids: Vec<u32> = Vec::new();
        let mut emitted_bytes = 0usize;
        let mut fragments = 0usize;
        let mut suppressed = false;
        let mut reason = FinishReason::MaxTokens;
        let mut full = String::new();

        while gen_ids.len() < opts.max_tokens {
            let next = self.pick(&logits, opts, &mut rng)?;
            if self.config.eos_ids.contains(&next) {
                reason = FinishReason::Eos;
                break;
            }
            gen_ids.push(next);

            // Décoder **tout** le généré à chaque pas : un jeton BPE peut
            // porter un demi-caractère UTF-8, et seul le texte complet est
            // sûrement bien formé. Coût négligeable devant un pas de GPU.
            full = self
                .tokenizer
                .decode(&gen_ids, false)
                .map_err(|e| LlmError::Model(format!("decode: {e}")))?;

            if !suppressed && full.len() > emitted_bytes && full.is_char_boundary(emitted_bytes) {
                let pending = &full[emitted_bytes..];
                if let Some((at, seq)) = first_stop(pending, &opts.stop) {
                    if emit(sink, &mut fragments, &pending[..at]).is_err() {
                        reason = FinishReason::Cancelled;
                        emitted_bytes += at;
                        break;
                    }
                    emitted_bytes += at;
                    reason = FinishReason::Stop(seq);
                    break;
                }
                // La balise d'outil coupe l'émission sans arrêter la
                // génération : le modèle doit finir son appel.
                let tool_at = OPEN_TAGS
                    .iter()
                    .filter_map(|t| pending.find(t))
                    .min()
                    .filter(|_| !opts.tools.is_empty());
                let (out, advance) = match tool_at {
                    Some(at) => {
                        suppressed = true;
                        (&pending[..at], at)
                    }
                    None => {
                        let keep = holdback(pending, &cutters);
                        let cut = pending.len() - keep;
                        (&pending[..cut], cut)
                    }
                };
                if emit(sink, &mut fragments, out).is_err() {
                    reason = FinishReason::Cancelled;
                    emitted_bytes += advance;
                    break;
                }
                emitted_bytes += advance;
            }

            // Le pas suivant n'a besoin que du jeton qu'on vient de produire :
            // tout le contexte est déjà dans le cache. C'est là qu'est le
            // facteur 30 — sans cette ligne, il faudrait repasser tout le
            // prompt à chaque jeton.
            if gen_ids.len() >= opts.max_tokens {
                break; // `reason` vaut déjà MaxTokens : pas de passe inutile
            }
            let (next_logits, next_past) = self.step(&[next], past, ctx)?;
            logits = next_logits;
            past = next_past;
            ctx += 1;
        }

        // Reste à pousser : ce que la rétention gardait, si rien ne l'a coupé.
        if matches!(reason, FinishReason::Eos | FinishReason::MaxTokens)
            && !suppressed
            && full.len() > emitted_bytes
            && full.is_char_boundary(emitted_bytes)
        {
            let tail = full[emitted_bytes..].to_string();
            if let Some((at, seq)) = first_stop(&tail, &opts.stop) {
                let _ = emit(sink, &mut fragments, &tail[..at]);
                reason = FinishReason::Stop(seq);
            } else if emit(sink, &mut fragments, &tail).is_err() {
                reason = FinishReason::Cancelled;
            }
        }

        let (_, raw_calls) = if opts.tools.is_empty() {
            (String::new(), Vec::new())
        } else {
            parse_tool_calls(&full)
        };
        let context = format!("{}|{}", prompt.len(), gen_ids.len());
        let calls: Vec<ToolCall> = raw_calls
            .iter()
            .enumerate()
            .map(|(i, (name, args))| ToolCall::local(&context, i, name, args))
            .collect();

        let finish = if !calls.is_empty() && matches!(reason, FinishReason::Eos) {
            Finish::tool_call(calls)
        } else {
            Finish::new(reason, calls)
        };
        // Exactement une fois, quel que soit le chemin de sortie.
        sink.on_finish(&finish);

        Ok((
            finish,
            Usage {
                prompt_tokens: prompt_ids.len(),
                completion_tokens: gen_ids.len(),
                ms: started.elapsed().as_millis() as u64,
                // Un modèle qui tourne ici ne réessaie rien et n'a pas de
                // cache de fournisseur : il n'y a pas de fournisseur.
                retries: 0, recovered_calls: 0, cached_prompt_tokens: 0 },
        ))
    }

    fn context_len(&self) -> usize {
        self.config.context_len
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parsing des appels d'outils ─────────────────────────────────────

    #[test]
    fn a_well_formed_call_is_extracted_and_hidden() {
        let raw = "Je cherche.\n<tool_call>\n{\"name\": \"search\", \"arguments\": {\"q\": \"rust\"}}\n</tool_call>";
        let (text, calls) = parse_tool_calls(raw);
        assert_eq!(text, "Je cherche.");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "search");
        assert_eq!(calls[0].1, r#"{"q":"rust"}"#);
    }

    #[test]
    fn alternate_tags_are_tolerated() {
        for (open, close) in [("<tool call>", "</tool call>"), ("<toolcall>", "</toolcall>")] {
            let raw = format!("{open}{{\"name\":\"f\",\"arguments\":{{}}}}{close}");
            let (text, calls) = parse_tool_calls(&raw);
            assert!(text.is_empty(), "{open} : {text:?}");
            assert_eq!(calls.len(), 1, "{open}");
            assert_eq!(calls[0].0, "f");
        }
    }

    #[test]
    fn a_markdown_fence_around_the_json_is_stripped() {
        let raw = "<tool_call>\n```json\n{\"name\":\"search\",\"arguments\":{\"q\":1}}\n```\n</tool_call>";
        let (_, calls) = parse_tool_calls(raw);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "search");
        assert_eq!(calls[0].1, r#"{"q":1}"#);
        // Sans le `json`, la clôture nue passe aussi.
        let raw = "<tool_call>```\n{\"name\":\"s\",\"arguments\":{}}\n```</tool_call>";
        assert_eq!(parse_tool_calls(raw).1.len(), 1);
    }

    #[test]
    fn two_calls_keep_the_model_order() {
        let raw = "<tool_call>{\"name\":\"a\",\"arguments\":{\"i\":1}}</tool_call>\
                   <tool_call>{\"name\":\"b\",\"arguments\":{\"i\":2}}</tool_call>";
        let (text, calls) = parse_tool_calls(raw);
        assert!(text.is_empty());
        assert_eq!(calls.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(calls[1].1, r#"{"i":2}"#);
    }

    #[test]
    fn plain_prose_yields_no_call() {
        let (text, calls) = parse_tool_calls("La réponse est 42.");
        assert_eq!(text, "La réponse est 42.");
        assert!(calls.is_empty());
    }

    #[test]
    fn a_truncated_call_is_still_recoverable() {
        // Coupé par max_tokens : ni JSON valide, ni balise fermante. Il faut
        // quand même un `name`, sinon l'appel reste orphelin.
        let raw = "<tool_call>\n{\"name\": \"search\", \"arguments\": {\"q\": \"ru";
        let (text, calls) = parse_tool_calls(raw);
        assert!(text.is_empty(), "{text:?}");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "search");
        assert!(calls[0].1.starts_with('{'), "{:?}", calls[0].1);
    }

    #[test]
    fn a_body_without_a_name_stays_visible_text() {
        // Pas d'appel fantôme : sans nom, ce n'est pas un appel.
        let raw = "<tool_call>{\"oops\": true}</tool_call>";
        let (text, calls) = parse_tool_calls(raw);
        assert!(calls.is_empty());
        assert!(text.contains("oops"), "{text:?}");

        let raw = "<tool_call>n'importe quoi</tool_call>";
        assert!(parse_tool_calls(raw).1.is_empty());
    }

    #[test]
    fn missing_arguments_default_to_an_empty_object() {
        let (_, calls) = parse_tool_calls("<tool_call>{\"name\":\"ping\"}</tool_call>");
        assert_eq!(calls[0].1, "{}");
        // Des arguments déjà sérialisés en chaîne sont pris tels quels.
        let (_, calls) =
            parse_tool_calls("<tool_call>{\"name\":\"p\",\"arguments\":\"{\\\"a\\\":1}\"}</tool_call>");
        assert_eq!(calls[0].1, r#"{"a":1}"#);
    }

    #[test]
    fn text_around_a_call_survives() {
        let raw = "avant <tool_call>{\"name\":\"a\",\"arguments\":{}}</tool_call> après";
        let (text, calls) = parse_tool_calls(raw);
        assert_eq!(calls.len(), 1);
        assert_eq!(text, "avant  après".trim());
    }

    // ── Échantillonnage ─────────────────────────────────────────────────

    #[test]
    fn temperature_zero_is_argmax_and_deterministic() {
        let logits = [0.1f32, 9.5, -3.0, 2.0];
        let mut rng = 1;
        for _ in 0..5 {
            assert_eq!(sample(&logits, 0.0, 1.0, 40, &mut rng), 1);
        }
        // Négatif = glouton aussi (pas de division par zéro déguisée).
        assert_eq!(sample(&logits, -1.0, 1.0, 0, &mut 7), 1);
    }

    #[test]
    fn top_k_of_one_is_argmax_whatever_the_temperature() {
        let logits = [1.0f32, 5.0, 4.9, 0.0];
        for seed in [1u64, 42, 9999] {
            let mut rng = seed;
            assert_eq!(sample(&logits, 2.0, 1.0, 1, &mut rng), 1);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_draw() {
        let logits = [1.0f32, 1.1, 0.9, 1.05, 0.95];
        let draws = |seed: u64| {
            let mut r = seed;
            (0..12).map(|_| sample(&logits, 1.0, 1.0, 0, &mut r)).collect::<Vec<_>>()
        };
        assert_eq!(draws(7), draws(7), "reproductible à graine égale");
        assert_ne!(draws(7), draws(8), "et la graine sert vraiment");
    }

    #[test]
    fn sampling_only_ever_returns_a_valid_index() {
        let logits = [0.5f32, 0.4, 0.3, 0.2, 0.1, 0.05];
        let mut r = 3;
        for _ in 0..300 {
            let t = sample(&logits, 0.8, 0.9, 3, &mut r);
            assert!((t as usize) < logits.len(), "index {t} hors bornes");
        }
    }

    #[test]
    fn top_p_narrows_the_pool() {
        // Une masse écrasante sur 0 : top_p bas ne doit laisser que lui.
        let logits = [20.0f32, 0.0, 0.0, 0.0];
        let mut r = 11;
        for _ in 0..50 {
            assert_eq!(sample(&logits, 1.0, 0.5, 0, &mut r), 0);
        }
    }

    // ── Rendu et cartographie des tours ─────────────────────────────────

    #[test]
    fn tool_calls_and_ids_reach_the_template_message() {
        let call = ToolCall::new("call_1", "search", r#"{"q":"rust"}"#);
        let m = to_message(&Turn::assistant_with_calls("", vec![call]));
        assert_eq!(m.role, "assistant");
        assert_eq!(m.tool_calls.len(), 1);
        // `arguments` doit être un OBJET : le template fait `| tojson`.
        assert!(m.tool_calls[0]["function"]["arguments"].is_object());
        assert_eq!(m.tool_calls[0]["function"]["name"], "search");
        assert_eq!(m.tool_calls[0]["id"], "call_1");

        let m = to_message(&Turn::tool_result("call_1", "search", "[]"));
        assert_eq!(m.role, "tool");
        assert_eq!(m.extra["tool_call_id"], "call_1");
        assert_eq!(m.extra["name"], "search");
    }

    #[test]
    fn truncated_arguments_degrade_instead_of_failing() {
        // JSON invalide (appel coupé) : on passe la chaîne, on n'échoue pas.
        let call = ToolCall::new("id", "f", "{\"a\":");
        let m = to_message(&Turn::assistant_with_calls("", vec![call]));
        assert!(m.tool_calls[0]["function"]["arguments"].is_string());
    }

    // ── Configuration ───────────────────────────────────────────────────

    #[test]
    fn config_geometry_matches_the_generated_graph() {
        let c = QwenConfig::default();
        assert_eq!(c.n_layers, 24);
        assert_eq!(c.n_past(), 48, "le graphe généré prend 48 tenseurs de cache");
        assert_eq!((c.n_kv_heads, c.head_dim), (2, 64));
        assert!(c.eos_ids.contains(&151_645), "<|im_end|>");
        // 96 Mio à 8 k jetons — le chiffre qui rend ce modèle embarquable.
        assert_eq!(c.kv_cache_bytes(8192) / (1024 * 1024), 96);
    }

    #[test]
    fn default_dir_follows_the_other_models() {
        let dir = BurnLlm::default_dir();
        assert!(
            dir.ends_with("qwen2.5-0.5b-instruct")
                || std::env::var("RAG3WEAVER_QWEN_DIR").is_ok()
        );
    }

    #[test]
    fn fences_are_stripped_but_plain_bodies_are_untouched() {
        assert_eq!(strip_fence("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_fence("  {\"a\":1}  "), "{\"a\":1}");
        assert_eq!(strip_fence("```\nx\n```"), "x");
    }
}
