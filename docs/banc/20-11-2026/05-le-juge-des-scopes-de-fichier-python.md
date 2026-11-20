# Le juge des scopes de fichier Python

20 novembre 2026, de 13 h 32 à 14 h 12, à luciepc, à la demande de codeparsers et au mot de c7. Le lot
`codeparsers/python-file-scopes-export-star-2` (`7ce7c9a4b5`, règles d'analyse à 12, pointeur codeparsers `15d123d`)
donne à tout fichier Python ses scopes de fichier (`module`, `file_scope_NN`) : le code hors fonctions et classes. Un
premier niveau de plus de 80 caractères devient candidat à la recherche. `export * from` réexporte les définitions de
premier niveau (côté arêtes seulement).

**Verdict : vert.** Le lot tient le critère contre master, ne perd aucune réponse, et rend trouvable un premier niveau
que master ne trouvait pas.

## Le dispositif

La recette du juge des déclarations courtes (page `04-…` du même jour) : la copie migrée de code-principal de G, une
copie reflink par côté, master à la base de fusion du lot (`0c4facb61d`) et le lot, chacun avec son binaire et son hôte
jetable, l'index de la même source (l'arbre de Lucie à luciepc, `284cc00c0b`). Sorties à luciepc :
`~/.cache/rag3weaver-build/banc-parite/sorties/codeparsers-export-star-20261120/`.

## Le lot contre master

- **Le critère de G** : recouvrement **99 %** (une seule requête, « test e2e », à 9/10), même premier **10/10**, usages
  identiques (acquireLock 5, FileSource 99, lockRowForWrite 3, Catalog 369).
- **Mes 30 cas** : 0 premier changé.
- **Les noms courts** (10 variables et 10 constantes tirées de l'arbre) : rangs identiques des deux côtés.
- **Les quatre jumeaux de mtga** : aux mêmes rangs à une place près (23 à 26 contre 24 à 27).

## L'analyse du lot a bien été prise

Le journal d'index compte le même nombre de conteneurs écartés des deux côtés (7 645), ce qui faisait douter. La lecture
par Cypher, sur une copie de chaque base indexée, le lève :

| | master | lot |
|---|---|---|
| `file_scope_*` dans les `.py` | 3 | **937** |
| `File.analyse` de `benchmark/click/load.py` | `9a9c50c7…` | `96849ff9…` |

Les scopes de fichier sont **les trous entre les déclarations**, pas le script entier. `benchmark/click/load.py` donne
`file_scope_01` (lignes 1-6, les imports, 65 caractères), `_02` (11-12, `con.execute(…)` puis le `COPY hits`, 100) et
`_03` (14-15, 18). Les lignes `db = …`, `start = …` restent des `variable` à part.

## Les premiers niveaux par leur sens

Un premier jeu décrivait chaque script entier : aucun ne remontait, ni sur master ni avec le lot. La requête était mal
posée. Le second jeu vise **un fragment par requête**, compté au premier résultat qui commence dans sa plage de lignes :

| fragment | requête | master | lot |
|---|---|---|---|
| `load.py` 11-12 | « exécuter create.cypher puis copier hits.csv dans la table hits » | absent | **1** |
| `plan_wildcards.py` 25-29, le bloc `__main__` | « lire les demandes d'un fichier json, lancer le plan avec asyncio et écrire le résultat en json » | 1 | 1 |
| `finish_catalog_checks.py` 8-10, la boucle | « pour chaque vérification lancer le script avec python et écrire sa sortie dans un journal » | absent | absent |
| `prepare_engine_collection.py` 9-16 | « écrire le schéma des cartes, la collection possédée et le manifeste magic_collection du moteur » | absent | absent (le fichier au rang 6, par un autre fragment) |

- **Le lot apporte ce qu'il promet** : le fragment de `load.py`, que master ne trouvait pas, sort premier.
- **Il ne pousse dehors aucune bonne réponse** : pour les quatre requêtes, le reste du top 3 est identique des deux côtés.
- Le bloc `__main__` de `plan_wildcards.py` était déjà trouvé sur master, par la variable `result` de la ligne 27.
- Deux requêtes ne trouvent rien des deux côtés : elles sont trop éloignées du texte de leur fragment. Elles ne disent
  rien contre le lot.
