# Les sources suivies : une abstraction pour les couches, pas trois modes

Synthèse de la discussion avec Lucie du 14 novembre 2026 (13 h 55 à 15 h 15), pour ED (2a′, 2b′) et
arbre principal (la liaison, 1b). Les décisions d'hier (page 30 : les cinq réponses, les branches
suivies, la rétention) restent ; cette page les généralise avant que 2a′-3 n'écrive une couche.

## Ce que Lucie demande

« Tout doit être paramétrable. Quelqu'un qui veut strictement garder tout son arbre git indexé doit
pouvoir ; quelqu'un qui veut juste plusieurs worktrees chargés en même temps doit pouvoir aussi ;
quelqu'un qui veut quelques worktrees plus telle et telle branche racine, par worktree, doit pouvoir
aussi. Il faut que ce soient trois paramètres d'un même truc générique qui sert à tout, pas trois
implémentations, juste des configs. Il faut une abstraction : même des trucs pas prévus doivent
marcher, pas trois modes stricts. » Et : « le parent peut être zéro code ; un scope commun à deux
états doit être détecté et affiché comme actif sur les deux ; pourquoi pas une déduction du parent
commun le plus récent depuis les branches actives ? »

## Le socle qui ne se règle pas : le partage par contenu

Depuis 1a (e62151904, 14 novembre), un scope est **une version par contenu** : une ligne, adressée par
le hash de son texte et de sa forme, citée par toutes les générations qui la contiennent. Deux
branches qui portent la même fonction, même sans aucun lien git, citent la même version ; elle
n'est embarquée qu'une fois et elle est « active » dans les deux couches par construction. L'IDE
montre pour une version la liste des couches qui la citent : « présente dans a et b, modifiée dans c ».
**Le parent git n'y est pour rien** : le partage se fait par le contenu, jamais par l'histoire.

## Le truc générique : une source suivie

Un dépôt déclare des **sources suivies**. Une source est **un sélecteur git** plus trois réglages.
Rien d'autre n'existe : ni mode, ni notion à part pour un worktree ou pour l'arbre entier.

- **Le sélecteur** : ce que git sait nommer — une branche, un motif de refs (`refs/heads/*`,
  `release/*`, `agent/*`), un worktree, un commit, une plage. Tout ce que `git for-each-ref` ou
  `git rev-parse` comprend est une source possible : un cas non prévu marche dès que git le nomme.
- **La parente** : *aucune* ; *inférée* (la source suivie dont le parent commun est le plus récent,
  marquée inférée) ; *déclarée* (par l'atelier à l'ouverture d'une tâche, ou dans le manifeste).
- **L'histoire** : *tous les commits* ; *les N derniers* ; *la pointe seule* ; *ceux qu'une fiche cite*.
- **La couche de travail** : le non-commité du worktree, *oui* ou *non*.

Tout le reste se dérive : chaque commit résolu est une génération ; chaque source est une couche ;
ce qu'aucune source ne sélectionne est « connu, non indexé » (le nœud existe, `view: <branche>@<commit>`
le matérialise à la demande) ; la rétention (30 Go par dépôt, fermeture à la fusion, bail sur une
génération épinglée) vaut par source ou par défaut.

## La racine : un réglage, pas une nécessité

- **vide** (« zéro code ») : une génération sans occurrence ; chaque couche se décrit par son ensemble
  complet de versions ; le partage entre couches reste entier puisqu'il passe par le contenu. Le cas le
  plus simple à raisonner ;
- **calculée** : le `merge-base` de toutes les sources suivies ; pas besoin de nommer main ; elle avance
  avec les sources (une génération de plus, les anciennes restent pour les lectures épinglées jusqu'à la
  rétention — ce que les lignes immuables de 2a′ permettent) ;
- **déclarée** : un commit ou une branche nommée.

Cas limites dits, jamais devinés : une seule source (sa racine est elle-même, l'histoire dit jusqu'où
on remonte) ; des historiques orphelins sans ancêtre commun (chaque source est sa propre racine, l'état
le dit).

Le merge-base n'apporte que deux choses : une **histoire** (retrouver un commit ancien sans tout
réindexer) et une **parente** inférée. C'est un confort pour qui pense en branches.

## Les cas d'usage, chacun une déclaration du même objet

| Ce que veut l'utilisateur | Sources | Parente | Histoire | Travail | Racine |
|---|---|---|---|---|---|
| tout son arbre git indexé | `refs/heads/*` | inférée | tous | non | calculée |
| plusieurs worktrees chargés ensemble | un par worktree | inférée | la pointe | oui | calculée |
| quelques worktrees + une branche racine par worktree | un par worktree | déclarée (`main`, `release/2`) | la pointe | oui | déclarée |
| les agents : chaque tâche de l'atelier | `agent/*` + main | déclarée à l'ouverture | la pointe + N | oui | `main` |
| une revue de deux branches sans lien | `a`, `b` | aucune | la pointe | non | vide |
| « tout `agent/*` à la pointe, main en entier » (non prévu) | `agent/*` ; `main` | inférée ; aucune | la pointe ; tous | oui ; non | calculée |

**Le défaut quand rien n'est déclaré** : les worktrees présents comme sources, la pointe plus la couche
de travail, parente inférée, racine calculée. `main` n'a de place particulière que déclarée.

## Ce qui est déjà là, ce qui se généralise

- **Déjà sur master** : les versions par contenu (1a) ; les « branches suivies » déclarées par dépôt
  dans `_catalog_meta`, la parente estampillée (déclarée → reflog → fourche), une tâche qui entre et
  sort des suivies (la liaison, 0672d624f et 912ea2084) ; le plan de 2b-B et la page 03 de 2a′ (les
  générations, les rangs de visibilité, l'ancre immuable).
- **À généraliser avant 2a′-3** (l'écriture d'une couche) : la déclaration — un **sélecteur** au lieu
  d'un nom de branche, les trois réglages par source, la racine en réglage ; l'inférence de la parente
  mesurée contre les sources suivies (le parent commun le plus récent) plutôt que contre une main
  supposée ; l'état des couches dans `/etat` et l'IDE qui dit la racine et sa nature (vide, calculée,
  déclarée) et, pour une version, les couches qui la citent.
- **Déclaratif** : tout vit dans le manifeste du backend et `_catalog_meta` avec des défauts ; aucun
  cas n'est codé à part ; un témoin par ligne du tableau ci-dessus, rouge d'abord, et un témoin pour
  le cas non prévu.

## Questions ouvertes

- Le nom dans le manifeste : `sources`, `suivies`, `couches` ? (Lucie.)
- La forme exacte des sélecteurs acceptés et de leur résolution (`for-each-ref` + worktrees + commits ;
  un sélecteur qui ne résout rien se dit, il ne devine pas).
- Ce que coûte une racine calculée qui avance souvent (une génération par mouvement du merge-base) :
  à mesurer avec 2a′-2.
