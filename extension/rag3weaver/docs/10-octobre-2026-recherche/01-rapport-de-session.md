# Session recherche — rapport (chantier C : l'exécution asynchrone des graphes)

## 23 h 15 — LE LOT C EST FUSIONNÉ : master `afdc6923e`

Avance rapide après DEUX rebases de course (master prenait l'IR
Select/Hop, le moteur de script du chantier I, un refactor recherche
pendant mes rejeux) ; un seul conflit (Cargo.toml, la ligne tokio contre
les dépendances script), rejeu final **1361/1361** et toutes les cibles
compilent. Les signes sont envoyés : 73 rebase commande-en-fond-3 dessus,
96 monte Reactor::watch_bound après sa fusion, « everything declarative »
rejoue ses noms possédés par-dessus et a reçu son message d'entrée sur
src/dataflow/ (relecture de reactor.rs/runtime.rs par moi avant chacun de
ses commits qui les touchent).

**La mesure (fenêtre exclusive accordée par l'orchestration)** — avant =
`001054116`, après = la tête rebasée, banc réparé `aeec6888f`, granite-278m
+ bge-m3 : l'asynchrone ne coûte RIEN de visible. Banc 441,5 s avant
(passe stabilisée ; la première, 689 s, est le contrôle à froid après
bâti) contre 448,3 / 445,0 s après (~+1 %, bruit) ; MRR au millième
(0,456→0,455 ; 0,467→0,466 ; 0,433→0,435), « tel quel » identique
(0,440 / 14 / 27) ; latences 127-149 → 133-146 ms. `test_backend_code`
HORS comparaison (rouge aux deux bouts pour des raisons différentes, voir
20 h 45). **Moteur : le MÊME `.so` aux quatre passes** (bâti 21:26, âges
0/15/22/30 min dans les lignes d'âge). Précision demandée par
l'orchestration : les quatre passes (jusqu'à 22 h 40) ont tourné sur une
extension vecteur (`libvector.rag3db_extension`, arbre principal) rebâtie
à 21 h 26 par un autre bâti (le banc, vraisemblablement) — SANS effet sur
les chiffres : les lignes d'âge prouvent que les deux colonnes ont partagé
le même artefact (21:26, âges 0/15/22/31 min).

**Trouvaille de méthode, vérifiée par rag3db-96 contre ses journaux** :
`RAG3DB_BUILD` non posée + un worktree qui a bâti = `run_e2e.sh` se replie
EN SILENCE sur le moteur du worktree (`BUILD="${RAG3DB_BUILD:-$ROOT/build/lecteurs-csv}"`)
— on mesure son propre moteur sans le savoir. Sans conséquence ici (C++
identique à mes deux bouts, `d10b92306` dedans : la réserve MOTEUR_ANCIEN
tombe), et la ligne d'âge l'attrape parce qu'elle imprime le CHEMIN, pas
seulement la date. Garde-fou suggéré à 73 (dire le repli, ou refuser
`ROOT` sans `BUILD`/`LIBRARY_DIR`).

**L'autopsie des 307 Go** (target-C effacé à la demande de l'orchestration,
disque 92 % → 67 %) : deps = 302 Go, debug seul, incremental bien à 0 —
171 binaires e2e d'environ 1,4 Go chacun (`cargo test --tests --no-run`
toutes-features les bâtit TOUS, et deux fois après les rebases qui
changent les empreintes), moteur C++ lié EN STATIQUE dans chacun
(RAG3DB_SHARED absent du bâti). Règles qui en sortent : vérifier les
cibles par `cargo check --tests` (dix fois plus petit), ne bâtir en
binaire que les suites jouées, et RAG3DB_SHARED=1 sur les bâtis de test.

