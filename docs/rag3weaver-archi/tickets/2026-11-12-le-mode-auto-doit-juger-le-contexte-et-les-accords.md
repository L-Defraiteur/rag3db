# Le mode auto doit juger le contexte et les accords

- **État** : ouvert ; conception et comparaison de projets autorisées le 12 novembre, aucun raccord de production autorisé par cette étude.
- **Famille** : produit
- **Gravité** : capacité manquante ; réduire les demandes répétées tout en distinguant les actes cohérents avec la demande et les engagements qui exigent un accord explicite.
- **Atteignable en service** : non pour le jugement contextuel proposé ; le mode Auto et sa sentinelle de règles existent déjà.
- **Touche rag3weaver** : oui, garde de commandes, outils, contexte utilisateur, acquis et parcours du chat atelier.
- **Pour** : ED et archi ; orchestration pour la conception et la comparaison initiales.
- **Signalé par** : Lucie, demande du 12 novembre de comparer notamment OpenCode et de reconnaître les autorisations déjà données pour des actions similaires.

## Demande et périmètre actuel

Étudier des sources primaires de projets d'agents de code, leurs règles, accords
mémorisés et éventuels juges contextuels. Relier la proposition aux primitives
existantes du projet : Mode::Auto, Sentinelle, Verdict, Portee, Fondement,
Autorisations, codeparsers et bac à sable. Distinguer documentation et code
observés, règle de motif et décision sur l'intention, proposition et livraison.

Définir les entrées et sorties du jugement, les preuves et limites d'un accord
réutilisable, sa durée et sa révocation, l'incertitude et le parcours utilisateur.
Une similarité de commande ne crée pas une autorisation ; le juge doit vérifier
ce que l'accord couvre. Une décision du modèle ne devient pas un consentement
humain ni un droit noyau supplémentaire.

## Critères de fin

La conception est une étape de ce ticket, pas sa fermeture : le raccord futur
devra être autorisé séparément, puis prouvé sur le vrai chat, y compris les
répétitions permises, les variantes hors portée, les actes non autorisés, les
changements de contexte et les pannes du juge. Aucune modification de politique,
permission ou modèle réel pendant cette étude. Garder les sources et les choix
ouverts dans la vision liée au ticket avant tout signalement d'implémentation.

## Conception et comparaison du 12 novembre

[Vision sourcée](../../orchestration/visions/2026-11-12-mode-auto-contextuel-et-accords.md) :
OpenCode pour les règles et accords par projet, Gemini CLI/Conseca pour le mandat
temporaire et le contrôle des appels, Cline pour les catégories et le marqueur
fourni par le modèle. Révisions exactes, limites de la comparaison, contrat
proposé, cas contrastifs et décisions restantes sont conservés dans la note.
Aucun code ou réglage de production modifié. Reste le raccord futur, après revue
et autorisation séparée ; la conception seule ne ferme pas ce ticket de capacité.

Relectures ED et archi intégrées : autorité des faits/fondements imposée par le
runtime, portée composée, identité du mandat et des reprises de carte, décision
idempotente, cartes périmées, révocation distincte de l'annulation, inférence hors
verrou et contrôle des données avant fournisseur. Les cas contrastifs et le
parcours distinguent manque d'accord, indisponibilité du juge et capacité absente.
L'hypothèse statique de publication concurrente a son ticket de qualification
séparé, sans test exécuté ni défaut déjà annoncé.
