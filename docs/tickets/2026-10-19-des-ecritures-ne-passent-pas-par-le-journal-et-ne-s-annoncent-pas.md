# Des écritures ne passent pas par le journal du catalogue et ne s'annoncent pas

- **État** : ouvert (relevé le 19 octobre 2026 par l'arbre principal, à la relecture de la
  pièce 1 « le catalogue dit tout » de la session mémoire longue)
- **Gravité** : réponse fausse en silence pour qui écoute : une réaction, le miroir des
  fichiers composés ou une page de l'IDE ne voient pas ces écritures. Ni perte ni blocage.
- **Atteignable en service** : oui pour le PlanNode, dès qu'un outil déclaré écrit par un plan ;
  rare pour les trois autres (reprise d'un drain, retour d'une mise de côté, fin d'instantané).
- **Touche rag3weaver** : oui, seulement.
- **Ouvert le** : 19 octobre 2026, session mémoire longue, à la demande de l'arbre principal.

## Ce que c'est

Depuis la pièce 1, chaque écriture du drain et de la pose de donnée est notée dans un journal
(`src/ecritures.rs`), et le catalogue la publie sur `catalog` au succès, au commit sous une
transaction (`Catalog::publier`). Quatre chemins écrivent sans passer par là :

- **le PlanNode** (`src/plan.rs:317`) ouvre son `BEGIN` sur la connexion, sans
  `set_in_transaction` et sans le catalogue : ses écritures ne publient rien, et une
  publication faite pendant son plan partirait avant son COMMIT ;
- **le retour d'une mise de côté** (`src/catalog/aside.rs`, `undo_snapshot_finish`) ;
- **la marque d'absence d'un instantané** (`src/catalog/synchronisation.rs:855`,
  `mark_absent_since`) ;
- **la reprise d'un drain** (`Catalog::drain_resume`) : son graphe ne reçoit pas le journal.

Le `Detach` de `code_sync` était le cinquième ; il publie depuis la pièce 1 (`removed`, en
état net avec ce que l'ingestion repose).

## Recette

Pas de Cypher : c'est l'absence d'un événement. Un plan déclaré qui écrit une ligne, et un
abonné à `catalog` qui ne reçoit rien.

## Témoin

À écrire, un par chemin, dans `tests/e2e_catalogue_dit_tout.rs` : l'écriture, puis
l'événement attendu (rien sous un ROLLBACK).

## Ce qu'il faut pour le fermer

Le PlanNode passe par le catalogue (`set_in_transaction`, `publier` au COMMIT, jeter au
ROLLBACK) ou reçoit le journal comme un drain ; les trois autres notent leurs lignes, et le
catalogue les publie comme celles d'un drain.
