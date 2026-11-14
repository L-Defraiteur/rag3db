# Un ADD COLUMN passe sous une transaction d'écriture ouverte sur la même table

- **État** : ouvert (14 novembre 2026, hotfixs, relevé par le témoin du dictionnaire ; demandé par c7)
- **Famille** : moteur
- **Gravité** : à trancher. Aucune donnée fausse n'a été vue : l'écrivain valide, et ses lignes
  prennent le défaut de la colonne ajoutée. Mais une écriture en cours voit le schéma changer sous
  elle, sans attente ni refus.
- **Atteignable en service** : oui, dès qu'une migration (`ALTER TABLE … ADD`) tourne pendant qu'un
  autre client écrit dans un `BEGIN … COMMIT`. C'est le cas de `TwentyWriters.DdlEveryMinute`.
- **Touche rag3weaver** : par le moteur (ses migrations de schéma)
- **Pour** : cœur c++ (réattribution par c7)

## Le cas joué

`StringRewriteAcrossAddColumn.AnOpenWriterCommitsAfterAnAdd`, branche `chaine-au-point-de-reprise`
(test seul, au-dessus de master `267c25e80`), joué sous ASAN à luciepc le 14 novembre 2026 :

1. Table `Chunk(_uuid STRING PRIMARY KEY, _text STRING, marker STRING)`, 5 000 lignes par COPY,
   `auto_checkpoint=false`.
2. **Connexion A** : `BEGIN TRANSACTION`, puis `MERGE … SET` sur les 5 000 lignes, puis `CREATE` de
   500 lignes neuves.
3. **Connexion B**, dans un autre fil, pendant que A est ouverte :
   `ALTER TABLE Chunk ADD note STRING DEFAULT 'n-defaut'`.
4. 200 ms plus tard, **A** lit `CALL table_info('Chunk')`, puis fait `COMMIT`.

## Ce qu'on voit

- **L'ADD de B aboutit avant le COMMIT de A.** Il ne l'attend pas et n'est pas refusé.
- **A garde le catalogue du début de sa transaction** : `table_info` lui rend 3 colonnes, pas 4. Une
  requête de A qui nomme `note` échoue au liage (« Cannot find property note »), ce qui fait annuler
  toute sa transaction.
- **Le COMMIT de A réussit.** Après lui, les 5 000 lignes réécrites et les 500 lignes neuves de A ont
  toutes `note = 'n-defaut'`. Les chaînes de A sont justes après le point de reprise et après la
  réouverture, et ASAN ne signale rien.

## Ce qui reste à trancher

- **Le comportement voulu.** PostgreSQL fait prendre à `ALTER TABLE … ADD COLUMN` un verrou ACCESS
  EXCLUSIVE sur la table, qui attend la fin des transactions qui la tiennent. Le moteur, lui,
  laisse passer l'ADD et rattrape les lignes de l'écrivain au COMMIT. Il faut choisir : attendre,
  refuser l'un des deux, ou garder ce comportement en le documentant comme contrat.
- **Les cas que ce témoin ne couvre pas** : un `DROP` d'une colonne que l'écrivain ouvert vient
  d'écrire, un `ADD` puis un `DROP` pendant sa transaction, et un point de reprise automatique qui
  tombe entre l'ADD et son COMMIT. Le plantage du dictionnaire sous `DdlEveryMinute` vient peut-être
  de cette famille ; il n'est pas reproduit à ce jour.
