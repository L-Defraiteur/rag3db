# Les mensonges du moteur, et l'ordre pour les régler

**6 septembre 2026.** Ce document liste **tout** ce qu'on a trouvé de faux ou de
muet, et propose un ordre. La consigne de Lucie tient en une phrase, et c'est
elle qui commande la forme du document :

> dans l'ordre pour régler tous les soucis mentionnés, **pas pour les éviter**.

Donc : rien n'est écarté ici parce que c'est cher. Ce qui est cher est plus bas
dans la liste, pas hors de la liste. Un point qu'on décide de ne pas faire devra
être barré explicitement, avec sa raison — jamais oublié par glissement.

La suite de [`01-les-objectifs-et-leur-ordre.md`](../5-septembre-2026-16h13/01-les-objectifs-et-leur-ordre.md),
qui donnait le cap ; celui-ci donne l'inventaire.

## Ce qui a été réglé aujourd'hui

| | ce que c'était | commit |
|---|---|---|
| `meta.partial` | faux dans le mode **par défaut** : il ne regardait que `Immediate`, alors qu'en `Eventual` relations et agrégats restent en file | `558c06662` |
| `pending_count` | mesuré **avant** la consigne : en `Strict` il annonçait une file que le drain venait de vider | `558c06662` |
| le verdict de `attendre_les_ecritures` | calculé et **jeté** : une attente qui expirait rendait un résultat annoncé complet | `558c06662` |
| `Drop for Catalog` | perdait la file en attente **sans un mot**, après avoir rendu `Ok` | `558c06662` |
| `ready()` | attente **active et infinie** : un ref jamais résolu brûlait un cœur pour toujours au lieu d'échouer | `558c06662` |
| `Consistency` | **aucun appelant en production** — le chemin composable ne traversait aucune de ses trois branches, et `Strict` n'était construit nulle part dans `src/` | `0b611562b` |

Et une correction de prémisse qu'il faut garder en tête pour lire la suite :
**`Catalog::create` n'a aucun appelant hors tests.** Le verbe qu'on croyait
menteur n'est appelé par personne ; les deux `update` et le seul `delete` sont
suivis d'un `drain()` dans la même fonction. Le mensonge d'acquittement est donc
un **défaut d'API**, pas un défaut en exercice — ce qui ne le rend pas moins
réel, mais le place après ce qui ment à quelqu'un aujourd'hui.

## 0. La passe complète, d'abord

Rien ne s'ordonne sur un socle non vérifié. Deux changements sont entrés sans
que les suites e2e aient tourné : **lucivy 4.0.1** (format d'index v4,
dictionnaire partagé) et les deux commits ci-dessus. Les 916 tests de
bibliothèque ne touchent ni la base native ni les modèles.

C'est le préalable, pas une tâche.

## 1. Les comptes cessent de mentir

**Pourquoi en premier :** tout ce qui suit se mesure. Un compteur faux fausse
chaque mesure qu'on prendra ensuite, et la concurrence est précisément un sujet
qui ne se juge que par la mesure.

### 1.a `drain()` rend `failed: 0` en dur

`catalog.rs`, branche de succès : `processed = op_count`, `failed = 0` écrit à la
main. Deux choses fausses dans une seule structure :

- `op_count` est le nombre d'opérations **enfilées**, pas de lignes posées. Un
  nœud qui saute des items — et il y en a un documenté, l'indexation sautée sans
  handle FTS — rend quand même le compte plein.
- `failed` ne peut pas être non nul au succès, parce qu'il n'existe **aucun canal
  d'échec par enregistrement**. `UpdateResult` et `DeleteResult` n'ont pas de
  statut d'échec ; le graphe est tout-ou-rien au niveau `Err`.

Le travail réel est donc de **créer le canal**, pas de changer une constante :
un enregistrement qui n'a pas abouti doit pouvoir le dire avec sa raison. C'est
la même forme que les avertissements de recherche — ce qui n'est pas remontable
n'est pas dit.

### 1.b `ingest_entities` : le court-circuit « inchangé » est indiscernable

Quand rien n'a changé, il rend `FlushResult { processed: record_count, .. }` —
exactement ce que rend une vraie ingestion de la même taille. L'appelant ne peut
pas distinguer « j'ai tout réécrit » de « je n'ai rien eu à faire ». Il faut un
troisième compte, `inchanges`, et non un booléen : sur un lot, les deux cas
coexistent.

