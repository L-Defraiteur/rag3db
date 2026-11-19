# Le témoin de la porte de l'équipier lit le second retour avant qu'il arrive

- **État** : fermé (19 novembre 2026, ED) — déjà fermé par b0301a0e32 (8 nov., 03:37) : le témoin attend aussi la ligne « en attente du superviseur depuis », que l'hôte n'écrit qu'après que le modèle a reçu tous ses résultats. Le rouge du ticket avait été vu sur 624aaadc9 (8 nov., 02:14), qui ne l'a pas. **Preuve** : un retard de 300 ms injecté dans le faux modèle avant qu'il enregistre son second retour (une machine chargée, rendue déterministe) laisse le témoin vert ; le retard reste dans le témoin, comme garde.
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
