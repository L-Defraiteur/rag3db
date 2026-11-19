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
- `fenetres` : les fenêtres ouvertes, `{rôle, backend, depuis, focus}`. **Le worker la tient lui-même** (relecture d'ED) : chaque page lui déclare son rôle au branchement, son battement l'oublie, il note la dernière qui a eu le focus, et les pages la lisent sans l'écrire. Une liste réécrite par les pages perdrait une fenêtre sur deux qui s'ouvrent ensemble, puisque c'est le dernier qui écrit qui gagne. Même règle pour toute clé à plusieurs auteurs.
- `selection:<backend>` : ce qu'on regarde, `{type: "fichier" | "symbole" | "graphe" | "run", cible, ligne?}`. Un seul geste à la fois : le dernier écrit suffit. Avec plusieurs fenêtres du dépôt, c'est **la dernière qui a eu le focus** (notée par le worker dans `fenetres`) qui suit la sélection, pour que deux fenêtres n'ouvrent pas le même fichier.
- `conversation` : la conversation ouverte, `{backend}`.
- ~~`accords:<backend>`~~ : retiré (relecture d'ED). Écrit par le chat, ce serait une copie de l'hôte, juste seulement quand une fenêtre de chat est ouverte. Une fenêtre qui n'est pas le chat compte les accords en attente depuis l'état que l'hôte pousse déjà (`onEtat("chat")`) : une seule source, et aucune requête au repos.

L'état du tour n'y est pas. Il reste à l'hôte : le run en vol, les tours et les accords sont lus par `fetchChat` et suivis par le flux, comme aujourd'hui. Le worker ne garde que ce que l'hôte ne sait pas : ce que la personne regarde, et où.

## La course de la relecture, en deux fenêtres (relecture d'ED, avant le code)

ED a corrigé le 19 novembre (`ed-chat-reload-epoch`) une relecture du chat qui, partie avant un tour et revenue après lui, l'effaçait de l'écran. Son époque des tours n'avance que sur les envois de la page. Une fenêtre B qui n'envoie rien, et dont le tour vient de A, garde la course. **L'époque avance aussi** sur le `RunStarted` et le `RunFinished` d'un tour venu d'ailleurs (`chat:<id>`), et sur l'état poussé de fin de tour (`onEtat("chat")`). Le témoin de la tranche 1 l'éprouve par un ordre et non par une durée : il retient une relecture de B (`window.__ide.relectureRetenue`), fait faire le tour par A, lâche la relecture, et vérifie que B garde le tour. Le lot se pose sur `ed-chat-reload-epoch`.

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
5. **Le compte des accords** en attente, dans une fenêtre qui n'est pas le chat, depuis l'état poussé par l'hôte.

Et, à chaque tranche, **un témoin au repos** (relecture d'ED, pendant du témoin des six onglets, 967224067b) : deux fenêtres ouvertes, une seule connexion `/ide/flux` à l'hôte, et aucune requête au repos.

Sans SharedWorker (navigation privée), chaque fenêtre garde son flux, et `etatPartage` reste local (`ecrireLocalement`) : les fenêtres ne se guident pas, et rien ne casse. La page le dit une fois dans l'en-tête.

## Tranche 1, faite (19 novembre au soir)

**Deux fenêtres du même chat suivent le même tour.** Elle ne tenait pas : une fenêtre ouverte en plein tour reprenait bien le run en vol (`reload`), mais ne montrait que les mots arrivés après son ouverture (« six sept huit neuf dix », sans « un deux trois quatre cinq »).

- **Le worker garde le texte des tours en vol** (`lib/flux-worker.ts`) : pour chaque run, le texte de ses `Token` et le numéro du dernier. C'est borné à 512 Kio, et oublié à son `RunFinished`. Il répond à la demande `tour` d'une page.
- **La page le demande** (`tourEnVol(run)` de `lib/host.ts`, qui rend `null` sans worker ou au bout d'1 s) quand sa relecture trouve un run en vol. Le chat pose ce texte dans la bulle vivante de ce run, et saute les jetons déjà comptés. Rien n'est demandé à l'hôte.
- **L'époque des tours d'ED avance aussi** sur le `RunStarted` et le `RunFinished` d'un tour venu d'ailleurs, et sur l'état poussé de fin de tour. Une relecture de la seconde fenêtre, partie avant un tour de la première et revenue après lui, ne l'efface plus.
- Un premier essai rendait la reprise derrière `runRef.current === run`. Or la réponse du worker arrive avant le rendu qui pose `runRef`, et la reprise renonçait. Le débogage l'a montré : le worker avait tout le texte, la page l'abandonnait. La mise à jour vise maintenant la bulle vivante de ce run.

Les témoins, navigateur, joués à luciepc, vus rouges d'abord (f80e058514, puis f5f1132f4f sans le code), puis verts (845b95fef9) :
- `deux_fenetres_du_chat_suivent_le_meme_tour` ;
- `une_relecture_retenue_de_la_seconde_fenetre_n_efface_pas_le_tour_de_la_premiere`, avec la barrière d'ED.

Ce qui n'est pas repris par une fenêtre ouverte en plein tour : les appels d'outil d'avant son ouverture. Elle les voit à la fin du tour, par l'état poussé. La tranche 1 ne porte que le texte.

### La relecture d'ED de la tranche 1

- **La borne du worker** (512 Kio) : passé elle, le texte et son `seq` s'arrêtent ensemble, et `tronque` le dit. La page rend le texte gardé, une marque « […] », puis chaque jeton reçu après lui (`recus`). Rien n'est sauté, rien n'est doublé, et le manque est dit. Témoin : `passe_la_borne_du_worker_le_texte_repris_dit_ce_qui_lui_manque`, la borne abaissée à 12 caractères par le message `tour-max`, réservé aux témoins.
- **Le compteur des relectures** (`window.__ide.relecturesAppliquees`, compté à la fin de chaque relecture, appliquée ou écartée) : le témoin de la relecture retenue l'attend, au lieu de 500 ms.
- **Le texte repris passe devant les outils** venus en direct, et l'entrelacement « mots, outil, mots » n'est pas refait. La fin du tour remet l'ordre de la session. C'est dit en commentaire.
- **Au plus 8 tours** sont gardés par le worker, le plus ancien oublié d'abord, et tous sont oubliés sur `EventsMissed`.
- **Une demande de tour sans réponse** quitte la file de `host.ts` au bout de la seconde.

Rouges d'abord (290141f1d1 : pas de marque au-delà de la borne, et un compteur qui n'existait pas), puis verts (b0e034b81a). Restent verts : les deux témoins de la tranche, `ide_par_le_flux` (2) et le contrôle du bundle.

## Tranche 2, faite (19 novembre au soir)

**Le chat guide une fenêtre du dépôt détachée.** Avant, cliquer un fichier cité dans le chat emmenait la fenêtre du chat sur la vue du fichier, hors de sa conversation, même quand une fenêtre du dépôt était ouverte à côté.

- **Le worker tient `fenetres`** (`lib/flux-worker.ts`). Une page ouverte avec `?fenetre=<rôle>` lui déclare son rôle et son backend à son branchement, puis son focus à chaque `focus`. Le worker numérote chaque page et lui rend son numéro (`moi`). Il publie la liste `{id, rôle, backend, focus}` sous la clé `fenetres`, à chaque changement, au départ d'une page, et quand son battement l'oublie. Les pages ne l'écrivent jamais (relecture d'ED).
- **`host.ts`** expose `fenetres()`, `roleDeLaFenetre()` et `jeSuisLaFenetre(rôle, backend)`, cette dernière vraie pour la fenêtre de ce rôle qui a eu le focus en dernier. La liste est recopiée dans `window.__ide.fenetres` pour les témoins.
- **Le chat** (`Chat.tsx`) capte, en phase de capture, le clic sur un `a.code-ref` (un fichier ou un symbole cité). Si une fenêtre du dépôt est ouverte pour son backend, il écrit `selection:<backend>` (`{href, at}`) et reste sur sa conversation. Sinon, le lien va comme avant.
- **La fenêtre du dépôt** (`main.tsx`) qui a eu le focus en dernier suit `selection:<backend>`. Elle ouvre l'adresse citée en gardant `?fenetre=depot`, et ignore une sélection antérieure à son ouverture, qu'elle reçoit pourtant entière à son branchement.

Le témoin, `le_chat_guide_une_fenetre_du_depot_detachee` (navigateur, joué à luciepc), suit ce chemin : le chat cite `src/main.rs:1` ; une seconde fenêtre s'ouvre sur l'arbre avec `?fenetre=depot` et prend le focus ; le clic dans le chat ouvre `fichier/src/main.rs` dans la seconde fenêtre, qui reste `?fenetre=depot`, et le chat reste sur `/chat`.

- Un premier rouge était faux : la réponse citait `main.rs:1`, qui n'est pas un lien (un chemin cité doit contenir un « / »). Le rouge butait avant le clic.
- Rouge ensuite (ef275fdc67 : le chat ne sait pas qu'une fenêtre du dépôt est ouverte), puis vert (bdb740179a).
- Restent verts : les trois témoins de la tranche 1, `ide_par_le_flux` (2) et le contrôle du bundle. tsc est propre.

À la relecture d'ED, la fenêtre du dépôt suit l'adresse citée par `URL` : `?fenetre=depot` collé derrière l'adresse donnait `?q=main?fenetre=depot` pour une adresse qui avait sa propre requête, et la fenêtre perdait son rôle. Le témoin le prouve par une sélection `…/recherche?q=main`, vue rouge (339b9edcfc), puis verte (c5bf47bd47). Remarque d'ED pour la tranche 3 : `location.assign` recharge la fenêtre du dépôt à chaque clic (défilement remis à zéro, connexion au worker rouverte) ; une navigation interne viendra avec elle.

Pas encore fait : le bouton « détacher » et la vue seule sans barre (tranche 3). Aujourd'hui, une fenêtre du dépôt s'ouvre par son adresse, avec `?fenetre=depot`.
