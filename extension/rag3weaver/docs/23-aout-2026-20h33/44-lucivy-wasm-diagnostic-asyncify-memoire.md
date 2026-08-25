# lucivy — WASM dans le navigateur : ce qui bloquait, ce qui est corrigé

> Numéroté 41 chez lucivy — renuméroté 44 ici (collision avec la passation 41).

Session lucivy, nuit du 24 au 25 août 2026. Branche `wip/publication-3.0.0`
(`a3693ff`, `1fb67ec`). Rien de tout ça ne touche le chemin natif que vous
utilisez, sauf deux mécanismes génériques signalés en fin de doc.

## Le test

Le playground navigateur indexe en direct un corpus kernel de **15 440
fichiers** (`drivers/net`, `fs`, `net`, `kernel`, `sound`, `mm`, `arch/arm`,
tar.gz servi localement, `?corpus=`), 4 shards, commit tous les 2 000 docs,
puis un panel de 21 requêtes (tous les modes) comparé à une **référence
native** sur le même corpus (`lucivy_core/tests/test_playground_parity.rs`,
`playground/parity_run.js`, `parity_diff.py`). Le snapshot v2 commité
n'est plus utilisé.

## Ce qui bloquait le premier commit — trois couches

1. **Le `ccall` synchrone de commit bloquait le thread JS du worker**, qui
   est le « main » du runtime emscripten : tout ce que les pthreads lui
   proxifient mourait avec lui. Le worker commite désormais via
   `lucivy_commit_async` (thread + statut dans le SharedArrayBuffer).

2. **`-sASYNCIFY` + backend OPFS de WASMFS sur pthreads = cassé par
   construction.** Pile obtenue dans la console : dans le thread proxy OPFS,
   chaque opération (`open_access`, `write_access`, `get_child`) passait par
   `handleAsync`/`handleSleep` (Asyncify), et pendant que la pile wasm était
   déroulée, `checkMailbox` réentrait et lançait l'opération suivante
   *imbriquée* — des centaines de niveaux — puis `unreachable`. Symptôme
   vu de loin : la première écriture de segment (`.managed.json.tmp` à 0
   octet) qui pend, et avant ça un « memory access out of bounds ».
   Rien chez nous n'a besoin d'Asyncify : retiré du link (9,8 → 6,3 Mo).

3. **Mémoire : 4 Go atteints pendant les fusions.** Quatre fusions
   simultanées (une par shard, 14 segments et ~650 k tokens chacune) : la
   table de clés du builder FST échoue sur un `realloc` de 192 Mo
   (`merge_dag::SfxNode → BuildFstV3Node`). La même séquence, même forme
   de writer (tas 15 Mo, 1 thread, 4 threads scheduler), en **natif 32
   bits** (`i686-unknown-linux-gnu`, `gcc -m32`) : 15 s, pic RSS 1,96 Go.
   Le WASM faisait ×2 : pas de mmap, donc **chaque sidecar ouvert = une
   copie en RAM** (`StdFsDirectory::read_bytes` à l'ouverture : 837 Mo de
   sidecars v3 pour 2 000 docs, recopiés par les readers, rouverts par le
   merge), et dlmalloc ne rend rien.

## Ce qui est en place maintenant

- **`merge_permits`** (ld-lucivy) : borne sur les fusions simultanées,
  `LUCIVY_MERGE_CONCURRENCY`, défaut 1 sur wasm32, illimité ailleurs.
  L'attente sur un thread scheduler exécute du travail au lieu de bloquer
  (4 threads, 4 attentes bloquantes = interblocage).
- **Lectures paresseuses** dans `StdFsDirectory` (lucivy_core) : ouvrir un
  segment ne lit rien (un `stat`), les petites lectures (footers, ≤ 64 Ko)
  vont directement au fichier, la première vraie lecture matérialise le
  fichier dans un **cache LRU global borné** (`LUCIVY_FILE_CACHE_BYTES`,
  768 Mo wasm, 4 Go natif) ; `delete`/`atomic_write` évincent. C'est mmap
  au grain fichier : on paie ce qu'on touche, les froids partent.
- Outillage gardé : hook de panic et allocateur qui écrivent l'échec
  **avec la pile emscripten** dans le ring SAB ; `--no-opfs`, `--verbose`
  (`LUCIVY_VERBOSE`, `V3_PROFILE`) via `Module.arguments` ;
  `LUCIVY_WASM_DEBUG=1` dans `build.sh` (noms, asserts, cookies de pile) ;
  `luciole::scheduler::set_task_label` ; étiquettes par phase de
  finalisation ; trace `[fs]` par étape d'`atomic_write` ;
  `lucivy_test_fs_task` (FS depuis un thread scheduler, étape par étape).

**Résultat** : le premier commit passe, tas stable à ~900 Mo entre les
fusions, 1,6 Go pendant une fusion `content`, indexation en cours sur les
15 440 fichiers au moment où j'écris (résultats du panel dans le doc
suivant).

## Chiffres à garder en tête

- WASM instrumenté (asserts + cookies) : build v3 d'un segment de 40 docs
  ~4,7 s contre ~250 ms en natif 32 bits ; le merge `content` 14 s contre
  0,7 s. Un build release mesurera le vrai écart ; à profiler ensuite.
- Écriture OPFS de 16 Mo de sidecars : 40 ms (plus rapide que le disque
  natif à 130-200 ms).
- Sidecars v3 ≈ 15× le texte (837 Mo pour 50 Mo de source) : c'est le
  poste à attaquer pour le volume, en natif comme en WASM.

## Pour vous, côté natif

Deux mécanismes sont génériques, désactivés par défaut chez vous :
`LUCIVY_MERGE_CONCURRENCY` (si un jour vos fusions parallèles se
marchent dessus en RAM) et le cache de fichiers de `StdFsDirectory` (vous
êtes sur `MmapDirectory`/`BlobDirectory`, non concernés). Aucun changement
de format, aucun changement d'API.
