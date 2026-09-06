# Rapport de session, et les chantiers qui suivent

**6 septembre 2026, 11 h 43.** Vingt-six commits depuis `f2c424892`. La session
a commencé sur une montée de version et a fini sur la moitié écriture des
quatre disponibilités.

## Les documents du même genre, avant celui-ci

À lire dans cet ordre si on remonte le fil :

| | quoi |
|---|---|
| [`6-septembre-2026-01h42/01`](../6-septembre-2026-01h42/01-les-mensonges-et-l-ordre-pour-les-regler.md) | l'inventaire des mensonges et l'ordre pour les régler — **le document de référence** de cette série |
| [`6-septembre-2026-01h42/02`](../6-septembre-2026-01h42/02-au-reveil.md) | la note de réveil : ce que la nuit a réglé, ce qui attend une décision |
| [`5-septembre-2026-16h13/01`](../5-septembre-2026-16h13/01-les-objectifs-et-leur-ordre.md) | les objectifs et leur ordre : la vraie concurrence, les quatre disponibilités, les deux régimes |
| [`5-septembre-2026-16h13/02`](../5-septembre-2026-16h13/02-rapport-de-session-et-ou-regarder.md) | rapport du 3-5 septembre, et le tableau des choses affirmées puis corrigées |
| [`3-septembre-2026-17h50/01`](../3-septembre-2026-17h50/01-rapport-de-session.md) | la session d'avant |
| [`30-aout-2026-07h00/01`](../30-aout-2026-07h00/01-rapport-de-session-et-objectifs.md) | rapport + objectifs, fin août |
| [`25-aout-2026-18h58/08`](../25-aout-2026-18h58/08-objectifs-immediats-et-long-terme.md) | les objectifs immédiats et long terme, version d'août |
| [`vision_roadmap_09_2026/00`](../vision_roadmap_09_2026/00-index.md) | la série vision, relue et datée le 5 septembre |

## Ce que la session a fait

### 1. lucivy 4.0.1 et le dictionnaire partagé

`lucivy-core`, `ld-lucivy` et `sparse-vector` de 3.0.8 à 4.0.1. **Aucune rupture
d'API** — mais le gain n'arrivait pas tout seul : `build_schema_config` posait
`sfx_version: 3` *explicitement*, avec un test qui l'épinglait. Passé à 4, en
gardant le principe : c'est écrit, pas subi. ~20 % de disque et de RAM en moins.

Deux choses inscrites dans le code parce qu'elles se paient plus tard : la 4.0
ouvre les index 3.0.x mais son premier commit les convertit **pour de bon** ; et
un index à dictionnaire lit ses `dict-*` entiers à l'ouverture, donc
`BlobBacked { lazy: true }` rend moins qu'avant.

### 2. Onze silences réparés dans le chemin d'écriture

Le plus grave n'était pas sur la liste : **`flush_insertions` — le chemin par
défaut de la lecture — n'indexait rien en plein texte.** C'était le quatrième
point d'entrée d'ingestion, le seul à ne pas appeler `open_fts_handles_for`, et
son registre « minimal » ne portait pas `fts_handles`. Les entités qui y
passaient étaient consommées (`mem::take`) et jamais indexées. Le commentaire
qui annonçait ce défaut était en place depuis des semaines, à l'endroit exact où
il s'est produit.

Il a survécu parce qu'**aucun test n'empruntait la combinaison du produit** :
`create()` puis une recherche en `Eventual`. Tous les e2e passent par
`ingest_entities` et `Consistency::Immediate`.

Les dix autres, par famille :

- **le contrat de lecture** — `meta.partial` mentait dans le mode par défaut,
  `pending_count` était mesuré avant la consigne, et le verdict de
  `attendre_les_ecritures` était calculé puis jeté ;
- **les comptes** — `FlushResult` gagne `unchanged` (le compte existait et
  n'allait qu'à un `eprintln!`) et `warnings` (les nœuds parlaient déjà dans le
  vide) ;
- **cinq erreurs avalées**, trouvées par balayage systématique, dont **deux qui
  rendaient de la donnée périmée plutôt que rien** — pire, puisque rien ne
  signale qu'il faut douter ;
