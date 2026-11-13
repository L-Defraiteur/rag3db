# L'identifiant d'un dépôt dépend de la branche extraite, et une couche est nommée par sa branche

- **État** : ouvert (13 novembre 2026).
- **Famille** : produit
- **Gravité** : réponse fausse, rare — le même dépôt sous deux identifiants, ou une couche qui hérite de l'estampille d'une autre
- **Atteignable en service** : oui, dès qu'un worktree du dépôt extrait une branche orpheline (gh-pages, un import d'historique), ou qu'une branche effacée renaît sous le même nom
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/couches.rs` : `identifiant_du_depot`, `cle_de_la_couche`, `cle_de_fermeture`)
- **Pour** : arbre principal (la liaison branche → couche)
- **Signalé par** : everything declarative, relecture de `arbre-principal/branches-suivies-2` (598eeb9a1), 13 novembre 2026 au soir, à la demande de c7 ; lignes de cette branche

## Ce que c'est

1. `identifiant_du_depot` lit `git rev-list --max-parents=0 HEAD` : les racines de l'histoire de
   **HEAD**. Un worktree sur une branche orpheline a une autre racine, donc un autre identifiant,
   pour le même dépôt. Ses branches suivies, ses estampilles et ses couches se rangent alors sous
   une autre clé de `_catalog_meta`.
2. Les estampilles et la marque de fermeture sont rangées par **nom** de branche
   (`couche:{dépôt}:{branche}`, `couche_fermee:{dépôt}:{branche}`). Un nom n'est qu'une étiquette :
   une branche effacée puis recréée sous le même nom (les tâches de l'atelier reviennent :
   `agent/doc`) hérite de ce que portait l'ancienne. Le lot de la liaison efface la marque de
   fermeture à l'ajout ; l'estampille de parente, elle, se réécrit seulement si l'atelier la redéclare.

## Le témoin à écrire

- Un dépôt avec une branche orpheline extraite dans un worktree : les deux worktrees donnent le même
  identifiant.
- Une branche suivie effacée puis recréée ailleurs sous le même nom, sans déclaration : sa parente se
  réinfère, l'ancienne estampille n'est pas reprise.

## Ce qui le fermerait

- L'identifiant par la racine de **l'ancre** déclarée, et non de HEAD : l'ancre est déclarée, et son
  histoire ne change pas d'un worktree à l'autre. `--all` changerait l'identifiant à la création
  d'une orpheline.
- Les clés d'une couche par branche **et** commit de création : la base estampillée, ou la première
  pointe vue. Une branche recréée est alors une autre couche.
- Les témoins verts.
