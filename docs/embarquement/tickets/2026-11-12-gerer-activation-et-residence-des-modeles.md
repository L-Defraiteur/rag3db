# Gérer l'activation et la résidence des modèles selon les usages

- **État** : ouvert ; conception issue de l'audit du 12 novembre, mise en œuvre à planifier.
- **Famille** : produit
- **Gravité** : coût et disponibilité ; éviter l'accumulation de modèles résidents et permettre leur reprise à la demande.
- **Atteignable en service** : non pour cette gestion automatique ; lancement sondé et chauffe initiale existent séparément.
- **Touche rag3weaver** : oui, ModelSource, Serveur, fournisseurs de modèles et déclarations de ressources.
- **Pour** : embarquement et ED pour les contrats communs ; optimiseur pour l'activation distante et les unités.
- **Signalé par** : Lucie, demande de résidence pertinente et de warming ; audit des services résidents.
- **Ouvert le** : 12 novembre 2026

## Conception

[Vision](../../orchestration/visions/2026-11-12-residence-des-modeles-et-prechauffage.md)
et [audit](2026-11-12-revoir-les-services-d-embarquement-residents.md).
Composer ModelSource, Serveur/Attache et les unités existantes. Qualifier
résidence, activation partagée par modèle/profil/périphérique, baux d'usage,
éviction sans requête en cours, préchauffage anticipé et états observables.
La forme service actuelle joint un fournisseur ; elle ne l'active pas à distance.
Le groupe BGE/relecteur/OCR reste couplé au processus tant qu'une granularité
plus fine n'est pas implémentée et prouvée.

## Fin attendue

Première tranche revue puis réellement accessible aux appelants du modèle,
avec identités et espaces préservés, activation unique sous concurrence,
annulation isolée, budget et reprise à froid qualifiés aux deux postes.
Décrire chauffe partielle et erreurs par capacité sans les transformer en
réussite vide. Aucune nouvelle boucle d'agent nécessaire pour ce contrat de
ressources. La note seule et les arrêts manuels de l'audit ne ferment pas ce ticket.
