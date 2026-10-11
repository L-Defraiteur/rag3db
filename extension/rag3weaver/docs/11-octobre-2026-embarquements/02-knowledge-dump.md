# Session embarquements — ce que la journée a appris (11 octobre 2026)

## Le chaînage

- Un graphe de formes (`rag3weaver_ir::Plan`) se vérifie **sans dialecte**
  (`Plan::check`), se traduit **entier** avant d'exécuter (un refus P3 nomme le
  nœud et rien ne part), puis se regroupe : un nœud rejoint le groupe de son
  amont si l'amont en est le dernier, n'a que lui pour suivant, n'est pas une
  sortie, que leurs paramètres ne se recouvrent pas et que le dialecte sait les
  fondre (`SchemaDialect::fuse`, `None` par défaut).
- Une arête passe les valeurs **distinctes et non nulles** de sa colonne. Chaque
  fusion dit cette règle dans sa langue : `WITH DISTINCT … WHERE … IS NOT NULL`
  en Cypher, `ARRAY(SELECT DISTINCT … WHERE … IS NOT NULL)` en SQL. Sans elle, une
  fusion rendrait une ligne par chemin et différerait du repli.
- **Cypher** : le second `WITH` (qui ne garde que l'alias) fait sortir de portée
  `m`, `d`, `r`, `u` ; l'aval les redéclare comme s'il était seul, et les
  prédicats compilés sur l'alias `m` restent justes. Un amont ordonné ou borné ne
  se fond pas (l'`ORDER BY` d'un `WITH` ne voit plus ses variables).
- **PostgreSQL** : une CTE garde ses lignes, ordre et limite compris ; une chaîne
  fondue devient le corps de la CTE suivante (la `__chain` intérieure ne se voit
  que dans son corps). Une écriture ne se fond jamais en amont : toutes les
  parties d'une instruction voient le même instantané.
- `PlanOptions::split` force le repli : c'est l'oracle de toute fusion. Chaque
  scénario d'`e2e_chainage` joue fondu puis découpé et compare ses sorties ; le
  même corps tourne sur PostgreSQL depuis `e2e_postgres`.
- Un catalogue PostgreSQL du banc écrit **par lot** : un scénario qui lie doit
  vider la file (`drain`) avant de lire, sinon les liens n'existent pas.

## PostgreSQL et le pool

- Un réglage de session (`SET`) n'est tenu que par **une** session du pool
  deadpool. Un réglage qui doit valoir pour une requête part avec elle :
  `DbConnection::execute_with_local_settings`, `set_config(…, true)` dans la même
  transaction, sur la session épinglée de l'appelant s'il en a une, sinon sur
  une session du pool avec sa propre transaction. L'épinglage de `begin` est
  partagé par la connexion : deux recherches concurrentes se le seraient disputé.
- La file de deadpool est FIFO : des requêtes successives tournent sur les
  sessions ouvertes ; quelques fils concurrents d'abord ouvrent plusieurs
  sessions, et un témoin de réglage par session voit alors le défaut (7 sur 24).

## Façons de travailler

- Le nom du dialecte PostgreSQL dans les refus est `postgresql`.
- Un témoin rouge d'abord : un bouchon par variable d'environnement, retiré
  avant le commit, vérifié par `grep -c BOUCHON`.
- Avant tout hash qui touche `tests/` ou les features : `scripts/check_features.sh`.
  Un vert n'est pas vide quand la sortie montre les tests neufs.
- Un ticket fermé : `git mv` vers `closed/` **et** sa ligne déplacée au README, les
  comptes des sections à jour, dans le même commit.

## Les éventails et la mesure (après-midi)

- Le relevé des vrais appelants a montré que les chaînes pures sont rares : le coût
  d'un outil vient des **éventails** (R sauts, R degrés sur la même entrée) et du Rust
  voulu entre les niveaux (vus, budget, filtre sur le degré).
- Un éventail se dit `UNION ALL` avec une colonne de rang en tête. Cypher ne partage
  pas de préfixe entre les branches (l'amont est relu par branche), SQL partage sa CTE.
  Les branches doivent avoir la même forme de sortie (types des colonnes).
- **Mesurer avant de choisir.** Moins d'allers-retours n'est pas moins de temps : en
  local (rag3db dans le processus, PostgreSQL sur la boucle), la fusion des éventails
  ne gagne rien et coûte 1 à 20 %. D'où `DialectCapabilities::fanout` faux par défaut,
  levé par un montage distant (`Catalog::set_fanout`).
- Une mesure avant/après se prend sur un corpus que le lot ne touche pas
  (`src/daemon` pour impact), sur la même base de code, sous `poste mesure`, avec une
  empreinte des lignes **triées** : le rendu d'impact change d'ordre d'un processus à
  l'autre (ticket).
- Une « redondance » n'en est pas une tant que le rendu n'est pas prouvé identique :
  les sauts directs de `usages_of` comptent les arêtes devinées que `saut` écarte.
- Un message de commit doit rester vrai au moment de la fusion : deux commits non
  fusionnés se réunissent en un seul si le premier annonce un gain que le second retire.

## Le Cypher brut de l'indexation (fin d'après-midi)

- Un relevé avant tout lot : fichier, ligne, la forme qui remplace, ou la forme qui
  manque. Sept endroits, quatre lots, deux prédicats et une forme de plus, deux
  crochets : chaque manque comblé une fois, pour les deux dialectes.
- **Le témoin d'une forme nouvelle est le repli qu'elle remplace**, sur le même
  scénario : `Write::Detach` contre Select → Hop → Unlink (les paires relevées
  avant, « exactement toutes les autres » après).
- **Un réglage du moteur est un crochet du dialecte, pas une forme** :
  `copy_session_setup`, `engine_setting` (`None` hors rag3db, un nom non identifiant
  ne s'envoie pas).
- `e2e_code` indexe du code vivant : son relevé d'avant se prend sur **la même base**
  que l'après. Quand un rebase fait grandir le corpus, refaire le relevé d'avant, lot
  mis de côté par un patch. Et ne jamais éditer une source pendant qu'un relevé
  compile : il prendrait l'édition.
- La parité entre moteurs se prouve par les empreintes des lignes rendues : impact sur
  `src/daemon` rend les mêmes empreintes sur rag3db et sur PostgreSQL ; le voisinage de
  `grep`, les mêmes lignes.
- Une colonne lue sur une table qui ne la déclare pas : `Column::Null`, pas une table
  fixe — rag3db échoue sur un champ absent d'une table nommée, et l'accepte d'une
  table quelconque.
- `poste push` : `pousser.sh` rebase sans rejouer quand seuls des `.md` sont arrivés,
  et rejoue sur le moteur ancien quand on le lui dit (`MOTEUR_ANCIEN=1`), en l'écrivant.
