# Un nœud WebAssembly : le verdict de l'essai

21 novembre 2026, session everything declarative. C'est la suite de la page 07, qui a choisi wasmi. Cette page dit ce que
le lot `wasm-node-trial` livre, ce que ses témoins prouvent, et ce que coûte un appel, en release, à côté du Rust et du
TypeScript de la page 01.

## Ce que le lot livre

- **Une langue de script `wasm`**, derrière la feature `wasm-nodes` (wasmi 2.0.2, sans sa feature `wat` : l'hôte prend
  les octets d'un module, jamais son texte ; la validation reste). Sans la feature, la langue existe et refuse en le
  disant : « il faut la feature wasm-nodes ».
- **Une déclaration `.node.json` dont le script est un `.wasm`** : le module est lu en octets et voyage en hexadécimal
  jusqu'au moteur, par le même chemin que les nœuds TypeScript (rechargement à chaud compris).
- **Un contrat d'invité court** : le module exporte `memory`, `alloc(len) -> ptr` et `run(ptr, len) -> u64`. L'entrée
  est du JSON écrit à l'adresse que rend `alloc` ; la sortie est du JSON, son adresse et sa longueur tassées dans le
  `u64` (`ptr << 32 | len`).
- **Le bloc « Previous step » de l'équipier en WebAssembly**, comme exemple et comme témoin : la source est dans
  `extension/rag3weaver/wasm/previous_step/` (une crate à part, son propre workspace), bâtie par
  `scripts/build_wasm_nodes.sh` vers `tests/fixtures/wasm_nodes/`. Il n'est **pas livré dans un gabarit** : un gabarit
  ne peut pas demander une feature que l'hôte n'a peut-être pas.

## Les bornes et l'isolement

- **Isolé par construction** : l'hôte n'offre aucune import. Un module qui importe quoi que ce soit (un fichier, le
  réseau, une horloge) est refusé à la préparation, et l'erreur nomme l'import (`Forbidden("env::read_file")`).
- **Borné à chaque appel** : une instance neuve par appel, sa mémoire sous `memory_bytes`, son travail sous `fuel`.
  - Le carburant de wasmi est **déterministe** : le même module sur la même entrée s'arrête au même point, quelle que
    soit la charge du poste. Le défaut est 100 millions d'unités (`ScriptLimits.fuel`).
  - Une boucle sans fin s'arrête sur son carburant, dite `Timeout` ; une mémoire qui grandit sans fin s'arrête au
    plafond de l'hôte, dite `Memory`. Ce dernier classement **lit le code du piège** (`GrowthOperationLimited`), jamais
    le texte de l'erreur : le premier essai comparait un texte que wasmi n'écrit pas ainsi, et classait la mémoire en
    faute ordinaire. Le témoin l'a vu rouge.
  - Un module de plus de `module_bytes` (8 Mio par défaut) est refusé avant la compilation.

## Les témoins

| Témoin | Ce qu'il prouve | Rouge vu |
|---|---|---|
| `script::tests::wasm::a_wasm_script_is_bounded_by_its_fuel_and_memory_and_isolated` | la boucle stoppée par le carburant, la mémoire par le plafond, l'import refusé et nommé | moteur neutralisé, puis classement par le texte |
| `…carry_over_tests::wasm_previous_step::the_wasm_previous_step_is_the_rust_one_and_a_broken_module_falls_back` | le bloc WebAssembly égal au bloc Rust **à l'octet près** sur les sept états (coupe, accents, caractères hors du plan de base compris) ; un module cassé laisse servir le Rust, et l'erreur nomme le `.wasm` | moteur neutralisé |

Les deux vivent dans un fichier à eux, déclarés sous `cfg(feature = "wasm-nodes")`. La lib entière sous les features de
service plus `wasm-nodes` : 2 879 tests passés, le seul rouge était le classement de la mémoire, corrigé depuis.

## Ce que coûte un appel

En release à luciepc, médiane de 200 appels, sur l'état de la page 01 (200 lectures, 50 écritures, une commande et ses
1 500 derniers caractères, un dernier texte de 3 000 caractères, au-dessus du budget de 8 000) :

| | Rust | TypeScript (page 01) | WebAssembly |
|---|---|---|---|
| Le bloc seul | 6,8 µs | — | — |
| Le nœud tel qu'un pas l'appelle : la déclaration relue, le script préparé, l'appel | — | 1,33 ms | **3,62 ms** |
| Le script préparé une fois : l'appel | — | 1,09 ms | **0,37 ms** |

Le Rust de la page 01 était mesuré à 8,8 µs ; l'écart avec 6,8 µs est celui de deux passes, pas d'un changement.

**Lecture.**

- **Préparé une fois, un appel WebAssembly coûte le tiers d'un appel TypeScript** (0,37 ms contre 1,09 ms), et
  l'essentiel de ces 0,37 ms est l'instance neuve de chaque appel, pas le calcul. C'est le prix de l'isolement entre
  deux appels.
- **Préparé à chaque pas, il coûte presque trois fois plus** (3,62 ms contre 1,33 ms) : relire 65 Ko, les passer en
  hexadécimal, les relire en octets, puis valider et compiler le module. La compilation de wasmi est rapide (0,42 ms à
  froid en page 07), mais le chemin par le texte hexadécimal et la relecture la triple.
- **À l'échelle d'un pas de l'équipier**, qui attend le modèle plusieurs secondes, ni l'un ni l'autre ne se voit. **À
  l'échelle d'une ligne d'index ou d'un résultat de recherche**, seul le module préparé une fois et gardé est
  acceptable, et même lui reste 55 fois le Rust.

## Le verdict

1. **WebAssembly tient ses promesses de sûreté** : bornes déterministes, aucune import, chaque trappe classée par son
   code. C'est la langue qu'il faut pour un nœud qu'un agent écrit et que l'hôte exécute sans le lire, ce qui est le cas
   de la porte du mode auto.
2. **Il ne remplace pas le Rust sur un chemin chaud** : 55 fois le Rust une fois préparé.
3. **Ce qui manque pour qu'il serve hors de l'essai**, sans ticket tant que personne ne le demande :
   - garder le module préparé d'un pas à l'autre, sous la signature du fichier (le rechargement à chaud la tient déjà
     pour le dossier `nodes/`) : le coût d'un pas descend de 3,62 ms vers 0,37 ms ;
   - lire le `.wasm` en octets jusqu'au moteur, sans le détour par l'hexadécimal ;
   - livrer la feature `wasm-nodes` dans le bâti de service, pour qu'un gabarit puisse porter un nœud WebAssembly.
4. **La feature reste hors du service** tant que Lucie ne l'a pas décidé : elle ajoute 32 paquets au bâti.
