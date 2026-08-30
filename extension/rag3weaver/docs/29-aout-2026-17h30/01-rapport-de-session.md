# Rapport de session — 29 août 2026, après-midi et soir

Dix commits, de `0ba1d0dc2` à `9f2f2468b`. Le fil conducteur n'était pas prévu :
on est parti d'une question sur l'asynchrone et on a fini par rendre le poste
utilisable pendant les passes de test.

## 1. Ce qui a été construit

### `src/serveur.rs` — démarrer un serveur, et savoir s'il est déjà là

Le moteur savait charger un modèle, exécuter un graphe, tracer un run ; il ne
savait **rien lancer**. Le motif est celui de `Cwd` : *ça ne ment pas*. On ne
demande pas si un processus vit — un pid se recycle, un fichier de pid survit à
son processus — on demande au service s'il répond, **et qu'il réponde en tant
que lui**.

D'où trois états et non deux : `Repond`, `Absent`, et `Occupe(aperçu)` —
quelqu'un répond, ce n'est pas lui. On ne le tue pas, on le dit.

La sonde est en `std` pur, sans `ureq` : la boucle étrange devra lancer un
backend dans une compilation qui n'a aucune raison d'embarquer un client HTTP.
La sortie va dans un **fichier, jamais un tube** — un tube qu'on ne lit pas fige
le serveur, ce qui ressemble à un serveur lent.

### `src/daemon/` — deux processus qui tiennent une ressource rare

- **`embeddings`** : un modèle chargé une fois, servi à plusieurs. Une passe E2E
  rechargeait BGE-M3 sept fois — 2 047 s de chargement contre 1 111 s de tests.
- **`db`** (rag3daemon) : la base qu'un seul processus peut ouvrir, mise
  derrière une adresse.

`mod.rs` porte le partagé : le trait `Service`, `servir`, la sonde, le client.
**`/sante` est répondue par la plomberie, pas par le démon** — c'est la route
dont dépend `Serveur` pour trancher entre `Repond` et `Occupe`, et elle doit
répondre même quand la ressource est occupée.

Deux points qui comptent :

- **`is_mock()` traverse le fil, jamais blanchi.** Un démon posé devant un
  `HashEmbedder` reste factice, sinon on désarmait en silence le garde-fou du
  catalogue.
- **Une valeur de fil étiquetée** plutôt que le serde de `CypherValue` : celui-ci
  est `untagged` et sa variante `Blob` est `#[serde(skip)]` — un blob n'aurait
  pas traversé, **en silence à la lecture**.

### `src/regime.rs` — un nom pour « carte, rythme, rafale »

| | `confort` | `plein` (défaut) |
|---|---|---|
| carte de l'embarqueur | la moins chargée | celle du système |
| rapport cyclique | 60 % | 100 % |
| rafale | 2 048 caractères | 8 192 |

Précédence : **le code l'emporte sur la variable, qui l'emporte sur le régime,
qui l'emporte sur le défaut**. Un régime ne force rien, il fournit ce que
personne n'a dit.

## 2. Ce qui a été mesuré

| | |
|---|---|
| chargement local de BGE-M3 | **4,22 s** |
| attachement à un démon debout | **1,16 ms** puis 5,09 ms |
| deux processus, quarante travaux, par rag3daemon | **20 / 20**, aucun doublon |
| ouverture directe de la base pendant que le démon la tient | **refusée** |
| deux lecteurs en lecture seule sur la même base | **partagée** ✓ |
| un lecteur pendant qu'un écrivain tient | **refusé** ✓ |
| occupation de la carte de calcul, 100 échantillons | **31 %**, 67/100 sous 20 % |
| carte d'affichage pendant toute la passe en `confort` | **0 %** |

Tests unitaires : **902 verts, 0 échec**.

## 3. Ce qui a été trouvé

### Dans le code

- **La crate ne compilait plus sans la feature `code`** — `render_nodes.rs`
  appelait `crate::code_tools` sans garde.
- **`BurnBgeM3Embedder` n'avait pas de `name()`** — inoffensif jusqu'à ce qu'un
  démon en fasse son identité publique.
- **`Rag3dbConnection` n'exposait pas la lecture seule**, la seule forme de
  partage que le moteur offre nativement.
- **`for_role` mentait sur la provenance** : il annonçait la variable
  d'environnement même quand la valeur venait du régime.
- **`e2e_charge_ingestion` mentait sur ses propres réglages** : il lisait les
  variables lui-même, avec ses propres valeurs de repli.

### Sur la machine

