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


### Précisions après revue statique5ec8

Tombstone « scope » signifie ici occurrence de l’entité Scope par UUID, PAS
cellule(org,project). OccurrenceKey doit être unique dans l’origine choisie.
FileKey est une identité logique fournie : l’appelant inclut domaine/cellule
si nécessaire ; deux cellules nommant a.rs par la même clé seraient masquées
ensemble. Le résolveur ne peut reconstruire un domaine absent de ses entrées.
Des racines indépendantes d’origines différentes sont admises ; seule une
arête parent entre origines différentes est refusée.

Le juge existant ajoute remplacement fichier tombstone+valeurs locales DANS
la même révision et deux FileKeys qualifiées partageant le chemin affiché,
ainsi que racines indépendantes multi-origines. Compte inchangé4fonctions,
toujours NON JOUÉES. Validation cycles O(n²) dans une chaîne, aucun coût mesuré.
Deux propositions de fenêtre ont été reçues(900sRecherche/3000sED) ; ni durée
ni compilation autorisées avant décision/recette/GOexact C7 et disponibilité.


## Pilote concret préparé après arbitrage3000 C7

Application69dc figée, contrôle en WT/branche séparés ; [recette11](../13-11-2026/11-recette-qualification-couches-2a.md).
Build1800→inventaires4exact0ignored/ELFJSONcopiéledgerpublié→4exact60
chacun→publication120→checknatif900 dans même échéance3000 après poste
réel. Attestation maps/profil déterministe au main du premier exact via gdb,
puis continuation de ce même test, aucun test ou modèle supplémentaire.
Sup/driver/final horsW, environnement liste blanche, cgroup32/HIGHmax/CPUj8
etSDK29, target privé neuf absent avant GO. Sources et pilote propres figés.
Compilation adaptateur ne vaut pas runtime natif. Revue+GO du couple SHA
encore requis ; zéro Cargo/rustc/target/DB/service créé pendant préparation.
