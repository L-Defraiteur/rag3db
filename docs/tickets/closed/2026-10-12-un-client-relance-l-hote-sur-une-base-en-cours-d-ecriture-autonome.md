# Un client relance l'hôte sur une base en cours d'écriture autonome

*Ouvert le 12 octobre 2026 par l'arbre principal, à la demande de l'orchestration. Le
correctif vit dans le produit : `daemon_lifecycle.rs` (`ensure`), l'hôte (`daemon_host.rs`).*

## Le défaut

Pendant que codeparsers réindexe `code-rag3db` en autonome, l'hôte de Lucie est arrêté et un
binaire seul écrit la base. Tout appel d'outil d'un client mince MCP passe par `ensure`, qui ne
trouve pas d'hôte vivant et **en lance un**. Cet hôte ouvre la même base, alors qu'elle est en
cours d'écriture par un autre processus. Une base rag3db n'a qu'un écrivain : l'ouverture échoue,
ou pire, deux écrivains tiennent la même base.

On en est réduit à interdire à Lucie ses outils MCP pendant la réindexation, mémoire longue
comprise.

## La forme attendue

- **Une écriture autonome marque sa base** : un fichier `<base>.index-en-cours` avec le pid, l'heure
  et l'outil, posé à l'ouverture et retiré à la fermeture ; ou le verrou du moteur, lu avant
  l'ouverture. Un pid mort rend la marque caduque.
- **`ensure` la voit et refuse de lancer l'hôte**, en nommant la base et le pid ; le client mince le
  dit à l'agent (« code-rag3db : réindexation autonome en cours, pid N depuis H ; réessayer après »).
  Ou bien l'hôte démarre **sans ce backend**, et le dit dans `/etat` et `daemon_status`.
- Les autres backends de l'hôte (mémoire longue) restent servis.

## Le témoin

Un client appelle pendant une réindexation autonome. Il reçoit un refus nommé, et aucun hôte ne
naît (ou l'hôte naît sans ce backend, et `/etat` le dit).

## Suite

À l'arbre principal, après le lot L2 des lectures sans attente.

## Fermé (12 octobre 2026, arbre principal)

- **La marque** : toute ouverture d'une base en écriture pose `<base>.ecrivain` (pid, instant de
  démarrage, rôle `hôte` ou `autonome`, outil, depuis quand), retirée quand la base se ferme
  (`marque_d_ecrivain.rs`, posée par `Rag3dbConnection::open_for_writing`). L'hôte se déclare
  `hôte` à son démarrage ; tout autre écrivain est autonome. Un pid mort, ou repris par un autre
  processus, rend la marque caduque.
- **Le client** ne lance pas d'hôte quand un écrivain autonome vivant tient la base de son
  manifeste : `daemon_status` dit `refusé`, avec la base, le pid, la durée et l'outil, et quoi
  faire. Au départ de l'écrivain, l'appel suivant lance l'hôte et s'y attache.
- **L'hôte** refuse d'ouvrir un backend dont la base est tenue de même, et `/etat` garde la raison
  (`refused`) jusqu'à l'ouverture suivante ; ses autres backends restent servis.
- **Le témoin** : `e2e_mcp_cycle_de_vie::un_client_ne_lance_pas_d_hote_sur_une_base_tenue_par_un_ecrivain_autonome`,
  avec un vrai écrivain (`mcp --standalone`). Rouge sans le refus du client (l'état disait
  `absent`, « connexion à l'hôte en cours » : un hôte naissait), vert avec, la suite entière 10 sur 10.
