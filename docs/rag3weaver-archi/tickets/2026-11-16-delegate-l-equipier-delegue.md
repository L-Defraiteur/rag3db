# delegate : l'équipier lance un sous-agent et lui parle

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : fonction manquante, voulue par Lucie (« j'ai pas l'impression qu'il sache lancer de sous-agent ni leur parler en temps réel »).
- **Atteignable en service** : non, à construire.
- **Touche rag3weaver** : oui
- **Signalé par** : c7 et Lucie, le 15 novembre 2026

## Ce qu'il faut faire

Suivre la conception acceptée par c7 : `docs/rag3weaver-archi/15-11-2026/10-conception-delegate.md`
(quatre outils `delegate`, `reply`, `stop`, `wait` ; branche `agent/<parent>--<nom>` ; les lignes du
sous-agent recopiées chez le parent en `from: "subagent"` avec `name` et `task` ; fusion ou
proposition ; bornes `teammate.delegation`). Les deux préalables vérifiés y sont écrits : la branche
d'une sous-tâche (`taches.rs:634`) et `parent_task` en dur à `null` (`daemon_host.rs:1645`).

## Comment le prouver

Les témoins rouges de la page, sur le faux modèle, en commençant par les préalables (une sous-tâche
prend `agent/<parent>--<nom>` et part de la tête du parent ; `parent_task` exposé). Passe comme le
plafond, plus `e2e_demon_hote`.

## Taille et blocage

3 jours, en trois lots (préalables ; outils et boîte ; fusion et bornes). Après S1 et S2 : un
sous-agent qui s'épuise en lecture n'apporte rien.

