# Un appel d'outil attend derrière un index sans le dire

- **État** : corrigé (20 novembre 2026), par ses quatre lots, tous sur master : `arbre-verrou-invocation`
  (14247c0856, une invocation lit sa cellule par la vue, sans le mutex du catalogue), `arbre-strict-borne-2` (9710cea8c0,
  une recherche `Strict` dit « occupé » au lieu d'attendre derrière un index), `arbre-garde-dette-ouverture`
  (34179694fc, la garde : un premier appel n'attend pas une dette de vecteurs) et `outil-pendant-index` (05c3b343ce,
  la part de l'hôte de tty : un appel n'attend plus sans borne le slot d'un backend qu'une ouverture tient).
- **Gravité** : blocage (un agent reste suspendu des minutes, sans réponse ni délai, jusqu'à ce qu'on l'interrompe)
- **Atteignable en service** : oui, dès qu'un index (ou une réindexation de la montre) court sur le backend qu'un agent interroge
- **Touche rag3weaver** : oui (verrou du catalogue, service d'embarquement, réponses des outils)
- **Famille** : produit

## Ce que c'est

Le 11 octobre à 20 h 16, `code-principal` (hôte de luciepc, binaire master faa00f279) commence sa
réindexation. Le cœur interroge ce même backend : `search_code {signals: [bm25]}` puis `usages` sur
un fichier frais restent **ouverts environ deux minutes, sans réponse, sans « occupé », sans délai**,
jusqu'à ce que Lucie les interrompe. À 20 h 20, un `usages` de la session codeparsers ne rend rien
en 60 s. Pendant ce temps :

- l'index rebâtit le plein texte depuis les lignes en tenant le catalogue ;
- l'index vectoriel « tombe le temps du rattrapage », puis se reconstruit ;
- l'index tourne à 98-99 % des vecteurs plusieurs minutes, le total bougeant (120 316, 120 321,
  120 317) : la montre réindexe les fichiers déposés puis retirés, l'index rattrape sans fin.

Ce n'est pas le « Catalogue occupé : une indexation le tient » de 18 h, qui, lui, **répondait**.

## Ce qu'il faut pour le fermer

- **Un appel d'outil ne reste jamais ouvert** : au-delà d'un délai court, il répond « occupé par un
  index (N %, depuis N s) », ou sert le dernier état lisible, en le disant.
- **Les appels d'outil passent avant les lots d'index**, au verrou du catalogue comme au service
  d'embarquement (la requête d'une recherche ne fait pas la queue derrière des lots de 128).
- **Les deux questions de Lucie** :
  - le signal lexical d'un fichier frais peut-il être servi avant ses vecteurs (le texte est là dès
    l'écriture) ?
  - un fichier relié en 6 s (mesure du cœur, 20 h 10) : où passe le temps — la montre (`settle_ms`,
    l'intervalle de balayage), l'extraction, l'écriture, le catalogue ? Un relevé par étape
    manque.
- Témoin : un index long sur un backend, un `usages` pendant ; la réponse arrive en moins de N s,
  « occupé » ou servie.

## Un second cas : l'ouverture d'un backend qui rattrape ses vecteurs (11 octobre, 20 h 35)

Mesuré par la mémoire longue en redémarrant l'hôte de Lucie (binaire e9d690915, lib de
20 h 08) : le **premier `initialize`** d'un client sur `code-rag3db` a pris **26 s**. Le journal
de l'hôte annonçait juste avant un rattrapage de **2 074 morceaux** à embarquer sur
`Scope_Chunk` (l'index vectoriel tombe le temps du rattrapage) : le backend se rouvre et
rattrape **avant** de répondre au premier appel. Les appels suivants : `daemon_status`
instantané, `usages` 0,46 s. C'est la même forme que le cas de l'index : le rattrapage se fait
derrière une réponse qui dit le retard (« vecteurs de N morceaux en rattrapage »), pas avant
elle — le client attend 26 s sans un mot, Claude Code reste sur « Connecting… ».

## Relevé du 19 novembre 2026 (session tty)

Le chemin d'un appel, de `DaemonHost::call_with_deadline` à `Backend::call_on`, relevé sur master 50b1ff4a2. Chaque attente avec sa borne :

- **Corrigé, côté hôte** : un appel attendait sans borne le slot d'un backend qu'une ouverture tenait en écriture (`with_backend`, `hosted.backend.read()`) ; un second appel arrivé pendant une ouverture de 26 s attendait les 26 s. Il lit maintenant le slot avec une borne de 5 s (`SLOT_READ_WAIT`). Au-delà, il répond « <manifeste> s'ouvre depuis N s » (ou qu'une fermeture ou une réouverture le tient), sans rien faire. Le témoin `e2e_demon_hote::a_call_during_a_long_open_says_the_backend_is_opening` a été vu rouge (d985884ee9 : 14,7 s d'attente sur une ouverture de 15 s), puis vert ; la suite `e2e_demon_hote` reste verte (27).
- **Déjà borné** : la lecture du catalogue (`lock_for_read`, 1 s, puis « Catalogue occupé »), la poignée de `catalogue_vivant` (1,5 s), une connexion de lecture (`read_pool`, 5 s), l'attente d'un index en retard (`search.index_wait_s`, 6 s ; un index entier en cours répond tout de suite `indexing: true`).
- **Reste, pour l'arbre principal** :
  - **Le premier appel, celui qui ouvre, attend l'ouverture entière** : le schéma, `rebuild_detached_vector_indexes` → `ensure_vector_index` (le HNSW rebâti s'il est détaché), les réactions, `seed`, la comparaison des fichiers (`Backend::open_with_reason`). Le rattrapage des vecteurs, lui, part en fond (`worktrees::a_l_ouverture`). Les 26 s du `initialize` viennent sans doute du rebâti du HNSW : à mesurer, puis à faire passer derrière une réponse qui dit le retard.
  - **Des `catalog.lock()` sans borne hors du chemin de la vue** : `reactions_de_la_cellule_on` et `build_services` (une session dont la cellule diffère de la vue lue, ou sans vue), `invoke_read_on`, et le repli « L4 : sans vue » des nœuds de recherche (`generic_search_nodes.rs`, `code_nodes.rs`, `search_nodes.rs`). L'index tient le catalogue toute sa phase de texte (`sync_source`) et chaque passe de vecteurs.
  - **La requête d'une recherche fait la queue au service d'embarquement** derrière le lot en cours (le verrou `passe` par lot, sans priorité) ; elle est bornée à 60 s (`RAG3WEAVER_EMBED_TIMEOUT_SECS`). C'est à la session embarquement de dire si elle doit passer avant les lots.

## Relevé d'arbre principal, 19 novembre 2026 : les `catalog.lock()` hors du chemin de la vue

Relevé statique sur master (`e0784a0d23`+), les trois fonctions et les replis nommés par tty. **En usage normal**
(une vue de lecture publiée, un appel dans la cellule du catalogue), **un appel d'outil ne prend pas le verrou du
catalogue**. Deux chemins le prennent quand même, et attendent donc un index qui tient le catalogue :

1. **`Backend::invoke_read_on`** prend le verrou **à chaque invocation**, seulement pour lire
   `scope()`. La vue de lecture porte la même cellule. C'est le seul chemin d'usage courant qui bloque, et il se
   corrige en lisant la cellule dans la vue.
2. **Une recherche en cohérence `Strict`** (`generic_search_nodes.rs`, le repli « Strict, ou sans vue ») prend le
   verrou par construction : `Strict` vide la file d'abord. Elle doit dire « occupé » au-delà d'un délai, au lieu
   d'attendre en silence.

Les autres prises ne servent que hors de l'usage courant :

- `build_services` : pour un appel dans une autre cellule que celle de la vue, ou sans vue publiée ;
- `reactions_de_la_cellule_on` : pour une autre cellule pas encore montée ;
- les replis « L4 : sans vue » de `code_nodes.rs`, `schema_nodes.rs`, `field_weight.rs` et
  `composable_results.rs`.

« Sans vue publiée », c'est l'ouverture : avant le premier `publish_read_view`. Elle rejoint le point 1 de tty,
l'ouverture entière avant le premier appel.

**Ce qui reste à mesurer** : où passent les secondes de l'ouverture (schéma, rebâti du HNSW détaché, réactions,
seed). Il faut une copie fidèle d'une vraie base, l'hôte arrêté pour elle ou par un instantané de G. Demandé à c7.

