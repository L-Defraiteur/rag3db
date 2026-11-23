# Le témoin de la tranche archivée en fond lisait un fil de fond sans l'attendre

23 nov. 2026, signalé au pousseur (passe de `code-output-40k`, 12 h 49).

`daemon_host::chat::tests::a_slice_is_filed_in_the_background_and_the_mark_moves_after_the_commit` a rougi une fois à
`src/daemon_host/chat/tests.rs:2636` : `left: None`, `right: Some(("chat:…", 1, 1))`. Le témoin lisait `faux.slices`
aussitôt après le second `chat_turn`, alors qu'`archive_slice` est appelé par un fil de fond voulu sans attente.

Mesure, binaire de chaque tête joué avec son propre arbre, à luciepc :

| tête | seul | sous `yes` | 24 en parallèle × 5 |
|---|---|---|---|
| master 66df99358d | 30/30 verts | 10/10 | 120/120 |
| 527d4b383b (le lot des 40 000) | 30/30 | 10/10 | 120/120 |
| avec l'attente | 30/30 | 10/10 | 120/120 |

Aucun rouge reproduit en 160 lancers par tête. La course se lit dans le code : le témoin attend désormais l'appel de
l'archiviste, au plus 60 s, puis la marque, au plus 60 s ; il continue dès que l'appel arrive et échoue toujours s'il ne
vient jamais. Fermé par le lot `slice-wait`.
