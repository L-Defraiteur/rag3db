# Knowledge dump — lancer les tests, et où regarder

Écrit pour reprendre froid. Tout est vérifié dans la session du 29–30 août.

## 1. Lancer les tests

### La règle qui coûte le plus cher quand on l'oublie

> **Les suites E2E ont besoin de `RAG3DB_SHARED=1`.** Sans lui, le cœur C++ est
> lié en **statique**, aucun de ses 190 symboles n'atteint la table dynamique,
> et un `dlopen` d'extension ne résout rien. On croit alors à une extension
> périmée — j'y ai perdu une heure le 29 août.

`run_e2e.sh` le pose. `cargo test` lancé à la main, **non**.

```bash
cd extension/rag3weaver

# La passe complète, poste utilisable pendant
RAG3WEAVER_REGIME=confort ./run_e2e.sh --features daemon --summary

# À plein régime
./run_e2e.sh --features daemon --summary

# Une seule suite
./run_e2e.sh --features daemon --test e2e_catalogue_gabarits
```

### Les tests unitaires — sans environnement particulier

```bash
cargo test --features code,daemon,rag3db-native --lib -j $(( $(nproc) - 2 ))   # 952
cd codeparsers && cargo test -j $(( $(nproc) - 2 ))                            # ~90
```

**Jamais `-j$(nproc)`** : c'est la compilation qui fige le poste, pas les
tests. `.cargo/config.toml` pose déjà `jobs = -2`.

### Une suite à la main, quand il faut le faire

```bash
ROOT=/home/lucied/git_workspaces/rag3db; BUILD=$ROOT/build/native-test
export LD_LIBRARY_PATH="$BUILD/src:$LD_LIBRARY_PATH" RAG3DB_SHARED=1 \
       RAG3DB_LIBRARY_DIR="$BUILD/src" RAG3DB_INCLUDE_DIR="$BUILD/src" RAG3DB_ROOT="$ROOT"
cargo test --features rag3db-native,burn-embedder,daemon,code \
  --test e2e_catalogue_gabarits -- --ignored --nocapture --test-threads=1
```

## 2. Les modèles

### Le démon d'embedding — rien à lancer à la main

Il démarre seul au premier besoin (`DaemonEmbedder::assurer`), sur
`127.0.0.1:7878`, et **survit** entre les binaires de test. Journal :
`/tmp/rag3weaver-demons/`.

```bash
pkill -f 'rag3weaver-embedding[s]'   # les crochets sont volontaires — voir §5
```

Poids : `~/.cache/rag3weaver/bge-m3/{model.bpk,tokenizer.json}`.

### Le modèle local — llama.cpp, build HIP

```bash
M=~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/Huihui-Qwen3-Coder-30B-A3B-Instruct-abliterated.i1-Q6_K.gguf
HIP_VISIBLE_DEVICES=1 /home/lucied/git_workspaces/llama.cpp/build-hip/bin/llama-server \
  -m "$M" --port 8080 -c 16384 --jinja -ngl 999 --host 127.0.0.1
```

`HIP_VISIBLE_DEVICES=1` = **`card0`**, la carte libre. Vérifié : 25 Go dessus,
écran intact. Chargement en 5 s. `--jinja` n'est pas facultatif — sans lui, pas
d'appels d'outils.

Puis : `RAG3WEAVER_LOCAL_LLM=http://127.0.0.1:8080/v1 RAG3WEAVER_LOCAL_MODEL=qwen3-coder-30b`.

### Le nuage — Vertex

```bash
GOOGLE_APPLICATION_CREDENTIALS=$ROOT/.vault/vertex-sa.json GOOGLE_CLOUD_PROJECT=lr-hub-472010
```

**`GenOptions::default()` donne 512 jetons**, et un modèle qui raisonne les
dépense en réflexion : on lit alors la queue d'un raisonnement en croyant lire
une réponse. Mettre `with_max_tokens(16_000)` pour toute vraie question.

## 3. Les cartes, sur ce poste

| | | |
|---|---|---|
| `card0` | PCI 07:00.0 · Vulkan `gpu:1` · HIP `1` | **libre** |
| `card2` | PCI 04:00.0 · Vulkan `gpu:0` · HIP `0` | **les deux écrans** |

