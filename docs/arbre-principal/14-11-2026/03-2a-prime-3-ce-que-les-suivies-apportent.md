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

## Les réponses d'ED (14 novembre, 23 h 55)

ED écrira sa page de 2a′-3 après l'écrivain de 2a′-2, en partant de celle-ci. Ses réponses :

1. **Une seule bascule : oui**, sous trois conditions :
   - elle se répète d'abord sur les copies (code-lourd, code-rag3db-depot), avec la parité de banc au
     régime « migration » et la mesure par étage ;
   - son retour arrière est l'instantané lui-même, gardé jusqu'au mot de Lucie ;
   - elle n'a lieu qu'une fois fermé le ticket « une recherche à versions prend des Gio », qui tient déjà
     1b.
2. **Qui écrit une couche d'histoire : c'est `history` qui décide**, et non un choix global.
   - `tip` : l'hôte écrit la pointe à chaque mouvement de la ref, avec un amortisseur (une écriture par
     couche et par fenêtre de quelques secondes ; la dernière gagne).
   - `all` et `{last: N}` : la pointe tout de suite, puis le rattrapage en tâche de fond.
   - `cited` : rien tant qu'aucune vue, fiche ou couche ne la cite ; la citation déclenche l'écriture.
   - **Une source `refs:` (un motif large) prend `cited` par défaut**, et non `tip`. Sinon, cent
     branches coûteraient cent écritures à chaque fetch. **C'est un changement du défaut des suivies
     livrées** (aujourd'hui `tip` pour toute source) : il se fera rouge d'abord, en petit lot, quand la
     pile sera vide.
3. **Une suivie sans worktree qui avance de cent commits, sous `all`** : la pointe d'abord, puis un
   rattrapage paresseux.
   - Les générations intermédiaires s'écrivent en tâche de fond, à faible priorité, bornées par passe,
     chacune contre sa prédécesseure : le coût suit le diff, pas le commit.
   - L'état dit « k générations à rattraper ».
   - Une vue qui cite une génération pas encore écrite la fait passer devant.
4. **Ce que l'état dit à la place des recopies**, par génération : le nombre de chemins écrits, les
   lignes neuves et retirées, et la durée de l'écriture.

## La conception fine des deux pièces des suivies (15 novembre, 0 h 30, à la demande de c7)

### L'amortisseur de `tip`

- **Ce qui déclenche** : la résolution voit bouger l'empreinte des refs, et une couche dont la source dit
  `tip` a une pointe neuve. L'empreinte est surveillée par une montre sur `.git/refs` et `packed-refs`,
  pas seulement à la lecture de l'état.
- **Le groupement, sans seuil par minute** : l'écriture se prévoit à **5 s** après le dernier mouvement.
  Chaque mouvement neuf repousse l'échéance, et seule la dernière pointe s'écrit. Un plafond de **60 s**
  après le premier mouvement écrit quand même, pour qu'une ref qui bouge sans cesse (un rebase
  interactif, un script) ne repousse pas l'écriture à jamais.
- **Une seule écriture à la fois par couche**, en tâche de fond de l'hôte. Une pointe qui bouge pendant
  une écriture se met en attente ; elle remplace la précédente attente, elle ne s'y ajoute pas.
