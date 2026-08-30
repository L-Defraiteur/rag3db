# Où on en est

**30 août 2026, milieu d'après-midi.** État de la session : ce qui a été fait,
ce qui a été trouvé, ce qui reste ouvert. Écrit pour qu'une autre session
reprenne sans redécouvrir.

## 1. Ce qui a été fait

Trois choses, dans cet ordre.

### Mesurer avant d'implémenter

Le [cahier des charges](../../codeparsers/docs/30-aout-2026-06h00/01-cahier-des-charges-tout-le-fichier-est-couvert.md)
posait des questions ouvertes à son §5, et son
[troisième document](../../codeparsers/docs/30-aout-2026-06h00/03-comment-on-saura-que-ca-marche.md) demandait de
compter **avant** de commencer. Personne n'avait répondu.

`codeparsers/examples/couverture.rs` y répond. Elle parse un corpus avec les
parseurs d'aujourd'hui, prend les scopes de premier niveau, et mesure trous,
recouvrements, et — indépendamment de ce que le parseur prétend — les octets
réellement sous un nœud `ERROR` de tree-sitter.

Le détail est dans [05-ce-que-la-mesure-dit](../../codeparsers/docs/30-aout-2026-06h00/05-ce-que-la-mesure-dit.md).
Commit `d89f9e973`.

### Sortir codeparsers dans son propre dépôt

https://github.com/L-Defraiteur/codeparsers — public, MIT, onze commits
d'historique conservés. Ajouté ici en sous-module, au même chemin. Commit
`a62b29452`.

### Corriger ce que la mesure a désigné

`.h` partait à la grammaire C. Corrigé dans les **deux** tables qui
divergeaient — celle du parseur de projet et celle du résolveur de relations.
Commit `91c3ef1` côté codeparsers, 90 tests au vert, et l'avant/après mesuré
sur le dépôt entier :

| | avant | après |
|---|---:|---:|
| fichiers avec arbre en erreur | 1 654 | **761** |
| octets sous `ERROR` | 6 771 254 (17 %) | **5 041 624 (12 %)** |
| scopes extraits | 65 559 | **79 600** |

Le pavage descend de 98,2 % à 97,8 % et les trous grossissent de 718 Ko à
899 Ko : c'est le signe attendu que plus de structure réelle a été trouvée, et
que ces trous sont exactement ce que le chantier de couverture doit paver.

### Donner à codeparsers ses documents

Trois documents décrivaient le parsage tout en vivant dans un autre dépôt que
le code qu'ils spécifient. Ils sont partis avec lui, avec un `00-index.md` côté
codeparsers et un `README.md` ici qui dit où. **Déplacés, pas copiés** : deux
exemplaires d'une spécification, c'est deux vérités qui divergent.

### Faire entrer le texte dans l'index

Un fichier sans grammaire était ignoré ; il produit désormais un `File`, **un**
`Scope` `texte_brut` couvrant tout, et l'arête qui les relie. Rien d'autre : ni
import à résoudre, ni référence à rapprocher, ni symbole — il ne passe donc pas
par le résolveur.

Côté codeparsers (`6c9e4b1`) : `ScopeInfoType::TexteBrut` et
`analyser_texte_brut`, plus les octets du fichier dans `ScopeFileAnalysis`. Des
**faits** ; la politique est chez le consommateur.

Côté rag3weaver (`40cca2f1f`) : `verdict(chemin, taille)` en un seul endroit,
en trois couches du moins cher au plus cher — la liste noire qui se décide sur
le nom sans ouvrir le fichier, la grammaire si elle existe, puis le seuil de
128 Kio.

| | fichiers | octets |
|---|---:|---:|
| code | 3 929 | 40,2 Mo |
| **texte brut** | **2 326** | **12,2 Mo** |
| écarté, avec sa raison | 700 | |

`e2e_code` : toutes les suites d'ingestion passent sans qu'un compteur ait eu
besoin d'être touché.

### Ce qui n'a pas été fait

Le chantier du cahier des charges lui-même — les deux autres genres de scope,
le pavage des trous, la couverture dérivée. Et la remesure de la pondération,
qui est maintenant **due** : voir le §3.

## 2. Ce que la mesure a trouvé

Sur 3 918 fichiers, 40,7 Mo.

**Les craintes du §5 étaient infondées.** Le pavage est déjà à 98,2 %, aucun
scope de premier niveau ne commence en milieu de ligne, les recouvrements
tiennent en 1 666 octets sur 8 fichiers. Le trou n'est pas le problème.

