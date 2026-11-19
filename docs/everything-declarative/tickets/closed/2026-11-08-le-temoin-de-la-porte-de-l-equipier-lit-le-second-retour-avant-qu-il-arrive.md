# Le témoin de la porte de l'équipier lit le second retour avant qu'il arrive

- **État** : fermé le 19 novembre 2026 par la session tty : déjà corrigé par b0301a0e32 (8 novembre, 4 h 20), venu après la base vue rouge (624aaadc9, 3 h 50). Voir « Fermeture ».
- **Famille** : produit
- **Gravité** : faux rouge (une course du témoin).
- **Atteignable en service** : non (le témoin).
- **Touche rag3weaver** : oui (`src/daemon_host/teammate_tests.rs`).

## Ce que c'est

`daemon_host::teammate_tests::a_teammate_runs_commands_through_the_declared_gate_and_asks_for_the_rest`
attend la question d'accord dans la boîte du superviseur, puis lit `vus[1]`, le second retour d'outil
vu par le faux modèle (`teammate_tests.rs:819`). La question peut être rangée dans la boîte **avant**
que le faux modèle ait reçu ce second retour : `vus` n'a alors qu'une entrée, « index out of bounds:
the len is 1 but the index is 1 », et le fil `teammate-doc` panique ensuite sur le verrou empoisonné
(`teammate_tests.rs:782`).

## Les chiffres (le portable, lib 17a094d12, `MOTEUR_ANCIEN=1`, base 624aaadc9 + le lot du délai)

| où | résultat |
|---|---|
| la lib entière sous un faux service muet (`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7999`), 8 nov. ~4 h | 1 rouge (ce témoin) |
| la lib entière sans service, la même passe | vert |
| seul, sans service | 3/3 verts |
| seul, sous le service muet | 3/3 verts |

## Ce qu'il faut pour le fermer

Attendre, dans le témoin, que le faux modèle ait vu ses deux retours (`vus.len() >= 2`, par la même
attente bornée que la boîte) avant de les lire ; rouge d'abord sous charge.

## Fermeture (19 novembre 2026, session tty)

Le correctif n'a pas pris la forme proposée (`vus.len() >= 2`), mais il ferme la course. Depuis b0301a0e32 (« l'état d'attente »), le témoin n'attend plus la seule question d'accord. Il attend aussi la ligne de l'hôte « en attente du superviseur depuis … ». Cette ligne s'écrit dans `run_teammate` après le retour du graphe du pas (`daemon_host/teammate.rs`, « Un pas qui finit sur une question attend la réponse »). Le pas ne revient qu'après le `step_done` du faux modèle. Or le faux modèle ne rend ce `step_done` qu'à son appel qui porte deux retours d'outil, et il range le second dans `vus` avant de répondre. Quand le témoin lit `vus[1]`, il est donc toujours là.

La base vue rouge (624aaadc9, 3 h 50) précède b0301a0e32 (4 h 20) : le rouge du ticket est celui de l'ancienne attente. Depuis, aucun rouge de ce témoin n'est relevé dans les passes.
