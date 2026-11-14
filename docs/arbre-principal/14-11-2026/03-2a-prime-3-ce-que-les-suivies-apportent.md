# 2a′-3 : ce que les suivies apportent, et ce qui se tranche avec ED

14 novembre 2026, 23 h 50, arbre principal. C'est le côté des suivies, pour la conception de 2a′-3 avec
ED : l'écriture d'une suivie par les objets git, où la génération devient l'arbre du commit, et où la
position sort de l'identité des lignes, en une seule migration (décision de 22 h 55, page 03 d'ED, § 9).
Aucun code avant la conception commune ; pas de poste lourd tant que la pile n'est pas vide (c7).

## Ce que les suivies donnent déjà (master 844d01247, `src/suivies.rs`)

- **Ce qui est suivi**, résolu en refs complètes et en couches de travail (une par worktree, nommée par
  son chemin), sous un cache qui ne bouge qu'avec les refs.
- **La pile** : la racine dans sa nature, la parente de chaque couche (déclarée, inférée parmi les
  suivies, égales, ambiguë, dans un flux, aucune) et sa base.
- **L'identifiant stable de chaque couche**, distinct de son nom, gardé par sa fiche
  (`fiche_couche:<dépôt>:<id>`) et son index (`couche_nom:<dépôt>:<nom>`). La couche de travail du
  worktree du backend reçoit `backend` (COUCHE_DU_BACKEND de 2a′-1).
- **L'histoire déclarée par source** (`tip`, `all`, `{last: N}`, `cited`), lue et résolue, mais servie
  par personne encore.

## Ce que 2a′-3 doit en faire (proposition, à trancher avec ED)

1. **Une couche d'histoire s'écrit par les objets git**, sans checkout : `git ls-tree -r <commit>` et
   `git cat-file --batch` pour les seuls chemins qui diffèrent de la base de la couche (sa parente, à son
   rang épinglé). La génération est l'arbre du commit. Ses lignes portent `_couche` = l'id de la couche,
   jamais son nom.
2. **La couche de travail reste au worktree**, au rang en attente (2a′-2, page 06 d'ED). L'arbre sale
   d'une génération publiée se dit (`arbre_propre: false` et le nombre de chemins sales).
3. **L'histoire de chaque source dit quelles générations se gardent** :
   - `tip` : la pointe seule, et toute génération qu'une couche enfant vivante épingle, toujours
     gardée (la règle d'ED) ;
   - `{last: N}` : les N dernières, plus les épinglées ;
   - `all` : tout ;
   - `cited` : ce qu'une fiche, une couche ou un appel épinglé cite, avec la liste des citants.

   La rétention applique ; la résolution dit.
4. **La position sort de l'identité** : l'uuid d'une ligne devient `[clé, couche, _depuis]` sans les
   lignes ni les octets. La position va dans un enregistrement par génération (proposition d'ED,
   page 03, § 2 bis). Les arêtes entrantes ne se recopient plus pour un scope seulement déplacé : les 86 %
   de la mesure des déplacés.

## Ce qui se tranche avant le code

- **Une seule migration pour les bases de Lucie.** Elles sont encore de la forme d'avant les versions.
  La bascule doit-elle les faire passer d'un coup par les trois formes (1b, les versions ; 2a′, le rang
  0 ; 2a′-3, la position hors de l'identité), sous un seul instantané et une seule fenêtre ? Ou bien en
  trois bascules ? Je penche pour une seule bascule, puisque chaque forme change de quoi se lit la base,
  et trois fenêtres sur les bases de Lucie coûtent trois fois le filet. La bascule attend que Lucie
  dise la durée de la validation.
- **Qui écrit une couche d'histoire** : l'hôte, à la résolution qui voit une pointe neuve d'une suivie
  avec histoire, ou un geste explicite (« indexer ce commit », `view: b@c`) ? Le premier suit les
  décisions de Lucie (une génération par commit indexé) ; le second borne le coût d'un motif large
  (`refs:refs/heads` sur un dépôt à cent branches).
- **Le coût d'une suivie sans worktree** : une branche qui avance de cent commits indexe-t-elle cent
  générations (`all`), ou la seule pointe (`tip`) ? La résolution donne l'histoire déclarée, il reste à
  dire qui la lit au moment d'écrire.
- **Le compte des recopies** dit à chaque publication (la règle de 22 h 55) disparaît avec la position
  hors de l'identité. Que dit l'état à la place ? Je propose le nombre de chemins écrits par génération.
