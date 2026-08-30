# Ce que la mesure dit

Avant d'implémenter le [cahier des charges](01-cahier-des-charges-tout-le-fichier-est-couvert.md),
répondre à ses §5 (« ce qui rendrait la chose fausse ») et au §4 du
[troisième document](03-comment-on-saura-que-ca-marche.md) (« à compter
**avant** de commencer »).

Sonde : `codeparsers/examples/couverture.rs`. Elle parse le corpus avec les
parseurs d'aujourd'hui, prend les scopes de premier niveau (`depth == 0`,
`parent == None`), et mesure trous, recouvrements, et — indépendamment — les
octets réellement sous un nœud `ERROR` de tree-sitter.

```
cargo run --example couverture -- <racine>...      # le tableau
DETAIL=1 cargo run --example couverture -- ...     # la 1re erreur de chaque fichier
FORCER=cpp cargo run --example couverture -- ...   # .c/.h avec la grammaire C++
```

Sur le dépôt entier : **1 min 57 s**, 3 918 fichiers parsés. Elle ne suit pas
les liens symboliques — `tools/rust_api/rag3db-src` pointe sur `../..`, c'est
la racine du dépôt, et le suivre boucle sans fin.

## 1. Les craintes du §5, vérifiées

| crainte | verdict |
|---|---|
| les scopes de premier niveau se recouvrent déjà | **presque pas.** 0 octet sur 170 fichiers Rust ; 1 334 octets sur 5 fichiers (`.js`, `.mjs`, `.h`) dans le corpus large. Réel, à corriger, mais pas un blocage. |
| les offsets à la granularité de la ligne cassent le pavage | **non.** Sur 798 fichiers, **aucun** scope de premier niveau ne commence en milieu de ligne. |
| l'encodage | 8 fichiers non-UTF8 dans les extensions, tous binaires (`.wasm`, `.node`, `.luce`, index tantivy). Le refus franc est le bon geste. |
| les fichiers binaires | 10 `.png`, 2 `.fast` — un octet nul dans les 8 premiers Kio les attrape tous. |

**Le pavage est donc déjà à 99,5 %.** Ce n'est pas le trou qui manque : c'est
le fait que ce qui reste ne se dit pas.

## 2. Ce qui manque vraiment : `ast_valid` ment

`validate_ast` est ceci, en entier :

```rust
pub fn validate_node(&self, node: SyntaxNode) -> bool { node.kind() != "ERROR" }
pub fn validate_ast(&self, root_node: SyntaxNode) -> bool { self.validate_node(root_node) }
```

La racine d'un arbre tree-sitter est `source_file` ou `translation_unit`,
**jamais** `ERROR`. Le champ est donc structurellement vrai.

La mesure le confirme, et l'écart est le chantier :

| corpus | fichiers | `root.has_error()` | `ast_valid == false` | octets sous `ERROR` |
|---|---:|---:|---:|---:|
| extensions (798 fichiers, 10,4 Mo) | 798 | **113** | **3** | **1 352 285 (13 %)** |

Treize pour cent du corpus n'est pas compris, cent treize fichiers le savent,
trois le disent.

## 3. Par langage — la lecture qu'on n'avait jamais eue

### 3.0 Le dépôt entier

3 918 fichiers, 40,7 Mo. C'est le tableau du §6 du cahier des charges, et il
est déjà lisible.

| ext | fich. | octets | pavage | recouvr. | arbre en erreur | `ast_valid` faux | scopes | oct. sous `ERROR` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `h` | 1 541 | 10 706 740 | 99,4 % | 371 | **1 470** | 64 | 8 181 | **2 725 816 (25 %)** |
| `cpp` | 1 208 | 10 041 851 | 95,3 % | 0 | 106 | 2 | 19 111 | 911 985 (9 %) |
| `rs` | 861 | 13 249 997 | 99,5 % | 0 | 9 | 0 | 29 415 | 354 (0 %) |
| `c` | 71 | 3 913 212 | 99,0 % | 0 | 60 | 1 | 2 591 | **2 767 575 (71 %)** |
| `py` | 95 | 649 647 | 90,1 % | 0 | 0 | 0 | 1 225 | 0 |
| `js` | 71 | 467 842 | 99,1 % | 1 124 | 0 | 0 | 1 804 | 0 |
| `hpp` | 47 | 1 293 835 | 99,5 % | 0 | 7 | 1 | 2 059 | 55 749 (4 %) |
| `mjs` | 16 | 58 062 | 98,3 % | 171 | 0 | 0 | 400 | 0 |
| `ts` | 4 | 40 027 | 96,3 % | 0 | 0 | 0 | 36 | 0 |
| `cc` | 3 | 165 631 | 98,5 % | 0 | 1 | 1 | 324 | **158 198 (96 %)** |
| `hxx` | 1 | 152 995 | 100,0 % | 0 | 1 | 0 | 413 | **151 577 (99 %)** |
| **total** | **3 918** | **40 739 839** | **98,2 %** | 1 666 | **1 654** | **69** | **65 559** | **6 771 254 (17 %)** |

