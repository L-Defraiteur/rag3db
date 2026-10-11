# Le démon possède le catalogue — la forme C, avant le code

*Arbre principal et mémoire longue, 11 octobre 2026. Pour Lucie. Rien n'est codé : cette page
attend son avis.*

**La décision** (Lucie, au réveil du 11 octobre, sur
`10-octobre-2026-arbre-principal/03-l-indexation-portee-par-le-demon.md`) : la forme C,
directement. « Le serveur MCP ne doit être qu'une interface vers un démon. » Le démon possède le
catalogue, et donc l'indexation, la commande en fond et le réacteur. Les serveurs MCP, le chat,
l'atelier et la CLI deviennent des clients minces, qui lui parlent tous de la même façon.

*Chaque section porte le nom de la session qui l'a écrite.*

## 1. Ce que le démon tient aujourd'hui, et ce qu'il devra tenir

*Arbre principal.*

Trois processus tiennent aujourd'hui un morceau chacun. Aucun ne tient l'ensemble.

| | Ce qu'il tient | Fil | Remarques |
|---|---|---|---|
| `rag3daemon` | **une base, rien d'autre** : une `DbConnection`, sans catalogue, sans plein texte, sans bus ni réacteur | HTTP/1.1 par tiny_http, JSON, une route `POST /cypher` (`daemon/db.rs:232`), plus `GET /sante` et `POST /quitter` | aucune clé : la confiance est celle de la boucle locale, `--exposer` pour en sortir |
| `rag3weaver-backend serve` | **un backend entier** : la base en écriture, le catalogue, le réacteur monté, l'atelier et son flux | HTTP par tiny_http : les routes du manifeste, `/_atelier/*` (SSE `events`, `state`, `graph`, `chat`), `POST /arret` | boucle locale obligatoire ; c'est déjà presque le démon de C, pour un seul backend |
| `rag3weaver-backend mcp` | **un backend entier, par session** : catalogue, plein texte, bus, réacteur, indexation, commandes en fond | stdio JSON-RPC (MCP) | avec `--demon`, seule la connexion part chez `rag3daemon` |

Ce que montre la carte :

- **`--demon` ne déporte que la connexion.** Le catalogue, lucivy, le bus d'événements, le
  réacteur, l'indexation et la table des commandes en fond restent dans chaque processus MCP. Avec
  N sessions sur une base, on a N réacteurs : chaque réaction s'exécute N fois. Chaque bus vit en
  mémoire de son processus : une écriture de la session 1 n'atteint pas le réacteur de la
  session 2. Enfin, `DaemonConnection` ne redéfinit pas `begin`/`commit`, donc les transactions de
  deux clients passent par la même connexion du démon (lecture, à éprouver). `--demon` est faux par
  construction, dès qu'un manifeste a des réactions (mémoire longue).
- **Tout ce qui est long vit dans une table du processus.** L'indexation (`spawn_index`) jette sa
  poignée : pas de registre, pas d'arrêt possible. Les commandes en fond sont dans `suivies()`, un
  `HashMap` statique (`run_nodes.rs:126`). Les deux meurent avec le serveur qui les a lancées.
  Depuis `8ab83640a`, une commande en fond écrit ses journaux directement en fichiers et survit à
  son hôte. Sa poignée, elle, meurt encore avec lui.
- **Le chat terminal parle un troisième dialecte** : il lance le backend en enfant et lui parle
  en JSONL `{"op": describe|call|shutdown|…}` sur stdio (`bin/rag3weaver-chat.rs`, mode
  positionnel de `rag3weaver-backend`).

**Ce que le démon devra tenir** : tout ce que tient aujourd'hui un processus MCP, pour chaque
backend, et une seule fois. Il portera les catalogues et leurs bases, le plein texte, le bus et le
réacteur de chaque backend, les travaux longs (indexations, commandes en fond) avec leurs poignées
et leurs journaux, l'atelier et ses flux. Il ne tiendra pas les modèles : le démon d'embarquement
(`rag3weaver-embeddings`) reste à part, il est souvent sur une autre machine (luciepc), et le démon
de C lui parle comme le fait aujourd'hui un serveur.

## 2. Le démon : un processus, ses backends, ses travaux

*Arbre principal.*

**Le démon, c'est `serve`, pas `rag3daemon`.** Les deux parlent déjà HTTP par la même
bibliothèque, mais `serve` héberge déjà un backend entier, ses routes, l'atelier, le flux SSE et le
réacteur. Ce qui lui manque : héberger plusieurs backends, et porter les travaux. `rag3daemon`
n'apporte que `POST /cypher`, qui devient une route du démon par backend, réservée à l'outillage
(le banc, les sondes). Le binaire `rag3daemon` est retiré quand plus rien ne l'appelle, à la
marche 3. La mémoire penchait pour l'inverse (`rag3daemon` qui apprend à charger des
manifestes) ; on garde son idée, un seul processus et une seule adresse, portée par l'autre
binaire, parce que c'est lui qui a déjà tout le reste.

