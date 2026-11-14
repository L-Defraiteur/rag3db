# Une fiche créée par l'archiviste est décrite par un bout de phrase

- **État** : corrigé le 14 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96).
- **Famille** : produit
- **Gravité** : réponse fausse. L'`about` d'une fiche est ce que le rappel et le juge lisent d'elle ; « 1. » ou « Accepté par c7. » ne disent rien du sujet.
- **Atteignable en service** : oui, depuis la première passe sur la mémoire de Lucie (95 fiches créées).
- **Touche rag3weaver** : oui : `src/dataflow/decide_nodes.rs` (l'option `about_as`) et le gabarit `memory` (`archive.mmd`, `archive_decide.mmd`, `archive_decided.rhai`).
- **Pour** : session mémoire.
- **Signalé par** : [la première passe de l'archiviste](../../memoire/14-11-2026/01-la-premiere-passe-de-l-archiviste.md), § 5.

## Ce qu'on voyait

Une fiche créée prenait pour `about` le texte de sa section jusqu'au premier « . ». Une section qui
commence par une liste numérotée, une date ou un mot d'accord donnait « 1. », « Session archi,
9 novembre 2026. », « Accepté par c7. ».

## Le correctif

- **`Decide` reçoit une option `about_as='<champ>'`** (voie llm), sur le modèle de `name_as`. Le schéma
  demande aussi `about`, une phrase posée sur l'élément sous ce champ. Sans l'option, rien ne change.
- **L'archiviste la pose** (`about_as='subject_about'`). Son instruction demande, en create, une phrase
  complète d'au moins six mots qui dit ce que la fiche couvre, pas la première phrase de la section.
- **La garde** (`archive_decided.rhai`, `phrase_suffisante`) : une phrase de moins de six mots est
  écartée. La fiche prend alors la première phrase de la section qui en a au moins six, sans les
  titres, les tableaux, le code ni les marques de liste. Sans aucune, la création est refusée, le
  refus se dit dans le rapport et la section reste dans le L0.
- **Pas de contrôle de verbe** : sans étiqueteur grammatical, une règle de verbe en français serait
  fausse plus souvent que juste. Le seuil est le nombre de mots.

## Le témoin

`e2e_archiviste::une_fiche_creee_porte_une_phrase_qui_dit_son_sujet` : deux sections créent chacune
une fiche.

- La première reçoit du modèle une phrase de onze mots, qu'elle prend telle quelle (sans le
  correctif, « 1. »).
- La seconde reçoit « Accepté par c7. », écarté, et prend la phrase suivante de sa section.

Rouge sur master 8a1d05ecf (le schéma ne demandait pas `about`), vert après. `e2e_archiviste` :
13 sur 13. Les tests `decide_nodes::` et `foreach_node::` : 37 sur 37. La fixture des alias
(`preparer_alias`) passe d'une section de quatre mots à une phrase de huit : sa création aurait été
refusée par la garde, et ce test porte sur les alias, pas sur la phrase.
