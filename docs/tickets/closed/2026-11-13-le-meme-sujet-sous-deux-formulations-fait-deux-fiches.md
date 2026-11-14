# Le même sujet, formulé autrement dans un autre document, fait une seconde fiche

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). La fusion des fiches proposées voisines déjà écrites reste à faire : c'est une seconde piste, voir plus bas.
- **Famille** : produit
- **Gravité** : réponse fausse. Un sujet se disperse sur plusieurs fiches voisines, et un rappel n'en rend qu'une partie.
- **Atteignable en service** : non, la mémoire de Lucie n'a pas encore l'outil `archive` (M8).
- **Touche rag3weaver** : oui, le gabarit `memory` : `archive.mmd`, `archive_subject.mmd` (nouveau), `archive_decided.rhai`, `archive_subject.rhai` (nouveau), `backend.json`.
- **Pour** : session mémoire ; l'orchestration (c7) l'a demandé pour la voie 3.
- **Signalé par** : [le second archivage à blanc](../../memoire/13-11-2026/03-le-second-archivage-a-blanc.md) (blanc-96-2).

## Ce qu'on voyait

Après `cd51ab5cd`, les sections d'un même document sur un même sujet se rangeaient sur une seule
fiche. D'un document à l'autre, en revanche, le même sujet faisait de nouveau une fiche dès que le
modèle le formulait autrement :

- « Hooks Claude Code », « Hooks Claude Code de rag3weaver » et « Hooks de Claude Code », de 11 à
  12 sections chacune ;
- quatre « Nettoyage des worktrees… » ;
- « Classeur local du rappel » et « Classeur de rappel ».

La recherche des candidates part du texte de la section. Elle ne retrouvait pas toujours la fiche du
même sujet, et un nom proposé n'était comparé aux candidates qu'à l'identique.

## Le correctif

- **Le voisinage de deux noms de sujet** (`sujets_voisins`). On compare les noms en minuscules, sans
  ponctuation et sans mots vides (de, du, des, la, le, les, l, d, et, à, au, aux, pour, sur, en, un,
  une). Deux noms sont voisins si leurs jeux de mots sont égaux, ou si l'un contient l'autre et que
  le plus petit compte au moins deux mots. « Archiviste » ne rejoint donc pas « Archiviste de la
  mémoire ».
- **Dans `archive_decided`.** Un sujet proposé qui est voisin d'une candidate, ou d'une fiche
  proposée plus haut dans le même document, la rejoint. Le nom exact est essayé d'abord, puis la
  première voisine dans l'ordre des candidates.
- **Un étage `archive_subject` après la décision** (`archive.mmd`). Chaque élément décidé cherche
  parmi les fiches (`search_structured`) par son nom de fiche, et non par sa première phrase. Pour
  une section qui créerait une fiche, la première fiche rendue qui porte le même nom, ou un nom
  voisin, devient la sienne (kind `fiche`). Les autres éléments passent tels quels.
  - Un `Route(key='stage')` ne laisse entrer que les éléments décidés. Les échecs du Decide restent
    dans son rapport.
  - L'étage tourne avant les écritures, car une écriture pendant une recherche la rendrait
    indisponible.
  - Il ferme aussi le reste du ticket clos
    [sur le nom du sujet](2026-11-13-une-fiche-creee-par-l-archiviste-porte-le-titre-de-sa-section.md) :
    un sujet qui nomme une fiche existante hors des candidates ne la réécrit plus par la voie
    create, pourvu que la recherche par nom la rende.

## Les témoins

- `e2e_archiviste::le_meme_sujet_sous_deux_formulations_fait_une_seule_fiche`. Deux documents :
  - le premier crée « Hooks Claude Code » ;
  - le second, sans aucune candidate (`candidates: 0`), répond create avec « Hooks de Claude Code ».

  Rouge sur master 2ac8080ca (un apport sur la fiche au lieu de deux), vert après.
- `e2e_archiviste::un_sujet_voisin_d_une_fiche_proposee_plus_haut_la_rejoint`. Trois sections d'un
  document : « Hooks Claude Code », puis « Hooks de Claude Code », qui la rejoint, puis « Hooks »,
  un seul mot, qui fait sa fiche. Rouge sans le voisinage d'`archive_decided`, vert avec.
- `e2e_archiviste` : 9 sur 9.

## Ce qui reste

- **La fusion des fiches proposées voisines déjà écrites.** C'est la seconde piste, qu'on n'a pas
  codée ce soir. Les doublons nés avant ce correctif restent séparés.
- **Le voisinage ne voit que les mots.** « Classeur local du rappel » et « Classeur de rappel » se
  rejoignent. Deux formulations sans mots communs, ou un nom au pluriel face au singulier, ne se
  rejoignent pas. Le troisième archivage à blanc (blanc-96-3) dira combien de doublons il reste.
- **Une fiche « Journal du jour » est trop large** : ticket à part, clos le 14 novembre,
  [un document journal fait une fiche trop large](2026-11-13-un-document-journal-fait-une-fiche-trop-large.md).
