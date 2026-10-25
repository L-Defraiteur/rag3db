# Une définition C++ se lie à la déclaration de son en-tête, pas à son nom de fichier

- **État** : en cours — la forme exacte est écrite sur `codeparsers/includes` (5a368e2f5,
  `visibility = "declaration"`), sa mesure sur code-lourd tourne ; il se ferme avec elle, ou garde
  ses limites ci-dessous comme défauts connus.
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

## Ce qui a été mesuré (25 octobre 2026)

Deux réindexations de code-lourd par lot, clangd sur des arêtes de `src/` (que couvre
`compile_commands.json`) :

- **Le pas 1** (inclusion, entete, homonymes recomptés après le filtre) promouvait 6 089 arêtes
  par le nom, fausses à 80 % : les homonymes se comptent désormais avant le filtre.
- **Ce qu'il ôtait** sous `src/` (1 021, 300 jugées) : 263 bonnes pertes, 36 appels justes perdus,
  tous de la même forme, une classe éclatée en plusieurs `.cpp` dont l'en-tête porte un autre nom
  de base (`transformer.cpp` → `transform_ddl.cpp`).
- **La co-inclusion** (une source qui inclut un en-tête de la fermeture est visible) reprenait 78
  justes sur 81, mais aussi 143 fausses sur 169 : l'en-tête commun la rend presque toujours vraie.
  Retirée.

## La forme exacte, telle qu'écrite (pas 2)

Un définisseur est visible si une de ses déclarations est dans la fermeture des #include du
mentionneur, avec le même nom et le même conteneur. La déclaration est ce que codeparsers relève
déjà : une ligne du champ `declarations` d'une classe, d'un namespace, ou du scope de fichier pour
un prototype (depuis le ticket du 4 octobre). Elle n'est jamais un scope, donc jamais un
définisseur.

Ses limites, à mesurer avec elle :

- **Le conteneur se compare par le dernier segment de son nom** : deux classes `Transformer` de
  deux namespaces se confondent (comme pour HAS_PARENT aujourd'hui).
- **Les surcharges ne se distinguent pas** : aucune signature n'est comparée.
- **Une fonction déclarée derrière une macro** (`RAG3DB_C_API rag3db_state f(…);`) n'est pas
  relevée comme déclaration par tree-sitter : ni membre, ni visible par elle.