**Le problème est ailleurs, et il est plus gros.** `validate_ast` teste
`root.kind() != "ERROR"`, et la racine d'un arbre tree-sitter est
`source_file` : le champ `ast_valid` est structurellement vrai. Résultat :
**1 654 fichiers ont un arbre en erreur, 69 le déclarent, et 17 % des octets
du dépôt sont sous un `ERROR`.** Un `.hxx` sort 413 scopes et 100 % de pavage
avec 99 % de son texte non compris.

**Deux causes concrètes, indépendantes du chantier :**

- `.h` est envoyé à la grammaire C alors que ce sont des en-têtes C++. Avec la
  bonne grammaire : 1 470 fichiers en erreur → **577**, octets en erreur 25 %
  → **9 %**, scopes 8 181 → **22 192**. Une ligne dans
  `EXTENSION_TO_LANGUAGE`.
- Les `.c` de `third_party/snowball` ouvrent `extern "C" {` dans un `#ifdef
  __cplusplus` et le referment dans un autre. Un `ERROR` avale 98 % du
  fichier, et la grammaire C++ n'y change rien. C'est le cas d'école du cahier
  des charges.

**Et le contre-exemple qui change un critère de réussite.** En forçant `.c`
vers C++, les *fichiers* en erreur montent de 60 à 62 pendant que les *octets*
tombent de 71 % à 45 %. Le booléen et les octets pointent en sens contraire :
le critère du §2 du troisième document (« Rust : 0 fichier en erreur ») doit
se réécrire en octets. Il est d'ailleurs déjà faux — quatre de nos fichiers
Rust « échouent » parce qu'une variable s'appelle `raw` et que tree-sitter
0.23 y lit l'emprunt brut de Rust 2024. Quatre octets.

**Le volume.** ≈ 2 120 fichiers texte honnêtes sont hors index (945 `.md`,
573 `.test`, 279 `.txt`, 171 `.cypher`, 30 `.java`), ≈ 23,4 Mo. Le corpus
passerait de 3 918 à ≈ 6 040 fichiers.

**Et ce qui existe déjà et qu'on allait réécrire :** `extract_file_scopes`
(`base_scope_extraction_parser.rs:3023`) fait **déjà** le pavage du §3.3. Le
parseur générique (`src/generic/`, 677 lignes, accolades ou indentation, avec
un score de confiance par scope) fait déjà ce à quoi on pensait pour les
langages inconnus. Aucun des deux n'est atteignable : `EXTENSION_TO_PARSER` ne
mappe rien sur `Generic`, et `rag3weaver` n'appelle jamais la chaîne non-code.

## 3. Ce qui reste ouvert

Par ordre de rapport sur effort.

1. ~~**`.h` → C++.**~~ **Fait** (`91c3ef1`) : +14 041 scopes, 893 fichiers
   sortis de l'erreur. C'est la ligne de base contre laquelle le chantier de
   couverture doit désormais se mesurer.
2. ~~**Câbler la chaîne non-code.**~~ **Fait** (`40cca2f1f`), mais pas comme
   prévu : plutôt que d'appeler le parseur Markdown, un genre `texte_brut`
   commun à tous les fichiers sans grammaire. Un parseur par format aurait
   voulu dire un chemin d'ingestion par format ; un genre veut dire un seul
   mécanisme, et le Markdown reste disponible pour plus tard si on veut des
   scopes par section.

   **Et le chiffre annoncé était faux** : 2 326 fichiers, pas 945 ; 12,2 Mo,
   pas 13 ; et le corpus grossit de **31 %**, pas du double. Les 945 ne
   comptaient que les `.md`, et le double ne comptait pas les garde-fous.

2 bis. **La pondération, à remesurer — c'est dû maintenant.** 2 326 passages de
   texte libre viennent d'entrer dans un index où BM25 comme le vecteur les
   avantagent : ils ressemblent à de la prose, et les requêtes sont en prose.
   Le banc existe (`e2e_catalogue_gabarits`, trois questions françaises avec
   `product 3,85 · user 7,23 · conversation 7,30`). Tant qu'il n'est pas
   repassé, on ne sait pas si les vraies réponses ont reculé.
