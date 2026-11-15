# Le pont navigateur : l'extension du fork et le relai Native Messaging, la conception

15 novembre 2026, midi. La suite du [cœur Rust](04-conception-coeur-pont-navigateur.md), qui est
sur master (`36ee817a7`). **Conception seule** : aucun code, aucun fork distant et aucune
installation avant le mot de Lucie sur le compte GitHub (c7). Les points du code amont sont lus
dans jackwener/browsermcp au SHA `d6597aacb`.

## La conception, en quatre lignes

1. **Le fork de l'extension : peu de changements, chacun dans son commit, pour suivre l'amont.**
   - **L'identité** : un nom à nous (le README de BrowserMCP le demande pour une version modifiée ;
     ici, c'est pour ne jamais confondre avec l'extension publiée). Une `key` neuve dans
     `extension/manifest.json`, donc un identifiant d'extension neuf. Un hôte natif à nous :
     `NATIVE_HOST_NAME` passe de `com.browsermcp.host` à `com.rag3weaver.pont_navigateur`
     (`src/protocol.ts`).
   - **Le fil reste en révision 5**, celle que le cœur parle. Le `hello` annonce une capacité de
     plus, `rag3weaver-gardes`, pour que l'hôte sache qu'il parle au fork.
   - **La garde du passage de main, dans l'extension aussi** : dans `resolveTab`
     (`extension/src/sessions.ts`, vers la ligne 373), un bail dont `state` n'est pas `active`
     rend `page_handed_off`. Aujourd'hui, une commande qui nomme la page prend l'onglet dès qu'un
     bail existe, `handoff` compris. L'hôte refuse déjà avant l'envoi ; l'extension refuse aussi
     si un autre client lui parle.
   - **Les passe-partout fermés dans l'extension** : `cdp`, `chrome-call`, `chrome-describe` et
     `extension-logs` (`background.ts`, lignes 71, 72, 103) refusent `action_closed`, sauf ce
     qu'une liste du build ouvre (vide par défaut). **`cookies` est retiré** :
     `handleCookies` rend la valeur de tous les cookies d'un domaine, `httpOnly` compris
     (`background.ts`, vers la ligne 290).
   - **Un test** : l'amont n'en a aucun sur les baux de l'extension. Le fork ajoute un test vitest
     de `resolveTab`, avec un `chrome` simulé : un bail `handoff` refusé, un nouveau `claim` qui le
     reprend.
2. **Le relai : un exécutable mince, `rag3weaver-pont-navigateur`, que Chrome lance.**
   - **Ce qu'il fait** : il se connecte à l'hôte rag3weaver du poste par un socket Unix
     (`$XDG_RUNTIME_DIR/rag3weaver/pont-navigateur.sock`, en mode 0600), puis recopie les octets
     dans les deux sens. Il ne comprend pas les trames, et n'écrit rien d'autre que les trames sur
     sa sortie standard, qui est le canal de Chrome : tout journal va sur l'erreur standard. Il
     s'arrête quand l'un des deux côtés se ferme.
   - **Si l'hôte ne répond pas**, le relai s'arrête en le disant sur l'erreur standard, et
     l'extension réessaie (attente doublée de 1 à 30 s, côté amont).
   - **Le `Pont` vit dans l'hôte**, sur le socket accepté : c'est le `Read` et le `Write` du cœur,
     sans un second code de protocole. Il y a un pont par `browserId` ; une nouvelle connexion du
     même navigateur remplace l'ancienne, et l'hôte le dit.
   - **Le manifeste d'hôte natif de Chrome**
     (`~/.config/google-chrome/NativeMessagingHosts/com.rag3weaver.pont_navigateur.json`, avec le
     chemin du relai et `allowed_origins` limité à notre seule extension) s'écrit par une commande
     d'installation, **seulement sur le mot de Lucie**, comme les hooks. L'extension se charge non
     empaquetée dans son Chrome : c'est son geste.
3. **Ce qui vient après, sur ce socle (pas dans le lot suivant)** :
   - les routes de l'hôte vers le pont, puis les outils de l'atelier sous leurs accords ;
   - le passage de main comme pas du run ;
   - la façade MCP.

   La topologie se règle là : Chrome et le relai sont sur le portable, à côté de son hôte ; un
   agent de luciepc passe par le chemin entre hôtes qui existe déjà, et jamais par un port du
   navigateur.
4. **Rouge d'abord** :
   - **le relai**, par des tests Rust : un faux Chrome écrit des trames sur l'entrée du relai, un
     faux hôte écoute sur le socket. Les octets passent dans les deux sens, à l'identique. Le
     relai s'arrête quand l'hôte ferme, puis quand Chrome ferme. Sa sortie standard ne porte que
     des trames, même quand l'hôte est absent.
   - **le pont par le socket** : le cœur, sur un socket accepté, rejoue la garde du passage de main.
   - **l'extension**, par le test vitest du fork.
   - **la qualification sur le vrai Chrome du portable**, au mot de Lucie et sans compte réel :
     `claim`, observation, `handoff`, reprise.

## Ce qu'il faut de Lucie

- **Le compte GitHub du fork** (c7 le lui demande).
- **Le nom de l'extension.** Je propose « rag3weaver — pont navigateur ».
- **Son mot pour poser le manifeste d'hôte** dans son profil Chrome, puis pour charger l'extension.

## Où en est le fork (15 novembre, 12 h 4x)

- **Le fork** : https://github.com/L-Defraiteur/BrowserMCP, créé au mot de Lucie. La branche
  `rag3weaver-gardes` porte trois commits :
  - le rouge `047785c` ;
  - les gardes `e8bef9f` : handoff gardé dans `resolveTab`, passe-partout fermés par
    `extension/src/gardes.ts`, cookies retiré, capacité `rag3weaver-gardes` ;
  - l'identité `0f9e2a5`.
- **L'identité** :
  - le nom : « rag3weaver pont navigateur » ;
  - l'hôte natif : `com.rag3weaver.pont_navigateur` ;
  - **l'identifiant de l'extension : `amleopmgbfaknijhmkefmghlgnoimibj`**, tiré de la clé
    publique du manifeste ;
  - la clé privée est dans `~/git_workspaces/rag3db/.vault/pont-navigateur/extension-key.pem`
    (mode 600, ignorée par git), et n'est jamais dans un dépôt.
- **Vérifié dans le fork** : vitest 39/39, dont les 11 témoins des gardes ; tsc de l'extension
  et des tests ; `npm run build` à 0. L'extension bâtie (`extension/dist/`) porte le nom, la
  clé, l'hôte natif et la capacité, et plus le gestionnaire de cookies.
- **Pas fait** : rien n'est installé chez Lucie. Le chargement non empaqueté et le manifeste
  d'hôte natif attendent son mot.
