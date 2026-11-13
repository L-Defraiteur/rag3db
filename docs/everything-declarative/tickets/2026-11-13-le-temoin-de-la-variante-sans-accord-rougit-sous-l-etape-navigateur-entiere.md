# Le témoin de la variante sans accord rougit sous l'étape navigateur entière, vert seul

- **État** : ouvert (13 novembre 2026), relevé par arbre principal pendant la passe du lot du raccord physique.
- **Pour** : ED (décision et reprise de commande, auteur du témoin `846829ba3`).
- **Famille** : produit
- **Gravité** : passe rouge sans cause dans le lot qui la joue ; une fois, sous charge.
- **Atteignable en service** : à départager ; vu seulement sous les 40 témoins navigateur en parallèle.
- **Touche rag3weaver** : oui (`src/daemon_host/chat_contextuel.rs`, `src/daemon_host/navigateur.rs`).

## Ce que c'est

`daemon_host::chat_contextuel::un_accord_exterieur_reprend_la_ligne_exacte_et_n_accorde_pas_sa_variante`
a rougi dans l'étape navigateur entière (`scripts/temoins_navigateur.sh`, 40 témoins). La passe est
celle de la branche `arbre-principal/ci-raccord-physique` (`fff456c08`), sur lucie-tablette, lib
`207701bb2`, sous `poste lourd` avec 32 Go au plus :

```
panicked at src/daemon_host/navigateur.rs:326:17:
le nouveau tour de variante reçoit son verdict : « document.querySelector('#status')?.textContent === 'fini' && document.querySelectorAll('details.outil[data-outil=run_command]').length === 3 » jamais vraie en 20 s (20 s de calme)
test result: FAILED. 39 passed; 1 failed
```

La vue, au moment du rouge, montrait la troisième carte `run_command` (la variante) encore en
attente de décision, avec « accorder » et « refuser », et aucune requête en vol.

Joué seul, sur la même branche, le même target et le même environnement, il est vert trois fois sur
trois, en 1,27 s.

Le lot de la passe ne touche que l'ouverture en écriture des bases (`rag3db_connection.rs`) et les
tests du raccord physique.

## Ce qu'il faut pour le fermer

Départager une attente trop courte sous charge d'un tour de variante qui ne reçoit vraiment jamais
son verdict. Pour cela, rejouer l'étape entière en gardant le journal de l'hôte du témoin.

## La cause (13 novembre 2026, soir)

Mesurée par everything declarative (`docs/everything-declarative/13-11-2026/02-mesure-de-l-etape-navigateur.md`) :
ce n'est pas une attente trop courte. Le tour était fini dans la seconde ; la page de chat ouvrait
une bulle fantôme sur un `RunStarted` arrivé après la réponse de son POST, et son statut ne passait
jamais à « fini ». Trouvé et corrigé par archi (`origin/chat-sans-bulle-fantome`, témoin
`a_late_run_started_of_a_finished_turn_opens_no_ghost_bubble`, rouge sur master puis vert). Ce
ticket se ferme quand ce lot est sur master.
