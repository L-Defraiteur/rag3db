# Plusieurs développeurs, leurs migrations, aucun bloqué

*Arbre principal, 13 octobre 2026. Demande de Lucie, relayée par l'orchestration : présenter le cas
d'usage « rag3weaver comme base d'une application à plusieurs développeurs ». Chacun écrit et fait
ses migrations (ajouter une colonne, une relation, un index implicite) sur la même base, en même
temps, et aucun n'est bloqué par l'autre. La part moteur est lue dans le code de master
(`0227206ab`), et le cœur C++ l'a relue et complétée le 13 octobre (sa carte complète :
`coeur-cpp/15-migrations-concurrentes-la-part-moteur.md`). Le scénario (§6) est celui du banc. Rien n'est
mesuré ici : le poste est en pause jusqu'au 17 octobre, et chaque chiffre cité vient d'une page
déjà écrite.*

## 1. Le cas

Une application dont rag3weaver est la base. Trois développeurs y travaillent. Chacun a sa branche
et son gabarit (`backend.json` et ses outils), et tous visent la même base partagée, celle de
l'équipe : la base de recette, ou la base de développement servie par l'hôte du démon.

| ce que fait un développeur | ce qu'il attend |
|---|---|
| ajoute un champ `priorite` à l'entité `Ticket` | le champ existe pour tous dès que sa migration passe ; les lignes existantes ont la valeur par défaut ; les autres continuent d'écrire pendant ce temps |
| ajoute une relation `BLOQUE` entre deux `Ticket` | la relation existe, ses outils l'écrivent ; rien d'autre n'a bougé |
| rend `Ticket.resume` cherchable en plein texte, ou embarqué | l'index se construit en fond ; les recherches répondent pendant ce temps, l'état d'index dit « en construction » ; il n'a écrit aucun `CREATE INDEX` |
| se trompe : sa migration échoue à mi-chemin | rien de sa migration ne reste ; les autres n'ont rien vu ; le refus nomme l'étape |
| arrive sur une base neuve (un collègue, la CI) | les migrations se rejouent dans l'ordre, et la base est la même que celle de l'équipe |
| rejoue par erreur une migration déjà passée, ou modifiée depuis | un refus nommé, jamais une seconde application |

« Aucun bloqué » se lit ainsi : une migration fait attendre les autres au plus le temps d'une
écriture de catalogue (une centaine de millisecondes, §6), jamais le temps de réécrire une table
ou de bâtir un index. Une écriture ordinaire n'est jamais refusée parce qu'une migration passe.

## 2. Ce que fait PostgreSQL

La référence, comme pour le reste du moteur (« s'inspirer des moteurs établis ») :

- **Le DDL est transactionnel.** `BEGIN; ALTER …; ALTER …; COMMIT` passe en entier ou pas du tout,
  écritures comprises. Les exceptions se comptent : `CREATE INDEX CONCURRENTLY`, `CREATE DATABASE`,
  `VACUUM`.
- **`ADD COLUMN` est une écriture de catalogue seule** depuis la version 11, même avec un défaut
  constant. Le défaut est rangé au catalogue (`attmissingval`) et rendu aux lignes anciennes à la
  lecture ; aucune ligne n'est réécrite. Seuls un défaut volatil (`now()`, `random()`) et un
  changement de type réécrivent la table.
- **Des verrous par table, gradués.** `ALTER TABLE` prend `ACCESS EXCLUSIVE` sur sa table seule, le
  temps de l'écriture de catalogue. `CREATE INDEX` prend `SHARE` (les lectures passent, les
  écritures attendent la construction) ; `CREATE INDEX CONCURRENTLY` ne bloque pas les écritures,
  mais ne vit pas dans une transaction. Le piège connu est la file des verrous : un `ALTER` qui
  attend derrière une longue transaction fait attendre tous ceux qui arrivent après lui. L'usage
  impose donc un `lock_timeout` court et un nouvel essai.
- **Pas de table des migrations native.** L'usage l'impose (Flyway, Liquibase, Rails, sqitch) :
  - une table `schema_migrations` (version, nom, somme de contrôle, date) ;
  - un ordre total ;
  - un verrou consultatif (`pg_advisory_lock`) pour qu'un seul exécutant migre à la fois ;
  - le refus d'une migration déjà passée dont la somme a changé ;
  - pour les changements sans arrêt, la forme « étendre puis contracter » : ajouter, écrire aux
    deux endroits, migrer les données, retirer l'ancien plus tard.

## 3. Ce que fait le produit aujourd'hui

Lu dans le code de master. Les références sont relatives à `extension/rag3weaver/`.

**Le schéma évolue à l'ouverture, de façon additive, hors transaction.**
- `Backend::open_with_reason` (`src/backend.rs:1529`) appelle `initialize`, puis `register_entity`
  et `register_relation_with` pour chaque déclaration du manifeste.
- `migrate_entity` (`src/catalog.rs:2362`) compare la configuration rangée en méta
  (`entity_config:{E}` dans `_catalog_meta`) à la nouvelle :
  - un champ nouveau donne `ALTER TABLE … ADD` ;
  - un champ retiré ou retypé est refusé (« destructive migration not supported ») ;
  - un signal vecteur ajouté crée le stockage du modèle et son index ;
  - un changement de contenu pose seulement un drapeau `needs_reindex`.
