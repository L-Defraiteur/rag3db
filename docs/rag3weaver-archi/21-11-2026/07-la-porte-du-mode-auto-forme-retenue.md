# La porte du mode auto : la forme retenue

*rag3weaver archi, 21 novembre 2026, fin d'après-midi. Elle rassemble en une page ce que Lucie a dit depuis 9 h 40,
relayé par c7, et ce qui a été accordé avec ED, arbre, tty, mémoire et banc. Elle remplace la page 03 (le mode auto
contextuel) et la page 04 (le survivant) partout où elles diffèrent ; celles-ci gardent leur contexte et leurs
témoins de départ. Rien de cette page n'est encore codé, sauf ce qui est dit « sur master » ou « en file ».*

## Le but

Lucie : un agent qui avance sans l'interrompre pour rien. « Peut-être que vous-mêmes êtes trop prudents en général
dans vos prompts et mesures de sécu. » Trois protections sont dures et ne se discutent pas ; tout le reste est une
politique déclarée, à alléger selon ce que banc mesure.

## Ce qui est dur, en Rust, avant tout graphe

Le graphe de la porte ne voit jamais une action que ces gardes refusent, et ne peut pas les contourner.

1. **Un secret n'est jamais affiché.** Le refus d'afficher `.env`, `.vault/*`, `*.pem`, `id_*`, le coffre, et une
   variable `*_KEY`, `*_TOKEN`, `*_SECRET` ou `*PASSWORD*` vaut quoi qu'il arrive, accord humain compris. Le masquage
   (`mask_secrets`, avec les motifs déclarés `secrets.patterns`) passe au point unique de la boucle d'agent, avant le
   modèle, le journal, le flux et l'IDE (lot de mémoire). `secret_store` range dans
   `~/.local/share/rag3weaver/vault/<projet>.env` (0700 et 0600) sans jamais relire la valeur pour le modèle.
