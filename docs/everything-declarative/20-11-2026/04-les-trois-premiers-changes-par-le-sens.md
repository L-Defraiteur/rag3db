# Les trois premiers changés par le sens, avec les déclarations courtes

20 novembre 2026, session everything declarative. Le juge du banc sur le lot `variable-not-candidate` (03fbecf8b8, contre
master 216c3a70e6). **Cette page a été corrigée le soir même** : sa première version lisait mal les scopes rendus, voir
« La première lecture, et pourquoi elle était fausse ».

## Les trois cas, lus par l'hôte

Par l'hôte de 03fbecf8b8 (limit 10, format json, le nom et le contenu de chaque résultat ;
`banc-parite/sorties/feuille-5d-20261120/quel-scope.json` à luciepc) :

**1. « parse a json manifest », sans filtre**
- avec le lot : 1ᵉʳ `manifest` (une variable de 51 caractères, `experiments/mtga/scripts/prepare_engine_relations.py:14`),
  2ᵉ `manifest` (33 caractères, `extension/rag3weaver/scripts/test_backend_persistence.py:78`), 3ᵉ `parse_declaration`
  (méthode, 1 143 caractères) ;
- sur master : 1ᵉʳ `parse_declaration`. Sur les références de G (l'ancienne fusion), une méthode en tête, et la
  variable courte au rang 3 ou 4.

**2. « rebuild the vector index », langage python**
- avec le lot : aux rangs 1 à 4, des variables `index = ROOT/'README.md'` (22 caractères), dans
  `build_plant_variant.py:32`, `build_red_variants.py:50`, `build_theme_variants.py:42` et
  `build_ultimatum_variant.py:31`.

**3. « transaction rollback », langage python**
- sur master : `last_accepted` (une variable de 29 caractères) ; avec le lot, elle sort du top 10. C'est l'effet voulu :
  une déclaration courte n'a plus de vecteur.

## Ce qui explique les cas 1 et 2 : le revers de la décision (A)

Ce sont des déclarations **courantes**, sans vecteur, dont **le nom est un mot de la requête** (`manifest`, `index`).
Elles sont premières en BM25 pour cette raison. La décision (A) ramenait le score d'une ligne sans vecteur au poids
total, soit × 2,2 environ : une première en BM25 battait alors tout ce qui a les deux signaux, y compris ce que le sens
trouve. Ce n'est pas la règle du nom exact (B) : la requête n'est pas leur nom.

C'est le revers que c7 voulait exclure, et il est apporté par la bascule. Il a son ticket
(`tickets/2026-11-20-une-declaration-courte-prend-la-tete-par-un-mot.md`), fermé par la décision (A') : le signal dense
d'une ligne sans vecteur compte au rang qui suit le dernier de sa liste, sans mise à l'échelle. La recherche par le nom
reste tenue par (B).

## La première lecture, et pourquoi elle était fausse

La première version de cette page attribuait les cas 1 et 2 à des scopes `variable` qui recopiaient toute leur ligne
(220 et 152 caractères), et ouvrait un ticket pour codeparsers. **C'était faux.** La clé du banc (`chemin:ligne`) ne
disait pas quel scope de la ligne remontait. Ces scopes de la ligne entière sont des **versions fermées** d'une analyse
antérieure au bornage aux octets (`_depuis` 0, `_jusqu_a` 1), et ils n'ont pas été rendus. Les générations 0, 1 et 2
sont publiées, et la recherche ne lit pas de version fermée. Codeparsers l'a confirmé : l'analyse courante ne produit
aucun scope de la ligne entière. Le ticket est fermé comme faux défaut.
