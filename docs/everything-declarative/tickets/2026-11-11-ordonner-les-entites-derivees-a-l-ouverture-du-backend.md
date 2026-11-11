# Ordonner les entités dérivées à l'ouverture du backend

- **État** : ouvert (11 novembre 2026), capacité à vérifier pour l'intégration déclarative du lot 1a.
- **Famille** : produit
- **Gravité** : planification et risque de refus à l'ouverture d'un manifeste dérivé ; pas de reproduction e2e encore établie.
- **Atteignable en service** : à confirmer avec un manifeste déclarant une racine et sa dérivée.
- **Touche rag3weaver** : oui, src/backend.rs, enregistrement du schéma ; DerivedConfig et Catalog::verifier_la_derivation.
- **Pour** : everything-declarative ; arbre-principal porte le raccordement au lot 1a.
- **Signalé par** : arbre-principal, [ticket du contenu partagé](../../arbre-principal/tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).

## Éléments relus

PreparedBackend conserve les configurations dans un HashMap<String, EntityConfig>. Son ouverture
initialise un catalogue vide, puis appelle register_entity dans l'ordre de cette HashMap ; les
relations déclarées sont enregistrées après les entités. Catalog::verifier_la_derivation exige que
la racine soit déjà enregistrée et que les relations de gather existent. Ces contraintes doivent
être composées à l'ouverture, indépendamment de l'ordre des noms ou de la HashMap.

Repères de lecture sur la base du lot arbre-principal 49caf64c6 : backend.rs vers 961 et 2248–2308,
catalog.rs vers 1935. Les primitives natives du contenu partagé sont éprouvées par API Catalog,
mais cela ne prouve pas encore l'accessibilité depuis un manifeste de backend.

## Travail attendu et critères de fin

Poser un e2e minimal de chargement/ouverture de manifeste avec racine, index dérivé et association
publique search_index, via le chemin normal PreparedBackend. Reproduire ou invalider le risque
d'ordre ; réutiliser l'ordonnancement existant s'il couvre déjà ces contraintes, sinon composer
racines, relations puis dérivées selon leurs dépendances, avec refus explicite d'un cycle ou manque.
Prouver écriture sur la racine et recherche sur sa cible publique sans dépendre des noms ni des
semences de HashMap. Les dérivées avec gather doivent également avoir leurs relations disponibles.

Aucun cas spécial Scope, aucune activation des bases de service. Ce ticket est distinct des preuves
natives du lot 1a et n'autorise pas un nouveau lot de code sans coordination avec l'orchestration.
Le porteur peut invalider le risque avec sa preuve ; il ne doit pas être présenté comme reproduit avant ce témoin.
