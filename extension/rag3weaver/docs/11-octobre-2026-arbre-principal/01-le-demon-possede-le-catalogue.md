# Le démon possède le catalogue — la forme C, avant le code

*Arbre principal et mémoire longue, 11 octobre 2026. Pour Lucie. Rien n'est codé : cette page
attend son avis.*

**La décision** (Lucie, au réveil du 11 octobre, sur
`10-octobre-2026-arbre-principal/03-l-indexation-portee-par-le-demon.md`) : la forme C,
directement. « Le serveur MCP ne doit être qu'une interface vers un démon. » Le démon possède le
catalogue, et donc l'indexation, la commande en fond et le réacteur. Les serveurs MCP, le chat,
l'atelier et la CLI deviennent des clients minces, qui lui parlent tous de la même façon.

*Les sections marquées « à écrire » ne sont pas encore rédigées. Chacune porte le nom de la session
qui l'écrit.*

## 1. Ce que le démon tient aujourd'hui, et ce qu'il devra tenir

*Arbre principal, à écrire.*

## 2. Le démon : un processus, ses backends, ses travaux

*Arbre principal, à écrire.* Il y a un démon pour toutes les bases, et un backend se nomme par le
chemin canonique de son manifeste. Les travaux longs (index, commande en fond) se lancent, se
suivent et s'arrêtent. La reprise d'une indexation commence par la forme A : l'arrêt propre entre
deux paquets en est la première marche.

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
et le transport est celui du démon (§6, HTTP). Trois ajouts :

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
| CLI positionnelle | possède sa base, `op` sur stdio | **reste telle quelle** — §4, la seule exception |

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

`--manifest` ne dit plus « ouvre ceci » mais « parle-moi de ceci ». Et **le
client ne lie plus le moteur** : la recette perd `LD_LIBRARY_PATH` et
`RAG3DB_ROOT`, et la mise en garde n° 5 de la mémoire branchée de Lucie
(`10-octobre-2026-18h00/07-l-ordre-a-la-memoire-branchee.md`) tombe avec eux.
*(À trancher : un binaire `rag3weaver-mcp` à part, petit, ou un rôle du même
binaire, une seule chose à installer.)*

**Ce que devient `--demon`.** Aujourd'hui le backend reste dans le processus MCP
et seule sa connexion passe par `rag3daemon`. **Il est faux par construction
depuis la nuit** : depuis `0387fe401`, `Backend::open` monte les réactions du
manifeste, donc N sessions = N processus MCP = **N réacteurs**, et chaque
réaction s'exécute N fois ; pire, chaque catalogue a **son** bus d'événements en
mémoire, et une écriture de la session 1 n'est pas vue par le réacteur de la
session 2. **Il est retiré**, le nom passe à `--daemon <adresse>` (« je suis le
client de ce démon »), et le témoin `e2e_demon_backends_concurrents.rs` (branche
`memoire-temoin-demon-2`, jamais fusionnée) est abandonné avec lui.

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
  (`Scope { org, project }`, branché). Proposition : le client l'envoie avec
  `open`, pour que deux sessions de deux projets partagent la base sans partager
  la cellule ;
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
- `tools/list` rend **un seul outil**, `etat_du_demon`, dont la description dit
  pourquoi le démon manque et quoi faire — une liste vide sans explication serait
  un défaut, pas une sécurité ;
- tout autre `tools/call` rend `isError` avec la même raison ;
- la ligne sur stderr le dit.

*(L'autre forme, à trancher : refuser `initialize` par une erreur JSON-RPC
nommée. Plus juste pour le protocole, invisible pour le modèle.)*

**Quand le démon meurt entre deux appels**, l'appel suivant passe par `assurer` :
le démon renaît, rouvre le backend, et l'appel en cours rend une erreur
transitoire nommée — jamais un tube cassé, jamais un rejeu silencieux.

**Ce que le client ne fait jamais : ouvrir la base lui-même en repli.** Un repli
silencieux vers « je possède ma base » recréerait le cas que C supprime — deux
écrivains possibles dès que le démon revient — et le cacherait.

**La seule exception, à décider par Lucie** : le mode positionnel
`rag3weaver-backend <backend.json>` reste un hôte qui possède sa base, **parce que
le constructeur de decks s'en sert**. Il parle déjà les messages de C sur stdio :
le jour où le constructeur passe au démon, il change de transport, pas de
messages.

## 5. Le cycle de vie du démon

*Arbre principal, à écrire.* Qui le lance, qui l'arrête, le paquet npm, un démon qui tombe et ce
qu'il emporte avec lui.

## 6. Sur le fil : transport, format, clés

*Les deux sessions, à écrire.* L'exposition par clés se fait à la frontière du démon.

## 7. Les marches, testables une à une, et leur coût

*Les deux sessions, à écrire.*

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

*À écrire en dernier.*