- **`status=connected` ne veut pas dire qu'il y a un écran.**
  `card0-HDMI-A-3` se déclare `connected`, `enabled`, `dpms=On` — avec zéro
  octet d'EDID et un seul mode 640×480. Rien n'est branché dessus.
- **Les suites E2E ont besoin de `RAG3DB_SHARED=1`.** Sans lui le cœur est lié
  en statique et aucun de ses 190 symboles n'atteint la table dynamique : un
  `dlopen` d'extension ne résout rien. `run_e2e.sh` le pose ; `cargo test` en
  direct, non.

### Mes propres défauts, dans l'ordre

1. **Deux démons construits sans être branchés.** Le démon d'embedding est resté
   trois heures sans qu'aucun test s'en serve — exactement la faute relevée huit
   fois cette semaine. Corrigé par `tests/common/mod.rs`, un seul point d'entrée
   pour quinze suites.
2. **Le binaire du démon appelait `BurnDevice::default()`** au lieu de
   `for_role`, donc tenait la carte d'affichage. Le commentaire de `BurnRole` le
   disait déjà : *« les mettre tous sur celle qui porte l'affichage marche
   jusqu'au jour où ça ne marche plus »*.
3. **J'allais construire le découpage et le rythme.** Ils existaient, mesurés et
   documentés, dans `record_nodes.rs`. Le défaut n'était pas leur absence,
   c'était leur étage : branchés sur le seul nœud d'ingestion.
4. **Les pauses se multipliaient.** `record_nodes` soufflait après un appel qui
   passait par le démon, lequel soufflait déjà — sur un temps qui contenait donc
   les pauses d'en face. 36 % effectifs au lieu de 60. D'où la règle : **celui
   qui touche la carte souffle** (`Embedder::distant()`).
5. **Un diagnostic erroné annoncé à Lucie** : j'avais conclu à une extension
   périmée là où il manquait `RAG3DB_SHARED=1`.

## 4. Ce qui reste ouvert

- **L'agentique vers Gemini** — la moitié manquante de `confort`. Le choix du
  `Llm` se fait chez l'appelant ; rien n'est déclaré ici pour ne pas créer un
  mécanisme de plus que personne ne lit.
- **L'arbitre entre processus.** Le verrou du démon ne sérialise que les
  embarquements entre eux ; rien n'empêche llama.cpp de prendre la même carte.
  C'était le point de départ de Lucie et il reste entier.
- **Deux suites rechargent encore le modèle** : `e2e_burn_embedder` et
  `e2e_sparse_dump` ont leur propre `LazyLock` au lieu de passer par
  `common::burn`.
- **Le coût réel de `confort` n'est pas mesuré.** La passe a été coupée parce
  qu'elle mesurait la configuration fautive du point 4 ci-dessus. À refaire.
- **Les quatre manques de la file** (issue 03 §8) : état « déposé », bail,
  essais, réveil.
- **[Issue 05](../issues/29-08-2026/05-rag3daemon-execute-du-cypher-pour-qui-atteint-le-port.md)** :
  rag3daemon exécute du Cypher sans authentification. Non bloquant en local, à
  fermer avant qu'un `--adresse 0.0.0.0` traîne.
- **LadybugDB** — la continuation de Kuzu, MIT, v0.19.1 au 4 août 2026, poussée
  quotidiennement. On a forké à Kuzu v0.11.2.2. Nos modifications du cœur :
  **26 fichiers, 520 insertions**. Le repérage de fusion n'est pas fait.

## 5. Comment reprendre après un redémarrage

**Rien à relancer à la main.** Les démons se lancent tout seuls au premier
besoin (`DaemonEmbedder::assurer`), et le premier binaire de test paie le
chargement.

```bash
# la passe complète, poste utilisable pendant
RAG3WEAVER_REGIME=confort ./run_e2e.sh --features daemon --summary

# à plein régime (défaut)
./run_e2e.sh --features daemon --summary

# forcer le chargement local, sans démon — donne l'A/B
RAG3WEAVER_SANS_DEMON=1 ./run_e2e.sh --features daemon
```

Réglages fins, qui l'emportent tous sur le régime :

| variable | effet |
|---|---|
| `RAG3WEAVER_REGIME` | `confort` \| `plein` |
| `RAG3WEAVER_BURN_DEVICE_EMBEDDER` | `gpu:1` — sur ce poste, la carte libre |
| `RAG3WEAVER_GPU_DUTY` | 5 à 100, pourcentage |
| `RAG3WEAVER_EMBED_CHAR_BUDGET` | caractères par appel GPU |
| `RAG3WEAVER_EMBEDDINGS_ADDR` | défaut `127.0.0.1:7878` |
| `RAG3WEAVER_SANS_DEMON` | chargement local |

