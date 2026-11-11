# Deux occurrences identiques réembarquent le même contenu

- **État** : ouvert (11 novembre 2026), lot des versions de contenu 1a.
- **Famille** : produit
- **Gravité** : coût inutile, un contenu identique est embarqué une fois par occurrence.
- **Atteignable en service** : oui (deux fichiers ou branches portent le même scope).
- **Touche rag3weaver** : oui (déclaration du gabarit code, identité de contenu, ingestion/recherche).

## Preuve

Le témoin deux_occurrences_partagent_un_embarquement_et_gardent_leurs_positions,
suite e2e_scope_versions, branche cx-arbre-scope-version-2 (c4bce011c), est joué sur
luciepc avec la lib29a7bc52a : deux occurrences a.rs/b.rs, positions distinctes correctes,
mais le compteur reçoit deux fois le même texte de fonction au lieu d'une.
Journal hors target : ~/.cache/rag3weaver-build/cx-scope-version-etat.log ; résultat de
la suite : 2 réussis, 1 échoué. La projection générique corrigée ne partage pas à elle seule
le stockage ; aucun format de base vivante n'a été changé.

## Pour le fermer

Mettre en œuvre le partage de contenu/chunks/vecteurs déclaré du lot1a, avec empreinte sur
l'entrée effectivement chunkée et version de découpage, modèle/dimension distingués. Conserver
les occurrences, positions, relations et visibilité. Prouver un seul embarquement pour le même
contenu et des contenus distincts pour docstrings/titres pertinents ; migration explicite1b sur
copies avant toute base partagée. Contrat : docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md.
