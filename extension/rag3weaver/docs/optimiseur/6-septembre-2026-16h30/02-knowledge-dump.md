# Knowledge dump — mesurer le moteur, et où regarder quand

**6 septembre 2026, session « optimiseur ».** Ce qu'il faut savoir pour
reprendre le moteur burn sans retrouver les mêmes murs. Le
[01](01-l-etat-du-moteur.md) dit l'état, le [03](03-chantiers-ouverts.md) ce
qui reste. Pour lancer les suites, le régime, PostgreSQL, git et la machine :
le knowledge dump de la session architecture,
[`docs/6-septembre-2026-11h43/03`](../../6-septembre-2026-11h43/03-knowledge-dump.md).

## 1. Les bancs

Tous dans `tests/`, tous `#[ignore]`, tous sur la carte TV pour ne pas gêner
le bureau (`RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1` ; le régime `confort`
la choisit aussi). **Un banc à la fois** : deux tests dans le même processus
se partagent la carte et rendent des chiffres mélangés — `--exact` et un nom.

```sh
# BGE-M3 (le défaut), local, sans démon
RAG3WEAVER_SANS_DEMON=1 RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1 ./run_e2e.sh --test e2e_banc_bge_m3
# un autre modèle, seul
F=rag3db-native,burn-embedder,burn-ocr,code,daemon
RAG3WEAVER_SANS_DEMON=1 RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1 \
  cargo test --features $F --test e2e_banc_bge_m3 jetons_par_seconde_granite_107m -- --ignored --nocapture --exact
#   jetons_par_seconde | _granite_107m | _granite_278m | _minilm | _multilingual_minilm
# parité d'une précision contre f32 : d'abord f32 dépose, puis l'autre compare
RAG3WEAVER_BURN_FLOAT=f32 … --test e2e_banc_bge_m3 le_f16_rend_les_memes_vecteurs …
… --test e2e_banc_bge_m3 le_f16_rend_les_memes_vecteurs …        # Flex32, cosinus > 0,999
# Vertex (quota du compte : 429 dès huit appels en vol)
GOOGLE_APPLICATION_CREDENTIALS=../../.vault/vertex-sa.json GOOGLE_CLOUD_PROJECT=lr-hub-472010 \
  cargo test --features burn-embedder,daemon,openai-llm --test e2e_banc_vertex_embedding -- --ignored --nocapture
# llama.cpp, le comparateur « spécialisé » (GGUF sous ~/.cache/rag3weaver/gguf/)
~/git_workspaces/llama.cpp/build/bin/llama-bench -m ~/.cache/rag3weaver/bge-m3/bge-m3-FP16.gguf \
  --device Vulkan1 -ngl 99 --embeddings 1 -n 0 -p 816,3264,6528 -b 8192 -ub 8192 -r 3
```

Le banc imprime pour chaque lot « premier » (la première rencontre d'une
forme : compilation des noyaux, autotune) et « chaud » (moyenne de trois) ; la
ligne « 12 formes inédites » est celle qui ressemble à une ingestion. Les
lots de 900 mots sont tronqués à 512 jetons par tout ce qui n'est pas BGE-M3.

Les suites de sens par modèle : `e2e_burn_embedder`, `e2e_burn_minilm`,
`e2e_burn_multilingual_minilm`, `e2e_burn_reranker`, `e2e_burn_xlmr_reranker`,
`e2e_burn_ocr`, `e2e_burn_granite`. Sept vertes = le moteur va bien.

## 2. Les variables

| variable | effet |
|---|---|
| `RAG3WEAVER_BURN_FLOAT` | `f32`, `flex32` (défaut), `f16`, `bf16` — la précision de la carte et des poids chargés |
| `RAG3WEAVER_BURN_DEVICE_EMBEDDER` / `_RERANKER` / `_OCR` | `gpu:N`, `igpu:N`, `rocm:N`, `cpu` ; sinon le régime choisit |
| `RAG3WEAVER_EMBED_MODEL` | le modèle que sert le démon : `bge-m3`, `granite-107m`, `granite-278m` |
| `RAG3WEAVER_DEMON_CHAUFFE=0` | pas de chauffe des classes de forme au démarrage du démon |
| `RAG3WEAVER_SANS_DEMON=1` | les tests chargent le modèle en local plutôt que par le démon |
| `RUST_LOG=cubecl_wgpu=debug,cubecl_runtime=info` | ce que cubecl voit : tailles CMMA, « Tuning … », cache chargé (collecteur `env_logger` dans le démon et le banc) |
| `CUBECL_DEBUG_LOG=<fichier> CUBECL_DEBUG_OPTION=debug` | chaque compilation de noyau, chaque candidat d'autotune et son temps, chaque gagnant ; lourd (×5 sur le chaud) |
| `CUBECL_AUTOTUNE_CACHE=false` | forcer un retune sans toucher à la base |
| `CUBECL_AUTOTUNE_LEVEL` | `minimal` / `balanced` (défaut) / `extensive` / `full` — la largeur des classes de forme |

