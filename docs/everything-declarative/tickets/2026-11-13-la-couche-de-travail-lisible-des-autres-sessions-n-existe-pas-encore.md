# La couche de travail lisible des autres sessions n'existe pas encore

- **État** : ouvert (13 novembre 2026) ; un lot à part, après la liaison branche → couche.
- **Famille** : produit
- **Gravité** : capacité manquante — la décision 3 de Lucie n'a pas de code
- **Atteignable en service** : non, rien ne la lit
- **Touche rag3weaver** : oui (à venir : la montre du worktree, la résolution des couches, le paramètre `view`)
- **Pour** : arbre principal (la liaison branche → couche)
- **Signalé par** : everything declarative, relecture de `arbre-principal/branches-suivies-2` (598eeb9a1), 13 novembre 2026 au soir, à la demande de c7 ; lignes de cette branche

## Ce que c'est

Lucie, décision 3 du 13 novembre (page 30, « Les décisions de Lucie ») : la couche de travail d'un
worktree, son non-commité, est **lisible des autres sessions**, sous le nom `travail`. C'est un
argument pour plusieurs agents qui codent en même temps et se parlent. La branche
`arbre-principal/branches-suivies-2` lie les branches suivies et les tâches aux couches, mais n'a
ni couche de travail, ni nom `travail`, ni lecture d'un autre worktree. Arbre principal la
prévoit en lot à part, et la page 30 le dira.

## Ce qu'il faudra tenir (relecture du 13 novembre, page 30)

- Hors du chargeur durable : la couche de travail vit dans l'hôte, par worktree, et se reconstruit
  depuis le disque après un redémarrage.
- **Figée par lecture** : une lecture prend, à son début, la génération épinglée et un instantané
  de la couche de travail (une map immuable échangée en entier à chaque réécriture, avec un
  compteur). Une réécriture pendant la lecture ne la touche pas : c'est le témoin cruel.
- Lisible des autres sessions par un `view` explicite (`<worktree>:travail`). La vue courante d'une
  session ne prend que la couche de travail de son propre worktree.

## Le témoin à écrire

- Une édition non commitée du worktree A se lit depuis une session B par le `view` de A, et pas
  dans la vue courante de B.
- Une lecture commencée avant une réécriture de la montre rend l'état d'avant, entier.

## Ce qui le fermerait

Le lot de la couche de travail, ses témoins verts, et la page 30 à jour.
