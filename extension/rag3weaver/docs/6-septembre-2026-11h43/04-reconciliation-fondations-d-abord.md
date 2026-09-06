# Réconciliation : ce qui reste à faire, fondations d'abord

**6 septembre 2026, après-midi.** Relecture des trois documents du matin, des
deux de la nuit, des objectifs du 5, et des 3 400 lignes de diff depuis
`f2c424892`. Ce document **remplace** l'ordre des chantiers donné dans
[`01-rapport-de-session-et-chantiers.md`](01-rapport-de-session-et-chantiers.md)
§« Les chantiers » ; il ne remplace rien d'autre.

Lucie, en le demandant :

> pas de décision naïve surtout ; faut que cette passe on revoie pour aller en
> ce sens : pré-chantier important nécessaire avant autre — à faire en prio,
> pas à dire « on met un truc pour dire on fera plus tard, pas grave ». Même
> des trucs super techniques genre écrivain en parallèle, on fait toute
> fondation en premier, peu importe même si monument.

## Le mécanisme du glissement, pour ne pas le refaire

Le document de la nuit avait été écrit pour empêcher exactement ça — *« ce qui
est cher est plus bas dans la liste, pas hors de la liste »*. C'est **dans ce
document même** que les reports ont été consignés, sous la forme « à décider ».
Un report habillé en décision ne se voit pas comme un report.

Trois formes, toutes présentes dans les docs du matin :

| la forme | l'exemple |
|---|---|
| **la décision déjà prise, redemandée** | « le niveau `data` devient synchrone par défaut » — tranché le 5, revenu en « change ce que `create` promet, à toi » le 6, **absent** du doc de 11h43 |
| **le travail présenté comme une décision** | `PendingWork` par ressource : la décision est l'invariant, déjà énoncé ; ce qui manque est du travail |
| **le rendez-vous manqué** | « `Catalog::search` en gabarit la prochaine fois qu'on rouvre la recherche » — rouverte deux fois dans la session, et le demi-portage annoncé s'est reproduit |

**La règle de cette passe.** Rien n'est reporté par « à décider ». Un item ne
descend dans la liste que s'il **dépend d'une chose qui n'existe pas encore**,
et cette chose est elle-même dans la liste, au-dessus. Les vraies questions
pour Lucie sont posées en tant que questions, avec la réponse que je prendrais
faute d'autre, et le travail avance sur cette réponse.

## 1. Le tableau de réconciliation

Tout ce qui a été listé depuis le 5, avec son **état vrai** et non celui que
les docs annonçaient. Le rang renvoie à la section 2.