- Chaque instruction part en autocommit, avant que le catalogue soit partagé.
- La règle est écrite au catalogue : aucun DDL dans une transaction par paquet (`src/catalog.rs:283`),
  pour qu'une annulation n'emporte pas le schéma avec les lignes.

**Changer le schéma, c'est fermer le backend.**
- Un rechargement à chaud refuse tout changement d'entité ou de relation : « c'est une migration, pas
  un rechargement » (`src/backend.rs:2505`).
- La seule voie est `POST /backend/open` sur un manifeste modifié. L'hôte ferme le backend et le
  rouvre (`src/daemon_host.rs:331`), et la fermeture est refusée tant qu'une indexation le tient.
- Pendant ce temps, **toutes les sessions de cette base attendent** : c'est le blocage que le cas
  veut supprimer.

**Pas de journal des migrations en service.**
- Un `MigrationRunner` existe (`src/dataflow/migrations.rs:266`). Il a :
  - des fichiers `NNN_nom.mmd` ;
  - une table `_DataflowMigration` (version, statut, somme de contrôle) ;
  - un verrou de 10 minutes et une annulation.
- Mais **aucun binaire, aucune route ni aucun outil ne l'appelle**. De plus :
  - sa somme de contrôle est rangée mais jamais comparée ;
  - ses étapes sont du Cypher brut, qui ne met pas à jour la méta `entity_config` : il créerait une
    dérive entre ce que le catalogue croit et ce que la base porte.
- La seule version de schéma est interne (`schema_version` = 8, `src/scope.rs:65`), pour les colonnes
  système. Elle ne connaît pas l'application.

**L'atomicité manque même à l'ouverture.**
- L'`ALTER` part avant que la méta soit écrite (`src/catalog.rs:2401`, puis `:1846`).
- Sur rag3db, `ALTER … ADD` n'a pas de `IF NOT EXISTS` (`src/dialect.rs:1612`).
- Une mort entre les deux laisserait une colonne que la méta ignore, et le même `ALTER` échouerait
  à l'ouverture suivante. Lu dans le code, pas éprouvé.

**Le schéma est global à la base.** Les cellules (`org`/`project`) marquent les lignes et ont leurs
index plein texte et sparse à elles, mais les tables, les colonnes et les index vecteurs sont
communs. Un `ALTER` d'un projet vaut pour tous.

**Les écrivains d'une base sont sérialisés deux fois.**
- Un seul processus écrit (verrou du fichier, et la marque d'écrivain de `3e5d3a136`).
- Dans l'hôte, chaque appel tient le backend (`with_backend`, `src/daemon_host.rs:221`), et toute
  écriture prend le mutex du catalogue.
- Les lectures n'attendent plus ni l'un ni l'autre (L1 à L3, et `4aa8c95d5` en attente de la
  reprise).
- Les écritures, si : deux développeurs qui écrivent passent l'un après l'autre, appel par appel.

## 4. Ce que fait le moteur

Lu dans `src/` et `extension/vector/` à `0227206ab`, relu et complété par le cœur à `65cb9a1f3`.

**Le DDL ordinaire est transactionnel, comme chez PostgreSQL.**
- CREATE et DROP de tables, ALTER ADD/DROP/RENAME de propriété, RENAME de table : tous passent par
  le catalogue versionné et le tampon d'annulation (`src/catalog/catalog_set.cpp:77-220`,
  `src/storage/undo_buffer.cpp:273-295`), et sont journalisés.
- Ils vivent dans un `BEGIN … COMMIT` mêlés à des écritures. Les tests le montrent, pour chaque
  opération, en Commit, Rollback et Recovery :
  - `test/test_files/transaction/ddl/ddl_tinysnb.test` ;
  - `test/test_files/transaction/concurrency/ddl_serial_execution.test`.
- Un ROLLBACK défait l'entrée de catalogue. Pour un ADD, la colonne physique reste jusqu'au point de
  reprise suivant, qui ne garde que les propriétés du catalogue (`src/storage/table/node_table.cpp:996`).

**Ce qui refuse la transaction explicite :**

| instruction | refus | où |
|---|---|---|
| `CREATE_VECTOR_INDEX` | « CREATE_VECTOR_INDEX is only supported in auto transaction mode. » | `extension/vector/src/index/hnsw_index_utils.cpp:150` |
| `CHECKPOINT` | « Found active transaction for CHECKPOINT. » | `src/processor/operator/transaction.cpp:86` |
| `DROP_VECTOR_INDEX` | **permis, mais non annulable** : il efface l'index tout de suite, et rien ne le remet au ROLLBACK (E9) | `node_table.cpp:1341-1351` |
| index spatial (`extension/geo`) | ses CREATE et DROP forcent un point de reprise (`create_spatial_index.cpp:202`, `drop_spatial_index.cpp:85`) ; qu'ils refusent la transaction explicite n'est pas vérifié | — |
| `CREATE_FTS_INDEX` | d'après la stèle du moteur seulement ; sa source n'est pas dans ce dépôt | — |

C'est l'écart que le banc a vu le 5 octobre, quand un CREATE INDEX a refusé la transaction explicite
(`docs/tickets/closed/2026-10-05-transaction-a-moitie-apres-une-mort-pendant-son-point-de-reprise.md`).
Il ne touche que la **création d'index**, pas le DDL en général.

