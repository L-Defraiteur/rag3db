# Des écritures ne passent pas par le journal du catalogue et ne s'annoncent pas

- **État** : corrigé par le commit qui déplace ce ticket (19 octobre 2026, session mémoire longue). Le
  chemin du Plan (`Catalog::run_plan`), sans appelant en service, a son ticket à lui :
  `2026-10-19-un-plan-qui-ecrit-ne-s-annonce-pas.md`
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

## Ce qui a été fait (19 octobre 2026)

- **Le retour d'une mise de côté** (`undo_snapshot_finish`, branche des transitionnées) publie
  un `transitioned` par état rendu, de l'état de la transition déclarée à l'état d'avant, par
  `publier` (au commit sous une transaction). Les lignes retirées puis rendues passaient déjà par
  `ingest_entities`, qui annonce `ingested`. Témoin :
  `e2e_synchronisation::l_annulation_d_une_fin_qui_transitionne_s_annonce`.
- **La marque d'absence** (`mark_absent_since`) n'écrit que la colonne interne `_absent_since`.
  La transition qui l'accompagne passe par le drain et s'annonce déjà, avec ses états. Il n'y a
  rien de dû : une colonne `_` n'est pas une donnée de l'entité.
- **La reprise d'un drain** (`drain_resume`) recevait déjà le journal, mais ne le publiait pas.
  Plus grave, un drain tombé jetait les notes de ses nœuds terminés, alors que rien ne les défait :
  le point de reprise les garde, et la reprise les saute.
  - Le drain, la pose de donnée et l'ingestion publient maintenant ce que leurs nœuds ont écrit,
    **aussi à l'échec**, par `publier` : retenu sous une transaction, que son rollback jette.
  - La reprise publie les siens, réussie ou non.
  - Témoin : `e2e_catalogue_dit_tout::un_drain_tombe_puis_repris_dit_tout_ce_qu_il_a_ecrit`.
