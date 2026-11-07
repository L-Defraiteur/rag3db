# Les essais de l'équipier, nuit du 7 au 8 novembre 2026

Lucie, le 7 novembre au soir : « fais des essais seule, et demain je regarderai les sessions
d'exemple que tu as faites, en essayant de les mettre sur un vrai taff quand même », et la
méthode : « faut s'attendre à ce que ça marche pas direct… itère à chaque fois, regarde bien les
retours de ses outils et les params, s'il est bloqué quelque part, et itère ». Cette page est ce
journal : chaque essai, ce que l'équipier a fait, où il a buté, ce que j'ai dit, ce que le juge
en a dit, et ce qui en est sorti pour le code.

## Le cadre

- **L'hôte de Lucie** (7970), donc le binaire que G a posé à 19 h 57 (master 17a094d12 côté
  moteur, Rust de la pose du soir) : aucun lot de la nuit n'y entre avant la pose suivante de G
  (8 h 30). Tout ce que je constate ci-dessous vaut pour ce binaire ; les correctifs d'ED
  nés de ces essais (plus bas) sont sur master, pas encore dans l'hôte.
- **Un arbre de déclarations à moi**, pour ne pas toucher le manifeste de Lucie :
  `~/.config/rag3weaver/code-essais-c7/` (un dépôt git, comme tout arbre de déclarations), inscrit
  par `rag3weaver-backend code-tree add ~/git_workspaces/rag3db-essais-c7 --from code-rag3db --as code-essais-c7`
  (2,3 s, base copiée par reflink). Backend `e4d8635ecfe7`, base
  `~/.local/share/rag3weaver/code-essais-c7.rag3db`. Le worktree `~/git_workspaces/rag3db-essais-c7`
  (branche `essais-c7`, master 5eb6fb277) est jetable ; il s'efface à la fin avec ses branches `agent/*`.
- **Une tâche** = `POST /ide/teammate/mailbox {backend, task, kind:"request", text}` ; l'hôte ouvre
  `~/.local/share/rag3weaver/taches/e4d8635ecfe7/<tâche>/{arbre,code,base,mailbox.jsonl,plan.json}`,
  un worktree `agent/<tâche>` depuis le dépôt, et lance l'équipier par pas (Gemini Flash, projet
  Vertex de Lucie). Je lis sa boîte (`GET /ide/teammate?backend=&task=`) et le journal de l'hôte.
- Trois consignes prêtes, des tickets réels et bornés :
  1. `journaux-annulation-tmp` : les `*.undo.json` de plus de 64 Ko laissés dans
     `/tmp/rag3weaver-checkpoints` à la fin d'un graphe (ticket du 4 octobre) ;
  2. `racine-disparue` : un backend dont la racine a disparu ;
  3. `tests-bases-dans-tmp` : des tests qui laissent des bases dans /tmp.
  Chacune dit l'ordre attendu : lire le ticket et le code, témoin ROUGE d'abord, correctif au
  plus étroit, rejeu, ticket fermé, commit sur sa branche, et « écris dans ta boîte à chaque pas ».

## Essai 1 — `journaux-annulation-tmp` (00 h 03 → 00 h 13)

**Ce qu'il a fait.** Douze itérations, toutes en lecture : il a audité les manifestes
(`list_files`, `read_file` sur `templates/`) à la recherche d'un outil d'écriture, puis a fini
par un texte, sans une ligne dans sa boîte. Le run se termine `Eos, ok:true`.

**Pourquoi.** Deux causes, les deux à moi et au produit :
- le manifeste de la tâche n'avait **ni `edit_file` ni `run_command`** : une tâche prend les
  déclarations **au HEAD du dépôt d'arbre**, et mon ajout d'`edit_file` n'était pas commité ;
  `code-tree add` pose `commands: "off"` ;
- un pas qui finit **sans `step_done`** ne laissait rien dans la boîte : le superviseur ne voit
  qu'un silence.

**Le juge** (bilan de la mémoire longue, source `2026-11-07-agent-1a1b84639f6-315.md`) : note
**1/10**, « absence totale d'outils d'écriture et d'exécution » — juste.

