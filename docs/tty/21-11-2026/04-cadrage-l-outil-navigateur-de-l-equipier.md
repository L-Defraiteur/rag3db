# Cadrage : l'outil navigateur de l'équipier

Session tty, 21 novembre 2026. Lucie, le 20 novembre à 23 h 1x : « l'extension navigateur est bien plus prio ». Cette page dit ce que l'équipier pourra faire dans le navigateur, ses gardes, ses témoins et sa taille. La forme des outils est à accorder avec archi avant toute ligne de code.

## Ce qui existe (sur master)

Le cœur Rust du pont est fait : `src/pont_navigateur/` (lot d'archi du 15 novembre ; conception `docs/rag3weaver-archi/15-11-2026/04-conception-coeur-pont-navigateur.md`).
- **Le protocole de l'extension** : fork L-Defraiteur de jackwener/BrowserMCP, révision 5. Une `Commande` porte une action libre (`action`, `page`, `url`, `act`, `code`…), et une réponse porte `{id, ok, data, error, errorCode}`. Les événements `tab_*` tiennent la table des onglets.
- **L'écoute** : `Ecoute::ouvrir` crée un socket Unix 0600 sous `$XDG_RUNTIME_DIR/rag3weaver/`, avec un pont par navigateur (`browserId`). Le relai `rag3weaver-pont-navigateur` (Native Messaging) recopie les octets de Chrome vers ce socket, et le manifeste n'admet que l'extension de Lucie.
- **Les gardes du pont** :
  - les passe-partout sont fermés par défaut (`cdp`, `chrome-call`, `cookies`, la capture réseau, les journaux de l'extension…) : refus `action_closed`, sans envoi ;
  - un onglet rendu à l'humain (passage de main) ne se reprend que par un nouveau `claim` ;
  - les délais (`timeout`) et la déconnexion sont gérés.
- **21 témoins** sur une fausse extension par socket : le cadrage, la révision, les délais, les baux, les passe-partout, le relai, l'écoute.

## Ce qui manque

- **Personne n'ouvre l'écoute** : ni l'hôte, ni une route, ni un outil.
- **Aucun outil** de l'équipier, de l'atelier ni MCP n'appelle le pont.
- **Aucune garde de domaine ni d'identifiants** dans le pont.
- Les sources de l'extension ne sont pas dans ce dépôt : la liste exacte des gestes de `act` (cliquer, remplir) et la lecture de la console sont à relire dans le fork.

## Ce que l'équipier pourra faire

Une famille « navigateur » de la palette, déclarée au gabarit, moins de dix outils, bornée par `teammate.*` (le socle d'archi, page 02 du 20 novembre). Proposition, à accorder avec archi :

| Outil | Geste | Classe |
|---|---|---|
| `browser_tabs` | les onglets que l'équipier peut prendre, et prendre l'un d'eux (`claim`) | lecture |
| `browser_read` | lire la page : l'arbre ARIA, le texte, l'URL, le titre | lecture |
| `browser_console` | lire la console de la page | lecture (si le fork l'offre sans passe-partout) |
| `browser_open` | aller à une URL, dans l'onglet pris ou un onglet neuf | écriture |
| `browser_click` | cliquer un élément de l'arbre ARIA | écriture |
| `browser_fill` | remplir un champ | écriture |
| `browser_handoff` | rendre la main à l'humain (« à toi »), puis la reprendre | pas du run |

Le JavaScript arbitraire (`exec`) reste fermé, comme les passe-partout : il n'entre pas dans cette famille.

**La forme, accordée avec archi le 21 novembre** (chaque paramètre a sa phrase de description, que le témoin `every_parameter_of_the_teammates_tools_is_described` exige) :
- `browser_tabs {claim?: tab_id}` → `[{tab_id, url, title, claimed_by}]` ;
- `browser_read {tab_id, mode: "aria" | "text" | "url", max_chars?}` → `{url, title, content, truncated, masked_fields}` ;
- `browser_console {tab_id, since?}` → `[{at, level, text}]` ;
- `browser_open {url, tab_id?}`, `browser_click {tab_id, ref}`, `browser_fill {tab_id, ref, value}` → `{ok, url_after, navigated, …}`. `ref` est la référence ARIA rendue par `browser_read`, jamais un sélecteur CSS libre ;
- `browser_handoff {tab_id, reason}` → une ligne `handoff` dans la boîte ; le pas attend sa réponse.

**La palette** : une famille « browser » à part, déclarée au gabarit, et allumée par `teammate.browser`. Absente si le manifeste ne la déclare pas, comme `web_fetch` et `web_search` de mémoire.

**L'écoute** : une fois pour tout l'hôte, ouverte au premier manifeste qui déclare `teammate.browser`, et dite à `/sante` (le socket, les onglets réclamés). L'extension ne connaît qu'une adresse. Les domaines restent par manifeste, donc la garde par tâche.

## Les gardes

1. **Jamais d'identifiants.**
   - `browser_fill` refuse un champ de mot de passe (`type=password`, `autocomplete` en `*-password`, `one-time-code`), un champ de carte bancaire (`cc-*`), et tout champ que l'arbre nomme « mot de passe », « code », « token » ou « secret ».
   - `browser_read` masque la valeur de ces champs.
   - Les cookies et le stockage restent fermés, comme au pont.
   - Le refus est dit à l'équipier, avec le passage de main proposé : c'est l'humain qui se connecte.
2. **Jamais hors d'un domaine déclaré.**
   - `teammate.browser.domains` au manifeste (vide par défaut, donc rien n'est permis) ; `browser_open` et tout geste sur une page hors de cette liste sont refusés sans envoi.
   - Une redirection hors liste (l'événement `tab_navigated`) suspend l'onglet jusqu'à un accord.
3. **L'humain pour tout envoi.** (Il n'y a pas aujourd'hui de voie d'accord générique : celle de `run_command` lit `args.command`, et son « oui » vaut pour toute la tâche. Le lot « l'accord d'un outil » l'ouvre : une ligne `approval {tool, args, scope: "once" | "task"}`, posée pour tout outil d'écriture déclaré `needs_approval`, et un « oui » qui rejoue l'appel exact une fois quand `scope` vaut `once`. `browser_click` et `browser_fill` sont toujours `once`.)
   - Lire est libre dans les domaines déclarés.
   - Tout geste d'écriture (`browser_open`, `browser_click`, `browser_fill`) passe par une carte d'accord, comme une commande : dans la boîte, au terminal (o / n) et dans la page de la tâche.
   - Un clic qui soumet un formulaire, ou dont la cible est un bouton d'envoi, ne s'accorde jamais pour toute la tâche : un accord par envoi.
4. **Bornes** : `teammate.browser.max_actions` par pas, un délai par commande, une sortie de `browser_read` bornée et dite (`[…]`).

## Les témoins, rouges d'abord, sans réseau ni Chrome

Sur la fausse extension des témoins du pont (un socket Unix), branchée à un hôte de test :
- l'hôte ouvre l'écoute quand le manifeste déclare `teammate.browser`, et la ferme avec lui ;
- `browser_read` sur un onglet pris rend l'arbre ARIA borné, avec le champ de mot de passe masqué ;
- `browser_open` hors domaine est refusé sans rien envoyer (la fausse extension compte zéro commande) ;
- `browser_fill` sur un mot de passe est refusé sans envoi, et dit le passage de main ;
- `browser_click` sur un bouton d'envoi pose une carte d'accord ; le « oui » de la page envoie la commande, le « non » ne l'envoie pas ;
- le passage de main : « à toi », puis « fait », puis un nouveau `claim` qui compare l'URL attendue.

La qualification sur le vrai Chrome du portable vient après, avec Lucie (l'extension installée par elle).

## La taille

Trois lots, chacun avec ses témoins et son attestation, plus le lot de l'accord, avant le lot 2 :
1. **L'hôte ouvre l'écoute, et la lecture** : `browser_tabs`, `browser_read`, `browser_console` ; garde des domaines et masquage des identifiants. Environ 400 lignes.
- **L'accord d'un outil** (`approval {tool, args, scope}`, `needs_approval`, le « oui » qui rejoue l'appel une fois) : archi le propose, avant le lot 2.
2. **Les gestes d'écriture et leurs accords** : `browser_open`, `browser_click`, `browser_fill` ; refus des identifiants, un accord par envoi. Environ 400 lignes.
3. **Le passage de main comme pas du run**, et la carte « à toi » au terminal et dans l'IDE. Environ 300 lignes.

## Questions

- **À Lucie** (aucune ne bloque le lot 1) :
  - l'installation de l'extension sur le Chrome du portable, et l'écriture du manifeste Native Messaging : ce sont ses gestes ;
  - les domaines à déclarer pour le premier essai ;
  - si le fork expose la console sans passe-partout, ou s'il faut l'y ajouter.
