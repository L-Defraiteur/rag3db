# Les sept PR amont : état de l'amont au 2 octobre 2026, textes prêts, rien d'envoyé

**2 octobre 2026, session « optimiseur ».** Lucie est d'accord pour proposer à
tracel-ai les sept correctifs que portent nos forks (chantier B du
[03 du 6 septembre](../6-septembre-2026-16h30/03-chantiers-ouverts.md)), à une
condition : vérifier d'abord qu'ils n'ont pas déjà corrigé. C'est fait ici,
sur leurs branches principales récupérées ce jour (burn `8942d406`, cubek
`fbed329f`, cubecl `a6321cd4`, toutes du 1er octobre ; une pre.4 est taguée sur
les trois dépôts). **Rien n'a été envoyé ni poussé**, ni vers tracel-ai ni vers
les forks : l'envoi attend son mot, sur la liste et sur l'identité.

Les branches sont locales, dans des clones du scratchpad de la session
(`<scratchpad>/amont/{burn,cubek,cubecl}`, donc perdues au redémarrage) : ce
document porte de quoi les refaire — chaque diff tient en quelques lignes, et
les quatre qui « s'appliquent proprement » sont les commits de nos forks.

## Le tableau

| # | PR | dépôt | défaut encore là sur leur `main` ? | issue / PR amont qui en parle | branche locale | état |
|---|---|---|---|---|---|---|
| 1 | Vulkan inscrit Flex32 | cubecl | oui (`register_types` de `vulkan.rs` ne liste pas Flex32, WGSL si) | aucune | `vulkan-register-flex32` `1bdde749` | s'applique proprement, 1 ligne |
| 2 | `float_from_data` accepte Flex32 | burn | oui (`unimplemented!` pour Flex32) | aucune | `cubecl-from-data-flex32` `5624daaa` | reprise à la main (contexte changé), 3 lignes |
| 3 | matmul : sortie Flex32 accumule en f32 | cubek | oui (`from_globals` ne promeut que f16 et bf16) | aucune ; cubecl#1053 (ouverte, 2025) demande une précision d'accumulation séparée, autre sujet | `matmul-flex32-accumulates-in-f32` `ec71d198` | s'applique proprement, 1 ligne |
| 4 | `try_cast(Flex32)` garde Flex32 | burn | oui : l'API a été réécrite (`try_cast`), le défaut est resté (`self.dtype = Target::dtype()`, Target = f32) | aucune ; burn#5387 (ouverte) touche la construction de TensorData, pas ce point | `tensordata-cast-keeps-flex32` `54e54c44` | **écrite aujourd'hui** (nous n'avions qu'un contournement), avec un test unitaire |
| 5 | la flash accélérée se lance en Flex32 | burn | oui (cubek refuse toujours « global ≠ tuile », burn dégrade toujours en silence) | aucune | `cubecl-flash-attention-on-flex32` `33187806` | **reprise à la main** (ils ont retiré le paramètre de runtime) |
| 6 | le masque matérialisé lu avec son vrai pas de ligne | cubek | oui (`seq_kv_shape` et son `TODO not sure if mandatory` sont toujours là) | burn#4778 (ouverte, avril) : même famille, **autre axe** (masque diffusé sur lots et têtes, corrigé par cubek#414 en juillet ; le test n'est plus sauté) ; notre axe (`seq_q`) n'est signalé nulle part | `attention-mask-row-stride` `a5fd2bfb` | s'applique proprement |
| 7 | fusion : locaux F32 et Flex32 numérotés ensemble | burn | oui (`LocalVariablePool` par précision, registre `l_f32` partagé) | aucune | `fusion-flex32-locals-share-f32-registry` `49bb1396` | s'applique proprement |

**Aucune des sept n'a été corrigée en amont.** Quatre s'appliquent telles
quelles, deux ont été reportées à la main, une a été écrite aujourd'hui.

## Ce qu'il faut avant d'envoyer

1. **Compiler, et prouver.** Aucune branche n'a été compilée contre leur `main`
   (le poste portait une livraison aux tests sensibles au temps). Les 1, 3, 6, 7
   viennent de commits testés chez nous sur la base pre.3 ; les 2, 4, 5 sont
   écrites à la main sur `main`. Le gabarit de PR de burn demande
   `cargo run-checks`. À faire au signal de l'orchestrateur : une branche à la
   fois, `target` à part, deux cœurs laissés libres ; pour 2, 4 et 5, le test
   qui reproduit, **rouge sans le correctif et vert avec**.
2. **L'identité.** Proposée par l'orchestrateur, à confirmer par Lucie :
   l'adresse personnelle (celle que signe le dépôt rag3db) et le compte `L-Defraiteur` (celui des forks). Les sept commits
   locaux sont à cette adresse, par un `git config user.email` **local** à
   chaque clone ; la configuration globale du poste (adresse professionnelle)
   n'a pas été touchée, ni le compte actif de `gh`. Le jour de l'envoi, le
   compte passe par commande, sans changer l'état du poste (le jeton du compte
   `L-Defraiteur` passé à `gh pr create` par la variable `GH_TOKEN`).
