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
