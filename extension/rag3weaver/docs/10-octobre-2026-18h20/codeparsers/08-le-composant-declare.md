# Le composant déclaré : son graphe de clic transpilé en TSX (marche 3, niveau 2)

*Étude de codeparsers, 18 octobre 2026 au soir, docs seules, à la demande de l'orchestration.
Suite de la page 07 (niveau 1 : le TSX comme donnée, fait jusqu'au moteur `script::tsx`, master
`748133ef3`). Sources : la vision générale (`visions/00-vision-generale.md:232-315, 458-469`), le
code de master lu par un repérage, citations en `fichier:ligne` relatives à `extension/rag3weaver/`.*

**Ce que dit la vision** (`:258-267`) : *« Un clic n'est pas une boucle, c'est un tour. »* Dans un
seul événement, le chemin est sans cycle : (l'événement, l'état d'avant) → l'état d'après → ce que
chaque composant reçoit. Trois sortes de nœuds : les **cases d'état** (la sélection, un filtre, la
page, la valeur d'un formulaire), les **nœuds purs** (une expression qui calcule l'état d'après ou
une donnée dérivée), les **nœuds d'effet** (appeler un contrôleur, changer de page). Un clic purement
d'écran ne traverse que les deux premières sortes. Le moteur WASM étant abandonné (8 octobre,
`:309-315`), le graphe ne s'exécute pas dans la page : **l'hôte le transpile en TSX**, que le niveau 1
compile.

> **Tranché par c7, à relire par Lucie** (18 octobre, 21 h 45, sur la consigne « la voie la plus
> générique ») :
>
> 1. **L'état** : chaque case déclare sa **portée**. `fenêtre` (dans la page, perdue au
>    rechargement : la sélection d'une vue), `session` (dans la page, rendue par l'hôte à la
>    reconnexion), ou `hôte` (tenue par l'hôte, partagée par toutes les fenêtres via `/ide/flux` :
>    un filtre commun, l'état d'une application). Par défaut `fenêtre`. La case `persiste` proposée
>    plus bas devient la portée `session`, et la règle de la vision (« la page n'a pas d'état que
>    l'hôte n'ait pas ») tient pour tout ce qui n'est pas `fenêtre`. L'état de présentation reste en
>    `useState`, hors des cases.
> 2. **Les droits d'écran** : dans le **manifeste**, à côté des autres politiques d'outil
>    (`ToolPolicy` étendue), pas dans un commentaire `%%` du `.mmd`. Un outil n'est appelable
>    depuis un écran que s'il le déclare (`ecran: lecture` ou `ecriture`). Un effet en écriture
>    passe par un nœud `Confirmer`. Le composant déclare ses `effets`, et le contrat se vérifie à la
>    déclaration : un effet vers un outil non appelable est refusé à `declare`.
>
> Les paragraphes ci-dessous gardent la proposition d'avant ; là où ils diffèrent, ce qui vaut est
> ce cadre.

## 1. La déclaration

