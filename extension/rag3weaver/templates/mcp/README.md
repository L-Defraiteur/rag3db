# Brancher rag3weaver sur Claude Code, par MCP

`rag3weaver-backend mcp` expose **les outils que le manifeste déclare**, et rien
d'autre. Un serveur = un manifeste. Les deux cas d'usage sont donc la même
mécanique avec deux manifestes différents :

| ce qu'on veut | le gabarit |
|---|---|
| chercher dans un dépôt de code | `templates/backends/code` |
| une mémoire longue, d'une session à l'autre | `templates/backends/memory` |

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

Les autres chemins du manifeste (les `graph` des outils) sont relatifs au
manifeste : si vous copiez le dossier entier, ils restent justes.

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

- `LD_LIBRARY_PATH` → `<bâti>/src`, pour que `librag3db.so` se charge. Sans
  elle, le serveur meurt au démarrage sans rien dire d'utile ;
- `RAG3DB_ROOT` → l'arbre où le moteur est bâti. **Jamais dérivé du chemin du
  code** : depuis un worktree, le code vit ici et l'extension est ailleurs
  (piège 6 du journal des chantiers) ;
- `RAG3WEAVER_EMBED_SERVICE` → les adresses **complètes** du service
  d'embarquement (`127.0.0.1:7878,127.0.0.1:7879`), jamais les ports seuls. Le
  gabarit de code déclare des vecteurs, donc il en exige un.

## 4. La mémoire partagée entre projets : `--demon`

Sans `--demon`, le serveur **possède** sa base — bon pour un dépôt de code, une
session. Mais Claude Code lance **un processus MCP par session**, et une base
rag3db ne s'ouvre que par un seul processus : deux sessions sur la même mémoire
seraient deux écrivains, refusés par le moteur.

Avec `--demon <adresse>`, le serveur ne possède plus rien : ses outils restent
locaux et sa **connexion** passe par `rag3daemon`, le processus unique qui tient
la base. N sessions, un écrivain. C'est la forme à utiliser dès que deux
sessions partagent une mémoire, et c'est la forme produit.

```json
"args": ["mcp", "--manifest", "…/memory/backend.json",
         "--demon", "127.0.0.1:7979", "--keys", "dev"]
```

Le démon se lance tout seul s'il ne répond pas déjà (`DaemonConnection::assurer`).

## 5. `--keys` : déclaré n'est pas exposé

Chaque déclaration portera une `exposure` (une expression sur des clés), et le
serveur n'exposera que ce que ses clés rendent vrai — **rien par défaut**. La
lecture des clés est en place ; le filtrage viendra avec elle. Le serveur
annonce au démarrage ce qu'il expose et sous quelles clés :

```
[mcp] code-poste — 12 outils, clés présentées : dev
```

Le jour où des déclarations seront écartées, cette ligne est ce qui évitera de
chercher pourquoi la liste est vide.

## Ce que le serveur ne fait pas

Ni `resources`, ni `prompts`, ni `sampling`, ni `roots` — **absents et déclarés
absents** dans ses `capabilities`, plutôt que silencieux. Pas de transport HTTP :
stdio seulement.

## Si ça ne marche pas

- **le serveur meurt au lancement** : presque toujours `LD_LIBRARY_PATH`. Lancez
  la même ligne à la main, l'erreur de chargement s'affiche ;
- **la liste d'outils est vide** : lisez la ligne `[mcp] …` sur stderr, elle dit
  combien d'outils et sous quelles clés ;
- **un outil refuse « unknown entity: File »** : la base n'est pas indexée.
  Appelez l'outil `index` d'abord. (Ce refus ne le dit pas encore ; c'est un
  défaut connu, relevé le 10 octobre 2026.)
- **le serveur s'arrête avec le code 75** : la base demande une réouverture, et
  il vous l'a dit par un `notifications/message` avant de partir. Relancez-le.