**Verdict pour l'ordre** : pas de blocage franc sous un usage normal, hors invocation et `Strict`. Le lot 6 de 2a′-3
passe devant, puis ce ticket : `invoke_read_on` par la vue, `Strict` borné, et l'ouverture mesurée puis découpée.

## La mesure de l'ouverture, 19 novembre 2026 (arbre principal, à luciepc)

Sur la copie fidèle de code-rag3db que G a faite (instantané du portable pris à 10 h 04, hôte arrêté, base de 3,6 Go,
`mesures/code-rag3db-parent-ouverture-20261119`). Une copie reflink par relevé, sous `poste mesure`, avec le binaire
posé `rag3weaver-backend` et la lib 7c2b506ec. Le temps compte l'ouverture, un `describe` et l'arrêt propre :

| relevé | ouverture + describe + arrêt |
|---|---|
| avec les réactions et la veille du workspace | 2,6 s |
| sans réactions ni veille | 1,2 s |

`models.llm` est retiré des copies : il déclare Vertex sans `GOOGLE_CLOUD_PROJECT`, que l'hôte tient de son
environnement, et l'ouverture échouait en 0,6 s.

**Pas encore un verdict**, pour deux raisons :

- **L'extension VECTOR notée à côté de la base ne charge pas** : le `.extensions` de la copie pointe sur celle de la
  lib 3d706f709, et le backend dit « une extension notée à côté de la base n'a pas pu être chargée à l'ouverture ».
  Le rebâti du HNSW détaché, principal suspect des 26 s du 11 octobre, n'est peut-être pas exercé ici.
