# Une fiche créée par l'archiviste porte le titre de sa section, pas le nom d'un sujet

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). Relevé par
  l'archivage à blanc des docs de la semaine (`docs/memoire/13-11-2026/02-l-archivage-a-blanc-des-docs.md`).
- **Famille** : produit
- **Gravité** : réponse fausse. « 1. Le jeu », « Le témoin », « En file » deviennent des fiches, et
  des sections sans rapport s'y rangent ensuite sur leur seul titre.
- **Atteignable en service** : non : la mémoire de Lucie n'a pas encore l'outil `archive` (M8).
- **Touche rag3weaver** : oui, `src/dataflow/decide_nodes.rs`, le gabarit `memory` (`archive.mmd`,
  `archive_decide.mmd`, `archive_candidates.rhai`, `archive_decided.rhai`).

## Ce qu'on voyait

Sur 326 fiches créées par l'archivage à blanc, 95 portaient un nom de moins de 25 caractères, le
titre de leur section ; « 1. Le jeu » a réuni deux pages du banc et une page de la précision du
rappel, qui parlent de jeux différents. Le nom d'une fiche créée était, par conception, le titre de
la section.

## Le correctif

- `Decide` reçoit une option `name_as='<champ>'` (voie llm) : le schéma demande `{label, name}`, et
  le nom est posé sur l'élément sous ce champ ; sans elle, rien ne change.
- L'archiviste la pose (`name_as='subject'`) et dit au modèle, dans son instruction, de donner dans
  `name` le nom du sujet en quelques mots, comme le titre d'une fiche de mémoire. Une fiche créée
  prend ce nom, sinon le titre de la section. Un sujet qui nomme une candidate, ou une fiche proposée
  plus haut dans le document, la rejoint au lieu d'en créer une homonyme.

## Le témoin

`e2e_archiviste::une_fiche_creee_porte_le_nom_du_sujet_que_le_modele_donne` : une section titrée
« 1. Le jeu », un faux modèle qui répond `create` et le nom « La reprise du journal ». Rouge sur
master acb4f02e0 (le schéma ne demandait pas de nom), vert après ; `e2e_archiviste` 7/7,
`decide_nodes` et `foreach_node` 37/37.

## Ce qui reste

Un sujet qui nomme une fiche existante hors des candidates écrirait cette fiche par la voie create.
L'écriture d'une fiche par son nom ne distingue pas encore « créer » de « remplacer ».