C'est aussi ce que la spécification de février appelait la **synchronisation**
— *93 inchangés, 2 modifiés, 5 nouveaux, 3 disparus* — et qu'on n'a jamais su
rendre.

## 2. `temp_uuid` : la dernière valeur plausible et fausse

`EntityRef::temp_uuid()` rend **toujours** une chaîne, au format UUID, qui
n'existe dans aucune base et **ne change pas** après résolution. `uuid()`, lui,
rend honnêtement `Err(Pending)`.

C'est le seul accès du chemin d'écriture qui rend quelque chose de crédible et
de faux, et c'est la pire forme : un appelant qui s'en sert croit tenir une
identité. Deux issues possibles — le nommer pour ce qu'il est (`cle_provisoire`,
et jamais un UUID) ou le faire disparaître au profit de `uuid()`. Petit, isolé,
donc tôt.

## 3. Les quatre disponibilités remplacent `Consistency`

`Consistency` est aujourd'hui **trois valeurs** — `immediate`, `eventual`,
`strict` — là où Lucie a posé un **ensemble** :

> on peut autant à la lecture qu'à l'écriture attendre « data / textsearch /
> sparse / dense ».

Ce n'est pas un niveau mais un ensemble : `data` précède tout, les trois index
sont des frères parallèles qui se commitent séparément. Conséquence de surface :
on n'attend pas « jusqu'au niveau N », on attend **les signaux qu'on nomme**.
Qui ne veut que du plein texte n'attend pas l'embarquement GPU — ce qu'il fait
aujourd'hui.

**Pourquoi maintenant :** depuis `0b611562b` il n'y a plus qu'**un seul endroit
à changer**, `Catalog::appliquer_la_consigne`, et les deux chemins de recherche
en héritent. Avant, il y en avait deux, dont un que personne n'empruntait.

Une nuance mesurée à ne pas perdre : sur le chemin **natif**, l'indexation plein
texte se fait *dans* `InsertRecordNode` et l'index vit avec les données —
`data` et `textsearch` arrivent donc au même instant, par construction. Ils ne
se séparent que sur le chemin lucivy. L'API doit le **dire** plutôt que de le
cacher : « déjà prêt » est une réponse, pas une absence de réponse.

## 4. Le régime « au tick », et le niveau `data` synchrone

C'est l'objectif **1a** dans sa forme entière, et il vient après le vocabulaire
parce qu'il l'emploie.

| régime | ce qu'il fait | pour qui |
|---|---|---|
| **au tick** | chaque écriture passe à sa disponibilité déclarée, arbitrée par ressource | « mon client a acheté un produit » |
| **par lot** | on accumule volontairement, on vide quand on le dit | une ingestion massive |

**`drain` n'est pas le tort et ne se retire pas** — Lucie l'a dit et c'est juste.
Le lot n'est un tort *que lorsqu'il n'a pas été choisi*. `ingest_entities` est
déjà le verbe de lot honnête ; le défaut est que **le verbe par item se comporte
comme le verbe de lot, en silence**.

Et le graphe a déjà l'arête qu'il faut couper :

```
entities → InsertRecordNode ── done → LinkRecordNode ── done → KBGatherNode
                                                                  └→ KBUpdateNode → KBChunkNode → KBEmbedNode
```

Les deux premiers nœuds posent lignes et liens — c'est `data`, et c'est bon
marché. Toute la queue à partir de `KBGather` rend *trouvable* : agrégation,
découpage, embarquement. La distinction n'est pas à inventer, elle est déjà une
frontière de nœuds ; ce qui manque, c'est de pouvoir rendre la main entre les
deux.

Note qui change l'effort : **une entité ordinaire ne produit ni chunks ni
embeddings au drain**. C'est `ingest_entities` qui a le graphe lourd. Seule une
entité qui alimente une KB (`titleFor`) traîne la queue coûteuse. Rendre le cas
« un achat » synchrone est donc presque gratuit.

## 5. `PendingWork` : une file, une barrière

Le couplage faux, celui que l'invariant de Lucie condamne :