**Un DDL ne prend aucun verrou de table.**
- Le gestionnaire de verrous ne connaît que `ROW` et `INDEX` (`src/include/transaction/lock_manager.h:40`).
- Aucun opérateur de `src/processor/operator/ddl/` ne l'appelle.
- Le catalogue n'a qu'un `shared_mutex` par ensemble, tenu le temps d'une opération sur la table de
  hachage (`src/include/catalog/catalog_set.h:91`).
- Un DDL tient pourtant d'autres verrous, et c'est l'essentiel :
  - `ADD COLUMN` tient le verrou de la collection de groupes de sa table pendant toute sa boucle,
    en temps proportionnel au nombre de lignes, ce qui arrête `append` et la création de groupes ;
  - or les ajouts ont lieu à la validation (`localStorage->commit()`), sous le mutex qui sérialise
    begin, commit et checkpoint : **toutes les validations du processus attendent derrière un ADD
    COLUMN**, celles des autres tables comprises. Pendant leur exécution, les écrivains des autres
    tables ne l'attendent pas ; à leur validation, si ;
  - le verrou exclusif du catalogue n'est tenu que pendant `alterTableEntry`
    (`catalog_set.cpp:185`), relâché avant le stockage ;
  - les lecteurs ne sont pas bloqués (le catalogue est versionné).
- Rien ne sérialise un DDL avec les écrivains de **sa** table.
- Pour les conflits de catalogue :
  - deux CREATE concurrents du même nom se refusent (« Write-write conflict on creating catalog entry
    with name … », `catalog_set.cpp:101`) ;
  - en revanche, `alterTableEntry` et `dropEntry` ne contrôlent aucun conflit (`catalog_set.cpp:159-220`).
- **Deux ALTER concurrents sur une table pourraient écraser l'un l'autre.** C'est déduit du code : le
  seul test du cas (`ddl_concurrent_execution.test`, `ConcurrentMultiWriteAlter`) est marqué `-SKIP`
  depuis Kuzu.
- **Un écrivain de la table pendant un ADD, c'est pire que « ne se voient pas ».** La colonne
  partagée est ajoutée pour tous tout de suite (`node_table.cpp:845`, puis la collection de
  groupes), et seule la table locale de la transaction DDL la reçoit (`:848`). L'autre écrivain a
  des lignes locales sans cette colonne. À sa validation, elles sont fusionnées dans des groupes qui
  en ont une de plus : ce qui s'y passe (des NULL ? une assertion ?) n'est pas éprouvé, et c'est ce
  que E3 doit mesurer. De plus, l'ADD ne prend pas le verrou des ajouts en mémoire : la liste des
  colonnes et celle des morceaux se réallouent sous un lecteur qui ne tient que lui
  (`node_group.h:280-285`).
- **ADD, ROLLBACK, puis un autre ADD avant tout point de reprise** : le catalogue rend son
  compteur de colonnes (`property_definition_collection.cpp:56`), mais le stockage garde la colonne
  annulée. La colonne suivante reçoit au catalogue l'identifiant de l'annulée et au stockage
  l'indice d'après, si bien qu'une lecture par l'identifiant lirait la colonne annulée. Aucun test ne
  le joue (E10).
- **Le gestionnaire de verrous** n'a qu'un mutex et une condition pour toute sa table, et réveille
  tout le monde à chaque libération (`notify_all`, `lock_manager.cpp:251`, par tranches de 10 ms) :
  un coût qui grandit avec le nombre d'écrivains. Et sur une table qui a un index HNSW, chaque
  écrivain tient INDEX en exclusif toute sa transaction (`node_table.cpp:624-637`) : **après la
  création d'un index vecteur, les écrivains de la table sont sérialisés** (E11).

**`ADD COLUMN` réécrit la table.**
- `NodeGroupCollection::addColumn` tient le verrou de la collection de groupes pendant toute la
  boucle (`src/storage/table/node_group_collection.cpp:181-190`).
- Pour chaque groupe, `ChunkedNodeGroup::addColumn` (`chunked_node_group.cpp:474-485` à
  `65cb9a1f3`) crée un morceau de pleine capacité et y évalue l'expression de défaut par vecteurs de
  2 048 lignes (`column_chunk_data.cpp:524-539`), **même sans défaut** : des NULL écrits un à un.
- Un groupe sur disque voit son morceau écrit tout de suite, dans des pages neuves du fichier de
  données, **avant la validation** (`:314-322`).
- Une relation le fait deux fois, dans les deux sens (`rel_table.cpp:470-482`).
- Le coût suit donc le nombre de lignes, même sans défaut, au lieu de l'écriture de catalogue de
  PostgreSQL.
- Le journal ne porte que l'instruction, et le rejeu la recalcule.
- **Le coût n'est pas mesuré.**

**Le point de reprise sous plusieurs écrivains.**
- Begin, commit, rollback et checkpoint sont sérialisés par un mutex, avec un fsync par commit.
- Un point de reprise explicite ou forcé arrête les nouvelles transactions, lecteurs compris, puis
  attend le départ des autres, 5 s au plus (« Timeout waiting for active transactions to leave the
  system before checkpointing… »).
- `CREATE_VECTOR_INDEX` force un tel point de reprise, et attend **avant** de valider. Il ne retient
  pas « tout le monde » : il bloque les nouveaux BEGIN et attend, 5 s au plus, que les transactions
  ouvertes partent, sans les arrêter. Si elles ne partent pas, c'est lui qui échoue.
