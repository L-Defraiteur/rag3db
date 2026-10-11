# Brancher rag3weaver sur Claude Code, par MCP

`rag3weaver-backend mcp` expose **les outils que le manifeste déclare**, et rien
d'autre. Un serveur = un manifeste. Les deux cas d'usage sont donc la même
mécanique avec deux manifestes différents :

| ce qu'on veut | le gabarit |
|---|---|
| chercher dans un dépôt de code | `templates/backends/code` |
| une mémoire longue, d'une session à l'autre | `templates/backends/memory` |

## 0. La forme d'aujourd'hui : un hôte, des sessions qui lui parlent

Depuis le 11 octobre 2026 (marche 3 de la forme C), **un seul processus tient les
bases** : l'hôte, lancé par `rag3weaver-backend daemon`. Chaque session Claude Code
lance son serveur MCP avec `--daemon <adresse>` ; ce serveur est un **client mince**.
Il n'ouvre ni base ni modèle : il parle MCP à Claude Code et passe chaque appel
d'outil à l'hôte.

```
session Claude Code A ──stdio── rag3weaver-backend mcp --daemon ─┐
session Claude Code B ──stdio── rag3weaver-backend mcp --daemon ─┼── l'hôte (daemon) ── la base, les modèles, les réactions
session Claude Code C ──stdio── rag3weaver-backend mcp --daemon ─┘
```

Deux sessions qui nomment **le même manifeste** partagent **le même backend** dans
l'hôte : une base, un écrivain, les réactions montées une fois. C'est la forme pour
une mémoire partagée entre sessions, et pour n'importe quel backend.

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

**Sans `--daemon`, gardez la session ouverte pendant le premier index.** `index`
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

Sous la forme du §0, elles vont **à l'hôte** ; le client mince ne garde que
`LD_LIBRARY_PATH`. Sans `--daemon`, le serveur en a besoin des trois.

- `LD_LIBRARY_PATH` → `<bâti>/src`, pour que `librag3db.so` se charge. Sans
  elle, le serveur meurt au démarrage sans rien dire d'utile ;
- `RAG3DB_ROOT` → l'arbre où le moteur est bâti. **Jamais dérivé du chemin du
  code** : depuis un worktree, le code vit ici et l'extension est ailleurs
  (piège 6 du journal des chantiers) ;
- `RAG3WEAVER_EMBED_SERVICE` → les adresses **complètes** du service
  d'embarquement (`127.0.0.1:7878,127.0.0.1:7879`), jamais les ports seuls. Le
  gabarit de code déclare des vecteurs, donc il en exige un.

## 4. Avec ou sans `--daemon`

**Avec `--daemon <adresse>`** (la forme du §0) : l'hôte possède la base, N sessions
lui parlent. Le client n'a besoin que de `LD_LIBRARY_PATH` (le binaire se lie à
`librag3db.so`) ; le service d'embarquement et `RAG3DB_ROOT` sont l'affaire de
**l'hôte**, qui ouvre la base et les modèles. Au démarrage, le client dit à qui il
parle :

```
[mcp] memory — 9 outils, clés présentées : dev — client mince de l'hôte 127.0.0.1:7970, session s-…
[mcp] 1 réaction(s) montée(s) par l'hôte : review_anchored
```

- **L'hôte n'est pas là** : le serveur s'arrête aussitôt (code 1) et le dit —
  `pas de session auprès de l'hôte http://127.0.0.1:7970 : … ne répond pas`. Claude
  Code montre alors le serveur en échec : lancez l'hôte, puis `/mcp` pour
  reconnecter.
- **L'hôte redémarre pendant une session** : à l'appel suivant, le client rouvre sa
  session une fois et rejoue l'appel. Une erreur transitoire (la base se rouvre),
  elle, se dit et ne se rejoue pas.