- **quatre cadrans débranchés** (`title_boost`, `content_boost`, `special_ops`,
  `boost`) qui se signalent au montage ;
- **`Drop`** perdait la file sans un mot ; **`ready()`** était une attente active
  infinie.

### 3. `Consistency` avait zéro appelant en production

Elle vivait dans le corps de `Catalog::search`, que le chemin composable
n'emprunte pas : l'outil `search` des agents ne traversait **aucune** de ses
trois branches, et `Consistency::Strict` n'était construit nulle part dans
`src/`. La marque d'eau d'ingestion, bâtie et éprouvée la veille sur deux
processus réels, n'avait donc aucun appelant.

Réparé par un **unique écrivain**, `Catalog::appliquer_la_consigne`, appelé par
les deux chemins.

### 4. Le schéma v3 et le marqueur sparse

Un seul `_embed_hash` répondait pour deux signaux. Sur le chemin dual,
l'écriture dense posait le marqueur juste après l'écriture sparse : **un vecteur
sparse perdu restait annoncé écrit**, et le court-circuit de l'inchangé sautait
ces chunks pour toujours.

`_sparse_hash` sépare les deux. Quatre endroits suivaient le marqueur et chacun
était faux à sa façon — l'écriture, le filtre de réembarquement,
`embed_check_hashes` (qui filtrait sur `_embed_hash IS NOT NULL`, cachant
exactement le cas qui compte), et l'**undo**, qui n'en remettait qu'un.

Au passage, le marqueur sparse était posé **avant** le vecteur : un crash ou un
handle absent laissait un trou permanent et silencieux. `embed_get_offset`
existait déjà et ne pose rien ; on lit, on écrit, **puis** on marque.

### 5. Les quatre disponibilités, et la coupe

`Disponibilites` est un **ensemble** — `data` précède tout, les trois index sont
des frères parallèles. `Consistency` survit comme raccourci nommé mais ne porte
plus d'état : il le **traduit**. `SearchOptions::ce_qui_doit_etre_pret` est
l'unique arbitre, et l'attente des autres processus est devenue une question
séparée, ce qu'elle a toujours été.

Puis la coupe : `drain_jusqu_a` s'arrête après que les chunks sont posés **et
indexés en plein texte** quand ni `dense` ni `sparse` n'est demandé. Les nœuds
d'embarquement sont des feuilles du graphe — sauf un, sur le chemin de
réécriture, dont le flush FTS attend la fin ; celui-là prend son déclencheur en
amont.

**La dette n'est pas gardée en mémoire.** Elle est dans la base — chunks au
marqueur vide — donc elle survit à un processus qui meurt.
`embarquer_le_retard` la retrouve par une requête bornée à 512 par table.

Et `ingest_entities_jusqu_a` fait la même chose pour le verbe de lot, en
**disant sa portée** : `FlushResult.rendu_pret`. `ingest_entities` sans suffixe
ne bouge pas — il reste complet, et un test l'épingle.

## Ce que la session a corrigé sur elle-même

Quatre fois, et c'est gardé avec le mécanisme, pas seulement la conclusion :

| ce que j'ai cru | ce qui était vrai |
|---|---|
| l'OOM de la passe venait du régime `plein` | il vient du cgroup à 16 Go du script — la passe est morte aussi en `confort` |
| mon test de non-régression attendait « partiel » | la fiche `Product` n'a aucune KB : rien ne reste en file |
| deux suites e2e étaient mortes | elles sont derrière `openai-llm`, exclusion délibérée et documentée |
| Nagle était gratuit | il exige un fil de fond, et ce fil rencontre le verrou du catalogue |

Et une de plus, dans le code : **j'ai écrit mon propre silence.** La première
version du rattrapage avalait l'erreur de sa requête, qui demandait `_kb_name`
sur une table qui ne l'a pas. Le rattrapage reprenait zéro chunk sans un mot,
pendant que l'avertissement d'à côté en annonçait deux.

## Les chantiers, dans l'ordre

### A. Le tick — et ce qui n'est pas mécanique

