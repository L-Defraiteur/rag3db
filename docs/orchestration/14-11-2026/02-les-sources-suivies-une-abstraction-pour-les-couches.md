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

- **Le sélecteur** : ce que git sait nommer — une branche, un motif de refs (`refs:refs/heads`, un préfixe : comme pour git, le joker `*` ne franchit pas `/`,
  `release/*`, `agent/*`), un worktree, un commit, une plage. Tout ce que `git for-each-ref` ou
  `git rev-parse` comprend est une source possible : un cas non prévu marche dès que git le nomme.
- **La parente** : *aucune* ; *inférée* (la source suivie dont le parent commun est le plus récent,
  marquée inférée) ; *déclarée* (par l'atelier à l'ouverture d'une tâche, ou dans le manifeste).
- **L'histoire** : *tous les commits* ; *les N derniers* ; *la pointe seule* ; *ceux qu'une fiche cite*.
  Par défaut la pointe seule, sauf pour une source `refs:` (un motif large) : *ceux qu'on cite*, sans quoi
  cent branches coûteraient cent écritures à chaque fetch (ED et c7, 14 nov., 23 h 55).
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
| tout son arbre git indexé | `refs:refs/heads` | inférée | tous | non | calculée |
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

## Après les relectures d'ED et d'arbre principal (15 h 30)

**Le nom : `tracked`** (arbre principal). Les clés du manifeste sont en anglais ; `source` est déjà pris
dans le schéma de code (`File.source` = `file` ou `snapshot:…`, dans la clé hashsafe ; `code_sync`
synchronise « une source »). La clé de `_catalog_meta` devient `tracked:<dépôt>` avec lecture de
l'ancienne ; en français dans les docs : « les suivies ». Lucie peut dire autrement.

**Précisions tenues** :

1. **`_couche` est l'identifiant d'une suivie**, pas « ancre » : pour les lignes d'aujourd'hui, la suivie
   qui couvre l'arbre indexé (le worktree du backend, sa couche de travail comprise). Une lecture sans
   vue est « la suivie courante de la session, à son rang en attente », l'arbre vivant comme avant (ED).
2. **Une suivie sans worktree s'indexe par les objets git** (`git cat-file --batch`, sans checkout) : la
   racine calculée, un motif `refs:refs/heads`, un `view: b@c`. Capacité neuve, avant ou dans 2a′-3 (ED).
3. **Une racine vide coûte N fois les occurrences** : les versions et les vecteurs restent partagés, mais
   les lignes `Scope` et les arêtes sont recopiées une fois par suivie (sur code-lourd : N × 82 855 lignes
   et 151 744 CONSUMES). Simple à raisonner, chère ; le défaut (racine calculée) l'évite (ED).
4. **La parente inférée garde l'unicité** : admise seulement si le parent commun le plus récent est
   unique, sinon « ambiguë », dite (ED, arbre principal).
5. **Le défaut n'est pas « tous les worktrees présents »** (le dépôt de Lucie en a des dizaines) : c'est
   **le worktree que le backend indexe, plus les tâches que l'atelier ouvre** ; les autres worktrees
   restent connus, non indexés, et un sélecteur de worktrees les prend à la demande (arbre principal).
6. **Une racine calculée suit la suivie la plus vieille** : une branche oubliée dans `refs:refs/heads` tire
   le merge-base des années en arrière et chaque couche grossit. L'état nomme la suivie qui fixe la
   racine ; le témoin de « tout son arbre git » la montre (arbre principal).
7. **L'identifiant du dépôt** ne peut plus venir de HEAD ni de l'ancre (racine vide, historiques
   orphelins) ; `--git-common-dir` n'est stable ni d'un poste à l'autre ni à un déplacement. **Décision
   c7 : la racine du plus vieux commit parmi toutes les suivies** (date d'auteur ; à égalité, les deux
   nommées) — stable entre postes, sans ancre. Lucie peut dire autrement.
8. **Un sélecteur à motif change d'ensemble** sans déclaration : la résolution se met en cache sous
   l'empreinte des refs (`for-each-ref` en un appel + mtime de `packed-refs` et du dossier `refs`), et ne
   se refait que si l'empreinte bouge — sinon `/etat` relance des dizaines d'appels git (le ticket
   O(suivies²)) (arbre principal).
9. **La parente inférée parmi les suivies est presque là** : la résolution actuelle prend la fourche parmi
   les branches nommées et l'ancre ; généraliser, c'est retirer l'ancre de l'ensemble quand la racine
   n'est pas déclarée. Témoin : deux suivies sans ancêtre commun ont la parente « aucune », dite (arbre
   principal).

Ordre : 2a′-1 et 2a′-2 (ED, rien ne change pour eux) ; 1b (arbre principal, la copie réelle migre) ;
puis la généralisation de la déclaration (arbre principal, ED relit) ; puis 2a′-3.

## Le flux multi-env (Lucie, 15 h 40) : develop / staging / main, hotfix sur main, resync main → develop

Pour **la base de partage**, la déduction ne pose pas de problème : après un hotfix sur main et un resync
vers develop, le parent commun de develop et de main est le commit du hotfix, récent ; la racine calculée
des trois avance à chaque resync (une génération de plus, l'ancienne reste pour les lectures épinglées) ;
et le hotfix est la même version par contenu dans les trois couches, embarquée une fois.

Pour **la parente au sens métier**, un flux multi-env n'est pas une arborescence mais un cycle dans le
temps (develop → staging → main, puis main → develop) : le parent commun le plus récent de staging est
develop après une promotion, main après un hotfix redescendu, selon le dernier événement. La déduction
dirait tantôt l'un tantôt l'autre, et « ambiguë » à égalité — c'est la vérité du graphe, pas une erreur.
La parente métier se **déclare** une fois dans le manifeste.

Ajout : **quand des suivies forment un cycle de fusions, l'état le dit** (« flux détecté entre develop,
staging et main : déclare leurs parentes ») au lieu d'une ambiguïté qui changerait au prochain resync.
Témoin : trois branches avec promotions et redescentes, l'état nomme le flux et ne pose aucune parente
inférée entre elles.