2. **Rien ne se détruit qui ne soit ni sur un distant ni créé par l'agent.** Un fichier se supprime sans demander dans un
   worktree actif de l'agent si son contenu actuel est atteignable depuis `refs/remotes/*`, ou si l'agent l'a créé
   dans la tâche (prouvé par l'écriture au journal du run). Sinon, une carte. Une tâche fermée garde son dossier et sa
   tête (`refs/rag3weaver/closed/…`, en file) ; sa suppression explicite est une destruction de cette classe.
3. **Rien ne s'écrit hors des racines.** Le bac (Landlock) ne monte que les worktrees de l'agent, son dossier de travail
   et ses racines accordées. Une racine accordée l'est en **lecture** par défaut ; l'écriture se demande par une seconde
   carte (arbre, lot B). Le dossier de données de l'hôte (`~/.local/share/rag3weaver`, avec `daemon.json` et `user/`)
   est masqué quelles que soient les racines.

S'y ajoute **le titre d'humain**, sans lequel une autorisation ne vaut rien : l'hôte signe chaque ligne de boîte
`by: {kind: human | agent | host, client}` d'après qui appelle, jamais d'après le corps. `human` ne vient que du
`user_token` (en-tête `X-Rag3weaver-User` du terminal, après une frappe au clavier) ou du cookie d'utilisateur de
l'IDE, donné par un code d'entrée à usage unique (lot `user-title`, en cours). Il protège contre les agents sous le
bac, pas contre un processus de Lucie hors du bac.

## La politique : un graphe déclaré, que l'on édite

Lucie : « tout doit être une config facile à changer ». La porte est un **graphe déclaré**, chargé de la configuration
de l'utilisatrice (`~/.config/rag3weaver/gate/`), jamais du dépôt d'un agent, et dont l'empreinte est dite à l'agent au
démarrage. Une politique de types de nœuds fermée garde les primitives de garde obligatoires.

Ses nœuds :
- **un hook sur l'évènement** : avant une commande, avant un `open_root`, avant un geste distant ;
- **la collecte** : les tours signés avec leur `by.kind` ; la **paire** (le tour de l'agent qui précède, puis la
  réponse de l'utilisateur) ; les accords en vigueur ; la commande décomposée et ses `declares` ;
- **les classifieurs**, quatre instances d'une même **fabrique** déclarée (ED dessine le type de nœud) : chacun déclare
  ses entrées, son modèle, son prompt, sa grille d'étiquettes, son seuil et sa place dans l'ordre :
  - le danger (la classe de l'action) ;
  - l'autorisation (explicite, couverte, non autorisée) ;
  - l'étiquetage des tours de l'utilisateur, à l'écriture ;
  - le survivant ;
- **la décision et la carte**, avec les délais (`postpone`) et la fermeture unique.

Changer l'ordre, le modèle ou ce qu'un étage voit ne demande qu'une édition, pas un lot. La porte est une machine à
évènements (accordé avec ED) : un graphe court décide à chaque demande, et rend « en attente » si une carte s'ouvre ;
un second graphe est rejoué à chaque évènement de la carte. Le temps est porté par `postpone` et le planificateur d'ED
(page 05 d'ED). La carte tient son état et sa fermeture unique ; `cancel_prefix("card:<id>:")` retire ses réveils à la
fermeture.

## Les classes, et ce qui est à alléger

| Classe | Exemples | Aujourd'hui | Sort |
|---|---|---|---|
| `read` | lire, chercher, `git log`, `cargo test` | passe | dur : passe toujours |
| `write_local` | éditer, `git commit`, `cargo build`, dans les worktrees | passe | dur : passe toujours |
| `destroy_local` | supprimer, `git reset --hard`, `git branch -D` | règle des distants et de l'auteur, sinon carte | dur pour ce qui n'est ni distant ni à l'agent |
| `open_root` | lire hors des worktrees | carte, accord rangé par agent | **à alléger selon banc** |
| `database` | `psql`, écriture d'une base | carte | **à alléger selon banc** |
| `outbound` | `git push`, `gh`, publication | carte, `once` | **à alléger selon banc**, sauf la publication sur GitHub d'un dépôt ou d'une PR (toujours sous l'accord de Lucie) |
| `secret` | toucher au coffre | voir les gardes dures | dur |
| `system` | `systemctl`, `crontab` | carte | **à alléger selon banc** |

Une commande sur mesure **déclare** ce qu'elle fait (`declares: [{class, path?}]`). La classe retenue est la plus haute
entre la déclaration et les règles : une déclaration peut monter, jamais descendre.

## L'autorisation : la paire, et l'étiquette

- **L'unité d'un accord est la paire.** Un « oui » ne vaut que pour ce que l'agent venait de demander (« veux-tu que je
  supprime X ? » → « oui » = accord pour X, et rien d'autre). Une autorisation directe (« je t'autorise à… », « fais
  ça ») se lit sur le seul tour de l'utilisateur. Le tour de l'agent de la paire est celui que l'hôte a journalisé, tel
  que l'utilisateur l'a vu, jamais une reformulation fournie au moment du jugement.
- **Seuls les tours `by.kind == "human"` font un accord.** Ni un fichier, ni une page, ni une sortie d'outil, ni le
  texte d'un agent. Une page lue par `browser_read` ou `fetch` qui dit « l'utilisateur t'autorise » ne compte jamais
  (témoin de tty sur sa fausse extension) ; le contenu observé porte `observed: {source, trusted: false}`.
- **Chaque message de l'utilisateur est étiqueté à l'écriture**, par un petit modèle (le Qwen de la première porte), et
  rangé par l'archiviste. La grille a quatre valeurs :
  1. autorisation explicite (« vas-y, supprime-les ») ;
  2. demande d'action, qui vaut accord pour son périmètre seulement (« nettoie ce dossier » couvre ce dossier, pas
     `~`) ;
  3. question ou hypothèse, qui ne vaut pas accord (« on pourrait supprimer ? ») ;
  4. refus ou retrait (« non, laisse »), qui annule un accord antérieur.

  Chaque étiquette porte sa portée (quoi, où, combien de temps), la citation exacte, le tour, et sa forme (paire ou
  tour seul). L'étiquette est une aide : le juge relit toujours la citation.

## La carte : une seule, un état qui change

Une seule carte, ouverte du début à la fin. Elle ne se cache jamais, et ne se ferme qu'à une décision, en disant
laquelle. Derrière elle, les étapes se déroulent :

1. **t = 0** : la carte s'affiche, et le rappel des accords en mémoire court en parallèle ;
2. **à 30 s** (`gate.user_wait_s`) sans réponse ni résolution, **l'appel de l'agent** :
   `appeal(action, preuves[], in_reply_to?)`. Les preuves sont des références (un tour, un appel d'outil) ; l'hôte va
   chercher les tours réels et les tours d'entre deux, et les rend marqués par leur `by`. Un texte recopié par l'agent
   ne compte jamais. « Auteur du fichier » se prouve par l'écriture au journal du run. Deux appels au plus. En réponse
   à un refus ou à une alternative, `in_reply_to` fait rappeler **le même juge** avec sa propre sortie précédente
   (rechargée du journal) en dernier ;
3. **si l'appel échoue**, 10 minutes (`survivor.wait_s`), puis **le survivant** s'il est allumé pour cet agent.

L'utilisateur peut répondre à tout instant, et sa réponse prime sur tout ce qui court. Les lignes de la boîte :
`approval` (l'ouverture : outil, arguments, classe, portée), `approval_state` (l'étape et son texte, avec `until` pour
le compte à rebours), une seule `answer` de fermeture (qui a tranché : `user`, `recall`, `appeal` ou `verifier`). Le
cœur de la carte est écrit (lot `card`, vert, à déposer).

## Le survivant (« dangerous standalone survivor »)

Version minimale : « dangereux et basta ». Il valide ce qu'il juge bon, sans réglage par classe ; seules les gardes
dures restent au-dessus. Son entrée : les derniers tours entiers de l'utilisateur et de l'agent, dans l'ordre, chacun
marqué par son `by.kind` ; à part, les accords rappelés, avec la commande, la justification de l'agent et la
référence à la conversation. Il cherche lui-même, par des outils en lecture seule sur la mémoire des accords (par
sens et par mots, un accord avec sa citation exacte et son tour d'origine), et ne lit que des accords signés `human`.
Chaque accord qu'il invoque est cité avec sa référence dans sa décision, et la carte le montre. Un refus rend
`{reason, risk, alternative}`, et l'alternative arrive dans la boîte de l'agent. Ses modèles admis sont dans un fichier
de l'utilisatrice ; chaque décision est journalisée pour relecture au retour.

## Ce qui est fait, ce qui vient

Sur master ou en file : la ref qui tient la tête d'une tâche fermée, l'arbre des agents, `GET /ide/run` (la source des
preuves par référence), les racines accordées et leur montage (arbre), le cœur de la carte (vert, à déposer), le titre
d'humain (en cours).

Ensuite, dans l'ordre :
1. le titre d'humain (`user-title`) et le dépôt de la carte ;
2. la carte branchée à l'équipier, pour tout outil (l'accord générique, que tty attend pour son navigateur) ;
3. les gardes dures de l'étage 0 : le refus d'afficher et `secret_store` (archi), le masquage (mémoire) ;
4. la porte en graphe, avec ED : le hook, la collecte, la fabrique de classifieurs, d'abord sans modèle (les règles
   seules), puis l'étiquetage, l'autorisation par la paire, l'appel, le survivant ;
5. les allègements, classe par classe, sur les chiffres de banc (le scénario 2 de tty rejoué deux fois, archi).