- **Ce que l'état dit** d'une couche, par couche : `pointe vue`, `génération écrite`, et `en attente
  depuis <t>, écriture prévue à <t'>`. Une vue sur la couche lit la génération écrite, et elle le dit (« en
  retard d'une génération sur la pointe »).

### Le rattrapage paresseux de `all` et de `{last: N}`

- **Le budget** : il ne tourne que quand la file de l'indexation courante est vide. Il fait au plus
  **5 générations ou 30 s par passe**, puis rend la main. Son embarquement des versions neuves passe
  sous le même régime GPU que l'index (`RAG3WEAVER_GPU_DUTY`), jamais au-dessus.
- **L'ordre : la plus récente d'abord**, en remontant de la pointe. Une vue a plus de chances de citer
  une génération récente. Chaque génération s'écrit contre sa voisine plus récente, déjà écrite : le coût
  suit le diff, comme le veut ED, quel que soit le sens.
- **`{last: N}`** s'arrête à N générations depuis la pointe. Les plus vieilles ne s'écrivent pas, sauf si
  une couche enfant les épingle ou si une citation les demande.
- **L'arrêt et la reprise** : le progrès est l'ensemble des générations écrites, gardé dans la fiche de la
  couche. Un hôte arrêté reprend, à la passe suivante, sur les générations qui manquent. L'id d'une
  génération est déterministe, donc une génération réécrite est la même, et rien ne se double.
- **Une citation d'une génération pas encore écrite** (`view: b@c`, une fiche, une couche) la fait passer
  devant le rattrapage, juste après l'indexation courante. L'appel qui la cite attend **au plus 5 s**. Au-
  delà, il répond par un refus transitoire nommé (« génération non écrite, écriture en cours, rang k
  dans la file ») : l'agent réessaie. Il n'y a jamais de réponse partielle mêlant deux générations. Le
  plein texte n'est pas servi avant le reste : une génération se publie entière ou pas du tout.

**Les deux gardes de c7 (0 h 2x)** :
- **Les nombres sont des réglages du manifeste**, avec ces valeurs pour défaut, et non des constantes
  (« tout doit être paramétrable ») : sous `tracked`, une clé `writes` (`{"settle_s": 5, "ceiling_s": 60,
  "catch_up": {"generations": 5, "seconds": 30}, "cited_wait_s": 5}`), relue comme les autres clés,
  avec un refus qui nomme toute clé inconnue.
- **Un seul fil de montre** : la montre des refs ne s'ouvre pas à côté du fil `index-suivi` de ce
  soir, qui regarde déjà le dépôt ; c'est ce même fil qui voit `.git/refs` et `packed-refs`. Si un dépôt
  n'a pas ce fil, l'état le dit, au lieu d'en ouvrir un second.

Rien ne se code avant la page de 2a′-3 d'ED.

## L'ordre du rattrapage contre l'identité des lignes (ED et arbre principal, 15 novembre, 0 h 45)

**Le point dur (ED)** : l'uuid d'une ligne est `hash(clé, couche, _depuis)`, et une ligne inchangée de g à
g+1 est une seule ligne, née au plus ancien rang. Si g+1 est écrite avant g, comme pour la pointe d'abord,
le rattrapage et la citation qui passe devant, il n'y a que deux issues : descendre `_depuis` (l'uuid change
et les arêtes perdent leur cible), ou poser un doublon né à g et retiré à g+1.

**Écarté** : (1), l'ordre chronologique dans le trou. Avec « la pointe d'abord », chaque génération
rattrapée tombe sur le bord haut, quel que soit l'ordre.

**À mesurer** (ED, avec la sonde de la clause sur la copie migrée), contre les deux colonnes seules :
- **(3a)**, l'uuid par l'empreinte du contenu, `hash(clé, couche, contenu)`, et une table de
  visibilité, une rangée par intervalle (une ligne qui revient à l'identique, A → B → A, a deux
  intervalles disjoints) ;
- **(3a′)**, la même chose, mais les deux colonnes portent l'intervalle principal, et une annexe ne porte
  que les réapparitions. Le prédicat devient « colonnes visibles OR EXISTS dans l'annexe » ; l'élagage de
  l'annexe vide est à prouver par le plan. Elle se mesure annexe vide et annexe à 1 % des lignes.

**Le défaut, tant que la mesure n'a pas parlé** (c7) : (2), le doublon au seul bord haut d'un trou,
compté dans l'état. Il est honnête : il coûte des lignes, jamais une identité.

**La mesure de (3a) se juge aussi à l'écriture** (c7) : une table d'intervalles, c'est une rangée de plus par
réapparition. Combien de réapparitions sur 50 commits de code-lourd ? Le nombre est à mettre à côté du
coût du prédicat.

**L'ordre (ED)** : l'écrivain de 2a′-2 sur tous les chemins d'écriture, puis la mesure, puis la page de
2a′-3 avec les chiffres.
