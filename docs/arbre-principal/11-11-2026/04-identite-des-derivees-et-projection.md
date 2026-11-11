# Identité des contenus dérivés et projection vers les occurrences

11 novembre 2026. Préparation du lot 1a ; proposition, pas fonctionnalité livrée.
Le cap et les contrats relus restent dans 03-contrats-et-lots-des-vues.md.

## Ce que le runtime prouve aujourd'hui

- derive_nodes.rs construit derived_uuid(entity, root_uuid). Une dérivée appartient donc à une
  racine ; déclarer hashsafe sur son schéma ne transforme pas cette identité en adresse de contenu.
- DeriveNode produit déjà une relation générique, nommée par derived_rel_name(entity), de la
  dérivée vers sa racine. Ce mécanisme peut porter plusieurs racines si l'identité le permet ;
  il faut examiner cette composition avant d'ajouter une seconde relation synonyme OF_VERSION.
- _source_uuid et _source_entity sont aujourd'hui singuliers. Le hash _render_hash est calculé
  sur le contexte de la racine et de ses voisines, pas seulement sur le texte réellement rendu.
  Il ne peut pas représenter plusieurs occurrences sur une seule version partagée.
- La projection dense/creuse perdait un parent par écrasement dans HashMap. Le pré-lot 1a0 corrige
  cette cardinalité, la déduplication par paire, les scores ex aequo et l'attribution par parent ;
  il réadmet aussi chaque parent sous le filtre effectif et la cellule de la même lecture L6.
  Sa passe est verte (branche cx-arbre-projection-occurrences-2, 23f4c2f1), mais son tour de push
  est encore en attente. Ce pré-lot ne partage aucun stockage et ne change aucun format.
- Le témoin de stockage sur cx-arbre-scope-version-2 reste rouge : deux fichiers portant le même
  corps de fonction gardent leurs occurrences/positions, mais l'embarquent deux fois.

## Modèle proposé, à éprouver par composition

1. **Déclarer l'identité de la dérivée.** Conserver le mode actuel par racine pour les formats
   existants. Une identité par contenu doit couvrir les champs effectivement rendus/chunkés,
   le titre et la docstring quand ils y participent, les contextes déclarés (langage/type, etc.)
   et la recette versionnée de découpage. Conserver les octets au départ ; aucune normalisation
   qui invaliderait les highlights. Ne pas confondre cette adresse avec l'identité des vecteurs :
   modèles et dimensions restent distingués par les mécanismes existants du magasin vectoriel.
2. **Garder l'état de rendu par racine/lien.** Le contenu partagé est immuable. Une mise à jour
   de position peut modifier le hash d'entrée d'une occurrence sans créer une autre version de
   contenu. Le suivi _render_hash doit donc être situé sur la référence ou dans un état propre
   à la racine, plutôt qu'écraser l'état des autres occurrences dans la version.
3. **Remplacer une référence, pas toutes les origines.** Quand le contenu d'une occurrence change,
   repointer son lien seulement, dans la transaction du changement. Les autres racines restent
   sur leur version. Les anciennes versions ne sont collectées que selon les règles de rétention
   (autres occurrences, citations et lecteurs épinglés). Ne pas choisir de compaction avant M5.
4. **Déclarer la projection de recherche.** Distinguer stockage indexé et entité publique rendue,
   via les relations déjà déclarées. Un simple changement target=ScopeVersion ne suffit pas :
   chemins, lignes, fraîcheur, liens, regroupement et after attendent encore des occurrences.
   Étendre la primitive commune et les graphes, pas dupliquer une recherche spéciale Scope.
5. **Projeter le masque avant le top-k.** Même sur l'ancre seule, les versions sans occurrence
   visible ne doivent pas consommer le budget des index. Dense, creux et plein texte recherchent
   sous le masque projeté ; le résultat se redéploie ensuite vers les seules occurrences admises.
   Un morceau admis via A ne donne aucun droit sur B. Refuser si le signal ne garantit pas le
   masque. Masque, enrichissement et provenance restent dans le même L6.

La forme exacte de la déclaration d'identité, du suivi par lien et de la projection reste à
implémenter et vérifier. Le mode par racine existant doit rester inchangé. Les tables, relations
et traitements passent par les contrats et graphes communs ; pas d'activation sur les bases de
service avant migration déclarée 1b sur copies privées et refus explicite d'un format incompatible.

## Preuves déjà disponibles et prochaines preuves

La projection du pré-lot 1a0 dispose d'un rouge perte de parent, d'un rouge de fuite provoquée en
neutralisant la réadmission, d'un rouge sur l'égalité des scores puis de leurs verts. Passe finale
88136cc0e, rebasée en docs seules sous 23f4c2f1 : features5/5 ; lib2096/0/58 ignorés ; navigateur33/0/0 ;
e2e_projection_occurrences1/0, e2e_search40/0, e2e_scope12/0, e2e_postgres34/0. Journaux sur luciepc :
~/.cache/rag3weaver-build/cx-projection-passe-v3/ ; PostgreSQL a tourné dans la base privée
cx_projection_2191_20261111. Les lancements antérieurs interrompus/rouges restent séparés.

Pour 1a, rendre vert le compteur d'embarquement partagé, puis prouver les variantes de titre/docstring,
de recette et de modèle ; la conservation des occurrences et arêtes ; la couverture des résultats de
l'ancre. Avant de choisir un cache de masques, mesurer M3 (dont le coût de matérialisation des parents
admis), M4 puis M5 dans un créneau non perturbé. Aucun chiffre de ces mesures n'est disponible.

Ne pas réutiliser le target d'un lot prêt en attente de son nettoyage. Le pré-lot 1a0 tient encore
cx-target-scope-version à luciepc ; les travaux de préparation ci-dessus n'y lancent aucun cargo.
Après atterrissage, rebaser 1a sur le master final et conserver la correction des égalités de score,
le contrat L4 et le témoin dédié e2e_projection_occurrences.
