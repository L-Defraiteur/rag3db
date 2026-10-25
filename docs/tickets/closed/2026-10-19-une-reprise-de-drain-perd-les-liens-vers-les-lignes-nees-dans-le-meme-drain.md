# Une reprise de drain perd les liens vers les lignes nées dans le même drain

- **État** : corrigé par le commit qui déplace ce ticket (19 octobre 2026, session mémoire longue)
- **Gravité** : perte. Un lien disparaît sans un mot ; la reprise se dit réussie.
- **Atteignable en service** : oui, dès qu'un drain avec points de reprise (le défaut,
  `CheckpointMode::Full`) tombe après son nœud d'insertion et se reprend
  (`Catalog::drain_resume`). Le lien doit viser une ligne créée dans le même drain.
- **Touche rag3weaver** : oui, seulement. Le moteur n'y est pour rien : il exécute un MATCH qui
  ne trouve aucun nœud pour un uuid vide.
- **Ouvert le** : 19 octobre 2026, session mémoire longue. Trouvé par le témoin de la reprise du
  ticket « des écritures ne passent pas par le journal ».

## Ce que c'est

Le point de reprise enregistre les entrées du drain avant qu'aucun nœud ne tourne
(`runtime.rs`, la création de l'exécution). Un lien dont un bout est le ref d'une création du lot
n'y a que la clé de corrélation de ce bout (`from_temp_uuid`, `to_temp_uuid`). À la reprise,
`CheckpointRelationRecord::into_relation_record` rendait ce bout en `Uuid("")`. Le nœud de liens
cherchait un nœud d'uuid vide, n'en trouvait pas, et comptait la paire « sans bout ». Le nœud
d'insertion, terminé avant la chute, n'enregistre que son déclencheur : rien ne faisait
correspondre la clé à l'uuid. Celui-ci était pourtant dans le même point de reprise, dans la
donnée de la ligne (`_uuid`, posé à la mise en file).

## Recette

`create(A)`, `create(B)`, `link(R, refA, refB)`, `set_fail_node(Some("links"))`, `drain()`,
`set_fail_node(None)`, `drain_resume(exec)` : `MATCH ()-[r:R]->() RETURN count(r)` rend 0.

## Témoin

`tests/e2e_checkpoint.rs::une_reprise_garde_les_liens_vers_les_lignes_nees_dans_le_drain`, rouge
sans le correctif (0 lien), vert avec.

## Ce qui a été fait

- `checkpoint::correspondances_des_entites` lit, dans un lot d'entités enregistré, la clé de
  chaque ligne et son uuid (le ref résolu, sinon `_uuid`).
- La reprise (`runtime.rs`) relit les clés de toutes les entrées, puis les entrées elles-mêmes
  avec ces correspondances (`port_value_from_checkpoint_avec`).
- `into_relation_record_avec` rend le bout par son uuid. Une clé que rien ne nomme rend un bout
  en échec, que le nœud de liens compte et dit, au lieu d'un lien perdu en silence.