3. **Ce qui est déjà public sous l'adresse professionnelle.** Les six commits
   poussés sur les forks `L-Defraiteur/{cubecl,burn,cubek}`, branche
   `rag3weaver/pre.3`, sont signés de l'adresse professionnelle (auteur et
   committer) : cubecl `0565518f`, burn `669686ec`, `ee16daac`, `630c546c`,
   cubek `3062b6a1`, `e9821ceb` (6 et 18 septembre). Les corriger veut dire
   réécrire ces branches, les pousser en force et remonter les 36 révisions de
   `Cargo.toml` : à Lucie de dire si elle le veut.
4. **L'ordre d'envoi.** 1, 2, 3, 4 sont indépendantes et minuscules (Flex32
   utilisable de bout en bout sur Vulkan). **6 avant 5** : la 5 fait enfin
   tourner la flash en Flex32, et avec un masque de remplissage diffusé ses
   résultats sont faux tant que la 6 n'est pas fusionnée dans cubek et la
   révision de cubek remontée dans burn. La 7 est indépendante.
5. **Laissé hors de la 5, pour une issue à part.** Notre fork garde aussi la
   voie naïve en lice sous 256 Mio de scores (plus rapide sur les séquences
   courtes : 184 000 contre 106 000 jetons/s sur des lots de 100 jetons,
   granite-107m). C'est un changement de politique d'autotune, qu'ils viennent
   de refondre (burn#5822) : à proposer en issue avec nos mesures, pas dans la
   PR.
6. **À savoir pour notre montée en pre.4.** Leur `main` a rendu privés les
   champs de `TensorData` (burn#5838) : notre `Flex32Adapter`
   (`src/burn_device.rs`), qui ré-étiquette les octets en reconstruisant un
   `TensorData` marqué Flex32, sera à reporter. Il disparaît si la PR 4 est
   acceptée : `FloatCastAdapter` suffira.

## Une faute de cette session, pour mémoire

En écrivant ce document, un script passé au shell par un heredoc non protégé
a fait exécuter comme commandes les passages entre accents graves, dont une
ligne d'exemple `gh pr create`. Elle a échoué sur son argument avant toute
action. Vérifié aussitôt : aucune PR créée sur les sept dépôts concernés pour
aucun des deux comptes, compte actif de `gh` et identité git globale
inchangés. La règle : un texte qui contient des accents graves ne passe pas
par un heredoc non protégé ; on l'écrit par un fichier.

## Les textes

### 1. cubecl — `fix(wgpu): register Flex32 on the Vulkan backend`

The WGSL backend lists `FloatKind::Flex32` among its supported types, and
cubecl-spirv emits it as a 32-bit float, but `register_types` in the Vulkan
backend leaves it out.

**Reproduce**: on a wgpu device compiled through SPIR-V, set Flex32 as the
default float type (from burn: `device.configure(FloatDType::Flex32)`). It
fails with an unsupported dtype error; the same device accepts it through WGSL.

**Change**: one line, Flex32 added to `default_types`.

Tested on radv (RDNA4): BERT-style encoders run in Flex32 and match their f32
output (cosine 1.00000).

### 2. burn — `fix(cubecl): accept Flex32 data in float_from_data`

Flex32 is stored as f32, so its bytes upload like F32 ones, but
`float_from_data` only lists F64, F32, F16 and BF16.

**Reproduce**: create a float tensor on a cubecl backend from a `TensorData`
whose dtype is `Flex32` (for instance a record whose tensors were cast by a
store adapter). It panics with `Unsupported dtype for float_from_data`.

**Change**: `DType::Flex32` joins the accepted dtypes.

### 3. cubek — `fix(matmul): a Flex32 output accumulates in f32, like f16 and bf16`

`MatmulElems::from_globals` promotes the accumulator to f32 for f16 and bf16
outputs, but keeps it as-is for Flex32. `adjust_dtypes` then lowers the Flex32
stage and register types to f16 for the accelerated routines, and no
cooperative-matrix configuration takes f16 operands with a Flex32 accumulator.

**Reproduce**: launch a matmul with Flex32 operands and output on a device
with f16 cooperative matrices and look at the autotune log: every `cmma`
candidate is dropped, the matmul never reaches the tensor cores.

**Change**: one line, Flex32 joins f16 and bf16 in the condition.

Measured on RDNA4 / Vulkan with a 24-layer encoder: 3 800 → 11 500 tokens/s,
cosine 0.999999 against f32.

### 4. burn — `fix(tensor): TensorData::try_cast to Flex32 keeps the Flex32 dtype`

Flex32 shares f32's storage, and the conversion helpers tag their result with
the element type's dtype (`self.dtype = Target::dtype()`, with `Target = f32`
for both F32 and Flex32). Casting to Flex32 therefore returns data tagged F32.

**Reproduce**:

```rust
let data = TensorData::from([1.0f32, 2.0]).try_cast(DType::Flex32).unwrap();
assert_eq!(data.dtype(), DType::Flex32); // fails: F32
```

The consequence is silent: `convert_dtype(DType::Flex32)`, and
`FloatCastAdapter` built on it, do nothing for a Flex32 target. The weights
stay F32 and the whole model computes in f32.

**Change**: `try_cast` re-tags the result when the requested dtype is Flex32.
A unit test covers the in-place path, the cloning path, and the way back.

### 5. burn — `fix(cubecl): launch the accelerated flash attention on Flex32 inputs`

*(to open after the cubek mask fix below is merged and the cubek revision
bumped: with a broadcast padding mask, the flash result is wrong until then)*

The blackbox accelerated routine requires the global type to be the tile type
(f16) and rejects Flex32 with `Query global and tile types must be the same
because no stage to cast in between`. `attention` degrades that
`InvalidConfig` to the fallback, including inside the autotune candidates.

**Reproduce**: call `attention` on Flex32 tensors of shape
`[128, 12, 512, 64]` with the autotune log on. The three
`blackbox_accelerated_*` candidates report the same time to 0.01 ms: all three
ran the fallback. At `[256, 12, 512, 64]` the fallback's score matrix is 3 GiB
and the launch fails with `failed to reserve 3221225472 bytes`.

**Change**: `flash_attention` casts Q, K and V to f16 for the accelerated
routine when the input is Flex32 and the shape can launch, and returns the
output as Flex32 — the precision Flex32 promises. The degradation now logs its
reason at debug level. A cast stage inside cubek would be the alternative; this
keeps the change on the burn side.

Measured on RDNA4 / Vulkan, 6-layer encoder, 128 × 512 tokens: 110 000 →
416 000 tokens/s; `[256, 12, 512, 64]` now runs. Flash against
`attention_fallback`: max absolute difference 8e-4.

### 6. cubek — `fix(attention): read the materialized mask with its own row stride`

The materialized mask reader strides tile rows by `seq_kv` (the field carries
a `TODO not sure if mandatory`). That is only right for a contiguous
`[batch, heads, seq_q, seq_kv]` mask. A padding mask usually comes as
`[batch, 1, 1, seq_kv]` broadcast to the full shape, with a stride of 0 on
`seq_q`: every row after the first is read from other batches' rows, or past
the end of the buffer.

**Reproduce** (f16, from burn):

```rust
let mask = pad_mask /* [4, 1, 1, 512], bool */ .expand([4, 16, 512, 512]);
let flash = attention(q.clone(), k.clone(), v.clone(), Some(mask.clone()), None, Default::default());
let reference = attention_fallback(q, k, v, Some(mask), None, Default::default());
// cosine(flash, reference) = 0.04
```

**Change**: the reader takes the tensor's `stride(2)`, which equals `seq_kv`
for a contiguous mask and 0 for a mask broadcast over `seq_q`. After the fix
the max absolute difference against the fallback is 8e-4.

Same family as tracel-ai/burn#4778 (mask broadcast over batch and heads, fixed
by #414); this is the remaining axis.

### 7. burn — `fix(cubecl-fusion): number F32 and Flex32 locals together, they share one registry`

The generated kernel keeps F32 and Flex32 block locals in the same `l_f32`
registry (`codegen/io.rs`), but `LocalVariablePool` numbers each declared
precision on its own. A Flex32 local and an F32 local can get the same
position and overwrite each other. It happens when a fused kernel casts Flex32
to F32 mid-chain: `hard_sigmoid` on a Flex32 tensor computes in F32.

**Reproduce**: a squeeze-and-excitation block on Flex32 tensors,
`y.mul(hard_sigmoid(gate))`. The fused block (debug log) reads:

```
Assign(Input(1, Flex32)   -> BlockLocal { pos: 2, ty: Flex32 })   // y
Add(.., Scalar(1, F32))   -> BlockLocal { pos: 2, ty: F32 }       // overwrites y
Mul(BlockLocal { pos: 2, ty: Flex32 }, ..)                        // reads the sum
```

The PP-OCRv6 detector rendered an empty probability map in Flex32, while every
operation taken alone, and every fused chain without a cast, was bit-exact.

**Change**: the pool is keyed by storage precision (Flex32 maps to F32) and
keeps the declared precision next to each position. After the fix the detector
matches f32 (cosine 1.00000).

## Les correctifs eux-mêmes

`patches/` porte les sept commits tels qu'ils seraient envoyés (un fichier
par PR, à appliquer par `git am` sur la branche principale du dépôt nommé) :
les clones du scratchpad ne survivent pas à un redémarrage, et trois de ces
correctifs (2, 4, 5) n'existent nulle part ailleurs.
