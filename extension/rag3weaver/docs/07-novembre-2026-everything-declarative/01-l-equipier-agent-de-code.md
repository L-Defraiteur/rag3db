# L'équipier, agent de code fonctionnel

Page de conception, 7 novembre 2026, everything declarative ; à c7 avant le code. Le mot de Lucie,
par c7 : « le faire se comporter comme un vrai agent de code, qui peut communiquer avec Claude Code,
se lancer des sous-sessions et communiquer avec elles et inversement, faire du code, et lui
demander à chaque fois s'il a eu des problèmes avec ses outils standards avant de s'intéresser à
sa partie mémoire ». Et la condition absolue du même jour : « si défaillance du moteur
d'ingestion, elle ne plante jamais et se comporte comme un Claude Code normal ».

L'équipier existe (lot équipier-1, page `docs/25-octobre-2026-everything-declarative/07-l-equipier.md`) :
une boucle de pas, chacun un tour de chat dans une tâche (worktree + copie de base), un mini-plan,
une boîte aux lettres avec un superviseur, le juge sur chaque pas. Cette page dit ce qui lui manque
pour être un agent de code qu'on fait travailler pour de vrai, dans l'ordre de c7. Identifiants en
anglais, le reste en français.

## Ce qui est là (relevé sur master du 7 novembre)

- **Les outils d'un pas** : ceux du manifeste (`BackendTools`, `backend_tools.rs:12-58`, les
  graphes `templates/tools/*.mmd`), enveloppés par `DeclarationTools` puis `OutilsDuChat`
  (`chat.rs:419-440`), plus ceux du pas (`StepTools`, `teammate.rs:574-600` : `edit_plan`,
  `step_done`, `suggest`, `report`), douze appels par pas (`gate`, `teammate.rs:488-495`).
- **Ce qui dépend du moteur** : tout passe par `Backend::call` sous le verrou de lecture du slot
  (`with_backend`, `daemon_host.rs:402`), pour tout le tour. `read_file`, `grep_files`,
  `list_files` lisent le disque (`FileSource`, le worktree de la tâche par `surcharges_de`) mais
  s'annotent par le catalogue ; `edit_file` écrit puis réindexe par le catalogue
  (`edit_file_shared_with_hash`) ; `search_code`, `outline`, `usages`, `impact` interrogent l'index.
- **Ce qui casse aujourd'hui** : une erreur d'outil est déjà rendue au modèle (`{"error": e}`,
  `backend_tools.rs:51-57`) et le pas continue. Mais un backend qui ne s'ouvre pas, ou qui se ferme
  pendant l'appel (`NOT_HOSTED … fermé pendant cet appel`, `daemon_host.rs:436`), fait rendre `Err`
  à `chat_turn_with` ; `teammate_loop` le propage (`teammate.rs:784`) et **tout le lancement de
  l'équipier s'arrête** (`ended "erreur : …"`, `teammate.rs:736-751`). Une fermeture demandée
  pendant un pas attend la fin du tour (le verrou est tenu).
- **Les commandes** : la porte est déclarée (`workspace.commands` : `off`, `standard`,
  `approval`, `auto`, `backend_code.rs:40-43` ; le gabarit `code` dit `approval`), l'outil existe
  (`run_command` → `RunCommandNode`, sortie entière en journaux `.out/.err`, suivie par
  `wait_output`), le bac à sable aussi (Landlock, `workspace.sandbox`), sans shell (`sh -c` ne
  passe jamais la liste, `commande.rs:11-20`), et `cargo check`, `cargo test` sont dans la liste
  de lecture (`LECTURE_SEULE`, `commande.rs:116`). **Mais le chat et l'équipier ne l'ont pas** :
  `OutilsDuChat` retire tout outil `policy.run_commands` (`chat.rs:424-425`).
- **La boîte** : `<tâche>/mailbox.jsonl`, une ligne `{at, from, kind, text, item?}`, publiée sur
  `mailbox.<task>` ; `GET /ide/teammate`, `POST /ide/teammate/mailbox` (`ide.rs:664-…`). Le
  superviseur n'a pas d'identité : `from: "supervisor"`.
