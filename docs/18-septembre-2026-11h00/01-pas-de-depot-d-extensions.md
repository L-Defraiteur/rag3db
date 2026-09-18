# Pas de dépôt d'extensions : le refus, et ce que la liaison statique demande

**18 septembre 2026.** Le banc du 7 septembre avait mis au jour un défaut de
produit : le renommage avait fait pointer le dépôt d'extensions sur
`extension.rag3db.com`, un domaine inventé, et installer une extension ne
pouvait pas fonctionner. Lucie a tranché : **il n'y aura pas de dépôt**. Les
extensions seront liées statiquement dans le binaire.

## Ce qui est fait

La constante disparaît. `INSTALL x` sans `FROM`, et `UPDATE x`, passent par le
lieur qui résout le dépôt en deux temps — `RAG3DB_EXTENSION_REPO` si la variable
est posée, sinon un refus qui dit pourquoi et comment passer outre :

> No extension repository is configured: this build has no default repository,
> extensions are meant to be linked statically into the binary. To install x
> from a repository anyway, set the environment variable RAG3DB_EXTENSION_REPO
> to its URL, or name it in the statement: INSTALL x FROM '<url>'.

Le refus vit dans `bind_extension.cpp`, à côté des autres refus d'extension ;
l'analyseur reste pur. Les trois cas de `extension/` qui téléchargeaient — donc
échouaient hors ligne, et depuis le renommage — vérifient désormais ce refus et
passent : **4 sur 4, hors ligne**. Les deux portes sont éprouvées à côté : la
variable posée, ou `FROM` dans l'énoncé, mènent bien au téléchargement à
l'adresse donnée.

## Ce que la liaison statique demande en natif — noté, pas fait

**Le mécanisme existe déjà.** Il sert au WASM, et il n'est pas propre au WASM :

| pièce | où | ce qu'elle fait |
|---|---|---|
| `EXTENSION_STATIC_LINK_LIST` | `extension/extension_config.cmake:13` | la liste des extensions à lier ; aujourd'hui un exemple commenté (`fts`) |
| `add_static_link_extension(ext)` | même fichier | construit `rag3db_${ext}_static_extension` |
| l'édition de liens | `CMakeLists.txt:442-448` | lie chaque `rag3db_${ext}_static_extension` dans `rag3db` **et** `rag3db_shared` |
| `autoLoadLinkedExtensions` | `src/extension/extension_manager.cpp:93` | au démarrage, charge les extensions liées dans une transaction de reprise |

Pour un binaire natif qui embarque `vector` et `geo`, la première étape tient
donc en une ligne : `set(EXTENSION_STATIC_LINK_LIST vector geo)`. Ce qui reste à
vérifier avant de la poser, et que ce document ne fait pas :

1. **Le double.** Les extensions sont aujourd'hui construites comme
   bibliothèques dynamiques à part (`BUILD_EXTENSIONS`, par défaut `vector;geo`).
   Les lier en statique *et* les construire en dynamique produirait deux copies
   du même code ; il faut choisir, par extension, et probablement retirer la
   forme dynamique de celles qu'on lie.
2. **`LOAD EXTENSION` devient superflu** pour une extension liée, mais il
   existe dans les scripts et les tests. Que fait-il sur une extension déjà
   chargée au démarrage — rien, ou une erreur ? À décider et à tester.
3. **`INSTALL` / `UNINSTALL` sur une extension liée** n'ont plus de sens. Le
   refus ci-dessus couvre `INSTALL` ; `UNINSTALL` d'une extension liée dirait
   aujourd'hui « pas installée », ce qui est vrai mais trompeur.
4. **Les dépendances tierces des extensions** (le HNSW n'en a guère ; `geo`
   apporte la géométrie) entrent dans la bibliothèque du cœur. Taille du binaire
   et symboles exportés à mesurer.
5. **Les tests des extensions** compilent avec `__STATIC_LINK_EXTENSION_TEST__`
   quand `BUILD_EXTENSION_TESTS` est posé : ce chemin existe, il est à exercer.
6. **rag3weaver se lie au `.a` du cœur** (`build/…/src/librag3db.a`) plus les
   bibliothèques tierces ; une extension liée en statique dans le cœur y entre
   automatiquement — c'est précisément l'intérêt — mais `build.rs` liste les
   bibliothèques tierces à la main (`tools/rust_api/build.rs:112-131`) et devra
   apprendre celles de `geo`.

Rien de tout cela n'est long ; mais chaque point est une décision ou une
mesure, et aucun n'a été pris ce matin.
