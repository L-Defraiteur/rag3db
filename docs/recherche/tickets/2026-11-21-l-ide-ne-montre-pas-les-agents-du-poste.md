# L'IDE ne montre pas les agents du poste, actifs ou non

- **État** : ouvert (21 novembre 2026, session recherche, scénario 1 de la nuit).
- **Gravité** : manque de la consigne de Lucie (« gestion de tous les agents actifs ou non »)
- **Atteignable en service** : oui (hôte de service de la tablette, fed132c964 rebâti, posé à 01 h 55).
- **Touche rag3weaver** : oui (`ui/ide`, l'état de l'hôte `/etat`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Le pas exact

Dans l'IDE de service, l'accueil liste les dépôts ; la barre du chat liste les sessions de chat par dépôt (`rag3db`,
`lucied`) et leurs tâches. Les agents Claude Code du poste (c7, G, tty, archi, recherche…), qui tiennent chacun une
session de l'hôte par le MCP, n'y paraissent nulle part. `/ide/etat` rend `sessions: []` alors que `daemon_status`, au
même instant, nomme ma propre session (`this_session`). Rien ne dit quel agent travaille, sur quoi, depuis quand, ni
ceux qui se sont arrêtés.

## Le contournement

Aucun dans l'IDE. `ListAgents` côté Claude Code, ou le journal de l'hôte (« session … fermée : son client … »).

## Pour le fermer

Une page « Agents » : chaque session de l'hôte (client, nom Claude, pid vivant ou non, dernière activité), ses
dossiers (table des dossiers), ses tâches et ses vues ; les sessions fermées, gardées un temps et marquées. Témoin
navigateur : une session MCP ouverte paraît, fermée elle passe « arrêtée ».
