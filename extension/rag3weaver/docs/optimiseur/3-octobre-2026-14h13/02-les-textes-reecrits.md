# Les textes réécrits : cinq envois courts, et de quoi répondre en revue

**3 octobre 2026, session « optimiseur ».** Suite de
[l'étude du ton](01-comment-on-ecrit-chez-tracel-ai.md). **Rien n'est parti.**
Ces textes sont des brouillons : c'est Lucie qui signe, elle les reprend à sa
main, et l'envoi attend son mot.

Décisions de Lucie (3 octobre) : on envoie les trois dépôts d'un coup, et on
déclare l'aide d'une IA dans chaque envoi.

## La forme retenue

| # | dépôt | envoi | contenu | branche locale |
|---|---|---|---|---|
| A | cubecl | une PR | Flex32 inscrit sur Vulkan (ex-PR 1) | `vulkan-register-flex32` |
| B | cubek | une PR | le masque d'attention lu avec son vrai pas de ligne (ex-PR 6) | `attention-mask-row-stride` |
| C | cubek | une PR | matmul : une sortie Flex32 accumule en f32 (ex-PR 3) | `matmul-flex32-accumulates-in-f32` |
| D | burn | une issue | deux défauts de Flex32 sur wgpu, et une question sur la flash attention | — |
| E | burn | une PR liée à D | trois commits : `try_cast`, `float_from_data`, le pool de locaux (ex-PR 4, 2, 7) | `flex32-fixes` |

Ce qui a changé depuis le 2 octobre, et pourquoi :

- **Les ex-PR 2 et 4 sont un seul correctif.** Sur leur `main`, rien ne
  produit de données étiquetées Flex32 : le défaut de `float_from_data` n'est
  atteignable qu'une fois `try_cast` corrigé, et `try_cast` corrigé seul le
  fait paniquer. Vérifié dans les trois états.
- **L'ex-PR 5 (flash attention) ne part pas.** Elle devient une question à la
  fin de l'issue D.
- **B et C restent deux PR** : deux sujets sans rapport, et leur règle est
  « one PR should address one concern ».
- **E regroupe trois commits** sous un seul sujet (Flex32 utilisable sur
  wgpu), comme l'a fait burn#3551. Si un mainteneur préfère deux PR, la
  branche se coupe en deux sans rien réécrire.

## La ligne de déclaration

Elle ne doit affirmer que ce qui est vrai le jour de l'envoi. Version
d'aujourd'hui, **avant** que Lucie ait relu les correctifs :

> AI disclosure: I found these while running embedding models in Flex32 on
> AMD GPUs with Vulkan, and the changes were written with Claude Code. Each
> fix comes with a test that fails without it; I ran them on a Radeon 8060S.
> I am not a specialist of burn's internals, so tell me if a fix sits at the
> wrong level.

Si elle a lu les fiches et les diffs avant l'envoi, la dernière phrase peut
devenir : « I went through each change and can explain it. »

---

## A. cubecl — `fix(wgpu): register Flex32 on the Vulkan backend`

> Is Flex32 left out of the Vulkan backend on purpose?
>
> The WGSL backend registers `FloatKind::Flex32`, `cubecl-spirv` emits it as a
> 32-bit float, and `tests_spirv` generates its tests for `flex32`. But
> `register_types` in `backend/vulkan.rs` does not list it, so a device that
> compiles through SPIR-V answers "unsupported" for a type the WGSL path
> accepts on the same adapter.
>
> If it is an oversight, this PR adds the one line. If it is on purpose, I
> would like to understand why, and I will close this.
>
> ## Change
>
> `ElemType::Float(FloatKind::Flex32)` joins `default_types` in
> `register_types`.
>
> ## Testing
>
> New `flex32_is_a_registered_type` in `tests_spirv`. It fails on `main` and
> passes with this change.
>
> I ran that test only, not the whole cubecl suite.
>
> ## Validate your PR with burn
>
> burn has one test that states the opposite:
> `should_support_vulkan_dtypes` in `crates/burn-wgpu/src/lib.rs` asserts
> `!supports_dtype(Flex32)`. It will need flipping when the cubecl revision is
> bumped.
>
> I did not open a companion PR. I ran burn locally against this commit on
> top of the cubecl revision burn pins (`36314551`), with that assertion
> flipped:
>
> | burn `8942d406`, Vulkan, `--features vulkan,std,fusion --test tensor` | passed | failed |
> |---|---|---|
> | as pinned | 1638 | 5 |
> | with this change | 1638 | 5 |
>
> The 5 failures are the same before and after (`test_mean_dims_2d` and four
> `remainder` tests) and are not related to this change.
>
> `cargo test -p burn-wgpu --features vulkan --lib`: 2 passed with the
> assertion flipped, `should_support_vulkan_dtypes` fails without. Rust 1.99.0.
>
> I left the cubek boxes empty: this adds a type to a list, and cubek builds
> unchanged against it in the run above.
>
> Linux, AMD Radeon 8060S, RADV.
>
> ⟦ligne de déclaration⟧

## B. cubek — `fix(attention): read the materialized mask with its own row stride`

> With a padding mask broadcast over `seq_q`, the accelerated attention
> returns a wrong result: cosine 0.04 against `attention_fallback` in f16.
>
> A padding mask usually comes as `[batch, 1, 1, seq_kv]` expanded to
> `[batch, heads, seq_q, seq_kv]`, so its stride on `seq_q` is 0. The
> materialized mask reader steps rows by `seq_kv` (the field has a
> `TODO not sure if mandatory`), which is only right for a contiguous mask.
> Every row after the first is read from another batch's row, or past the end
> of the buffer.
>
> Same family as tracel-ai/burn#4778, which #414 fixed for the batch and head
> axes. This is the `seq_q` axis.
>
> ## Fix
>
> The reader takes the tensor's `stride(2)`. It equals `seq_kv` for a
> contiguous mask and 0 for a mask broadcast over `seq_q`.
>
> ## Tests
>
> `padding_mask_broadcast_over_seq_q_unit` and `…_blackbox`: a mask with
> strides `[seq_kv, 0, 0, 1]`, compared with the same mask made contiguous.
> Both fail on `main` and pass with this change.
>
> ## Validate your PR with burn
>
> No companion PR. burn run locally against this branch and my other cubek
> PR (⟦B ou C⟧) together:
>
> | burn `8942d406`, Vulkan, `--features vulkan,std,fusion --test tensor` | passed | failed |
> |---|---|---|
> | as pinned | 1638 | 5 |
> | with this change | 1638 | 5 |
>
> The 5 failures are the same before and after (`test_mean_dims_2d` and four
> `remainder` tests) and are not related to this change.
>
> Rust 1.99.0.
>
> Linux, AMD Radeon 8060S, RADV.
>
> ⟦ligne de déclaration⟧

## C. cubek — `fix(matmul): a Flex32 output accumulates in f32, like f16 and bf16`

> A matmul with Flex32 operands never reaches the cooperative-matrix
> routines: autotune drops every `cmma` candidate.
>
> `MatmulElems::from_globals` promotes the accumulator to f32 when the output
> is f16 or bf16, but keeps it as is for Flex32. `adjust_dtypes` then lowers
> the Flex32 stage and register types to f16, and no cooperative-matrix
> configuration takes f16 operands with a Flex32 accumulator.
>
> `accumulator_dtype` in cubek-pool (#639) already promotes f16, bf16 and
> flex32 together. This does the same for matmul.
>
> ## Fix
>
> One line: Flex32 joins f16 and bf16 in the condition.
>
> ## Tests
>
> `narrow_outputs_accumulate_in_f32` in `definition/elems.rs`. It fails on
> `main` for Flex32 (`stage for Float(Flex32)`) and passes with this change.
> `cargo test -p cubek-matmul --lib`: 42 passed.
>
> It only matters once Flex32 is registered on Vulkan (tracel-ai/cubecl#⟦A⟧).
> With both, on burn 0.22.0-pre.3 and an RDNA4 card, a 24-layer encoder went
> from 3 800 to 11 500 tokens/s in September. I have not measured it again on
> `main`.
>
> ## Validate your PR with burn
>
> No companion PR. burn run locally against this branch and my other cubek
> PR (⟦B ou C⟧) together:
>
> | burn `8942d406`, Vulkan, `--features vulkan,std,fusion --test tensor` | passed | failed |
> |---|---|---|
> | as pinned | 1638 | 5 |
> | with this change | 1638 | 5 |
>
> The 5 failures are the same before and after (`test_mean_dims_2d` and four
> `remainder` tests) and are not related to this change.
>
> Rust 1.99.0.
>
> Linux, AMD Radeon 8060S, RADV.
>
> ⟦ligne de déclaration⟧

## D. burn — issue : `Flex32 on wgpu: explicit dtype dropped by from_data, wrong values in fused kernels`

> **Describe the bug**
>
> Two things I ran into while running encoder models in Flex32 on wgpu. Both
> reproduce on `main` with the default WGSL compiler.
>
> **1. `from_data` with an explicit Flex32 dtype returns an F32 tensor.**
>
> ```rust
> let t = Tensor::<1>::from_data([1.0], (&device, DType::Flex32));
> assert_eq!(t.dtype(), DType::Flex32); // fails: F32
> ```
>
> `TensorData::try_cast(DType::Flex32)` tags its result F32, because Flex32
> shares f32's element type. `FloatCastAdapter` goes through the same cast, so
> loading a record "as Flex32" silently keeps everything in f32.
>
> **2. A fused kernel that mixes F32 and Flex32 locals returns wrong values.**
>
> ```rust
> let y = Tensor::<2>::from_data([[1.0, 2.0, 3.0, 4.0]], &device).cast(DType::Flex32);
> let gate = Tensor::<2>::from_data([[-3.0, 0.0, 1.5, 3.0]], &device).cast(DType::Flex32);
> let out = y.mul(hard_sigmoid(gate, 0.2, 0.5));
> // expected [0, 1, 2.4, 4], got [0, 0.25, 0.64, 1.1] with fusion on
> ```
>
> `hard_sigmoid` computes in F32. The generated kernel keeps F32 and Flex32
> block locals in the same `l_f32` registry, but `LocalVariablePool` numbers
> each precision on its own, so the F32 local takes the position of the
> Flex32 local that holds `y`.
>
> **Expected behavior**
>
> The requested dtype is kept, and the fused result matches the unfused one.
>
> **Environment**
>
> burn `main` at `8942d406`, Linux, AMD Radeon 8060S (RADV, Mesa 26.2.3), wgpu, Rust 1.99.0.
>
> **Fix**
>
> I have a PR for both: ⟦lien⟧.
>
> **A question on the side**
>
> On Vulkan, `attention` on Flex32 inputs always ends in
> `attention_fallback`: the blackbox accelerated routine wants the global type
> to be the tile type and returns `InvalidConfig`, and the autotune candidates
> degrade the same way. On my RDNA4 card, at `[256, 12, 512, 64]`, the
> fallback's score matrix is 3 GiB and the launch fails. (Flex32 is not registered on Vulkan today; I
> opened tracel-ai/cubecl#⟦A⟧ for that.) Casting Q, K and
> V to f16 around the accelerated call works for me, but it looks like
> something you may prefer to solve inside cubek. Would you take a PR for
> this, and at which level?
>
> ⟦ligne de déclaration⟧

## E. burn — `fix(flex32): keep an explicit Flex32 dtype, and number F32 and Flex32 fused locals together`

> ### Checklist
>
> - [x] Confirmed that `cargo run-checks` has been executed.
> - [x] Made sure the book is up to date with changes in this PR. (No book change needed.)
>
> ### Related Issues/PRs
>
> Fixes ⟦issue D⟧. Earlier work on the same type: #3551.
>
> ### Changes
>
> Three small commits.
>
> - `TensorData::try_cast` re-tags its result when the requested dtype is
>   Flex32. Before, a cast to Flex32 came back as F32.
> - `float_from_data` in burn-cubecl accepts Flex32 data. It is stored as f32,
>   so it uploads like F32. Without this, the first commit alone makes
>   `from_data` panic with `Unsupported dtype`.
> - `LocalVariablePool` in burn-cubecl-fusion is keyed by storage precision
>   (Flex32 maps to F32) and keeps the declared precision next to each
>   position, so two locals of the shared `l_f32` registry can't get the same
>   position.
>
> ### Testing
>
> - Flex32 added to the loop of
>   `explicit_dtype_is_preserved_after_default_dtype_is_locked`. On `main` it
>   fails with `left: F32, right: Flex32`; with the first commit alone it
>   panics in `float_from_data`; with both it passes.
> - New `try_cast_to_flex32_keeps_the_flex32_dtype` in burn-std.
> - New `fusion_flex32_mul_by_hard_sigmoid_matches_reference`. On `main` it
>   returns 0.25 where 1 is expected; it passes with the third commit.
> - Two unit tests on the pool in `trace/block.rs`.
>
> - `cargo run-checks` (Flex): exit 0, 19 `test result: ok` lines, 8 min.
> - `cargo clippy -p burn-cubecl-fusion -p burn-cubecl -p burn-std --all-targets -- -D warnings`: clean.
> - `cargo test -p burn-backend-tests --no-default-features --features wgpu,std,fusion --test tensor`
>   for the tests above.
> - Same suite with `--features vulkan,std,fusion`: 1638 passed and 5 failed
>   on `main`, 1639 passed and the same 5 failed with this branch
>   (`test_mean_dims_2d` and four `remainder` tests, not related). On Vulkan
>   the two Flex32 tests are skipped today, since Flex32 is not registered
>   there.
> - Rust 1.99.0.
>
> Linux, AMD Radeon 8060S (RADV, Mesa 26.2.3).
>
> ⟦ligne de déclaration⟧

---

## Les questions probables d'un mainteneur

Ce sont des appuis pour que Lucie réponde avec ses mots, pas des réponses à
coller : chez eux, des réponses collées sont ce qui ferme une PR (burn#3383).

1. **« Pourquoi Flex32 sur Vulkan, alors que f16 y est natif ? »** (A). Parce
   que Flex32 garde un stockage et une accumulation en f32 : le modèle se
   charge sans convertir ses poids, et seuls les produits matriciels
   descendent en f16. Si leur réponse est « sur Vulkan, prenez f16 »,
   c'est un choix de conception à entendre : on ferme A, et C tombe avec.
2. **« Est-ce encore là sur `main` ? »** Oui : chaque test a été joué rouge
   sur leur `main` du 1er octobre (burn `8942d406`, cubek `fbed329f`, cubecl
   `a6321cd4`), et les correctifs s'appliquent sur celui du 3 octobre.
3. **« Pourquoi re-étiqueter dans `try_cast` plutôt que de donner à Flex32 son
   propre type d'élément ? »** (E). Parce que c'est le plus petit changement
   qui rend l'API vraie ; un type d'élément `flex32` côté burn-std serait plus
   propre et plus large. S'ils préfèrent cette voie, le test reste valable.
4. **« `into_data()` d'un tenseur Flex32 rend des données F32 : voulu ? »**
   Nous ne l'avons pas touché. C'est cohérent avec le stockage, mais cela
   veut dire qu'un aller-retour perd l'étiquette. À signaler s'ils posent la
   question, sans le défendre.
5. **« Le pool : pourquoi ne pas séparer les registres plutôt que fondre la
   numérotation ? »** (E). Séparer demanderait de changer la génération
   (`codegen/io.rs`) ; fondre la numérotation ne touche que le pool. Les deux
   corrigent le test.

## Une fiche par correctif

### Flex32 inscrit sur Vulkan (A)

- **Le défaut.** cubecl a deux compilateurs pour wgpu : WGSL et SPIR-V
  (Vulkan). Chacun déclare la liste des types qu'il sait compiler. WGSL
  déclare Flex32 ; SPIR-V sait l'émettre mais ne le déclare pas.
- **Le correctif.** Une ligne dans `register_types` de
  `crates/cubecl-wgpu/src/backend/vulkan.rs`.
- **Le test.** Demande au runtime SPIR-V si Flex32 est inscrit.
- **Le point faible.** On ne sait pas si l'absence est voulue ; burn a un test
  qui l'affirme. D'où la question en première phrase.

### Le masque d'attention (B)

- **Le défaut.** Un masque de remplissage a la forme `[lot, 1, 1, seq_kv]`,
  « étiré » sans copie à `[lot, têtes, seq_q, seq_kv]` : en mémoire il n'y a
  qu'une ligne par lot, et le pas pour passer d'une ligne `seq_q` à la
  suivante vaut 0. Le lecteur avançait de `seq_kv` à chaque ligne, comme si
  le masque était plein : il lisait les lignes des autres lots.
- **Le correctif.** Lire le pas réel du tenseur (`stride(2)`).
- **Le test.** Le même masque, étiré et recopié plein, doit donner le même
  résultat.
- **Le point faible.** Aucun connu ; c'est le plus solide des cinq.

### L'accumulation du matmul (C)

- **Le défaut.** Un produit matriciel en demi-précision additionne dans un
  accumulateur f32 pour ne pas perdre de chiffres. La règle listait f16 et
  bf16, pas Flex32 : aucune configuration des « matrices coopératives » du
  GPU n'acceptait alors la combinaison, et le matmul retombait sur la voie
  lente.
- **Le correctif.** Flex32 ajouté à la condition, une ligne dans
  `crates/cubek-matmul/src/definition/elems.rs`.
- **Le test.** Vérifie les types d'accumulation choisis pour f16, bf16 et
  Flex32.
- **Le point faible.** N'a de sens que si A est accepté.

### `try_cast` et `float_from_data` (E, commits 1 et 2)

- **Le défaut.** Flex32 n'a pas de type Rust à lui : il emprunte `f32`. Les
  conversions étiquettent leur résultat d'après le type Rust, donc « F32 ».
  Demander Flex32 rend F32, sans erreur.
- **Le correctif.** `try_cast` remet l'étiquette demandée ; et le backend
  accepte alors ces données (il les refusait faute de les avoir jamais vues).
- **Le test.** Créer un tenseur en demandant Flex32, vérifier qu'il l'est.
- **Le point faible.** La question 3 ci-dessus.

### Le pool de locaux (E, commit 3)

- **Le défaut.** La « fusion » colle plusieurs opérations en un seul noyau
  GPU ; les valeurs intermédiaires y vivent dans des cases numérotées. F32 et
  Flex32 partagent le même tableau de cases, mais chacun était numéroté de
  son côté à partir de 0 : deux valeurs pouvaient recevoir la même case, et
  la seconde écrasait la première.
- **Le correctif.** Une seule numérotation pour les deux.
- **Le test.** `y * hard_sigmoid(gate)` sur quatre valeurs : 0,25 au lieu de
  1 avant, juste après.
- **Le point faible.** La question 5 ci-dessus.

## État des vérifications

Toutes jouées le 3 octobre sur `lucie-tablette` (Radeon 8060S, RADV), les
dernières avec Rust 1.99.0, la `stable` de leur CI.

| envoi | branche, tête | rouge sans, vert avec | format | lint | validation burn sur Vulkan |
|---|---|---|---|---|---|
| A | cubecl `vulkan-register-flex32` `d27f6ed0` | oui (`flex32_is_a_registered_type`) | propre | clippy : 14 erreurs, toutes dans leur `cubecl-core/src/runtime_tests/unary.rs` (constantes dépréciées sous 1.99), aucune dans nos fichiers | 1638 / 5, comme la base ; le test Vulkan de burn échoue tant que son assertion n'est pas retournée |
| B | cubek `attention-mask-row-stride` `65bedd3c` | oui (deux tests) | propre | clippy propre | 1638 / 5, comme la base |
| C | cubek `matmul-flex32-accumulates-in-f32` `e20293ce` | oui ; 42 tests du crate | propre | clippy propre | jouée avec B, 1638 / 5 |
| E | burn `flex32-fixes` `ca25dd87` | oui : trois états pour `from_data`, 0,25 au lieu de 1 pour le pool | propre | clippy propre | `cargo run-checks` code 0 ; 1639 / 5 avec tout branché |

Ce qui n'a **pas** été fait, et que les textes disent :

- la suite complète de cubecl et celle de cubek (seuls nos tests et le crate
  `cubek-matmul`) ; `cargo xtask validate` de cubecl ;
- les PR compagnes que leur gabarit décrit (remplacées par le tableau local,
  ce que leurs mainteneurs acceptent pour une petite PR) ;
- une mesure de débit sur leur `main` : les chiffres cités datent de
  septembre, sur l'ancien poste (RDNA4) et burn 0.22.0-pre.3 ;
- les cinq échecs de leur `main` sur cet iGPU (`test_mean_dims_2d`, quatre
  tests `remainder`) ne sont pas de notre fait et ne sont pas signalés : une
  issue à part, plus tard, si Lucie le veut.

Les correctifs tels qu'ils partiraient sont dans `patches/` (un fichier par
envoi, `git am` sur la base nommée dans `patches/BASES.md`).
