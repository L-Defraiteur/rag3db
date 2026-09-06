# Bilan de la passe « fondations d'abord »

**6 septembre 2026, après-midi et soirée.** Quinze commits depuis la
réconciliation ([`04`](04-reconciliation-fondations-d-abord.md)), quarante-deux
depuis `f2c424892` ce matin. Le contrat de cette passe était celui de Lucie :

> pré-chantier important nécessaire avant autre — à faire en prio, pas à dire
> « on met un truc pour dire on fera plus tard, pas grave ». Même des trucs
> super techniques genre écrivain en parallèle, on fait toute fondation en
> premier, peu importe même si monument.

Ce document dit ce qui est fait, ce qui a été trouvé en le faisant, et ce qui
reste — avec, pour chaque reste, la raison **vraie** et non « plus tard ».

## En trois lignes

Tout ce que le doc de réconciliation ordonnait est fait, sauf deux choses qui
attendent une décision de surface et une qui attend le cœur C++ : le
monolithe `Catalog::search` a un successeur (`Catalog::rechercher`) mais n'est
pas retiré ; les agrégats de base de connaissances ne sont pas encore une
dette en base ; et la vraie concurrence sur rag3db a maintenant sa relecture,
qui dit qu'elle n'est pas une greffe. **949 tests de bibliothèque, toutes les
suites e2e jouées vertes, suite par suite.**

## Ce qui est fait, par fondation

### A — le contrat d'écriture ne ment plus

| | commit |
|---|---|
| A1 `SparseSearchNode` explique son silence, par la méta | `864779778` |
| A2 `Donnee` exact : les relations dont les deux bouts sont posés partent avec la donnée | `864779778` |
| A3 `create`/`update`/`delete`/`link` **posent avant de rendre** ; `RegimeEcriture::{AuTick, ParLot}` ; variantes `_jusqu_a(…, exige)` qui rendent un `FlushResult` | `7ef572261` |
| A4 le rattrapage opportuniste sur le drain complet | `7ef572261` |
| A5 `FlushResult.failed` cesse d'être `0` en dur : le canal d'échecs par groupe, `UpdateStatus::Failed`, `DeleteResult.echec`, les refs d'un groupe raté résolus en échec | `392c196ff` |
| A6 la clé de corrélation **unique entre processus** (elle repartait de zéro à chaque run) ; `create` dérive l'identité comme `ingest_entities` | `864779778` |
| A7 un seul registre de services d'ingestion — `event_bus` manquait à deux des quatre, et une trace voyait la moitié des ingestions | `864779778` |

### B — un seul chemin de recherche

La cartographie a renversé la question : `Catalog::search` **n'était plus sur
le chemin du produit**. Les agents passaient déjà par le graphe — avec douze
écarts. Tous sont fermés :

| | commit |
|---|---|
| B1 `target.parent_table` (une KB était un `MATCH` sur une table absente) ; l'enrichissement par le backend | `cda21d1a3` |
| B2 le dialecte du service, plus rag3db en dur (faux sur PostgreSQL) | `cda21d1a3` |
| B3 la page voyage avec la requête : sur-fetch, `PaginateNode`, `offset` n'est plus inerte | `e842c4a3e` |
| B4 les poids de fusion : appelant > base de connaissances > gabarit > défaut | `e842c4a3e` |
| B5 le filtre hérité (`filters`) descend | `e842c4a3e` |
| B6 la requête s'embarque **une fois**, par la source, dual si besoin, avec le cache | `965988259` |
| B7 le rerank : pool de la requête, plancher, enrichissement du pool, **méta** | `e842c4a3e` |
| B8 les diagnostics, depuis les durées de nœuds du runtime | `c71aee224` |
| B9 l'index plein texte s'ouvre à la source, paresseusement | `965988259` |
| B10 BM25 suit les défauts de la requête (`Auto`, distance 1) et vérifie sa cible | `965988259` |
| B11 `SourceResolved` après la page, avec la déduplication du monolithe | `965988259` |
| B12 fan-out de cellules autour du graphe | `c71aee224` |
| B13 **`Catalog::rechercher`**, le lanceur ; `KBSearchNode` passe par lui ; la branche sparse dans le gabarit | `c71aee224` |

### C — la ressource est l'unité

| | commit |
|---|---|
| C1 la **fermeture** : un drain n'emporte que ce qui est en lien ; une recherche sur A ne paie pas B ; la marque ne s'efface que quand plus rien n'attend | `9fae08275` |
| C2 la marque publie **par niveau et par table** ; un lecteur n'attend que les écrivains qui le concernent | `80697d3c4` |
| C3 un catalogue **ouvert en lecture**, qui ne pose rien et refuse d'écrire en le nommant — l'appelant que `crate::acces` n'avait pas | `ccc1a7064` |
| C4 la **réclamation** sur la dette d'embarquement (schéma v4, `_embed_claim`) : deux rattrapages ne calculent pas deux fois, prouvé sur PostgreSQL à deux catalogues | `ef220cafb` |
| C5 la **dette de découpage** en base (schéma v5, `_chunked_hash`) : une mise à jour se pose au niveau donnée, exactement | `a6e71bafa` |

### D — la concurrence, relue

`docs/6-septembre-2026-13h08/01-relecture-du-mvcc-de-vela.md`, à la racine
(`97f474667`). Deux choses qu'on ne savait pas : **le port de Vela n'est pas
dans l'arbre** (trois lignes reprises), et **un arbitre inter-processus
par-dessus le MVCC existant n'est pas une greffe** — l'horloge de transaction,
l'allocation d'offsets et le cache de pages sont enfermés dans le processus
par construction.

