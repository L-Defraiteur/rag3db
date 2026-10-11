# Rapport de session — mémoire longue, 11 octobre 2026 (matin et midi)

*Pour Lucie et pour l'orchestration. Ce qui est sur master, ce qui reste, ce qui attend.*

## Sur master

**La page C, avec l'arbre principal** (`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
Mes parties : le client mince (§3), quand le démon n'est pas là (§4), ma part des
marches (§7), puis l'alignement sur tes treize décisions. Deux faits de la carte
portent ma partie : le protocole client ↔ backend existe déjà (le chat le parle, il
est donc déjà un client mince), et `mcp.rs` a déjà sa couture (`trait Hote`) — le
client de C est un autre `impl Hote`. Et un défaut que C rend atteignable, nommé et
réglé par ta décision 12 : la bascule de cellule du catalogue entier, devenue une
course dès que plusieurs clients partagent un backend.

**Le lien générique, quatre marches sur cinq** (`01-un-lien-entre-deux-identites-declarees.md`) :

| | commit | ce que c'est |
|---|---|---|
| témoin | `49142d58f` | la garde de `RelationBatchNode` ne protégeait aucune garantie déclarée — établi champ à champ |
| 1 | `6d212551e`, `cde65dc9d` | `RelationBatchNode` devient le nœud de lien général ; existence vérifiée au catalogue (base ou file) ; propriétés déclarées et typées ; `add_ref` ne prétend plus avoir lié une fiche absente |
| 2 | `13c6d950f` | `from_states` / `to_states`, une politique d'écriture du backend, appliquée par les deux nœuds qui lient |
| 3 | `a6c2a61dd` | `link.mmd` ; `anchor` et `supersede_link` dans le gabarit `memory` ; la réaction ancre par l'outil |
| 4 | `ad51a6630` | le huitième refus, lu sur la marque « relation lue / écrite » des paramètres |

**Ta mémoire branchée, rafraîchie** (`9734e75e0`) : neuf outils, et la chaîne entière
éprouvée par MCP sur une copie de ta base — ancrer, changer le sujet, la mémoire passe
« à revoir » d'elle-même. Ta base n'est pas touchée, l'ancien est gardé.

## Ce qui reste

- **Marche 1 bis** : que `Catalog::link` refuse lui-même un bout absent. Elle attend la
  traduction PostgreSQL du `Select` que rag3db-6f pose après l'échappement des noms
  (`link` est le verbe de tous les backends). Le ticket du bout absent se ferme avec elle.
- **L'enveloppe transactionnelle d'un outil d'écriture** : une marche à part. La forme
  légère de rag3db-97 : si la fermeture à drainer ne contient que des liens,
  `begin`/`commit` autour est sûr.
- **SIGTERM traité comme la fermeture du tube** dans `servir_mcp` : écrit au §4 de la
  page C comme une marche à part, à moi.

## Cinq tickets ouverts, trois fermés

Ouverts : le canal d'échecs d'une réaction sans lecteur ; un `put` sans le champ de
cycle de vie qui réécrit l'état initial ; une réaction qui écrit dans la cellule
basculée (se ferme à la marche 1 bis de C) ; un lien vers un bout absent qui rend
`Ok` (se ferme à la 1 bis du lien).
Fermés : le nœud qui ne relierait pas deux entités gérées (il n'en manquait pas) ;
l'outil d'ancrage absent du gabarit ; le huitième refus.

## Ce que la session a appris

- **Une prémisse se mesure avant d'être promise.** J'ai écrit « six lignes » pour
  l'ancre ; l'orchestration a dit oui sur ma parole ; un essai de cinq minutes a
  montré que le nœud refusait. Le bon ordre est l'essai, puis la promesse.
- **Une garde sans témoin se prouve avant de s'enlever, et la preuve suffit à la
  trancher.** Le témoin champ à champ a rendu la décision évidente pour tous les
  relecteurs.
- **Les relectures ont attrapé trois vrais défauts** que j'aurais livrés : drainer
  « ses seuls liens » (un lien perdu sans erreur), la paire donnée deux fois
  (perdue sans un mot), et l'affirmation fausse sur les transactions. C'est le prix
  d'une page avant le code, et il est bas.