**Où** : à côté du composant du niveau 1, un `components/<nom>.component.json` (props en ports
d'entrée, `on*` en ports de sortie, page 07 §2) et **son graphe de clic** `components/<nom>.clic.mmd`,
dans la syntaxe des graphes du backend (`src/dataflow/mermaid.rs:1-8` : `nom["Type(clé='valeur')"]`,
arêtes `a -->|port:port| b`, en-têtes `%%`). Même lecteur, mêmes lignes citées dans les erreurs,
même `to_mermaid` pour l'afficher (`:422`).

**Trois genres de nœuds d'écran**, nouveaux, refusés hors d'un graphe de clic :

- **`Etat(nom='filtre', schema='…', initial='…', portee='fenêtre')`** — une case (portée
  `fenêtre`, `session` ou `hôte`, tranché ci-dessus). Son port `valeur` en sortie (ce que
  les nœuds lisent), `suivante` en entrée (ce qu'un tour y écrit). Le schéma est un JSON Schema,
  comme les ports des nœuds scriptés (`src/dataflow/scripted_node.rs:10-13`). L'ensemble des cases
  est l'état du composant.
- **`Pur(expr='…')`** — une **expression TypeScript**, en ligne, sur ses ports d'entrée nommés :
  `Pur(expr='liste.filter(p => p.nom.includes(filtre))')`. C'est le « langage d'expression » que la
  vision laissait ouvert (`:281-283`) : TypeScript plutôt que rhai, puisque la page l'exécute. Rien
  dans le dataflow n'en tient lieu aujourd'hui : `RhaiNode` est le seul nœud d'expression en ligne
  (`src/dataflow/rhai_node.rs:30-50`), et il tourne dans l'hôte. Un nœud scripté
  (`nodes/*.node.json` + `.ts`, `run({inputs, config})`) peut aussi servir de nœud pur, s'il ne
  touche à rien d'autre que ses entrées : il est alors inliné tel quel dans le module généré.
- **`Effet(outil='search_code')`** — un appel `/ide/call` (`src/ide.rs:9-16, 304-314` : `{manifest,
  name, arguments}`), ses ports d'entrée devenant les `arguments`, ses ports de sortie la réponse et
  l'erreur. Et **`Page(vers='…')`** pour changer de page. Un effet est asynchrone : il termine le
  tour, et sa réponse **ouvre un tour nouveau** (l'événement « réponse de l'effet »). Chaque tour
  reste donc sans attente et sans cycle.

**Quels effets un écran a le droit d'appeler.** Aujourd'hui, rien ne le dit : la seule liste est
celle de la page de l'IDE (`src/ide/page.html:78-86`, vérifiée par le test
`the_page_only_calls_what_the_code_template_serves`, `src/ide.rs:429-460`). Et la politique des
outils (`ToolPolicy`, `src/backend_code.rs:219-237` : `read_files`, `write_files`, `run_commands`)
ne suffit pas : un outil sans politique peut écrire en base par les `BASE_NODES` (`:243-301`,
`EntityRecordNode`, `IndexNode`). La forme proposée :
- le composant **déclare ses effets** (`"effets": ["search_code", "save_product"]`), vérifiés à la
  transpilation contre les outils exposés du backend, avec leurs `%% param:` (comme le test de la
  page) ;
- un outil se déclare **appelable d'un écran** dans son en-tête (`%% ecran: lecture` ou
  `%% ecran: ecriture`), et un outil d'écriture n'est appelé qu'avec la confirmation que le graphe
  déclare (un nœud `Confirmer` avant l'effet) ; sans en-tête, un outil n'est pas appelable d'un
  écran.

**La limite de l'état de présentation** (`:277-280`) : un menu ouvert, un survol, un focus restent
dans le composant, **en TSX écrit à la main** (niveau 1), par un `useState` local. Le graphe ne porte
que l'état qui compte pour l'application. Une tension est à trancher par Lucie : la section de l'IDE
du 18 octobre dit « la page n'a pas d'état que l'hôte n'ait pas » (`:467-469`), alors que les cases
vivent dans la page. Proposition : les cases vivent dans la page (un tour s'exécute d'une traite,
sans aller-retour), et une case déclarée `persiste` est recopiée vers l'hôte par un effet implicite
en fin de tour ; c'est l'hôte qui la rend au rechargement.

## 2. La transpilation

Le graphe de clic devient **un module TSX généré**, qui entre par `compile_component` comme tout
composant (même registre, même React Refresh, même hash).

- **L'état** : toutes les cases en un `useReducer(reduire, initial)`. `initial` vient des `initial=`.
- **Un tour = une action** : chaque port de sortie d'événement (`onClick`, `onSaisie`…) et chaque
  réponse d'effet est un genre d'action `{ type, charge }`. Le réducteur évalue, pour ce genre, le
  sous-graphe sans cycle qui va de l'événement aux cases, **dans l'ordre topologique**, chaque `Pur`
  devenant une constante (`const n3 = liste.filter(…)`), et rend l'état d'après. Il est pur, donc
  rejouable : React peut l'appeler deux fois en mode strict sans dommage.
- **Les effets** sortent du réducteur : le tour rend aussi la liste des effets demandés (des
  valeurs), et un `useEffect` les lance après le rendu, par `fetch("/ide/call", …)`. La réponse est
  un `dispatch({ type: "effet:<nœud>", charge })`, donc un tour nouveau. Un effet obsolète (deux clics
  rapides) se reconnaît à son numéro de tour.
- **Le rendu** : le composant généré passe l'état et les données dérivées en props au composant du
  niveau 1 écrit à la main (la vue), et lui donne ses gestionnaires d'événements : *le graphe câble,
  le composant affiche* (`:241-249`).
- **La source map vers le graphe, pas vers le TSX généré** : le transpileur garde, pour chaque ligne
  qu'il écrit, le nœud du `.clic.mmd` d'où elle vient (fichier, ligne, nom du nœud), en une carte
  TSX → `.mmd`. `compile_component` rend la carte module → TSX. L'hôte **compose les deux** et sert
  la carte module → `.mmd` : une erreur levée dans `n3` montre la ligne du `Pur` dans le graphe. Les
  erreurs de transpilation (type d'une case, port inconnu, effet non déclaré) citent déjà la ligne
  du `.mmd`, comme le parseur des graphes.
- **Pas sur le disque** : le TSX généré est un produit en mémoire, servi par hash. Écrit pour le
  déboguer, il va sous `generated/` avec un en-tête `// @generated from components/<nom>.clic.mmd`.

## 3. Ce que l'IDE en montre

- **Le même afficheur pour un outil et pour un clic** : l'afficheur de A5 (`src/ide/page.html:337-408`)
  dessine aujourd'hui le « papillon » d'un symbole (`impact` et `callees`), pas un graphe de
  dataflow. Les graphes d'outils ne sortent qu'en mermaid, par l'atelier
  (`GET /_atelier/graph`, `src/atelier.rs:248-262`). Il faut **une route de l'IDE qui rende un graphe
  en JSON** (nœuds, genres, ports, arêtes, ligne du `.mmd`), pour un outil comme pour un graphe de
  clic, et l'afficheur de A5 généralisé à ce format (sa mise en page par niveaux se reprend).
- **Le tour en direct** : en développement, chaque tour publie sur `/ide/flux` (sujet `components`)
  son genre, les nœuds traversés et les cases changées. L'IDE surligne le chemin dans le graphe,
  comme `highlight=` le fait dans l'atelier.
- **L'agent modifie le graphe, pas le code** : il édite le `.clic.mmd` et le `.component.json` par
  `declare` (`src/declare.rs:74+` : valider, échanger, commiter), et le module se régénère. Le TSX
  généré n'est jamais une cible d'édition. Le TSX de la vue, écrit à la main, reste éditable comme au
  niveau 1.

## 4. L'indexation

- **Le composant déclaré s'indexe comme un gabarit** : son `.component.json` et son `.clic.mmd`
  entrent dans l'index comme du texte (`src/code.rs:815-859`). Il n'y a ni entité ni lien de gabarit
  dans l'index du code (`code.rs:37-47` : `File`, `Scope`, `Library`, `Symbol`). Un lot, plus tard :
  lire un `.mmd` comme du code, un scope par nœud, `CONSUMES` vers l'outil d'un `Effet` et vers le
  nœud scripté employé. `usages search_code` dirait alors quels écrans l'appellent.
- **Le TSX généré ne crée pas de faux usages** : il n'est pas écrit dans l'arbre ; et écrit pour
  déboguer, il est hors de l'index par la règle des fichiers générés (`src/generated.rs:30-77` :
  dossier `generated/`, marqueur `@generated` dans les 2 048 premiers octets). Le composant de vue,
  écrit à la main, est indexé comme du TSX : depuis `938c25e37`, `<Vue …/>` y est un usage.

## Les lots, dans l'ordre

1. **Les genres d'écran dans le lecteur de graphes** : `Etat` (avec sa portée), `Pur`, `Effet`,
   `Page` et `Confirmer`, validés seulement dans un `.clic.mmd`, avec les types des cases et des
   ports ; `ecran: lecture|ecriture` dans la `ToolPolicy` du manifeste et `effets` du composant,
   vérifiés à `declare`. Témoin : un graphe à cycle dans un tour, un effet non déclaré, un effet vers
   un outil sans `ecran`, une écriture sans `Confirmer` sont refusés à leur ligne.
2. **Le transpileur** graphe → TSX (réducteur, effets, rendu de la vue) avec sa carte vers le
   `.mmd`, puis `compile_component`, et la composition des cartes. Témoin : un compteur filtré dont
   le tour rend l'état attendu (exécuté dans QuickJS comme le témoin CommonJS de `748133ef3`), et une
   erreur dans un `Pur` située à sa ligne du `.mmd`.
3. **La route du graphe en JSON** et l'afficheur de A5 généralisé, puis le surlignage du tour.
4. **L'indexation des `.mmd`** comme du code (un scope par nœud, ses usages).

Le niveau 1 reste le chemin unique : le transpileur n'est qu'un producteur de TSX de plus. Les
deux points ouverts sont tranchés par c7 (en tête de page), à relire par Lucie.