**Un démon pour toutes les bases.** Claude Code lance un processus MCP par session, et une
utilisatrice a plusieurs bases : sa mémoire, plus une par dépôt de code. Le démon a donc une seule
adresse, et chaque client nomme son backend. Un backend se nomme par le **chemin canonique de son
manifeste** : deux copies de `memory` s'appellent toutes deux « memory », un chemin est unique. Le
démon charge un backend à la première demande qui le nomme, et le garde.

- **Un écrivain par base**, naturellement : il n'y a plus qu'un processus. Le verrou de fichier
  du moteur (`F_WRLCK`) reste la garde contre un second démon ou un outil direct.
- **Le prix, à dire tel quel** : un démon qui tombe emporte toutes les bases. Et le tampon du
  moteur se compte **par base** : la règle d'aujourd'hui (la moitié de la mémoire, au plus 8 Gio)
  vaut pour une seule base. Avec trois bases ouvertes, il faut un budget de démon partagé entre les
  backends. C'est un point dur de la marche 1, à mesurer.

**Les travaux.** Un travail est une indexation ou une commande en fond. Le démon les tient dans un
registre : un identifiant, un genre, un journal en fichier, un état (en cours, terminé, échoué,
arrêté), le backend qui l'a lancé.

- `lancer` rend aussitôt l'identifiant et le chemin du journal, comme `index` aujourd'hui.
- `suivre` rend l'état et la fin du journal.
- `arrêter` demande l'arrêt.
- Un client qui part n'emporte rien : le travail appartient au démon.

**La reprise d'une indexation : la forme A en est la première marche.**

- **L'arrêt propre.** Un drapeau d'arrêt se lit entre deux paquets. L'endroit est la frontière
  transactionnelle de la transaction par paquet : en tête de la boucle des paquets
  (`code_sync.rs:687`), ou juste après `terminer`. Il se lit aussi entre deux passes de vecteurs
  (`index_nodes.rs:356`). L'arrêt referme la session de synchronisation par `abort_snapshot`, et
  le travail est marqué « arrêté au paquet N ».
- **La reprise.** Relancer `index` reprend à partir de là. Les paquets validés restent, et le
  journal de la passe dit ce qui reste à faire.
- **Un démon tué** (SIGKILL, plantage) laisse la session ouverte. Aujourd'hui, la relance est alors
  refusée sans `takeover` (ticket `2026-10-04-reprise-apres-arret-exige-takeover.md`). Sous C, le
  démon est le seul propriétaire possible de la session : à son redémarrage, il peut la reprendre
  lui-même (`takeover`), en le disant dans le journal du travail. Le refus actuel servait à
  protéger un autre processus vivant, et ce cas n'existe plus.
- **L'arrêt du démon** (`POST /arret`) arrête d'abord ses travaux au prochain paquet, puis ferme
  les bases proprement, avec leur point de reprise, et sort.

**La commande en fond** passe telle quelle dans le registre. Ses journaux sont déjà des fichiers
plafonnés (`8ab83640a`), et la portée du run (`run_scope`, tuée en fin de run) reste juste : le
run aussi appartient désormais au démon.

**Rouvrir un backend sans toucher aux autres** (question de la mémoire au §3) : oui. Chaque
backend a sa propre instance de base, ouverte par son chemin (registre `OPEN_FOR_WRITING`,
`rag3db_connection.rs:908`). Rouvrir un backend, c'est arrêter son réacteur et ses travaux au
prochain paquet, fermer sa base, la rouvrir et remonter ses réactions ; les autres backends ne
voient rien. `mustReopen` devient donc l'erreur transitoire que décrit le §3. Le seul lien
entre les backends est le budget de tampon partagé.

## 3. Le client mince, et une seule forme sur le fil

*Mémoire longue.*

**Ce qui existe déjà, et qui fait la moitié du chemin côté client.**

- **Un protocole client ↔ backend existe, et il a deux clients.** Le mode
  positionnel `rag3weaver-backend <backend.json>` parle en JSON par lignes sur
  stdio : `{"op":"describe"}`, `{"op":"call","name","arguments"}`, les trois `op`
  réservées à l'hôte (`journal`, `journal_read`, `index_state`) et `shutdown` ;
  il rend `{"ok":true,"result"}`, `{"ok":false,"error"}` ou `{…,"mustReopen":true}`.
  Ses deux clients : **le constructeur de decks** — le seul usage réel du produit
  aujourd'hui — et **le chat** (`bin/rag3weaver-chat.rs`, `BackendProcess`), qui
  lance ce backend en enfant et le relance **une fois** sur `mustReopen` sans
  rejouer l'appel. Le chat est donc déjà un client mince, d'un backend qu'il
  lance lui-même.
- **Le serveur MCP a déjà sa couture.** `mcp.rs` ne connaît pas `Backend` : il
  parle à un `trait Hote { nom, version, outils, appeler, doit_rouvrir }`, et
  `impl Hote for Backend` tient en quinze lignes au bas du fichier. Le client
  mince MCP est **un autre `impl Hote`**, qui porte `outils` et `appeler` au
  démon. `traiter`, `servir` et leurs huit tests ne bougent pas d'une ligne.
