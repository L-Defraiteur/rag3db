# 13 — Knowledge dump, au soir du 27

Reprend le [doc 03](03-knowledge-dump.md) et le met à jour. Le document qu'on
ouvre en reprenant froid.

## 1. Le strict minimum

```sh
cd extension/rag3weaver
./run_e2e.sh --summary              # tout : 33 suites, ~277 E2E, ~40 min
cargo test --lib                    # 819 unitaires, 0,15 s
less target/e2e-last.log            # le journal survit à la passe
```

**Ne jamais** lancer `cargo test --test e2e_*` à la main → `undefined symbol`.

## 2. Une suite, un test, un journal

```sh
./run_e2e.sh --summary --test e2e_code
./run_e2e.sh --summary --test e2e_code reingest_is_idem     # filtre par nom
RAG3WEAVER_E2E_LOG=/ailleurs/passe.log ./run_e2e.sh --summary
```

Le résumé **nomme les tests en échec** et dit où est le journal. Les deux
branches écrivent, avec ou sans `--summary`.

> **Écrire d'abord, filtrer ensuite.** Rediriger vers un fichier puis `tail -f`
> dessus. Jamais `commande | tail | grep` en bout de chaîne : le filtre
> bufferise, rien ne s'affiche avant la fin, et c'est le seul endroit où la
> sortie aurait existé.

**Pas de guetteur `pgrep`.** Le motif est contenu dans la ligne de commande du
guetteur, donc il se trouve lui-même et tourne pour toujours. Un est resté
21 heures.

## 3. Une passe à la fois

```sh
pgrep -f "[c]argo test --features rag3db-native" && echo occupé
```

**N'importe quelle compilation pendant une passe compte comme une deuxième** :
même `target/`, même verrou. Les suites tardives ne testent alors plus le même
code que les premières — *une passe incohérente est pire qu'une passe absente*.

## 4. Les cartes graphiques — **mesuré sur ce poste**

```
Vulkan GPU0 = card2  → pilote DP-6 et HDMI-A-2 : les écrans de travail
Vulkan GPU1 = card0  → llama-server (--device Vulkan1), Qwen 30,5 Go/32
Vulkan GPU2 = card1  → Intel, ne pilote aucun écran
```

```sh
export RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1
export RAG3WEAVER_BURN_DEVICE_RERANKER=gpu:1
export RAG3WEAVER_BURN_DEVICE_OCR=gpu:1
```

Une passe entière ainsi épinglée laisse la carte d'affichage **sous 6 %**,
contre 98-100 % au défaut. Le défaut tombe sur `gpu:0`, donc sur tes écrans.

**L'iGPU est inutilisable** : ~60× plus lente, et son débit *empire* quand le
lot grandit (148 → 241 → 121 tok/s). Elle partage la RAM système.

Débit BGE-M3 en release, R9700 :

```
lot ×  seq  = jetons   tok/s
 64 ×  32   =  2048    7550   ← crête
 16 × 128   =  2048    7417   ← crête
  4 × 512   =  2048    6210   ← crête
 64 × 128   =  8192    5507   ← ça redescend
 64 × 512   = 32768    5378
```

**L'optimum est ~2 048 jetons par passe**, quelle que soit la répartition.

## 5. Le disque

```sh
./menage_target.py target        # refuse de tourner pendant une passe
RAG3WEAVER_NO_GC=1 ./run_e2e.sh  # sauter le ménage
```

`run_e2e.sh` fait le ménage **avant chaque passe** et met `CARGO_INCREMENTAL=0`
— l'incrémental ne rapporte rien sur 34 binaires construits une fois, et
coûtait 124 Go en trois jours. Premier ménage : **491 Go libérés**, `target/` de
580 à 101 Go.

Le ménage ne touche **que nos artefacts** : les dépendances externes sont
stables, les effacer coûterait une reconstruction complète pour rien.

## 6. L'agent, en nuage et en local

```sh
# Local — llama.cpp EST le client OpenAI
~/git_workspaces/llama.cpp/build/bin/llama-server \
  -m ~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/*.gguf \
  --device Vulkan1 -ngl 99 --host 127.0.0.1 --port 8080 --jinja \
  -c 131072 --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0

RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
  ./run_e2e.sh --features openai-llm --test e2e_cloud_code_agent
```

`--jinja` **n'est pas facultatif** : sans lui, pas de gabarit, donc pas
d'appels d'outils.

## 7. Les expériences à plusieurs

