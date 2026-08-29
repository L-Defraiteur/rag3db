# Un démon, et la fin du tout-synchrone

**Ouvert le 29 août 2026.** Pas un défaut : une contrainte qui a survécu à sa
raison d'être, et un manque qu'on vient de rencontrer trois fois.

## 1. Ce qui manque : on ne sait pas démarrer un serveur

Le moteur sait charger un modèle, exécuter un graphe, tracer un run. Il ne sait
**rien lancer**. Trois besoins l'ont montré en trois jours :

- **Un démon d'embedding.** Une passe E2E recharge BGE-M3 **sept fois**. Un
  chargement seul prend 3 à 7 s ; sept qui se marchent dessus prennent 531,
  465, 341, 280, 175, 151 et 55 s — **2 047 s de chargement contre 1 111 s de
  tests**. Ce n'est pas le chargement qui est lent, c'est la concurrence : sept
  processus tirant chacun 2,2 Go du disque vers la même carte.
- **Le terminal à plusieurs.** Il tiendra des agents vivants entre deux
  commandes. C'est un démon, avec un client qui s'y attache.
- **La boucle étrange** ([doc 08](../../vision_roadmap_08_2026/08-des-catalogues-de-gabarits.md)).
  Un agent qui pose un gabarit de backend doit pouvoir le **faire tourner** —
  sinon on s'arrête à écrire des fichiers, et le critère de réussite (« un
  backend debout qu'on ouvre ») est hors de portée.

Ce qu'il faut, minimalement : lancer un processus, savoir s'il répond déjà,
l'attendre, et le retrouver au prochain démarrage. *Vérifier si un démon
tourne, et sinon le lancer* (Lucie).

## 2. `burn-remote` ne répond pas à ça

Vérifié avant de le proposer : il transporte des **opérations de tenseur**
(`Task::RegisterOperation`, `RegisterTensor`), c'est-à-dire de l'exécution
déportée. Chaque client chargerait toujours les poids **et devrait les
envoyer**. Pour « charger une fois, servir plusieurs », il faut un démon à
nous, avec une API `embed` — pas des tenseurs.

## 3. Le synchrone n'a plus de raison d'être

`Node::execute` est synchrone, donc le trait `Llm` l'est, donc le client cloud
est `ureq` bloquant, donc un appel LLM occupe un fil entier le temps de la
réponse.

**Pourquoi c'était ainsi**, et ce n'est plus vrai — doc 07 §6, 26 août :

> **Wasm abandonné pour rag3weaver** : ça ne servait qu'à se contraindre — pas
> de fils, pas d'async — pour un usage que personne n'avait.

La contrainte a été levée il y a trois jours. Le code n'en a pas pris acte : on
continue de payer une discipline dont le motif a disparu. Le `tiny_http` que
je venais d'ajouter était choisi « symétrique d'`ureq`, parce que le moteur est
synchrone » — un raisonnement correct sous une prémisse périmée.

## 4. Ce qui en profiterait, par ordre de gain

- **N appels cloud sans N fils.** Le commentaire du `Cargo.toml` le prévoyait
  déjà : *« quand on voudra N appels cloud sans N fils, ce sera un client async
  sur tokio »*. Trois agents en parallèle, c'est trois fils bloqués sur une
  socket aujourd'hui.
- **Le terminal à plusieurs.** Des agents qui vivent, qui s'interrompent, qui
  attendent une approbation humaine — c'est exactement ce que l'async sert.
  Aujourd'hui `Agent::run` monopolise son fil.
- **Un démon qui sert plusieurs clients** sans un fil par requête.
- **Les outils asynchrones** (doc 10 du 26 août) : un appel rend un accusé,
  le résultat arrive plus tard. Le mécanisme existe côté fiche (`%% async`) ;
  l'exécution, elle, est bloquante.
- **L'ingestion.** 153 s sur 165 dans la phase « entités », GPU à 2 % —
  découpage, embedding et écriture s'y attendent l'un l'autre.

## 5. Ce que ça coûte, et où le sync reste bon

À dire avant de s'engager :

- **`Node::execute` est le point de bascule.** Le rendre `async` teinte tout le
  dataflow, et c'est un chantier, pas une correction.
