# L'état d'index ne voit pas les orphelins du plein texte : `fts_counts` date du dernier index complet

- **État** : ouvert (15 novembre 2026, session recherche, à la demande de c7).
- **Pour** : recherche (l'état d'index), sur le relevé à sec de lucivy (`POST /backend/maintain` avec
  `dry_run: true`, lot confié à lucivy par c7).
- **Famille** : produit
- **Gravité** : un défaut de données reste muet. L'état d'index affiche « autant de documents que de lignes »
  alors que des orphelins existent : le 15 novembre, il disait 68 585 pour 68 585 au dépôt, chiffre du
  25 octobre, quand le compte à froid en trouvait 32.
- **Atteignable en service** : oui (`daemon_status`, `/etat`, la vue de l'état dans l'IDE).
- **Touche rag3weaver** : oui (`Catalog::noter_les_comptes_du_plein_texte`, appelé par
  `dataflow/index_nodes.rs` à la fin d'un index complet ; l'état qui le lit).

## Ce que c'est

`index.fts_counts` dans l'état d'un backend vient de `noter_les_comptes_du_plein_texte`. Seule la fin
d'un index complet l'appelle (`index_nodes.rs`, la « garde des comptes, dite »). Les écritures
incrémentales de l'hôte (montres, réindexation d'un fichier, fin d'instantané d'un fichier retiré) ne le
rafraîchissent jamais, et la maintenance du plein texte non plus.

Le 15 novembre à 13 h 5x, sur `code-rag3db` au portable :
- l'état affichait `updated_ms` 1792966868365 (environ le 25 octobre) et `fts_counts` Scope 68 585 pour
  68 585, File 6 508 pour 6 508 ;
- `POST /backend/fts-live-ids` rendait 81 706 documents vivants Scope et 8 130 File ;
- le compte à froid de 14 h 00 (copie de l'instantané `recherche-compte-orphelins-1-20261115-1400`, méthode
  de hotfixs) trouvait 81 674 lignes Scope et 8 126 File, donc **32 et 4 orphelins**.

L'état ne peut donc pas dire « des orphelins existent ». Un compte vrai demandait jusqu'ici de fermer le
backend pour en prendre une copie fidèle.

## Ce qu'il faudrait

- Le relevé vivant, base ouverte. Le relevé à sec de lucivy rend
  `{live_docs, graph_docs, orphans, dictionary_texts}` sans rien purger.
- L'état l'écrit à chaque passe de maintenance, pas seulement à la fin d'un index complet, et dit sa date
  à côté du chiffre.
- Un chiffre ancien se dit ancien : « relevé du <date> », jamais présenté comme l'état présent.
- Le témoin : un orphelin posé dans un test (une suppression lucivy non validée, comme avant 262be6fc4),
  puis l'état relu, qui doit le compter sans index complet entre les deux.

## Reprise Recherche — plan du 16 novembre, avant diff

Base lue927941a67a5805450d49f1e04b39fdd542a6059b ; W privé
`~/.cache/rag3weaver-build/cx-wt-recherche-fts-counts-freshness-r1`, branche
`cx-recherche-fts-counts-freshness-r1`. Aucun WIP produit antérieur repris.
Réservation à coordonner avec ED : `Backend::index_states` et tests
`fts_measurement_*` seulement, aucun témoin ties. Landing après ED.

Ordre prévu :
1. Extraire sans changement de comportement la projection JSON des comptes
   dans `catalog/fts_counts.rs` (dépendance serde_json seule) ; un harnais CPU
   privé peut inclure ce vrai module, sans DB/SDK/recopie de l'implémentation.
   Écrire le témoin d'absence d'horodatage avant correction. Pas de grep utilisé
   comme substitut d'un rouge comportemental.
2. Écrire le témoin backend réel : orphelin FTS ajouté/commité dans fixture
   privée, relevé à blanc, état relu sans index complet ; vérifier présence
   de l'orphelin, date et absence de purge. NON JOUÉ pendant fenêtreG/Arbre.
