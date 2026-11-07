# Lot 3 bis — la table des serveurs du run, et le témoin sur une vraie mémoire

7 novembre 2026, rag3weaver archi. Ce que le lot 3 a laissé ouvert, et la leçon des deux défauts
de 73 : **un faux hôte qui répond à tout ne prouve rien sur la forme des Refs**.

## 1. La table `serveurs` dans le run d'un `Stop`

Pour que le juge (73) rejoue un appel `mcp__<serveur>__<outil>` d'un tour de Claude Code sur le
bon backend, le run porte `serveurs: { "<nom du serveur>": "<manifeste>" }` — les seuls serveurs
MCP de la session qui sont **les nôtres** : commande qui finit par `rag3weaver-backend`, premier
argument `mcp`, et un `--manifest <chemin>`. Lus par le hook, processus neuf à chaque appel, dans
`~/.claude.json` (`projects[<cwd>].mcpServers`, puis `mcpServers` à la racine) et dans le
`.mcp.json` du dossier ; `RAG3WEAVER_CLAUDE_JSON` donne un autre fichier (les témoins). Un serveur
qui n'est pas notre binaire, ou sans manifeste, n'y est pas : le juge dit « non rejoué : serveur
inconnu du run ». La table est toujours là, vide s'il le faut.

## 2. Le témoin sur un vrai gabarit `memory` (`tests/e2e_hooks_claude_code.rs`)

L'hôte est réel (`DaemonHost`, un journal des runs temporaire), la mémoire aussi (le gabarit
`memory` livré, une base temporaire, ses signaux par `RAG3WEAVER_EMBED_SERVICE`) ; le hook parle
à l'hôte **en direct** par le même trait `Hote` que son client HTTP (`host.call`, `host.handle`
pour `/journal/run`). Le fil :

1. une session inconnue : la demande du handle, et **rien d'écrit** (aucune fiche `t1`) ;
2. l'agent fait ce que la consigne dit — `put_fiche t1`, le L1 `handle` sous
   `sessions/<uuid>`, `create_ref_type handle`, `citer_source` genre `handle` valeur
   `session:<uuid>` — puis son `identity` (sept lignes) qui cite `fiche:essai des hooks` ;
3. `SessionStart` **retrouve la session par la valeur nue du handle** (le défaut que 73 avait vu),
   rend « Tu es **t1** », la rubrique `doing`, la tâche courante sans plan ;
4. `PostCompact` range le L1 `compaction` sur la fiche de tâche (`via: hook`, `author: t1`) ; le
   `cited_by session:<uuid>` genre `handle` rend la fiche d'identité **et** la fiche de tâche ;
5. `Stop` dépose le run `claude_code_turn` **sous le backend de la mémoire**, relu au journal des
   runs : `genre`, `agent: t1`, `session`, quatre tours dont l'appel
   `mcp__memoire-longue__get_fiche`, `fin: RunFinished`, la table `serveurs`.

Suite `#[ignore]`, jouée par `run_e2e.sh --test e2e_hooks_claude_code` (montée au binaire unique
par `generer.py`) ; dans la passe du lot après la lib entière.

## 2 bis. Deux retouches de l'usage

- **Les deux formes de commande sont acceptées ; la stable est celle qu'on pose.** Le binaire lit
  l'événement dans le JSON de son entrée (`hook_event_name`) et honore `--config` : l'ancienne
  forme `<binaire> hook --config <hooks.json>` (posée par le lot 2) rend le même bloc que la
  stable `<binaire> hook <Événement>` (vérifié par c7 sur le binaire posé : 901 octets, journal
  identique). La session d'essai de Lucie marche donc sans redémarrer. `--poser` **converge** :
  une entrée à lui qui n'a pas la forme stable est réécrite, et il le dit (« commande d'avant
  remplacée par la commande stable ») ; témoin : un settings à l'ancienne forme devient la forme
  stable.
- **`--rejouer`, `--poser-les-gabarits` et `--help` ne lisent pas l'entrée**, et une entrée qui
  est un terminal ne se lit jamais (G : `--rejouer` lancé depuis un terminal attendait sans rien
  dire jusqu'au délai).

## 2 ter. Ce que la vraie mémoire a montré (le témoin de ce lot)

Trois défauts que les faux hôtes cachaient, tous corrigés ici :

- `cited_by` rend chaque fiche sous `{uuid, score, entity, data: {name, apports…}}` : le hook
  lisait `name` au premier niveau, et **aucune session n'était jamais retrouvée** sur une vraie
  mémoire (ni ses diffs de fichiers). Les faux hôtes rendent désormais la forme réelle.
- un nom de fiche a des espaces (« essai des hooks », « bilan : … ») : il court jusqu'à `(`, `)`,
  `;`, `,`, la citation suivante ou la fin de ligne — la même lecture que `whois` (73).
- l'origine d'une Source est `envoi`, `filet` ou `migration` (le schéma refuse le reste) : le L0
  d'une compaction est un **filet** (page M9 du 18 octobre §3), plus `compaction`.

Et, de l'essai de Lucie : le bloc de `UserPromptSubmit` ne redit pas l'identité (le démarrage la
dit), exclut la fiche de la session du rappel, nomme l'échelle du score (« similarité du meilleur
signal, de 0 à 1 — pas le rang fusionné »), ne garde qu'à partir de 0,50 (réglable,
`recall_min_score`, à régler sur les scores du journal) ; une panne du rappel (son délai propre,
`recall_timeout_ms`, 3 s) a sa ligne, « rien en rapport » se tait ; le journal donne la durée de
chaque appel à l'hôte ; un rejeu passe par un hôte en lecture seule (ni écriture, ni run, ni marque).

## 2 quater. Deux fiches citent le handle après une compaction

Le témoin e2e l'a montré au `Stop` (« agent: essai des hooks » au lieu de `t1`) : le L1
`compaction` cite le handle de la session sur la **fiche de tâche**, et le `cited_by` du handle
rend alors deux fiches. Le hook prenait la première. Il prend maintenant celle qui porte un apport
`identity`, et la première seulement si aucune n'en a (une session inscrite sans encore son
identity). `whois` (73) lit, lui, la Ref `fiche` citée par l'identity : le lot 4 n'aura plus ce
choix à faire.

## 3. Ce qui reste

`whois` (sur master, 2f4ed7841) à la place de `cited_by` au `SessionStart` et au rappel — il lit
les Refs `fiche` de l'apport identity, pas le texte : la consigne du défi demandera donc
`citer_source` genre `fiche` pour chaque tâche de `doing` ; la pièce du juge pour `claude_code_turn` (73) ; `ranger` / `archive` (96)
pour `PostCompact` ; la seconde moitié de l'exigence de Lucie (page 04 §1 bis). Et à relire sur
l'essai de Lucie : les scores de rappel au journal, pour régler `recall_min_score`.
