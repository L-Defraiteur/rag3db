# Un pont navigateur généraliste pour les agents, par MCP

12 novembre 2026. Piste demandée par Lucie **pour plus tard** : regarder s'il
existe un équivalent open source de l'expérience d'une extension d'assistant
dans Chrome, utilisable par différents agents via MCP.
[Ticket de recherche](../../rag3weaver-archi/tickets/2026-11-12-etudier-un-pont-navigateur-ouvert-par-mcp.md).
Aucune solution évaluée, choisie, installée ou intégrée à ce stade.

## L'expérience recherchée

Depuis le chat de l'atelier, l'agent pourrait observer une page, comprendre son
état, naviguer et interagir avec elle, avec un retour lisible de ses actions.
Le besoin mérite d'abord d'examiner les projets existants : une extension de
navigateur pourrait servir de pont vers les onglets d'une session humaine ;
un navigateur piloté séparément pourrait couvrir d'autres usages. Ces deux
parcours doivent être distingués dans la comparaison.

Le contrat doit être généraliste : un serveur MCP réutilisable par plusieurs
clients agents, sans imposer un fournisseur de modèle. Si un projet convient,
composer ses capacités avec les outils de l'atelier avant d'écrire notre propre
extension ou un second moteur de navigation.

## Ce que la recherche devra vérifier

- Licence réellement open source du pont, de l'extension éventuelle et des
  composants nécessaires ; maintenance, versions et sources datées.
- Accès aux onglets existants ou lancement d'une session distincte ; comportement
  avec plusieurs fenêtres, profils, agents et connexions simultanées.
- Observation et actions disponibles : structure de page, captures, navigation,
  clic, saisie, attentes et erreurs explicites. Distinguer possibilités annoncées
  et outils effectivement accessibles par MCP.
- Exécution locale ou dépendance à un service tiers ; circulation des pages et
  des données de session. Le rattachement à un profil humain doit être un choix
  explicite, sans acquisition implicite de ses identifiants ou de ses onglets.
- Intégration aux accords, à l'annulation et aux journaux de l'atelier : politique
  commune pour les actions et leurs effets, sans second décideur caché dans un
  client. Relier cette étude à la [vision du mode auto](2026-11-12-mode-auto-contextuel-et-accords.md).

La première sortie attendue est une comparaison sourcée et une recommandation
de réutilisation ou de petit adaptateur. L'étude et un éventuel prototype seront
planifiés ensuite ; cette note garde l'idée, elle ne lance pas de nouveau service.
