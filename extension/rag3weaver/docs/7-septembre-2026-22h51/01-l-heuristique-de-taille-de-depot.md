# L'heuristique de taille de dépôt

**7 septembre 2026, 22 h 51.** Conception, avant le code. Suite de
[l'index à plusieurs embarquements](../7-septembre-2026-21h34/01-un-index-a-plusieurs-embarquements.md),
fusionné (`3ea70fe93`).

## 1. La demande, et ce qui la rend possible

> quand on aura plusieurs embarqueurs possibles, on commencera par 107m selon
> une heuristique de taille de dépôt — 278m restant le défaut.

Ce qui existe déjà :

- **278m est le défaut du démon** (`MODELE_PAR_DEFAUT`, `bb538cb0d`) ;
  `RAG3WEAVER_EMBED_MODEL` l'écrase.
- **Un démon sert un modèle**, et se lance par `Serveur::assurer`
  (`serveur.rs:247`) avec l'environnement que l'appelant lui donne
  (`serveur.rs:329`). Le modèle d'un démon est donc une **variable posée au
  lancement** — c'est là qu'une heuristique peut parler.
- **Un index accueille plusieurs modèles** : commencer en 107m n'engage à
  rien. Passer en 278m plus tard est un retard de 100 % que le rattrapage
  solde, sans redécouper.

Le troisième point est ce qui change la nature du choix : ce n'est plus « quel
modèle pour cet index », c'est « **par quel modèle on commence** ». Un mauvais
choix coûte une passe de rattrapage, pas une réindexation.

## 2. Les chiffres qu'on a

**Vitesse** — le cœur C++ de rag3db, 1 642 fichiers, 20 136 chunks
(`docs/optimiseur/7-septembre-2026-21h56/01`, `docs/optimiseur/6-septembre-2026-16h30/04`) :

| modèle | temps de modèle | par chunk |
|---|---:|---:|
| granite-107m | 10,7 s | 0,53 ms |
| granite-278m | ~20 s | ~1 ms |

**Qualité** — le banc de l'optimiseur, 45 questions sur notre code :

| | MRR | R@1 | R@5 | sans commentaires : MRR · R@5 |
|---|---:|---:|---:|---|
| granite-107m | 0,779 | 0,62 | **1,00** | 0,738 · 0,91 |
| granite-278m | **0,844** | **0,71** | **1,00** | 0,812 · 0,93 |

Ce que ça dit : 107m perd **6 à 7 points de MRR et 9 de rappel à 1**, et rien
en rappel à 5 quand le code est commenté. Deux fois plus rapide, un index deux
fois plus petit. Il n'y a pas de dépôt où 107m est *meilleur* ; il y a des
dépôts où 278m est *trop long à attendre au premier index*.

Le critère est donc **un temps d'attente**, et la question devient : à partir
de combien de chunks 20 s deviennent une gêne ?

## 3. Le critère — tranché par Lucie

> seuil de je sais pas 50k docs ou détection GPU pourri

Pas une minute d'attente, donc, mais **deux déclencheurs**, l'un ou l'autre.
Sous les deux : 278m.

### 3.1 Plus d'environ 50 000 documents

**Des fichiers lus, pas des octets** — Lucie raisonne en documents, et c'est
ce qu'on affiche. Les octets restent dans la *raison* écrite en méta : ce sont
eux qui expliquent le temps (~500 octets par chunk, ~1 ms par chunk en 278m
sur le cœur), et un jour où quelqu'un se demandera pourquoi cet index est en
107m, c'est le temps qu'il voudra lire.

| dépôt | fichiers | 278m au premier index |
|---|---:|---:|
| le cœur C++ | 1 642 | 20 s |
| **50 000** | | ~10 min |
| le noyau Linux | ~90 000 | ~17 min |

Le seuil de 50 000 est celui de Lucie ; il tombe là où 278m dépasse la dizaine
de minutes, et le noyau — l'exemple qu'elle avait en tête le 6 septembre —
bascule.

### 3.2 Une carte faible ou absente

Ce que `regime.rs` sait déjà lire dans `/sys/class/drm` : quelles cartes ont
un compteur d'occupation (`gpu_busy_percent`, « une vraie carte ; le reste est
du décor »), leur VRAM prise (`mem_info_vram_used`), leur adresse PCI. Il
manque une lecture, gratuite au même endroit : `mem_info_vram_total`.

**Le critère : aucune carte dédiée, ou une VRAM totale sous 4 Go.**

Le plancher vient de la mesure de l'optimiseur (`docs/issues/6-septembre-2026/03`,
§ flash attention) : le lot conseillé de Granite, 128 séquences de 512 jetons,
prend **1,6 Go de scores d'attention** — 3,2 Go à 256, refusé — plus ~1,1 Go
de poids f32 pour 278m. Sous 4 Go, le lot doit rétrécir, et 278m perd le débit
qui justifie son coût ; 107m (0,4 Go de poids, dimension 384) y tient avec son
lot entier. Ce n'est pas « 278m ne marche pas » — c'est « 278m n'est plus deux
fois plus lent, il est cinq fois plus lent ».

Sans carte du tout — un portable, une machine de CI — le calcul part sur le
processeur, et là 107m est le seul qui reste tolérable au premier index.

Sur cette machine : deux cartes à 31 Go. Aucun des deux déclencheurs ; le
cœur reste en 278m.

