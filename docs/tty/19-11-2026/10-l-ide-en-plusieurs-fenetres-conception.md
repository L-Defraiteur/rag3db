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

## Tranche 3, faite (19 novembre, dans la nuit)

- **Le bouton « dépôt à côté »** du chat (`#detacher-depot`) ouvre l'arbre du dépôt dans une fenêtre à lui (`target="_blank"`, `…/arbre?fenetre=depot`). Les fichiers cités dans le chat s'y ouvrent ensuite (tranche 2).
- **La vue seule** : une fenêtre ouverte avec `?fenetre=<rôle>` n'a dans son en-tête que son rôle (« fenêtre : dépôt ») et le flux. Ni le nom de l'IDE, ni les boutons des thèmes. `#entete` porte `data-fenetre`.
- **La navigation interne** (remarque d'ED à la tranche 2) : la fenêtre du dépôt suit une sélection par `history.pushState` et un nouveau rendu. Elle ne recharge pas sa page, et garde son lien au worker. `popstate` refait le rendu : précédent et suivant ramènent les vues suivies.

Le témoin de la tranche 2 est étendu à ces trois points : l'adresse et la cible du bouton, l'en-tête d'une fenêtre détachée, et une marque posée dans la fenêtre du dépôt qui survit à la navigation. Il a été vu rouge (8c30bfe232), mais **à sa première assertion seulement** (le bouton manquait). L'en-tête et la navigation interne n'ont pas été vus rouges isolément. Il est vert (35f1a3ebe4). Restent verts : les trois témoins de la tranche 1, `ide_par_le_flux` (2) et le contrôle du bundle ; tsc est propre.

### La relecture d'ED de la tranche 3

- **Le même fichier à une autre ligne** (`A#L5` puis `A#L40`) : seul le hash change, et `pushState` ne déclenche pas de `hashchange`. La fenêtre restait sur L5. `File.tsx` lit maintenant l'ancre à chaque rendu (`ancre = location.hash`, dans les dépendances de l'effet qui vise la ligne). Le témoin cite `src/long.rs:5` puis `src/long.rs:40`, un fichier de 60 lignes écrit dans l'espace de travail.
- **Chaque promesse vue rouge isolément**, avec son seul morceau de code retiré (deux variantes du bundle, jouées à luciepc, le worktree remis après chacune) :
  - l'en-tête détaché (`fenetre` forcé à `null`) : rouge à « a detached window shows its role and the stream only » ;
  - la navigation interne (`location.assign` remis) : rouge à « the window of the repository followed without reloading its page » ;
  - la ligne suivie (sans le correctif de `File.tsx`, 03147ec4c4) : rouge à « the window aims at line 40 of the same file ».
- Vert (532b2716d2), avec les trois témoins de la tranche 1, `ide_par_le_flux` (2) et le contrôle du bundle.

Vu en route, hors tranche : la vue d'un fichier garde l'erreur du fichier précédent quand son chemin change. Ticket : [`docs/tty/tickets/2026-11-19-la-vue-d-un-fichier-garde-l-erreur-du-precedent.md`](../tickets/2026-11-19-la-vue-d-un-fichier-garde-l-erreur-du-precedent.md).

## Tranche 4, faite (20 novembre, dans la nuit)

**Une fenêtre du graphe détachée suit les outils du tour.** La vue `dag` montre le graphe d'un outil (`dag/tool/<nom>`), que le sujet `dataflow` du flux allume nœud par nœud. Il n'y a pas de vue par run. « Le graphe suit le run du tour » se lit donc ainsi : il montre le graphe de chaque outil que le tour appelle, au moment où il l'appelle.

- **Le chat**, à chaque `ToolCallStarted` de son tour, écrit `graphe:<backend>` (`{href: …/dag/tool/<nom>, at}`) quand une fenêtre du graphe est ouverte pour son backend. C'est une clé à part de `selection:<backend>` : la fenêtre du dépôt ne suit pas les outils.
- **`main.tsx`** : chaque rôle suit sa clé (`depot` → `selection`, `graphe` → `graphe`), par la même navigation interne, et garde son rôle dans l'adresse. L'en-tête dit « fenêtre : graphe ».