| item | d'où | état vrai le 6 à midi | ce qui manque | rang |
|---|---|---|---|---|
| l'acquittement de `create`/`update`/`delete`/`link` | 5 sept, obj. **1a**, premier de l'ordre | **inchangé** : ils mettent en file et rendent `Ok` ; `Drop` le dit maintenant, c'est tout | le niveau `data` synchrone par défaut, le lot déclaré | **A3** |
| `Consistency` sans appelant | nuit | réparé : `appliquer_la_consigne`, unique écrivain | — | fait |
| les quatre disponibilités | 5 sept | faites côté lecture (`exige`) et côté verbe de lot (`rendu_pret`) | les verbes unitaires ne les parlent pas ; la marque d'eau ne publie pas par niveau | **A3**, **C2** |
| la coupe et la dette en base | 6 sept | faite, testée, **un demi-portage** : `SparseSearchNode` n'explique pas son silence, `Catalog::search` si | quatre lignes | **A1** |
| le tick | 6 sept | la dette et le rattrapage existent ; **aucun déclencheur** | l'appel opportuniste là où on paie déjà le GPU ; le fil de fond n'a **pas d'hôte** (le démon tient une connexion, pas un catalogue) | **A4**, **F** |
| `Donnee` laisse les relations | table d'approximation | vrai | `LinkRecordNode` dans le graphe de `flush_insertions` | **A2** |
| `flush_insertions` pose **tout** | trouvé ce matin | une recherche sur A pose les entités de B — le couplage faux, sur le chemin par défaut | poser la **fermeture** de la cible, pas la file entière | **C1** — fait l'après-midi |
| une mise à jour ne se pose pas seule | trouvé en faisant C1 | ses conséquences sur les chunks ne sont ni en base ni en file : perdues si le graphe s'arrête à la donnée | la dette de découpage en base | **C5** |
| `PendingWork` file unique, `drain` barrière | 5 sept, invariant | vrai ; seul `AUCUNE` respecte l'invariant | partitionner par fermeture de ressource, drain compris | **C1** |
| `FlushResult.failed = 0` en dur | nuit, §1.a | vrai, **et lu** : `ingest_code` l'additionne, `code_tools` l'affiche | un échec par groupe d'enregistrements, sans faire tomber le graphe | **A5** |
| `temp_uuid` | nuit, §2, « petit, isolé, donc tôt » | documenté, pas renommé — au motif que le nom voyage dans des checkpoints ; c'est le **champ** qui voyage, pas la méthode | renommer la méthode, garder le champ | **A6** |
| `Catalog::search` en gabarit | 5 sept, §3 | 417 lignes, deux chemins, **deux corrections portées deux fois cette session, une à moitié** | le gabarit devient le seul chemin | **B** |
| sparse et dense partent ensemble | table d'approximation | vrai, et **ce n'est pas un défaut** sur un embarqueur dual : les deux sortent de la même passe | rien ; à écrire comme tel dans la table | fait ici |
| `fusion.rs` | nuit, §6 | en-tête d'avertissement ; aucun appelant, formule ancienne | supprimer | **E** |
| `KBSearchNode`, `templates/search.mmd`, `search_expansion.mmd` | nuit, §6 | chargés par deux tests de parsing, aucun outil | tranché par **B** : ce que le gabarit unique absorbe ou rend inutile | **B** |
| `crate::acces` | 5 sept, nuit §6 | sans appelant parce que **rien n'ouvre un catalogue en lecture** | l'ouverture en lecture — c'est une pièce de la concurrence | **C3** |
| `title_boost`/`content_boost`/`boost` | nuit, §6 | avertissement au montage | une branche BM25 par champ, pesée à la fusion — une **fonctionnalité**, pas une fondation | **F** |
| la troncature d'embarquement | 5 sept, nuit §7 | comptée et dite | dériver la taille de chunk de la fenêtre du modèle | **F** |
| le handle FTS manquant | nuit, §7, « c'est au type de le tenir » | une règle exacte qui **crie** ; le type ne le tient pas | avec **C1** : le point d'entrée unique de mise en file ouvre les handles, il n'y a plus quatre endroits à oublier | **C1** |
| les quatre registres de services | nuit | noyau extrait ; `ingest_entities` n'enregistre ni `kb_metadata` ni `event_bus` | vérifier si un nœud du graphe d'`ingest_entities` lit l'un des deux ; si non, l'absence est sans effet et se dit ; si oui, c'est un défaut | **A7** |
| le nom `immediate` sur la fiche d'outil | 6 sept | dit *quand* et pas *quoi* | avec **B**, la fiche parle `exige` comme la bibliothèque | **B** |
| la vraie concurrence sur rag3db | 5 sept, obj. 1b-d | rien de commencé ; le MVCC de Vela **jamais relu** | voir §2 D | **D** |
| lecture des documents, tableurs, base étrangère | 5 sept, §2, 4, 5 | rien | additifs, derrière les fondations | **F** |

## 2. Les fondations, dans l'ordre

### A. Le contrat d'écriture ne ment plus — Rust, maintenant

