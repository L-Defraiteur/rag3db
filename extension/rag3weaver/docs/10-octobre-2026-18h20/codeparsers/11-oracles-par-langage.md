# Nos arêtes jugées par un serveur de langage, langage par langage

*codeparsers, 25 octobre 2026, demandé par l'orchestration (Lucie : « un serveur par langage, le
même juge »). Script : `scripts/oracle_lsp.py`, qui remplace `oracle_clangd.py`. Base jugée : la
copie de code-lourd du 25 octobre (`a72acfeef`), sur luciepc. Ces chiffres servent de référence
avant le lot INCLUDES ; ils se rejoueront après lui.*

## La méthode

Pour chaque langage, 300 arêtes CONSUMES sont tirées au hasard (graine 25) parmi celles dont
l'appelant est de ce langage. Pour chacune, on demande au serveur de langage la définition du nom
à la ligne de la mention (`textDocument/definition`, puis `declaration`). Le serveur est le juge :

- **juste** : la position rendue tombe dans notre cible, ou dans un scope de même nom et de même
  parent, ou dans la classe parente, ou sur la ligne de déclaration d'une fonction libre ;
- **faux** : le serveur désigne autre chose ;
- **décl** : le serveur pointe la mention elle-même (une déclaration locale, un paramètre) ;
- **muet** : le serveur ne rend rien.

Les verdicts se ventilent par **marque de résolution** (`resolution` sur l'arête : fichier,
import, type, nom). La marque « nom » est celle d'une arête posée par le seul nom, sans import ni
type ; c'est elle qu'on surveille.

| langage | serveur | corpus |
|---|---|---|
| C++ | clangd 22 | `src/` du moteur, avec `compile_commands.json` |
| Rust | rust-analyzer 1.98 (toutes les features, sans build scripts ni macros procédurales) | `extension/rag3weaver` |
| TypeScript / JavaScript | typescript-language-server 6.0.2 sur TypeScript 5.9.3 | `tools/wasm`, `tools/nodejs_api` |
| Python | pyright | ce que code-lourd contient |
| Go | gopls 0.24 | aucun : code-lourd n'a pas de Go |
| C# | csharp-ls | en attente d'un corpus de Lucie (et de dotnet) |

## Les résultats

| langage | juste | faux | décl | muet |
|---|---|---|---|---|
| C++ | 226 | 25 | 13 | 36 |
| Python | 270 | 9 | 9 | 12 |
| TypeScript / JavaScript | 271 | 11 | 6 | 12 |
| Rust | 253 | 19 | 1 | 27 |

### Par marque

C++ :

| marque | juste | faux | décl | muet |
|---|---|---|---|---|
| type | 108 | 6 | 3 | 14 |
| fichier | 87 | 11 | 7 | 16 |
| nom | 31 | 8 | 3 | 6 |

Python :

| marque | juste | faux | décl | muet |
|---|---|---|---|---|
| fichier | 195 | 7 | 6 | 1 |
| import | 52 | 0 | 0 | 0 |
| nom | 18 | 2 | 3 | 10 |
| type | 5 | 0 | 0 | 1 |

TypeScript / JavaScript :

| marque | juste | faux | décl | muet |
|---|---|---|---|---|
| fichier | 255 | 11 | 0 | 4 |
| nom | 13 | 0 | 6 | 8 |
| type | 3 | 0 | 0 | 0 |

Rust :

| marque | juste | faux | décl | muet |
|---|---|---|---|---|
| fichier | 90 | 3 | 0 | 19 |
| import | 71 | 2 | 0 | 1 |
| nom | 49 | 14 | 0 | 4 |
| type | 43 | 0 | 1 | 3 |

## Ce qu'on en lit

1. **La marque « nom » est la moins sûre partout.** En C++, elle est fausse 8 fois sur 48. En
   Rust, 14 sur 67 : les trois quarts des faux du langage. En Python et en TypeScript, elle porte
   presque tous les muets. Les outils ont raison de dire leur marque.
2. **« import » et « type » sont sûrs** : « import » est juste 52 fois sur 52 en Python et 71
   sur 74 en Rust ; « type » n'a aucun faux en Rust (43 justes).
3. **Le C++ est le plus faible**, et ses faux se concentrent dans « fichier » et « nom ». C'est
   la cible d'INCLUDES : un définisseur invisible par les #include du mentionneur ne devrait plus
   être candidat. On rejouera la même graine après le lot, avec la marque « entete » à part.
4. **Rust est au niveau de Python et de TypeScript** (84 % justes, 6 % faux). Une première passe
   avait rendu 162 muets : l'oracle répondait `null` à la `workspace/configuration` de
   rust-analyzer, qui retombait sur ses défauts, et sans `cargo.features = "all"` chaque module
   sous `cfg(feature = "daemon")` restait hors du graphe des crates. Les 27 muets restants sont
   pour 18 d'entre eux une ligne de mention qui ne porte pas le nom (la ligne de déclaration de
   l'appelant : un type de sa signature écrit sur la ligne suivante).

## Trois trouvailles en chemin

- **TypeScript 7 n'a plus de `tsserver.js`.** Le paquet `typescript` installé sur luciepc est la
  7.0, le portage natif. typescript-language-server refuse alors de s'initialiser, et la première
  passe a rendu 300 muets sans rien dire. L'oracle prend un TypeScript 5 désigné par
  `ORACLE_TSSERVER`, et s'arrête désormais si un serveur refuse son `initialize`.
- **Un serveur qui indexe se lit par `$/progress`.** L'oracle attend que ses travaux annoncés
  soient finis (30 s de calme) avant d'envoyer l'échantillon, et répond à
  `workspace/configuration` par ses propres options : une réponse nulle les efface.
- **Le JavaScript de l'IDE est invisible à tout serveur de langage.** Les composants de l'IDE ne
  sont pas des fichiers `.tsx` : leur code est inline dans `src/ide/page.html` et
  `src/atelier/page.html`. Aucun serveur de langage ne lit un script dans du HTML, et notre
  indexeur ne peut pas être jugé dessus. C'est un argument pour sortir ce code en fichiers (les
  composants déclarés y poussent déjà) ; ce choix revient à everything declarative.

## Rejouer

```
oracle_lsp.py <langage> <arêtes> <scopes> <racine de l'arbre> <racine du projet> 300 <verdicts.tsv> [compile_commands]
```

Les arêtes et les scopes viennent de la sonde (`tests/sonde_aretes_par_relation.rs`,
`SONDE_LISTER`). Pour TypeScript, `ORACLE_TSSERVER=<typescript 5>/lib`.
