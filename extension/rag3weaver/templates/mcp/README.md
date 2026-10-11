# Brancher rag3weaver sur Claude Code, par MCP

`rag3weaver-backend mcp` expose **les outils que le manifeste déclare**, et rien
d'autre. Un serveur = un manifeste. Les deux cas d'usage sont donc la même
mécanique avec deux manifestes différents :

| ce qu'on veut | le gabarit |
|---|---|
| chercher dans un dépôt de code | `templates/backends/code` |
| une mémoire longue, d'une session à l'autre | `templates/backends/memory` |

## 0. La forme d'aujourd'hui : un hôte, des sessions qui lui parlent

Depuis le 11 octobre 2026 (marches 3 et 4 de la forme C), **un seul processus tient
les bases** : l'hôte (`rag3weaver-backend daemon`). Le serveur MCP que chaque session
Claude Code lance est un **client mince** : il n'ouvre ni base ni modèle, il parle
MCP à Claude Code et passe chaque appel d'outil à l'hôte.

```
session Claude Code A ──stdio── rag3weaver-backend mcp ─┐
session Claude Code B ──stdio── rag3weaver-backend mcp ─┼── l'hôte (daemon) ── la base, les modèles, les réactions
session Claude Code C ──stdio── rag3weaver-backend mcp ─┘
```

**Personne ne lance l'hôte à la main.** Le premier client qui ne le trouve pas le
lance, détaché ; les suivants le trouvent par `daemon.json`. L'hôte survit aux
sessions. Deux sessions qui nomment **le même manifeste** partagent **le même
backend** : une base, un écrivain, les réactions montées une fois.

**La recette entière, de zéro, est au §6.** Les sections 1 à 5 disent pourquoi
chaque morceau est là.

## 1. Le manifeste du projet

Copiez le gabarit et changez **trois champs**, pas plus :

```sh
mkdir -p ~/.config/rag3weaver/mon-projet
cp -r extension/rag3weaver/templates/backends/code/* ~/.config/rag3weaver/mon-projet/
```

Dans la copie de `backend.json` :

- `database` → un chemin absolu à vous, p. ex. `~/.cache/rag3weaver/mon-projet.rag3db` ;
- `workspace.root` → **la racine du dépôt** à fouiller, en absolu ;
- `vector_extension` → le chemin absolu de `libvector.rag3db_extension` dans
  l'arbre où le moteur a été bâti.

**Et copiez tout `templates/`, pas le seul dossier du gabarit** — corrigé le
11 octobre après avoir joué la recette : le gabarit `code` range ses graphes
**hors de son dossier** (`../../tools/*.mmd`), donc copier `backends/code/` seul
donne un manifeste qui ne charge pas. Réécrire les chemins en absolu n'est pas la
réponse non plus : pointés sur un worktree, ils meurent avec lui. Copier
l'arborescence préserve les chemins relatifs et rend le manifeste autonome.

```sh
cp -r <dépôt>/extension/rag3weaver/templates ~/.config/rag3weaver/mon-projet
# puis éditer mon-projet/backends/code/backend.json
```

**Avec `--standalone`, gardez la session ouverte pendant le premier index.** `index`
rend aussitôt et travaille en fond, mais ce travail est un **enfant du serveur** :
fermer le tube le tue (mesuré deux fois). Sous l'hôte, le travail vit dans l'hôte ;
un premier index par l'hôte n'a pas encore été joué. Une fois l'index fait, la base est persistée et la
session peut fermer — y compris après un rebâti du moteur. Défaut connu :
`docs/tickets/2026-10-11-indexation-enfant-du-serveur-mcp.md`.

La recette complète, avec les temps mesurés et une question réelle, est à
`docs/10-octobre-2026-18h00/04-une-base-par-session-recette-jouee.md`.

**Ne retirez pas `workspace.index`.** Mesuré le 10 octobre 2026, après deux
passes perdues : ce n'est pas seulement l'indexation qu'il commande, c'est la
déclaration du schéma de code (`File`, `Scope`, `Symbol`). Sans lui, même
`read_file` répond « unknown entity: File ».

## 2. Le `.mcp.json`

Posez `claude-code.exemple.mcp.json` à la racine du dépôt sous le nom
`.mcp.json`, en remplaçant les quatre chemins marqués `À REMPLACER`.

**Et sachez ce que ça fait** : un `.mcp.json` à la racine d'un dépôt est chargé
par **toute** session Claude Code ouverte dedans. Sur un dépôt partagé entre
plusieurs sessions, ce n'est pas un réglage personnel — c'est un réglage commun.
Pour un essai, préférez la configuration utilisateur (`claude mcp add`) ou un
dépôt à vous.

## 3. Les variables d'environnement, et pourquoi chacune