**Dix-sept pour cent du dépôt n'est pas compris. 1 654 fichiers le savent,
69 le disent.** Et `.hxx` est la caricature du défaut : un fichier, 413 scopes
extraits, 100 % de pavage — et 99 % de ses octets sous un `ERROR`.

### 3.1 Le détail, sur les extensions

Corpus : `extension/{fts,vector,llm,lucivy,algo}`.

| ext | fich. | octets | pavage | recouvr. | arbre en erreur | `ast_valid` faux | scopes | oct. sous `ERROR` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `c` | 33 | 1 322 843 | 99,8 % | 0 | 30 | 0 | 1 666 | **1 300 546 (98 %)** |
| `h` | 78 | 126 814 | 98,7 % | 152 | **76** | 3 | 331 | **51 673 (41 %)** |
| `cpp` | 52 | 517 735 | 100,0 % | 0 | 5 | 0 | 896 | 56 (0 %) |
| `rs` | 605 | 8 112 467 | 99,5 % | 0 | 2 | 0 | 18 765 | 10 (0 %) |
| `py` | 5 | 48 847 | 86,0 % | 0 | 0 | 0 | 171 | 0 |
| `js` | 10 | 180 704 | 98,7 % | 1 124 | 0 | 0 | 333 | 0 |
| `mjs` | 12 | 48 989 | 98,2 % | 58 | 0 | 0 | 326 | 0 |
| `ts` | 3 | 28 853 | 99,2 % | 0 | 0 | 0 | 17 | 0 |

C'est le tableau du §6 du cahier des charges, et il est déjà lisible. Il dit
deux choses que personne ne savait.

### 3.2 `.h` est envoyé à la grammaire C, et ce sont des en-têtes C++

`EXTENSION_TO_LANGUAGE` (`parallel/project_parser.rs`) mappe `.h` sur
`SupportedLanguage::C`. Les en-têtes de nos extensions sont du C++ :
`namespace`, `std::string`, `::common`. D'où 76 fichiers sur 78 en erreur.

Avec la grammaire C++ sur les mêmes fichiers, aux deux échelles :