**A1. Le silence du sparse sur le chemin composable.** `SparseSearchNode`
appelle `expliquer_le_silence_d_un_signal` quand il rend zéro, comme
`VectorSearchNode`. Quatre lignes. C'est le demi-portage que B supprimera
structurellement ; on ne laisse pas un agent conclure « ça n'existe pas » en
attendant.

**A2. `Donnee` exact.** Le graphe de `flush_insertions` gagne `LinkRecordNode`
après `InsertRecordNode`, avec `entity_configs` et ce que le nœud lit. Les
relations dont les deux bouts sont posés partent avec la donnée ; les agrégats
restent au dérivé, ce qui est juste : un agrégat *est* du dérivé. La table
d'approximation passe `Donnee` à « exact, agrégats exclus par définition ».

**A3. Les verbes unitaires deviennent honnêtes.** C'est l'objectif 1a du 5,
tel que Lucie l'a tranché :

- `create`, `update`, `delete`, `link` **posent la donnée avant de rendre**.
  Le `EntityRef` rendu par `create` est résolu ; `uuid()` répond tout de suite.
  Ce qui reste en file est le dérivé, et il est dit par le compte en attente.
- La forme est celle des autres options du moteur, *une option, pas un
  remplacement* : `create_jusqu_a(entity, data, exige)` rend
  `(EntityRef, FlushResult)` ; `create` = `create_jusqu_a(…, DONNEE)`.
  `exige = AUCUNE` est le lot déclaré — l'ancien comportement, pour qui empile
  puis draine, et ça se lit dans l'appel. `TOUT` draine la fermeture de
  l'entité, GPU compris. Même forme pour `update_jusqu_a`, `delete_jusqu_a`,
  `link_jusqu_a`.
- Les avertissements du flush partent sur le bus d'événements du catalogue,
  comme tout ce que le catalogue dit de lui-même ; la variante `_jusqu_a` les
  rend aussi dans son `FlushResult`.
- **Ce qui n'est pas naïf ici** : `create` en boucle sur dix mille lignes
  paierait dix mille graphes d'un nœud. C'est *exactement* l'écrivain que
  Lucie veut chaperonner, et la réponse est déclarée : `ingest_entities` pour
  le lot, ou `create_jusqu_a(…, AUCUNE)` puis `drain`. La doc de `create` le
  dit en tête. Le défaut protège « mon client a acheté un produit » ; il ne
  protège pas celui qui fait du lot sans le dire, et c'est voulu.
- Dépend de **C1** pour poser *la fermeture de cette entité* et non la file
  entière : tant que C1 n'est pas là, `create` posera toute la file d'entités
  en attente, ce qui est correct mais couplé. A3 et C1 se font donc ensemble,
  C1 d'abord.

**A4. Le rattrapage opportuniste.** Là où une passe GPU est déjà payée —
`drainer(true)` et `ingest_entities` complet — si l'indice
`peut_devoir_un_embarquement` est posé, `embarquer_le_retard(TOUT, borne)` en
sortie. Qui paie le GPU solde aussi la dette d'hier ; qui ne le paie pas ne le
paie pas. Ce n'est pas le tick, c'est ce qui en tient lieu tant qu'aucun
processus ne garde un catalogue en vie (voir F).

