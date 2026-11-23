# Un appel au modèle peut mettre une minute sans que la vue le dise

- **État** : ouvert, le 23 novembre 2026
- **Famille** : produit
- **Gravité** : moyenne : rare, mais l'utilisatrice voit l'agent « au repos » pendant près d'une minute et croit qu'il ne répond pas
- **Atteignable en service** : oui (une fois sur 80 premiers appels le 23 novembre)
- **Touche rag3weaver** : oui (l'appel du modèle dans `agent.rs`, la vue du terminal et de l'IDE)
- **Pour** : archi (le délai et la relance), tty et recherche (ce que la vue montre)
- **Signalé par** : l'essai de Lucie `~/essai-3`, le 23 novembre 2026 à 16 h 36

## Ce qui s'est passé

« bonjour! » à 16:36:15 ; le premier appel à `google/gemini-3.8-flash` (Vertex) a pris **54 473 ms**, sans relance,
pour 10 179 jetons d'entrée et 39 de sortie (`runs/9d2394394ab4`, `LlmCall.ms`). Le second appel du même tour : 2,1 s.
Pendant ce temps la vue est revenue à « YOUR MOVE ».

## La mesure de la journée

Tous les runs d'agent du 23 novembre sur l'hôte de Lucie (`~/.local/share/rag3weaver/runs/*/agent-*.json.zst`) :

| | n | médiane | p90 | max | > 30 s | > 45 s |
|---|---|---|---|---|---|---|
| premier appel d'un run | 80 | 2,1 s | 8,8 s | 54,5 s | 1 | 1 |
| appels suivants | 841 | 2,1 s | 7,6 s | 32,0 s | 3 | 0 |

C'est rare, mais une minute de silence suffit à faire croire à une panne.

## À faire

- La vue montre qu'un appel est en vol (son début, le modèle, le temps qui passe) et, passé un seuil déclaré
  (par exemple 20 s), le dit en clair : « le modèle met du temps (40 s)… ».
- L'hôte borne un appel sans premier morceau : passé un délai déclaré (par exemple 45 s), il l'annule et le relance
  une fois, et le dit ; la relance compte dans `retry_log`.
