# Le pont navigateur par MCP : étude des trois ponts à extension, et la conception du cœur Rust

15 novembre 2026, nuit. Réponse d'étude au
[ticket du 12 novembre](../tickets/2026-11-12-etudier-un-pont-navigateur-ouvert-par-mcp.md), sur les critères de la
[vision](../../orchestration/visions/2026-11-12-un-pont-navigateur-generaliste-par-mcp.md). **Lecture des sources seule** :
rien n'a été bâti, installé ni exécuté. Les échanges cités sont reconstruits depuis le code, pas
capturés sur le fil. Le ticket reste ouvert : la qualification réelle et le prototype sont des
étapes à planifier.

## Les trois ponts, au SHA lu

| | Playwright Extension (`--extension`) | BrowserMCP/mcp | jackwener/browsermcp |
|---|---|---|---|
| SHA lu | `bac1f9f67` (11 nov.) | `33da4f6a3` (tête `a51ddc4ee`, une ligne d'AGENTS.md de plus) | `d6597aacb` (0.0.42 ; quatre commits plus loin non lus) |
| Licence | Apache-2.0 | Apache-2.0 ; « renommer une version modifiée publiée » | Apache-2.0 ; embarque le script injecté de Playwright (Apache, généré, 338 Ko) |
| Transport | WebSocket ; l'**hôte écoute** sur un port éphémère, lance Chrome sur `connect.html?mcpRelayUrl=…` | WebSocket ; l'hôte écoute sur 9010–9012, l'extension sonde `/browsermcp/status` puis se connecte | **Native Messaging** : l'extension lance l'hôte (stdin/stdout, longueur sur 4 octets + JSON) |
| Hôte distant | non : la page de connexion refuse tout sauf 127.0.0.1 | non sans tunnel ; la signature couvre le port composé | non : Native Messaging impose un processus local |
| Appairage | choix d'onglet par l'humain, ou jeton dans l'URL | **clé ECDSA P-256 de l'hôte, engagement SHA-256, code à six chiffres, bouton Allow** | aucun secret : `allowed_origins` de Chrome et socket 0600 |
| Ce que fait l'extension | **relai `chrome.debugger` brut** (5 commandes, 5 événements) | relai CDP mince **avec une politique** : refuse cookies, Storage, la plupart de `Target.*`, sérialise `Input.*`, Stop/Resume | **presque tout** : instantané ARIA et références, actions (clic, saisie, envoi de fichier), captures, JavaScript, attentes de chargement |
| Ce que fait l'hôte | Playwright entier par `connectOverCDP` | playwright-core 1.65 entier par un faux point CDP | des commandes typées (`claim`, `act`, `exec`, `screenshot`…) ; un REPL Node par session |
| Pont hôte à transcrire | ~916 lignes TS → 300–500 lignes Rust | ~1 000 lignes TS → du même ordre | ~765 lignes TS → **800–1 200 lignes Rust, observation et action comprises** |
| Ce qui reste à refaire au-dessus | observation, actions avec actionnabilité, attentes : **le moteur de Playwright** (des milliers de lignes, script injecté compris) | idem | rien pour observer et agir ; le signal « l'humain a fini », la garde du passage de main |
| Emprunt d'un onglet humain | l'onglet choisi à la connexion, ou glissé dans le groupe | onglet exposé, `attach {tabId}` ; groupes par application | **`claim` avec `expectedUrl`/`expectedTitle`, `release`, `mark: handoff`** ; origine `user` ou `agent` sur chaque onglet |

Les chiffres de transcription sont des ordres de grandeur donnés à la lecture, pas des mesures.

## Ce que cela dit

- **Deux familles.** Playwright et BrowserMCP gardent une extension mince et mettent l'intelligence
  dans l'hôte, qui est Playwright. Transcrire leur pont en Rust est court, mais il resterait à
  refaire tout le moteur d'observation et d'action. C'est le portage large que la vision écarte.
  jackwener met l'intelligence dans l'extension (le script de Playwright injecté dans un monde
  isolé) : l'hôte n'envoie que des commandes typées. **C'est la seule des trois où « transcrire
  seulement la partie qui parle à l'extension » donne observation, actions et JavaScript.**
- **Le passage de main n'est complet nulle part.** jackwener a les bons états (emprunt, restitution,
  `handoff`), mais pas de signal « l'humain a fini ». De plus, une commande qui nomme l'onglet
  explicitement n'examine pas l'état du bail : elle reprendrait un onglet en `handoff` sans nouvel
  emprunt. Aucun des trois ne détecte une saisie humaine pendant le contrôle.
