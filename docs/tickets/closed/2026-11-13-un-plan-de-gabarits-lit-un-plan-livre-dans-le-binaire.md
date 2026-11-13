# Un plan de gabarits lit un Plan livré dans le binaire, pas dans l'instantané fourni

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). Relevé par
  everything declarative en relisant la première tranche de M8.
- **Famille** : produit
- **Gravité** : réponse fausse en silence. Un plan de montée réussit sur un instantané auquel il
  manque un Plan, ou valide une autre version que celle fournie.
- **Atteignable en service** : non : la voie fermée de M8 n'a pas encore d'appelant en service.
- **Touche rag3weaver** : oui, `src/dataflow/plan_node.rs` (`lire_le_plan`), `src/dataflow/graph_tool.rs`.

## Ce qu'on voyait

`lire_le_plan` retirait lui-même le préfixe `builtin:` d'une citation `@builtin:…json` et lisait le
Plan dans le binaire (`builtin_templates::get`), sans passer par le lecteur que le chargement lui
donne. Dans la voie fermée (`lecteur::Contenus`, le plan de M8), un Plan absent de
`Package.builtin` réussissait, et une autre version fournie était ignorée. Le contrat de M8 dit :
« le contenu du binaire ne remplace pas silencieusement l'instantané fourni ».

## Le correctif

La citation entière va au lecteur. La voie ordinaire lit toujours le binaire
(`Lecteur::lire_reference`), la voie fermée l'instantané fourni. Les outils livrés
(`graph_tool.rs`), qui passaient un lecteur refusant tout, lisent eux-mêmes leurs Plans livrés.
Témoin : `dataflow::plan_node::tests::un_plan_livre_se_lit_par_le_lecteur_fourni`, rouge sur
master dd8111736 (« left: [] » : le lecteur ne reçoit rien), vert après.