3. Réutiliser `fts_releve_a_blanc` pour enregistrer par entité publique/table
   physique les documents/lignes/orphelins et `measured_at_ms`. Conserver les
   clés existantes documents/rows ; anciennes notes sans date = date inconnue,
   jamais rajeunies à la lecture. L'âge calculé n'est pas un relevé vivant.
4. Rafraîchir fin d'index et maintenance, sans modifier la purge ni son jeton.
   Une erreur de relevé/enregistrement ne renouvelle pas la date et reste dite.
   Ne pas compter uniquement les lignes visibles de la vue comme orphelins :
   le relevé physique existant porte la table de l'index, toutes cellules.
5. Qualification sur lib neuve uniquement après FINI G : témoin rouge réel,
   correction, check_features/lib entière/NAV et e2e_code_sync+e2e_lectures par
   passe canonique bornée32GiB/j8, reçus/journaux réels. Aucun master sans ÀTOI.

Noms proposés : Catalog::record_fts_measurement / refresh_fts_measurements,
Catalog::fts_releve_a_blanc / fts_maintenance_nocturne, hook finindex et
Backend::index_states. La projection JSON pure porte les tests CPU ; pas de
nouvelle interface d'outil ni calcul par lecture d'état sur toute la base.
Pendant fenêtre G/Arbre : RO/écriture privée seulement ; aucune passe lourde,
service, base vivante, suppression, ancien GO ou délai réactivé. La portée
exacte d'une petite compilation du harnais CPU doit être confirmée par C7
avant lancement ; aucune compilation ni test encore joué dans cette reprise.

Clarification C7 après le plan : CPU hermétique = interpréteur déjà présent,
AUCUN Cargo/rustc avant FINI G. Les quatre témoins backend fts_measurement_*
et le renforcement du juge de finindex sont committés AVANT correctif mais
NON JOUÉS ; aucun rouge observé revendiqué. L'extraction de projection JSON
reste un choix d'implémentation, pas un prétexte pour compiler dans la fenêtre.
Précision réservation : les comptes sont rendus par index_states_du_code,
appelé par index_states ; seule cette section FTS et ces tests sont touchés.

Cinquième juge spécifié AVANT correctif : une égalité de comptes historiques
ne promeut pas l'état texte running en ready. Les cinq tests backend et
le test finindex restent NON JOUÉS ; C7 a levé seulement l'écriture privée,
pas la compilation pendant la fenêtre. Aucun harnais de substitution joué.

## WIP publié avant isolation de l’urgence existing-only

Les témoins ont été publiés seuls dans e5ec4bb8c5 puis 78db6b3026 : cinq fonctions backend et renforcement du témoin de fin d’index. Ils restent NON JOUÉS ; aucun rouge observé, aucune compilation ni qualification. Le harnais/extraction du premier plan n’est pas retenu : les fixtures appellent directement les primitives existantes.

Ce WIP ajoute en Catalog la persistance de relevés réussis datés (live/graph/orphans/table), les hooks dry-run/maintenance/fin d’index, et expose âge/date inconnue dans `Backend::index_states_du_code` (précision du nom réservé). Les comptes historiques ne promeuvent plus text en ready. La maintenance pure, ses jetons, sa purge et son ratio restent inchangés ; seule une erreur de rafraîchissement est ajoutée au rapport. Le commentaire host est ajusté pour la seule écriture des métadonnées.

Diff contrôlé textuellement seulement ; NON QUALIFIÉ, pas destiné à master en cet état. La priorité C7 devient le ticket fils `2026-11-16-la-maintenance-du-plein-texte-ne-peut-pas-ouvrir-un-index-existant.md`, isolé dans un nouveau W sans ces changements. Qualification ultérieure sur pin G exact, après ED, avec rouge réellement observé avant validation du correctif et suites requises code_sync/lectures.