## 3. Le cache d'autotune

Actif par défaut, SQLite. Avec `cubecl.toml` (`[environment] path = "global"`)
il est unique par poste : `~/.cache/cubecl/default.db`. Sans ce fichier,
cubecl écrit sous `<workspace>/target/environment/` quand il trouve un
`Cargo.toml` au-dessus du cwd — deux bases selon d'où on lance, et chacune
repaie la chauffe.

Les clés de matmul sont arrondies à la puissance de 2 (m, n, k) : une forme
inédite ne coûte un autotune que si elle change de classe. Le checksum d'une
entrée ne voit que les **noms** des candidats : **patcher un noyau
(cubek-matmul, cubecl-wgpu) n'invalide pas le cache** — vider la base
(`rm ~/.cache/cubecl/default.db*`) sinon on mesure l'ancien gagnant.

Ce que le cache ne couvre pas : la compilation des noyaux (SPIR-V, puis le
pipeline radv), par processus, ~1 s par classe de forme. C'est le « premier »
du banc, et la raison de la chauffe du démon.

## 4. Où regarder, et à quel moment

- **Une optimisation ne change rien** (même débit) : comparer les vecteurs au
  bit près contre la référence (`écart absolu max`, nombre de composantes
  différentes). Un écart de 0 veut dire « rien n'a changé » — Flex32 a eu
  quatre portes fermées de cette façon (type refusé, poids ré-étiquetés f32,
  `from_data` refusé, accumulateur Flex32). Le cosinus à cinq décimales
  affichait 1,00000 dans tous les cas.
- **`DTypeMismatch` dans `burn-ir/src/builder.rs`** : la fusion refuse un
  mélange f32/Flex32 que les noyaux nus toléraient (le binaire prend le dtype
  de gauche). Chercher qui est en f32 : un `.float()` (Flex32) contre une
  constante f32 initialisée dans le code, ou l'inverse. Réponses connues :
  `--casts-neutres` sur le graphe, le masque du pooling casté sur
  `hidden.dtype()`, l'image OCR castée à `PRECISION_OCR`.
- **`to_vec` refuse (« expected Flex32, got F32 »)** : la sortie est
  étiquetée Flex32 ; `.to_data().convert::<f32>()` avant `try_to_vec`.
- **« Can't execute the autotune plan » ou un serveur cubecl qui panique
  sur `failed to reserve N bytes`** : un candidat d'autotune demande un
  tenseur au-dessus de la taille maximale d'un tampon wgpu (~2 Go) ; c'est le
  candidat naïf de l'attention sur un lot trop grand. Réduire le lot (le
  conseil de Granite est à 128 × 512 pour ça) ; cubecl pre.3 se rattrape mais
  pre.2 non.
- **Un test de déterminisme échoue à 1e-6** : en Flex32, un texte seul et le
  même dans un lot n'ont pas le même pavage de matmul, et les réductions
  accélérées ne fixent pas l'ordre des additions. Chauffer d'abord, comparer à
  1e-3 (logits : 1e-2). Les tests concernés sont déjà réécrits.
- **Chauffe de 60 s ou plus** : burn compilé en -O0 (vérifier le profil), ou
  cache d'autotune froid dans une nouvelle base (voir §3), ou un modèle qui
  vient d'être remplacé sans chauffe.
- **Le chiffre a changé entre deux runs** : autotune à cache neuf, ou un
  autre test dans le même processus, ou le bureau sur la même carte. Regarder
  la ligne `[rag3weaver] burn : …` et le « premier ».

## 5. Les forks

