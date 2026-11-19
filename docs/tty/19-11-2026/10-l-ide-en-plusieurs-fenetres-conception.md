# L'IDE en plusieurs fenêtres : la conception

Session tty, 19 novembre 2026, au mot de c7, qui me donne le React. ED relit cette page avant le code, et le diff avant le dépôt. Ticket : [`docs/tty/tickets/2026-11-19-l-ide-en-plusieurs-fenetres.md`](../tickets/2026-11-19-l-ide-en-plusieurs-fenetres.md). Graine : `docs/orchestration/visions/README.md`, « L'IDE en plusieurs fenêtres (Lucie, 13 novembre 2026, au soir) ».

## Ce qui existe déjà

- **Chaque vue a son adresse** : `/ide/code/<id>/chat`, `/graphe/<nom>`, `/arbre/<dossier>`, `/fichier/<chemin>`, `/symbole/<nom>`, `/recherche`… Une seconde fenêtre sur une autre vue s'ouvre donc déjà par son adresse.
- **Un seul flux par navigateur** : le SharedWorker (`ui/ide/lib/flux-worker.ts`) tient `/ide/flux` et redistribue aux pages. Il porte aussi un **petit état partagé** (`etat-ecrire` / `etat`, `etatPartage` dans `lib/host.ts`). Une page neuve le reçoit en entier à son branchement. **Aucune vue ne s'en sert encore.**
- **Le chat suit un tour venu d'ailleurs** : un `RunStarted` de `chat:<id>` ouvre la bulle vivante, même quand la page n'a rien envoyé (`envoiDeLaPage`). L'hôte pousse l'état de la conversation à la fin du tour (`onEtat("chat", id)`), et la page relit son état à chaque ouverture du flux.

## Ce qui se sépare

Trois rôles de fenêtre, chacun une vue qui existe déjà :
- **le chat** (la conversation) ;
- **le dépôt** : l'arbre, un fichier, un symbole ;
- **le graphe** : un graphe d'outil, le DAG d'un run.

Une fenêtre détachée est la même page, sur l'adresse de sa vue, avec `?fenetre=<rôle>` : sans la barre latérale du chat, l'en-tête réduit à son rôle et au flux.

## Ce que porte le worker (l'état partagé)

Ce sont des clés de `etatPartage`, écrites par une page et reçues par toutes, et par une page neuve à son branchement :
- `fenetres` : les fenêtres ouvertes, `{rôle, backend, depuis}`, et le battement du worker les oublie quand elles partent.
- `selection:<backend>` : ce qu'on regarde, `{type: "fichier" | "symbole" | "graphe" | "run", cible, ligne?}`.
- `conversation` : la conversation ouverte, `{backend}`.
- `accords:<backend>` : les cartes d'accord en attente, pour qu'une fenêtre qui n'est pas le chat en montre le compte. La carte reste au chat.

L'état du tour n'y est pas. Il reste à l'hôte : le run en vol, les tours et les accords sont lus par `fetchChat` et suivis par le flux, comme aujourd'hui. Le worker ne garde que ce que l'hôte ne sait pas : ce que la personne regarde, et où.

## Ce que garde chaque fenêtre

- **Le chat** garde sa saisie (non contrôlée, dans le DOM), son défilement et ses outils dépliés. C'est l'état d'une fenêtre, jamais partagé.
- **Le dépôt et le graphe** gardent leur zoom et leur défilement. Ils suivent `selection:<backend>` quand ils sont détachés : un fichier cliqué dans le chat s'ouvre dans la fenêtre du dépôt, au lieu de faire quitter la conversation.

## Ouvrir, refermer, sans perdre le tour en cours

- **Ouvrir** : un bouton « détacher » ouvre la vue dans une fenêtre neuve (`window.open`, l'adresse et `?fenetre=`), et l'écrit dans `fenetres`. Une fenêtre ouverte en plein tour relit l'état du chat (`fetchChat` : le run en vol, s'il y en a un), puis suit le flux. C'est déjà le chemin d'un tour venu d'ailleurs.
- **Refermer** : la page prévient le worker en partant (`partir`, déjà là). Le tour, qui vit à l'hôte, continue, et la fenêtre qui reste le suit. Fermer la fenêtre du dépôt rend les clics du chat à la navigation d'avant.
- **Aucune requête de plus au repos** : `the_ide_asks_the_host_nothing_while_idle_and_only_its_gesture_for_a_turn` reste la règle. L'état partagé passe par le worker, jamais par l'hôte.

## Les tranches, chacune avec son témoin navigateur rouge d'abord

1. **Deux fenêtres du même chat suivent le même tour** : un envoi dans la première, et la seconde, ouverte avant ou en plein tour, montre le même tour vivant, puis sa réponse. C'est la base de tout le reste ; si elle tient déjà, son témoin la garde.
2. **Le chat guide une fenêtre du dépôt détachée** : `selection:<backend>`, un fichier cliqué dans le chat qui s'ouvre dans l'autre fenêtre, le chat qui ne bouge pas.
3. **Le bouton « détacher » et `?fenetre=`** : la vue seule, sans barre.
4. **Le graphe détaché** suit le run du tour (`selection` de type `run`).
5. **Le compte des accords** en attente, dans une fenêtre qui n'est pas le chat.

Sans SharedWorker (navigation privée), chaque fenêtre garde son flux, et `etatPartage` reste local (`ecrireLocalement`) : les fenêtres ne se guident pas, et rien ne casse. La page le dit une fois dans l'en-tête.
