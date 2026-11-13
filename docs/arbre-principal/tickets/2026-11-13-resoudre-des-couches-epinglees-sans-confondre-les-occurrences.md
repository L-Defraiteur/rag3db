# Résoudre des couches épinglées sans confondre les occurrences

- **État** : ouvert ; sous-lot2a en préparation/code isolé, aucun runtime ni qualification.
- **Famille** : produit.
- **Gravité** : capacité manquante ; pas encore de résolution de couches épinglées.
- **Atteignable en service** : non pour la nouvelle primitive ; les outils lisent la vue L6 existante sans pile de couches.
- **Touche rag3weaver** : oui, résolution pure de publications/couches ; aucun stockage ou chemin de recherche modifié dans ce premier diff.
- **Pour** : arbre-principal ; revues C7/ED/Recherche/Mémoire.
- **Signalé par** : suite cohérente pyramide demandée C7 après landing b0a5177c0.

Parent [ancres](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).
Contrat [lots2a/2b](../11-11-2026/03-contrats-et-lots-des-vues.md).

Avant diff produit : déclarer une pile immuable de révisions de couches, chaque
révision liée à origine/publicationP existantes et parent exact. Résolution
pure de valeurs d’occurrences opaques, identité occurrence distincte du contenuH
qu’elles portent. Tombstone occurrence/fichier masque les ancêtres ; override
local prévaut sans fusionner les occurrences partageantH.

Réutiliser PublishedDeclaration(P) et QualifiedPhysicalState(neuf) pour
construire les références, aucune nouvelle génération parallèle ni création
d’index/base. Refuser doublons/parents absents/cycles/origines différentes,
vue inconnue, et sélection par nom ambigu entre plusieursP. Résultat lié à
un ensemble immuable et provenance par occurrence. Pas de certificat physique
forgé depuis la déclaration, pas d’accès index autorisé par cette primitive.

Témoins CPU à écrire avant implémentation, NON joués : f parente/g fille/h
ancre, suppression héritée occurrence/fichier, réintroduction descendante,
ancre avancée sans effet implicite, mêmeH/deuxoccurrences, inconnue/ambiguë,
parent manquant/cycle/origine différente, champs/positions/provenance propres.
Sous-lot2b reste distinct : masques avant top-k dense/creux/FTS et arêtes aux
deux bouts visibles, intégration L6/outils/stockage et durabilité non qualifiées
par un résolveur pur. Aucun clone de base, legacy, modèle, service ou M3-M5.

Fin de ce diff : API et contre-exemples publiés pour revue, recette CPU bornée
à proposer ensuite ; aucune exécution Cargo/DB/service sans GO exact.
