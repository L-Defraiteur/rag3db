# Identité des contenus dérivés et projection vers les occurrences

11 novembre 2026. Lot 1a partiellement implémenté en branche ; aucune activation en service.
Le cap et les contrats relus restent dans 03-contrats-et-lots-des-vues.md.

## Première implémentation générique, après cette proposition

Checkpoint initial 7748fa53c, branche cx-arbre-scope-version-3. DerivedIdentity déclare root
(défaut historique) ou content avec format. L'adresse couvre champs réellement rendus, octets,
rôles, découpage et cellule ; les références DERIVED_FROM portent leur état de rendu. Une origine
changée repointe seule, une version déjà présente ne se réinsère pas, les versions restent retenues.
La transaction L3 existante enveloppe le remplacement ; les ressources indépendantes de la cible
restent en file même après rollback. Les mutations directes des versions/références sont refusées.

Preuves ciblées : 10 unitaires, 8 e2e dérivées ; compteur générique 2→1, déplacement et réutilisation
sans nouvel embarquement ; modèles 4/6 dans des stockages distincts sans dupliquer les contenus ;
recette changée puis restaurée ; injections d'échec d'insertion et de lien sans perdre état, origines,
file indépendante ou cohérence FTS. Voir le rapport pour les journaux et essais non concluants.
Ce n'est ni une passe globale ni le partage du schéma Scope. Projection publique et masque avant
top-k restent ouverts ; la recherche de versions internes est explicitement refusée jusque-là.
Recherche recommande une seule association cible publique → index et la composition des relations,
filter_indirection et Forme/L6 existantes, pas un ResolveParent après pagination.

Les constats et propositions qui suivent décrivent le point de départ de cette implémentation ;
le rapport prioritaire donne l'état de reprise courant. Aucune base vivante n'a été modifiée.

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
  Il est livré sur master **6c4b5ad0c** après sa passe complète puis le rejeu court sur le lot ED. Ce pré-lot ne partage aucun stockage et ne change aucun format.
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
neutralisant la réadmission, d'un rouge sur l'égalité des scores puis de leurs verts. Passe complète
88136cc0e : features5/5 ; lib2096/0/58 ignorés ; navigateur33/0/0 ;
e2e_projection_occurrences1/0, e2e_search40/0, e2e_scope12/0, e2e_postgres34/0. Le rejeu après ED
sur 51a2faa1f a rendu features5/5, lib2097/0/58 ignorés et les mêmes 4 suites vertes. Journaux sur luciepc :
~/.cache/rag3weaver-build/cx-projection-passe-v3/ et cx-projection-rejeu-ed/ ; PostgreSQL a tourné dans la base privée
cx_projection_2191_20261111, supprimée après les trois contrôles du push. Les lancements antérieurs
interrompus/rouges restent séparés.

Pour 1a, rendre vert le compteur d'embarquement partagé, puis prouver les variantes de titre/docstring,
de recette et de modèle ; la conservation des occurrences et arêtes ; la couverture des résultats de
l'ancre. Avant de choisir un cache de masques, mesurer M3 (dont le coût de matérialisation des parents
admis), M4 puis M5 dans un créneau non perturbé. Aucun chiffre de ces mesures n'est disponible.

Le target cx-target-scope-version, les worktrees de projection et la base PG privée ont été
nettoyés après l'atterrissage. Le lot 1a a été rebasé sur ce master ; ses fichiers de runtime sont
identiques au master6c4b5ad0c lors du rebase, le reliquat porte sur les témoins et cette préparation. Nouveau target unique
prévu au besoin : ~/.cache/rag3weaver-build/cx-target-contenu-derive, non créé à cette étape.

## Contrôle supplémentaire de la cellule de la requête

La relecture a demandé de vérifier options.scope contre la cellule courante du catalogue.
Le témoin diagnostique fdd710eb2 a passé 12 cas : dense/creux × 3 modes × alpha explicitement
demandée/beta courante. Un morceau alpha relie deux occurrences alpha et une beta ; la seconde
alpha n'est atteignable que par le morceau partagé. Les UUIDs attendus sont exacts, les deux
alpha gardées, beta exclue, et le catalogue reste en beta. Le test est conservé pour 1a dans
la suite e2e_projection_occurrences ; il ne faisait pas partie du candidat 51a2faa1f poussé.

Le chemin public Catalog::rechercher consomme options.scope, puis register_search_services_in
fournit les services de la cellule demandée. Réadmettre par le service scope ne change donc pas
cette précédence dans l'API testée. Aucun correctif runtime n'a été nécessaire sur ce point.
Journal : ~/.cache/rag3weaver-build/cx-projection-cellules-v2.log.

Le premier diagnostic avait un montage sparse incomplet (sans connexion BlobStore explicitée et
sans initialisation du schéma dans les deux cellules). Son « no sparse handle » n'est pas compté
comme rouge du produit. Le comparateur sans pré-lot a rencontré l'ancienne perte de parent dès
alpha, avant d'atteindre le contrôle no-handle prévu ; l'hypothèse de comparaison exacte no-handle
n'est donc pas établie. Sources comparées restaurées, diff propre. Les recettes et logs restent dans
cx-projection-cellules-baseline/, séparés de la passe et du diagnostic corrigé.

## Coordination avant édition de code.rs

Codeparsers tient analyze_in_project (après porteur_de/reexportes et avant les références fa.scopes),
default_export_symbol (extension comprise) et resolve_across_batches. Son repère communiqué est
3f8f1e319 / sous-module 72810b4, à réactualiser avant reprise. Conserver default_exports et
pending_defaults tuple3, symbole @default:{source}#{fichier exact}. Le fait typé default_export
relève du fichier et du contexte de résolution : il peut changer les arêtes d'une occurrence sans
changer son contenu chunké. Aucun changement de stable_scope_keys, own_texts, ScopeRecord, IR ou
chunking n'est demandé à codeparsers pour le partage de contenu.
