# Chaque requête préparée reste en mémoire pour la vie de sa connexion

- **État** : corrigé par le commit qui déplace ce ticket (25 octobre 2026, seconde session cœur
  C++ ; ouvert le même jour par G, mesuré à luciepc ; forme retenue par l'orchestration). Voir « Ce
  qui a été fait », à la fin.
- **Gravité** : la mémoire croît sans borne. Un hôte que des agents lisent sans arrêt grossit d'environ
  1,7 Go par minute avec quatre lecteurs, jusqu'à ce que le noyau le tue (40 Go, le plafond de `poste`).
- **Atteignable en service** : oui. C'est le régime ordinaire d'un hôte de code : des connexions qui
  vivent longtemps et préparent chacune des milliers de requêtes.
- **Touche rag3weaver** : oui (chaque appel passe par `Rag3dbConnection::query_with_params_sync`, qui
  prépare puis exécute). Le correctif vit dans le moteur (`src/main`).

## Ce que c'est

`ClientContext::prepareWithParams` range chaque requête préparée dans
`CachedPreparedStatementManager::statementMap` (`src/main/prepared_statement_manager.cpp`), sous un
index qui ne fait que croître. Rien ne l'en retire : ni la destruction du `PreparedStatement`, ni la
fin de l'exécution, ni un plafond. Chaque `prepare` laisse donc vivants, pour toute la vie du
`ClientContext`, son `parsedStatement`, son `logicalPlan` et ses colonnes liées (les nœuds que crée
`Binder::createQueryNode`).

Le comportement vient de l'amont : Kuzu #5846, « Cache prepared statement in client context »
(0cfe4d549 ici). `executeWithParams` ne réutilise même pas l'entrée cachée : il relie une copie
neuve à partir de `cachedStatement->parsedStatement` à chaque exécution.

## La mesure

Le témoin réel `e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot` : 221 fichiers, quatre
lecteurs en boucle, `RAG3WEAVER_READ_POOL=0`, lucivy patché (branche `lucivy-patch-1`), lib
8b98f19f7 puis 58b46a9bf.

- **smaps** : toute la croissance est dans les arènes de malloc, de 4,8 à 26,8 Go (de 90 à 427
  segments de 64 Mio). Les régions du tampon du moteur et le tas principal ne bougent pas.
- **malloc_info** : 23,7 Go en usage sur 25,5 Go réservés. C'est de la mémoire vivante, pas de la
  mémoire libérée gardée par malloc.
- `MALLOC_ARENA_MAX=2` : 29,1 Go, la même chose. Ce ne sont pas les arènes.
- **Sans lecteurs** : 7,2 Go en fin de passe, lucivy publié ou patché.
- **Qui la tient** : `gros-mallocs3.so`, en LD_PRELOAD, échantillonne une allocation par Mio alloué
  et par fil, avec sa pile, et suit sa libération. À 14 h 51, 6,7 Gio d'échantillons vivants pour
  7,3 Gio en usage selon malloc_info, et entre 14 h 47 et 14 h 51 :
  - les piles qui passent par `ClientContext::prepareWithParams` : de 1,1 à 2,2 Gio (le double) ;
  - « moteur, ailleurs » : de 1,3 à 2,6 Gio. Ce sont surtout les mêmes allocations de
    `Binder::createQueryNode`, dont la pile est trop profonde pour les 48 trames de la sonde ;
  - lucivy : de 116 à 144 Mio ; rag3weaver hors moteur : de 270 à 518 Mio.

  Les trames rag3weaver de ces requêtes sont les lectures des lecteurs : `links_between`,
  `degrees`, `usages_of_in`, par `ReadView::run_read_plan` et `execute_read_with_params`.

## La recette

Pas encore de recette réduite. Celle qu'il faudrait : une connexion qui prépare et exécute N
fois la même requête (`MATCH (n) RETURN count(n)`), le `PreparedStatement` détruit à chaque fois.
La taille de `statementMap`, ou la mémoire vivante du processus, croît comme N.

## Pour le fermer

Retirer l'entrée du cache quand le `PreparedStatement` qui la nomme est détruit. Ou ne pas
cacher une requête qu'on n'exécutera qu'une fois. Ou borner le cache. Lire d'abord ce que l'amont
a fait après #5846 (le lire, ne pas le copier). Le témoin réel doit ensuite rester sous 10 Go avec
quatre lecteurs.

## Ce qui a été fait

**Le magasin n'était pas un cache.** `statementMap` n'était pas indexé par texte de requête, mais
par un compteur. Chaque `PreparedStatement` en gardait le nom, et `executeWithParams` y lisait
`parsedStatement` pour relier à neuf. Une entrée servait donc exactement autant que vivait son
`PreparedStatement`. Un cache borné (LRU) aurait cassé des préparations encore tenues, ou n'aurait
rien borné. Le magasin portait aussi deux autres défauts :
- `addStatement` prenait le verrou, mais `containsStatement` et `getCachedStatement` lisaient la
  table sans lui ;
- les noms étaient des compteurs par connexion. Une préparation exécutée sur une autre connexion
  que la sienne y trouvait l'entrée d'une autre requête, et l'exécutait sans erreur.

**La forme, à la PostgreSQL.** Une préparation vit jusqu'à sa libération, ici la destruction de
l'objet. `PreparedStatement` porte sa `CachedPreparedStatement` (un `shared_ptr` privé), et
`CachedPreparedStatementManager` disparaît. Un `PreparedStatement` qui survit à sa connexion, ce
que permet le type Rust, ne pointe plus vers elle. `test_runner` lit le plan par un accesseur
inline. L'ABI de `PreparedStatement` change : la bascule de lib rebâtit les hôtes et les liaisons.

**Les témoins** sont dans `test/api/prepared_statement_lifetime_test.cpp`, à luciepc, sur master
6fe9ffe9b puis avec le correctif :

| témoin | master | correctif |
|---|---|---|
| 3 000 préparations distinctes détruites | +27,0 Mo vivants (9 Ko chacune) | ±240 octets |
| la même requête préparée 3 000 fois | +17,9 Mo (6 Ko chacune) | ±500 octets |
| une préparation exécutée sur une autre connexion | exécute la requête n° 0 de l'autre | exécute la sienne |
| des préparations qui survivent à leur connexion, puis à leur base | vert (garde) | vert |

Le correctif est vert deux fois en Release, et une fois sous ASan, sans puis avec la détection des
fuites : aucune erreur ASan ni LSan.

**Le témoin réel** (`e2e_lectures::mesure_des_instantanes_sur_un_vrai_depot`, quatre lecteurs,
`RAG3WEAVER_READ_POOL=0`, recette de G) a été joué sur master 527a0356c, qui porte le patch lucivy
(`nm` : `finish_commit` vaut 1). La lib bâtie à part a été prouvée par `/proc/<pid>/maps`, et le
maximum de RssAnon + VmSwap relevé toutes les 5 s :
- sans le correctif : **30,0 Go**, tours de 3,3 / 388,8 / 233,2 s, vert en 700,8 s ;
- avec : **7,1 Go**, tours de 2,7 / 253,2 / 340,4 s, vert en 619,1 s. C'est le plancher sans
  lecteurs que mesurait G (7,2 à 7,3 Go).