**A5. `FlushResult.failed` cesse d'être `0` en dur.** La cartographie des
douze nœuds d'écriture dit où : onze sur douze n'ont que deux façons de
finir — `?` (tout le graphe tombe, sans undo, base partiellement écrite) ou
`ctx.warn` (rien n'est compté). Les abandons silencieux sur config manquante
(`DeleteRecordNode`, `UpdateRecordNode`) perdent un **groupe entier** sans
résultat ni avertissement, et `drainer` le compte quand même dans
`processed`. Une transition de cycle de vie non déclarée sur **une** ligne
tue toute l'ingestion.

Le canal se crée sur le patron qui existe déjà pour `update_results`,
`chunk_counts`, `delete_results` : un service partagé
`Arc<Mutex<Vec<EchecDeGroupe>>>` — nœud, table, nombre d'opérations, la
disponibilité perdue, la cause — écrit **pendant** `execute` (il survit donc
à l'abandon de la phase 3 du runtime, ce qu'un port ne fait pas), et lu par
`drainer` à côté des trois autres. Un nœud dont un groupe échoue le consigne
et **continue les autres groupes**. Les refs des entités d'un groupe
d'insertion raté sont résolus en échec, pour qu'un lien vers elles échoue
tout de suite et se compte, au lieu d'attendre trente secondes.

Ce que `FlushResult` en fait : `failed` = les opérations en file dont
l'écriture propre a échoué (insertion, lien, mise à jour, suppression,
agrégat) ; `processed` = `op_count − failed` ; un échec **dérivé**
(découpage, embarquement, commit d'index) ne compte pas une opération mais
**retire la disponibilité perdue de `rendu_pret`** — un drain dont le commit
FTS a échoué ne dit plus `textsearch`. `UpdateStatus` gagne `Failed`. Le
rattrapage opportuniste (A4) reste conditionné à `failed == 0` : on ne
solde pas la dette d'hier sur un drain qui vient de rater.

**A6. `temp_uuid` → `cle_de_correlation`.** La méthode seulement. Le champ
sérialisé garde son nom, avec une ligne qui dit pourquoi les deux diffèrent.

**A7. Les registres de services : vérifier, pas supposer.** Regarder si un
nœud du graphe d'`ingest_entities` lit `kb_metadata` ou `event_bus`. Si aucun
ne le fait, l'absence est sans effet et le commentaire le dit. Si un le fait,
c'est un défaut à réparer sur-le-champ. Dans les deux cas le registre complet
devient **un seul** constructeur, et la divergence cesse d'exister.

### B. Un seul chemin de recherche — et ce que la cartographie a changé

**La cartographie de l'après-midi renverse la question.** `Catalog::search`
(416 lignes) n'a **aucun appelant de production** hors `KBSearchNode`, qui
n'est atteint que par `search_with_strategy`, qui n'est appelé par personne.
Le chemin que les agents empruntent — `search.mmd` → `SearchTool` →
`search_base.mmd` — **ne passe déjà plus par lui**. B n'est donc pas « faire
de `Catalog::search` un gabarit » ; c'est **rendre le chemin composable aussi
complet et aussi juste que le monolithe**, puis faire de `Catalog::search` le
lanceur de ce graphe, pour que ses tests l'éprouvent lui.

Les douze écarts, mesurés dans le code (les références sont dans le rapport
de cartographie ; les numéros sont ceux de l'ordre de traitement) :

| | l'écart sur le chemin des agents | gravité |
|---|---|---|
| B1 | `ResolveParentNode` et les trois compilations de filtre passent `target.name` là où il faut `target.parent_table` : **faux pour toute cible KB** (`MATCH (n:MaKB)` ne désigne aucune table) | défaut en exercice |
| B2 | `resolve_vector_chunks` code en dur `Rag3dbDialect` : **faux sur PostgreSQL** | défaut en exercice |
| B3 | aucune pagination : `offset` est inerte, la liste rendue peut faire `2 × limit` ; et aucun sur-fetch `(limit+offset)×2`, donc la fusion et le pool du rerank sont appauvris | défaut en exercice |
| B4 | les poids de fusion sont figés dans le gabarit (`bm25:0.6,vector:0.4`) ; `options.fusion` et le `fusion` déclaré d'une KB sont ignorés | contrat ignoré |
| B5 | `options.filters` (le `HashMap`) est ignoré ; seul `filter_condition` descend | contrat ignoré |
| B6 | dense + sparse = deux passes avant au lieu d'une (`dual_embedder` jamais utilisé par le nœud vectoriel) ; le cache d'embarquement du catalogue jamais consulté | coût |
| B7 | `RerankNode` sans port `meta` : « aucun reranker configuré » n'atteint jamais l'agent ; pas de plancher `max(limit+offset)` sur le pool ; pas d'enrichissement du pool avant le cross-encoder ; `reranked_count` jamais rendu | silence |
| B8 | `diagnostics` jamais produit ; `search_time_ms` est un max de nœuds ; `fused_count` faux | mesure fausse |
| B9 | pas d'ouverture paresseuse de l'index FTS : un handle non ouvert est une erreur dure | fragilité |
| B10 | `BM25SearchNode` : défaut `Contains` là où `SearchOptions` dit `Auto`, `fuzzy_distance` 0 contre 1 ; ne vérifie pas que la cible déclare BM25 | divergence de défauts |
| B11 | `SourceResolved` appliqué par signal avant la fusion, pas après la pagination : la déduplication par source n'est pas celle du monolithe | divergence |
| B12 | pas de fan-out de cellules ni de bascule de cellule ; `SearchCompleted` jamais émis | manque |

Trois choses n'existent **que** sur le chemin composable et ne doivent pas
se perdre : le domaine de travail (`SearchSourceNode`), la provenance des
signaux (`bm25+vector`), le rendu markdown avec lentille de chemins.

**Le plan de B, dans l'ordre :** B1 B2 (les deux défauts en exercice, une
demi-heure), B3 (un nœud de pagination, le sur-fetch par les paramètres du
gabarit), B4 B5 (la précédence : `options` > déclaration de la cible >
gabarit), B7 (un port `meta` au rerank, comme aux trois signaux), B6 (le
`QueryPayload` porte les vecteurs de la requête, embarqués une fois par la
source), B8 B9 B10 B11 ; puis **B13 : `Catalog::search` devient le lanceur**
— il monte les services, exécute `search_base` (plus la branche sparse), fait
le fan-out de cellules autour, convertit et émet `SearchCompleted`. Les cinq
tests `generic_*_matches_catalog` deviennent alors tautologiques et sont
gardés comme non-régression du gabarit ; les tests de diagnostics, de
`result_mode`, de rerank et de cellules passent par le lanceur et éprouvent
le graphe. `KBSearchNode`, `search_with_strategy` et les deux gabarits
`templates/search*.mmd` n'ont plus d'objet : retirés avec B13, ou réécrits
sur le lanceur si un test montre qu'un chemin en dépend.

### C. La ressource est l'unité — la moitié Rust de la concurrence

**C1. `PendingWork` partitionné par fermeture.** `build_ingestion_graph` et
`flush_insertions` ne consomment plus la file entière : ils prennent la
**fermeture d'une cible** — ses entités en attente, les relations qui les
touchent et les entités à l'autre bout de ces relations, transitivement, ses
mises à jour et suppressions, et pour une base de connaissances ses agrégats
avec les entités sources qu'ils lisent. Le reste **retourne dans la file**.
C'est l'invariant de Lucie, écrit : *deux ressources sans lien ne s'attendent
pas* ; deux ressources en lien, si, et la fermeture est la définition du lien.

Conséquences qui tombent avec :

- `appliquer_la_consigne` prend la cible, et une recherche sur A ne paie plus
  les écritures sur B — sur le chemin par défaut aussi.
- **le handle FTS manquant cesse d'être un oubli possible** : la fermeture
  connaît ses tables, et c'est *elle* qui ouvre les handles, à un seul endroit.
  Il n'y a plus quatre points d'entrée à tenir d'accord.
- A3 pose la fermeture de l'entité créée, pas la file.
- Le drain sans cible (`drain()`) reste le drain complet : la fermeture de
  tout est tout.

**C2. La marque d'eau publie par niveau.** Aujourd'hui elle est binaire — « cet
écrivain a du travail non publié ». Elle dit *jusqu'où* : `data` posée
jusqu'ici, `textsearch` jusque-là, `dense`/`sparse` dues. `attendre_les_ecritures`
prend `exige` et n'attend que les niveaux demandés. C'est ce que le 5 septembre
appelait la symétrie qu'on n'avait pas vue, et c'est du Rust pur.

**C3. Ouvrir un catalogue en lecture.** `crate::acces` choisit un chemin de
lecteur et personne ne l'appelle parce que rien ne construit un catalogue qui
ne pourra pas écrire. `Catalog::ouvrir_en_lecture(…)` : pas de file, pas
d'embarqueur obligatoire, recherche seule, par le démon ou en direct selon
`Acces`. C'est la pièce qui manque pour que deux processus lisent la même base
pendant qu'un troisième écrit — la clause 1 de la concurrence, côté lecture,
qui est acquise dans le cœur (80 ouvertures mesurées) et **pas exposée** ici.

**C4. La réclamation sur la dette d'embarquement.** Deux processus qui
rattrapent voient la même dette. Une réclamation avec péremption — la forme
exacte de la marque d'ingestion — sur les chunks qu'une passe prend :
`_embed_claim`, écrivain et horodatage, périmée après `MARQUE_PERIMEE_MS`. Sans
elle, deux rattrapages calculent deux fois. **Dépend de D** pour être
exercée à deux processus écrivains ; **écrite avant**, parce que sa forme ne
dépend pas de D et qu'un processus seul la traverse sans coût.

**C5. La dette de découpage vit dans la base.** Trouvé en écrivant C1 : une
mise à jour ne peut pas se poser au niveau `data` seul, parce que ses
conséquences sur les chunks — le redécoupage — ne sont demandées qu'au
moment où `UpdateRecordNode` les émet sur un port. Sans nœud en aval, elles
sont **perdues** : le `_content_hash` de l'entité est déjà neuf, un drain
ultérieur ne voit plus de changement, et les chunks restent périmés pour
toujours. C'est exactement la forme du défaut que `_embed_hash` a réglé pour
l'embarquement, et la réponse est la même : un marqueur en base (le hash du
contenu à partir duquel les chunks ont été découpés, sur le parent), une
requête qui retrouve les parents dont les chunks sont en retard, et une passe
qui les redécoupe. Tant que ce n'est pas là, `appliquer_la_consigne` **draine
le graphe sans GPU** dès qu'une mise à jour ou une suppression est dans la
fermeture — plus que demandé, jamais moins. Après C5, la mise à jour se
posera seule et le redécoupage rejoindra la dette.

