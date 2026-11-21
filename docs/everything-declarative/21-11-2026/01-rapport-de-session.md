# Note de pause d'ED — samedi 21 novembre 2026, avant le redémarrage de la tablette

Session everything declarative (ED). À relire à la reprise (`claude --resume`).

## Où j'en suis

- **Atterri aujourd'hui** :
  - le nom exact par le nom ;
  - le lot `token-count` (lire sans compteur) ;
  - `options` facultatif ;
  - le nœud scripté à l'essai (« Previous step »), et l'erreur d'un nœud scripté qui nomme son fichier ;
  - les pages 02 (générations étiquetées), 03 (placement des témoins), 04 (ce qui reste codé en dur), 05 (le temps dans
    les graphes, `postpone`, la porte) et 06 (le coût d'une vue) ;
  - le ticket `explain_diff`.
- **En file au pousseur** : `ed-scripted-nodes-hot-reload` (branche `scripted-nodes-hot-reload`, tête `214f820f6a`,
  attestation verte, déposé à 14:28). À sa sortie, retirer le worktree `ed/21-11-2026-rechargement`, qui n'a plus de
  target.
- **En cours** : l'essai WebAssembly, au stade du banc de comparaison des moteurs. Il est hors du dépôt, à luciepc.

## Ce qui tourne à luciepc

- **Le banc du moteur wasmtime**, rebâti après correction d'une échéance d'époque qui débordait. Il attend une place
  `poste cpu`. Processus : `/bin/sh ~/.cache/rag3weaver-build/poste cpu -- timeout 1800 cargo build --release -j 8 -q`,
  pid 3393748, dossier `~/.cache/rag3weaver-build/ed-wasm-essai/host-wasmtime`. Sa sortie revenait par le ssh de la
  tablette et sera perdue. À la reprise, relancer seulement l'exécution du binaire :
  `~/.cache/rag3weaver-build/ed-wasm-essai/target-host-wasmtime/release/host_wasmtime <guest.wasm> <input.json> <out.json>`.
- **Le banc wasmi** est écrit (`ed-wasm-essai/host-wasmi`) mais pas encore bâti.
- **L'invité** est bâti : `ed-wasm-essai/target-guest/wasm32-unknown-unknown/release/previous_step_guest.wasm`, 81 Ko.
  L'entrée de mesure est `ed-wasm-essai/input.json`.

## Les premiers chiffres de wasmtime

- **Bâti à neuf** : 2 361 s, soit 39 minutes pour le seul hôte avec wasmtime, sous `poste cpu`.
- **À froid** : moteur et compilation, 58,7 ms.
- **À chaud** : 97 µs avec une instance neuve par appel, 67 µs avec une instance réutilisée. C'est à comparer à 8,8 µs en
  Rust et 1,09 ms en TypeScript.
- **La borne de temps par époques** arrête une boucle infinie en 105 ms, pour une échéance de 100 ms.
- **L'isolement** : un import `env::read_file` est refusé à l'instanciation, avec un linker vide.
- **Le carburant et la mémoire sont à remesurer** : l'échéance d'époque à `u64::MAX` débordait et interrompait tout de
  suite. C'est corrigé (`1 << 40`), et le rebâti est en attente.

## Mes branches et leurs têtes

- `scripted-nodes-hot-reload` : `214f820f6a`, en file.
- `wasm-node-trial` : `origin/master` au moment de la création, rien de commité. Worktree `ed/21-11-2026-wasm`.
- `sonde-ecriture-en-couche-2` : les trois sondes de mesure, poussées. Elle ne va pas sur master.
- `sonde-ecriture-en-couche-3` : la sonde en règles 15, poussée. Elle ne va pas sur master.
- `ed-pause-21-11` : cette note.

## Ce qui attend

1. **Les trois pièces de la porte** (`timeout_s`, la politique de types fermée, le chargement depuis
   `~/.config/rag3weaver/gate/`), et `postpone` avec son planificateur. Elles attendent que la page 03 d'archi, mise à
   jour, soit en file. Ma page 05 en donne la forme.
2. **L'essai WebAssembly** :
   - finir le banc des deux moteurs : bornes, isolement, coût à froid et à chaud, bâti et `Cargo.lock` ;
   - écrire une page courte de choix ;
   - puis le lot : un langage `wasm` dans le moteur de script, l'invité, les témoins (l'octet près sur sept états, le
     repli sur le Rust, le rechargement), la mesure en release et la page de verdict.
3. Puis la page du contrat d'accès aux entités (avec archi), puis `explain_diff`.
4. **Des docs à verser au prochain lot** :
   - la page 03 : la règle affinée (un fichier de tests par lot, déclaré sous les features de son témoin), le piège du
     motif `tests/e2e_*.rs`, et le témoin du pousseur compilé sous ses features de service ;
   - la page 06 : les mesures (A) et (B). Sur 281 fichiers, (A) en règles 13 écrit 5 585 contenus et 7 156 vecteurs en
     62 s ; (B) en règles 15 écrit 4 124 contenus et 5 790 vecteurs en 73 s. L'écart de règles n'aggrave rien, et une
     version de scope ne dépend pas de la position.

## Ma prochaine action

Attendre l'annonce de c7 (`confiner-poste-classes` atterri) avant toute validation. Puis, à luciepc : relancer le banc
wasmtime, bâtir et lancer le banc wasmi, et écrire la page de choix du moteur.