### E — `fusion.rs` retiré (`7c3df252f`).

## Ce qui a été trouvé en le faisant

Trois défauts qui n'étaient sur aucune liste, tous en exercice :

1. **La clé de corrélation repartait de zéro à chaque processus** (blake3 d'un
   compteur). Pour `create` sans clé déclarée, qui la prenait comme `_uuid`,
   deux runs se donnaient les mêmes identifiants dans le même ordre, et le
   second écrasait le premier. Réparé par un sel de processus, et par
   l'identité dérivée de l'entité (A6).
2. **Une KB sur le chemin des agents faisait un `MATCH` sur une table qui
   n'existe pas**, et PostgreSQL recevait du Cypher (B1, B2). Aucun test
   n'empruntait une KB par ce chemin.
3. **`FilterValue` relisait une liste d'opérateurs comme une valeur directe**
   (`untagged`, `Direct` essayé en premier) : un filtre qui traversait du
   JSON ne restreignait plus rien, au journal seulement. Trouvé par le domaine
   de travail qui ne rétrécissait plus dès que la requête passait par le
   lanceur (B13).

Et une chose que la relecture a corrigée dans ma propre note du matin : le
fil de fond du tick n'avait pas à attendre Vela — il n'a **pas d'hôte**,
aucun processus de production ne garde un catalogue en vie. C'est une autre
raison, et elle mène ailleurs (voir ci-dessous).

## Ce qui reste, et pourquoi

| reste | la raison vraie |
|---|---|
| **`Catalog::search` n'est pas retiré.** Ses appelants sont des tests (`e2e_result_mode`, `e2e_rerank`, `e2e_scope`, diagnostics). Le lanceur `Catalog::rechercher` est prouvé équivalent sur BM25, vecteur et hybride | **décision de surface** : un pont qui déplace le catalogue dans un `Arc` le temps du graphe (`Arc::try_unwrap`, erreur nommée si un nœud le retient), ou la migration des tests sur le lanceur. Les deux se font en une heure ; c'est le choix qui est à Lucie |
| **C5 bis — les agrégats de base de connaissances comme dette en base.** Aujourd'hui une mise à jour d'une entité qui alimente une KB emmène le graphe sans GPU (plus que demandé) | il faut une forme en base : `_content_hash = ''` sur les lignes d'index dont une source a changé, posé au niveau donnée, et un rattrapage qui relance `KBGather` dessus. Une demi-journée, sur le même patron que C4 et C5 |
| **Des cellules par requête**, sans bascule d'état global. Le fan-out et `options.scope` changent la cellule du catalogue le temps de l'appel — comme le monolithe, pas plus sûr face à une recherche concurrente | les handles lucivy sont ceux de la cellule courante ; il faut que le catalogue expose ses handles **par cellule** et que la requête porte la sienne jusqu'aux nœuds. C'est de la concurrence intra-processus, à faire avec la fusion de Vela |
| **Le battement de cœur de la marque** (C2 bis). Une marque figée plus de 60 s passe pour celle d'un mort ; une ingestion de 32 minutes sans appel intermédiaire l'est donc à tort | un fil qui rafraîchit l'horodatage n'a besoin que de la connexion, pas du catalogue — mais deux fils sur une même connexion rag3db, c'est la question de D |
| **Le tick de fond** (Nagle) | n'a pas d'hôte. Le jour où un processus garde un catalogue — le démon, quand il tiendra un catalogue et pas seulement une connexion — il s'écrit en vingt lignes sur `embarquer_le_retard` et `rattraper_le_decoupage` |
| **D — le monument** | dans l'ordre que la relecture donne : fusion de Vela (une demi-journée, intra-processus propre), arbitre externe qui met les écrivains en file (confort), puis horloge partagée, allocateur d'offsets, cohérence de cache. Cœur C++, session `rag3db-57` |
| `debug_enable_multi_writes` | promet plus qu'il ne fait : un `throw` sauté, une détection de conflit partielle. À renommer ou à faire avertir avant tout chantier de concurrence |

## Les questions à Lucie, avec ma réponse faute d'autre

1. `Catalog::search` : pont par `Arc`, ou migration des tests ? *Je prendrais la
   migration — le pont est un tour de passe-passe, et les tests qui appellent
   `search` sur un catalogue possédé sont exactement ceux qui gagneraient à
   éprouver le lanceur.*
2. Le battement de cœur de la marque : un fil sur la connexion, ou rien tant
   que D n'a pas dit si deux fils sur une connexion rag3db sont sûrs ? *Rien
   tant que D n'a pas dit ; la relecture de Vela penche pour « non sûr » sans
   leur travail.*
3. C5 bis : maintenant, ou après la fusion de Vela ? *Maintenant : c'est du
   Rust pur, sur un patron qui a marché deux fois aujourd'hui.*

## Comment vérifier

```sh
cargo test --lib                       # 949, ~0,5 s
./run_e2e.sh --test e2e_generic_search # le lanceur, le sparse muet, le filtre hérité
./run_e2e.sh --test e2e_prise_atomique # deux processus : le catalogue en lecture
./run_e2e.sh --test e2e_postgres       # la réclamation à deux catalogues, le chemin composable sur SQL
./run_e2e.sh --test e2e_simple_entity  # la dette de découpage, la coupe, le lot
```

Suite par suite, toujours : la passe entière dépasse le cgroup de 16 Go du
script (knowledge dump §1).