- **La réserve de la page MCP devient la forme.** `10-octobre-2026-18h00/02-un-serveur-mcp-par-manifeste.md`
  gardait « le pont `describe`/`call` vers un hôte unique » pour le cas où
  `--demon` serait rouge. Il l'est, par construction (plus bas).

**Une seule forme sur le fil : les messages du protocole `op`, portés par le
démon.** On n'invente pas de protocole ; on étend celui qui a déjà deux clients,
et le transport est celui du démon (§6, HTTP). Quatre ajouts :

0. **`session`, la première chose qu'un client fait** (décision 12 de Lucie, sur le modèle de
   connexion de PostgreSQL). Le client s'enregistre auprès du démon : « je suis tel client, je
   prends telle cellule, avec telles clés ». Il reçoit un identifiant de session. Ensuite, chaque
   requête porte sa session, et la cellule est celle de la session ; une requête peut en demander
   une autre, dans les droits de la session. Le catalogue n'a plus de « cellule courante » : deux
   sessions sur deux cellules ne partagent plus rien de mutable. Le démon sait qui est connecté et
   sur quoi, et l'outil « état du démon » le dit. Une session meurt quand son client le dit, ou
   faute de nouvelles au bout d'un délai. Elle ne meurt pas avec un appel, et un travail lancé par
   elle lui survit ;
1. **le backend est nommé** dans chaque échange, par le chemin canonique de son
   manifeste (§2) ;
2. **`open`** — charger le manifeste s'il ne l'est pas : base ouverte,
   embarqueur connecté, **réactions montées** ; idempotent ; rend ce que
   `describe` rend, plus les réactions montées. C'est exactement ce que
   `servir_mcp` fait aujourd'hui avant d'entrer dans sa boucle, déplacé ;
3. **`events`** — un flux par backend, pour le chat et l'atelier (journal, runs,
   rechargements). L'atelier en a déjà un (`/_atelier/events`, SSE) : c'est lui
   qu'on généralise. MCP n'en a pas besoin aujourd'hui (`listChanged: false`) ;
   il en aura besoin le jour où un rechargement doit dire
   `notifications/tools/list_changed`.

Et **`mustReopen` change de sens.** Aujourd'hui : « le processus va sortir avec
le code 75, relancez-le ». Sous C, le démon possède la base et la rouvre
lui-même ; le client reçoit une erreur transitoire nommée — « la base a été
rouverte, cet appel n'a pas abouti et n'est pas rejoué » — et ne relance rien.
*(Question à l'arbre principal : le démon sait-il rouvrir un backend sans toucher
aux autres ?)*

| client | aujourd'hui | sous C |
|---|---|---|
| MCP (Claude Code) | possède sa base, ou `--demon` | `impl Hote` qui parle au démon |
| chat | lance le backend en enfant, `op` sur stdio | les mêmes messages, au démon |
| atelier | une page de `serve` | une page du démon, inchangée |
| CLI positionnelle | possède sa base, `op` sur stdio | l'ouverture directe reste pour les tests, la CLI et le démon lui-même, **sans exception nommée** ; le constructeur de decks se refera après (décision 7) |

