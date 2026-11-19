# Le premier passage de l'écrivain sur une base migrée remplit le tampon du moteur

- **État** : ouvert (19 novembre 2026), relevé pendant la bascule des bases de code vers les versions
- **Famille** : produit (l'écrivain de rag3weaver)
- **Gravité** : bloque la bascule des trois bases de code ; aucune perte, la base est revenue sur son instantané
- **Atteignable en service** : oui, au premier index de toute base passée par 1b puis les générations
- **Touche rag3weaver** : oui

## Ce qui s'est passé

Base : code-rag3db-depot (portable), 3,6 Go, 8 675 fichiers retenus. Lib 6e2476d41, hôte posé à 17:31.

1. 1b (`rag3weaver-backend migration versions`) en 770 s, puis les générations en 3 s, sont verts. La
   réouverture par l'hôte se fait sans avertissement.
2. L'outil `index` (`confirm=true`) lance le repassage. Le journal dit « première synchronisation par l'écrivain
   (version de l'analyse changée) : 6 439 fichiers à repasser ». Puis :

   > indexation échouée : recopier les arêtes DEFINED_IN : db error: must reopen: Query execution failed:
   > Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be
   > freed! — le tampon du moteur est plein (8.0 Gio, règle du produit …)

   Les libérations des réclamations File_Chunk, Library_Chunk et ScopeContent_Chunk échouent ensuite de la même
   façon, et le backend reste à rouvrir.
3. Seconde tentative avec `"buffer_pool": 25769803776` (24 Gio) au manifeste, après réouverture : **même échec, à
   la même étape**, « tampon plein (24.0 Gio, manifeste (buffer_pool)) ». Ce n'est donc pas une question de
   taille : la recopie des arêtes DEFINED_IN tient en une requête, ou en une transaction, dont la mémoire grandit
   avec la base.
4. Retour : l'hôte est arrêté à froid, puis RETOUR.sh de l'instantané `avant-versions-20261119` est joué (VERT,
   18:11). Le banc search_code (10 requêtes, top 10) et `usages` (4 noms) sont identiques à ceux d'avant.

Journaux au portable : `~/.cache/rag3weaver/commandes/index/1795107858371-1251835-0.out` (8 Gio) et
`1795108143224-1251835-1.out` (24 Gio) ; heures et sorties dans
`~/.cache/rag3weaver-build/notes-G/bascule-versions-code-rag3db/`.

## Le trou de la répétition

La répétition sur copie (arbre principal, puis G sur le binaire du soir) jouait 1b et les générations, puis une
ouverture. Elle ne jouait **pas** le premier index de l'écrivain. Toute nouvelle tentative de bascule passe
d'abord par une répétition qui joue aussi le repassage complet, sur une copie de la vraie base, sous le tampon
par défaut.

## Pour le correctif

Un témoin rouge d'abord : une base migrée (1b et générations), assez d'arêtes DEFINED_IN pour dépasser un
tampon réduit (`RAG3DB_BUFFER_POOL_SIZE` petit), puis le premier index de l'écrivain, qui doit finir. Ensuite,
une recopie par lots bornés. Pistes : `recopier les arêtes` dans le premier passage de l'écrivain, et la forme
de sa requête, à lire par son plan (EXPLAIN).