- **Les tâches** : `ouvrir_une_tache(parent, nom)` part du HEAD du dépôt du manifeste parent ; ni
  `Tache` ni l'équipier ne connaissent de tâche parente.
- **Le juge** : `bilan_des_outils` existe (`juge.rs:291`, schéma
  `templates/agents/juge/bilan_des_outils.schema.json` : par outil, appels, échecs, inutiles, part
  utilisée, défauts bloquant / gênant / mineur), seulement si le manifeste déclare `bilans`.
  `step_done` porte `{item, succeeded, done, remaining}`.
- **Le hook UserPromptSubmit** (`claude_code_hook.rs:727-…`, archi) ne lit ni l'équipier ni sa boîte.

## 1. Les outils standards, complets et étanches

**Ce que l'équipier voit, par source de défaillance** :

| défaillance | aujourd'hui | après |
|---|---|---|
| le backend ne s'ouvre pas (base abîmée, lib refusée, format) | le lancement meurt | le pas tourne en **mode fichiers** : les outils de fichiers sur le worktree de la tâche, les outils d'index « indisponible : <cause d'ouverture> » |
| le backend est fermé pendant un pas (`/backend/close`, hôte qui recharge) | `Err`, le lancement meurt | l'appel en cours rend son erreur ; les appels suivants du même pas passent en mode fichiers |
| l'index est en retard ou sous un verrou d'écriture long (un `index` en cours) | l'appel attend derrière | au-delà d'un délai annoncé (`RAG3WEAVER_TEAMMATE_INDEX_WAIT_S`, 20 s par défaut), le repli sur les fichiers, qui le dit |
| un graphe d'outil échoue (nœud en erreur) | `{"error": …}` au modèle, le pas continue | pareil, et le juge le compte en repli d'outil |
| l'hôte meurt | le pas meurt avec lui, la tâche reste « en cours » | le pas interrompu est journalisé comme tel ; à la reprise de l'hôte, l'équipier dont le plan est vivant et sans `ended` reprend à la frontière suivante |

**Le mode fichiers** (`FileTools`, à côté de `BackendTools`, dans `teammate.rs` ou un module à
lui) : `read_file`, `list_files`, `grep_files`, `edit_file` implémentés directement sur le
worktree de la tâche, chemins confinés à ce worktree (comme `FileSource`), **la même forme de
résultat** que leur graphe, avec un champ `fallback: "index indisponible : repli sur les fichiers
(<cause>)"`. `edit_file` y garde l'empreinte attendue (`expected_hash`, calculée sur le disque) et
laisse l'index en dette : la montre réindexe le fichier quand le backend revient. `search_code` y
devient un grep des mots de la requête, rendu dans la forme de `search_code` (`result[]`, un
signal `grep_repli`, la ligne de `warnings` qui le dit). `outline`, `usages`, `impact` rendent
`{"ok": false, "unavailable": "<cause>"}`. Le pas ne tient plus le verrou du slot pour tout le
tour : chaque appel d'outil le prend pour lui (une fermeture passe entre deux appels).

**Les commandes, par la porte déclarée** : dans une tâche, l'équipier reçoit `run_command` et
`wait_output` quand `workspace.commands` n'est pas `off`, sous la même `Garde`, avec le bac à sable
confiné au worktree de la tâche (et son target). Les commandes de lecture (`cargo check`,
`cargo test`, `git diff`…) passent sous `standard` ; sous `approval`, une commande hors liste part
en question dans la boîte (`kind: "approval"`, la commande en argv) et le pas continue sans elle ;
la réponse arrive à la frontière. Sortie entière journalisée dans le dossier de la tâche. Pas de
shell libre. Les scripts du dépôt (`scripts/*.sh`) : déclarés au manifeste par leur chemin
(`workspace.scripts: ["scripts/check_features.sh"]`), exécutés en argv, jamais par `sh -c` —
**question 1**.

