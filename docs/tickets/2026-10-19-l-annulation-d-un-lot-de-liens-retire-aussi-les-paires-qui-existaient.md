# L'annulation d'un lot de liens retire aussi les paires qui existaient avant lui

- **État** : ouvert (relevé le 19 octobre 2026 par la session mémoire longue, en écrivant le
  compte exact des liens neufs de la pièce 1)
- **Gravité** : perte : des arêtes posées avant le lot disparaissent quand le lot est défait
  par l'annulation du nœud (hors transaction).
- **Atteignable en service** : peu : seulement quand un drain hors transaction échoue après
  `LinkRecordNode` et que le runtime joue ses `undo` ; sous une transaction, le ROLLBACK du
  moteur défait juste ce qui a été écrit.
- **Touche rag3weaver** : oui, seulement.
- **Ouvert le** : 19 octobre 2026, session mémoire longue.

## Ce que c'est

`LinkRecordNode::undo` (`src/dataflow/record_nodes.rs`) supprime **toutes** les paires résolues
du lot, par `Write::Unlink`. Une paire déjà là avant le lot (reposée, sans effet) est donc
retirée par l'annulation, alors que le lot ne l'avait pas créée.

## Recette

Poser `a → b` ; dans un second drain hors transaction, reposer `a → b` et faire échouer un nœud
après `LinkRecordNode` : après l'annulation, `a → b` n'existe plus.

## Témoin

À écrire : la faute injectée (`Catalog::injecter_faute_d_insertion` sur un nœud aval) sur un
catalogue sans transaction, et le compte de la paire après coup.

## Cause

L'annulation ne sait pas ce qui a été créé. Depuis la pièce 1, le nœud le sait pour les
relations déclarées : le COPY rend les paires écrites, et le chemin par lots lit les paires
avant et après chaque tranche.

## Le pendant des déliaisons

L'annulation d'une déliaison (`Catalog::unlink`, clé `__retires` de l'undo) repose la paire
**sans ses propriétés** : un `CONSUMES` y perd sa `resolution`. Lire les propriétés avec les
paires avant de délier, et les reposer avec elles.

## Ce qu'il faut pour le fermer

N'annuler que les paires créées (les mêmes que le journal note), et reposer une déliaison
avec ses propriétés ; pour une relation non
déclarée (liens de morceaux), garder le comportement, ou faire la même lecture.
