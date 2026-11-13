# Une section ne voit pas les fiches proposées plus haut dans son document

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). Relevé par
  l'archivage à blanc des docs de la semaine (`docs/memoire/13-11-2026/02-l-archivage-a-blanc-des-docs.md`).
- **Famille** : produit
- **Gravité** : réponse fausse. Un document de dix sections sur un même sujet donne jusqu'à dix
  fiches au lieu d'une.
- **Atteignable en service** : non : la mémoire de Lucie n'a pas encore l'outil `archive` (M8).
- **Touche rag3weaver** : oui, `src/dataflow/foreach_node.rs`, le gabarit `memory` (`archive.mmd`,
  `archive_decide.mmd`, `archive_candidates.rhai`, `archive_decided.rhai`).

## Ce qu'on voyait

L'archivage à blanc de 52 documents (432 sections, création allumée sur une copie) a créé 326
fiches, dont 274 à une seule section, et aucune qui réunisse deux sections d'un même document.
L'outil décide toutes les sections d'un document, puis écrit ; et ses sous-graphes écrivent dans une
seule transaction, que la recherche des candidates ne voit pas. Une section ne pouvait donc jamais
rejoindre la fiche que la précédente venait de proposer.

## Le correctif

- `ForEachNode` reçoit une option `carry='<paramètre>'` : un paramètre du graphe cité porté d'un
  élément au suivant ; le champ du même nom dans le résultat d'un appel devient sa valeur pour
  l'élément suivant. Elle exige `parallelism = 1`, et un paramètre que le graphe déclare.
- L'archiviste porte `proposed`, la liste des fiches proposées plus haut dans le document. Elles
  sont ajoutées aux candidates de chaque section (marquées « proposée plus haut dans ce document »).
  Une section qui en choisit une la rejoint par la voie `create`, avec le même enregistrement de
  fiche (une écriture identique), pour que la fiche existe avant le lien de son apport.

## Le témoin

`e2e_archiviste::deux_sections_d_un_document_sur_le_meme_sujet_font_une_seule_fiche` : deux
sections sur le même sujet, un faux modèle qui crée pour la première puis choisit `candidate_1`
pour la seconde. Rouge sur master 289ce7edc (« « candidate_1 » n'est pas une étiquette admise
(none, create) »), vert après ; la suite `e2e_archiviste` 6/6.

## Ce qui reste

Le nom d'une fiche créée est toujours le titre de sa section : « 1. Le jeu » ou « Le témoin » ne
sont pas des sujets. Une création qui nomme un sujet est l'étape suivante, à mesurer par un second
archivage à blanc.