- Avec la rétention de la session hotfixs (α), le CREATE ne prend pas INDEX : les écrivains ne
  l'attendent pas pendant le bâti, mais l'index manque leurs lignes (ticket de hotfixs, témoin rouge
  sur master).
- A7 range les validations forcées en file : rouge à 5 015 ms sur l'ancien master, vert à 117 ms
  (page `10-octobre-2026-coeur-cpp-tickets/05`).
- A8 laisse le point de reprise automatique au dernier partant. Un BEGIN retenu plus de 250 ms fait
  reporter la demande : avec huit écrivains sans pause, le journal culmine à 74-91 Kio au lieu de
  210-358 Kio (page `06`).
- **La plus grande mesure à plusieurs écrivains est à huit. Il n'existe aucune mesure à vingt.**

**L'index vecteur a un défaut ouvert.** Un `DROP_VECTOR_INDEX` annulé perd l'index, et un CREATE
annulé bloque le suivant
(`docs/tickets/2026-10-12-un-index-retire-ou-recree-reste-visible-a-tous-dans-le-stockage.md`).

## 5. Les collisions

| collision | moteur aujourd'hui | produit aujourd'hui | ce qu'il faut |
|---|---|---|---|
| deux ADD COLUMN sur la même table, noms différents | aucun contrôle de conflit sur ALTER : écrasement possible (déduit, test `-SKIP`) | n'arrive pas : les migrations n'existent qu'à l'ouverture, un seul écrivain | les deux passent l'un après l'autre, chacun le temps d'une écriture de catalogue |
| deux ADD COLUMN du même nom | le second échoue au binder (le nom existe), ou passe s'ils sont concurrents (déduit) | — | le second est refusé en nommant le premier et sa migration |
| une migration pendant qu'un autre écrit dans la table | la colonne partagée existe tout de suite, mais pas dans les lignes locales de l'écrivain : sa validation les fusionne dans des groupes qui ont une colonne de plus (non éprouvé) ; ADD tient la collection de groupes le temps de réécrire la table, et toutes les validations du processus l'attendent | la migration ferme le backend : l'autre attend tout | l'écrivain attend au plus l'écriture de catalogue, ou écrit avant et la migration le voit |
| une migration qui crée un index pendant que d'autres écrivent | `CREATE_VECTOR_INDEX` force un point de reprise et attend le départ de tous (5 s au plus, sinon refus), nouvelles transactions retenues | l'index naît à l'ouverture, ou en fond par la dette d'embarquement | l'index se bâtit en fond, hors de la transaction de migration, sans retenir les écrivains ; l'état d'index le dit |
| une migration qui échoue à mi-chemin | transactionnelle au catalogue si elle n'a que du DDL ordinaire, mais la colonne annulée reste au stockage jusqu'au point de reprise, et un ADD suivant se trompe d'indice (E10) ; un index en fait deux transactions, et un DROP d'index annulé est perdu | chaque instruction en autocommit : la première reste, et la méta peut ignorer une colonne (lu, pas éprouvé) | une migration = une transaction : DDL, méta et journal ensemble ; l'index après le COMMIT |
| un rejeu sur une base neuve | — | le manifeste porte l'état final, pas le chemin : une base neuve reçoit tout d'un coup, ce qui marche pour l'additif seul | les migrations du journal, dans l'ordre, rejouées à l'identique |
| une migration déjà passée, rejouée ou modifiée | — | — | refus nommé : déjà passée le …, ou somme différente de celle du journal |

## 6. Le scénario du banc

*Écrit par le banc le 13 octobre (`~/.cache/rag3db-banc-notes/scenario-vingt-clients.md`), avec la
colonne `vec` corrigée avec l'arbre principal. Sa forme finale, bornes arrêtées avec le cœur
d'après sa lecture du code à `65cb9a1f3`, est du 13 octobre au soir. Il se jouera le samedi 17, au mot
de Lucie. Il vivra dans
`test/transaction/concurrence/twenty_writers_scenario_test.cpp` (`long.txt`, 6 minutes ; une
version courte, un DDL toutes les 15 s, dans la passe ordinaire).*

**Ce qui tourne.** Une base, ouverte une fois, en mode multi-écrivains, avec une connexion par fil :
- **vingt écrivains**, chacun en boucle sur une petite transaction : `MERGE` d'un `Doc` par sa clé,
  `SET` de deux propriétés et de `vec`, une relation vers un voisin ; une fois sur dix, deux
  instructions dans un `BEGIN … COMMIT` ;
- **deux écrivains d'une autre table, `Note`**, que le DDL ne touche jamais : un ADD COLUMN retient
  toutes les validations du processus (§4), on le mesure donc sur une table étrangère aussi ;
- **quatre lecteurs** : un `Doc` par sa clé, puis ses voisins ;
- **un DDL par minute.**

Au départ : `Doc(id INT64 PRIMARY KEY, titre STRING, corps STRING, vec FLOAT[64])`, 50 000 `Doc`
(vecteur NULL), 150 000 relations, chargés par `COPY`, puis un point de reprise.

