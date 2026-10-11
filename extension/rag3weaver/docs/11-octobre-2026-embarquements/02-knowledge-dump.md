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