- **La sécurité est meilleure chez BrowserMCP** : appairage signé avec code, et une politique CDP qui
  refuse ce qui atteint d'autres sites. jackwener expose au contraire des passe-partout (`cdp`,
  `chrome-call`) que notre cœur doit fermer par défaut.
- **La topologie** (agent sur luciepc, Chrome sur le portable) n'est résolue par aucun des trois :
  l'hôte du pont tourne à côté de Chrome, et c'est à nous de le relier à l'agent.

## Recommandation : forker jackwener/browsermcp

Garder son extension (TypeScript, MV3) et remplacer son hôte Node par un hôte Rust qui parle son
protocole (révision 5), en empruntant à BrowserMCP sa politique CDP et l'idée de l'appairage
signé. Les adaptations du fork, à porter dans l'extension :

1. **Le bail en `handoff` se garde** : toute commande vers un onglet en `handoff` ou `released`
   est refusée, onglet nommé compris, jusqu'à un nouveau `claim`.
2. **Les passe-partout sont fermés** : `cdp` et `chrome-call` refusés sauf liste blanche
   (la politique de `cdp-policy.ts` de BrowserMCP en point de départ).
3. **Renommer** l'extension et son identifiant, épingler la révision du protocole, et suivre l'amont
   par sa révision plutôt que par sa version.

## La conception du cœur Rust, en quatre lignes

1. **Une bibliothèque `navigateur` dans rag3weaver**, appelable sans MCP :
   - le cadrage Native Messaging et la corrélation par `id`, avec un `deadlineAt` relié au
     `CallDeadline` de l'hôte ;
   - `hello`/`ready` et le contrôle de révision ;
   - les types serde de `Command`, `Result`, `BrowserEvent` et `ActSpec` ;
   - les baux d'onglet (`claim`, `release`, `handoff`) avec leur origine `user` ou `agent`.

   L'exécutable que Chrome lance par Native Messaging est un mince relai vers l'hôte rag3weaver
   du poste. L'hôte le relie à l'agent, où qu'il tourne : luciepc parle au portable par le chemin
   des hôtes, pas par un port du navigateur. La façade MCP, pour les agents externes, appelle la
   même bibliothèque, sans logique à elle.
2. **Chaque action est un outil de l'atelier, sous ses accords** :
   - observer (instantané ARIA) est une lecture ;
   - naviguer, cliquer, saisir et envoyer un fichier sont des écritures, avec une carte d'accord ;
   - exécuter du JavaScript est de la classe la plus haute ;
   - l'annulation passe par le `CancelToken` du run.

   Le journal du run garde chaque commande et son résultat, avec l'origine de l'onglet.
3. **Le passage de main est un pas du run, pas une notification** :
   - l'agent demande, avec l'onglet et ce qu'il attend : le bail passe en `handoff`, le débogueur
     se détache, et la carte « à toi » s'affiche à Lucie dans l'atelier ;
   - Lucie agit dans son Chrome puis répond « fait » ;
   - le cœur refait un `claim` avec l'URL attendue, observe, compare à l'état d'avant, et ne
     reprend que sur une observation conforme ;
   - un onglet fermé, une URL inattendue ou une annulation donnent un état nommé, jamais une
     réussite implicite ;
   - le journal marque l'intervalle comme humain.
4. **Rouge d'abord, sur un faux pont** : un faux processus d'extension qui parle le protocole
   (révision 5) sur stdin/stdout joue `claim`, `aria`, `act`, `handoff`, puis un `exec` vers
   l'onglet en `handoff`, qui doit être refusé. La qualification sur le vrai Chrome du portable
   suit le parcours de la vision : connexion de test, intervention, reprise, onglet fermé. Aucun
   compte réel.

## Ce qui reste à vérifier avant un prototype

- les quatre commits de jackwener après `d6597aacb`, et que l'extension publiée correspond à ces
  sources (sinon, ne charger que la nôtre, non empaquetée) ;
- le bandeau de débogage de Chrome pendant `chrome.debugger.attach`, et ce que voit Lucie quand elle
  intervient sur un onglet détaché ;
- `observeFrame` et la liste blanche `checkPublicCdp` de jackwener, lues en partie seulement ;
- le REPL persistant (JavaScript Node côté hôte) : l'abandonner, ou le remplacer par un `exec`
  répété dans la page ; c'est la seule capacité de jackwener que le cœur Rust ne reprend pas tel
  quel.

## Les décisions qui reviennent à Lucie

- le socle : jackwener (le plus court vers observer et agir), ou BrowserMCP (le meilleur appairage,
  mais il faut un moteur à la place de Playwright) ;
- installer l'extension du fork non empaquetée, ou passer par le Web Store ;
- le fork distant sous son compte : rien n'est créé par cette page.
