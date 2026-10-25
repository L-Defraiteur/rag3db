# Nos arêtes jugées par un serveur de langage

*codeparsers, 25 octobre 2026, à la demande de l'orchestration (Lucie : « un serveur par langage, le
même juge »). Script : `scripts/oracle_lsp.py`. Base jugée : code-lourd, et ses réindexations de la
mesure d'INCLUDES, sur luciepc ; arbre a72acfeef. Cette page a été réécrite le soir même : le
premier juge était faux (voir « Deux juges faux, et pourquoi »), et seuls les chiffres du juge strict
y restent.*

## La méthode

Pour un langage, on tire au hasard (graine 25) 300 arêtes CONSUMES dont l'appelant est de ce
langage. Pour chacune, on demande au serveur la définition du nom à la ligne de la mention
(`textDocument/definition`, puis `declaration`).

Le **juge strict** (`ORACLE_STRICT=1`) ne déduit rien :

- le fichier de la définition que vise l'arête est ouvert aussi, pour que le serveur la connaisse ;
- **juste** : une position rendue tombe dans la cible même (son fichier, ses lignes) ;
- **décl** : le serveur pointe la mention elle-même, une locale ou un paramètre de la ligne. L'arête
  visait un homonyme sans rapport : c'est un faux ;
- **en-tête seul** : le serveur ne rend qu'une position d'en-tête, même la cible ouverte. Quand la
  cible est une source, l'index d'arrière-plan de clangd départage (`ORACLE_BG_INDEX=1`, qui refuse
  de juger s'il n'a pas fini). Sur les 196 cas départagés ce soir, tous étaient une autre entité que
  notre cible (`numRows`, le champ de `wal_record.h`, contre une variable de test ; `size`,
  `std::vector::size`) : des faux ;
- **faux** : une autre définition ;
- **muet** : rien.

Les verdicts se ventilent par **marque de résolution** (`resolution` : fichier, import, type, nom)
et, en C/C++, par **visibilité** (`visibility` : inclusion, entete, declaration).

## C++

La référence, 300 arêtes CONSUMES du moteur (`src/`, que couvre `compile_commands.json`) :

| marque | juste | faux | décl | en-tête seul (cible source / en-tête) | muet |
|---|---|---|---|---|---|
| type | 110 | 4 | 2 | 1 / 1 | 13 |
| fichier | 77 | 9 | 7 | 4 / 8 | 16 |
| nom | 31 | 1 | 3 | 1 / 7 | 5 |
| **total** | **218** | **14** | **12** | **6 / 16** | **34** |

La marque « nom » reste la moins sûre ; « type » est juste à 85 % et presque jamais faux.

## La mesure d'INCLUDES, jugée par lui

La visibilité par les #include (la règle du langage : un définisseur C/C++ n'est candidat que s'il
est visible du mentionneur). Deux réindexations de code-lourd, sur 300 arêtes de `src/` chaque fois.
« faux » compte ici les « en-tête seul » à cible source départagés.

| ensemble | juste | faux | décl | en-tête seul, cible en-tête | muet |
|---|---|---|---|---|---|
| marque « declaration » (la forme exacte) | 299 | 1 | 0 | 0 | 0 |
| marque « entete » (le .cpp de même nom de base) | 283 | 6 | 2 | 0 | 9 |
| ôtées par la forme exacte | 104 | 72 | 103 | 20 | 1 |
| ôtées par la seule inclusion (pas 1) | 187 | 50 | 49 | 13 | 1 |
| reprises de la co-inclusion | 236 | 44 | 20 | 0 | 0 |
| promotions par les homonymes recomptés | 76 | ≥ 179 | 33 | (52) | 4 |

Ce qu'on en lit :

- **La forme exacte est nécessaire.** Sans elle, la règle ôtait 187 justes sur 300, une classe
  éclatée en plusieurs `.cpp` dont l'en-tête porte un autre nom de base. Elle en reprend 140 des 142
  appels, sans réadmettre une seule locale.
- **Ce qu'elle ôte encore** est à 58 % une bonne perte (72 faux, 103 locales). Des 104 justes, 94
  sont des liens déclaration → définition de l'API C (`rag3db.h`, derrière `RAG3DB_C_API`), pas des
  usages : une perte nommée, à reprendre par une arête DECLARES. Restent 10 vrais usages perdus.
- **La co-inclusion** (une source qui inclut un en-tête de la fermeture est visible) réadmettait 64
  mauvaises sur 300 : retirée.
- **Recompter les homonymes après le filtre** promouvait le mauvais homonyme d'un même en-tête (la
  mention du membre d'enum `PhysicalTypeID::UINT32` reliée à la fonction `LogicalType::UINT32()`) :
  au moins 212 mauvaises sur 300. Les homonymes se comptent avant le filtre.

## Deux juges faux, et pourquoi

1. **Le premier juge comparait des chemins de deux arbres.** `compile_commands.json` venait de
   `rag3db-lourd/build` ; clangd en suit les `-I` et rend les en-têtes de `rag3db-lourd`, pas de la
   copie jugée, au même commit. Toute réponse dans un en-tête passait pour fausse : la co-inclusion
   et le recomptage ont d'abord été jugés sur ce biais. `ORACLE_ARBRE_CC` ramène ces chemins.
2. **Il jugeait une règle par elle-même.** Il disait juste une déclaration d'en-tête « dans la
   classe parente », c'est-à-dire par la règle même que la marque « declaration » applique. Le juge
   strict ouvre la cible et ne compare que la cible (relecture de l'arbre principal). Avec la cible
   ouverte, clangd rend la définition là où il ne rendait que la déclaration : son index dynamique
   ne connaît que les fichiers ouverts.

## Rust, Python, TypeScript : à rejuger

Les premiers chiffres de ces trois langages sont **retirés**. L'oracle ne déclarait pas
`window.workDoneProgress` : aucun serveur ne lui envoyait de `$/progress`, et son attente de
l'indexation (rust-analyzer, pyright, typescript-language-server) ne voyait rien venir ; chaque
« indexation finie en 30 s » était un calme pris pour une fin. Ils se rejoueront avec l'oracle
corrigé (la capacité déclarée, un fichier ouvert avant d'attendre, une indexation annoncée exigée).

## Deux trouvailles en chemin

- **TypeScript 7 n'a plus de `tsserver.js`** (le portage natif) ; typescript-language-server
  refuse alors de s'initialiser. L'oracle prend un TypeScript 5 par `ORACLE_TSSERVER`, et s'arrête
  si un serveur refuse son `initialize`.
- **Le JavaScript de l'IDE est invisible à tout serveur de langage** : inline dans
  `src/ide/page.html` et `src/atelier/page.html`. C'est un argument pour le sortir en fichiers ; ce
  choix revient à everything declarative.

## Rejouer

```
ORACLE_STRICT=1 [ORACLE_ARBRE_CC=<arbre du compile_commands>] [ORACLE_BG_INDEX=1 ORACLE_PLAFOND=5400] \
  oracle_lsp.py <langage> <arêtes> <scopes> <racine jugée> <racine du projet> 300 <verdicts.tsv> [compile_commands]
```

Les arêtes et les scopes viennent de la sonde (`tests/sonde_aretes_par_relation.rs`,
`SONDE_REQUETE`). Pour TypeScript, `ORACLE_TSSERVER=<typescript 5>/lib`. L'index d'arrière-plan
s'écrit à côté du `compile_commands.json` : on lui en donne une copie dans un dossier à soi.
