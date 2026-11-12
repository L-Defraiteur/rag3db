# Vérifier la reprise des sessions toutes les 90 minutes

- **État** : fait ; timer utilisateur installé et activé, validation systemd et mise en file de test vérifiées ; premier passage périodique à observer.
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

## Réalisation

Installation le 12 novembre à 09:05 CET. Unités `rag3db-orchestration-watch.service` et
`.timer` sous ~/.config/systemd/user, activées sans dispositif antérieur écrasé.
Validation systemd code0, message de test 01a1ceca-0e7e-7471-94e1-cd7d978e1541 mis en file
pour la session existante ; premier passage annoncé à 10:35:35 CET. La mise en file est
vérifiée, pas l’exécution d’un premier tour périodique futur.
Documentation, contrôle, arrêt et limites dans
`docs/orchestration/12-11-2026/01-rappel-periodique-des-sessions.md`.
Ce commit documentaire clôt la demande d’installation ; l’activité des sessions se vérifie
à chaque rappel, elle ne se déduit pas du succès d’une commande de queue.
