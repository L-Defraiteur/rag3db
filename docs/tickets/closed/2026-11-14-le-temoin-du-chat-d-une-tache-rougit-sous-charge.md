# Le témoin du chat d'une tâche rougit sous la charge d'autres passes

- **État** : fermé (21 novembre 2026, session recherche) : la cause prouvée par un témoin rouge, corrigée par `recherche-flux-sans-verrou` (a60f9f7a71, PR 278). Ouvert le 14 novembre 2026, session recherche.
- **Gravité** : faux rouge de passe. Il coûte un rejeu entier.
- **Atteignable en service** : non établi. Le témoin attend la page du chat, et sous charge la page
  n'a jamais rendu sa vue.
- **Touche rag3weaver** : oui (`src/daemon_host/chat_qui_code.rs`, `src/daemon_host/navigateur.rs`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

Le 14 novembre à 23 h 3x, à luciepc, la passe du lot des orphelins (ff4ced00a, lib 373b5477b) a
rougi un seul des 45 témoins navigateur :
`daemon_host::chat_qui_code::le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page`.

Le message :
`…/ide/code/<dépôt>/chat : « … #model === 'modèle : faux-modele' && #lien === 'flux : ouvert' »
jamais vraie en 20 s (20 s de calme) ; la vue : "" ; en vol : [] ; erreurs : []`

La page n'a donc rien rendu, sans requête en vol et sans erreur.

D'autres passes lourdes tournaient en même temps (everything declarative, banc). Rejoué seul, sur
le même target et la même lib, le témoin est vert trois fois de suite, en 2,2 à 2,4 s. Le lot ne
touche ni le chat ni l'hôte.

## Ce qu'il faudrait

Lire ce que la page attend quand elle reste vide sans rien en vol : un module qui ne se charge pas,
ou un premier rendu suspendu à un événement manqué. Ensuite, soit le témoin attend la cause, soit la
page le dit. Un délai plus long ne serait pas une réponse : 20 s de calme sans aucune requête, c'est
un blocage, pas de la lenteur.

## Suite (19 novembre 2026, recherche) : une page de chat qui ne rend rien sous charge

**Les occurrences**, toutes au chargement de la page du chat, sous la charge d'autres passes :
- **14 novembre, 23 h 3x** : `chat_qui_code::le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page`, la vue vide ;
- **18 novembre, 13 h 01** (`passe-garde-124920`, lib 59ac9c86b) :
  `chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page`. La même condition (`#model` et « flux : ouvert ») n'est
  jamais vraie en 20 s : « la vue : "" ; en vol : [] ; erreurs : [] ».

**La fréquence** : depuis le 15 novembre à luciepc, 2 rouges sur environ 215 passages de ces deux témoins.

**Ce que le témoin ne voyait pas.** « En vol » ne compte que les appels que la page suit elle-même. Un module de la
page qui ne vient pas, ou le flux (tenu par un `SharedWorker`) qui ne s'ouvre pas, ne sont ni en vol ni une erreur
de la page. Le rouge ne disait donc pas où la page s'était arrêtée.

**Le diagnostic posé** (branche `chat-page-diagnostic`). Une attente qui échoue dit maintenant l'état de la page :
l'adresse du document, son état, s'il est l'ancien ou le neuf, et les trente dernières ressources chargées, avec leur
départ, leur durée et leur statut. Le témoin `daemon_host::navigateur::tests::a_wait_that_fails_says_what_the_page_loaded`
le garde.

**Ce qu'il faut pour le fermer** : à la prochaine occurrence, lire l'état rendu. Un module manquant ou lent mènera
à la page de l'IDE et à son chargement. Une ressource de flux absente mènera au `SharedWorker`. Une adresse restée
l'ancienne mènera à la navigation du témoin.

## Le 20 novembre 2026 : deux essais sans rouge, la lecture du démarrage, une hypothèse (recherche)

**Les essais**, à luciepc, sur master 1a890cefec, lib 4ad394f4d2, les deux témoins du chat
(`chat_qui_code::le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page` et
`chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page`), joués en boucle par le binaire de test :
- **sous une passe lourde parallèle** (`e2e_code` répétée quatre fois dans un autre worktree, son propre target, sous
  `timeout`) : **0 rouge sur 120 passages** (60 tours), de 15 h 2x à 16 h 15. Charge du poste : 8 à 12 sur 24 cœurs ;
- **sous une famine déterministe** (au mot de c7) : le binaire et son Chromium épinglés par `taskset` sur deux cœurs
  partagés avec quatre boucles actives, sous `timeout` et `poste lourd` : **0 rouge sur 120 passages** (60 tours),
  jusqu'à 17 h 25.

Au taux observé jusque-là (2 sur environ 215), 240 passages sans rouge disent que le défaut ne vient ni de la seule
charge CPU, ni de la lenteur du témoin. Pas de troisième charge (règle de c7).

**La lecture du démarrage de la page** (`ui/ide`, `src/daemon_host/navigateur.rs`, `src/ide.rs`) :
- une vue `""` veut dire que `#root` est vide, donc qu'`App` n'a jamais été monté. Dès que React rend, `#vue` porte du
  texte (« … l'état de l'hôte se lit », puis « Connexion… »). Le flux du SharedWorker ne peut donc pas expliquer une
  vue vide : bloqué, il laisserait la page rendue avec « flux : fermé » ;
- le bundle est un seul module ESM (`/ide/app.js`), sans import dynamique. Comme le script est un module, donc
  différé, `readyState` vaut déjà `interactive` avant qu'il ne s'exécute : la condition `readyState !== 'loading'`
  passe pendant qu'`app.js` est encore en route ;
- « en vol » ne compte que les `fetch` de la page : le chargement d'`app.js` n'y est pas, ni comme requête, ni comme
  erreur. **Seul un `app.js` qui n'arrive pas** colle à « vue "", en vol [], erreurs [] ». Le diagnostic de page
  (896dcb7967) le montrera à la prochaine occurrence : `document` à `interactive`, et `/ide/app.js` absent des
  ressources, puisqu'une ressource n'y figure qu'une fois finie.

**L'hypothèse, à éprouver par un témoin** : le serveur de l'hôte des témoins n'a que **deux fils**
(`DaemonServer::new(server, 2)`). `Flux::publier` (`src/ide.rs`, l. 166-173) écrit dans la socket de chaque abonné
SSE **sous le verrou `abonnes`**, et `battre` aussi. Un abonné qui ne lit plus (une page partie, ou lente sous charge)
peut faire bloquer cette écriture quand le tampon de sa socket est plein ; le verrou reste alors tenu. Toute requête
qui publie, et toute nouvelle abonnée, attend ce verrou ; si les deux fils du serveur y attendent, `app.js` ne part
jamais, sans erreur ni requête en vol.

**Le témoin proposé** (un lot à part, rouge d'abord) : un abonné SSE qui ne lit plus jusqu'à remplir sa socket,
une publication, puis une requête d'un autre client (`GET /ide/app.js`) ; elle doit répondre en moins d'une seconde.
Si elle attend, le correctif écrit hors du verrou (copier la liste, écrire, retirer les abonnés morts) avec une
écriture bornée, et le ticket se ferme sur ce témoin. Si elle répond, l'hypothèse tombe, et le ticket attend la
prochaine occurrence et son diagnostic.

Le journal d'un autre rouge du même après-midi, sans rapport avec ce ticket, est gardé à luciepc :
`~/.cache/rag3weaver-build/gardes-recherche/2026-11-20-relecture-retenue-sous-charge/`. C'est la relecture retenue
de la seconde fenêtre : la page y est rendue, `app.js` arrivé.

## Ce qui le ferme (20 et 21 novembre 2026, recherche)

**L'hypothèse du verrou `abonnes`, prouvée.** Le témoin `daemon_host::tests::a_window_that_stops_reading_does_not_starve_the_page_of_another`
suit le chargement d'une page sur l'hôte des témoins à deux fils :
- une fenêtre s'abonne au flux, puis ne lit plus ;
- des publications de 64 Ko suivent ;
- puis deux `GET /ide/etat`, qui comptent les fenêtres par `fenetres()`, sous le même verrou ;
- puis `GET /ide/app.js`.

**Rouge** sur d082708414 : après 39 publications, la socket de la fenêtre muette est pleine, et le publieur bloque en tenant
le verrou. Les deux `/ide/etat` l'attendent sur les deux fils, et `app.js` ne vient pas en 5 s. C'est la page vide du
ticket, « en vol [], erreurs [] ».

**Le correctif** (`src/ide.rs`) : chaque fenêtre a son propre fil d'écriture et une file bornée
(`EN_ATTENTE_PAR_FENETRE`). La publication, le battement et le rejeu n'y font que pousser sous le verrou, sans
attendre. Une file pleine (la fenêtre ne lit plus) ou un écrivain tombé retire la fenêtre, qui revient par son
`Last-Event-ID`. `fenetres()` ne compte que les vivantes.

**Vert** :
- le témoin, avec 131 tests du flux, de l'IDE, de l'équipier et du chat ;
- les **54 témoins navigateur** (luciepc, `RAG3WEAVER_PAQUETS_REACT`), dont le journal est gardé à
  `luciepc:~/.cache/rag3weaver-build/gardes-recherche/2026-11-20-flux-sans-verrou-navigateur/` ;
- l'attestation check_features, 8/8 ;
- la passe du pousseur, qui rejoue aussi les témoins navigateur.

Les témoins navigateur ont trouvé en route un effet de bord : une fenêtre partie restait comptée jusqu'à 45 s. Il est
corrigé avant le dépôt (`fenetres()` ne compte que les vivantes).

Si une page vide revient, le diagnostic de page (896dcb7967) dira où elle s'est arrêtée.

