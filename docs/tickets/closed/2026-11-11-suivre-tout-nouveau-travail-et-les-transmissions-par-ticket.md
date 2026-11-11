# Suivre tout nouveau travail et les transmissions par ticket

- **État** : fait b8f608c17 (11 novembre 2026), procédure écrite et consigne transmise aux treize autres sessions actuelles.
- **Famille** : produit
- **Gravité** : suivi ; une suite de travail peut se perdre lorsqu’elle reste seulement dans un message.
- **Atteignable en service** : non, règle de travail des sessions.
- **Touche rag3weaver** : oui, documentation et suivi des travaux du projet ; aucun runtime modifié.
- **Pour** : orchestration
- **Signalé par** : demande de Lucie du 11 novembre 2026, tickets pour tout nouveau travail et signalement entre sessions, classement dans closed à la fin ou à l’abandon.

## Travail attendu

Étendre la règle existante des défauts connus à toutes les actions identifiées : tâches,
capacités manquantes, suites de lot, conceptions à compléter et mesures. Le signalement
intersessions cite un ticket existant ou nouvellement ouvert, sans doublon.

Préciser le porteur, les preuves, les critères de fin, le transfert et la clôture dans les
documents de référence, puis transmettre la consigne aux sessions actuelles. Faire régulariser
les actions encore sans ticket lors des reprises, sans prétendre avoir audité tout l’historique.

## Pour fermer

Règle enregistrée dans `docs/README.md`, `docs/tickets/README.md`, la procédure de push et les
décisions de l’orchestration ; vérification des tickets ; consigne envoyée aux treize autres
sessions actuelles. Citer le commit et le résultat de la transmission à la clôture.

## Réalisation et preuve

Le commit `b8f608c17` met à jour les quatre références et ouvre aussi le ticket de la suite
archiviste M9, jusque-là portée seulement par sa conception. Le contrôle des tickets est vert :
353 en-têtes valides avant clôture.

La consigne de Lucie a été envoyée avec succès aux treize sessions : recherche, déclaratif,
architecture IDE, arbre principal, codeparsers, cœur C++, optimiseur, mémoire, lucivy, hotfixs,
banc, embarquement et tty. Les treize appels de transmission ont rendu sans erreur. La consigne
demande à chacune de relier ses actions en cours aux tickets et de régulariser les manquants
à sa reprise ; cela ne constitue pas un audit achevé de leurs travaux passés.
