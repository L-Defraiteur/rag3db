# Qualifier les relations sur les projets du corpus public

- **État** : ouvert — suite distincte de la référence par fichier livrée e5e2d9cf5a
- **Famille** : produit
- **Gravité** : les compteurs du premier banc ne prouvent aucune résolution de projet
- **Atteignable en service** : oui, résolution et ingestion de code ; cette suite reste isolée des services
- **Touche rag3weaver** : oui, faits et relations de codeparsers puis consommateurs à qualifier séparément
- **Pour** : codeparsers, coordination C7 et oracle indépendant Banc
- **Signalé par** : contrat du premier corpus et constat explicite project_resolution=not_run

La référence FULL02b (32382 fichiers, ELFbff1cafb) est par fichier ; ses relations
CSS/documentaires ne prouvent pas des liens de code. Les occurrences, propriétaires,
imports, alias et positions doivent rester distingués du contenu dédupliqué.

Choisir des unités de projet bornées dans les dépôts figés, avec leurs dépendances
locales disponibles. Une sélection aléatoire de fichiers ne forme pas forcément
un projet résoluble ; ne pas agréger les24dépôts sous une racine fictive et ne
pas inventer un lien quand une cible manque. Composer les options et manifestes
existants de ProjectParser/résolveur avant tout mécanisme neuf.
Poser avant résultats un oracle de couples mention/cible et de non-liens, puis
qualifier arêtes correctes/manquantes/fausses et provenance/positions. Les sorties
et budgets doivent être figés sous allocation neuve, sans scripts publics exécutés,
modèles, base de Lucie ou ancien GO. L'ingestion serait une étape séparée, pas
une réussite déduite du seul résolveur en mémoire.

Fin : recette et preuves par projet, limites explicites et enfants pour défauts,
sans taux de rappel global issu d'un échantillon syntaxique choisi.
Parent initial : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`
(référence exploratoire livrée ; ce ticket reprend la suite non exécutée).