**Témoins (faux modèle, `tests/common/fake_sse.rs`)** : un pas qui lit, édite et cherche pendant
que le backend est fermé par `/backend/close` au milieu (le pas finit, chaque retour après la
fermeture porte `fallback`, l'édition est sur le disque, le plan intact) ; un backend qui ne
s'ouvre pas dès le départ (le pas tourne en mode fichiers) ; un `index` qui tient le verrou (le
repli après le délai, dit) ; `cargo check` lancé, sa sortie dans le journal ; une commande hors
liste sous `approval` (la question dans la boîte, le pas continue) ; l'hôte arrêté au milieu d'un
pas puis relancé (le pas interrompu journalisé, la reprise à la frontière).

## 2. Parler avec Claude Code, dans les deux sens

- **L'adresse** : une ligne de boîte gagne `to` (facultatif) et `from` porte une clé stable :
  `claude-code:<session_id>` pour une session Claude Code (la clé que le hook connaît),
  `teammate:<parent>/<task>` pour un équipier, `supervisor` pour l'IDE. La `request` qui lance
  l'équipier fixe son superviseur : ses questions vont par défaut à l'auteur de la demande.
- **De l'équipier vers Claude Code** : une question, un `report`, un `ended` adressés à
  `claude-code:<id>` restent « en attente » tant qu'ils n'ont pas de réponse. Une route de l'hôte
  les rend : `GET /ide/teammate/pending?for=claude-code:<id>` → `[{task, item?, kind, text, at}]`.
  Le hook UserPromptSubmit (archi) les montre en tête de son bloc : « l'équipier <task> te
  demande : … — réponds par `rag3weaver-backend hook reply <task> "<texte>"` ».
- **De Claude Code vers l'équipier** : `POST /ide/teammate/mailbox` avec `from:
  claude-code:<id>` (la commande `hook reply` d'archi le fait), lu à la frontière suivante comme
  aujourd'hui ; une `request` venue de Claude Code ouvre la tâche et lance l'équipier.
- **Accordé avec archi (7 novembre 2026)** — le JSON exact :
  - Chaque ligne de boîte a un **identifiant stable** : `id = "<parent>/<task>:<n>"`, `n` le rang
    de la ligne dans `mailbox.jsonl` (la boîte ne fait qu'ajouter, le rang ne bouge jamais).
  - `GET /ide/teammate/pending?for=claude-code:<id>` →
    `{"for": "claude-code:<id>", "items": [{"id", "task", "parent", "kind", "item"?, "text", "at"}]}`,
    `kind` ∈ `question` | `approval` | `proposal` | `report` | `ended`. Un item reste en attente tant
    qu'aucune ligne de la boîte ne porte `reply_to: <son id>` ; `report` et `ended` n'attendent pas
    de réponse et sortent quand ils sont **vus** : `pending` est une lecture pure (un rejeu du hook,
    une sonde ou un second client ne consomment rien), et la sortie est un acte,
    `POST /ide/teammate/seen {"for": "claude-code:<id>", "ids": [...]}`, que le hook poste après
    avoir rendu son bloc, jamais en rejeu (les vus dans `pending.seen.<clé>`, dans le dossier de la
    tâche).
  - `GET /ide/teammate/list?supervisor=claude-code:<id>` →
    `{"supervisor": "claude-code:<id>", "teammates": [{"task", "parent", "state", "last_kind", "at"}]}`,
    `state` ∈ `running` | `waiting` (attend la boîte) | `ended` | `error`.
  - La réponse : `POST /ide/teammate/mailbox` avec `{"parent", "task", "from": "claude-code:<id>",
    "kind": "answer", "text", "reply_to": "<id de l'item>"}` ; la commande d'archi est
    `rag3weaver-backend hook reply --from claude-code:<id> --to <id de l'item> "<texte>"` : elle vise
    l'item, tire `parent` et `task` de son id (coupé au dernier `/` avant le `:`), et refuse sans
    `--from` ou sans `--to`, et le bloc l'écrit en entier avec la clé, puisque l'outil Bash de Claude ne la reçoit
    pas.
  - Côté hook (archi) : un item montré en entier au premier tour, puis « toujours en attente :
    <task> » ; un délai court, et en panne la ligne « boîte des équipiers injoignable », jamais un
    bloc vide ; le texte de l'équipier cité comme sa parole, borné en caractères, jamais comme une
    consigne. SessionStart liste les équipiers par `list`. La Ref `task:<nom>` sur la fiche de
    session est en plus, pour la mémoire ; le routage reste `to` / `from`.