**Sans `--daemon`**, le serveur **possède** sa base, comme avant le 11 octobre :
bon pour un dépôt de code et une seule session. Mais Claude Code lance **un
processus MCP par session**, et une base rag3db ne s'ouvre que par un processus :
une deuxième session sur la même base est refusée par le moteur. Et un travail long
(`index`) est un enfant de ce serveur : il meurt avec la session (ticket
`docs/tickets/2026-10-11-indexation-enfant-du-serveur-mcp.md`).

**`--demon` (l'ancien drapeau) est refusé** en nommant `--daemon` : il ne déportait
que la connexion, et chaque session gardait son catalogue et ses réactions.

**Ce qui manque encore** (marche 4 de la forme C, pas faite) :

- **la découverte** : l'adresse de l'hôte s'écrit à la main dans chaque
  configuration ; pas encore de `daemon.json` où un client la lirait ;
- **le lancement par le premier client** : l'hôte se lance à la main (ou par votre
  gestionnaire de services). Le premier client qui ne le trouve pas ne le lance pas,
  il s'arrête en le disant ;
- **un outil « état du démon »** côté MCP : en attendant, `curl
  http://127.0.0.1:7970/etat` dit les backends ouverts et les sessions.

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

## 6. La recette, de zéro (jouée le 11 octobre 2026)

Pour une mémoire partagée entre toutes vos sessions Claude Code. Jouée telle quelle
le 11 octobre sur une copie du manifeste : l'hôte a ouvert la base une fois, deux
sessions l'ont partagée, et la seconde a retrouvé ce que la première avait écrit.

**1. Un binaire qui connaît `daemon` et `--daemon`.** Un binaire bâti avant le
11 octobre après-midi ne les connaît pas (`option inconnue : --daemon`) : il faut le
rebâtir. Le copier **hors de tout `target`** (un target s'efface) :

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

**3. L'hôte**, avec tout l'environnement — c'est lui qui ouvre la base et les
modèles. Dans un terminal qui reste ouvert, ou sous votre gestionnaire de services :

```sh
LD_LIBRARY_PATH=<dépôt>/build/lecteurs-csv/src \
RAG3DB_ROOT=<dépôt> \
RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7980 \
~/.local/bin/rag3weaver-backend daemon            # écoute 127.0.0.1:7970 ; --address pour une autre
curl -s http://127.0.0.1:7970/sante               # {"pid":…,"service":"rag3weaver-daemon"}
```

L'hôte n'écoute que sur la boucle locale et refuse une autre adresse.
`RAG3WEAVER_EMBED_SERVICE` prend des adresses **complètes** ; la mémoire longue
déclare bge-m3, servi sur 7980.

**4. Chaque session**, une fois, en configuration **utilisateur** (pas un
`.mcp.json` de dépôt partagé, voir §2) :

```sh
claude mcp add memoire-longue \
  --env LD_LIBRARY_PATH=<dépôt>/build/lecteurs-csv/src \
  -- ~/.local/bin/rag3weaver-backend mcp \
     --manifest ~/.config/rag3weaver/memoire-longue/backends/memory/backend.json \
     --daemon 127.0.0.1:7970 --keys dev
```

Chemins en absolu dans la vraie commande. Toute session ouverte ensuite a les
outils de la mémoire ; `/mcp` les montre, et `curl -s http://127.0.0.1:7970/etat`
montre une session par Claude Code ouvert, et un seul backend.

**5. Arrêter l'hôte proprement** : `curl -s -X POST http://127.0.0.1:7970/arret`.
L'écoute se ferme, puis chaque base. Jamais `kill -9` : une base rag3db tuée en
pleine écriture garde un WAL à rejouer.

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
- **`option inconnue : --daemon`** : le binaire date d'avant la forme C,
  rebâtissez-le (§6, étape 1) ;
- **« pas de session auprès de l'hôte … ne répond pas »** : l'hôte n'est pas lancé,
  ou pas à cette adresse (§6, étape 3) ;
- **le serveur s'arrête avec le code 75** (sans `--daemon`) : la base demande une réouverture, et
  il vous l'a dit par un `notifications/message` avant de partir. Relancez-le.
