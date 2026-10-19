# Le COPY de relations ne lit pas le mot NULL dans une colonne entière

- **État** : corrigé le 19 octobre 2026 par le commit qui déplace ce fichier (`git log --follow`) ;
  relevé le 19 octobre 2026 par la session mémoire longue, au tour de push de la pièce 1 « le
  catalogue dit tout »
- **Gravité** : blocage, et base à rouvrir. Le COPY est refusé après avoir commencé (« Conversion
  exception: Cast failed. Could not convert "__nul_…__" to INT64 »), et la connexion doit être
  rouverte.
- **Atteignable en service** : non, depuis le produit. Il ne nomme une colonne absente qu'en
  texte, et passe par lots quand la relation n'a pas de propriété texte. Le filet est cité plus
  bas.
- **Touche rag3weaver** : oui, par son filet.
- **Ouvert le** : 19 octobre 2026, session mémoire longue, à la demande de l'orchestration.

## Ce que c'est

Le COPY de relations reçoit `null_strings=['<mot>']` : le mot convenu de
`rag3weaver::dialect::csv_null`, parce qu'une cellule vide est une chaîne vide. Le mot est bien
lu NULL dans une colonne texte. Dans une colonne `INT64`, le moteur tente de le convertir, et
refuse le fichier entier.

## Recette

```cypher
CREATE NODE TABLE A(_uuid STRING, PRIMARY KEY(_uuid));
CREATE NODE TABLE B(_uuid STRING, PRIMARY KEY(_uuid));
CREATE REL TABLE R(FROM A TO B, raison STRING, poids INT64);
CREATE (:A {_uuid: 'a'}); CREATE (:B {_uuid: 'b'});
// liens.csv : a,b,__nul__
COPY R (poids) FROM 'liens.csv' (from='A', to='B', null_strings=['__nul__']);
// attendu : une arête, poids NULL ; obtenu : « Could not convert "__nul__" to INT64 »
```

## Témoin

Côté produit, le chemin qui le contourne est gardé par
`tests/e2e_catalogue_dit_tout.rs::une_relation_a_proprietes_se_copie_sans_elles`, par
`src/dataflow/copie_liens_tests.rs` et par `src/dataflow/chemin_d_ecriture_tests.rs`. Le témoin du
moteur reste à écrire : la recette ci-dessus, en test du moteur.

## Ses filets

- `extension/rag3weaver/src/dataflow/record_nodes.rs`, `copier_les_liens` :
  - un groupe sans propriété vers une relation qui en déclare ne nomme qu'une propriété texte, au
    mot NULL ;
  - sans propriété texte, il passe par lots.
  Fermé, le filet peut nommer n'importe quelle colonne déclarée, et garder le COPY pour toutes
  les relations.

## Cause et correctif

`setVectorNull` (`src/function/cast_from_string_functions.cpp`), appelé pour chaque cellule par
le lecteur CSV, ne consultait `null_strings` que pour une colonne `STRING` ; pour tout autre
type il ne reconnaissait que la cellule vide et le mot `NULL` (forme de l'amont). Les nœuds
étaient touchés comme les relations. Le mot convenu vaut maintenant pour toutes les colonnes,
hors guillemets ; entre guillemets il reste une valeur (et une colonne entière la refuse par
son nom) ; la cellule vide et le mot `NULL` se lisent toujours NULL hors texte.

## Témoin

`test/transaction/copy_null_strings_test.cpp` (`CopyNullStrings.*`) : la recette du ticket, le
mot dans chaque type de colonne d'un nœud, le mot entre guillemets, et le comportement sans
l'option.

## Ce qu'il fallait pour le fermer

Que `null_strings` vaille pour toutes les colonnes d'un COPY, quel que soit leur type, comme
pour le texte : fait. Le filet du produit (`copier_les_liens`) reste à retirer par la session
mémoire longue quand sa lib est `courant`.