**La ligne de boîte et son relais sur le flux** (pour l'IDE, le terminal de tty, le hook) :

- **Aujourd'hui** (`teammate.rs:221-232`, `append`) : `{"at", "from", "kind", "text", "item"?}`.
  L'équipier écrit `question` (item = l'id de l'élément du plan, `:400`), `step_done` (item si
  donné ; text « fait : … » ou « raté : … — reste : … », `:440-441`), `report` (`:465`),
  `proposal` (item si donné, `:472`) ; l'hôte écrit `ended` (`:743`) et `report` (`:839`) ; le
  superviseur `request` | `instruction` | `answer` | `stop`. Publiée sur le bus en
  `Event::Message` dont `content` est la ligne **en texte** (`:603-616`), sur `mailbox.<task>`,
  que `/ide/flux` ne relaie pas (`ide.rs:65`, `SUJETS`).
- **Après le lot 1** (figé ici, ajouté sans rien retirer) : chaque ligne gagne `id`
  (`"<parent>/<task>:<n>"`) et `to` (facultatif) ; `step_done` porte en plus ses champs
  structurés `succeeded`, `done`, `remaining` (et `tool_issues` au lot 3), `text` restant la
  phrase lisible ; `approval` (lot 2) porte `command` (argv) ; `answer` porte `reply_to` (et
  `approved` pour une approbation). Le flux relaie un sujet `mailbox` : enveloppe habituelle
  `{"sujet": "mailbox", "backend": "<manifeste de la tâche>", "evenement": {"kind": "MailboxLine",
  "parent", "task", "line": {…la ligne telle qu'écrite…}}}` — la ligne en objet, jamais en
  texte. L'`Event::Message` actuel reste pour les abonnés du bus. `GET /ide/teammate` rend aussi
  `backend`, le manifeste de la tâche (celui des trames du flux), pour filtrer le flux par tâche.

**Témoins** : un équipier qui pose une question à `claude-code:s1` ; `pending?for=claude-code:s1`
la rend, une autre clé ne la voit pas ; la réponse postée sous `claude-code:s1` est lue à la
frontière du pas suivant ; le bloc du hook (faux hôte, côté archi) la montre.

## 3. Les sous-équipiers

- **L'ouverture** : un outil du pas, `spawn_teammate {item, request}`, ouvre une sous-tâche sur un
  élément du plan : sa branche `agent/<parent>--<enfant>` part du HEAD de la branche du parent,
  après un commit des éditions du parent (`wip(équipier): avant <enfant>`), son worktree, sa copie
  de base (demain sa vue), sa boîte, son journal. `Tache` gagne `parent_task: Option<String>`.