| | `.h` → C (aujourd'hui) | `.h` → C++ |
|---|---:|---:|
| **extensions** (78 fichiers) | | |
| fichiers en erreur | 76 / 78 | **32 / 78** |
| octets sous `ERROR` | 51 673 (41 %) | **0** |
| scopes extraits | 331 | **688** |
| **dépôt entier** (1 541 fichiers) | | |
| fichiers en erreur | 1 470 / 1 541 | **577 / 1 541** |
| octets sous `ERROR` | 2 725 816 (25 %) | **996 186 (9 %)** |
| scopes extraits | 8 181 | **22 192** |

**Les scopes des en-têtes sont multipliés par 2,7, et 1,7 Mo d'octets sortent
de l'erreur.** C'est un changement d'une ligne dans `EXTENSION_TO_LANGUAGE`,
mesurable, et indépendant du reste du chantier.

*(Les fichiers restants n'ont souvent que des nœuds `MISSING` de largeur nulle
— `has_error()` est vrai, aucun octet n'est perdu. Distinguer les deux est
exactement ce que la sonde apporte.)*

Et le même essai sur `.c` donne le contre-exemple qui justifie tout le §6.1 :
les fichiers en erreur passent de 60 à **62** — c'est pire — tandis que les
octets sous `ERROR` tombent de 2 767 575 (71 %) à **1 777 226 (45 %)** — c'est
un mégaoctet regagné. **Le booléen et les octets pointent en sens contraire.**
Un critère de réussite écrit sur le booléen aurait rejeté un changement qui
récupère un quart du corpus C.

### 3.3 `.c` : 98 % du corpus dans **un seul** nœud `ERROR`

Tous les `.c` sont les stemmers Snowball de `extension/fts/third_party`. Chacun
commence par :

```c
#ifdef __cplusplus
extern "C" {
#endif
extern int english_UTF_8_stem(struct SN_env* z);
#ifdef __cplusplus
}
#endif
```

L'accolade s'ouvre dans un `#ifdef` et se ferme dans un autre. tree-sitter veut
des conditionnelles syntaxiquement complètes : un `ERROR` avale le reste du
fichier. **La grammaire C++ n'y change rien** — vérifié, 1 300 546 octets dans
les deux cas.

Et pourtant : 1 666 scopes extraits, 99,8 % de pavage, zéro fichier signalé.
C'est *le* cas d'école du cahier des charges — un fichier qu'on ne comprend
pas à 98 % et qui compte comme un succès.

### 3.4 Nos quatre fichiers Rust « en erreur » s'appellent `raw`

`burn_device.rs:152`, `gcp_auth.rs:118`, `dataflow/mermaid.rs:253`,
`markdown_parser.rs:193` — tous `match Self::parse(&raw)` ou `&raw[1..]`.
tree-sitter-rust 0.23 lit `&raw` comme l'emprunt brut de Rust 2024 (`&raw
const`) et pose un `ERROR` de **4 octets**.

Le §2 du troisième document attend « Rust : 0 fichier en erreur ». Ce sera
faux, et pour une raison qui n'est pas la nôtre. D'où la conséquence à tirer :
**le critère doit porter sur les octets, pas sur le booléen.** Quatre octets
perdus et treize pour cent du corpus perdu ne peuvent pas produire la même
ligne verte.

## 4. Le volume : ce qui n'entre pas dans l'index

Sur le dépôt entier : **3 918 fichiers dans l'index, 3 067 hors index.** En
octets, 536 Mo dehors contre 41 Mo dedans — mais le chiffre ne veut rien dire
tel quel : il est dominé par 272 Mo de CSV de test, 57 Mo de Parquet, un
`.luce` de 67 Mo et 12 Mo de bases.

Ce qui entrerait honnêtement avec le §3.5 du cahier — du texte, écrit par
quelqu'un, qu'une question pourrait viser :

| | fichiers | octets |
|---|---:|---:|
| `.md` | 945 | 13 239 484 |
| `.txt` | 279 | 4 596 655 |
| `.test` | 573 | 2 937 764 |
| `.cypher` | 171 | 2 194 034 |
| `.yml` | 42 | 172 956 |
| `.java` | 30 | 159 673 |
| `.toml`, `.sh`, … | ~80 | ~60 000 |
| **total** | **≈ 2 120** | **≈ 23,4 Mo** |

**Le corpus grossit de moitié en octets et se met à compter deux fois plus de
fichiers.** 3 918 fichiers / 40,7 Mo aujourd'hui, ≈ 6 040 fichiers / 64 Mo
après. Ce n'est pas 2 %, ce n'est pas 200 % : c'est le genre de chiffre qu'il
fallait avoir avant de commencer, pas après.

Deux entrées de cette liste méritent leur propre décision, et ni l'une ni
l'autre n'est du « texte non rattaché » :

- **`.cypher`, 171 fichiers** — c'est le langage de requête de la base. Des
  requêtes nommées et cherchables valent mieux qu'un bloc de texte.
- **`.java`, 30 fichiers** — `tools/java_api`. Il n'y a pas de parseur Java,
  et ce n'est pas un défaut de couverture : c'est un langage absent.

## 5. Ce que le code fait déjà, et qu'il ne faut pas réécrire

### 5.1 Le pavage des trous existe

`base_scope_extraction_parser.rs:3023`, `extract_file_scopes` : après
extraction, elle parcourt les scopes triés, prend les intervalles de lignes
libres et en fait des scopes. C'est le §3.3 du cahier, déjà écrit.

Trois écarts, et ils expliquent les 0,5 % manquants :

1. **`has_meaningful_content` jette les trous** qui ne contiennent que des
   commentaires ou de la ponctuation (elle retire `//…` et `/*…*/` puis teste
   le vide). Un en-tête de licence, un bloc `///` de documentation entre deux
   fonctions : perdus, silencieusement.
