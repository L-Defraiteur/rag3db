# Arbre principal — rapport de session, 10 octobre 2026 (pause pour le redémarrage du poste)

## Où en est chaque chose

**`defauts-bascules-2`** (`ac492c0dc`, poussée, **non fusionnée**) : la bascule des trois
défauts, rebasée sur `27eeb6a7f` sans conflit, dans le worktree
`~/.cache/rag3weaver-build/wt-defauts` (sous-module codeparsers initialisé).
- **Lucie a dit oui** (orchestration, 10 octobre) : la coexistence des bases en blobs, et
  les 8 à 10 s avant la première chose cherchable acceptées.
- Les tests du changement, joués sur la lib commune rebâtie, donnent :
  - la lib **verte** ;
  - les suites e2e vertes sauf `e2e_graphe_et_paquets`, rouge **pour une raison de
    worktree** : le test charge l'extension vecteur par un chemin relatif à son crate et
    ignore `RAG3DB_ROOT`. Ce n'est pas un défaut de la bascule, à rejouer depuis l'arbre
    principal ou à corriger dans le test.
- **Avant la fusion**, dans l'ordre :
  1. rebaser sur master (`2e67ce3e0`, DialectCapabilities, poussé par embarquement) ;
  2. ajouter l'accesseur `Catalog::dialect_capabilities()` ;
  3. la règle d'embarquement, convenue avec elle : la transaction par paquet demandée
     explicitement sur un dialecte sans `transactions` donne un refus nommé ; par
     défaut, pas de transaction et un avertissement nommé dans le rapport ;
  4. `catalog.rs:1557` passe sur `!capabilities().structured_fields` ;
  5. les tests du changement ;
  6. **la batterie complète une fois** (hors carte locale de jour, régime doux) ;
  7. prévenir embarquement, dont la série mesure l'ancien défaut ;
  8. l'avance rapide ;
  9. le README, et la page des défauts au présent.

**La lib commune** `build/lecteurs-csv` a été rebâtie à 10:46 sur `27eeb6a7f` (moteur
`fb98852e1`, avec `71cffbc4b`). Embarquement et rag3db-83 sont prévenus.

**La commande en fond** (chantier A, branche `commande-en-fond`, worktree
`~/.cache/rag3weaver-build/wt-fond`) :
- La page est faite et validée par l'orchestration :
  `01-la-commande-en-fond.md` (`14e0a8a48`, `83f2aed8a`).
- La session recherche prend la forme (b) et livre sur `execution-asynchrone` :
  `ctx.async_handle()`, un Sender vers la boîte, et un RunScope vidé avant de joindre
  les tâches, puis chat.rs. Elle enverra « prêt » avec les noms définitifs.
- `5080aeb06` : **WIP, non compilé**. Les parties indépendantes de la boucle sont
  écrites (groupe de processus, journaux en anneau sous le cache de la plateforme,
  `run` avec `background`, `tail`, cartes), avec leurs témoins **écrits, non joués**.
  La passe qui devait les compiler a été arrêtée pour la pause.
- `81ab7a2fd` : `landlock` pour Linux seulement (chantier G), **non vérifié par
  cargo check**. Il est sur cette branche, à pousser seul sur master après vérification.

## Ce qui reste, au redémarrage

1. Compiler `commande-en-fond` et jouer ses témoins (commande.rs : 5, run_nodes.rs : 4).
2. `cargo check --features code` sur le commit landlock, puis le pousser seul sur
   master.
3. La suite de `defauts-bascules-2` (ci-dessus).

Règle du poste depuis le 10 octobre : `mesure` seulement pour ce qui mesure, `lourd`
pour tout le reste, rebâtis compris.

Le worktree des docs `~/.cache/rag3weaver-build/wt-docs` porte une modification qui n'est
pas de moi (le rapport du banc de concurrence) : je n'y ai pas touché, et mes docs passent
désormais par `~/.cache/rag3weaver-build/wt-docs-2`.