### D. La vraie concurrence sur rag3db — le cœur C++

C'est le monument, et Lucie a dit de ne pas le contourner. Ses trois pièces
n'ont pas changé depuis le 5 :

1. **Relire le MVCC de Vela.** Rotation de WAL, points de reprise non
   bloquants. Personne ne l'a fait, et c'est lui qui décide si on bâtit
   dessus ou si on le remplace. **Préalable, pas première tâche** — mais un
   préalable qu'on **commence**, pas qu'on attend.
2. **Un gestionnaire de verrous inter-processus.** `F_WRLCK` en `F_SETLK`
   rend l'accès *impossible*, pas *ordonné*. Il faut un arbitre hors des
   processus.
3. **La granularité.** Sans elle, tout arbitrage dégénère en verrou global.

**Où ça vit.** Le cœur C++ est travaillé dans une autre session
(`rag3db-57`, Vela, MVCC). Ce que *cette* passe fait pour D, sans attendre :

- **la relecture du MVCC de Vela commence ici**, par une cartographie du
  gestionnaire de transactions et du WAL, écrite en doc, avec les questions
  précises que C1 à C4 lui posent : quelle granularité le cœur sait tenir
  (table, nœud, page ?), ce qu'un second écrivain voit d'une transaction en
  cours, ce que `debug_enable_multi_writes` fait vraiment. C'est la partie de
  D dont C dépend, et elle est de la lecture, pas de l'écriture ;