- **La copie n'est pas encore prouvée** par son nombre de documents, comparé à la base en service.

Ce que le relevé dit déjà : sans rattrapage de vecteurs, l'ouverture d'une base de 3,6 Go tient en une à trois
secondes, réactions comprises. Les 26 s du 11 octobre accompagnaient un rattrapage de 2 074 morceaux annoncé au
journal, et dépendent donc de la dette de la base, pas de sa taille.

### Le verdict de la mesure (19 novembre 2026, 14 h)

- **L'extension notée chargée** : dans une copie de mesure, le `.extensions` est repointé sur la lib 7c2b506ec, ce
  que le relevé dit. Les temps ne bougent pas : 2,6 s avec les réactions et la veille, 1,2 s sans, en comptant
  ouverture, `describe` et arrêt.
- **La copie est prouvée** par `index_state`, relevé sur une copie à part :
  - le plein texte a autant de documents que de lignes (File 6 508, Library 400, Scope 68 585, Symbol 75 745) ;
  - les vecteurs sont prêts à 100 % ;
  - la maintenance du matin n'a trouvé aucun orphelin.

**Verdict** : l'ouverture d'une base saine de 3,6 Go, sans dette de vecteurs, tient en 1 à 3 s, réactions et veille
comprises. Ni le chargement de l'extension, ni un rebâti du HNSW n'y pèse. Les 26 s du 11 octobre accompagnaient un
rattrapage de 2 074 morceaux à embarquer : c'est la **dette** qui fait attendre le premier appel, pas la taille de la
base.

Le reste du point 1 de tty se réduit donc à ceci : **le rattrapage d'une dette de vecteurs doit se faire derrière une
réponse qui dit le retard, et non avant le premier appel**. C'est déjà la forme que ce ticket demandait en octobre.
Un témoin qui l'éprouve doit ouvrir une base **avec** une dette ; c'est le dernier point ouvert de ce ticket, avec les
lots déposés (`arbre-verrou-invocation`, `arbre-strict-borne`) et la part hôte de tty.

## Le dernier point : un premier appel devant une dette de vecteurs (19 novembre 2026, 16 h 20)

**Le témoin** : `daemon_host::worktrees::tests::a_first_call_does_not_wait_for_a_debt_of_vectors`, déposé comme garde
(`arbre-garde-dette-ouverture`, branche `arbre-principal/dette-a-l-ouverture`). Il reproduit la condition du
11 octobre :

- une base sur disque, et un index coupé qui laisse une dette de vecteurs sous un service d'embarquement lent ;
- le backend fermé, puis 20 fichiers changés et 5 retirés, pour que la réconciliation de départ de la montre les
  réindexe par la réaction ;
- la réouverture par l'hôte avec `catch_up_on_open`, puis le premier appel.

Il exige trois choses :

- l'ouverture et le premier appel en moins de 5 s ;
- l'état qui dit les vecteurs encore dus ;
- le rattrapage qui continue derrière.

**Il est vert sur master**, avec et sans la réconciliation : 10 s de test, l'ouverture et le premier appel sous la
borne.

**Pourquoi le 11 octobre attendait** : le binaire d'alors (e9d690915) datait d'avant L2 (12 octobre) et L4 (18 octobre).
`call_in` prenait le mutex du catalogue deux fois avant la vue, dans `reactions_de_la_cellule` puis dans
`build_services`, c'est-à-dire le témoin « un appel par la cellule du catalogue n'attend pas l'écrivain » du
12 octobre. Ce mutex, la réaction de la montre et le rattrapage de 2 074 morceaux le tenaient : le rattrapage
s'annonçait au journal juste avant (« l'index tombe le temps du rattrapage »). Depuis :

- les appels lisent par la vue et l'instantané, sans ce mutex ;
- `index_states` ne fait qu'un `try_lock` ;
- le rattrapage à l'ouverture tourne en fond (`catch_up_on_open`, la forme (b) du 8 novembre).

Le point est clos par la garde. Le ticket se ferme quand ses lots sont sur master : `arbre-verrou-invocation`,
`arbre-strict-borne`, la garde, et la part hôte de tty (`cx-outil-pendant-index`).

## La fermeture (20 novembre 2026)

Les deux cas du ticket sont couverts, chacun par un témoin sur master :

- **un appel pendant un index** : il lit par la vue et l'instantané, sans le mutex du catalogue
  (`backend::reload_tests::an_invocation_does_not_wait_behind_the_catalog_lock`), et une recherche `Strict` qui
  trouverait l'écrivain dit « occupé » (`dataflow::generic_search_nodes::tests::a_strict_search_says_busy_instead_of_waiting_behind_an_index`) ;
- **un premier appel devant une dette de vecteurs à l'ouverture** : l'ouverture et l'appel tiennent sous 5 s, l'état
  dit les vecteurs dus et le rattrapage continue derrière
  (`daemon_host::worktrees::tests::a_first_call_does_not_wait_for_a_debt_of_vectors`) ;
- **côté hôte**, un appel arrivé pendant une ouverture n'attend plus sans borne son slot (tty, `outil-pendant-index`).
