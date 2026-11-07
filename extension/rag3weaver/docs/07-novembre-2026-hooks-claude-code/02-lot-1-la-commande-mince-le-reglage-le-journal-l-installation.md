# Lot 1 — la commande mince, le réglage, le journal, le silence, l'installation

7 novembre 2026, rag3weaver archi. Le premier des quatre lots de la page 01 (§7). Rien n'est
injecté dans une session : ce lot prouve la mécanique, les blocs viennent au lot 2.

## Ce qui est posé

- **`rag3weaver-backend hook`** (`src/claude_code_hook.rs`, derrière `daemon` et
  `rag3db-native`) : lit le JSON de l'événement sur son entrée, le réglage, joint l'hôte, rend
  son bloc, écrit le journal, **sort en 0 quoi qu'il arrive** — une entrée illisible, un
  réglage fautif, un hôte absent sont des lignes de journal, pas des codes de sortie.
  `--config <chemin>` pour un autre réglage, `--daemon <adresse>` pour un hôte donné (les
  témoins) ; sans `--daemon`, l'hôte est celui de `daemon.json`, et **la commande ne le lance
  jamais**.
- **L'événement** (`Evenement::lire`) : les cinq connus (`SessionStart`, `UserPromptSubmit`,
  `PreCompact`, `PostCompact`, `Stop`) et leurs champs propres (`source`, `trigger`, `prompt`,
  `compact_summary`, `last_assistant_message`, `stop_hook_active`, `context_tokens`) ; un
  événement inconnu est accepté et journalisé.
- **Le réglage** (`~/.config/rag3weaver/hooks.json`, `Reglage`) : manifeste du backend
  mémoire, clés, plafond de contexte (`context_tokens`, défaut 1 000 000, ou
  `RAG3WEAVER_HOOK_CONTEXT_TOKENS`), `suggest_ratio` 0,7, bornes des blocs (1 500 / 3 000
  caractères), délais propres (5 s pour `UserPromptSubmit`, 20 s pour les autres), journal.
  Un champ absent prend son défaut, un champ inconnu est refusé en nommant les connus, un
  fichier absent vaut les défauts.
- **Le client de l'hôte** (`ClientHote`) : `POST /session` (client `claude-code-hook`, le pid),
  `POST /open` du manifeste, des `POST /call`, `DELETE /session` ; le jeton de `daemon.json` en
  `Authorization: Bearer`, la session en `X-Rag3weaver-Session` ; **un délai global par
  requête** ; aucun relancement, aucune relecture du jeton. Derrière le trait `Hote`, pour que
  les témoins jouent sans réseau.
- **Le journal** (`~/.local/state/rag3weaver/hooks.log`) : une ligne datée par événement —
  « hôte joint, backend « … », source compact » ou « silence — <raison> ».
- **`scripts/hooks_claude_code.sh`** : pose le réglage et, **sur `--poser` seulement**, les
  cinq entrées dans `~/.claude/settings.json` (`matcher` sur `SessionStart` et `PreCompact` /
  `PostCompact`, `timeout` = délai propre + 5 s) ; sans `--poser`, il montre et ne touche à
  rien. Idempotent (relancé : « rien à changer ») ; `--retirer` enlève les siennes et rien
  d'autre ; l'ancien fichier gardé en `settings.json.avant-hooks` à la première pose.
  **Rien n'est posé chez Lucie** : c'est son fichier, au mot de c7 et au sien.

## Les témoins

- `claude_code_hook::tests` : les cinq événements lus avec leurs champs ; une entrée sans nom
  ou pas JSON refusée en le disant ; le réglage (défauts, champ inconnu, relecture à
  l'identique, fichier absent) ; ce lot ne rend rien et journalise l'hôte joint ou le silence ;
  **un hôte en carton** (un `TcpListener` du test) : l'ordre `/session`, `/open`, `/call`,
  `DELETE /session`, le jeton et la session sur chaque requête, le corps du `/call` ; un refus
  401 et un `/open` qui échoue sont des raisons dites ; un hôte muet rend la main dans le délai
  (300 ms demandés, moins de 5 s mesurées) ; la commande entière sans hôte (port fermé) : code
  0, sortie vide, journal ; avec l'hôte en carton : code 0, sortie vide, « hôte joint » au
  journal.
- `scripts/test_hooks_claude_code.sh` : un `settings.json` qui porte déjà un autre hook de
  `Stop` — `--montrer` ne touche rien, `--poser` pose les cinq et garde l'autre, une seconde
  pose ne change rien, `--retirer --poser` rend l'original. VERT.

## Ce que la suite précise (réponses de c7 et de 96, 7 novembre)

- **Deux fiches.** L'identité : `name` = la clé stable, ancre `agent:<clé>`, jamais créée par
  le hook. La tâche : `name` = `nom_de_tache` (jamais une date) ; ses apports citent
  `agent:<clé>`. Les sections sont des identifiants : `identity`, `alias`, `handle` sur
  l'identité ; `plan`, `bilan`, `compaction` sur la tâche.
- **La tâche courante d'une session** se lit sans recherche : la fiche d'identité par
  `handle:session:<id>` → son dernier apport `identity` → ses Refs `fiche:` (la rubrique
  `doing`) → la première citée est la courante. `cited_by agent:<clé>` est le repli d'une
  première session.
- **Le hook écrit ce qui est mécanique** (un L1 `handle` par session, écrit une fois si la fiche
  d'identité existe ; le L1 `compaction` de `PostCompact` ; la fin de run), toujours
  `via: hook`, `author` = la clé. **Ce qui est une parole de l'agent reste à l'agent** : après
  une compaction, le bloc de `UserPromptSubmit` du premier tour dit « ta session a été
  compactée ; voici ta fiche d'identité et le résumé ; écris ton apport `identity` par
  `put_apport` avant toute autre chose » ; au tour suivant, si l'apport n'est pas là, le bloc le
  redemande une fois et le journalise. C'est le mot de Lucie du 25 octobre : « un agent est
  challengé de mettre à jour son identité à chaque compaction ».
- **Le remplissage du contexte** se lit sur la dernière ligne `assistant` de la transcription
  (`message.usage` : `input_tokens` + `cache_read_input_tokens` + `cache_creation_input_tokens`),
  pas sur les octets. Mesuré sur 15 transcriptions de ce projet (122 compactions avec usage
  lisible) : médiane 965 k jetons au dernier tour avant compaction, max 967 k — Claude Code
  compacte à ~96 % du million. Les octets entre deux compactions vont de 76 Ko à 10 Mo : ils ne
  disent rien du contexte. L'octet reste le repli, dit comme tel, si aucune ligne n'a d'usage.
- `POST /journal/run` depuis dehors : oui, sous le même jeton, un run de genre
  `claude_code_turn` portant le handle de session ; le juge le juge comme un tour du chat.