Sous la forme du §0, **donnez-les toutes au client** : c'est lui qui lance l'hôte
quand il manque, et l'hôte hérite de son environnement. Le client lui-même ne se
sert que de `LD_LIBRARY_PATH`.

- `LD_LIBRARY_PATH` → `<bâti>/src`, pour que `librag3db.so` se charge. Sans
  elle, le serveur meurt au démarrage sans rien dire d'utile ;
- `RAG3DB_ROOT` → l'arbre où le moteur est bâti. **Jamais dérivé du chemin du
  code** : depuis un worktree, le code vit ici et l'extension est ailleurs
  (piège 6 du journal des chantiers) ;
- `RAG3WEAVER_EMBED_SERVICE` → les adresses **complètes** du service
  d'embarquement (`127.0.0.1:7878,127.0.0.1:7879`), jamais les ports seuls. Le
  gabarit de code déclare des vecteurs, donc il en exige un.

## 4. Comment le client trouve l'hôte, et ce qu'il fait sans lui

**Par défaut** (aucun drapeau) : le client lit `~/.local/share/rag3weaver/daemon.json`,
que l'hôte écrit en démarrant (adresse réellement servie, pid, jeton local, bâti ;
mode 0600). Si l'hôte de ce fichier répond, le client s'y attache. Sinon — pas de
fichier, ou un fichier périmé (pid mort, hôte muet) — il **lance**
`rag3weaver-backend daemon` (même exécutable, `127.0.0.1:7970`), détaché, son journal
dans `~/.local/state/rag3weaver/daemon.log`, et attend qu'il réponde. Deux clients
qui démarrent ensemble ne lancent qu'**un** hôte (verrou sur `daemon.lock`).

Au démarrage, le client dit à qui il parle :

```
[mcp] memory — 10 outils, clés présentées : dev — client mince de l'hôte, session s-…
[mcp] 1 réaction(s) montée(s) par l'hôte : review_anchored
```

- **Le jeton local.** Chaque requête à l'hôte porte `Authorization: Bearer <jeton>`
  (celui de `daemon.json`), sauf `GET /sante`. Sans lui : 403. Pour regarder à la
  main : `curl -s -H "Authorization: Bearer $(jq -r .token ~/.local/share/rag3weaver/daemon.json)" http://127.0.0.1:7970/etat`.
- **L'outil `daemon_status`**, exposé par le client lui-même : l'état de l'hôte
  (adresse, bâti, sessions, backends ouverts), ou, s'il manque, la raison, le
  journal à lire et quoi faire. C'est l'outil à appeler quand un autre échoue.
- **L'hôte ne peut pas être joint** (le port est pris, l'hôte meurt au démarrage) :
  le client démarre quand même. `initialize` dit pourquoi, seul `daemon_status` est
  listé, et tout autre appel est une erreur d'outil qui le dit — et retente de
  joindre l'hôte. Claude Code ne relit pas la liste d'outils de lui-même : quand
  l'hôte répond de nouveau, `/mcp` pour reconnecter.
- **Un client meurt sans fermer sa session** (tué, ou la sonde de `claude mcp get`) :
  l'hôte connaît son pid, et ferme sa session au relevé suivant (toutes les 10 s, et à
  chaque lecture de l'état). `daemon_status` compte ces fermetures
  (`closed_dead_sessions`).
- **L'hôte meurt pendant une session** (tué, arrêté) : l'appel suivant le relance,
  rouvre la session et rejoue l'appel une fois — la connexion refusée prouve qu'il
  n'avait pas été reçu. Une erreur transitoire (la base se rouvre pendant l'appel),
  elle, se dit et ne se rejoue pas.

**`--daemon <adresse>`** force une adresse : ni lecture de l'hôte à lancer, ni
relance. Le jeton vient de `daemon.json` s'il décrit cette adresse.