| minute | opération | chez PostgreSQL |
|---|---|---|
| 1 | `ALTER TABLE Doc ADD note STRING` | une écriture de catalogue |
| 2 | `ALTER TABLE Doc ADD rang INT64 DEFAULT 0` | une écriture de catalogue (défaut constant) |
| 3 | `CREATE REL TABLE Cite(FROM Doc TO Doc)` | une écriture de catalogue |
| 4 | `CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'cosine')` | `CREATE INDEX CONCURRENTLY` : les écritures passent |
| 5 | `CHECKPOINT` au milieu des écritures | n'arrête aucun écrivain |
| 6 | `BEGIN ; ALTER TABLE Doc ADD a STRING ; ALTER TABLE Doc ADD a STRING ; COMMIT`, puis, avant tout point de reprise, `ALTER TABLE Doc ADD b STRING`, un SET de `b` et sa relecture | tout annulé, personne n'a rien vu ; `b` se lit tel qu'écrit |

**Ce qu'on mesure**, par acteur, hors DDL et pendant chaque DDL :
- la latence des écrivains (médiane, p99, maximum) et des lecteurs ;
- la durée de chaque DDL, et l'écart entre son début et la première écriture qui repasse ;
- chaque erreur, rangée par son message ; une erreur sans nom est un défaut en soi ;
- à la fin, les comptes égaux aux écritures validées, après un arrêt propre puis après un arrêt
  brutal pendant la minute 5 ;
- le catalogue attendu : `note`, `rang`, `Cite` et `doc_vec`, mais pas `a`.

**Les bornes, arrêtées avec le cœur :**
1. ADD sans défaut et ADD avec défaut : **pas de borne promise**. Le moteur réécrit chaque ligne
   (§4) : la première mesure donne la durée et l'attente, sur `Doc` et sur `Note`, et l'écart à
   PostgreSQL est le constat (E1).
2. CREATE REL TABLE : une écriture de catalogue, personne n'attend plus que sa durée plus 50 ms.
3. CREATE_VECTOR_INDEX : c'est le seul DDL qui force un point de reprise. Il attend au plus 5 s
   les transactions ouvertes, puis échoue sous un nom ; les lecteurs ne sont pas refusés. Après lui,
   les écrivains de `Doc` sont sérialisés par le verrou INDEX exclusif (E11), et on le mesure.
4. CHECKPOINT : personne n'attend plus que lui plus 50 ms.
5. La migration échouée ne laisse rien (catalogue, colonnes, journal relu après réouverture), et
   personne d'autre n'a reçu d'erreur. ALTER est permis en transaction explicite : elle se joue
   telle quelle.
6. `b` se relit tel qu'écrit. **Rouge probable** : c'est E10, le trou que le scénario doit montrer.
7. Partout : aucune erreur sans nom, aucune perte. Le p99 des écrivains hors DDL est **mesuré, pas
   promis** : un fsync par validation, sous le mutex public.

L'attente d'un verrou n'est lisible nulle part dans le moteur : tout se mesure de bout en bout.

Ce que la lecture du code (§4) annonce avant la passe :
- la minute 1 réécrira 50 000 lignes, et les écrivains de `Note` l'attendront à leur validation ;
- la minute 4 bloquera les nouveaux BEGIN jusqu'à 5 s, le temps que les transactions ouvertes
  partent ;
- la migration annulée de la minute 6 sera transactionnelle au catalogue, mais le `ADD b` qui la suit
  lira probablement la colonne annulée (E10) ;
- après la minute 4, les écrivains de `Doc` seront sérialisés par le verrou INDEX exclusif (E11).

## 7. Les écarts mesurables

| # | écart | à qui | comment il se mesure |
|---|---|---|---|
| E1 | **`ADD COLUMN` réécrit la table** sous le verrou de sa collection de groupes, et toutes les validations du processus l'attendent ; PostgreSQL écrit le catalogue seul et range le défaut | cœur | minutes 1 et 2 du scénario ; durée d'un ADD sur 50 000 puis 500 000 lignes, et l'attente d'une validation de la même table et d'une autre |
| E2 | **Aucun contrôle de conflit sur ALTER et DROP concurrents** (déduit du code, test hérité `-SKIP`) | cœur | un témoin à deux ALTER concurrents sur une table, rouge d'abord ; le scénario ne l'exerce pas |
| E3 | **Aucun verrou de table pour le DDL** : l'écrivain d'une table pendant un ADD valide des lignes qui ont une colonne de moins que leurs groupes ; la liste des colonnes se réalloue sous un lecteur | cœur | un écrivain de `Doc` pendant la minute 1 : sa validation passe-t-elle, et sa ligne a-t-elle `note` à NULL ? |
| E4 | **La création d'un index refuse la transaction explicite et force un point de reprise** qui retient tout le monde jusqu'à 5 s ; PostgreSQL a `CONCURRENTLY` | cœur, puis produit | minute 4 ; le produit, lui, n'a pas besoin que l'index soit dans la transaction (§8) |
| E5 | **Vingt écrivains et un point de reprise en route** : la plus grande mesure est à huit (A8) | banc | le scénario entier |
| E6 | **Le produit migre en fermant le backend** : toutes les sessions attendent | arbre principal | une migration pendant que deux sessions écrivent : leur attente, avant et après le §8 |
| E7 | **Le produit écrit le DDL et la méta en deux temps**, sans `IF NOT EXISTS` sur rag3db | arbre principal | un arrêt brutal entre l'ALTER et la méta, puis la réouverture (témoin rouge d'abord) |
| E8 | **Pas de journal des migrations** : ni ordre, ni rejeu, ni refus d'une migration passée | arbre principal | les trois lignes du bas du §5, en témoins |
| E9 | **Un ROLLBACK d'index vecteur perd l'index** : DROP_VECTOR_INDEX est permis en transaction, mais non annulable (ticket ouvert du 12 octobre) | cœur | ses témoins `known_red` existent |
| E10 | **ADD, ROLLBACK, puis ADD avant un point de reprise** : le catalogue réutilise l'identifiant, le stockage garde l'indice, et la seconde colonne lit l'annulée | cœur | la minute 6 suivie d'un `ADD b`, puis une lecture de `b` |
| E11 | **Le gestionnaire de verrous réveille tout le monde** à chaque libération, et une table HNSW sérialise ses écrivains (INDEX exclusif pour toute la transaction) | cœur | les minutes 4 à 6 : le p99 des écrivains de `Doc` avant et après la création de l'index |