`card0-HDMI-A-3` se déclare `connected` avec **zéro EDID et un mode 640×480** :
connecteur fantôme, rien n'est branché. `status=connected` ne veut donc pas
dire qu'il y a un écran — d'où le critère « la carte la moins chargée ».

```bash
for c in /sys/class/drm/card*/device; do n=$(basename $(dirname $c));
  b=$(cat $c/gpu_busy_percent 2>/dev/null); [ -n "$b" ] &&
  printf "%-7s %3s%% VRAM %.2f Go\n" "$n" "$b" \
  "$(awk "BEGIN{print $(cat $c/mem_info_vram_used)/1073741824}")"; done
```

## 4. Les réglages

| variable | effet |
|---|---|
| `RAG3WEAVER_REGIME` | `confort` \| `plein` |
| `RAG3WEAVER_BURN_DEVICE_EMBEDDER` | `gpu:1` sur ce poste |
| `RAG3WEAVER_GPU_DUTY` | 5–100, pourcentage |
| `RAG3WEAVER_EMBED_CHAR_BUDGET` | caractères par appel GPU |
| `RAG3WEAVER_EMBEDDINGS_ADDR` | défaut `127.0.0.1:7878` |
| `RAG3WEAVER_SANS_DEMON` | force le chargement local (donne l'A/B) |
| `RAG3WEAVER_RENDER_TEMPLATES` | où chercher les gabarits de rendu |

Précédence, partout : **le code > la variable > le régime > le défaut.**

## 5. Deux pièges qui coûtent du temps

**`pkill -f motif` tue son propre shell** si la ligne de commande contient le
motif. Trois fois dans la session, dont deux passes de test perdues. Écrire
`pkill -f 'rag3weaver-embedding[s]'` : les crochets font que la chaîne ne
correspond pas au motif.

**Deux `cargo` d'affilée, on lit le second.** Un `cargo check | grep` suivi
d'un `cargo check | tail` fait croire à une compilation instantanée : la
première a compilé, la seconde a lu le cache. Rediriger vers un fichier et
lire le fichier.

## 6. Où regarder dans le code

| pour | fichier |
|---|---|
| lancer un serveur, savoir s'il répond | `src/serveur.rs` |
| démons (embedding, base) | `src/daemon/{mod,embeddings,db}.rs` |
| la porte des commandes, verdict, exécution | `src/commande.rs` |
| réduire une ligne shell en argv | `codeparsers/src/shell.rs` |
| régime confort/plein, carte la moins chargée | `src/regime.rs` |
| rythme GPU et taille des lots | `src/embedder.rs` (bas du fichier) |
| choix de carte par rôle | `src/burn_device.rs` |
| catalogue de gabarits | `src/template.rs`, `src/dataflow/template_nodes.rs` |
| `run`, `run_bg`, `wait` | `src/dataflow/run_nodes.rs` |
| la carte du graphe | `src/dataflow/schema_nodes.rs` |
| rendu par gabarit (`rendre`) | `src/dataflow/render_nodes.rs` |
| parseur Mermaid | `src/dataflow/mermaid.rs` |
| fiches d'outils | `templates/tools/*.mmd` |
| gabarits de rendu | `templates/render/*.jinja` |
| boucle d'agent, boîte, pauses | `src/agent.rs` |
| postures, interblocages | `src/postures.rs` |
| mémoire de session, `Absorb` | `src/session.rs` |

## 7. Les bancs qui mesurent quelque chose

| banc | ce qu'il mesure |
|---|---|
| `e2e_catalogue_gabarits::brique_2_quel_signal_ment` | les trois questions françaises — **3/3**, scores 3,85 · 7,23 · 7,30 |
| `e2e_prise_atomique` | un second processus ne peut pas ouvrir la base ; deux lecteurs le peuvent |
| `e2e_rag3daemon` | deux processus, quarante travaux, 20/20 |
| `e2e_demon_embeddings` | 4,22 s de chargement contre 1,16 ms d'attachement |
| `e2e_lecture_mermaid` | lecture/écriture Mermaid, nuage **et** local |
| `e2e_avis_du_modele` | l'avis d'un modèle sur notre surface d'outils |
| `e2e_charge_ingestion` | ce qu'une ingestion coûte à la machine |

**Ne jamais mesurer sans catalogue branché** : `e2e_avis_du_modele` échoue
exprès si l'énumération des cibles est vide — sinon on évalue une surface plus
pauvre que la vraie.
