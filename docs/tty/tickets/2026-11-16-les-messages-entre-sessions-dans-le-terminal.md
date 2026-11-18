# Les messages entre sessions dans le terminal de l'agent

- **État** : ouvert. Le rendu est fait sur le faux hôte (tty-23, 18 nov. 2026, branche `cx-tty-23`, page `docs/tty/18-11-2026/04-tty-23-messages-entre-sessions.md`) ; il attend la forme d'archi pour aller sur master.
- **Famille** : produit
- **Gravité** : manque. Sans lui, Lucie ne voit pas ce qu'Algareth envoie aux sessions Claude ni ce qu'il en reçoit (vision `docs/orchestration/visions/16-11-2026-algareth-pair-des-sessions-claude.md`, point 2).
- **Atteignable en service** : oui, dès que le mode pair d'archi pose des messages dans la boîte d'une tâche.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce que c'est

L'agent d'une tâche (Algareth) parle aux sessions Claude par leur bus. Ce qu'il envoie et ce qu'il reçoit arrive au terminal comme une ligne de la boîte de la tâche (`MailboxLine` sur le flux, et à la relecture de `GET /ide/teammate`). Le terminal doit les montrer dans la forme de Claude Code, que Lucie a collée dans la page de vision :

```
  ⎿  “c7 : tête corrigée notée (5081d2281, témoin 21) ; …” → sent to everything declarative — another Claude
     session on this machine
› Message from @everything-declarati…: ED → c7 (copie banc, lucivy, arbre) : la tête corrigée … (ctrl+o to expand)
```

## La forme de ligne proposée à archi (le 16 nov.)

Une ligne de la boîte, même fichier, même `id` stable que les autres :
- `from: "session"` ;
- `peer` : le nom de la session, tel que `ListAgents` le donne (« everything declarative ») ;
- `direction` : `"in"` ou `"out"` ;
- `kind: "message"` ;
- `text` : le message entier ;
- `delivery`, pour `out` : `"delivered"`, `"queued"` ou `"refused"`, avec `delivery_reason` quand ce n'est pas `delivered` ; l'accusé peut arriver plus tard, en ligne `kind: "delivery"` avec `reply_to: <id de l'envoi>` ;
- `name` : celui de l'agent, sur les lignes `out`.

Si la page de relevé d'archi en donne une autre, c'est la sienne qui vaut ; ce ticket s'y cale.

## Ce qui est décidé (c7, 16 nov.)

- **Dans l'historique, repliés**, comme une demande et une réponse :
  - reçu : `› Message de @<peer> : <début du texte>… (Ctrl-O pour tout lire)`, sur une ou deux lignes ;
  - émis : `“<début du texte>…” → envoyé à <peer>`, puis l'accusé (`remis`, `en file`, ou `refusé : <raison>`), ou rien si `delivered` arrive avec l'envoi.
- **Aussi sur la ligne discrète** de la zone vivante (`Vue::discret`), au moment où ils arrivent ou partent.
- **Ctrl-O ouvre le message entier** dans la visionneuse (`src/visionneuse.rs`), comme un appel d'outil.
- **Une demande reçue passe par la porte** comme une demande de Lucie : si elle demande un accord, ce sont les mêmes boutons (`[ Oui ] Non`), avec le `peer` d'origine dit sur la ligne d'état.
- Une ligne s'écrit une fois (règle de `ligne_de_boite` : `id` dédoublonné).

## Où le faire

- `src/boite.rs` : `en_tete` connaît déjà un `from` libre (il s'écrit tel quel) ; ajouter le cas `from: "session"`.
- `src/vue.rs`, `ligne_de_boite` : le rendu replié, l'accusé qui met à jour l'envoi (par `reply_to`), la ligne discrète.
- `src/outils.rs` et `src/visionneuse.rs` : la visionneuse ne connaît que les `Appel` ; lui donner une seconde sorte d'entrée (le message), sans changer ce qu'elle fait des appels.
- `mesures/faux_hote.py` : une route `POST /faux/session {peer, direction, text, delivery?}` qui pose une telle ligne.
- Les noms neufs (fonctions, champs, tests) sont en anglais ; le texte à l'écran reste en français.

## La preuve

- Tests de la vue, rouges d'abord : un message reçu replié avec « (Ctrl-O pour tout lire) » ; un envoi puis son accusé `refused` qui dit la raison ; un message long ouvert entier par Ctrl-O ; une ligne reçue deux fois écrite une fois.
- Le harnais : une séance nouvelle dans `mesures/mesure.py` qui pose un message par `/faux/session` et lit l'écran, sous `tmux-local-ouvert`.
- La passe courte : crate (avec et sans `--no-default-features`), `check_features`, harnais entier.

## Taille et blocage

8 à 10 heures. Le rendu, les tests et le faux hôte peuvent se faire tout de suite, sur la forme proposée ; le branchement réel attend la page de relevé du bus et le mode pair d'archi. Ne rien pousser sur master qui suppose une forme qu'archi n'a pas confirmée.