### 3.3 La précédence, la même que partout

**Le code l'emporte sur la variable, qui l'emporte sur l'heuristique, qui
l'emporte sur le défaut.** `RAG3WEAVER_EMBED_MODEL` posée gagne toujours ;
l'heuristique ne parle que dans le silence. Pas d'exception cette fois : la
variable dit ce qu'on veut *pour ce dépôt*, et un régime `confort` ne change
pas de modèle — la carte est la même, c'est le temps qui diffère.

## 4. Où ça vit

### 4.1 Le calcul

Un module pur, `embedding_choice.rs` :

```rust
pub fn recommended_model(files: usize, source_bytes: u64, card: CardClass) -> Choice
// CardClass { Dedicated { vram_bytes }, Weak { vram_bytes }, None }
// Choice { model: "granite-107m" | "granite-278m", reason: String }
```

`CardClass` vient d'une lecture de `/sys/class/drm` à côté de
`carte_la_plus_libre`, avec `mem_info_vram_total` en plus. Testable sans base
ni carte : la classe est un paramètre. La raison est une phrase —
*« 62 000 fichiers (31 Mo), au-delà de 50 000 : granite-107m pour le premier
index »*, ou *« carte de 2 Go, sous 4 Go : granite-107m »* — parce qu'un
choix qui ne se dit pas se conteste.

### 4.2 Le moment

**Là où l'appelant construit le `Serveur` du démon**, avant `Catalog::new` :
il a lu les sources, il connaît les octets, il pose
`RAG3WEAVER_EMBED_MODEL` dans l'environnement du `Serveur` si elle n'est pas
déjà posée. `Serveur::assurer` lance le démon avec, `DaemonEmbedder::assurer`
s'y attache, le catalogue reçoit un embarqueur qui se nomme.

**Un démon déjà là sert ce qu'il sert.** Si son `Identite.modele` n'est pas
celui que l'heuristique voulait, on **prend ce qui tourne et on le dit** — on
ne relance pas un démon qu'une autre session utilise peut-être. Relancer est
un geste explicite (`RAG3WEAVER_EMBED_MODEL` posée, ou `quitter`).

### 4.3 Où le choix se lit

- **Au montage**, une ligne : `[rag3weaver] granite-107m pour le premier
  index — 31 Mo de source, au-delà de 25 Mo ; 278m viendra en retard si on
  l'exige`. Et, si le démon servait autre chose : `… mais le démon en place
  sert granite-278m — on le garde`.
- **Dans la méta**, une clé `embedding_choice` : `{model, reason, at}`,
  posée au premier enregistrement. Ce que `embedding_model:{slug}` ne dit
  pas, c'est *pourquoi* ce modèle est là le premier ; c'est ce qu'on lira
  dans six mois en se demandant pourquoi cet index est en 107m.

## 5. Quand le dépôt grossit après coup

**Rien.** Le seuil ne se réévalue pas : il décide du *premier* index, et le
premier index a eu lieu. Un dépôt qui passe de 20 à 60 Mo garde son 278m et
embarque plus longtemps ; un dépôt né gros et resté en 107m y reste tant que
personne ne demande autre chose.

Demander autre chose, c'est : `RAG3WEAVER_EMBED_MODEL=granite-278m`, ouvrir,
exiger « dense ». Le modèle s'enregistre, le retard vaut 100 %, le rattrapage
le solde — avec l'index tombé au-delà du seuil de reconstruction, puisque
c'est un gros rattrapage par construction. Le 107m reste disponible dans sa
colonne. C'est exactement le chemin du chantier précédent, sans une ligne de
plus.

Ce qu'il ne faut **pas** faire : réévaluer à chaque ouverture et changer de
modèle courant en silence. Deux colonnes à moitié pleines, et personne ne
sait laquelle a été cherchée.

## 6. Ce qui dirait que ça marche

1. `recommended_model(1 642, 10 Mo, Dedicated 31 Go)` rend 278m ;
   `(62 000, …, Dedicated)` rend 107m « au-delà de 50 000 » ;
   `(1 642, …, Weak 2 Go)` rend 107m « sous 4 Go » ; `(…, None)` rend 107m
   « sans carte ». Chacun avec sa raison, sans base ni carte.
2. La variable posée gagne : `RAG3WEAVER_EMBED_MODEL=granite-278m` sur
   62 000 fichiers donne 278m, et la ligne de montage le dit.
3. Un démon en place qui sert un autre modèle est **gardé** et nommé.
4. Après un premier index en 107m, `RAG3WEAVER_EMBED_MODEL=granite-278m` +
   « dense » exigé : les deux colonnes existent, la recherche prend 278m,
   `embedding_choice` dit toujours 107m et pourquoi.

## 7. Tranché

- ~~**La minute.**~~ Lucie n'a pas tranché sur une attente mais sur deux
  déclencheurs : **50 000 documents**, ou **une carte faible ou absente**
  (§3). Le seuil de 25 Mo de la première version est retiré ; les octets
  restent dans la raison.
- ~~**Le démon en place.**~~ **Il se garde et se dit.** On ne relance jamais
  un démon qu'une autre session peut servir (session architecture, 7 septembre).

Reste à confirmer à l'exécution, pas à trancher : le plancher de **4 Go** est
déduit des mesures de lot, pas mesuré sur une petite carte. Le premier portable
qui passe le dira.