**Le client MCP.** Ce qui reste dans le processus que Claude Code lance : le
protocole MCP entier (stdio, JSON-RPC par lignes, `initialize`,
`server/discover`, `tools/list`, `tools/call`, `ping`, les notifications sans
réponse, les capacités déclarées absentes), la mise en forme d'un résultat
(`content[0].text` = la `presentation` si le backend en rend une ; un refus de
l'outil part en `isError`, pas en erreur de protocole), la lecture de `--keys`,
les lignes d'annonce sur stderr. **Rien qui touche une base, un fichier ou un
modèle.** Tout le reste — décrire, appeler, `doit_rouvrir`, ouvrir, monter les
réactions, `shutdown` — part au démon.

```
rag3weaver-backend mcp --manifest <backend.json> [--keys dev] [--daemon <adresse>]
```

`--manifest` ne dit plus « ouvre ceci » mais « parle-moi de ceci ». **C'est un
rôle du même binaire** (décision 8) : la sous-commande `mcp` devient le client
mince, une seule chose à installer.

*Correction de la mémoire longue, après la décision 8.* J'avais écrit que le
client ne liait plus le moteur et que la recette perdait `LD_LIBRARY_PATH`. Avec
un seul binaire, le rôle `mcp` lie le moteur comme les autres. Et la variable ne
vient pas de C : elle vient de **notre liaison partagée** (`RAG3DB_SHARED`, posée
sur nos postes pour ne pas bâtir des e2e de 1,35 Go) ; le bâti par défaut porte
le moteur en statique et n'en a pas besoin. Donc : sous C, le client perd
`RAG3DB_ROOT` (il n'ouvre ni base ni extension) ; il ne perd `LD_LIBRARY_PATH`
qu'avec un binaire statique — celui qu'un paquet livre. La mise en garde n° 5 de
la mémoire branchée de Lucie (`10-octobre-2026-18h00/07-l-ordre-a-la-memoire-branchee.md`)
tombe avec le binaire statique, pas avec C.

**Ce que devient `--demon`.** Aujourd'hui le backend reste dans le processus MCP
et seule sa connexion passe par `rag3daemon`. **Il est faux par construction
depuis la nuit** : depuis `0387fe401`, `Backend::open` monte les réactions du
manifeste, donc N sessions = N processus MCP = **N réacteurs**, et chaque
réaction s'exécute N fois ; pire, chaque catalogue a **son** bus d'événements en
mémoire, et une écriture de la session 1 n'est pas vue par le réacteur de la
session 2. **Il est retiré**, le nom passe à `--daemon <adresse>` (« je suis le
client de ce démon »), et le témoin `e2e_demon_backends_concurrents.rs` (branche
`memoire-temoin-demon-2`, jamais fusionnée) est abandonné avec lui.

**Le nom, tranché** (l'arbre principal l'a relevé) : deux drapeaux à une lettre
près, `--demon` et `--daemon`, qui veulent dire deux choses opposées, c'est un
piège. Donc : **`--daemon`**, en anglais comme tout identifiant du code depuis
le 6 septembre, est le seul nom de C ; **`--demon` est refusé** dès la marche 3,
avec un message qui nomme le remplaçant et le changement de sens (« `--demon`
déportait la connexion seule ; sous C, le client passe tout au démon :
`--daemon <adresse>`, ou rien et l'adresse se découvre ») — pas accepté en
silence comme un synonyme. Son code part à la marche 7.

Les trois énigmes du ticket `2026-10-11-mode-demon-ni-valide-ni-refute.md` :

1. **N `initialize` concurrents reposent les DDL** — tombe : un seul catalogue par
   base ;
2. **une lecture pend après le départ des écrivains** — tombe telle qu'elle est
   posée, puisque plus aucun client ne tient de connexion à la base ; **mais son
   témoin reste**, retourné : des clients successifs sur un même backend (§7) ;
3. **le journal du démon annonce une autre adresse que celle demandée** — tombe
   avec `rag3daemon`, mais **pas la question qu'elle cache** : le client de C
   trouvera son démon par `Serveur::assurer`, et on ne sait toujours pas si
   `assurer` peut annoncer une adresse et en servir une autre. À élucider avant
   que tout client en dépende.

Le ticket se ferme alors comme « remplacé par C », avec ces trois sorts.

**La mémoire partagée devient le cas par défaut.** L'ordre C de
`10-octobre-2026-18h00/05-la-memoire-par-mcp.md` coûtait « B, plus les trois
énigmes, plus un témoin ». Sous C : **un témoin**. Deux sessions Claude Code sur
la même mémoire sont deux clients qui nomment le même manifeste ; le démon n'a
qu'un catalogue, qu'un bus, qu'un réacteur. Ce qui reste n'est pas le démon :

- **la portée** — « ce projet » et « global » sont deux cellules
  (`Scope { org, project }`, branché). C'est la cellule **de la session** (ajout
  0 ci-dessus, décision 12) : deux sessions de deux projets partagent la base sans
  partager la cellule, et le fan-out « les deux d'un coup » est une requête qui
  demande une seconde cellule dans les droits de sa session ;
- **les deux lots de l'ancre**, acceptés par Lucie ce matin — sans eux, la
  réaction de la mémoire tourne à vide, démon ou pas.

La mémoire de Lucie déjà branchée migre sans toucher à sa base ni à son
manifeste : sa ligne `claude mcp add` perd deux variables d'environnement.

## 4. Quand le démon n'est pas là

*Mémoire longue, avec l'arbre principal pour le lancement.*

**Le client le lance.** `Serveur::assurer` (`src/serveur.rs`) est déjà le verbe :
s'il répond on s'y attache, sinon on le lance — détaché, journal en fichier — et
on attend. L'utilisateur n'installe ni ne lance jamais un démon à la main ; le
premier client le fait naître, et il survit à tous ses clients. C'est la seule
lecture de « installer rag3weaver, c'est installer un démon » qui ne demande à
personne de le gérer.

**L'adresse se découvre.** Proposition : le démon écrit
`~/.local/share/rag3weaver/daemon.json` (adresse, pid, version, bâti) en
démarrant ; le client le lit ; `--daemon <adresse>` l'emporte quand il est donné.
*(Arbre principal : l'adresse par défaut.)*

**Quand il ne peut pas naître** — binaire absent, port pris par un processus
étranger, base tenue par un ancien propriétaire — **le client MCP ne meurt pas au
lancement.** Claude Code montre mal un serveur mort au démarrage ; la recette du
10 octobre le notait déjà (« le serveur meurt au lancement sans rien dire
d'utile »). Donc le client démarre toujours, et :

- `initialize` réussit, avec la raison dans `instructions` ;
- `tools/list` rend **l'outil « état du démon »**, seul, dont la description dit
  pourquoi le démon manque, quoi faire, et propose de le lancer — une liste vide
  sans explication serait un défaut, pas une sécurité ;
- tout autre `tools/call` rend `isError` avec la même raison ;
- la ligne sur stderr le dit.

**Quand le démon répond, les outils du manifeste apparaissent, et « état du
démon » reste** (décision 6) : il dit alors les backends ouverts, les travaux en
cours et les sessions connectées. Un seul outil pour les deux cas, donc un modèle
qui l'a appris une fois sait toujours où regarder. Son apparition et sa
disparition des autres outils passent par `notifications/tools/list_changed` —
la première raison pour laquelle le client MCP a besoin du flux `events` (§3,
point 3), et `listChanged` devient `true`.

**Quand le démon meurt entre deux appels**, l'appel suivant passe par `assurer` :
le démon renaît, rouvre le backend, et l'appel en cours rend une erreur
transitoire nommée — jamais un tube cassé, jamais un rejeu silencieux.

**Ce que le client ne fait jamais : ouvrir la base lui-même en repli.** Un repli
silencieux vers « je possède ma base » recréerait le cas que C supprime — deux
écrivains possibles dès que le démon revient — et le cacherait.

**Pas d'exception** (décision 7) : le constructeur de decks est mis de côté
pour l'instant et se refera après ; l'ouverture directe d'une base reste pour les
tests, la CLI et le démon lui-même, sans être nommée comme exception. Le jour où
le constructeur revient, il parle déjà les messages de C sur stdio : il changera
de transport, pas de messages.

## 5. Le cycle de vie du démon

*Arbre principal.*

- **Qui le lance** : le premier client, par `Serveur::assurer` (§4). Il n'y a pas de service système
  par défaut. Une unité `systemd --user` (ou son équivalent macOS) peut venir plus tard, installée
  à la demande par le paquet. Elle n'est pas nécessaire pour que tout marche.
- **Où il écrit** : son journal va dans `~/.local/state/rag3weaver/daemon.log`, plus dans
  `$TMPDIR`. Sur ce poste, `/tmp` est de la mémoire vive, et le journal d'un démon qui vit des
  jours doit survivre à un redémarrage. `daemon.json` (§4) est écrit **après** que le port est
  pris, avec l'adresse réellement servie.
- **Qui l'arrête** : jamais un client qui part. `POST /arret`, ou SIGTERM, qui revient au même :
  les travaux s'arrêtent au prochain paquet, les réacteurs s'arrêtent, les bases se ferment avec
  leur point de reprise, `daemon.json` est retiré, puis le démon sort. Un démon sans client reste
  en vie (`Fin::Laisser`, comme aujourd'hui).
- **Un binaire plus neuf que le démon qui tourne** : `GET /sante` dit déjà l'empreinte de
  l'exécutable (`chemin@mtime`) ; on y ajoute l'identifiant de bâti du moteur. Un client plus neuf
  qui trouve un démon périmé fait deux choses selon le cas :
  - si aucun travail ne tourne, il lui demande `/arret` et le relance ;
  - si un travail tourne, il garde l'ancien démon et le dit (« démon périmé, N travaux en
    cours »), jamais en silence. C'est le mécanisme du démon d'embarquement (`est_a_jour_avec`,
    `quitter`), étendu aux travaux.
- **Un démon qui tombe** emporte toutes les bases ouvertes. L'appel suivant d'un client le relance
  (§4). Les backends se rouvrent à leur premier appel, et les indexations interrompues se reprennent
  d'elles-mêmes (§2, `takeover` par le seul propriétaire possible). Chaque base est rejouée par son
  journal comme après tout arrêt brutal. Ce qui ne revient pas : un appel en vol (erreur transitoire
  nommée) et une commande en fond, que sa mort tue avec son groupe de processus.
- **L'adresse par défaut** (question de la mémoire au §4) : `127.0.0.1:7970`. Les ports 7979 à 7981
  servent aux tunnels d'embarquement de ce poste, 7878 à 7880 aux services de luciepc, et un démon
  de C ne doit pas croiser ces adresses. `daemon.json` reste la vérité : `--daemon <adresse>`
  l'emporte, et le port par défaut ne sert qu'au premier lancement.
- **`assurer` peut-il annoncer une adresse et en servir une autre** (question de la mémoire au §3,
  l'énigme 3) ? Pas lu à fond. Le démon de C la rend sans objet : il écrit lui-même l'adresse
  servie et son pid dans `daemon.json` après avoir pris le port, et `/sante` rend le même pid.
  Un client qui voit deux pids différents sait qu'il ne parle pas au démon annoncé, et le dit.
  Le témoin vient à la marche 4.

## 6. Sur le fil : transport, format, clés

*Arbre principal, à relire par la mémoire longue.*

**Transport** : HTTP/1.1 en JSON, sur la boucle locale. C'est ce que parlent déjà les deux démons
(tiny_http), et les flux d'événements y sont déjà en SSE. Une socket unix serait plus étroite, mais
elle ne passe pas sous Windows (le paquet npm vise trois plateformes) et ne s'ouvre pas depuis la
page de l'atelier. Elle peut venir plus tard comme second transport, avec les mêmes messages.

**Les routes** : les messages du protocole `op` (§3), une route chacun. Le backend est nommé dans
le corps, par le chemin canonique de son manifeste. `open` rend aussi un identifiant court (une
empreinte du chemin), utilisable dans les chemins d'URL.

| route | message | notes |
|---|---|---|
| `POST /session` | `{client, cell, keys}` | rend `{session}` ; toute autre requête porte `session` (en-tête), sauf `/sante` |
| `DELETE /session` | | le client part ; ses travaux lui survivent |
| `POST /open` | `{manifest}` | idempotent ; charge, n'accorde rien ; rend `describe` + réactions montées + `id` |
| `POST /describe` | `{manifest}` | |
| `POST /call` | `{manifest, name, arguments}` | `{ok, result}`, `{ok:false, error}`, erreur transitoire nommée |
| `GET /events/{id}` | SSE | le flux de l'atelier, généralisé, par backend |
| `POST /travaux` | `{manifest, genre, …}` | rend `{travail, journal}` ; `index` passe par là |
| `GET /travaux/{t}` | | état et fin du journal |
| `POST /travaux/{t}/arreter` | | arrêt au prochain paquet |
| `POST /cypher/{id}` | `{query, params}` | l'ancienne route de `rag3daemon`, réservée à l'outillage |
| `GET /m/{id}/…` | | les routes déclarées au manifeste (vues HTML, JSON), sous l'identifiant de leur backend |
| `GET /_atelier…` | | l'atelier, qui choisit son backend |
| `GET /sante`, `POST /arret` | | la plomberie d'aujourd'hui |

**Deux choses différentes, à ne pas confondre** :

- **Le jeton dit qui parle.** Sur la boucle locale, tout processus de la machine atteint le port,
  quel que soit l'utilisateur qui l'a lancé. Aujourd'hui, `rag3daemon` exécute du Cypher pour qui
  atteint son port (issue `29-08-2026/05`). Proposition : le démon écrit un jeton aléatoire dans
  `daemon.json`, en mode 0600, et l'exige sur chaque requête (`Authorization: Bearer`). On obtient
  la frontière d'une socket unix (le même utilisateur), sans la socket. Un client le lit en même
  temps que l'adresse.
- **Les clés disent ce qui est exposé.** `--keys` existe et ne filtre rien (`mcp.rs:193`).
  `exposure`, une expression sur des clés par déclaration et par cellule, n'existe que dans les
  pages. Sous C, le filtre se fait **à la frontière du démon**, et le démon ne décrit ni n'exécute
  que ce que les clés exposent. Le client mince ne filtre rien lui-même : un client modifié ne doit
  rien gagner. **Les clés et la cellule se lient à la requête, pas au backend chargé** (relecture
  de la mémoire longue). Un backend est chargé une fois et partagé, et `open` est idempotent : si
  les clés se liaient au chargement, le second client hériterait de l'exposition et de la cellule
  du premier. **Elles appartiennent à la session** (§3, ajout 0) : le client les déclare en
  s'enregistrant, et chaque requête porte sa session.

Sortir de la boucle locale (`--exposer`) exige le jeton et des clés. C'est la marche 6, après le
reste.

**Relecture de la mémoire longue — un point qui change la forme de `open`.** Un backend est chargé
**une fois** et partagé par tous ses clients ; `open` est idempotent. Donc **les clés et la cellule
ne peuvent pas se lier au backend chargé** : le second client qui ouvre `memory` avec d'autres clés
hériterait de l'exposition du premier, et deux sessions de deux projets travailleraient dans la
cellule de celui qui a ouvert en dernier. Elles se lient **à la session** que le client a ouverte
(`POST /session`, ajout 0 du §3, décision 12), et chaque requête porte sa session. `open` charge ;
il n'accorde rien.

**Et la cellule par requête devient une condition de C, pas un confort.** Aujourd'hui, la bascule
de cellule (`options.scope`, le fan-out `options.scopes`) se fait **autour** du graphe, en changeant
la cellule du catalogue entier le temps de l'appel (`Catalog::rechercher`, `catalog.rs:8086` — la
limite y est déjà nommée : « ce n'est pas plus sûr qu'avant face à une recherche concurrente »). Le
verrou du catalogue est relâché entre la bascule et la recherche. Un processus MCP n'avait qu'un
client, donc la course était hors d'atteinte ; **un démon à N clients sur un backend la rend
atteignable** : la recherche de la session 2 tombe dans la cellule que la session 1 vient de poser.
Deux réponses, **et Lucie a pris les deux, dans cet ordre** (décision 12) :

- **sérialiser les appels par backend**, le temps de l'appel entier, à la marche 1 — le filet ;
- **plus de cellule courante dans le catalogue**, à la marche 1 bis : la cellule vient de la
  session et passe par la requête jusqu'au graphe. Le filet saute alors pour les lectures et reste
  pour les écritures. Une marche nommée de C, pas « quand un usage le demandera ».

**Le même défaut existe déjà, en petit, depuis le montage des réactions** (`0387fe401`) : le
réacteur tourne en même temps que les appels d'outil du même processus, et `ReactTransitionNode`
écrit dans la cellule **courante** du catalogue. Un `recall` à `options.scopes` qui bascule la
cellule pendant qu'une réaction part la fait écrire dans la mauvaise cellule. Atteignable seulement
si un outil bascule de cellule pendant qu'une réaction s'exécute, donc rare ; mais la sérialisation
par backend de C doit **inclure le réacteur**, pas seulement les appels des clients. Ticket ouvert
par la mémoire longue.

## 7. Les marches, testables une à une, et leur coût

*Les deux sessions.*

**Côté démon, les marches de l'arbre principal.** Elles sont testables une à une. Le coût est
compté en jours d'agent (un jour d'agent vaut environ une heure et demie réelle) : c'est un ordre
de grandeur relatif, pas un délai.

| marche | ce qu'elle fait | témoin | coût |
|---|---|---|---|
| **0. L'arrêt propre entre deux paquets** (forme A, dans le serveur d'aujourd'hui) | un drapeau lu à la frontière du paquet ; `abort_snapshot` ; le travail « arrêté au paquet N » ; `index` relancé reprend | tube fermé au paquet 2, puis reprise aux comptes d'une passe sans arrêt (sur le modèle d'`e2e_tx_par_paquet_arret`) | 1 à 2 |
| **1. `serve` hôte de plusieurs backends** | registre par chemin canonique, `open`/`describe`/`call`/`events`, réouverture d'un seul backend, budget de tampon partagé ; **les appels sérialisés par backend, réacteur compris** (la bascule de cellule de `Catalog::rechercher` ne tient qu'à un appel à la fois ; les cellules par requête viendront plus tard) | deux backends dans un processus ; la réouverture de l'un sans l'autre ; parité avec le mode positionnel ; deux clients sur un backend, chacun dans sa cellule | 2 à 3 |
| **1 bis. Les sessions** (décision 12) | `POST /session` ; plus de cellule courante dans le catalogue : la cellule passe par la requête jusqu'au graphe (`Catalog::rechercher` et son fan-out, `ReactTransitionNode`, les écritures) ; l'état du démon dit les sessions ; le filet de la marche 1 saute pour les lectures et reste pour les écritures | deux sessions sur deux cellules lisent en même temps, sans se voir ; une réaction écrit dans la cellule de l'écriture qui l'a déclenchée, pas dans celle d'une autre session (le ticket `2026-10-11-une-reaction-ecrit-dans-la-cellule-qu-un-appel-a-basculee.md` se ferme) | 2 à 3 |
| **2. Les travaux du démon** | registre ; `index` et la commande en fond ; lancer, suivre, arrêter ; reprise au redémarrage par `takeover` du propriétaire | le client part, l'index finit ; démon arrêté au paquet N puis relancé, la reprise rend les mêmes comptes ; démon tué, il redémarre sans refus | 2 |
| **3. Le client MCP** (mémoire longue) | `impl Hote` par le fil | ceux du côté client, ci-dessus | 1 à 2 |
| **4. Le cycle de vie** | `daemon.json` (adresse, pid, jeton, bâti) ; `assurer` qui vérifie le pid ; démon périmé avec ou sans travaux | l'énigme 3 retournée ; remplacement refusé pendant un travail | 1 |
| **5. Le chat et l'atelier sur le démon** | les mêmes messages ; l'atelier qui choisit son backend | le chat rejoué contre le démon ; le flux d'un backend ne reçoit pas les événements d'un autre | 1 |
| **6. Jeton et clés à la frontière** | le jeton sur chaque requête ; `exposure` filtrée dans le démon | une requête sans jeton est refusée ; une clé ne voit que ce qu'elle expose | 1 à 2 |
| **7. Le retrait** | `--demon` (l'ancien sens), le binaire `rag3daemon`, `DaemonConnection` sauf pour l'outillage | la batterie, sans eux | ½ |

En tout, 12 à 17 jours d'agent pour l'arbre principal et la mémoire longue. C'est la fourchette de
la page du 10 (« une à deux semaines »). Le paquet npm qui installe et lance le démon est le travail
de l'optimiseur : il vient après la marche 4.

**La marche 0 sert tout de suite, sans attendre le reste** : elle règle le cas mesuré (tube fermé,
travail perdu), et sa reprise entre deux paquets passe telle quelle dans la marche 2.

**Côté client, les témoins que la mémoire longue propose** (le coût en passes
viendra avec les marches de l'arbre principal) :

- **deux sessions, une réaction** — deux clients MCP sur un manifeste à réaction,
  une écriture par l'un : la réaction s'exécute **une fois**, et l'autre la voit.
  Le témoin qui condamne `--demon`, retourné ;
- **des clients successifs** — un client écrit et part, le suivant lit sans
  attendre : ce qui reste de l'énigme 2 ;
- **le démon absent** — un client sans démon le fait naître ; un client qui ne
  peut pas le faire naître rend `etat_du_demon` et ne meurt pas ;
- **le démon tué entre deux appels** — l'appel suivant le relance et rend une
  erreur transitoire nommée ;
- **`e2e_mcp_stdio`** rejoué contre le démon **sans changer une assertion** : la
  preuve que la couture `Hote` a tenu.

Les marches du client, dans l'ordre : `impl Hote` par le fil (avec `e2e_mcp_stdio`
inchangé) ; puis le lancement et l'absence (§4) ; puis le chat sur le démon ;
puis le retrait de `--demon`, en dernier, quand plus rien ne s'en sert.

## 8. Ce qui reste à Lucie

*Premier jet de l'arbre principal, à compléter par la mémoire longue.*

1. **`serve` devient le démon**, et `rag3daemon` est retiré (§2). L'inverse est possible, mais il
   referait dans `rag3daemon` tout ce que `serve` a déjà.
2. **Un démon pour toutes les bases** (§2). Le prix : un démon qui tombe emporte tout, et le tampon
   du moteur se partage entre les backends.
   **Décidé par Lucie (11 octobre, matin)** : un seul démon pour le moment. **À moyen terme, après
   ce chantier** : un démon routeur et des démons spécialisés — un par base, ou portant plusieurs
   bases — derrière lui ; noté en vision, pas dans ces marches.
3. **Lancé par le premier client**, sans service système par défaut (§5). Ou bien une unité
   installée par le paquet.
4. **L'adresse par défaut `127.0.0.1:7970`**, avec `daemon.json` (§4, §5).
5. **Un jeton local exigé sur chaque requête**, en plus des clés (§6).
6. **Le client sans démon** : un seul outil, `etat_du_demon`, ou un refus d'`initialize` (§4).
7. **Le mode positionnel reste propriétaire de sa base** pour le constructeur de decks (§4).
8. **Le client MCP** : un binaire à part, ou un rôle du même binaire (§3).
9. **L'ordre** : la marche 0 d'abord, puis 1, 2, 3 ; le paquet npm après la marche 4 (§7).
10. **Le nom du drapeau** : `--daemon` seul ; `--demon` refusé en le disant dès la marche 3 (§3).
11. **Les clés et la cellule se lient à la requête**, pas au backend chargé — `open` charge, il
    n'accorde rien (§6).
12. **Plusieurs clients sur un backend** : sérialiser les appels par backend, réacteur compris,
    d'abord ; les cellules par requête plus tard, quand un usage demande que deux clients cherchent
    en même temps (§6). C'est le seul point où C rend atteignable un défaut qui ne l'était pas.
13. **La mémoire partagée devient le cas par défaut** (§3) : deux sessions sur la mémoire de Lucie
    ne coûtent plus qu'un témoin — mais sa réaction tourne à vide tant que les deux lots de l'ancre
    (acceptés ce matin) ne sont pas faits ; ils sont indépendants de C et peuvent passer avant.

## 9. Les décisions de Lucie (11 octobre, matin)

| Choix | Décision |
|---|---|
| 1, 4, 5, 9, 10, 11, 13 | comme proposé (`serve` devient le démon ; `127.0.0.1:7970` + `daemon.json` ; jeton local sur chaque requête ; l'ordre des marches, le paquet après la 4 ; `--daemon` ; les clés liées à la requête ; les lots de l'ancre avant C) |
| 2 | **un seul démon pour le moment** ; à moyen terme, après ce chantier, un démon routeur et des démons spécialisés (un par base ou portant plusieurs), en vision |
| 3 | **lancé par le premier client** s'il n'est pas là (et le client le dit) ; l'unité système en option du paquet ; un verrou sur `daemon.json` pour qu'un seul démarre |
| 6 | **un outil « état du démon »** quand le démon n'est pas là (ce qui se passe, quoi faire, et le lancer) ; les autres outils apparaissent quand il répond ; le même outil dit ses backends, ses travaux et ses sessions quand il est là |
| 7 | tombe : « on oublie le constructeur de decks pour l'instant, il se refera après » ; l'ouverture directe d'une base reste pour les tests, la CLI et `serve` lui-même, sans exception nommée |
| 8 | **un rôle du même binaire** (la sous-commande `mcp` devient un client mince) |
| 12 | **sérialiser par backend dans la marche 1, comme filet** ; puis, **marche nommée de C, pas « quand un usage le demandera »** : plus de « cellule courante » dans le catalogue — **le client s'enregistre auprès du démon** (« je suis tel client, je prends telle cellule, avec telles clés ») et reçoit un identifiant de **session** ; chaque requête porte sa session, la cellule est celle de la session (une requête peut en demander une autre dans les droits de la session) ; deux sessions sur deux cellules ne partagent plus rien de mutable ; l'état du démon dit qui est connecté et sur quoi. C'est le modèle de connexion de PostgreSQL. Le filet de sérialisation saute pour les lectures quand ce lot est fusionné, reste pour les écritures. |

Écart rendu par le cœur en passant (T0) : au COMMIT d'une transaction en échec, Neo4j
refuse avec une erreur, PostgreSQL répond par un ROLLBACK silencieux ; le cœur prend
Neo4j, pour qu'un client ne croie jamais avoir validé. Lucie ne l'a pas renversé.
