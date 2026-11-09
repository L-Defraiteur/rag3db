# Un oui pour une commande accordait toute sa famille

- **État** : corrigé (9 novembre 2026, branche atelier-commandes-sur-la-carte, archi), trouvé et
  fermé dans le même lot ; écrit à la demande de c7, pour qu'on le retrouve quand on parlera du
  décideur d'actions.
- **Famille** : produit
- **Gravité** : accord plus large que demandé. Un oui donné à une commande exacte laissait passer
  toute sa famille pour la session du backend : `mkdir faits` accordé, `mkdir n'importe-quoi`
  passait sans demander.
- **Atteignable en service** : oui, depuis `ad496c462` (8 novembre 2026). Il passait par le oui du
  superviseur à une commande en attente d'un équipier, dans une tâche dont le workspace déclare
  une porte (`workspace.commands`).
- **Touche rag3weaver** : oui (`src/commande.rs`, `Garde::accorder_ligne`).

## Ce que c'était

`Garde::accorder_ligne` se documentait ainsi : « chacune de ses commandes, telle quelle, est
retenue (`CetteCommande`) ». Mais il jugeait d'abord la ligne avec le contexte de l'appelant :

```rust
let v = self.juger_ligne(ligne, ctx)?;   // ctx.accorde_par_l_utilisateur == true
```

Dans ce contexte, `SentinelleDeBase::juger` rend `Autorise`, de portée `CetteFamille` et de
fondement `UtilisateurExplicite`. `borner` la plafonne à `Toujours`, et `Garde::juger` appelle
`acquis.retenir(c, &v)`, qui inscrit **la famille** dans les portées acquises. L'accord exact
(`accorder_commande`) venait ensuite, mais la famille était déjà retenue.

Il y avait deux appelants :
- le oui du superviseur dans la boîte d'un équipier (`daemon_host/teammate.rs`) ;
- la carte d'accord de l'atelier (lot 7, la branche qui l'a trouvé).

## Le témoin

- `commande::tests::un_oui_accorde_la_commande_exacte_pas_sa_famille` : après l'accord de
  `mkdir faits`, `mkdir autres` demande encore. Rouge avant le correctif (`Autorise`), vert après.
- `daemon_host::atelier_fichiers::tests::a_command_on_hold_in_the_workshop_chat_is_decided_on_the_permission_card`
  l'a trouvé : sa seconde commande ne montrait plus de carte.

## Le correctif

La ligne se juge **sans** l'accord de l'utilisateur, ce qui ne retient rien d'autre que ce qui
était déjà autorisé. Une partie refusée (élevée…) refuse la ligne, et rien n'est accordé. Seules
les commandes exactes sont retenues (`accorder_commande`).

L'effet sur l'équipier : une commande voisine de celle accordée (`cargo test -p y` après un oui
sur `cargo test -p x`) est redemandée au superviseur.
