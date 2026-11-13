# La mesure de l'étape navigateur : où va le temps, et d'où viennent les rouges

13 novembre 2026, everything declarative, à la demande de c7. Trois témoins navigateur ont rougi
aujourd'hui sous l'étape entière (`scripts/temoins_navigateur.sh`, 40 témoins), tous verts seuls.
c7 : « si la cause commune est une attente bornée à 20 s sous la charge de l'étape, c'est l'étape
qu'il faut mesurer plutôt que relever chaque seuil ». Cette page mesure avant tout correctif.

**En une phrase** : la charge de l'étape n'est pas la cause. Les rouges ont deux causes distinctes,
toutes deux dans le produit, pas dans les seuils.
1. **Une bulle fantôme dans la page du chat.** Elle explique deux rouges sur trois ; archi l'a
   corrigée.
2. **Un premier appel de `search_code`, puis d'`impact`/`callees`, qui coûte 59 s à chaque jeu**,
   même seul. Elle explique le troisième rouge, et sa cause reste à trouver.

## Le banc

- Master `cb21e3cbb`, lib `835d5298f`, sans MOTEUR_ANCIEN, target neuf.
- Le binaire de tests de la lib (`--features <jeu large>,navigateur`) est joué directement :
  - le format JSON de libtest, avec `--report-time` (temps par témoin) ;
  - `RAG3WEAVER_PAQUETS_REACT` posé ;
  - aucun service d'embarquement (la recherche dense dit « not available »).
- **Une instrumentation de mesure**, hors master (`navigateur.rs`, sous `RAG3WEAVER_TEMOIN_TEMPS`) :
  - le temps de chaque `attendre` ;
  - toutes les 10 s d'une attente, les appels de la page encore en vol ;
  - après une attente de plus de 10 s, le texte de la vue.
