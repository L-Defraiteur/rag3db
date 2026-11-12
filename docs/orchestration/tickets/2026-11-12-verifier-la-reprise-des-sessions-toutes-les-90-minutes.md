# Vérifier la reprise des sessions toutes les 90 minutes

- **État** : en cours ; installation locale demandée, vérification à terminer.
- **Famille** : produit
- **Gravité** : planification ; éviter une attente de coordination après une dépendance levée.
- **Atteignable en service** : oui, rappel de la session d’orchestration sur le poste local.
- **Touche rag3weaver** : non, outillage local des sessions de travail.
- **Pour** : orchestration.
- **Signalé par** : Lucie, demande explicite du 12 novembre 2026 dans la session d’orchestration.

## Demande

Toutes les 90 minutes, vérifier les sessions existantes et leurs tickets : activité réelle,
attente justifiée, dépendance levée, lot terminé. Relancer seulement un travail déjà autorisé
qui peut reprendre. Ne pas créer du travail pour occuper toutes les sessions. Un nouveau
travail ou signalement reprend la convention des tickets ; un simple contrôle de statut
n’ouvre pas de doublon.

## Mise en place attendue

Un timer systemd utilisateur local envoie un rappel à cette même session, avec la commande
installée `codex queue --thread … --message … --resume`. Aucun nouveau modèle, session ou
service applicatif du projet n’est nécessaire. Le timer doit être activé, son prochain
passage visible, et un envoi manuel vérifié. Le rappel utilise les permissions existantes.
La machine et le serveur local doivent rester disponibles ; ce n’est pas un minuteur cloud.

## Critères de fin

Fichiers de service/timer installés sans écraser un dispositif existant ; validation systemd,
activation utilisateur, prochain passage à 90 minutes et preuve d’envoi dans cette session.
Documenter commande d’arrêt, journaux et limites avant clôture dans docs/tickets/closed/.