- **Les bornes** : profondeur 2 (un enfant n'ouvre pas de petit-enfant au-delà), 3 enfants vivants
  par équipier, 6 équipiers vivants par hôte ; au-delà, l'outil rend son refus comme résultat.
- **La parole** : le superviseur de l'enfant est le parent (`teammate:<parent>`). Les questions,
  `step_done` et `ended` de l'enfant se recopient dans la boîte du parent (`kind: "child"`, avec
  le nom de l'enfant) et se lisent à la frontière du parent ; le parent répond par
  `tell_teammate {child, text}` ; il voit les pas de l'enfant par la même recopie, et par
  `GET /ide/teammate` pour l'humain.
- **La fin** : l'enfant clos rend sa branche ; le parent la fond dans la sienne par
  `merge_child {child}` (même chemin que `fusionner_une_tache`, cible la branche du parent), ou la
  rejette — **question 2**. Effacer le worktree de l'enfant à la fusion suit la règle des tâches
  jetables (lot « tâche jetable »).

**Témoins** : un parent qui ouvre un enfant sur un élément ; l'enfant pose une question, le parent
la voit à sa frontière et répond, l'enfant lit la réponse ; les pas de l'enfant recopiés ; la
borne de profondeur et de nombre refusée en résultat ; la fusion de l'enfant dans la branche du
parent.

## 4. Les problèmes d'outils, dits par l'agent à chaque pas

- `step_done` gagne `tool_issues`, **requis** (une liste, vide si rien) : `[{tool, kind, detail}]`,
  `kind` ∈ `ambiguous_result` | `missing_parameter` | `too_slow` | `wrong_result` | `refused` |
  `fallback` | `other`. La consigne du pas dit qu'on le remplit avant de parler du reste.
- Le journal du pas le porte ; la boîte aussi (dans la ligne `step_done`).
- Le juge, dans `bilan_des_outils`, rend en plus `concordance[]` : par outil, ce que l'agent a dit,
  ce que le juge a vu, accord ou désaccord ; un repli (point 1) est compté en défaut d'outil, jamais
  en faute de l'agent. Pour une tâche d'équipier, le bilan se demande toujours (sans attendre
  `bilans` au manifeste) — **question 3**.

**Témoins** : un `step_done` sans `tool_issues` est refusé en résultat (le pas le redit) ; le
journal et la boîte le portent ; le bilan du juge (faux modèle du juge) rend la concordance.

## 5. La mémoire en dernier

Rien de la mémoire ne bloque un pas : le rangement du plan clos (`file_closed_plan`), la fiche du
juge, les rappels. Une erreur de mémoire s'écrit dans la boîte (`kind: "memory_failed"`, la cause)
et dans le journal, le pas et le plan continuent. La frontière d'un pas ne fait aucun rappel de
mémoire tant que les points 1 à 4 ne sont pas sur master.

**Témoin** : la mémoire longue absente (hôte de mémoire injoignable) pendant un plan entier : le
plan finit, la boîte porte un `memory_failed` par rangement manqué.

## Puis : une vraie tâche sur Gemini

Une fois 1 à 5 sur master : un lot réel du dépôt donné à l'équipier sur Gemini 3.8 Flash, jugé ;
puis la même tâche sur un second modèle (« Gemini Argon » selon Lucie, à vérifier par G sous le
projet Vertex), les deux bilans comparés par le juge.

## Les lots

1. Étanchéité et mode fichiers (point 1 sans les commandes), avec le verrou par appel.
2. Les commandes par la porte déclarée.
3. `tool_issues` et la concordance du juge (petit, utile dès la vraie tâche).
4. Claude Code ↔ équipier (avec archi).
5. Les sous-équipiers.
6. La mémoire non bloquante (peut se fondre dans 1).

## Réponses de c7 (7 novembre 2026)

1. **Les commandes autorisées se déclarent au manifeste**, jamais en dur dans le code :
   `workspace.commands` (la porte) existe ; `workspace.scripts` nomme chaque script par son nom
   et sa commande exacte (`{"name": "check_features", "command": ["bash",
   "scripts/check_features.sh"]}`), exécutée en argv, comptée comme écriture sauf mention.
   C'est générique, et c'est ce que Lucie veut pouvoir redéclarer.
2. **La fusion d'une sous-tâche dans la tâche parente est décidée par le parent seul**, son
   superviseur, avec le bilan du juge de l'enfant sous les yeux (`merge_child` le lui rend avant
   d'agir), et le parent le dit dans son plan (l'élément porte la décision et sa raison). La
   fusion du parent dans master reste à un humain.
3. **Le bilan du juge est toujours demandé pour une tâche d'équipier**, désactivable par le
   manifeste seulement (`teammate.judge: false`), jamais par l'agent.

Six lots, codés après les vues 5, 6, 7 et 3b, dans l'ordre de la page ; le lot 1 (étanchéité)
d'abord ; chacun sa passe et son push au mot de c7.
