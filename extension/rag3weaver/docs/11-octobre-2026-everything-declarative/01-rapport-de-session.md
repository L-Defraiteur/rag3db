# Everything declarative — rapport de session du 11 octobre 2026

Chantier ouvert à midi par l'orchestration, sur la demande de Lucie : « un
agent est un DAG, un DAG est un outil quand on en a besoin (dont les agents),
et les agents se parlent ». La conception est dans `03-les-agents-se-parlent.md`,
acceptée par l'orchestration. Ce qu'on sait du code est dans `02-knowledge-dump.md`.

## État

| Lot | Ce qu'il pose | État |
|---|---|---|
| 0 | les flux dans le moteur de graphes | **session recherche** : le pas 2 (une branche non prise est sautée, `NotTaken`) est sur master (`522fa6780`) et relu ici ; suivent `PortKind`/`Posture`, puis l'exécuteur concurrent |
| 1 | la boucle comme graphe à ports : les points d'accroche, la parité | **sur master** (`d3aa6f281`), relu par la recherche (accord, deux corrections faites) ; lib 1388/1388, toutes les cibles compilent |
| 2 | `decide` et `route`, posés sur un port | — (`route` peut s'écrire dès maintenant : une branche non prise = une sortie non posée) |
| 3 | `send` et l'annuaire | — |
| 4 | `AgentNode`, le parent, un agent appelé en attendant | — |
| 5 | `spawn`, rester à l'écoute, les bornes | — |
| 6 | la boîte de session dans l'atelier ; l'interface proposée au chat et au MCP | — |
| 7 | l'expérience à vrai modèle | — |

## Lot 1 — les ports d'agent

`src/agent_ports.rs` (nouveau) et des points d'accroche dans `src/agent.rs`.

- **Sept ports** : `user_in`, `peer_in`, `model_out`, `tool_call_out`,
  `tool_result_in`, `answer_out`, `turn_end`. Ce qui est branché sur les ports
  (`AgentPorts`, posé par `Agent::with_ports`) rend, pour chaque élément :
  l'élément (tel quel ou réécrit), rien (il est **retenu**), ou une panne
  (dite sur le bus ; l'élément passe inchangé).
- **Retenir** veut dire, selon le port :
  - `user_in`, `model_out`, `answer_out`, `turn_end` : le run s'arrête, sur
    `StopReason::Held { port }` ;
  - `peer_in` : le message est écarté ;
  - `tool_call_out` : l'appel reste annoncé, il ne s'exécute pas, et son
    résultat dit « retenu » ;
  - `tool_result_in` : le contenu dit « retenu ».
- **Les résultats d'outils en fond** arrivent par la boîte, mais passent par
  `tool_result_in`, pas par `peer_in` : ils sont reconnus à leur émetteur
  (`outil #<outil>-<n>`).
- **Ce que la trace dit** : `model_out` passe après le compteur, donc la
  trace dit ce qui a été payé. Un appel réécrit par `tool_call_out` garde son
  identifiant, et un avertissement dit ce qui s'est réellement exécuté.
  Quand `model_out` retire ou ajoute des appels, la raison de fin suit ; un
  identifiant vide ou en double est refusé en le disant.
- **`GraphPorts`** tient un graphe-outil par port (`%% param: element json!`).
  Un graphe qui ne pose pas son résultat, ou dont la branche n'est pas prise
  (`NoResult`, `NotTaken`), retient l'élément.
- **La parité** : sans port attaché, l'élément n'est pas même construit. Le
  témoin rejoue le même scénario sans ports et avec des ports transparents
  (un message de pair, un tour d'outil, une réponse) : même historique,
  mêmes jetons, même run.
- **Pas encore fait** : le premier flux (`model_out` en jetons) attend
  l'exécuteur concurrent du lot 0 ; aujourd'hui, `model_out` reçoit la sortie
  entière, un flux de un.

Témoins (`src/agent_ports/tests.rs`, 16) : la parité ; une panne qui laisse
passer ; retenir et réécrire à chaque port ; un appel réécrit dit sur le bus ;
des identifiants en double refusés ; un résultat en fond par `tool_result_in` ;
un graphe sur `peer_in` qui retient et réécrit ; une branche non prise qui
retient.

## Relectures faites pour la recherche

- La page de l'exécuteur de flux (`../11-octobre-2026-recherche/04-l-executeur-de-flux.md`).
- Le pas 2 du lot 0 : d'accord, avec trois remarques, toutes reprises (le saut
  en une seule méthode, le prix du saut écrit dans la page, le témoin
  `NotTaken` contre `NoResult`).

## Rouges vus en passant

- `daemon::embeddings::tests::un_gros_envoi_est_decoupe_en_lots` : « le démon
  n'a pas répondu » une fois, pendant la passe complète, sous charge. Le test
  attend 2 s qu'un démon lancé dans un fil réponde sur un `port_libre()`
  (port pris puis lâché). C'est la même famille que la course de `daemon::db`
  vue le matin. Ce code n'est pas le mien. Le test est passé aux deux passes
  complètes suivantes : il est intermittent, pas lié au lot 1.

Le chantier précédent (le proto « tout déclaratif », six lots) est rapporté
dans `../10-octobre-2026-18h20/everything-declarative/`.
