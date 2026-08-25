# Doc 47 — LLM / TTS / STT sur burn : ce qui est prouvé, ce qui est livré (25 août)

Chantier **4 bis** de l'ordre de Lucie (doc 41) : « LLM / TTS / STT sur burn en
streaming, interface substituable par un fournisseur cloud ». Quatre épreuves
menées en parallèle, **tout ce qui suit est mesuré**, pas déduit. Livré :
`898021ec2` (outils depuis les schémas), `ad7db7f92` (étape 1 du LLM).

Compagnons : [46 — OCR](46-ocr-unitaire-ppocrv6-sur-burn.md) ·
[36 — vision](36-vision-agents-comme-graphe-et-workflow.md) ·
[41](41-passation-progression-24-aout-nuit.md).

## 1. Le verdict qui change le plan : burn-onnx sait faire un décodeur

**Mon pronostic était faux.** Je pensais qu'un décodeur autorégressif à cache KV
était hors de portée de burn-onnx et qu'il faudrait écrire le module à la main
(façon `llama-burn`). C'est l'inverse :

```
GPT-2 124M      "The capital of France is" → "Paris"   (argmax 6342, vérifié)
Qwen2.5-0.5B    fp16, wgpu/Vulkan : prefill 4 077 j/s · décodage 25 j/s @ ctx 2048
                poids 996 Mo · cache KV 96 Mio @ 8k (GQA : 2 têtes KV)
contrôle        même graphe sans cache : ×30 plus lent. Le cache marche.
```

Cadeau : la passe de simplification de burn **refusionne l'attention toute
seule** — 24 appels au noyau flash `cubek-attention`, un seul `matmul` brut.
Ce qu'on aurait écrit à la main est déjà là.

**Le vrai blocage n'est pas burn, c'est la fourniture ONNX.** Les exports HF
récents utilisent les ops fusionnées `com.microsoft` d'ONNX Runtime GenAI —
`GroupQueryAttention`, `SkipSimplifiedLayerNormalization`,
`MultiHeadAttention`, `MatMulNBits` — **absentes** des 216 `NodeType` d'`onnx-ir`
→ panique. Éliminés de ce fait : `Qwen3-0.6B-ONNX`, `SmolLM2`, `Granite-4.1`,
`Ministral-3`. Le choix du modèle est donc contraint par l'existence d'un export
**décomposé**, pas par ses mérites — sauf à réexporter soi-même (§3).