## 8. Les migrations comme objet du produit

La forme proposée. Elle attend le choix de Lucie sur les trois questions du §9 ; aucune ligne de
code n'est écrite.

1. **Déclarées au gabarit.** Le développeur change le gabarit (un champ, une relation, un signal de
   recherche) et le produit en tire la migration. Le gabarit reste la vérité : une migration est la
   différence entre deux gabarits, nommée et rangée sous `migrations/NNN_nom.json`. Elle porte des
   **étapes déclaratives** (`ajouter_champ`, `ajouter_relation`, `ajouter_signal`…), jamais du
   Cypher. Un développeur ne l'écrit à la main que pour ce que la différence ne dit pas : un
   renommage, une conversion de type, un retrait.
2. **Appliquées par l'hôte, sans fermer le backend.**
   - Une route ou un outil `migrate` prend le mutex du catalogue le temps d'**une** transaction du
     moteur : le DDL, la méta `entity_config` et la ligne du journal, ensemble.
   - Puis l'hôte publie une vue de lecture neuve.
   - Les autres sessions attendent ce temps-là, et lui seul.
   - Pas de réouverture, donc pas de point de reprise forcé et pas d'indexation interrompue.
3. **Les index après le COMMIT, en fond.** Une étape qui rend un champ cherchable ou embarqué ne crée
   pas d'index dans la transaction. Elle pose la dette (plein texte à rebâtir, vecteurs à
   embarquer), comme le fait déjà le changement de modèle. L'index se construit ensuite par la passe
   en fond, et l'état d'index dit « en construction ». C'est la règle du §10 appliquée aux
   migrations, et elle contourne E4 sans attendre le cœur.
4. **Un journal, un ordre, des refus.**
   - Une table `_Migration` porte, pour chaque migration :
     - le numéro et le nom ;
     - la somme de contrôle de ses étapes ;
     - l'auteur (la session ou le développeur) ;
     - la date d'application et la durée.
   - L'hôte refuse en nommant la cause :
     - une migration déjà passée ;
     - une migration passée dont la somme a changé ;
     - un numéro déjà pris par une autre ;
     - une étape qui contredit la base (le champ existe avec un autre type).
   - Deux développeurs qui migrent en même temps passent l'un après l'autre, dans l'ordre d'arrivée,
     chacun le temps de sa transaction.
5. **Le rejeu.** Une base neuve reçoit les migrations du journal du gabarit, dans l'ordre. Chaque
   application est rejouée à l'identique, et l'ouverture vérifie que la base porte bien la dernière.
6. **Le `MigrationRunner` existant** n'est pas branché tel quel : ses étapes sont du Cypher brut qui
   contourne la méta (E8). Il se reprend :
   - sa table, sa somme et son verrou servent ;
   - ses étapes deviennent les étapes déclaratives du point 1 ;
   - la somme est comparée.

Ce que cette forme demande au moteur :
- **Pour commencer, rien de neuf, mais deux défauts à fermer.** Le DDL ordinaire est déjà
  transactionnel au catalogue, et les index sortent de la transaction. Mais une migration qui
  échoue doit ne rien laisser : il faut que l'annulation défasse aussi le stockage d'un ADD et d'un
  CREATE REL TABLE, et que le compteur de colonnes ne redescende pas tant que le stockage tient
  l'indice (E10). Et il faut que l'hôte ne laisse aucun écrivain de la table valider pendant l'ADD
  (E3) : dans un seul processus, le mutex du catalogue le fait déjà, puisque toute écriture le prend.
- **Ensuite, la liste du cœur**, dans son ordre :
  1. le contrôle de conflit écriture-écriture sur ALTER et DROP (E2) ;
  2. l'annulation du stockage (E10) ;
  3. un verrou nommé de table, exclusif au DDL et partagé aux écrivains, sur le modèle d'INDEX (E3) ;
  4. un ADD COLUMN paresseux à la PostgreSQL 11 (E1) : la colonne déclarée au catalogue avec son
     défaut, le morceau créé au premier SET ou au point de reprise.
- C'est E1 qui rend vrai « le temps d'une écriture de catalogue » sur une grande table, et E2 et E3
  qui laissent deux processus migrer sans se gêner.

## 9. Les questions à Lucie

*Tranchées par Lucie le 18 octobre : voir le §11.*

1. **Migration tirée de la différence de gabarits, ou fichier écrit à la main ?** Je propose tirée,
   avec un fichier rangé et nommé : c'est la règle « un index ne se pense pas » étendue au schéma.
2. **Un schéma par projet (cellule), ou un schéma par base ?** Aujourd'hui, une base. Plusieurs
   applications dans une base demanderaient des tables par cellule.