- **Deux postes, chacun seul sous `poste mesure`** :
  - luciepc (24 cœurs) ;
  - lucie-tablette (32 cœurs, l'affichage de Lucie), où les trois rouges ont eu lieu. Même
    binaire, copié ; la lib a la même empreinte sha256 sur les deux postes.

## Ce qui a été mesuré

| jeu | poste | fils | durée | rouges |
|---|---|---|---|---|
| étape entière ×3 | luciepc | 24 | 131,8 – 132,0 s | 0/40 |
| étape entière ×2 (avec --nocapture) | luciepc | 24 | 132,3 s | 0/40 |
| étape entière ×2 | luciepc | 8 | 138,6 – 139,3 s | 0/40 |
| étape entière ×3 | lucie-tablette | 32 | 131,5 – 132,1 s | 0/40 |

(Un premier jeu sans `RAG3WEAVER_PAQUETS_REACT` avait rouge les cinq témoins de l'aperçu, qui le
disaient : une erreur de mon banc, rejouée.)

- **La charge de l'étape est basse.**
  - luciepc : load au plus 8, pression CPU avg10 au plus 11,7 %.
  - lucie-tablette : load au plus 13,5, pression CPU au plus 12,4 %.
  - Huit fils au lieu de 24 ne changent presque rien (139 s au lieu de 132 s).
- **La durée de l'étape, c'est celle d'un seul témoin.**
  `les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur` dure 125 s seul, et 128 s dans l'étape.
  Viennent ensuite cinq témoins de 60 à 72 s ; tous les autres prennent moins de 17 s. Sous
  l'étape, un témoin perd 0 à 5 s.
- **Les attentes** (912 relevées sur trois jeux, lucie-tablette) : médiane 179 ms, p95 1,9 s,
  **p99 58 s, max 59,4 s**.
  - Les attentes de près de 60 s ne passent que parce qu'un appel de la page reste en vol pendant
    tout ce temps : le « calme » de 20 s se remet à zéro.
  - Elles touchent la recherche et le graphe du témoin des vues, la page du chat des deux témoins
    du relais, et la tâche de `l_agent_edite_un_depot_d_a_cote…`.

## Cause 1 : la bulle fantôme de la page du chat (deux rouges sur trois)

- **Les deux rouges** :
  - `un_accord_exterieur_reprend_la_ligne_exacte_et_n_accorde_pas_sa_variante` (arbre principal) ;
  - `le_navigateur_inventorie_les_outils_effectifs_atelier_depot_et_tache` (archi, journal entier
    gardé).
- **Ce que disent leurs vues au moment du rouge** :
  - le tour était fini, sa réponse affichée, et rien n'était en vol ;
  - pourtant le statut n'est jamais passé à « fini ». La vue d'archi montre une seconde bulle
    « Agent » vide, « l'agent travaille… » et Arrêter.
  - L'attente a duré 20 s en tout, toutes de calme : la page avait fini dans la seconde. Un seuil
    plus long n'y aurait rien changé.
- **La cause, trouvée par archi** (`ui/ide/src/views/Chat.tsx`, depuis le lot de la carte
  d'accord) : sous charge, le `RunStarted` du tour que la page vient d'envoyer arrive après la
  réponse du POST. La page le prend pour un tour venu d'ailleurs et ouvre une bulle que rien ne
  ferme.
- **Corrigé** sur `origin/chat-sans-bulle-fantome` : la page retient les runs finis et ignore un
  `RunStarted` tardif. Le témoin `a_late_run_started_of_a_finished_turn_opens_no_ghost_bubble` est
  rouge sur master, vert avec le correctif. Il atterrit avec le lot d'archi.

## Cause 2 : le premier `search_code`, puis le premier `impact`/`callees`, coûtent 59 s

- **Le rouge** : `les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur` (archi), sur la page
  `graphe/depart`. L'attente a cédé en 44 s : 24 s avec un appel en vol, puis 20 s de calme.
- **Mesuré, seul et sans charge** (luciepc), dans ce témoin :

  | page | attente |
  |---|---|
  | `recherche?q=arrivee` | 59,4 s |
  | `recherche?q=arrivee&reclasser=1`, juste après | 0,26 s |
  | `graphe/depart` | 59,2 s |
  | toutes les autres pages | 0,1 à 0,8 s |

  À chaque jeu, sur les deux postes.
- **Les appels qui tiennent ces 59 s**, relevés dans la page :
  - pour la recherche, `/ide/call search_code {query: "arrivee", options: {limit: 30, highlights: "sparse"}}` ;
  - pour le graphe, `/ide/call impact {depth: 3}` et `/ide/call callees {depth: 3}`.

  Les deux pages finissent par rendre le bon contenu : « 2 résultat(s) (mots 2, sens 0) ·
  partiel », « dense signal is not available: no embedding service ».
- **Le motif** : seul le **premier** appel de chacun de ces outils coûte environ 60 s ; le suivant
  prend 250 ms. Ce n'est pas la charge.
  - Le témoin passe parce que l'appel reste en vol : son calme se remet à zéro.
  - Sous charge, il suffit que l'appel rende une erreur ou se termine avant le rendu, et 20 s de
    calme suffisent à rougir. C'est le rouge d'archi.
- **La cause n'est pas encore trouvée.** Plusieurs délais de 60 s existent dans le code. Lu, et
  écarté pour ce cas :
  - le délai d'attente des écrivains de la cohérence stricte (`attendre_les_ecritures_pour`) :
    5 s par défaut, et aucun avertissement dans la vue ;
  - `LAUNCH_WAIT` de l'hôte : ce chemin ne lance rien.

  Restent à départager, entre autres :
  - `default_embed_timeout` (60 s, `daemon/embeddings.rs`), si un premier appel tente quand même
    un embarqueur ;
  - la première construction d'un graphe d'outil ;
  - un verrou tenu par l'index, encore en cours (« vecteurs 0 % »).
- **La sonde suivante** : un témoin de la lib, sans navigateur, qui appelle deux fois
  `search_code`, puis `impact`, par l'hôte, et mesure chaque appel. Rouge s'il passe 5 s. Il dira
  où vont les 60 s. Une prise de pile n'a pas été possible : `ptrace_scope=1` à luciepc, et gdb
  rend EINTR au témoin.

## Ce que j'en tire

- **Ne relever aucun seuil.** Les deux causes sont des défauts du produit ; un seuil plus long en
  cacherait une (la bulle fantôme ne finit jamais) et laisserait l'autre coûter une minute à
  l'IDE de Lucie.
- **La règle d'aujourd'hui reste bonne** : rouge d'environnement nommé, second jeu de l'étape,
  push. La cause 1 part avec le lot d'archi. La cause 2 me revient, par la sonde ci-dessus, rouge
  d'abord.
- **L'étape coûte 132 s, dont 118 viennent de ces deux attentes d'un seul témoin.** Corriger la
  cause 2 la ramène à environ 72 s.

## Les traces

Les fichiers sont sous `~/.cache/rag3weaver-build/ed-navigateur/` des deux postes :
- les JSON de libtest et la sortie d'erreur de chaque jeu, `run-*` (luciepc) et `p-*`
  (lucie-tablette) ;
- `charge*.txt` ;
- `vol.err`, les appels en vol ;
- `vue.err`, le texte des vues lentes.
- Les instrumentations : `instrumenter*.py` (luciepc).

## Cause 2, trouvée (13 novembre 2026, soir) : le plafond de connexions du navigateur

**Ce n'est pas l'hôte.**
- La sonde de la lib (`le_premier_appel_d_un_outil_ne_coute_pas_une_minute`, branche
  `premier-appel-sans-minute`) appelle les outils par l'hôte, sans navigateur. Tout est rapide :

  | appel | durée |
  |---|---|
  | `/etat` | 38 ms |
  | premier `search_code` | 49 ms |
  | `impact` | 29 ms |
  | `callees` | 11 ms |

- Le serveur, mesuré dans le témoin des vues (heure de réception et durée de chaque requête de
  l'IDE), **ne reçoit rien pendant les 59 s**. `/ide/etat` et `/ide/call` lui arrivent 59 s après
  la requête précédente, et il y répond en 11 et 33 ms. C'est le **navigateur** qui retient les
  requêtes de la page.

**Pourquoi il les retient.**
- Chaque vue chargée ouvre son flux (`GET /ide/flux`, SSE). Le témoin charge les vues l'une
  après l'autre (`Page.navigate`). Le blocage tombe au septième flux ouvert, puis de nouveau six
  vues plus loin (sur la recherche, puis sur le graphe).
- Chrome plafonne à 6 connexions HTTP/1.1 par hôte. Les flux des vues quittées restent tenus
  environ 60 s. La vue suivante attend qu'une connexion se libère.
- C'est la même chose pour les témoins de 60 à 72 s : relais du chat, tâche d'à côté.

**Ce que ça coûte hors des témoins.** Toute fenêtre de l'IDE tient un flux. À partir de six onglets
ou rechargements sur le même hôte en moins d'une minute, l'IDE de Lucie attend lui aussi une
minute. C'est peut-être aussi le « premier appel à une minute » à la réouverture de l'atelier.

**Ce qui reste à départager avant le correctif** : ce qui tient ces flux 60 s après le départ de
leur page (le cache d'aller-retour de Chrome, la fermeture côté navigateur ou côté serveur).
Le correctif touche la conception du flux ; il se propose à c7.
- (a) La page ferme son flux en partant (`pagehide`), et le serveur ferme un flux qui ne répond plus
  au battement.
- (b) Un seul flux par origine, partagé entre les fenêtres (un `SharedWorker`).
- (c) Le flux et les appels par un transport qui ne compte pas dans le plafond.

Le témoin rouge du correctif est un témoin navigateur, et non la sonde de la lib (verte) : plus de
six vues chargées de suite, chacune rendue en moins de 5 s.
