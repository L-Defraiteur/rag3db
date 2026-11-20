# Embarquement : la panique de granite à sa chauffe sur l'iGPU, départagée

Ticket `embarquement/tickets/2026-11-19-granite-278m-panique-a-sa-chauffe-sur-l-igpu.md`, demande de c7 du 19 novembre
(un départage borné : où ça casse, la plus petite reproduction, ce qui l'évite).

## Le départage

Tous les essais sont faits à luciepc le 19 novembre, sur `igpu:0` (`Device<Vulkan(IntegratedGpu(0))>`, un Intel Graphics ARL sous anv, Mesa 26.2.6 ; corrigé le 20 novembre, c'était écrit radv), avec
le binaire de master `91bd3ebd959ebedd`, Flex32, autotune et fusion actifs. Les journaux sont dans
`~/.cache/rag3weaver-build/igpu-panique/`.

1. **Par le service, sans chauffe** (`RAG3WEAVER_DEMON_CHAUFFE=0`), les 16 formes de la chauffe envoyées par `/embed`
   passent toutes (essai `i`). La chauffe réelle, elle, panique à chaque fois (essai `h`).
2. **La différence** : le service découpe chaque requête en lots de `lot_max` au plus (32 par défaut). La chauffe, elle,
   appelait l'embarqueur directement avec le lot conseillé par le modèle : 128 pour granite.
3. **Le modèle chargé dans le processus, sans service** (deux appels d'`embed` successifs) :

| Suite d'appels (mots × lot ; 90 mots = 182 jetons, 190 mots = 382) | Résultat |
|---|---|
| granite-278m, 90×128 puis 190×1 | **panique** : « Expand … '382 != 182' » (`burn-tensor/src/tensor/api/base.rs:3050`) |
| granite-278m, 90×8 puis 190×1 | passe |
| granite-278m, 90×32 puis 190×1 | passe |
| granite-278m, 40×128 puis 190×1 | passe |
| granite-107m, 90×128 puis 190×1 | passe |
| granite-278m, 190×128 puis 90×1 | ne finit pas en 10 minutes |
| le même binaire sur `gpu:0` (card2), chauffe complète | passe (c'est le 7878) |

**Où ça casse** : en amont. Notre code fait deux appels d'`embed` successifs ; après un lot de 128 × 182, l'appel
suivant reprend une forme d'attention de (182, 182). Ce serait la fusion ou l'autotune de burn/cubecl sur cet
adaptateur, non vérifié dans burn lui-même. La fiche est chez G (son lot `tracel-ai`). Avant toute issue, il faut une
reproduction réduite à burn seul, rejouée sur le burn du jour, et une recherche de doublon. Rien ne se publie sans le
oui de Lucie.

## La plus petite reproduction (un script, pas un test : elle demande l'iGPU)

Dans un worktree jetable, ajouter à `tests/e2e_burn_embedder.rs` le test `igpu_shape_sequence_repro` (charger
`BurnGranite278m` sur `BurnDevice::parse("igpu:0")`, puis appeler `embed` pour chaque pas de `IGPU_SEQ`, des textes
`mNN` comme la chauffe), puis :

```bash
cd extension/rag3weaver
export RAG3DB_LIB=<la lib épinglée> CARGO_TARGET_DIR=<un target à soi>
F=rag3db-native,burn-embedder,burn-ocr,code,daemon,openai-llm
IGPU_MODEL=granite-278m IGPU_SEQ=90x128,190x1 scripts/cargo_magasin.sh test -j8 --features $F --test e2e \
  e2e_burn_embedder::igpu_shape_sequence_repro -- --ignored --nocapture
```

Le test jetable est dans `~/git_workspaces/rag3db-igpu-repro` (worktree détaché, jamais commité), à luciepc.

## Ce qui l'évite, chez nous

**La chauffe passe par le découpage du service** (branche `warmup-lot`) : elle appelle `par_lots`, comme une requête.
Elle chauffe donc les formes que le service calcule, et seulement elles : au plus `lot_max` textes (32), sous le budget
d'octets (8 192 par défaut).

- **Un premier correctif ne suffisait pas.** Borner le lot à `lot_max`, sans le budget d'octets, laissait passer un lot
  de 32 × 382 jetons puis un texte de 512 : la panique revenait, « Expand 512 != 382 ». Le défaut tient à la taille du
  lot précédent, pas seulement au nombre de textes.
- **Le témoin** `the_warm_up_batches_stay_within_the_service_lot_max` : avec un factice qui conseille (128, 512), le
  plus grand lot de la chauffe vaut 32, et aucun lot de plus d'un texte ne dépasse le budget d'octets. Il est vu rouge
  deux fois (128, puis des lots de 32 jusqu'à 47 356 octets), puis vert.
- **Le test existant** `chauffe_partielle_garde_erreurs_et_formes` attend désormais autant de formes en erreur que de
  formes de 8 textes qui tiennent dans le budget : une forme plus grosse est coupée par le service, comme une requête.
- **Vérifié sur l'iGPU** (binaire de la branche, 19 novembre à 17 h) : granite-278m chauffe ses 16 formes sur `igpu:0`
  (113 s), « 292 textes dont 73 coupés », puis sert 90×32, 190×1, 380×32 et 40×1 sans faute.
- **Le défaut amont reste** : un appel direct de 128 × 182 puis 1 × 382 panique toujours. La fiche est chez G.