> jamais deux ressources qui ne sont pas en lien ne soient bloquées l'une par
> l'autre.

`PendingWork` est **une** structure à cinq vecteurs, partagée par toutes les
entités, vidée par **une** barrière. Une recherche sur l'entité A peut attendre
des écritures en attente sur l'entité B, qui n'a rien à voir. C'est dans le
chemin de lecture, aujourd'hui.

C'est le gros morceau côté Rust, et il dépend du point 4. La difficulté nommée
la veille reste entière : **savoir quelles ressources une écriture touche par
ricochet** — chunks, relations, lignes d'index KB. C'est le même problème que la
granularité des verrous côté C++, et la réponse ne peut pas être différente des
deux côtés.

## 6. Les pièces sans appelant

Une famille, pas quatre points isolés : **une pièce écrite mais jamais appelée
se dégrade sans bruit**. Chacune se règle de la même façon — la brancher, ou la
retirer. Jamais la laisser.

| pièce | état |
|---|---|
| `crate::acces` | le choix de chemin d'un lecteur (direct / démon), écrit et éprouvé le 5 septembre, **aucun appelant de production** |
| `KBSearchNode` | le seul nœud qui appelle `Catalog::search` ; il figure dans `templates/search.mmd` et `templates/search_expansion.mmd`, mais **aucun gabarit d'outil** (`templates/tools/`) ne l'emploie — il n'est référencé que par un test de parsing |
| `fusion.rs` | `pub mod fusion;` dans `lib.rs`, **aucun usage interne** |
| `title_boost` / `content_boost` | acceptés à la configuration, copiés dans `KBMetadata`, **jamais relus**. `config.rs` le dit déjà en toutes lettres |

Le cas des poids de champ est le plus instructif : leur remplacement n'est pas
un correctif mais une **topologie** — une branche BM25 par champ, pesée à la
fusion — parce que lucivy n'a aucune pondération par champ. C'est du travail
conçu, pas à concevoir.

## 7. Les silences restants

- **La troncature d'embedding.** La taille de chunk n'est pas dérivée de la
  limite du modèle : ce qui dépasse est coupé, sans un mot. C'est une idée de
  février jamais tenue, et c'est exactement la famille qu'on traque.
- **Le handle FTS manquant.** Sans handle ouvert, `InsertRecordNode` et
  `KBUpdateNode` sautent l'indexation **en silence**, et la recherche rend zéro
  sans que rien ne le signale. Il y a un filet — un commentaire qui dit
  d'appeler `open_fts_handles_for` depuis *chaque* point d'entrée — et un
  commentaire n'est pas un contrat. C'est au type de le tenir.

## 8. La dette de structure

- **`Catalog::search` devient un gabarit.** 409 lignes dans un `catalog.rs` de
  6 373, et le chemin composable existe en parallèle sans être emprunté. La
  dette a un prix **mesuré** : cette semaine, chaque correction de recherche a
  dû être portée aux deux chemins, et l'une l'a été à moitié. Aujourd'hui encore,
  `appliquer_la_consigne` a dû être extraite pour éviter la cinquième.
- **Le miroir Rust de `search_base`.** Le graphe est écrit deux fois, en Rust et
  en Mermaid, et un test compare les deux. Il a coûté sept assertions
  aujourd'hui — ce qui est le signe qu'il fait son travail, et aussi qu'il
  coûte. `GraphTool::new_avec_herites` a été ajouté pour que la voie Rust puisse
  dire ce que la voie Mermaid dit ; c'est un colmatage, pas la sortie.

## Ce que cet ordre suppose, et qui reste ouvert

- Les points **1 à 5** sont du Rust, ici, sans dépendance au cœur C++. Ils
  peuvent avancer pendant que la relecture du MVCC de Vela se fait ailleurs
  (objectif **1b**, terrain disjoint).
- Les points **4 et 5** changent la **surface** de l'API d'écriture. C'est un
  vrai changement, pas un ajout — et c'est le bon moment, tant que `create` n'a
  aucun appelant de production.
- Le point **6** peut se faire à n'importe quel moment ; il est placé après
  parce qu'il ne débloque rien. Mais il ne doit pas glisser indéfiniment : c'est
  la définition même de la dégradation sans bruit.
