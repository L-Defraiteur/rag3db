# Le rappel dit « l'hôte ne répond pas » quand il attend l'embarquement

- **État** : corrigé (9 novembre 2026, branche rappel-dit-ce-qu-il-attend, hooks-essai), demandé
  par c7.
- **Famille** : produit
- **Gravité** : diagnostic trompeur. Le bloc des hooks accusait l'hôte, alors que l'hôte répondait
  et que la requête attendait un service d'embarquement saturé.
- **Atteignable en service** : oui. Le 8 novembre de 19 h 19 à 19 h 38 (granite 7878 à 12 s par
  texte), et le 9 à 12 h 13 (bge-m3 à 19 s par texte), chaque rappel des fiches de hooks-essai est
  tombé en « l'hôte http://127.0.0.1:7970 ne répond pas : timeout ».
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs`, `src/backend.rs`, `src/daemon_host.rs`,
  `src/dataflow/runtime.rs`, `src/claude_code_hook.rs`).

## Ce que c'était

Le hook borne `recall_fiches` à 1 s et donne 95 % à l'hôte (`deadline_ms`). À l'échéance, l'hôte
arrête le run, mais l'arrêt ne se lit qu'entre deux niveaux du graphe. Le nœud bloqué dans
l'embarquement de la requête (un appel HTTP bloquant, 60 s de délai) gardait donc la réponse
jusqu'à son retour. Le client du hook, à bout de budget, n'avait que « l'hôte ne répond pas ».

## Le correctif

- Sous l'échéance d'un appel, `DaemonEmbedder::embed` borne son attente au temps qui reste. À
  l'échéance, il note dans l'échéance « l'embarquement de la requête par <service> : N ms ». Hors
  d'un appel (une indexation), rien ne change.
- L'échéance (`CallDeadline`, avec un nouveau champ `waiting`) suit les nœuds sur les fils d'un
  niveau à plusieurs nœuds.
- La réponse 422 « deadline » porte `waiting_on`, et son texte le dit.
- Le hook écrit « Rappel des fiches : l'hôte attendait l'embarquement de la requête par … (budget
  1000 ms) », et non plus « l'hôte ne répond pas ».

Témoins, rouges sur 419cdb5f2, verts sur 24c8e39ae :
- `daemon::embeddings::tests::sous_une_echeance_l_embarquement_rend_la_main_et_dit_ce_qu_il_attendait`
  (un vrai démon dont l'embedder dort 3 s, une échéance de 400 ms : 4,28 s avant, moins de 1,5 s
  après) ;
- `claude_code_hook::tests::un_rappel_en_delai_dit_ce_que_l_hote_attendait`.

Ce qui n'est pas couvert : l'embedder compatible OpenAI (`HttpEmbedder`) ne lit pas l'échéance, et
les graphes à flux (`execute_concurrent`) ne la transmettent pas à leurs fils.
