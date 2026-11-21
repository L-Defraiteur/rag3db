# Le moteur WebAssembly : wasmtime ou wasmi

21 novembre 2026, session everything declarative. C'est la première pièce de l'essai d'un nœud WebAssembly, que Lucie a
accepté le 21 au matin. c7 a demandé une page courte, avec les chiffres, puis le choix. Il voulait mesuré ce qui comptera
pour la porte du mode auto : les bornes, l'isolement, et le coût d'un appel court, à froid et à chaud.

## Le banc

- Il vit hors du dépôt, à luciepc, sous `~/.cache/rag3weaver-build/ed-wasm-essai/`, pour ne pas toucher au
  `Cargo.lock` du crate avant le choix.
- **L'invité** est une crate Rust compilée en `wasm32-unknown-unknown` (81 Ko, `serde_json` compris). Elle porte le bloc
  « Previous step » de l'équipier, qui reçoit et rend du JSON, plus deux sondes : une boucle infinie et une bombe
  mémoire.
- **Un hôte par moteur**, en release. L'entrée est un état de taille réelle : 200 lectures, 50 écritures, une commande
  et ses 1 500 derniers caractères, un dernier texte de 3 000 caractères.
- **Les deux moteurs rendent la même sortie, à l'octet près.**

## Les chiffres

| | wasmtime 38 (Cranelift) | wasmi 2.0 (interprète) |
|---|---|---|
| Bâti à neuf de l'hôte, `poste cpu`, `-j 8` | 2 361 s (39 min) | 326 s (5,4 min) |
| Paquets au `Cargo.lock` de l'hôte | 102 | 32 |
| Taille du binaire de l'hôte | 16 Mo | 4,9 Mo |
| À froid : moteur et compilation du module | 59 ms | 0,42 ms |
| Premier appel, à froid | 59 ms | 1,8 ms |
| À chaud, une instance neuve par appel | 98 µs | 404 µs |
| À chaud, une instance réutilisée | 68 µs | 371 µs |
| Carburant d'un appel | 1,4 million | 5,6 millions |

Pour comparaison, le même bloc coûte **8,8 µs en Rust** et **1,09 ms en TypeScript** dans QuickJS (page 01).

## Les bornes et l'isolement

| | wasmtime | wasmi |
|---|---|---|
| Une boucle infinie, borne en temps réel (époques, échéance 100 ms) | arrêtée en 105 ms | n'existe pas |
| Une boucle infinie, borne en carburant (10⁸) | arrêtée en 2,9 ms | arrêtée en 20 ms, « all fuel consumed by WebAssembly » |
| Une bombe mémoire, plafond de 64 Mio | arrêtée en 28 ms | arrêtée en 9,8 ms (« `unreachable` », l'invité qui manque de mémoire) |
| Un import non offert (`env::read_file`), linker vide | refusé à l'instanciation, en le nommant | refusé à l'instanciation, en le nommant |

**L'isolement est le même par construction.** Un module WebAssembly ne voit que ce que l'hôte lui offre : un linker vide
ne donne ni fichier, ni réseau, ni horloge. Un module qui réclame autre chose est refusé avant de tourner.

**Le piège trouvé en route** : sous wasmtime, une échéance d'époque à `u64::MAX` déborde dès qu'un fil fait avancer les
époques, et interrompt tout appel sur-le-champ. Une échéance se pose finie.

## Le choix : wasmi

- **Le bâti.** wasmtime ajouterait environ 35 minutes à un bâti à neuf du crate, et 70 paquets au `Cargo.lock`. Le crate
  se bâtit déjà dans chaque passe, chaque attestation et chaque pousse. wasmi en ajoute 5 minutes et une vingtaine de
  paquets.
- **Le chargement.** Un nœud se recharge à chaque édition de son fichier (le rechargement à chaud). Avec wasmi, compiler
  un module coûte 0,4 ms ; avec wasmtime, 59 ms à chaque fois.
- **Le coût d'appel suffit.** Pour un nœud de garde ou une mise en forme, appelés une fois par requête ou par pas, 0,4 ms
  est près de trois fois moins que le TypeScript. Les 0,1 ms de wasmtime ne changent rien à ces chemins-là.
- **La borne.** wasmi n'a qu'une borne en carburant, mais elle est **déterministe** : le même script, la même entrée,
  le même arrêt, quelle que soit la charge du poste. Pour une garde, c'est préférable à une borne en temps réel, qui
  dépend de la charge. Il faut seulement la régler : environ 6 millions d'unités pour ce bloc, donc une borne de 10⁸
  laisse un facteur 18 et arrête une boucle en 20 ms.
- **Les messages.** wasmi dit la cause en clair (« all fuel consumed ») ; wasmtime ne rend qu'une trace de pile.

**Quand revenir à wasmtime** : si un nœud scripté devait servir sur un chemin chaud (une fois par ligne d'index, par
morceau ou par résultat), où 0,4 ms par appel coûte des minutes. Ce n'est le cas d'aucun usage prévu.

## La suite

Le lot de l'essai, sur wasmi, derrière une feature `wasm-nodes` :
- un langage `wasm` dans le moteur de script, pour que le nœud déclaré, les ports vérifiés, l'erreur qui nomme le
  fichier, le rechargement à chaud et le repli sur le Rust valent sans rien réécrire ;
- l'invité du bloc « Previous step », bâti en `.wasm` ;
- les témoins : l'octet près sur les sept états de la page 01, le repli sur le Rust quand le `.wasm` est cassé, le
  rechargement, et les bornes de carburant et de mémoire ;
- la mesure en release dans le crate, et la page de verdict, avec comment un agent bâtit et pose un `.wasm`.