2. **Le genre est `Module`** (`create_file_scope:3121`) — indiscernable d'un
   vrai module.
3. **`content` est `trim()`é** mais les lignes ne le sont pas : le scope
   déclare des octets qu'il ne contient pas.

Le chantier n'est donc pas « écrire le pavage » mais **« cesser de jeter, et
nommer ce qu'on garde »**.

### 5.2 Le parseur générique existe — et il est mort

`src/generic/generic_code_parser.rs`, 677 lignes. Il fait exactement ce à quoi
on pense : détection de style (`Curly` / `Indent` / `Mixed` / `Unknown`),
mots-clés de fonction et de classe (`fn`, `def`, `func`, `sub`, `defun`, …),
fin de scope par comptage d'accolades ou par désindentation
(`find_indent_based_end`), et — le point important — **un score de confiance
par scope** (0,9 pour un mot-clé reconnu, 0,3 pour un morceau deviné).

Il pave même les trous, avec la même structure que `extract_file_scopes` (et
le même défaut : il jette les trous de moins de `min_chunk_lines`, 3 par
défaut).

À côté de lui : `markdown_parser.rs` (1 057 lignes), `scss_parser.rs` (1 151),
`svelte_parser.rs`, `vue_parser.rs`, `css_parser.rs`.

**Rien de tout cela n'est appelé.** Deux raisons, empilées :

- `EXTENSION_TO_PARSER` (`non_code_project_parser.rs:18`) ne mappe **aucune**
  extension sur `NonCodeParserType::Generic` — la variante existe dans le
  `match`, rien ne l'atteint ;
- `rag3weaver` n'appelle jamais la chaîne non-code. Un `grep` de `src/` sur
  `codeparsers::` ne rend que `project_parser`, `relationship_resolution`,
  `scope_extraction::types` et `shell`.

Donc `.md`, `.css`, `.vue` sont hors index **alors qu'un parseur les attend**.

### 5.3 Ce que ça change pour le §3.5

Le cahier dit : fichier sans parseur → **un** scope `TexteNonRattache`
couvrant tout. C'est juste, et c'est le plancher.

Mais le générique donnerait mieux pour beaucoup de ces fichiers : un `.sh`, un
`.lua`, un `.toml` y rendent des scopes nommés. Avec une réserve, et elle est
dans l'esprit du dépôt : **ses scopes sont des conjectures**, et il le sait
déjà — `confidence` est là. Un scope deviné à 0,3 ne doit pas se présenter
comme une fonction extraite d'un arbre. C'est un troisième genre, pas une
variante des deux autres.

À trancher, donc, et pas en passant.

## 6. Ce que je changerais à la spécification

Trois points, tous issus de la mesure.

1. **Le critère de réussite est en octets.** « 0 fichier en erreur » est déjà
   faux pour Rust à cause de quatre variables nommées `raw`. Le seuil doit
   porter sur `octets_non_compris / octets`, et le booléen ne doit servir qu'à
   pointer où regarder.

2. **`arbre_en_erreur` doit venir de `root_node().has_error()`**, pas de
   `validate_ast`. Et il ne suffit pas : 32 en-têtes ont `has_error() == true`
   pour des nœuds `MISSING` de largeur nulle. Garder les deux champs du §3.4
   (`arbre_en_erreur` **et** `noeuds_erreur`/`octets_non_compris`) est le bon
   choix — la mesure montre pourquoi.

3. **Le mapping des extensions est un sujet à part, et il rapporte plus vite.**
   `.h` → C++ double les scopes des en-têtes pour une ligne. À faire avant, et
   séparément — sinon le gain du chantier de couverture sera mesuré contre un
   corpus artificiellement cassé.

Et une possibilité que la couverture ouvre, qu'il ne faut pas confondre avec
le chantier : une fois les octets d'erreur mesurables, **essayer deux
grammaires et garder celle qui perd le moins** devient décidable. C'est ce
qu'on vient de faire à la main pour `.h`. Ce n'est pas dans le cahier des
charges ; c'est ce qu'il rend possible.