- **C1 à C4 sont écrits pour D** : la fermeture de ressource est ce que
  l'arbitre du cœur devra reconnaître, et la réponse ne peut pas être
  différente des deux côtés.

Ce qui reste à D après ça est du C++ dans le cœur, et il est **au-dessus** de
F dans l'ordre, pas en dessous.

### E. Ce qui se retire

- `fusion.rs` : supprimé. Supplanté, aucun appelant, ses tests éprouvent la
  formule ancienne. Un `git revert` le rend si quelqu'un y tenait.

### F. Ce qui attend derrière les fondations, et pourquoi c'est légitime

Chaque ligne nomme ce dont elle dépend. Aucune n'est « pas grave ».

| | dépend de |
|---|---|
| **le tick de fond** (Nagle) | un processus qui garde un catalogue en vie. Il n'existe pas : le démon tient une connexion. Le jour où le catalogue vit dans un processus durable — ce que D et C3 finiront par poser — le fil de fond s'écrit en vingt lignes sur `embarquer_le_retard` |
| **la pondération par champ** (`boost`, `title_boost`) | B : c'est une topologie de gabarit — une branche BM25 par champ, pesée à la fusion — et elle ne s'écrit qu'une fois qu'il n'y a qu'un gabarit |
| **la taille de chunk dérivée du modèle** | rien de technique ; c'est une surface de configuration. Après A, parce que A5 change déjà les structures publiques et qu'on groupe les ruptures |
| **la lecture des documents** (pdf, docx, pptx, html, csv) | additif. Premier des additifs, derrière D |
| **les tableurs**, **la base étrangère** | additifs, dans cet ordre, décision du 5 |

