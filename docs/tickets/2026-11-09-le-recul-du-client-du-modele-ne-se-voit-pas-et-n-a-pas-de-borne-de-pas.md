# Le recul du client du modèle ne se voit pas et n'a pas de borne de pas

- **État** : ouvert, le 9 novembre 2026. Relevé par la session mémoire (96) pendant l'essai de
  l'équipier sur les README ; pour everything declarative (l'équipier par pas).
- **Famille** : produit
- **Gravité** : une tâche réelle dort vingt minutes sans un mot ; le superviseur la croit morte.
- **Atteignable en service** : oui, par toute tâche confiée à l'équipier, et par le chat.
- **Touche rag3weaver** : oui, le client du modèle (la règle de reprise `llm-reprise-2`) et la
  boucle de l'équipier (`src/daemon_host/teammate.rs`).

## Ce qu'on voit

Tâche `readme-nouveautes-novembre`, backend 852ca016d10f, run `agent-1a1c02c5885-2122` : douze
appels à `gemini-3.8-flash`, dont trois avec des reprises.

| appel | durée | reprises |
|---|---|---|
| 5 | 97 s | 1 |
| 7 | 64 s | 1 |
| 8 | 1 004 s | 4 |

De 12 h 58 min 51 s à 13 h 18 min 20 s, l'équipier n'a écrit aucune ligne dans sa boîte, l'hôte
aucune ligne dans `daemon.log`, et le run n'existait pas encore sur le disque. Le fil
`teammate-readme` dormait en `hrtimer_nanosleep` : le recul du client, une minute puis le double.
Le superviseur ne pouvait pas distinguer un recul d'un appel pendu ou d'un outil qui attend l'index.

## Ce qu'il faut

- **Chaque recul se dit dans la boîte** au moment où il commence : « recul n° 3 du modèle,
  240 s, code 429 » (le code et le nombre de secondes), et une ligne au journal de l'hôte.
- **Le pas a un budget de temps** : au-delà, il échoue en le disant au lieu de dormir, comme
  `teammate.max_unavailable_steps` le fait déjà pour un refus qui remonte jusqu'à l'équipier.
- Témoin : un faux serveur qui rend trois 429 ; la boîte porte trois lignes de recul avec leur
  code ; un budget plus court que les reculs fait échouer le pas, nommé.
