# L'agent qui édite des gabarits, pas du code (marche 3)

*Étude de codeparsers, 18 octobre 2026 au soir, docs seules, à la demande de l'orchestration.
Sources : la vision générale, « L'atelier comme IDE des gabarits » (`visions/00-vision-generale.md:434-470`),
et le code de master lu par un repérage (`7bcd36e12`, puis `c5d3973e3` pour le branchement des
composants), citations relatives à `extension/rag3weaver/`.*

La vision le dit en une phrase (`:459-463`) : *un agent dont l'objet est le gabarit, pas le code —
il lit, propose, valide (la déclaration refuse ou recharge), et la page montre la différence.* Et
pour le code d'à côté (`:464-466`) : *édité par l'agent dans un worktree à lui, la différence relue
avant toute fusion.* La règle qui tient tout (`:468-470`) : *la page n'a pas d'état que l'hôte n'ait
pas.*

## 1. Les outils de l'agent : ce qui existe, ce qui manque

**Ce qui existe.**

- `Backend::declare(path, content, reason)` (`src/declare.rs:74`) : un fichier entier. Il écrit de
  façon atomique, recharge tout le backend (`check_reload`, puis `apply_reload_with_reactions`,
  `backend.rs:2602, 2543` ; version + 1) et commite (`git add`, `git commit` dans le dépôt du
  backend, `declare.rs:264-283`). Au moindre refus, il remet l'ancien contenu
  (`:94-115`). Un contenu identique ne fait rien.
- Les `DeclarationTools` (`declare.rs:288-335`) donnent à l'agent `list_declarations`,
  `read_declaration` et `declare`. L'atelier publie `declared` / `refused` sur son flux
  (`atelier.rs:396-419`), et sa montre recharge les éditions faites à la main (`:166-203`).

**Pour un `.mmd`, un `.component.json` ou un `.clic.mmd`, presque tout est là.** Les extensions
passent (`KINDS`, `declare.rs:33`), et le rechargement complet revalide `components/` par
`Components::load_in` (`backend.rs:949-957`) : les effets, l'`ecran` des outils et le graphe de clic
(`c5d3973e3`). Un graphe de clic mal écrit est donc refusé par `declare`, avec sa ligne.

**Ce qui manque**, du plus petit au plus structurant :

- **`tsx` et `jsx` dans `KINDS`.** La vue d'un composant ne se déclare pas (`components.rs:359`
  l'accepte, `declare` non).
- **Déclarer plusieurs fichiers d'un seul coup.** Un composant neuf, c'est son `.component.json`,
  sa vue, souvent son `.clic.mmd`, parfois `libs.json`. Fichier par fichier, la validation refuse
  l'ensemble à moitié écrit. Il faut un `declare_set(files, reason)`, qui valide le tout puis
  l'applique et le commite en une fois.
- **Des témoins** qui déclarent un composant et un graphe de clic, ce qu'aucun test ne fait
  aujourd'hui (`backend_reload_tests.rs`, autour de `:744`).
- **Le prompt de l'atelier** (`atelier.rs:42-47`), et la description des outils, à compléter des
  composants.

## 2. La boucle : proposer, valider, montrer, appliquer

Aujourd'hui, `declare` applique toujours : il n'existe pas d'état « proposé ». La seule
validation sans échange est `check_reload`, qui lit le disque : le fichier doit donc être écrit.

**La forme proposée, la plus générique : la proposition est une déclaration différée.**

- **`proposer(files, reason)`** : il valide **sur un recouvrement** (le dossier du backend, plus les
  fichiers proposés), sans rien écrire dans l'arbre. Concrètement, `PreparedBackend::load`
  lit déjà par chemins : on lui donne un `FileSource` en couches, le dossier réel sous les fichiers
  de la proposition, en mémoire. Il rend un verdict (accepté, ou refus avec chemin et ligne) et une
  **proposition gardée** par l'hôte : un id, les fichiers avec leur contenu d'avant et d'après, la
  raison, le verdict.
- **L'hôte la publie** sur `/ide/flux` (sujet `declarations`, événement `proposed`). La page
  montre :
  - la différence textuelle, fichier par fichier ;
  - pour un graphe, le graphe d'avant et le graphe d'après, par la route JSON des graphes (§4). Les
    nœuds ajoutés, retirés ou changés sont surlignés : c'est un diff de `GraphDefinition`, nœud à
    nœud et arête à arête, pas de texte ;
  - pour un composant à graphe de clic, le module régénéré se charge dans l'aperçu *sans être
    appliqué* : la proposition se joue avant d'exister.
