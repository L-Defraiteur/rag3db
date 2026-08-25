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

## 4. TTS / STT : les deux tombent, mais deux bugs de burn commandent

**Les mesures wgpu ont renversé le classement.** Tout avait d'abord été
chronométré sur ndarray ; sur le backend de production, un modèle exact devient
faux et un modèle lent devient largement temps réel.

| Modèle | wgpu (médiane) | RTF | ndarray | parité wgpu vs ORT |
|---|---|---|---|---|
| **NeMo FastConformer CTC** (5 s d'audio) | **93,6 ms** | 0,019 — **×53 le temps réel** | 1 190 ms | **1,000000** ✅ |
| **Kokoro** (2,3 s d'audio) | **515 ms** | 0,224 — ×4,5 | 42 700 ms (**×83**) | 0,964 ⚠️ |
| Whisper-tiny encodeur (30 s) | 2,75 ms | 0,0001 | 1 070 ms | 1,000000 ✅ |
| Moonshine-base encodeur (5 s) | 55,7 ms | 0,011 — ×90 | 770 ms | 1,000000 ✅ |
| **Zipformer streaming fr** (chunk 320 ms) | 115 ms | 0,360 — ×2,8 | 468 ms | **0,47** ❌ |

Compilation des noyaux : 0,4 à 2,1 s, payée une fois. **Aucun refus de dtype ni
d'opérateur** sur wgpu.

### Le Zipformer produit des valeurs fausses sur wgpu

Exact sur ndarray (**1,000000**), faux sur wgpu (**0,47** avec une entrée
réaliste — pire qu'avec une entrée nulle, donc ce n'est pas un cas dégénéré).
Déterministe (deux exécutions identiques bit à bit), aucun NaN. Bissection par
sondes sur les 18 appels du `forward` : `submodule1` (frontend convolutif) est
exact à 5,2e-6, **`submodule2` — le premier bloc d'encodeur — diverge déjà**
(corr 0,933) ; les 16 suivants ne font que propager. Opérateurs testés isolément
sur wgpu, **tous corrects** (`cumsum` 2D/3D, `gather`, `recip`, `repeat`,
`expand`, `arange`, `split_with_sizes`, `mean_dim`, `powf`, `matmul` 4D,
`softmax`) : c'est une combinaison ou un cas de forme, pas un opérateur cassé.
Et ce n'est pas le backend : NeMo et Moonshine sont à 1,000000 sur wgpu.

**Conséquence** : le seul candidat à streaming natif est **bloqué**. Son
architecture reste la bonne (35 tenseurs d'état explicites, décodeur k2
stateless de 9 nœuds, donc pas de cache KV) — c'est un bug amont à isoler, pas
un travail d'intégration.

### L'écart Kokoro : hypothèse retirée, cause encore ouverte

**Correction.** Une première passe avait désigné le `ConvTranspose1d` de l'iSTFT
finale comme coupable. **C'est faux et c'est retiré** : douze cas minimaux — dont
la configuration exacte du nœud (`kernel 20`, `stride 5`, **22** canaux → 1, aux
deux longueurs réelles), plus recouvrement nul, recouvrement maximal, sorties
multi-canaux — donnent tous **corr 1,000000** sur ndarray *et* wgpu. Le
`ConvTranspose1d` de burn est correct. L'hypothèse « normalisation de
recouvrement manquante » est réfutée par la mesure : le ratio burn/ORT point à
point a un écart-type de **0,326**, ce qu'un gain constant ne produirait pas.

Ce qui diverge, c'est **son entrée**. Sondes plus haut dans HiFi-GAN :

| sonde | corr burn vs ORT |
|---|---|
| `ups.0/ConvTranspose` (nœud 1854) | **1,000000** |
| **`ups.1/ConvTranspose` (nœud 2145)** | **0,997574** |
| `Exp` (magnitude) | 0,952435 |
| `Concat` (entrée iSTFT) | 0,913352 |

La cause primaire est **entre les nœuds 1854 et 2145** (`resblocks.0..2` et
`noise_res`) ; l'exponentielle en aval magnifie ensuite une erreur relative, ce
qui est cohérent avec une cause unique. Innocentés eux aussi par cas minimaux :
`Conv1d` dilatée (9 configurations), `Pow(x,2)` sur valeurs négatives,
l'activation Snake `x + sin(αx)²`, l'AdaIN manuelle. Toutes à 1,000000.

**Pas de contournement, et la corrélation reste 0,966** — l'annoncer autrement
serait inventer un chiffre.

### Le blocage de fond : aucune traçabilité ONNX → code généré

Les deux bugs numériques (Kokoro et Zipformer) butent sur **la même chose** :
il n'existe aucun lien entre les noms de nœuds ONNX et les variables du code
que burn-onnx génère. Vérifié : `conv1d50_out1` comparé aux **neuf** candidats
`resblocks.{0,1,2}/convs1.{0,1,2}/Conv` donne des corrélations de **−0,039 à
+0,043** — l'heuristique par ordre d'index est fausse. La bissection s'arrête
donc au grain qu'on peut apparier de façon fiable. **Émettre le nom du nœud
ONNX en commentaire du code généré débloquerait les deux** — c'est la demande
de fonctionnalité S8 du rapport amont.

### Le sifflement entendu à l'écoute : deux phénomènes, pas un

Lucie a signalé des sifflements dans **les deux** moteurs, plus marqués sur
burn. Il y a donc (A) un artefact de base présent jusque dans la référence
onnxruntime, et (B) le surcroît de burn.

**(B) est mesuré** : 3,65 % d'énergie au-dessus de 8 kHz contre 1,48 % pour la
référence sur la même phrase — **2,5×**, cohérent avec le SNR d'erreur de
10,7 dB.

**(A) n'est probablement pas dans notre pipeline.** Écartés par mesure : aucune
raie tonale dans aucun fichier ; F0 plausible (médiane 208 Hz, voix féminine) ;
l'index du vecteur de style n'est pas critique (les lignes `n−3` à `n` donnent
la même chose) ; l'écriture du WAV est correcte (`clip` puis ×32767, pics à
0,49 et 0,90). Le point décisif : **la référence anglaise connue-bonne
(`af_heart`) a *plus* d'énergie haute fréquence (4,44 %) que toutes nos sorties
françaises.** Explication plausible : l'iSTFTNet de Kokoro travaille à
`n_fft = 20` / hop 5, soit **11 bins pour couvrir 12 kHz** — une résolution très
grossière qui laisse un fond haute fréquence. Tranché à l'écoute par Lucie sur
un échantillon anglais de référence.

### Classement révisé

| | avant (ndarray) | après (wgpu) |
|---|---|---|
| STT n°1 | Zipformer streaming fr | **NeMo FastConformer CTC** — exact sur wgpu, ×53 le temps réel, multilingue français, et **décodable par le CTC déjà écrit pour PP-OCR** (`burn_ppocr.rs:699`) |
| STT n°2 | NeMo CTC | Zipformer fr — **bloqué** sur wgpu |
| TTS n°1 | Kokoro | **Kokoro** (inchangé) — ×4,5 le temps réel, voix `ff_siwis` |

Réserves sur NeMo : licence **CC-BY-4.0** (hors du triptyque Apache/MIT/BSD — à
trancher) et **non causal**, donc pseudo-streaming par fenêtres glissantes.

### Le G2P français : bon emballage, contenu insuffisant

`piper-plus-g2p` 0.4 (MIT) a exactement le profil voulu — **536 Ko, 24 crates,
aucun C/C++, `cargo check --target wasm32-unknown-unknown` passe**. Et **la
chaîne française complète est démontrée**, zéro Python : texte → G2P Rust → IPA →
ids Kokoro (**0 symbole hors vocabulaire**) → Kokoro sur wgpu → 2,25 s d'audio
en 496 ms.

Mais la qualité linguistique ne suit pas : **les nombres sont purement
supprimés** (« 21 h 30 » ne produit rien), un bug de table rend `y_vowel` en
toutes lettres au lieu du symbole `y` (une ligne à corriger), les consonnes
finales muettes sont prononcées (`vɛ̃ɡ` pour « vingt »), et **le mot « liaison »
n'apparaît pas une seule fois** dans `french.rs`.

### Les lexiques de prononciation : le dilemme est net

**Aucun lexique français permissif ne porte les catégories grammaticales** — or
elles sont indispensables aux liaisons (« les‿enfants » obligatoire, « et »
jamais, et « un savant‿anglais » ne dit pas la même chose que « un savant
anglais »). Licences vérifiées sur les fichiers `LICENSE`, pas sur des README :

| source | licence | entrées | catégories |
|---|---|---|---|
| `ipa-dict` fr_FR | **MIT** | 245 972 | ❌ |
| MFA `french_mfa` v3 | **CC BY 4.0** | 105 730 | ❌ |
| **Lexique 4.00** | CC BY-SA 4.0 | 189 863 | ✅ `Cgram`, genre, nombre, temps, lemme, **`Phono_IPA`** |
| GLÀFF | CC BY-SA 3.0 (la fiche HF disant `cc-by-3.0` est **erronée**) | 1,25 M formes | ✅ tags GRACE |
| Morphalou 3.1 | LGPL-LR — sa §5 impose que la ressource reste **remplaçable par l'utilisateur**, donc incompatible avec un `include_bytes!` | — | ✅ |
| espeak-ng, `phonemizer` | GPL-3.0 | — | inutilisable |
| MFA `french_prosodylab` | — | données corrompues (`vingt → v cinq`) | inutilisable |

Réserve honnête sur `ipa-dict` : la provenance des données françaises n'est
documentée nulle part et le README crédite Aspell (dont la version FR est GPL) ;
le risque porte sur la liste de mots, pas sur les transcriptions.

**Festvox : la réponse est négative, et c'était le piège pressenti.** Festival,
Festvox, Speech Tools et Flite sont bien sous licences X11/BSD autorisant
explicitement la vente — **mais aucun lexique français n'y est distribué**. Le
français de Festival vient de **LLiaPhon, qui est GPL**, et le lexique OALD est
restreint au non-commercial. « Festival est permissif » n'implique donc en rien
« le français dans Festival est permissif ». Reste l'outillage LTS
(`festvox/src/lts`, arbres CART via Wagon) — utilisable comme **outil de build
hors ligne**, mais il ne fournit aucune donnée.

**Blanchir une licence en entraînant des règles dessus : non.** Creative Commons
ne tranche pas (note officielle sur l'entraînement d'IA, mai 2025), mais
CC BY-SA 4.0 §4(b) dit explicitement qu'une base intégrant la ressource est du
« matériel adapté » au titre des droits *sui generis*. La voie propre est
d'apprendre depuis des données permissives.

**Chemin recommandé, en deux étages** — le premier ne dépend d'aucune donnée
externe et corrige le défaut le plus visible :

1. **Verbalisation des nombres, dates, heures et unités en Rust pur** — code
   original, MIT, aucune donnée tierce. 1 à 2 jours. C'est « 21 h 30 » qui
   redevient prononçable.
2. Puis, **au choix de Lucie sur critère juridique** : `ipa-dict` (MIT) plus une
   liste écrite à la main des ~200 mots-outils qui déclenchent 90 % des liaisons
   réelles — binaire **100 % permissif**, 1 à 2 semaines ; ou **Lexique 4.00**
   pour une qualité nettement supérieure en quelques jours, au prix de publier
   **la table dérivée** sous CC BY-SA avec attribution. Le code Rust, lui, n'est
   pas contaminé dans les deux cas.

**f16 : hors de portée** de burn-onnx 0.22.0-pre.1 — `half` non déclaré, puis 15
erreurs de type f16/f32 dans `interpolate` et `Conv1d`. Le `.bpk` fp16 ferait
pourtant exactement la moitié (270 contre 541 Mo).

À écrire nous-mêmes : log-mel 80 bandes (normalisation `per_feature` pour NeMo)
avec une FFT, et — quand le Zipformer sera débloqué — la recherche par faisceau
transducteur. **Aucune brique audio n'existe dans le crate** : ni FFT, ni mel,
ni lecture WAV. Vocodeur : rien à écrire, Kokoro sort la forme d'onde.

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

**Ordre revu le 25 après-midi, sur décision de Lucie** : « on devrait en avoir
un minimal dispo en local, mais bien se dire que les gens auront pas mes 64 Go
de VRAM — et **surtout** se faire un framework pour appeler des providers ».
Le **fournisseur devient le chemin normal**, le modèle local devient le cas
« zéro configuration, hors ligne ». Les fournisseurs passent donc devant.

| Étape | Livrable | j-h |
|---|---|---|
| ~~1~~ | ~~trait, mock, `LlmNode`, service~~ **fait** (`ad7db7f92`) | ~~2-3~~ |
| **2** | **Fournisseurs cloud** derrière le même trait : client SSE, Vertex AI / Gemini d'abord (Lucie a des crédits startup), tool calls normalisés, `Usage`. | 2-3 |
| 3 | `BurnLlm` paramétré : ONNX fp16 past-KV, tokenizer, chat template, sampling, boucle + EOS. **Génération locale réelle** — cible **Qwen2.5-0.5B fp16 (996 Mo)**, pas Luciole. | 3-4 |
| 4 | Streaming réel : mailbox luciole en port de flux, `ChannelSink`. | 2 |
| 5 | FFI wasm : `AsyncCallback` rappelable N fois, `PendingAsync` en file. **Jetons dans le navigateur.** | 2-3 |
| 6 | `llguidance` + tool calls : grammaire depuis `tools.rs`, `ToolNode`, boucle d'agent. | 2-3 |

### Le budget mémoire de l'utilisateur, pas le nôtre

La machine de dev a 32 Go de VRAM ; le produit ne peut pas en supposer autant.

| | poids | + cache 8k | rôle |
|---|---|---|---|
| **Qwen2.5-0.5B fp16** | **996 Mo** | ≈ 1,1 Go | **le local minimal** — tient sur un iGPU de portable |
| Luciole-1B fp16 | 2,46 Go | +384 Mio | option souveraine, *quand le fp16 marchera* |
| Luciole-1B fp32 | 6,33 Go | +384 Mio | ce qui tourne aujourd'hui — **pas un défaut acceptable** |

**Plancher dur** : en dessous de 0,5 Md, les modèles n'ont **pas de tool calls**
dans leur chat template (vérifié sur SmolLM2-360M, EuroMoE-2.6B, Baguettotron,
Ouro-1.4B, Motif-2.6B, Phi-tiny-MoE, ERNIE-4.5-0.3B). 0,5 B *est* le minimum
utile, ce n'est pas un choix de confort. La quantification q4 est ce qui
déplacerait ce plancher — et elle manque des deux côtés (§7).

Le trait livré à l'étape 1 est **déjà agnostique** : un flux SSE *pousse* des
fragments, ce qui est exactement la forme du puits. Le choix synchrone se paie
ici en avantage : lire un SSE ligne à ligne demande un client HTTP bloquant,
pas un runtime async.

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