Écarté : le module burn à la main (**+15 à 25 jours** pour rattraper ce qui
marche en une commande) ; le décodage dans le graphe via `Loop` (enfermerait le
sampling, les stops et l'annulation dans un graphe figé — la boucle doit rester
en Rust, c'est elle qui porte le streaming) ; un trait `async`
(`async-trait` est déclaré dans `Cargo.toml` et **utilisé nulle part** ;
l'introduire contaminerait `Node::execute` pour rien).

Aussi mesuré : **bf16 refusé** par RADV/Vulkan sur ce matériel (`Flex32` aussi) ;
f16 fait le travail (×2 en débit, ÷2 en mémoire). La pile de quantification de
burn est complète (`Q8F`…`Q2S`, jusqu'à BitNet 2 bits) mais **n'est pas
raccordée** à la sortie de burn-onnx, et `MatMulNBits` est absent : q4 est un
trou structurel des deux côtés.

## 2. Les libs : une seule, et une petite en prime

| | verdict |
|---|---|
| **`llguidance` 1.8.0** (MIT, guidance-ai) | **pris** — 25 à 48 crates, **zéro fichier C/C++**, wasm OK (notre `.cargo/config.toml` a déjà `getrandom_backend="wasm_js"`), lit un `tokenizer.json` HF. JSON Schema, regex, **grammaires Lark récursives**, préfixe forcé + reprise, `rollback`, **captures nommées**. En prod chez OpenAI, vLLM, llama.cpp, Chromium. |
| **`minijinja` 2.24 + contrib** (Apache-2.0) | **pris** — 2 à 8 crates, wasm sans flag. Les **4 vrais chat templates** (Qwen3, Llama 3.1, Mistral v0.3, Hermes-3) rendent avec une ligne de `pycompat`. En dur : ~800 lignes à resynchroniser à chaque modèle. |
| `outlines-core` | **refusé** — dépendance **non optionnelle** à `tokenizers features=["onig"]` → **94 fichiers C** (Oniguruma) + `esaxx-rs` C++, et duplique `tokenizers` (0.21 + 0.22). Ça annulerait exactement la précaution qu'on avait prise (`default-features = false, fancy-regex`). Plus : récursion tronquée à la profondeur 3, en silence. |
| `xgrammar` | **refusé** — cœur C++. On vient de sortir tout le C++ du chemin Rust. |

Mesuré sur le vocabulaire Qwen3 (151 669) : démarrage 280 ms / 170 Mo
(partagé), compilation d'une grammaire **0,44 ms** (schéma simple) à **1,3 ms**
(union de 27 outils), **200 générations sous schéma d'outil → 200 JSON valides**.
Masque : **3,3 µs** (enum + entier borné), 163 µs (chaîne libre) — le slicer vaut
×4,7. **28 à 45 % des jetons sont « fast-forward »** : forcés par la grammaire,
donc **sans passe forward du modèle**. Sur un tool call Qwen3,
`<tool_call>\n{"name":"get_weather","arguments":{"city":"` = 11 jetons gratuits.
Binaire wasm 1,19 Mo. Branchement : ~40 lignes, deux méthodes de `TokenizerEnv`.

**Trois pièges à ne pas repayer** :
- `SimpleVob::apply_to(&mut [f32])` écrit `0.0` sur les tokens **autorisés** :
  c'est un tableau de **biais** (`fill(-INF)` puis `apply_to`, puis `logits +=`),
  pas un masque. L'utiliser naïvement **inverse la contrainte**.
- `compute_ff_tokens()` rend `[]` **en silence** si `tokenize_is_canonical()`
  est faux (défaut d'`ApproximateTokEnv`) → 30-45 % de gain perdu sans message.
- Le `tojson` de minijinja **échappe en HTML** (`l'outil <a>`) :
  nos descriptions d'outils arriveraient corrompues au modèle. À remplacer.
  `raise_exception` et `strftime_now` sont à câbler aussi (~15 lignes).

Pas besoin de lib pour : le parsing des tool calls (les **captures nommées**
rendent `name` et `arguments` déjà séparés), le sampling (~80 lignes), les stops,
le SSE, et JSON Schema ← `NodeSchema` (§4).

## 3. Le modèle : `Luciole-1B` tourne

**`OpenLLM-France/Luciole-1B-Instruct-1.1`** (Apache-2.0, tool calls dans le
chat template) n'a **aucun ONNX public** — mais réexporté par nous, il passe :

```
prompt : « Quelle est la capitale de la France ? Réponds en une phrase. »
référence HF (torch fp32/fp16/bf16) : [6087, 344, 261] = 'Paris.<|im_end|>'
onnxruntime sur notre ONNX          : [6087, 344, 261]
burn-onnx 0.22.0-pre.1 (fp32)       : [6087, 344, 261]   ← identique
```

24 couches, 32 têtes Q / 8 KV, `head_dim` 64, vocab 128 000, `relu2`,
`partial_rotary_factor` 0.5 — **lus dans le config, jamais calculés** (piège :
`head_dim` ≠ `hidden/heads` sur plusieurs modèles). fp32 : **8 j/s**
(120-130 ms/jeton), cache KV fp16 384 Mio à 8 k.

**Correction honnête au balayage initial** : la model card dit que l'**instruct**
a été post-entraîné « almost entirely on English data » (c'est le **base** qui a
vu ~30 % de français), et sur des séquences de **16 384** jetons — le
`max_position_embeddings: 131072` est hérité du base. « Français en langue
première » et « 131 k de contexte » étaient trop forts. Le français reste
excellent en pratique.

**fp16 non résolu** : le même graphe converti en fp16 compile et tourne (48 ms/
jeton, ≈ 21 j/s) mais **diverge dès le premier jeton**. Deux hypothèses
éliminées par l'expérience (RoPE en fp32 : sortie inchangée ; valeur du masque
causal : la remonter aggrave). Suspect restant : LayerNorm décomposée et `relu2`
sans accumulation fp32. **Piste sérieuse** : le Qwen2.5 fp16 qui marche est un
export **natif** fp16, pas une conversion *a posteriori* — c'est probablement
toute la différence. À éprouver.

Recette de réexport (outil hors ligne, jamais une dépendance produit), les trois
points qui comptent : `attn_implementation="eager"` + `dtype=float32` (c'est ce
qui décompose), `torch.onnx.export(opset_version=16, dynamo=False)`, et
transformers 5 qui n'a plus `to_legacy_cache` (envelopper le modèle). Puis
4 rustines de codegen dans le `.rs` généré (rang 4 × rang 1 → `mul_scalar` ;
`as f64` sur `half::f16` ; deux `.clone()` sur des tenseurs déplacés).

**Pour l'étape 2** : partir de Qwen2.5-0.5B (prouvé, ONNX public, fp16 natif,
25 j/s) en **paramétrant la configuration** — les deux modèles ont la même
signature (`input_ids`, `attention_mask`, `position_ids`,
`past_key_values.{i}.{key,value}`) et le même nombre de couches ; seuls `KVH` et
le vocabulaire changent. Luciole devient alors une option, pas une réécriture.

## 4. TTS / STT : les deux tombent, le bloqueur est ailleurs

**STT — `sherpa-onnx-streaming-zipformer-fr`** (Apache-2.0) est le gagnant :
parité **1,000000** sur les trois graphes, **aucun patch ONNX**, et il est
*conçu* pour le streaming — **35 tenseurs d'état** entrent et sortent du graphe,
la boucle est `for chunk { (out, states) = enc(chunk, states) }`. Son décodeur k2
est « stateless » (9 nœuds) : **pas de cache KV**, donc pas le problème du LLM.
Whisper : encodeur ✅ parfait, **décodeur ❌** (nœud `If`, burn-onnx ne descend
pas les sous-graphes). Moonshine : encodeur ✅ et il prend **la forme d'onde
brute** (pas de mel à écrire), même mur sur le décodeur. NeMo FastConformer CTC
multilingue tourne (RTF 0,24) et **réutiliserait notre décodeur CTC de PP-OCR**,
mais il est CC-BY-4.0 et non causal.

**TTS — `Kokoro-82M`** (Apache-2.0) : le plus surprenant, **il tourne déjà de
bout en bout**, `STFT` et six `LSTM` compris, voix française `ff_siwis`. Piper
(MIT) échoue à l'exécution dans l'alignement VITS.

**Le bloqueur TTS n'est pas le modèle, c'est la phonémisation.** Kokoro et Piper
consomment de l'IPA/espeak, et le portage Rust d'espeak-ng est **GPL-3.0** —
hors doctrine. Seule piste propre : `piper-plus-g2p` (MIT, 8 langues dont le
français) — elle débloquerait **les deux d'un coup**. En cours d'épreuve.

À écrire nous-mêmes : fbank 80 mel (~150 lignes + une FFT), recherche par
faisceau transducteur (~300 lignes). Aucune dépendance audio n'existe
aujourd'hui dans le crate. Vocodeur : **rien** (Kokoro et Piper sortent la forme
d'onde).

**Chiffre manquant et décisif** : tout a été mesuré sur **ndarray**, jamais sur
wgpu (le backend de production), et ndarray est 30 à 180× plus lent
qu'onnxruntime ici. Le RTF de 1,46 du zipformer ne veut rien dire tant qu'on n'a
pas le chiffre wgpu. En cours.

## 5. Ce qui est livré

**`src/tools.rs`** (`898021ec2`) — les **28 nœuds du registre deviennent des
définitions d'outils OpenAI** sans une ligne écrite à la main. Le doc 36 dit
qu'un agent est un sous-graphe compilé en workflow ; la réciproque est ici : le
catalogue d'outils *est* le registre de nœuds. `additionalProperties: false`
(sans quoi une grammaire ne borne plus rien) et **tri par nom** (le registre est
une `HashMap` : un ordre instable changerait le prompt à chaque exécution et
ruinerait le cache de préfixes). Dette : `ConfigParamType::Json` devient un objet
libre, faute de sous-schéma déclaré.

**`src/llm.rs` + `src/dataflow/llm_nodes.rs`** (`ad7db7f92`, étape 1) — trait
`Llm`, `MockLlm`, `CallbackLlm`, `LlmNode`, `Catalog::set_llm`, 27 tests,
lib 648. **Zéro dépendance nouvelle.**

```rust
pub trait TokenSink: Send {
    fn on_token(&mut self, delta: &str) -> Flow;   // Flow::Stop annule
    fn on_finish(&mut self, _reason: &Finish) {}
}
pub trait Llm: Send + Sync {
    fn generate(&self, turns: &[Turn], opts: &GenOptions, sink: &mut dyn TokenSink)
        -> Result<(Finish, Usage), LlmError>;
    fn context_len(&self) -> usize;
    fn name(&self) -> &str { "llm" }
}
```

**Pourquoi un puits, et pas un itérateur ni un canal.** Un `Iterator` obligerait
à sortir le cache KV et l'état de sampling du corps de la boucle pour les stocker
dans la structure, et ne conviendrait pas à un fournisseur cloud dont le
transport *pousse*. Un canal nu perd l'annulation. Le puits donne les trois
propriétés qui comptent : **synchrone** (donc appelable depuis
`Node::execute(&mut self, ctx)` sans contaminer le dataflow d'async),
**annulable** (`Flow::Stop` remonte du consommateur au générateur, sans canal de
contrôle séparé), **substituable**. C'est aussi ce qui résout la dette « ports en
flux » du doc 36 : la boîte aux lettres luciole se branche derrière le puits,
**sans toucher au trait `Node`**.

Trois décisions prises en écrivant, et assumées :
- **`Finish::Cancelled` ≠ `Finish::Stop(seq)`** — une réponse abandonnée par le
  consommateur n'est pas une réponse terminée par un stop qu'on avait demandé.
  Une interface doit savoir laquelle elle affiche (`is_complete()`).
- **L'entrée `prompt` réutilise `PortType::Text`** : `OcrNode.text` →
  `LlmNode.prompt` se branche **sans adaptateur**. Un test l'épingle.
- **Une séquence de stop conserve le préfixe *verbatim***, espaces compris —
  rogner corromprait un bloc de code finissant par un retour à la ligne, et
  c'est ce que font llama.cpp et les API compatibles OpenAI.

## 6. La suite — 13 à 17 jours-homme

| Étape | Livrable | j-h |
|---|---|---|
| ~~1~~ | ~~trait, mock, `LlmNode`, service~~ **fait** (`ad7db7f92`) | ~~2-3~~ |
| 2 | `BurnLlm` paramétré : ONNX fp16 past-KV, tokenizer, chat template, sampling, boucle + EOS. **Génération réelle sur GPU.** | 3-4 |
| 3 | Streaming réel : mailbox luciole en port de flux, `ChannelSink`. | 2 |
| 4 | FFI wasm : `AsyncCallback` rappelable N fois, `PendingAsync` en file. **Jetons dans le navigateur.** | 2-3 |
| 5 | `OpenAiLlm` (SSE) derrière le même trait — puis ElevenLabs / Gradium. | 2 |
| 6 | `llguidance` + tool calls : grammaire depuis `tools.rs`, `ToolNode`, boucle d'agent. | 2-3 |

**Le point dur de l'étape 4** : le FFI wasm est **one-shot à trois endroits** —
`AsyncCallback` appelé exactement une fois sans drapeau terminal, `RETURN_BUF`
thread-local écrasé à chaque appel, et `PendingAsync` côté C++ avec un seul
`result` libéré par `asyncResult()` (un second callback écrirait dans de la
mémoire libérée). Le C ABI se prête *structurellement* au streaming (pointeur de
fonction + `user_data`), mais il faut **changer le contrat** : callback
rappelable N fois avec code de retour (0 = continue, ≠0 = annule — ce qui fait
remonter `Flow::Stop` du JavaScript jusqu'au GPU), file côté C++, et une
allocation par message.

## 7. Dettes nommées

- **fp16 de Luciole** diverge (§3) — éprouver l'export natif fp16.
- **Quantification q4** : absente des deux côtés (burn-onnx ne sort que f32/f16,
  `MatMulNBits` non supporté). C'est ce qui débloquerait les modèles plus gros.
- **Bugs de codegen burn-onnx 0.22.0-pre.1** à remonter en amont, tous
  contournés : `Tile` émet `alloc::vec::Vec` sans `extern crate alloc` ;
  sentinelle `INT64_MIN` typée `i32` ; `Expand` à shape dynamique qui panique ;
  `Range` à types mixtes ; `arange().cast()` qui garde le kind `Int` ;
  `GatherND` à const générique non inférable ; rang 4 × rang 1 ; `as f64` sur
  `half::f16` ; tenseurs déplacés puis réutilisés.
- **`.bpk` gonflés** : Kokoro 541 Mo pour un ONNX de 325 Mo, Luciole qui
  matérialise deux fois l'embedding liée (500 Mio à gagner). burn-onnx
  matérialise des constantes — à creuser.
- **Piper** : 4 rustines puis échec sémantique dans l'alignement VITS.
- **Écart Kokoro** (corr 0,966) : à qualifier (décalage de phase inaudible ou
  artefact réel) — en cours.
- **Aucune brique audio** dans le crate : ni FFT, ni mel, ni WAV.
