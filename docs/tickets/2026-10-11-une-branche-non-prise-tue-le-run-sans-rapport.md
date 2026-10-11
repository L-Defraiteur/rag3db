# Une branche non prise tue le run entier, sans rapport

- **État** : ouvert
- **Gravité** : blocage — le run entier meurt sur ce qui n'est pas une faute (P3, refus muet)
- **Atteignable en service** : oui, dès qu'un graphe aiguille (un producteur qui
  finit sans poser une sortie optionnelle, branchée sur une entrée requise) ;
  aucun graphe livré ne le fait aujourd'hui, et c'est justement ce qui manque
  pour brancher (les agents qui se parlent, lot 0)
- **Touche rag3weaver** : oui (`src/dataflow/runtime.rs`, `report.rs`)

## Ce que c'est

Un nœud dont une entrée **requise** ne reçoit rien — tous ses producteurs ont
fini sans émettre ce port — n'est ni joué ni sauté : la boucle de
l'exécuteur ne trouve plus aucun nœud prêt et rend
`Err("deadlock: nodes [...] cannot execute")`. Le run entier meurt, aucun
`ExecutionReport` n'est rendu (`execute_with_report_async` propage l'erreur
par `?`), aucun statut par nœud ne dit lequel attendait quoi. Une branche non
prise est donc indiscernable d'une panne.

## La recette

Un aiguillage qui ne pose que sa sortie `a`, une branche `b` à entrée requise,
une fusion à entrées optionnelles au bout : `execute_with_report` rend
« deadlock » au lieu de finir avec la seule branche prise.

## Le témoin

`dataflow::runtime::tests::une_branche_non_prise_est_sautee_et_dite`, en
direct (avec le rapport) et avec points de reprise.

## La cause

Le prédicat de disponibilité (`runtime.rs`, deux copies : direct et avec
points de reprise) n'a que deux issues pour une entrée requise sans donnée :
attendre, ou rien. Le rapport n'a pas de statut « sauté »
(`NodeStatus::{Completed, Failed, Resumed}`).

## Ce qu'il faut pour le fermer

Le saut : un nœud dont une entrée requise et branchée n'a rien reçu, ses
producteurs ayant fini ou été sautés, est **sauté** (`NodeStatus::Skipped`,
avec la raison), le saut se propage, et le run continue. Page :
`extension/rag3weaver/docs/11-octobre-2026-recherche/04-l-executeur-de-flux.md`, pas 2.