Trois dépôts sous `L-Defraiteur/` (cubecl, burn, cubek), branches
`rag3weaver/pre.2` et `rag3weaver/pre.3`, un commit chacune, épinglées par
révision dans `Cargo.toml`. Pour changer une ligne : cloner le fork, partir du
tag exact du lock, commiter en français, pousser, mettre la nouvelle révision
dans les 51 entrées `[patch.crates-io]` (un `sed` sur l'ancienne révision),
`cargo metadata` pour re-résoudre, vider le cache d'autotune si un noyau a
changé. Les clones de travail de cette session étaient dans le scratchpad ;
`~/.cargo/git/checkouts/` a la copie que cargo utilise.

**Itérer sur un fork sans toucher au `Cargo.toml` partagé** (7 septembre) :
un fichier `[patch.crates-io]` qui pointe **tout l'atelier** par chemin
(`burn = { path = "<clone>/crates/burn" }`, les 25 crates burn, les 11
cubek — patcher un seul crate tire ses voisins en double) et
`cargo --config <ce fichier> test …` : cargo compile contre le clone, l'autre
session ne voit rien, et on ne pousse que quand la mesure est bonne. Les
clones de ce soir : `<scratchpad>/burn` et `<scratchpad>/cubek`, remote
`fork` = L-Defraiteur (`origin` est tracel-ai, en lecture seule).

**Le piège fish** : `E="A=1 B=2"; env $E cargo …` pose une seule variable
`A` valant `1 B=2`. Toujours les variables en préfixe explicite, ou
`env A=1 B=2 cargo …` en toutes lettres. Le 7 septembre à minuit, ça a
lancé quatre bancs sans `RAG3WEAVER_SANS_DEMON`, sur la mauvaise carte, et
remplacé le démon de la session architecture.

Monter de version (pre.3 → suivante) : `docs/issues/6-septembre-2026/vers-pre3.py`
montre le geste (versions, branches, adaptateur), et
`flex32-adapter-pre3.rs` l'API burn-store à suivre.

## 6. Les autres sessions, et comment leur parler

Deux sessions Claude Code travaillent sur le même arbre, sans branche :
**architecture** (chemin d'indexation : découpe, lots, pipeline, catalogue,
base ; ses docs sous `docs/<date>/`) et **optimiseur** (celle-ci : moteur
burn, démon, forks ; `docs/optimiseur/<date>/`). Lucie relance l'une ou
l'autre avec `claude --resume <id>`.

- **Trouver l'autre** : l'outil `ListAgents` liste les sessions vivantes du
  poste avec leur nom ; le 6 septembre au soir, l'architecture s'appelait
  « Rag3weaver architecture backend et FTS ». L'identifiant de reprise que
  Lucie donne n'est pas le nom affiché — c'est le nom qu'il faut.
- **Lui écrire** : `SendMessage` avec ce nom ; ses réponses arrivent d'elles-
  mêmes dans la conversation (`cross-session-message`). Première ligne =
  l'essentiel (elle n'en voit que ça avant d'ouvrir). Un rapport long va dans
  un doc commité, le message ne porte que le hash et le résumé — c'est ainsi
  qu'est parti [`docs/issues/6-septembre-2026/04`](../../issues/6-septembre-2026/04-rapport-pour-la-session-principale.md).
- **Ce qu'on s'est promis**, après y avoir laissé une heure :
  1. `git add` avec des chemins explicites seulement, jamais `-A` — un
     `git add -A` de l'autre a ramassé huit fichiers de celle-ci dans son
     commit a7bb1b308 ;
  2. chacune commite ses fichiers elle-même, en français, sans trailer ;
  3. un fichier laissé non compilable dans l'arbre bloque l'autre : on le
     corrige ou on le commite tout de suite en prévenant, et on dit quels
     fichiers on tient (elle ne touche pas aux nôtres, on ne touche pas aux
     siens : `catalog.rs`, `scope.rs`, `record_nodes.rs`, `e2e_mesure_*`) ;
  4. on annonce chaque commit avec son hash et ce qu'il change pour l'autre
     (un conseil de lot, une Identite, une dimension) ;
  5. la carte TV est partagée : on dit quand on la prend pour une chaîne de
     bancs, elle mesure entre deux.
- **Le contrat entre les deux** : l'`Identite` du démon (`modele`, `dim`,
  `precision`, `lot_conseille`) et `Embedder::budget_conseille()` côté
  moteur ; `_catalog_meta` (nom et dimension du modèle, refus d'un mélange)
  et les lots en jetons côté indexation. Changer l'un se dit à l'autre.

## 7. Les pièges qui ont coûté une heure chacun

1. `run_e2e.sh` compilait les dépendances en -O0 : deux semaines de mesures
   dans ces conditions. Réglé dans `Cargo.toml`.
2. `burn-rocm` rallumait autotune et fusion pour les deux piles : le
   « facteur 14 » du 28 août était une chauffe d'autotune en -O0.
3. Sans la feature `autotune`, la stratégie d'attention par défaut de
   burn-cubecl est `Fallback` : la flash attention ne s'essaie jamais.
4. Le masque en biais additif renvoie aussi sur `Fallback` (règle 2 du script).
5. Le no-op de `cast(F32)` sur du Flex32 est le bon comportement ; le
   « corriger » casse la fusion.
6. Deux sessions sur le même arbre : `git add` avec des chemins explicites
   seulement (un `git add -A` de l'autre session a ramassé huit de mes
   fichiers dans son commit) ; un état non compilable se corrige ou se commite
   tout de suite en prévenant.
7. `pkill -f` avec un motif tue le shell qui le porte (mémoire du poste) :
   `pkill -x`, ou `pidof` sur le nom exact.
