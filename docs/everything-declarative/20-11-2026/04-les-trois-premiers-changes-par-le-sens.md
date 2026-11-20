# Les trois premiers changés par le sens, avec les déclarations courtes

20 novembre 2026, session everything declarative. Le juge du banc sur le lot `variable-not-candidate` (03fbecf8b8, contre
master 216c3a70e6) ; les types et les longueurs lus par Cypher sur une copie de la base du lot, les rangs dans
`banc-parite/sorties/feuille-5d-20261120/trois-premiers.json` à luciepc. Le lot lui-même est décrit dans la page 03 de ce
dossier.

## Les trois cas

**1. « parse a json manifest », sans filtre**
- master : `extension/rag3weaver/codeparsers/src/css/css_parser.rs:403`, méthode `parse_declaration` (lignes 403-441,
  1 143 caractères). Avec le lot : rang 3.
- lot : `experiments/mtga/scripts/prepare_engine_relations.py:14`, une `variable` d'une ligne de 220 caractères. Sur
  master : hors du top 10.

**2. « rebuild the vector index », langage python**
- master : `extension/rag3weaver/scripts/test_backend_persistence.py:89`, module `file_scope_08` (lignes 89-90, 105
  caractères). Avec le lot : rang 5.
- lot : `experiments/mtga/scripts/build_plant_variant.py:32`, une `variable` d'une ligne de 152 caractères. Sur master :
  hors du top 10.

**3. « transaction rollback », langage python**
- master : `experiments/mtga/scripts/audit_validated_run.py:40`, la `variable` `last_accepted`, 29 caractères. Avec le
  lot : hors du top 10. C'est l'effet voulu : une déclaration courte n'a plus de vecteur.
- lot : `extension/rag3weaver/scripts/test_backend_snapshot.py:128`, module `file_scope_21` (lignes 128-129, 167
  caractères). Sur master : rang 7.

## Ce qui explique les cas 1 et 2

Une ligne Python qui enchaîne des affectations (`manifest=…;cards=…;owned=…`) porte **plusieurs scopes `variable` sur
la même ligne**, de deux sortes :
- **les uns ont pour contenu leur seule affectation** : 26 à 59 caractères pour `prepare_engine_relations.py:14` ; 19,
  22 et 109 pour `build_plant_variant.py:32`. Sous 80, ils sont écartés par la règle du lot ;
- **les autres ont pour contenu la ligne entière** : 220 et 152 caractères, au-dessus du seuil, donc candidats, avec un
  vecteur.

Libérés de la concurrence de leurs jumeaux courts, ce sont ces scopes de la ligne entière qui prennent la tête. La clé
du banc (`chemin:ligne`) ne dit pas lequel des scopes de la ligne remonte. Mais la ligne entière est la seule de
longueur candidate, et c'est elle.

**Le cas 2 est la ligne même du 88 %** (`build_plant_variant.py:32`, celle de `s=index.read_text()`) : sa version
« ligne entière » remonte pour une requête sans rapport. Le défaut n'est pas dans la règle du lot. C'est un scope qui
recopie le texte de ses frères : un `variable` dont le contenu est toute la ligne, alors que chaque affectation a déjà
son propre scope. Il a son ticket (`tickets/2026-11-20-un-scope-variable-recopie-la-ligne-de-ses-freres.md`), pour
codeparsers.

## Ce qui n'est pas en cause

- **Le cas 3** est l'effet voulu du lot : une déclaration de 29 caractères perd son vecteur.
- **La fusion (A) et le nom exact (B)** ne jouent dans aucun des trois : aucune requête n'est un nom, et les lignes qui
  montent ont un vecteur.
