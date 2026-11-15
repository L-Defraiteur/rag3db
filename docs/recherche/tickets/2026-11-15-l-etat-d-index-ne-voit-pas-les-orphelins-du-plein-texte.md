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