## 3. Les questions qui sont vraiment à Lucie

Posées avec la réponse que je prends faute d'autre, pour que rien n'attende.

1. **`create` synchrone au niveau `data`** est-il bien le défaut, avec le lot
   déclaré par `exige = AUCUNE` ? *Je prends oui — c'est la décision du 5.*
2. **A5** change `UpdateResult`, `DeleteResult` et le contrat des nœuds
   d'écriture (un groupe en échec ne fait plus tomber le graphe). *Je prends
   oui — le tout-ou-rien au niveau `Err` était le mensonge de `failed`.*
3. **D côté C++** : cette session s'en tient à la cartographie du MVCC de Vela
   et aux pièces Rust ; le cœur reste à `rag3db-57`. *Si tu veux que le C++ se
   fasse ici aussi, dis-le, et il monte au-dessus de B.*

## 4. L'ordre d'exécution de cette passe

**État au fil de la passe** (mis à jour à chaque commit) :

| | état |
|---|---|
| A1 A2 A6 A7 | faits — `864779778` |
| C1 | fait — `9fae08275`, 25 suites e2e vertes |
| A3 A4 | faits — 26 suites e2e vertes |
| A5 | en conception (cartographie des nœuds d'écriture) |
| B | en conception (cartographie de `Catalog::search` contre les nœuds) |
| C2 | fait — onze suites e2e vertes |
| A5 | écrit, 953 tests de bibliothèque verts, passe e2e en cours |
| D (cartographie) | en cours — relecture du MVCC de Vela par un agent, en doc |
| C3 C4 C5, E | à faire |


```
A1  A2  A6  A7  E        — les lignes qui ne se décident pas, un commit chacune
C1                       — la fermeture de ressource
A3                       — les verbes honnêtes, sur C1
A4                       — le rattrapage là où le GPU est payé
A5                       — le canal d'échec
C2  C3  C4  C5           — marque par niveau, lecture seule, réclamation, dette de découpage
B                        — un seul chemin de recherche
D (cartographie)         — la relecture du MVCC de Vela, en doc
```

Chaque étape relance sa suite e2e ; la passe complète se fait suite par suite
(le plafond à 16 Go du script, voir le knowledge dump §1).