**Ce que j'ai fait.** Commité dans l'arbre `edit_file`, `run_command` (porte `approval`) et
`commands: "approval"` (422636b) ; l'hôte a relu le manifeste (« le manifeste a changé — le
backend est fermé et rouvert », 14 outils) ; relancé en `journaux-annulation-tmp-2` avec un
paragraphe de plus dans la consigne (ses outils, et « écris dans ta boîte à chaque pas »).

**Ce qui en est sorti pour le code** (ED, lot `equipier-jamais-muet-2`, 9914ab946, ticket dans le
commit) : (1) la `request` qui ouvre une tâche écrit dans la boîte « tâche ouverte : branche
agent/<nom> depuis <commit> ; ses déclarations et son code sont ceux de ce commit (une
modification non commitée n'y est pas) », et la réponse du POST porte `task: {branche, depart}` ;
(3) un pas qui finit sans `step_done` écrit sa réponse finale en `report` dans la boîte ;
(2) `commands: "off"` n'est pas un réglage de `code-tree` : le chat et l'équipier retirent
toujours `run_command` aujourd'hui (`OutilsDuChat`) — c'est le **lot 2 de la page 01 d'ED**, les
commandes par la porte déclarée, qu'ED avance cette nuit avant le réacteur : sans lui l'équipier
ne joue aucun test, donc aucune vraie tâche de code.

## Essai 2 — `journaux-annulation-tmp-2` (lancé 00 h 18)

**Ce qu'il a fait.** Un plan en quatre items (`lire_ticket_code` → `red_test` → `corriger` →
`doc_ticket_commit`), puis le pas 1.

**Où il a buté.** Pas chez lui : chez Vertex.
- 00:28:35, report de l'hôte : « le pas 1 n'a pas abouti : llm indisponible : service en panne
  (HTTP 503) après **0 reprise(s) en 600,1 s** » — un 503 attendu dix minutes sans une reprise :
  la politique de reprise du client LLM ne voit pas ce 503 (noté par ED pour après le lot 2) ;
- 00:33:50, « le pas 2 n'a pas abouti : llm indisponible : quota dépassé (HTTP 429,
  RESOURCE_EXHAUSTED) après 1 reprise(s) en 286 s » ; le pas repart seul à 00:33:54.
  Consommation de l'hôte sur l'heure : 92 appels Gemini entre 23 h 50 et minuit (le run de
  l'essai 1 et ses bilans), une dizaine ensuite ; les passes d'ED (e2e_juge, e2e_equipier) tirent
  sur le même projet depuis luciepc.
- Six lignes « [crochet read_file] tu — un état, pas un contenu » : l'après-outil `impact_fichier`
  (« ce fichier, qui l'utilise ») a rendu un état de l'index (partiel, occupé ou jamais indexé —
  le premier index de mon arbre n'était pas fini) et l'hôte l'a tu pour ne pas répéter la ligne
  d'état de l'outil ; le `read_file` lui-même a rendu le fichier. Rien à corriger ; ED note que le
  libellé pourrait dire lequel des trois.

**Comment il a fini.** 00:37:52 : « le pas 3 n'a pas abouti : quota dépassé (HTTP 429) après
2 reprise(s) en 169 s », puis `ended` : « 3 pas d'affilée n'ont pas abouti,
teammate.max_failed_steps = 3 ». Le plan est resté à son premier item (`lire_ticket_code` :
doing), aucun pas déclaré, le worktree intact. **Il n'a pas eu le temps de se tromper** : les
trois vies de la tâche sont parties en 503 et 429 du fournisseur. Pas de bilan du juge (rien à
juger). Deux constats de plus pour ED : un 429 ou un 503 ne devrait pas consommer une vie de
`max_failed_steps` (c'est le fournisseur, pas le pas) ; et la reprise sur 429 devrait tenir au
moins la fenêtre d'un quota par minute, avec un recul croissant, au lieu de rendre après deux
essais. Les crochets de Lucie (`hooks.log`) ne voient aucun 429 ce soir ; la passe d'ED
(e2e_juge, e2e_equipier à luciepc, même projet) a démarré vers 00:33.

*(suite au fil de la nuit)*
