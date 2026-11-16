# Un ADD COLUMN passe sous une transaction d'écriture ouverte sur la même table

- **État** : corrigé sur master en 101ff50a6 ; fermeture documentaire le 16 novembre après lecture des preuves historiques, sans nouvelle exécution
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

## Fermeture documentaire du 16 novembre

Le comportement demandé est livré par le verrou de schéma TABLE du lot
`cf62f6dbd..101ff50a6` (mécanisme `681b35b383`) : un écrivain tient sa table
en partagé jusqu’à sa fin ; ADD/DROP de colonne et DROP TABLE prennent
l’exclusif et attendent. Aucun nouveau correctif ni nouveau témoin ajouté ici.

Les tests sont déjà dans `test/transaction/alter_waits_for_writers_test.cpp`
et inscrits à `transaction_test` :
- `AnAddWaitsForTheOpenWriter` ;
- `ADropOfAColumnTheOpenWriterWroteWaits` ;
- `AnAddWithAnOpenWriterKeepsTheDataAcrossACheckpointAndAReopening`.
Le cas d’origine `StringRewriteAcrossAddColumn.AnOpenWriterCommitsAfterAnAdd`
a également été adapté au nouveau contrat.

[Extraits historiques avec chemin, numéro de ligne et SHA du fichier complet](../../coeur-cpp-hotfixs-tickets/16-11-2026/preuves/add-column/lecture-historique.json) :
- Rouge du témoin seul à `d8b60cc38` : ADD fini à1,148ms avant COMMIT405,270ms,
  check d’attente réellement en échec. Le journal porte aussi d’anciennes
  attentes de chaîne sans fin de ligne ; le rouge d’ordre suffit au constat.
- À `b7251cb77`, les trois cas ci-dessus passent dans le journal transaction
  entier : ADD433ms, relecture/réouverture340ms ; contrôleur code0.
  Bilan historique387joués,385passés et2outils sautés (WalReader et probe
  de réouverture), pas385tests inventés aujourd’hui.
- `git diff b7251cb77 101ff50a6 -- src test/transaction/alter_waits_for_writers_test.cpp`
  est vide : les chemins concernés sont identiques à la tête atterrie.
- Le même contrôleur contient un rouge de concurrence known_red code8 ;
  il n’est ni effacé ni requalifié. Cette fermeture concerne le témoin
  d’ADD/d’attente documenté, pas une déclaration de passe globale sans rouge.

Limite du témoin existant : synchronisation par300/400ms de sommeil et
comparaison d’horodatages, pas preuve d’une barrière déterministe interne.
La fermeture s’appuie sur le mécanisme livré et les tests existants vus
rouges/verts dans leurs journaux, sans prétendre avoir joué une passe nouvelle.
Le contrôle stable `Schema changed, retry:` des écrivains arrivés pendant
l’ALTER appartient au même lot ; aucune modification de verrou dans ce travail.