- **Appliquer** : `appliquer(id)` refait la validation sur l'arbre du moment (il a pu bouger), puis
  il passe par le chemin de `declare_set`, avec écriture, rechargement et commit. **Rejeter** :
  `rejeter(id, pourquoi)`, gardé dans l'historique des propositions. L'agent voit le verdict et les
  rejets, et repropose.
- **Qui applique** : la page, au clic d'une personne, ou l'agent lui-même si le backend le permet
  (`declarations: "auto" | "approbation"`, sur le modèle de `workspace.commands`,
  `backend_code.rs:20-45`). Par défaut `approbation` : rien ne s'applique sans un humain.

La règle de la vision tient : la proposition est un état **de l'hôte**, la page ne fait que la
montrer et dire oui ou non.

## 3. Le worktree de l'agent et la relecture avant fusion

Aujourd'hui, le produit ne crée aucun worktree (`git worktree add` n'apparaît nulle part dans
`src/`), et `declare` commite droit sur la branche courante. Aucun outil de relecture ni de fusion
n'existe.

**La forme proposée** :

- **Une tâche, un worktree.** `ouvrir_une_tache(nom)` crée une branche `agent/<nom>` et son
  worktree, puis ouvre une **seconde instance du backend** dessus : sa racine, sa base de
  déclarations. La base de données reste partagée en lecture, ou elle est copiée par reflink si la
  tâche écrit. L'agent y déclare et y propose ; les commits de `declare` vont sur sa branche. Ce
  n'est rien d'autre que la règle des sessions de ce dépôt, un worktree à soi, appliquée à l'agent
  du produit.
- **La relecture** : l'IDE montre la différence de la branche contre sa base (les fichiers, et les
  graphes changés en diff de graphe, comme au §2), avec les témoins rejoués dans le worktree. Une
  personne fusionne en avance rapide après rebase, ou rejette.
- **Le code d'à côté** (`:464-466`) suit le même chemin. Le gabarit code garde son arbre principal
  en lecture seule (`code_tree.rs:120-128`) ; ce sont les outils `edit_file` / `run_command` qui
  ouvrent l'écriture, et dans le seul worktree de la tâche.

## 4. Ce que l'agent voit d'un graphe

Aujourd'hui, l'agent a `describe_backend` (`backend.rs:2005`) et le texte des fichiers
(`read_declaration`). Le graphe mermaid d'un outil n'est servi qu'à la page de l'atelier
(`/_atelier/graph`, `atelier.rs:249-264`), et aucune route ne rend un graphe en JSON.
`GraphDefinition` se sérialise déjà (`checkpoint.rs:303-328`, émis par `validate_search`), mais
`PortDef` ne se sérialise pas (`port.rs:178`).

**Le lot C1 du proto** (sa page 05 §5) y répond : une route, et un outil pour l'agent,
`graphe(outil | composant)`, qui rendent les nœuds (`node_type`, config telle qu'écrite, ligne du
`.mmd`), leurs ports (type, genre, posture, schéma) et les arêtes. L'agent lit alors un graphe
comme une structure, et la même forme sert au diff du §2. Les genres d'écran du niveau 2 y ont leur
place telle quelle (`node_type` = `Etat`, `Pur`, `Effet`… ; convenu avec le proto). L'agent y
gagne aussi la ligne des nœuds : il cite et corrige le `.mmd` à l'endroit exact que le refus de
`declare` désigne.

## Les lots, dans l'ordre

1. **`tsx`/`jsx` dans `KINDS`, et `declare_set`** (plusieurs fichiers, une validation, un commit),
   avec des témoins qui déclarent un composant et son graphe de clic. Petit, il débloque l'agent
   des composants.
2. **La route et l'outil du graphe en JSON** (C1 du proto), avec la ligne des nœuds.
3. **Les propositions** : la validation sur un recouvrement, la proposition gardée et publiée, le
   diff de graphe, `appliquer` et `rejeter`, et le mode `approbation` par défaut.
4. **Le worktree de tâche** : branche, worktree, seconde instance du backend, relecture et
   fusion dans l'IDE.

Le premier lot est pour codeparsers ou le proto, au choix de l'orchestration ; le deuxième est déjà
au proto ; les deux derniers demandent une décision de Lucie sur le mode par défaut
(`approbation`) et sur la base d'une tâche (partagée en lecture, ou copiée).