3. **Les retraits et les conversions** : refusés comme aujourd'hui, ou en « étendre puis
   contracter » (ajouter, écrire aux deux, retirer plus tard, chaque temps une migration) ?

## 10. La règle « un index ne se pense pas »

Écrite dans la stèle du produit (`11-octobre-2026-arbre-principal/02-la-stele-du-produit.md`, §4),
avec ce qui la tient déjà et ce qui ne la tient pas encore.

## 11. Les décisions du 18 octobre, et les lots

*Arbre principal, 18 octobre 2026. Lucie a tranché les trois questions du §9 (notées par
l'orchestration dans `11-octobre-2026-10h51/orchestration/04-les-buts.md` §4 bis, `d888b6365`) :*

1. **un schéma par projet**, c'est-à-dire par cellule, et non par base ;
2. **la migration tirée de la différence de gabarits** quand c'est possible, rangée dans un fichier
   nommé et relisible ;
3. **les retraits et les conversions en étendre puis contracter** : ajouter, écrire aux deux,
   retirer plus tard, chaque temps étant une migration. Le refus d'aujourd'hui disparaît.

Ce qui suit dit ce que chaque décision demande au produit, puis les lots dans l'ordre. Rien n'est
écrit en code.

### 11.1 Un schéma par cellule

**Aujourd'hui** (§3), les tables, les colonnes et les index vecteurs sont communs à la base. Une
ligne porte `_org` et `_project`, et seuls le plein texte et le sparse ont des index par cellule
(suffixe `__org__project`, `ir/src/scope.rs:60-81`).

**Ce que demande la décision :**
- **Des tables par cellule.** `Ticket` d'`acme/alpha` et `Ticket` d'`acme/beta` deviennent deux
  tables du moteur, nommées comme le sont déjà les index plein texte d'une cellule. C'est la seule
  forme où un `ALTER` d'un projet ne touche pas l'autre. Une couche logique sur des tables
  partagées laisserait l'`ALTER` global, et avec lui le coût et les collisions du §5.
- **Ce que ça règle au passage :**
  - un `ADD COLUMN` ne réécrit plus que les lignes d'un projet (E1 réduit à sa part) ;
  - deux projets qui migrent en même temps ne visent plus la même table (E2 et E3 ne restent
    qu'au sein d'un projet) ;
  - le verrou exclusif d'un index HNSW ne sérialise plus que les écrivains d'un projet (E11).
- **Une méta par cellule.** `entity_config:{E}` devient `entity_config:{cellule}:{E}`. Le
  `migrate_entity` d'aujourd'hui compare dans la cellule, et `dans_la_cellule` bascule aussi les
  configurations, ce qu'il fait déjà pour les poignées plein texte.
- **Les relations restent dans leur cellule.** Une relation entre deux cellules n'aurait plus de
  table d'extrémités unique. Je propose de la refuser au chargement, en le nommant. Un renvoi
  d'un projet à l'autre passe par une propriété (l'uuid de l'autre, comme une clé étrangère), que
  les outils lisent sans saut. Si un cas réel le contredit (la mémoire longue relie-t-elle ses
  cellules ?), il revient à Lucie.
- **Une cellule neuve** reçoit les migrations de son gabarit dans l'ordre, sur des tables vides :
  c'est le rejeu du §1, cellule par cellule.
- **L'existant se migre lui-même en étendre puis contracter** :
  1. créer les tables de la cellule ;
  2. y copier les lignes marquées `_org`/`_project`, par lots, reprenable ;
  3. lire dans les tables neuves ;
  4. retirer les anciennes.
  Ce sont les bases de Lucie (`code-rag3db`, la mémoire longue), et chaque temps est une migration
  du journal.

### 11.2 La migration tirée de la différence de gabarits

**Ce que demande la décision :**
- **Un état appliqué**, par cellule : le gabarit tel que la dernière migration l'a laissé, rangé
  en méta avec le numéro de cette migration.
- **Un calcul de différence**, du gabarit appliqué au gabarit courant, en étapes déclaratives :
  - `ajouter_entite`, `ajouter_champ` (avec son défaut), `ajouter_relation` ;
  - `ajouter_signal` (bm25, vector, sparse) et `ajouter_modele` ;
  - `etendre` et `contracter` (§11.3).
- **Un fichier par migration**, `migrations/NNN_nom.json`, dans le dépôt de l'application, à côté
  du gabarit. Le développeur le relit et le commite comme du code. Il porte :
  - le numéro, le nom, l'auteur ;
  - les étapes ;
  - la somme du gabarit d'avant et celle du gabarit d'après.
- **Ce que la différence ne sait pas dire**, le développeur l'écrit dans le même fichier. Le
  calcul s'arrête dessus et le nomme : un renommage ressemble à un retrait plus un ajout, et c'est
  au développeur de dire que c'en est un.
- **Où il se calcule** : un outil ou une sous-commande (`migrations diff`), hors de l'hôte. Il ne
  touche pas la base, et se rejoue à l'identique.
- **Les refus, à l'application** :
  - une migration dont la somme « d'avant » n'est pas l'état appliqué : un autre est passé entre
    les deux, et le refus nomme sa migration ;
  - un numéro déjà pris ;
  - une migration déjà passée, ou passée avec une autre somme.
  Deux développeurs partis du même état se voient donc refuser le second, qui recalcule sa
  différence sur l'état neuf. C'est le « rebase » des migrations, et il est explicite.

### 11.3 Étendre puis contracter

**Ce que demande la décision.** Un retrait, une conversion de type ou un renommage deviennent
trois migrations, chacune du journal :

| temps | ce qu'elle fait | ce qui se passe entre-temps |
|---|---|---|
| **étendre** | ajoute le champ neuf (ou la relation neuve) et déclare au gabarit que l'ancien et le neuf sont **liés** : l'ancien se lit, les écritures vont aux deux | les outils écrivent les deux |
| **remplir** | une passe en fond, par lots, reprenable, qui écrit le neuf à partir de l'ancien (la conversion, ou la copie) | l'état d'index dit « remplissage n sur N », les lectures restent sur l'ancien |
| **contracter** | bascule les lectures sur le neuf, puis retire l'ancien (`ALTER … DROP`, déjà transactionnel au moteur) | refusée tant que le remplissage n'a pas fini, ou qu'un outil du gabarit nomme encore l'ancien |

- **Le lien entre l'ancien et le neuf** est une déclaration du gabarit, que les écritures du
  produit honorent (le put écrit les deux). Ce n'est pas une règle du moteur.
- **Le remplissage** est une passe du produit, sur le modèle de la dette d'embarquement : elle
  réclame des lots, écrit et marque. Elle hérite donc de la reprise après un arrêt brutal, et de
  l'état d'index qui la dit.
- **La contraction** relit les outils du gabarit, et le refus nomme celui qui lit encore
  l'ancien.
- **Pour le moteur**, `DROP` de propriété a eu deux défauts, fermés depuis (le point de reprise
  après un DROP, et le DROP d'une colonne déclarée avant la clé primaire). C'est le temps sans
  risque des trois.

### 11.4 Les lots, dans l'ordre

| # | lot | ce qu'il ferme | dépend de | témoin |
|---|---|---|---|---|
| M1 | **Une migration = une transaction, et son journal** : la table `_Migration` (par cellule dès le départ), le DDL, la méta et la ligne du journal dans un seul `BEGIN … COMMIT` ; plus d'`ALTER` avant la méta | E7, E8 (le journal) | le cœur pour E10 : sans l'annulation du stockage, une migration annulée laisse une colonne qu'un ADD suivant lit de travers. En attendant, une migration échouée force un point de reprise avant la suivante, et le dit | un arrêt brutal entre deux étapes, puis la réouverture : rien de la migration, et le journal sans elle (rouge d'abord) |
| M2 | **La différence de gabarits et ses refus** : l'état appliqué, le calcul, le fichier, les sommes, et les trois refus du §11.2 | les trois dernières lignes du §5 | M1 | deux développeurs partis du même état : le second refusé, en nommant le premier |
| M3 | **Le schéma par cellule** : les tables, la méta et les poignées par cellule ; le refus des relations entre cellules ; une cellule neuve par rejeu | la décision 1, et E1, E2, E3 et E11 entre projets | M1 (le journal porte la cellule) | deux cellules : un `ALTER` de l'une ne change ni les colonnes ni le temps d'écriture de l'autre |
| M4 | **L'application à chaud par l'hôte** : `migrate` sous le mutex du catalogue, la vue publiée, aucune fermeture de backend ; dans l'hôte, seules les sessions de la cellule migrée attendent | E6 | M1, M3 | deux sessions qui écrivent pendant une migration : leur attente au plus le temps de la transaction (le §1), et une session d'une autre cellule qui n'attend pas |
| M5 | **Les index après le COMMIT** : `ajouter_signal` et `ajouter_modele` posent la dette, l'index se bâtit en fond, l'état d'index dit « en construction » | E4 côté produit, R1 pour les migrations | M4 | une migration qui rend un champ cherchable : elle se valide sans `CREATE_VECTOR_INDEX` dans sa transaction, et une recherche pendant le bâti répond en le disant |
| M6 | **Étendre puis contracter** : le lien au gabarit, la passe de remplissage, la contraction et ses refus | la décision 3 | M2, M5 (la passe en fond) | un renommage en trois migrations, avec un arrêt brutal pendant le remplissage : rien de perdu, la contraction refusée avant la fin |
| M7 | **L'existant vers les cellules** : les bases de Lucie (`code-rag3db`, la mémoire longue), migrées par M6 | la décision 1 sur les vraies bases | M3, M6 | la base copiée (reflink) migrée, puis les comptes et les réponses des outils comparés avant et après |

**Pourquoi cet ordre :**
- M1 d'abord : sans transaction ni journal, rien ne tient.
- M3 avant M4 : une application à chaud qui ne retient que la cellule migrée est presque gratuite
  une fois les tables séparées. Sur des tables communes, elle retiendrait toute la base, et il
  faudrait la refaire.
- M5 après M4 : c'est l'hôte qui lance la passe en fond.
- M6 et M7 en dernier : ils s'appuient sur tout le reste.

**Ce qui reste au moteur**, dans l'ordre du cœur (§8) :
1. E10 avant M1 si possible ;
2. E2 et E3 pour deux processus ;
3. E1 (l'ADD paresseux) pour les grandes cellules.

Le scénario du banc mesure E1, E10 et E11 sur une seule table : il reste la référence de chaque
lot moteur.

**Une question nouvelle, pour Lucie** : les relations entre deux cellules, refusées comme je le
propose au §11.1, ou permises (alors leur table d'extrémités doit savoir viser plusieurs tables) ?
