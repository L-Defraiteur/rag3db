# Une réindexation de l'atelier en partie échouée se dit réussie

- **État** : corrigé (13 novembre 2026, branche atelier-dire-vrai, archi). Témoins
  `a_failed_reindex_says_it_failed`, `a_card_decided_during_its_own_turn_relaunches_after_it` et
  `a_decision_that_cannot_be_journaled_opens_nothing`, rouges puis verts.
- **Famille** : produit
- **Gravité** : réponse fausse — le modèle lit « réindexé » alors que des scopes ont échoué, et lit « sans index » sur un rendu non reconnu
- **Atteignable en service** : oui, à toute écriture dans un worktree attaché dont la réindexation échoue en partie
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : 491-499)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

Après `edit_file`, l'atelier lit le rendu de `EditResult::to_markdown`
(`code_tools.rs` 1356-1361), qui peut finir par `, F failed` : seuls « scopes upserted » et
« removed » sont lus (491-493). Un rendu non reconnu (vide, `result` non textuel) devient « sans
index » (499) au lieu d'une erreur.

## Le témoin à écrire

Une écriture dont la réindexation échoue sur un scope (un parseur qui refuse, injecté) : la réponse
dit l'échec et le nombre.

## Ce qui le fermerait

Lire le compte d'échecs et le dire ; un rendu non reconnu est une erreur dite ; le témoin vert.