**`--standalone`** : la forme d'avant le 11 octobre, le serveur **possède** sa base.
Une seule session à la fois (une base rag3db ne s'ouvre que par un processus), et un
travail long (`index`) meurt avec la session (ticket
`docs/tickets/2026-10-11-indexation-enfant-du-serveur-mcp.md`).

**`--demon` (l'ancien drapeau) est refusé** en nommant `--daemon`.

**Ce qui manque encore** : la notification `tools/list_changed` (les outils du
manifeste apparaissent à la prochaine lecture de la liste, pas d'eux-mêmes) ; une
unité systemd (une option du paquet, plus tard) ; le remplacement d'un hôte d'un bâti
plus ancien (aujourd'hui il est gardé tant qu'il répond — `daemon_status` dit son
bâti).

## 5. `--keys` : déclaré n'est pas exposé

Chaque déclaration portera une `exposure` (une expression sur des clés), et le
serveur n'exposera que ce que ses clés rendent vrai — **rien par défaut**. La
lecture des clés est en place ; le filtrage viendra avec elle. Le serveur
annonce au démarrage ce qu'il expose et sous quelles clés :

```
[mcp] code-poste — 13 outils, clés présentées : dev
```

Le jour où des déclarations seront écartées, cette ligne est ce qui évitera de
chercher pourquoi la liste est vide.

## 6. La recette, de zéro

Pour une mémoire partagée entre toutes vos sessions Claude Code.

**1. Un binaire de la marche 4.** Un binaire d'avant ne connaît pas `daemon_status`
(et, d'avant le 11 octobre après-midi, ni `daemon` ni `--daemon`). Le rebâtir et le
copier **hors de tout `target`** (un target s'efface) :

```sh
cd <dépôt>/extension/rag3weaver
RAG3DB_SHARED=1 RAG3DB_ROOT=<dépôt> \
RAG3DB_LIBRARY_DIR=<dépôt>/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=<dépôt>/build/lecteurs-csv/src \
cargo build -j 8 --release --features rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres \
  --bin rag3weaver-backend
cp <target>/release/rag3weaver-backend ~/.local/bin/
```

**2. Le manifeste**, comme au §1 : une copie de **tout** `templates/`, avec
`database` et `vector_extension` en absolu. Pour la mémoire longue :
`~/.config/rag3weaver/memoire-longue/backends/memory/backend.json`.

**3. Chaque session**, une fois, en configuration **utilisateur** (pas un
`.mcp.json` de dépôt partagé, voir §2). L'environnement complet, puisque ce client
lancera l'hôte s'il manque :

```sh
claude mcp add memoire-longue \
  --env LD_LIBRARY_PATH=<dépôt>/build/lecteurs-csv/src \
  --env RAG3DB_ROOT=<dépôt> \
  --env RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7980 \
  -- ~/.local/bin/rag3weaver-backend mcp \
     --manifest ~/.config/rag3weaver/memoire-longue/backends/memory/backend.json \
     --keys dev
```

Chemins en absolu dans la vraie commande. `RAG3WEAVER_EMBED_SERVICE` prend des
adresses **complètes** ; la mémoire longue déclare bge-m3, servi sur 7980.

**C'est tout.** La première session lance l'hôte — et `claude mcp get <nom>` aussi, qui lance le serveur pour le sonder : l'hôte peut donc être déjà là avant la première session, c'est voulu ; `/mcp` montre les outils, dont
`daemon_status`. Les sessions suivantes trouvent l'hôte. Pour voir l'état : l'outil
`daemon_status`, ou `curl` avec le jeton (§4).

**Arrêter l'hôte proprement**, si un jour il le faut (un nouveau binaire installé) :
`kill -TERM $(jq .pid ~/.local/share/rag3weaver/daemon.json)`. Il ferme ses bases,
retire `daemon.json` et sort ; la session suivante en relance un. Jamais `kill -9` :
une base rag3db tuée en pleine écriture garde un journal à rejouer.

## Ce que le serveur ne fait pas

Ni `resources`, ni `prompts`, ni `sampling`, ni `roots` — **absents et déclarés
absents** dans ses `capabilities`, plutôt que silencieux. Pas de transport HTTP :
stdio seulement.

## Si ça ne marche pas

- **le serveur meurt au lancement** : presque toujours `LD_LIBRARY_PATH`. Lancez
  la même ligne à la main, l'erreur de chargement s'affiche ;
- **la liste d'outils est vide** : lisez la ligne `[mcp] …` sur stderr, elle dit
  combien d'outils et sous quelles clés ;
- **un outil dit que « cette base n'a pas le schéma du code »** : votre manifeste
  ne déclare pas `workspace.index: "code"`, donc le schéma du code n'a pas été
  posé à l'ouverture. Déclarez-le, **puis** appelez l'outil `index` (`estimate`
  dit ce que ça coûtera). Le refus le dit lui-même depuis le 11 octobre 2026
  (`9dc3f083a`) ; avant, il ne disait que « unknown entity: File ».
- **seul `daemon_status` est listé** : l'hôte n'a pas pu être lancé ou joint.
  Appelez-le : il dit pourquoi et où est le journal
  (`~/.local/state/rag3weaver/daemon.log`) ; puis `/mcp` pour reconnecter ;
- **`option inconnue`** sur un drapeau de ce README : le binaire est ancien,
  rebâtissez-le (§6, étape 1) ;
- **le serveur s'arrête avec le code 75** (avec `--standalone`) : la base demande une réouverture, et
  il vous l'a dit par un `notifications/message` avant de partir. Relancez-le.