L'algorithme est celui de Nagle : **embarquer dès que la carte est libre,
accumuler pendant qu'elle travaille**. Il s'auto-règle sur le débit sans qu'on
ait à classer l'écrivain — celui qui ingère un fichier par minute trouve la
carte libre et part tout de suite ; celui qui pousse en continu la trouve
occupée, et ses lots grossissent tout seuls.

**Trois décisions, pas un branchement :**

1. **Nagle a besoin de concurrence pour exister.** Sans fil de fond, rien n'est
   jamais en vol, la carte est toujours « libre », et l'algorithme dégénère en
   « une passe par écriture » — exactement ce qu'on ne veut pas pour l'ingérant
   rapide.
2. **Le verrou.** `Catalog` vit derrière un `Mutex`. Un fil qui embarque en le
   tenant bloquerait toutes les recherches pendant une passe GPU — l'invariant
   de Lucie violé de la façon la plus flagrante. Il faut que la passe sorte le
   travail, **relâche**, embarque, puis reprenne pour écrire.
3. **Deux processus rattrapent les mêmes chunks.** Les deux voient le marqueur
   vide, les deux calculent, le dernier écrit. Du GPU jeté, pas de corruption.
   Il faut une réclamation avec péremption — la forme exacte de la marque d'eau
   d'ingestion.

**Disponible tout de suite, sans aucune des trois** : appeler
`embarquer_le_retard` à la fin de chaque verbe d'écriture. On est déjà dans le
verrou, la dette est fraîche. Ça donne le quasi-synchrone qui convient à
l'ingérant lent, ça ne chaperonne pas encore le rapide, et ça ne ferme aucune
porte.

**Ordre proposé** : la version opportuniste maintenant ; le fil de fond **après**
la relecture du MVCC de Vela, parce que les points 2 et 3 sont le même problème
que la concurrence sur rag3db et que les résoudre deux fois serait perdu.

### B. Resserrer les deux approximations restantes

La table en tête de [`src/disponibilite.rs`](../../src/disponibilite.rs) les
nomme :

- **`Donnee`** pose les entités mais laisse relations et agrégats.
  `flush_insertions` doit apprendre à poser les relations.
- **`Sparse` et `Dense` partent ensemble.** Le schéma v3 sait les distinguer par
  leurs marqueurs ; ce sont les nœuds d'embarquement qui ne savent pas encore
  n'en calculer qu'un.

### C. Ce qui attend toujours une décision

Repris de [`6-septembre-2026-01h42/02`](../6-septembre-2026-01h42/02-au-reveil.md),
allégé de ce qui est fait :

| | pourquoi c'est une décision |
|---|---|
| **`PendingWork` par ressource** | le couplage faux : une recherche sur A attend les écritures sur B. `Eventual` flushe **toute** la file, donc seul `AUCUNE` respecte l'invariant aujourd'hui |
| **`FlushResult.failed`** | vaut toujours `0` en dur au succès : il n'existe aucun canal d'échec **par enregistrement**. Le créer change une structure publique |
| **`fusion.rs`** | aucun appelant, supplanté par `search::fuse_by_strategy`. Supprimer, ou assumer comme surface publique |
| **`crate::acces`**, **`KBSearchNode`** | toujours sans appelant de production ; les brancher est un choix de produit |
| **les registres de services** | le noyau identique est extrait ; ce qui diverge reste chez chacun — `ingest_entities` n'enregistre ni `kb_metadata` ni `event_bus`, et personne ne sait si c'est voulu |
| **le nom `immediate`** sur la fiche d'outil | il dit *quand* et pas *quoi*. « Cherche dans ce qui est déjà là » se dirait mieux ; toucher à la liste close est un choix de surface |

### D. Les chantiers de fond, inchangés

1. **La vraie concurrence sur rag3db** — relire le MVCC de Vela (préalable),
   verrous inter-processus, arbitrage à la ressource. Voir
   [`5-septembre-2026-16h13/01`](../5-septembre-2026-16h13/01-les-objectifs-et-leur-ordre.md) §1.
2. **La lecture des documents** — pdf, docx, pptx, html, csv.
3. **`Catalog::search` devient un gabarit** — 409 lignes, et chaque correction
   de recherche doit être portée aux deux chemins.
4. **Le graphe de normalisation des tableurs.**
5. **Avaler une base étrangère.**