```sh
RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b \
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1 RAG3WEAVER_BURN_DEVICE_RERANKER=gpu:1 \
RAG3WEAVER_ARTEFACT=essai3 \
  ./run_e2e.sh --features openai-llm,burn-embedder --test e2e_conversation_a_plusieurs
```

- `RAG3WEAVER_ARTEFACT=nom` → `target/artefacts/fil-nom.md`, pour ne pas
  écraser la manche précédente.
- `RAG3WEAVER_TEMOIN=1` → **retire la phrase de domaine** des trois agents. Le
  témoin du critère falsifiable : si les réponses divergent autant, le rôle ne
  portait rien.

**Sans `--features burn-embedder`, l'embedder est un `HashEmbedder` factice** :
`search` devient du BM25 seul, une requête conceptuelle rend zéro, et les agents
préfèrent `grep` — à juste titre. C'est le piège n°1 de cette section.

## 8. Mesurer

```sh
cargo test --lib agent::tests::dix_tours -- --nocapture      # absorption : 900180 → 37567
cargo test --lib le_defaut_hybrid -- --nocapture             # coût d'un schéma
cargo run --release --example burn_throughput \
  --no-default-features --features burn-embedder -- <model.bpk> <tokenizer.json>
```

L'exemple de débit lit `RAG3WEAVER_BURN_DEVICE_EMBEDDER` — il codait sa carte en
dur, ce qui rendait toute comparaison impossible.

## 9. Les pièges, par probabilité

1. `cargo test --test e2e_*` à la main → `undefined symbol`.
2. Compiler pendant une passe → passe incohérente, verte et fausse.
3. Ne garder qu'une sortie filtrée d'une commande longue.
4. Un guetteur `pgrep` qui se trouve lui-même.
5. **Un montage d'expérience qui contredit sa question** — corpus limité,
   embedder factice, limite d'itérations trop basse. Les symptômes ressemblent
   à des défauts du moteur.
6. Ajouter un champ à une structure publique → littéraux cassés ailleurs, y
   compris **derrière des features que `cargo build --lib` ne compile pas**.
7. Filtrer une date par préfixe sur `at`.
8. Croire un commentaire plutôt que mesurer.
9. Un `%% param:` sans type ; sans `%% choices:`, le modèle invente.
10. Chercher `Symbol` par vecteur : pas de chunk, pas d'embedding.
11. Laisser `target/` grossir — corrigé, mais vérifier que le ménage tourne.

## 10. Symptôme → où regarder

| Symptôme | Regarder |
|---|---|
| « 0 passed, N non lancées » | une suite ne compile pas — `INCOMPLETE` le dit |
| une passe verte trop rapide | une compilation a tourné en même temps |
| un test en échec sans nom | le journal, `target/e2e-last.log` |
| le poste rame pendant les tests | quelle carte — `gpu_busy_percent` de card0 et card2 |
| l'embedding est lent | le lot : borné par le texte ? l'embedder est-il factice ? |
| `search` rend `bm25` partout | pas de `burn-embedder`, donc pas de dense |
| un agent boucle sur la même erreur | l'outil dit-il la correction ? l'agent peut-il l'appliquer ? |
| deux événements de trace ont disparu | `Trace` n'a pas de `hashsafe` |
| un renvoi `#read-2` introuvable | l'outillage est-il enveloppé par `SessionTools` ? |

## 11. Les tests qui valent une lecture

- `agent::tests::dix_tours_avec_et_sans_absorption` — le témoin dans le même
  test que la mesure.
- `dataflow::record_nodes::tests::les_plages_couvrent_tout_sans_trou_ni_recouvrement` —
  l'invariant, pas le cas nommé.
- `config::tests::un_etat_inatteignable_est_refuse` — ce qui ne se voit ni à la
  lecture ni à l'exécution.
- `e2e_agent_loop::a_graph_sends_a_message_and_the_agent_reads_it_between_turns` —
  la séquence exacte du bus, qui attrape tout changement de contrat.
- `e2e_conversation_a_plusieurs` — **une expérience, pas une validation** : sa
  seule assertion est faible exprès.

## 12. Ailleurs

- `LR_CodeRag/…/kuzu-wasm-exp/l5/code-rag/` — la maquette d'origine : le schéma
  actuel, plus `boostIf` et deux enrichissements qu'on a perdus.
- `LR_CodeRag/ragforge/docs/BRAIN_SEARCH_OUTPUT_PROPOSAL.md` — le format de
  sortie, avec notre défaut d'aujourd'hui décrit mot pour mot et le gain chiffré
  112 K → 8 K.
- `LR_CodeRag/ragforge/docs/brain_search_example_output.md` — la sortie réelle,
  à copier plutôt qu'à paraphraser.
