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

## 8. La file, c'est le DAG — et la question d'atomicité se dissout

Lucie, le 29 au soir : *« nos pipelines dag peuvent pas representer assez bien
une queue async ? parcequ'on a la persistence je crois déja sur l'execution
d'un dag »*. Oui — et plus que « assez bien ».

### Ce qui existe déjà, et qui est la moitié difficile

`src/dataflow/checkpoint.rs` et `checkpoint_store.rs`, branchés, utilisés par le
drain du catalogue. Un `ExecutionCheckpoint` persisté (`CypherCheckpointStore`)
porte le statut du run, **le statut de chaque nœud** (`Pending` / `Completed` /
`Failed`), **les sorties sérialisées** des nœuds terminés, les `initial_inputs`,
un `graph_hash` qui refuse une reprise si le graphe a changé, et un
`undo_context` par nœud. Et `find_incomplete()` — exposé sous
`Catalog::check_pending_checkpoints`, avec `drain_resume()` en face.

**Le travail, c'est le DAG.** Et il y a là une propriété qu'aucune file du
commerce ne peut avoir : BullMQ, Celery et `apalis` rejouent un travail **depuis
le début**, parce qu'ils ne savent pas ce qu'il contient. Nous reprenons au
premier nœud incomplet, sorties précédentes restaurées. C'est strictement mieux,
et c'est tout l'intérêt d'un graphe plutôt que d'une fermeture opaque.

### La question ouverte, tranchée par l'expérience

`tests/e2e_prise_atomique.rs` — deux faits mesurés, pas déduits :

- **Huit fils, une connexion, quarante travaux : 40 prises, 40 travaux
  distincts, 0 doublon, 0 erreur.** L'atomicité vient de ce que le moteur C++
  sérialise les appels traversant une `rag3db::Connection` — pas d'une
  isolation transactionnelle. C'est le cas facile, et il marche.
- **Un second processus ne peut pas ouvrir la même base** : l'enfant rend 3
  (refusé). `LocalFileSystem::openFile` pose un `F_WRLCK` en `F_SETLK`, non
  bloquant, donc échec immédiat.

Et dans le moteur, `TransactionManager::beginTransaction` **refuse** une seconde
transaction d'écriture : `enableMultiWrites` vaut `false`, et le seul réglage
qui le relâche s'appelle `debug_enable_multi_writes` — le nom dit son statut.
Côté Rust, `Rag3dbConnection` n'expose même pas de quoi ouvrir une seconde
connexion sur la même base.

**Donc la question se dissout.** Il ne *peut pas* y avoir deux preneurs
concurrents sur notre base. Une file adossée au `CheckpointStore` n'a besoin
d'aucune atomicité en base : elle a besoin d'un **arbitre unique**, et c'est un
processus — celui du §7, qu'on sait maintenant lancer, retrouver, et reconnaître
en tant que lui.

### Ce qui reste à faire, et c'est du bookkeeping

Dans un processus mono-écrivain, les quatre manques deviennent faciles :

1. **Un état « déposé »** — `CheckpointExecutionStatus` n'a que `Running`,
   `Completed`, `Failed` ; il manque le travail créé avant que quiconque le
   prenne. Une variante d'enum.
2. **Le bail** — `Running` ne se distingue pas de « tournait sur un processus
   mort ». Propriétaire + échéance, et le battement de cœur est gratuit puisque
   l'arbitre est vivant.
3. **Le compteur d'essais et le rebut** — sans quoi un graphe empoisonné se
   rejoue indéfiniment.
4. **Le réveil** — presque fait : `reactor.rs` a déjà minuteurs et sonnettes.

**Et donc pas de file du commerce.** Une file externe nous ferait perdre la
reprise par nœud pour gagner quatre champs et un service à faire tourner.
Mauvais échange. Si un jour plusieurs machines entrent en jeu — le critère de
déclenchement, et il n'est pas rempli — ce serait `pgmq` ou `apalis-sql` sur le
Postgres déjà déclaré, pas Redis : zéro service en plus.

## 9. rag3daemon — le processus qui tient la base

Décidé le 29 au soir : **pas Postgres.** Ce n'est pas ce qu'on recommandera, et
le brancher après coup n'est pas difficile — `DbConnection` et `SchemaDialect`
sont la couture, et `PostgresDialect` fait déjà 944 lignes qui compilent (sans
aucun test E2E, ce qui reste à savoir un jour). On garde la base embarquée et on
enlève sa contrainte au bon endroit.

**La contrainte, et la réponse.** Une base rag3db ne s'ouvre que par un
processus : `F_WRLCK` en `F_SETLK`, refus immédiat pour le second. C'est la
propriété d'une base embarquée, pas un défaut — SQLite et DuckDB font pareil, et
la réponse est la leur : **mettre le processus qui tient le verrou derrière une
adresse**. Un seul écrivain, plusieurs programmes qui lui parlent.

- `src/daemon/mod.rs` — la plomberie partagée : le trait `Service`, `servir`,
  la sonde, le client. **`/sante` est répondue par la plomberie, pas par le
  démon** : c'est la route dont dépend `Serveur` pour trancher entre `Repond` et
  `Occupe`, et elle doit répondre même quand la ressource est occupée. Un démon
  qui embarque un gros lot ne doit pas paraître mort.
- `src/daemon/db.rs` — rag3daemon et son client `DaemonConnection`, qui
  implémente `DbConnection` : un `Catalog` le prend tel quel, rien dans le
  moteur ne sait que la base est ailleurs.
- `src/bin/rag3daemon.rs` — `--adresse`, `--base <chemin|:memoire:>`.

**Une valeur de fil, et pourquoi.** `CypherValue` est `#[serde(untagged)]` et sa
variante `Blob` est `#[serde(skip)]` : un blob **ne traverserait pas**, en
silence à la lecture. Ce codage sert la configuration, où les types se devinent ;
un fil de base demande l'inverse. D'où `ValeurFil`, étiquetée, blobs en base64,
et un test qui fait passer les huit variantes par le démon.

**Mesuré** (`tests/e2e_rag3daemon.rs`) : deux processus, quarante travaux,
**20 / 20**, aucun doublon — et dans le même test, l'ouverture directe de la
base pendant que le démon la tient est toujours **refusée**. C'est la scène
exacte de `e2e_prise_atomique.rs`, rejouée avec le démon au milieu.

**Ce que ça débloque.** L'arbitre de la file existe maintenant : un processus
unique qui tient la base et à qui tout le monde parle. Les quatre manques du §8
— état « déposé », bail, essais, réveil — sont du bookkeeping à l'intérieur de
ce processus.