- Journal des démons : `/tmp/rag3weaver-demons/`.
- Les arrêter : `pkill -f 'rag3weaver-embedding[s]'` — **les crochets sont
  volontaires**, sans eux `pkill -f` reconnaît sa propre ligne de commande et
  tue le shell qui l'appelle (constaté deux fois aujourd'hui).
- Correspondance des cartes sur ce poste : `vulkaninfo` donne `GPU0` = bus 04 =
  `card2` (les deux écrans), `GPU1` = bus 07 = `card0` (libre). Donc `gpu:1`.

## 6. La phrase de la journée

Elle vaut au-delà du synchrone, et elle s'est vérifiée quatre fois :

> Chaque fois qu'on invoque une contrainte comme raison, vérifier que ce n'est
> pas juste l'habitude d'une contrainte levée.

---

# Suite — la nuit du 29 au 30 août

Onze commits de plus, en autonomie. `d38c4c443` → `a85407b25`.

## Ce qui a été livré

| | |
|---|---|
| Les deux dernières suites passent par le démon | plus **aucun** chargement local |
| `BM25Mode::Auto` | une phrase se pèse, un identifiant se contient |
| Les avertissements du moteur remontent | jusqu'à la fiche, même à zéro résultat |
| `--exposer` | un argument ne suffit plus à ouvrir un port |
| Docstrings C, C++, C#, Go | quatre langages qui rendaient `None` |
| `place` | poser un gabarit du catalogue |
| `adopt` | le catalogue apprend du projet |
| Les fiches se relisent à chaque tour | trouvé par le modèle lui-même |

**911 tests unitaires verts**, 0 échec, sur trois jeux de features ; 77 dans
`codeparsers` ; toutes les suites E2E compilent.

## L'issue 02 est résolue, et l'expérience a tranché

Trois questions en français, signal `bm25` seul :

| mode | résultat | scores |
|---|---|---|
| `Contains` (l'ancien défaut) | **0/3** | aucun résultat |
| `ContainsSplit` | **0/3** | −996, −1 992, −4 991 |
| `Parse` | **3/3** | 3,85 · 7,23 · 7,30 |

`ContainsSplit` combine des clauses « contient » en booléen : **sans IDF**, les
mots vides pèsent autant que « prix », et une clause non satisfaite coûte une
pénalité fixe. **Les scores négatifs viennent de là** — c'était l'hypothèse de
Lucie, et elle visait juste.

## L'avis du modèle, et ce qu'il a trouvé

[Le verbatim](../30-aout-2026-04h00/01-avis-du-modele-sur-nos-outils.md) et
[ce qu'on en a fait](../30-aout-2026-04h00/02-ce-que-l-avis-a-donne.md).

**Il a trouvé un défaut bloquant que personne n'avait vu** : `Agent::new`
calculait les fiches une fois, donc l'énumération des cibles de `search` était
figée. Un agent qui posait une entité avec `place` ne pouvait plus la chercher —
et comme cette liste contraint le décodage, il ne pouvait pas même prononcer le
nom. Le défaut était juste sous les deux outils qu'on venait d'ajouter.

Deux leçons de méthode, gravées dans le test : **envoyer la surface réelle**
(catalogue branché, échec sinon), et **un budget de jetons qui compte la
réflexion** — le premier essai a rendu 86 caractères qui étaient la queue d'un
raisonnement, et ça ressemblait à de la concision.

## Les décisions qui t'attendent

Le modèle réclame quatre outils, par ordre d'importance. Ce sont des choix de
périmètre, pas des correctifs :

1. **`run`** — exécuter une commande. Sa critique n°1, deux fois : *« je code à
   l'aveugle »*. `src/serveur.rs` a débloqué la moitié (savoir lancer un
   processus) ; le verbe manque, et il ouvre une question de sûreté.
2. **`delete_file` / `move_file`** — créer se fait par effet de bord de
   `edit(content=…)` ; supprimer et déplacer, pas du tout.
3. **`get_schema`** — on a `generate_full_schema`, non exposé.
4. **Inspecter un gabarit avant de le poser** — `place` ne dit ce qu'il a posé
   qu'après.

Restent aussi : l'exclusivité `content`/`old`+`new` de `edit` non exprimée dans
le schéma (il faudrait un `oneOf`), la file de travaux (§8 de l'issue 03,
délibérément repoussée faute de consommateur), l'agentique vers Gemini, et
l'arbitre GPU entre processus.
