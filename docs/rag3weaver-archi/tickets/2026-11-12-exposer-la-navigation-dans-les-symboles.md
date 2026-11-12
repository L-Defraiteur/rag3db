# Exposer la navigation dans les symboles

- **État** : ouvert ; conception autorisée par Lucie le 12 novembre, interfaces proposées et couverture actuelle à qualifier avant code.
- **Famille** : produit
- **Gravité** : capacité manquante ; l'agent doit pouvoir explorer des arbres de code sans lire tous les corps.
- **Atteignable en service** : non pour les nouvelles façades ; outline et les relations de parents existent déjà.
- **Touche rag3weaver** : oui, outils déclaratifs du code et de l'atelier, références de symboles, rendu et enrichissement du grep.
- **Pour** : ED et archi pour les contrats/outils ; codeparsers pour la couverture des membres et relations.
- **Signalé par** : Lucie, demandes list_methods puis list_members, parents/enfants bornés, list_symbols(file) en arbre et option symbol_hierarchy_depth du grep.

## Demande et conception

[Vision](../../orchestration/visions/2026-11-12-explorer-les-symboles-et-leur-hierarchie.md).
Composer outline/Plans/parcours existants, plutôt qu'un deuxième moteur par client.
Ne pas confondre contenance, appartenance déclarée et héritage ; une référence doit
être qualifiée dans sa vue. Décrire les limites de couverture et d'index au lieu
de rendre une absence trompeuse. Grep : profondeur supplémentaire0 par défaut,
enrichissement optionnel avec provenance, sans réindexation implicite.

## Fin attendue

Contrat commun et première tranche reviewables, puis autorisation et validation
du vrai parcours agent : lister l'arbre d'un fichier, choisir un membre, parcourir
ses parents/enfants et lire seulement ce qui est utile. Témoins de profondeur,
ambiguïté, membres partiels, fichier modifié/index périmé et troncature. La note
de conception ne ferme pas ce ticket et ne déclare aucune nouvelle API livrée.