- **Tokio est déjà là**, pour attendre : réacteur, minuteurs, bus. Ce n'est
  donc pas une dépendance nouvelle, c'est un usage élargi.
- **Le calcul reste synchrone à raison.** Embarquer, découper, écrire en base :
  rien n'y attend, tout y calcule. L'async sert ce qui **attend** — le réseau,
  un modèle distant, un humain — pas ce qui travaille.
- **Une voie intermédiaire existe** : garder `Node::execute` synchrone et
  n'introduire l'async qu'aux frontières qui attendent (le client LLM, le
  démon, le terminal). Moins ambitieux, réversible, et ça débloque les trois
  besoins du §1.

## 6. Ce qu'on décide

Rien encore. Ce document existe pour que la prémisse périmée ne se reproduise
pas dans un troisième choix technique : **le synchrone n'est plus un principe,
c'est un état de fait**. Chaque fois qu'on l'invoque comme raison, il faut
vérifier qu'il n'est pas juste l'habitude d'une contrainte levée.

La question ouverte, et elle est franche : commence-t-on par la voie
intermédiaire (§5), qui rend le démon possible cette semaine, ou par le
chantier `Node::execute`, qui ferme le sujet mais coûte plusieurs jours ?

## 7. Ce qu'on a décidé, et fait — le 29 août au soir

**Les deux questions étaient collées, et elles sont indépendantes.** Lancer un
serveur ne demande pas d'async : lancer un processus, le sonder, l'attendre,
c'est de l'attente *bornée et rare*, et le synchrone y est le bon outil.
Le démon n'attendait donc pas la décision du §6. C'est ce qui a été fait :

- **`src/serveur.rs`** — démarrer un serveur et savoir s'il est déjà là. Le
  motif est celui de `Cwd` : *ça ne ment pas*. On ne demande pas si un
  processus vit — un pid se recycle, un fichier de pid survit à son processus —
  on demande au service s'il répond, **et qu'il réponde en tant que lui**.
  D'où trois états et non deux : `Repond`, `Absent`, et `Occupe` — quelqu'un
  répond, ce n'est pas lui ; on ne le tue pas, on le dit.
- **`src/daemon.rs`** — le démon d'embedding et son client, qui se fait passer
  pour un `Embedder` ordinaire. Le point qui compte : `is_mock()` est **relayé,
  jamais blanchi**. Sans ça, poser un démon devant un `HashEmbedder` désarmerait
  en silence le garde-fou de `Catalog::register_entity` (issue 01).
- **`src/bin/rag3weaver-embeddings.rs`** — le binaire, sous
  `required-features` : un binaire qui manque une feature doit être *absent*,
  pas cassé.

**Mesuré** (`tests/e2e_demon_embeddings.rs`, vrais poids BGE-M3) : 4,2 s pour le
premier client, **599 µs** pour le second. Et ce n'est pas la bonne mesure du
gain : le 4,2 s est un chargement *seul*. Le gain réel se compte contre le cas
concurrent — 531, 465, 341, 280 s — que le démon supprime en le remplaçant par
une file. **La file d'attente est le remède, pas un défaut.**

`tiny_http` reste, mais pour une raison qui tient : le travail de ce serveur est
du **calcul GPU pour peu de clients**, et l'async n'y achèterait rien. La
première justification, elle, a été réécrite dans le `Cargo.toml` — c'était
exactement le piège que ce document existe pour éviter.

**Ce qui reste ouvert, et n'est plus bloquant** : le client LLM (N appels sans
N fils) et le terminal à plusieurs. Le patron existe déjà — `reactor.rs` héberge
un runtime tokio `current_thread` dans un fil nommé et rend une poignée
synchrone. La voie intermédiaire n'est pas un compromis inventé pour
l'occasion : c'est la forme que ce code a déjà choisie une fois.

**Deux prémisses périmées restantes**, notées pour qu'elles ne se rejouent pas :
`async-trait` est déclarée dans le `Cargo.toml` et **utilisée nulle part** ; et
`src/events.rs:3` dit encore « `async_broadcast` for WASM-compatible
broadcasting ».