Témoin `le_graphe_detache_suit_les_outils_du_tour` : une fenêtre ouverte sur `dag/?fenetre=graphe` avant un tour qui appelle `list_files` puis `read_file` finit sur `dag/tool/read_file`, sans s'être rechargée, son rôle gardé, et le chat reste sur `/chat`. Rouge (8404c763b7 : elle restait sur la liste des graphes), puis vert (01f7960f71). Restent verts : les témoins des tranches 1 à 3, `ide_par_le_flux` et le contrôle du bundle.

### La relecture d'ED de la tranche 4

- **Le bouton « graphe à côté »** du chat (`#detacher-graphe`, `…/dag?fenetre=graphe`), à côté de celui du dépôt : sans lui, l'utilisateur ne savait pas ouvrir la fenêtre du graphe. Le témoin le clique au lieu d'ouvrir l'adresse. Le pilote clique alors comme une personne (`cliquer`, `userGesture`), puisqu'un `target="_blank"` cliqué par un script seul est une fenêtre surgissante bloquée. Il rejoint ensuite l'onglet ouvert (`rejoindre`).
- **Un outil sans graphe déclaré** (`propose`, `report`… que l'hôte sert lui-même) : la fenêtre dit « « propose » : outil sans graphe » (`#sans-graphe`), au lieu de l'erreur de l'hôte. `/ide/graphe` répond pour lui 200 avec `sans_graphe: true`, et non plus 404 : ce 404 était une erreur de console dans chaque fenêtre qui suivait un tel outil. La vue `dag` laisse aussi le graphe et l'erreur de l'outil d'avant quand elle en suit un autre ; elle montrait encore le graphe d'avant sous l'erreur.
- **Chaque promesse vue rouge isolément** :
  - sans le bouton (a397b001a6) : rouge sur le bouton ;
  - avec le bouton seul (variante V3, jouée à luciepc, le worktree remis après) : rouge sur « outil sans graphe » ;
  - avec les deux, mais l'hôte encore à 404 (d02631d6d4) : rouge sur l'erreur de console.
- Vert (efde269a1c), avec les tranches 1 à 3, `ide_par_le_flux` et le contrôle du bundle. Les tests `ide::` de la lib restent verts (12).

**Graine, sans ticket (ED)** : une vue par run, qui montrerait le run lui-même (ses outils dans l'ordre, ses nœuds allumés, sa fin), et non le graphe de chaque outil. La fenêtre du graphe la suivrait par son `run`.

## Tranche 5, faite (20 novembre, dans la nuit)

**Une fenêtre qui n'est pas le chat compte les accords en attente.** L'état poussé du chat ne porte pas les accords ; une carte d'accord arrive comme composant `ui` d'un retour d'outil (`ToolCallFinished`), et sa décision sur le sujet `accords` (`AccordDecide`). L'hôte ne publie pas de demande à part. D'où, sans clé partagée et sans rien changer à l'hôte (relecture d'ED) :

- **Au chargement**, la fenêtre détachée lit une fois l'état du chat (`fetchChat`), puis, pour chaque carte d'accord de ses tours, sa décision une fois (`GET /ide/accords?id=`).
- **Ensuite, le flux seul** : un `ToolCallFinished` de son backend qui porte une carte `accord` l'ajoute ; un `AccordDecide` la retire, et une décision vue avant la fin du chargement n'est pas recomptée.
- **L'en-tête** de la fenêtre détachée porte `#accords-en-attente` (`data-compte`), et dit « 1 accord attend dans le chat » quand il y en a.

Témoin `chat_contextuel::une_fenetre_qui_n_est_pas_le_chat_compte_les_accords_en_attente` : un tour lance `echo accorde-moi`, qui attend son accord dans le chat. Une fenêtre du dépôt ouverte avant le tour dit 0, puis 1 par le flux ; une seconde, ouverte après la carte, dit 1 à son chargement ; le chat accorde, et les deux disent 0.

- **Rouge** (77b7e69bfe, puis 57cb0fbc17 une fois le compte parenthésé dans le témoin : `x ?? null === '0'` se lisait `x ?? (null === '0')`, et un compte de « 1 » serait passé pour « 0 »).
- **Chaque promesse vue rouge isolément**, avec une variante du bundle privée d'un seul morceau, jouée à luciepc, le worktree remis après chacune :
  - sans le chargement : rouge à « the window opened after the card counts it at its load » ;
  - sans le flux qui ajoute : rouge à « the window open before the turn counts the approval awaited, by the stream » ;
  - sans le retrait : rouge à « the approval granted, no approval awaited ».
- **Vert** (8763658748), avec les tranches 2 à 4, `ide_par_le_flux` et le contrôle du bundle ; tsc propre.

**Relecture d'ED (20 novembre, 3 h 3x) : un accord fantôme se comptait pour toujours.** Les demandes ne vivent qu'en mémoire de l'hôte (`etat_accord` lit `accords.demandes`) : après un redémarrage, la carte d'un tour d'avant y est inconnue, `GET /ide/accords?id=` répond sans `ok`, et le chargement la comptait comme en attente, alors qu'aucun `AccordDecide` ne viendra jamais. Même cas pour une carte refusée par l'hôte quand le journal de la décision n'a pas pu s'écrire. Correction : au chargement, seule une carte que l'hôte connaît **et** sans décision compte ; une carte inconnue n'est ni ajoutée ni retirée.

- Témoin étendu : un second tour demande un second accord, puis l'hôte oublie ses demandes (`oublier_les_demandes_pour_un_temoin`, sous `cfg(test)`, comme un redémarrage), et une fenêtre ouverte alors doit dire 0 une fois son chargement fini (`data-charge`, une marque d'ordre et non un délai).
- Mises au point du témoin avant le rouge, chacune vue au journal : une commande propre à chaque message (la ligne accordée au premier tour ne redemande pas d'accord), et seulement sur un message tapé par le témoin (la relance de l'hôte après un accord dit aussi « echo »).
- **Rouge** à 7e857c459c, sur l'assertion de la carte inconnue ; **vert** à b34ac65f0a, avec les tranches 2 à 4, `ide_par_le_flux` et le bundle.

**Second point d'ED, noté et non corrigé ici** : à chaque ouverture d'une fenêtre détachée, un chat long fait une requête par carte d'accord de son histoire. Si l'hôte offre un jour la liste de ses demandes ouvertes, ce sera une seule requête, et c'est elle qui dira aussi ce que l'hôte connaît.

Les cinq tranches de la conception sont faites. Restent hors d'elles : la graine de la vue par run (tranche 4), et la reprise des appels d'outil d'avant l'ouverture d'une fenêtre (tranche 1).

## Le rouge du pousseur sur la tranche 5 (20 novembre, 11 h)

Le témoin de la tranche 2 est tombé dans la passe de la tranche 5 : la vue de `src/long.rs` montrait l'erreur de `src/main.rs`, absent (« no such file … did you mean: main.rs »). La remise à zéro de l'erreur (`file-erreur`) était là ; mais l'effet de lecture de `File.tsx` n'écartait pas une réponse arrivée après que la vue avait changé de fichier. Une erreur, ou les lignes du fichier d'avant, se posaient sur la vue suivante. Sur master, l'erreur d'un fichier absent porte une suggestion (« did you mean »), ce qui la rend plus lente : la course est apparue là.

- **Témoin rendu déterministe** : dans la fenêtre du dépôt, les lectures de `src/main.rs` répondent 1,5 s en retard (le témoin enveloppe `fetch`), et l'alerte se juge une fois cette réponse arrivée. Rouge à luciepc (cc67ffec98), sur l'assertion de `file-erreur`.
- **Correction** : un garde `vivant` dans l'effet ; une réponse d'un fichier quitté ne pose plus rien. Vert (e675284d6d), avec les tranches 1 à 4, l'IDE au repos et le bundle.