Reste ouvert, hors lot : e2e_postgres vivante (le conteneur pgvector
attend Lucie ; F la joue dès qu'il existe — fusionné avec cette limite
dite, actée par l'orchestration) ; persistence/sparse (tampon 256 Mio)
chez 73 ; la suite de C (§4 de la page 02 : ports-flux, puits
d'affichage, run en fond, parallélisme async) attend ses pages.

## 20 h 45 — la batterie est rendue : tout vert sauf trois rouges DE MASTER

Reprise après le nettoyage : worktree `rag3db-async` remonté sur
`execution-asynchrone-2` (`a0c3e809c`), target unique
`~/.cache/rag3weaver-build/target-C` (`CARGO_INCREMENTAL=0`, -j8,
`timeout 1800` devant tout cargo test — règles du soir de l'orchestration ;
le target se garde entre les lots, effacé au-delà de 60 Go ou en fin de
chantier).

**La batterie complète du lot** (régime doux, `SANS_CARTE_LOCALE`, embed
service 7979-7981, lib commune de l'arbre principal) :

- lib 1261/1261 ; `e2e_code` (838 s), `e2e_dataflow_observe`,
  `e2e_checkpoint` VERTS ;
- backend : `harness`, `lifecycle_batch`, `must_reopen`, `snapshot`,
  `mcp_render` VERTS (mcp : module réinstallé dans un venv durable,
  `~/.cache/rag3weaver-build/venv-mcp` — l'ancien est parti au nettoyage) ;
- chat : `test_chat_app` et `test_chat_must_reopen` VERTS — l'étape 4
  (bus + boîte) tient en suite ;
- **trois ROUGES, prouvés DE MASTER** (rejoués au commit de base
  `001054116`, même montage — journaux `isole_*` au scratchpad, signalés à
  l'orchestration qui route) : `test_backend_persistence` et
  `test_backend_sparse` (« Buffer manager exception » sous leur tampon de
  256 Mio — suspect : `1b0d5483b`, paquets de 2 048) ;
  `test_backend_code` (sur master pur la section « Avant d'éditer » manque
  — vieille fixture filtrée ; sur ma branche, fixture réparée, le compte
  passe de « 1 directement » (socle 27eeb6a7f, vert ×2 avant-pause) à
  « 2 directement » (socle 001054116) — suspect : `1990769aa`, voisinage
  par la forme Hop). `test_backend_code` SORT de la comparaison de mesure :
  rouge aux deux bouts pour des raisons différentes.

**Deux pièges de montage à savoir** (coût : une demi-batterie) : un
worktree neuf n'a PAS `extension/vector/build` (piège 6 — symlink vers
celui de l'arbre principal posé, il survit aux checkouts) ; et
`rag3weaver-chat` exige la feature `openai-llm` en plus de la liste
habituelle. Et `poste lourd` est un verrou PARTAGÉ : deux de mes passes ont
pu se chevaucher pendant un checkout — toute passe qui bascule le worktree
doit tenir ses suites DANS la même invocation de `poste`.

Reste : la réponse de l'orchestration sur les trois rouges, puis la tenue
de mesure (gel de la lib annoncé à rag3db-91 juste avant), la fusion, les
signes.

## 18 h 20 — nettoyage du disque : TOUT est poussé, batterie à rejouer

L'orchestration efface les target cargo et les worktrees (disque saturé).
État exact au moment de l'arrêt :

- **Branche `execution-asynchrone-2`, tête `a0c3e809c`** — rebase sur
  master `001054116` (= le « avant » des mesures), lib REJOUÉE après le
  rebase : 1261/1261. Contenu depuis la 1 : fix RunScope scellé
  (`50a585028` — l'interblocage attrapé par l'orchestration, preuve au
  core, 200/200 tours + témoin déterministe), postgres (`d6b102262` —
  runtime possédé, constructeurs synchrones, bras Typed + denuder),
  chat bus+boîte (`a0c3e809c`). La branche `execution-asynchrone` (hashes
  d'avant rebase, tête `ca757a315`) reste en ligne : `commande-en-fond-3`
  de l'arbre principal est bâtie dessus.
- **Interrompu : la batterie complète** (script
  `scratchpad/batterie.sh` de ma session) — bâti des bins VERT (879 s),
  arrêtée pendant le bâti des e2e. À REJOUER ENTIÈRE après le nettoyage :
  e2e_code, e2e_dataflow_observe, e2e_checkpoint, 8 suites backend,
  2 suites chat (régime doux + SANS_CARTE_LOCALE de jour, embed service
  7979-7981, lib de l'arbre principal, timeout 1800 partout).
- **Puis les mesures**, au créneau demandé à l'orchestration après le
  « fini » de la batterie de l'arbre principal : une tenue exclusive,
  « avant » = `001054116`, « après » = `a0c3e809c`, banc réparé de
  `aeec6888f`, MÊME lib commune des deux côtés (celle de 17 h 36 —
  MOTEUR_ANCIEN assumé et dit, seul écart `d10b92306`, statistiques),
  l'âge du moteur relevé dans les deux journaux.
- Après : fusion master en avance rapide, signe à rag3db-73 (rebase de
  commande-en-fond-3) et à rag3db-96 ; e2e_postgres vivante dès que Lucie
  lance le conteneur pgvector (ni moi ni F n'avons docker) — F la joue sur
  ma branche.

## État au 10 octobre au soir

Branche `execution-asynchrone` (worktree `rag3db-async`, target sous
`~/.cache/rag3weaver-build/target-async`), deux commits poussés, étape 3 en
cours :

- **Étape 1 — poussée (`d19eb903d`)** : le trait à deux méthodes
  (`execute` à défaut-erreur « ni execute ni execute_async » ;
  `execute_async` à défaut `block_in_place(execute)`, forme manuelle
  `Pin<Box<dyn Future>>`, AUCUN attribut sur les 78 impl ; `is_async()`) ;
  le runtime du crate (`dataflow/rt.rs` : paresseux, multi-fil, 2 fils) et
  son pont `bloquer` à trois bras — fil ordinaire : `block_on` ; runtime
  multi-fil : `block_in_place` + `block_on` (la réentrance du « graphe dans
  le graphe » de l'ingestion est LÉGALE) ; fil unique : refus nommé, jamais
  la panique. `execute`/`execute_as`/`execute_with_report`/
  `execute_with_checkpoint(_mode)` gardent leurs noms en ponts ; un niveau
  tout-sync garde `run_level` au fil près ; un niveau avec un nœud async
  passe par `run_level_async` (séquentiel, dit et VU par le témoin « niveau
  mixte »). Page 02 §3.2 corrigée aux deux endroits. En ligne à part : la
  fixture de `scripts/test_backend_code.py`, rouge sur master depuis le
  filtre des arêtes devinées (forme `use` + appel nu validée par l'arbre
  principal).
- **Étape 2 — poussée (`da25d8e0f`)** : les trois pièces d'A en forme (b) —
  `RunScope` (`defer`/`vider`, vidé par garde de Drop AVANT le join des
  fils ; témoin `la_portee_du_run_tue_avant_de_joindre`), `ToolInvocation
  { handle "#outil-N", bus, inbox, scope }` par `ToolBox::call_with` (au
  défaut → `call_in`, enveloppes relayées) et services de couche
  `tool_handle`/`agent_bus`/`agent_inbox`/`run_scope` ;
  `NodeContext::tool_handle()`. Le réacteur en tâche (`stop()` = arrêt
  ATTENDU par le pont, Drop-abort en filet) ; `watch_bound(tool, arguments,
  nodes_policy)` pour la session mémoire (`watch` inchangé ; la politique
  reste un argument du montage, jamais un champ de manifeste — convenu avec
  rag3db-96). Page 02 §3.3 corrigée : la boucle d'agent RESTE un fil ce lot
  (48 sites `Agent::new` pour un gain nul ; en tâche au run en fond §4.3),
  les trois pièces survivront au changement de véhicule. Lib complète :
  1250/1250. « Prêt » envoyé à rag3db-73 avec les noms définitifs ; signe
  envoyé à rag3db-96.
- **Étape 3 — codée, compilation en cours** : `postgres_connection.rs` — le
  runtime DÉDIÉ possédé par la connexion (current_thread : il ne vit que
  pendant les appels), constructeurs devenus SYNCHRONES (`new`/`connect` —
  plus aucun contexte tokio exigé des appelants), pont à trois bras local
  (refus nommé en `DbError` sur fil unique), pool inchangé ; les bras
  `CypherValue::Typed` manquants (cypher_to_json, cypher_to_pg_param) +
  `denuder()` aux deux points d'inspection qui perdaient un `Typed` en
  silence (`est_liste_de_lignes`, le `filter_map` des identifiants). Le
  harnais de `tests/e2e_postgres.rs` simplifié : `Contexte` (runtime fuité
  + `rt.enter()`) supprimé, 6 sites de construction directs. Prévenir F
  (rag3db-6f) au push.
- **Étape 4 — codée avec l'étape 3** : `chat.rs` branche
  `.with_events(EventBus::new(64)).with_inbox()` sur son agent — le bus par
  lequel un outil en fond poste sa fin, la boîte qui la ramène au tour
  suivant.

## Mesures : l'AVANT est à REJOUER (banc réparé)

rag3db-96 a réparé `e2e_banc_etage` (rouge : la crate `rag3weaver-ir`,
`41b869ba4`, a déplacé 4 des 63 aiguilles hors de `src/`) et prévenu : mes
chiffres AVANT (banc 444,4/429,7 s ; batterie backend 9,93/10,24 s) ne sont
PAS comparables — le corpus `src/` a changé, 2 des 43 questions changent de
cible. À la fin du lot : rejouer « avant » AU COMMIT D'AVANT mon changement
ET « après » en tête de branche, tous deux sur SON banc réparé (attendre son
signe + le hash de son push master), bancs ×2 en une tenue de mesure
exclusive, batteries ×2 en lourd, même montage.

## Ce qui reste

1. Vert de `--features postgres` (check en cours), lib complète, commits
   étape 3 (postgres_connection.rs + e2e_postgres.rs) et étape 4 (chat.rs),
   push ; prévenir rag3db-6f (F).
2. Batterie complète du crate une fois (e2e_code, e2e_dataflow_observe,
   e2e_checkpoint, suites backend et chat) sous poste lourd.
3. Mesures avant/après sur le banc réparé (voir ci-dessus), au signe de
   rag3db-96.
4. Fusion master en avance rapide ; rapport final.

Coordinations : rag3db-73 a mes noms définitifs (« prêt » envoyé) ; F attend
le push postgres ; rag3db-96 convenu sur watch_bound et me fait signe pour
le banc.
