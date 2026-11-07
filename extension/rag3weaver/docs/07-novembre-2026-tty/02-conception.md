# Le terminal de l'agent de code : conception

7 novembre 2026, session « tty ». Cette page vient avant tout code. Elle est envoyée à c7 et à everything declarative. Le choix de la base revient à Lucie : la recommandation ci-dessous est la mienne, elle n'est pas encore tranchée.

## Ce que Lucie demande

- **Le streaming et ssh d'abord.** Les tokens doivent arriver au fil de l'eau, le redimensionnement doit suivre, ça doit marcher sous mosh et tmux, la latence doit rester basse, et rien ne doit dépendre d'un terminal local riche.
- **Le plus interactif possible**, ou en tout cas ne jamais être bloqué pour l'être. Elle veut de la 3D dans le terminal, une écriture déclarative « façon JSX », un look entièrement personnalisable, et un accès au bas niveau quand il le faut.
- **Rust, et une base MIT** qu'on peut réécrire entièrement s'il le faut.
- On ne garde de l'ancien que le design de chargement « Daemon Code Summoning » (page 01).

## Les bases étudiées

Les chiffres viennent de crates.io (licence, dernière publication, téléchargements sur 90 jours, dépendances directes non optionnelles), relevés le 7 nov. 2026.

| Base | Licence | Dernière publication | 90 jours | Modèle | Pour nous |
|---|---|---|---|---|---|
| **ratatui 0.30.2** (+ `ratatui-core`, des backends séparés) | MIT | 19 juin 2026 | 25,9 M | immédiat : on redessine un `Buffer` de cellules, et seule la différence part vers le terminal | **la base recommandée** |
| iocraft 0.9.3 | MIT/Apache | 14 oct. 2026 | 55 k | déclaratif : macro `element!`, `#[component]`, hooks, flexbox par taffy (inspiré d'Ink et de Dioxus) | le « JSX » existe, mais l'écosystème est maigre (pas d'effets, pas d'images, pas de 3D) ; on lui prend l'idée, on ne le prend pas comme base |
| termion 4.0.6 | MIT | 21 nov. 2025 | — | primitives seulement : termios, touches, ANSI | Unix seulement, 2 132 lignes, `libc` et `numtoa` ; devient le **backend** possible sous ratatui (`ratatui-termion`) |
| OpenTUI (sst) | MIT | actif | — | cœur Zig, liaisons TypeScript, React ou Solid, flexbox par Yoga, et un rendu three.js | le plus spectaculaire (c'est le moteur d'opencode), mais c'est du TypeScript sur Bun et non du Rust ; on s'en inspire |
| dioxus-tui / plasmo | MIT/Apache | févr. 2024 | 270 | déclaratif, rsx! | abandonné |
| cursive 0.21 | MIT | août 2024 | 468 k | widgets retenus | figé, et mal adapté au flux de tokens |
| r3bl_tui 0.7 | Apache-2.0 | janv. 2026 | 39 k | — | pas MIT, une cinquantaine de dépendances |
| notcurses 3.6 | MIT/Apache | oct. 2024 | 303 | bibliothèque C, pixels et médias | dépend d'une lib C, liaison Rust peu suivie |

### Ce que ratatui ouvre à côté

| Crate | Licence | Ce qu'elle apporte |
|---|---|---|
| tachyonfx 0.25 | MIT | des effets façon « shaders » sur les cellules : fondus, balayages, dissolutions, teintes, effets composables, un DSL |
| ratatui-image 11.1 | MIT | des images par les protocoles kitty, sixel ou iTerm2, et des demi-blocs quand rien de cela n'est disponible |
| bevy_ratatui_camera 0.16 | MIT/Apache | une scène Bevy 3D rendue dans un widget ratatui (demi-blocs, braille, détection de contours, tampon de profondeur) : **la 3D dans le terminal** |
| terminal3d 0.1 | MIT | un visualiseur `.obj` en braille ou en blocs, petit, bon exemple de rastériseur à lire |
| taffy 0.15 | MIT | un moteur flexbox et grid **sans aucune dépendance** : c'est lui qui rend possible une couche déclarative à nous |

## La recommandation : ratatui, avec notre couche déclarative par-dessus

1. **On ne sera jamais bloqué.** Chaque cellule du `Buffer` s'écrit directement (glyphe, couleur fg/bg, modificateurs), un widget n'est qu'un `fn render(area, buf)`, et le trait `Backend` est court : on peut écrire notre propre backend sur termios si on le veut un jour. C'est ça, l'accès bas niveau, et on peut tout refaire.
2. **Le haut niveau personnalisable existe déjà.** Les effets passent par tachyonfx, les images par ratatui-image, la 3D par bevy_ratatui_camera, ou par notre propre rastériseur en demi-blocs sur un `Buffer`.
3. **Pour le « JSX ».** Une macro `view!`, des composants et un arbre mis en page par taffy, rendus en widgets ratatui. iocraft prouve que la forme marche en Rust ; nous la posons sur ratatui pour garder tout l'écosystème. Ce n'est pas le premier lot, mais rien ne l'empêche.
4. **ssh.**
   - ratatui ne transmet que les cellules qui changent.
   - Le mode `Viewport::Inline` laisse le texte validé couler dans l'historique du terminal par `insert_before`, et redessine seulement une zone vivante en bas (saisie et ligne démoniaque), comme le fait Claude Code. Le défilement, la copie et tmux continuent de marcher.
   - Un plein écran (alternate screen, panneaux) reste possible en option plus tard.
5. **Les dépendances.** ratatui avec le backend crossterm compte une vingtaine de crates. Avec `ratatui-termion`, on garde le minimum. On décide au lot 1, en mesurant la taille du binaire et le temps de compilation.

## Ce que permet chaque lien

| Lien | Couleur vraie | Images kitty/sixel | Remarque |
|---|---|---|---|
| terminal local | oui | selon le terminal | — |
| ssh nu | oui | oui | une image coûte beaucoup en bande passante |
| tmux | oui (`Tc`/`RGB`) | sixel natif ; kitty avec `allow-passthrough` (3.3+) | — |
| mosh | oui à partir de 1.4, des deux côtés | **non** : mosh ne synchronise que l'état des cellules | — |

**Règle :** tout ce qui compte se dessine en cellules (texte, demi-blocs `▀`, braille). Les pixels sont un bonus, détecté au démarrage et jamais requis. La 3D de base est en demi-blocs ou en braille, donc elle passe aussi sous mosh.

## Le flux depuis l'hôte (relevé du code, 7 nov.)

- **Auth.** Le jeton se lit dans `~/.local/share/rag3weaver/daemon.json`, puis `Authorization: Bearer`. L'hôte n'écoute que sur 127.0.0.1:7970.
- **Où tourne le TUI.** Il tourne sur la machine de l'hôte, et ssh ne transporte que le terminal (`ssh luciepc rag3weaver tty`). Rien à exposer ni à tunneller. Un `ssh -L 7970:…` reste possible pour un client lancé à distance.
- **Flux poussé.** `GET /ide/flux?sujets=agent,tokens,declarations` en SSE, avec reprise par `Last-Event-ID` ou `depuis`. Un battement part toutes les 15 s. L'événement `lacune` signale qu'on a dépassé le tampon de 2048 événements : on relit alors l'état.
  - `tokens` : `Token{run, seq, text}`, un événement par delta du fournisseur.
  - `agent` : `RunStarted`/`RunFinished`, `LlmCall`, `ToolCallStarted` (nom et arguments), `ToolCallFinished` (`ok`, `error_kind`, `ms`, `bytes`, mais sans le contenu).
- **Commander.** `POST /ide/teammate/mailbox {backend, task, kind: request|instruction|answer|stop, text}` ; `GET /ide/teammate` relit la boîte, le plan et l'état (`running`, `closed`) ; `POST /ide/runs/arreter` arrête un run.

### Ce qui manque côté hôte (à everything declarative)

1. **Les lignes de boîte dans le flux** (un sujet `mailbox.<tâche>` relayé). Sans elles, une question de l'équipier ne se voit qu'en sondant `GET /ide/teammate`.
2. **Un état explicite** `running|waiting|ended|error`, ainsi que des ids de ligne stables et `reply_to`. Ta page 01 les décrit, ils ne sont pas codés.
3. **Le raisonnement** (deltas `reasoning`) republié sur le bus. C'est ce qui nourrit l'état « réfléchit ».
4. **Le résultat d'un outil** dans `ToolCallFinished`, au moins un extrait borné.
5. **Une demande de validation typée** (`approval`) et sa réponse.

Le TUI commence sans ces points : il sonde la boîte toutes les 2 s, et il dégrade l'état « réfléchit » en « entre `LlmCall` et le premier token ». Chaque point livré par l'hôte retire une rustine.

## Les états de l'agent et le design démoniaque

| État | D'après quoi | Animation (page 01) | Fréquence |
|---|---|---|---|
| réfléchit | `LlmCall` puis `reasoning`, avant le premier token | **Cercle** : runes `᚛ᚨᛒᛟ᚜⸸‡` qui tournent autour de `⛧`, « INVOCATION DU CODE… » | 150 ms |
| écrit | `Token` | aucune animation ; le texte coule, et `⛧` reste fixe en tête de la ligne vivante | — |
| agit (lecture, recherche) | `ToolCallStarted` d'un outil qui lit | **Transmutation** : `⁅ ⸸ ⁆—[ ☿ ]—⁅ ⸸ ⁆`, suivie du nom de l'outil | 200 ms |
| agit (écriture, exécution) | `ToolCallStarted` d'un outil qui écrit ou exécute | **Glitch** : `[∇ ⛧ ∫] …COMPILING CURSE…` | 100 ms |
| attend l'humain | une question dans la boîte, ou un `approval` | `⛧` qui pulse lentement (teinte), avec la question et la saisie | 500 ms |
| fini ou erreur | `RunFinished`, `ended` | la ligne s'efface ; un bilan d'une ligne s'insère dans l'historique | — |

On sépare les outils qui lisent de ceux qui écrivent d'après le manifeste de l'outil (effet déclaré), jamais d'après une liste de noms en dur.

**Transposition en cellules.** Une animation est une fonction pure `(numéro d'image, graine, capacités) → Line`. Le Glitch tire ses symboles d'un petit xorshift dont on fixe la graine, ce qui rend le témoin reproductible.

**Imiter le halo.** `⛧` est dessiné en `#ff3333` gras, les runes en `#8b0000` → `#ff3333` selon leur position dans le cercle, et tachyonfx fait pulser la teinte. Les couleurs se dégradent selon le terminal :

| Le terminal a | On dessine |
|---|---|
| la couleur vraie (`COLORTERM=truecolor`) | les couleurs exactes |
| 256 couleurs | 196 (rouge vif) et 88 (rouge sombre) |
| 16 couleurs | rouge vif et rouge |
| aucune (`NO_COLOR`) | gras et normal |

**Les glyphes.** On mesure au démarrage, en écrivant `⛧`, `🜂` et `ᚨ` puis en relisant la position du curseur (DSR). Si un glyphe ne fait pas une colonne, on bascule sur un jeu sûr, sans les symboles du plan astral (`🜂 🜄` remplacés par `△ ▽`). Un jeu tout ASCII se force par variable (`(*)`, `|/-\`, `[#]`).

**Le budget.**
- Une ligne démoniaque change de 10 à 30 octets par image ; même le Glitch à 10 images/s reste sous 300 o/s.
- L'image de l'écran est bornée à 30/s en local. Les tokens sont groupés : tous ceux d'une même image partent ensemble.
- La cadence descend toute seule (30 → 15 → 8 images/s) quand l'écriture vers stdout bloque, ce qui signale un lien lent ou un mosh qui rattrape.
- Chaque image est encadrée par la sortie synchronisée (mode DEC 2026) quand le terminal la connaît, pour éviter le déchirement.
- Le redimensionnement passe par l'événement `Resize` (SIGWINCH), puis la zone vivante est recalculée.

## Les premiers lots

Un lot à la fois, chacun avec un témoin rouge d'abord.

| Lot | Contenu | Témoin |
|---|---|---|
| **tty-1. Le client du flux, sans TUI** | binaire `rag3weaver tty --plain` : lit `daemon.json`, ouvre `/ide/flux`, écrit les tokens au fil de l'eau sur stdout, reprend par `Last-Event-ID`, gère `lacune`, envoie une demande à l'équipier (`request`) | un faux hôte SSE en test : coupure au milieu d'un run, reprise sans trou ni doublon de `seq`, `lacune` qui mène à une relecture |
| **tty-2. La vue inline** | ratatui en `Viewport::Inline` : historique par `insert_before`, zone de saisie, `Esc` qui fait `stop`, redimensionnement | instantanés sur `TestBackend` à 80×24 et 40×10, plus un compte d'octets émis pour 1 000 tokens, borné **après une mesure** |
| **tty-3. Le Daemon Code Summoning** | les trois animations comme widget pur, la détection des capacités, le dégradé des couleurs et des glyphes | instantanés image par image, graine fixée, pour chacun des trois niveaux de couleur et le jeu ASCII ; une animation qui s'arrête bien à `RunFinished` |
| **tty-4. Les états et la boîte** | réfléchit, agit et attend calculés à partir des événements et de la boîte ; une question de l'équipier, la réponse tapée, et le `POST answer` | faux hôte : une question apparaît, on y répond, et la ligne `answer` arrive bien dans la boîte |
| **tty-5. La mesure à travers les liens** | délai de la frappe à l'écho et octets par seconde, sous ssh, tmux et mosh, de luciepc vers ce poste | des chiffres dans le rapport ; aucun seuil sans eux |

Ensuite, à ordonner avec Lucie : la couche déclarative `view!` sur taffy, le plein écran à panneaux (plan, fichiers touchés, diff), la 3D (bevy_ratatui_camera, ou un rastériseur à nous pour, par exemple, un graphe du code en 3D), et les images par ratatui-image.

## Questions pour Lucie (par c7)

1. Est-ce que la base ratatui et notre couche déclarative te vont, plutôt que de partir sur iocraft, ou d'aller vers OpenTUI hors de Rust ?
2. Un terminal Windows natif un jour, ou Unix seulement ? Avec crossterm, ce n'est qu'un drapeau ; avec termion, non.
3. Le TUI tourne-t-il sur la machine de l'hôte (ssh ne porte que le terminal), ou veux-tu aussi un client distant qui parle à l'hôte par un tunnel ?

## Ce que Lucie a tranché (7 nov. 2026, soir)

1. **La base :** ratatui avec notre couche déclarative.
2. **Les systèmes :** Windows, macOS et Linux, natifs (« notre truc tourne partout »). Le backend est donc **crossterm**, celui par défaut de ratatui ; termion est écarté. Il reste à prouver sous Windows ce que termion aurait laissé de côté : le mode brut, le redimensionnement et les séquences VT de la console Windows 10+. Le lot tty-5 ajoute pour cela un passage sur Windows et un sur macOS.
3. **Où tourne le terminal :** les deux cas, l'hôte n'est pas forcément sur la même machine.
   - **Le terminal tourne là où est l'hôte**, et ssh ne porte que l'affichage. C'est le premier visé.
   - **Le terminal tourne à côté de l'hôte**, sur une autre machine, et parle à l'hôte distant par un tunnel (`ssh -L`, ou une commande qui lit le `daemon.json` distant, comme le client MCP `code-rag3db-luciepc`).
   Le lot tty-1 sépare donc l'adresse et le jeton (lus d'un fichier local ou d'une commande) du reste, et le lot tty-5 mesure aussi le client à distance.

## Le design : la page HTML d'abord, puis des thèmes (Lucie, 7 nov. 2026)

- On reprend la page « Daemon Code Summoning » au plus près. Les trois exemples donnent le ton de tout le reste : des symboles qui tournent ou mutent autour d'un cœur fixe, un libellé de rituel en capitales, une couleur unique avec un halo.
- Plus tard, **un sélecteur de thèmes**. Un thème rassemble les jeux de glyphes des trois animations, la palette et son halo, les libellés (« INVOCATION DU CODE… ») et le rythme. Pistes de Lucie : fées, forêt, bureau, papillons, Mordor, Dracula. Pour Dracula, on entend le vampire (sang, chauves-souris, crypte), pas la palette de couleurs « Dracula » des éditeurs de code.
- Conséquence dès le lot tty-3 : une animation lit son thème au lieu d'avoir ses glyphes en dur. « Daemon » est le premier thème, et le seul au départ.