3. **Le chantier lui-même** — les deux genres de scope, `Couverture` dérivée,
   et surtout *cesser de jeter* : `has_meaningful_content` supprime les trous
   purement commentaires, `create_file_scope` les type `Module`.
4. **Les décisions non tranchées.** Le genre du parseur générique (ses scopes
   sont des conjectures, `confidence` existe déjà — c'est un troisième genre,
   pas une variante). `.cypher` (171 fichiers) mérite mieux qu'un bloc de
   texte : c'est le langage de requête de la base. `.java` (30 fichiers) n'est
   pas un défaut de couverture mais un langage absent.
5. **La pondération.** Ne rien régler avant la mesure du §4 du deuxième
   document. Le banc existe (`e2e_catalogue_gabarits`).

## 4. Ce qu'il faut savoir pour reprendre

**La sonde ne suit pas les liens symboliques.** `tools/rust_api/rag3db-src`
pointe sur `../..`, la racine du dépôt. Le suivre boucle sans fin — ça a coûté
trente minutes de CPU avant qu'on comprenne que le blocage venait de la sonde
et pas du parseur.

```sh
cargo run --example couverture -- <racine>...      # 1 min 57 s sur le dépôt entier
DETAIL=1 cargo run --example couverture -- ...     # la 1re erreur de chaque fichier
FORCER=cpp cargo run --example couverture -- ...   # .c/.h avec la grammaire C++
```

**codeparsers est un sous-module.** Un `git clone` de rag3db a besoin de
`--recursive`, et une modification du crate se commite dans son dépôt d'abord,
puis le pointeur ici. La dépendance de chemin dans `Cargo.toml` n'a pas bougé.

**Son historique a été réécrit à l'extraction.** Il traînait 1,18 Go de
`target/` compilé, hérité du premier commit avant que le `.gitignore` n'arrive.
Le dépôt fait 368 Ko. Les SHA de codeparsers ne correspondent donc plus à ceux
de rag3db — c'est attendu.

**Une modification non commitée traîne dans l'arbre de travail** :
`.gitmodules`, `fuzzy-fst` de `https` vers `ssh`. Elle n'est pas de cette
session, elle a été préservée volontairement hors de ses deux commits.

## 5. Ce qui n'est pas sûr

*(Le doute sur `extension/rag3weaver/Cargo.lock` est levé : c'était l'entrée
`tree-sitter-bash` du parseur shell, commitée dans `b3d3acaad`, rien n'avait
été perdu.)*

- L'ordre de grandeur « +20 à 60 % de scopes » du §2 du troisième document
  reste une hypothèse. La mesure suggère bien plus pour `.h` (×2,7 rien qu'en
  changeant de grammaire), ce qui veut dire que la cible a été posée sans
  connaître le point de départ. À reposer une fois `.h` corrigé.
- Le README de codeparsers avoue les limites du crate plutôt que de le vendre.
  C'est un choix, sur un dépôt public, qui n'a pas été validé.
- **Le seuil de 128 Kio est un proxy**, et il sera faux un jour : une
  spécification écrite à la main de 300 Kio serait écartée, un fichier généré
  de 50 Kio passerait. Ce qui le rend acceptable n'est pas sa justesse mais le
  fait qu'il s'avoue — les 700 écartés se comptent avec leur raison. Si le
  proxy se met à mentir souvent, c'est la liste des écartés qui le dira.
- **Le `texte_brut` est découpé comme du code, pas comme un document.** La
  crainte du §2 du [doc en aval](02-ce-que-ca-change-en-aval.md) — « un scope de
  20 000 lignes ne doit pas produire **un** chunk » — ne se réalise pas : le
  découpage est déclaré sur l'entité `Scope` (`default_scope_chunking`, 1000 /
  100), donc il s'applique à tous les scopes, texte compris.

  Mais il s'applique avec `ChunkStrategy::Semantic`, réglé pour du code, alors
  qu'un `ChunkStrategy::Markdown` existe et « respecte les titres, les blocs de
  code et les listes ». **945 des 2 326 fichiers texte sont du Markdown**, et
  ils sont découpés sans que leurs titres comptent.

  Et il n'y a pas de porte de sortie évidente : le découpage se déclare **par
  entité**, pas par enregistrement. Découper le texte autrement voudrait dire
  soit une seconde entité — ce que le §1 du même document interdit, avec de
  bonnes raisons — soit un découpage par enregistrement, qui n'existe pas.
  C'est le seul point où le cahier des charges n'avait pas vu la contrainte.