## Décisions de relecture (14 novembre, 22 h)

- **L'identifiant d'une couche est le nom complet de la ref** (`refs/heads/<x>`) ; les clés courtes d'aujourd'hui
  se relisent comme `refs/heads/<nom>` et se réécrivent, avec témoin, à la résolution qui les voit ; une
  étiquette homonyme ne doit rien faire perdre (témoin). Un renommage de branche est une couche qui finit et
  une qui commence, sur la même génération.
- **La couche de travail appartient au worktree**, pas à la branche : une par worktree, parente = la couche
  de la branche extraite ; un worktree détaché a pour parente une génération épinglée sur son commit ;
  nommée par son chemin. Une génération épinglée par une enfant vivante est gardée ; `cited` rend ses citants.
- **Le flux** : X → Y quand une fusion faite par Y, sur sa ligne de premier parent, apporte un commit fait
  par X sur la sienne ; une composante à plus d'une couche est un flux nommé, sans parente inférée.
  **Une couche dont la pointe est contenue dans la pointe d'une autre suivie ne compte pas dans un flux**
  (branche finie : une fonctionnalité qui a repris main puis que main a fusionnée) ; un flux exige deux
  couches qui ne se contiennent pas. Décision c7, que Lucie peut retourner.
- Fait au 22 h 2x : déclaration (défaut, refus nommés, ancienne clé), résolution des sélecteurs, racine
  (trois natures, la suivie qui la fixe), parente inférée (ambiguë, aucune, déclarée), flux. Reste : identifiant
  du dépôt et migration des clés, cache sous le hash des deux sorties, branchement à la place de
  `couches.rs` (relevé des lecteurs par grep d'abord, e2e_lectures au rejeu), témoins par ligne du tableau.
- 22 h 2x : la fonctionnalité finie (strictement contenue → aucun flux ; fusionnée → parente = la fusionnante,
  base = la fourche avec ce qu'elle était avant ; pointes égales → `Egales` sans avertissement) ; l'identifiant
  du dépôt = racine du plus vieux commit (date d'auteur) parmi les historiques suivis, à égalité la première
  par hash ; migration des clés `couche:`, `couche_fermee:`, `branches_suivies:` d'un identifiant à l'autre et
  du court au complet, une clé courte nomme toujours une branche, une cible posée n'est jamais écrasée.
  Relevé des lecteurs : seul `couches.rs` lit ces clés ; `_couche` des générations (colonne de Scope,
  `COUCHE_DU_BACKEND`) entre dans l'uuid des lignes neuves : son passage au nom complet se décide avec ED
  avant d'écrire (ni recalcul ni dédoublement des lignes posées).
- 22 h 2x, **`_couche` tranché à deux (arbre principal et ED), voie (b)** : une couche a un identifiant stable
  de naissance, distinct de son nom — hash(dépôt, nom complet, commit de naissance), ou chemin et commit pour
  une couche de travail —, gardé dans `_catalog_meta` avec l'index nom → id, recalculable, jamais réutilisé
  pour une autre lignée. `COUCHE_DU_BACKEND` reste : aucun uuid de 2a′-1 ne bouge. **Un renommage ne
  termine plus une couche** : la règle écrite plus haut est retirée. Ferme le ticket « le nom comme
  étiquette ». Le cache est vert (sans mouvement de refs : deux appels git, aucun merge-base ; 11 tests).
- 22 h 3x : l'identifiant stable des couches vert (12 tests) : même id d'une résolution à l'autre, `branch -m`
  le garde, une branche effacée reçoit sa fin et recréée ailleurs sous le même nom un id neuf ; le worktree du
  backend reçoit `backend`. Gardes : effacée puis recréée sur une pointe que main porte encore = fin et
  naissance, pas renommage (ED) ; deux vivantes ne partagent jamais un id, un id dérivé déjà pris prend le
  suivant (c7). Écarts à la proposition d'ED, acceptés : le commit de naissance est la pointe à la première
  vue, gardée dans la fiche ; préfixes `fiche_couche:` et `couche_nom:`. Reste : le branchement à la place de
  `couches.rs` (relevé des appelants d'abord), les témoins du tableau, la passe avec e2e_lectures.
- 22 h 4x : **les suivies sont branchées** (tête 2e41a4a11, 14 tests, check_features 8/8) : `tracked` dans le
  manifeste ; `index_states.layers` vient de `rapport_des_suivies` ; un dépôt git dit ses couches par le
  défaut sans rien déclarer ; l'atelier ouvre et ferme ses tâches par les suivies (11/11) ; le reflog local
  départage une parente ambiguë ; `couches.rs` n'est plus appelé. Le test par ligne du tableau a trouvé deux
  points : le joker de `refs:refs/heads/*` ne franchit pas `/` (tout l'arbre s'écrit `refs:refs/heads`,
  corrigé dans cette page) ; une couche sans candidate dont d'autres partagent la pointe se dit « égales ».
  Passe complète en file à luciepc (22 h 48, neuvième), verdict relu à 23 h 19 ; rang 30 bis dans la pile.
