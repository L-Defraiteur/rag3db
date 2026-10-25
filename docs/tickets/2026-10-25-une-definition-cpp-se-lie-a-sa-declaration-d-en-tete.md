# Une définition C++ se lie à la déclaration de son en-tête, pas à son nom de fichier

- **État** : ouvert — la règle approchée (`visibility = "entete"`) est posée par le lot INCLUDES
  (branche `codeparsers/includes`) ; la forme exacte attend la mesure de l'oracle.
- **Gravité** : réponse fausse (une arête vers le mauvais homonyme, ou une abstention à tort).
- **Atteignable en service** : oui, dès que la visibilité par les #include est sur master.
- **Touche rag3weaver** : oui (`src/code.rs`, la résolution) et codeparsers (les déclarations).

## Ce que c'est

Un appel C++ voit une fonction par l'en-tête qui la **déclare** ; sa définition vit dans un
`.cpp` que l'appelant n'inclut jamais. La visibilité par les #include (25 octobre 2026) le
rattrape par une approximation : un `.c/.cc/.cpp/.cxx` est candidat si un en-tête de **même nom de
base** est dans la fermeture des includes du mentionneur (`node_table.cpp` ↔ `node_table.h`).
L'arête porte la marque `visibility = "entete"`, pour que l'oracle la mesure à part (condition
de c7, 25 octobre). Une définition dans un fichier d'un autre nom (`table_impl.cpp` pour
`table.h`), ou un `.cpp` homonyme d'un autre dossier, est manquée ou admise à tort.

## La forme exacte

La déclaration de l'en-tête devient un scope (ou un nœud) que la définition **rejoint** : DEFINES
de la définition liée à sa déclaration, par nom qualifié et signature. La visibilité suit alors
la déclaration, qui est dans la fermeture, et non le nom du fichier de la définition.

## Le témoin

`tests/e2e_usages.rs`, `un_definisseur_cpp_se_voit_par_ses_includes` : le cas `compute` (déclaré
par `z/calc.h`, défini dans `z/calc.cpp`) passe par la marque « entete ». Il n'y a pas de témoin
rouge de la forme exacte : il faudrait un `table_impl.cpp` qui définit ce que `table.h` déclare.

## Ce qu'il faut pour le fermer

1. Mesurer par l'oracle (`scripts/oracle_lsp.py cpp`) les arêtes « entete » à part : si elles
   sont aussi fausses que « nom », la règle approchée part.
2. Sinon, la forme exacte : la déclaration en scope, la définition liée, la visibilité par elle ;
   puis la règle par nom de base part.
