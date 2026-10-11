# L'ordre A, joué : la mémoire longue branchée sur Claude Code

*11 octobre 2026, 3 h 30. Session « mémoire longue ». Pour Lucie, à lire au réveil.*

L'ordre A de la page `05-la-memoire-par-mcp.md` était : brancher le gabarit
`memory` par MCP, tel quel, une passe, pas de signature nouvelle — pour savoir
si Claude range spontanément quand on lui donne l'outil. C'est fait, la base
existe, elle se souvient d'une session à l'autre, et la réaction du gabarit est
montée. Ce qui suit est la recette, puis ce que jouer a appris.

## La recette, en trois lignes

Le manifeste est déjà posé à `~/.config/rag3weaver/memoire-longue/` (une copie
de tout `templates/`, pas du seul dossier `memory` : le gabarit range des
graphes hors de son dossier). Son `database` et son `vector_extension` sont en
absolu, c'est tout ce qui a changé.

```sh
# 1. le service d'embarquement — adresse COMPLÈTE, et c'est bge-m3 que le gabarit déclare
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7980

# 2. le serveur, à la main, pour voir ce qu'il annonce
LD_LIBRARY_PATH=~/git_workspaces/rag3db/build/lecteurs-csv/src \
RAG3DB_ROOT=~/git_workspaces/rag3db \
~/.local/bin/rag3weaver-backend mcp \
  --manifest ~/.config/rag3weaver/memoire-longue/backends/memory/backend.json --keys dev

# 3. et pour de bon, en configuration UTILISATEUR (pas un .mcp.json de dépôt partagé)
claude mcp add memoire-longue \
  --env LD_LIBRARY_PATH=/home/lucied/git_workspaces/rag3db/build/lecteurs-csv/src \
  --env RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db \
  --env RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7980 \
  -- /home/lucied/.local/bin/rag3weaver-backend \
     mcp --manifest /home/lucied/.config/rag3weaver/memoire-longue/backends/memory/backend.json \
     --keys dev
```

**La troisième ligne n'est pas jouée.** Elle écrit dans ta configuration Claude
Code, et ça, c'est à toi de le décider — je l'ai laissée prête, pas exécutée.

Au démarrage, le serveur dit ce qu'il expose :

```
[mcp] memory — 7 outils, clés présentées : dev
[mcp] 1 réaction(s) montée(s) : review_anchored
```

La seconde ligne est née de cette passe : sans elle, rien en service ne disait
qu'un graphe tourne derrière les outils.

## Ce qui a été joué, et ce que ça rend

Deux passes, deux processus différents sur la même base — c'est la seconde qui
prouve quelque chose.

| geste | résultat |
|---|---|
| `tools/list` | `add_ref`, `create_ref_type`, `get_memory`, `put_memory`, `put_subject`, `recall`, `recall_subjects` |
| `put_subject` | un sujet écrit, uuid recomposable depuis son nom |
| `put_memory` | une mémoire écrite, `state=current` posé par le cycle de vie |
| `recall` | la mémoire retrouvée, `vector+bm25`, rendue en Markdown avec ses champs |
| `recall_subjects` | le sujet retrouvé |
| `create_ref_type` | **refuse d'abord**, et dit l'appel exact avec `confirm: true` |
| `add_ref` | la référence créée et liée (`CITE`), `verifiee: false` faute de harnais |
| **seconde passe, nouveau processus** | `recall` et `get_memory` retrouvent ce que la première avait écrit |

La base : `~/.local/share/rag3weaver/memoire-longue.rag3db`, 3,4 Mo pour une
mémoire, un sujet, un genre et une référence. Elle survit à la fermeture du
tube.

## Ce qui ne marche pas, et qu'il faut savoir avant d'essayer

**1. La réaction est montée mais elle ne peut rien transitionner.** Aucun des
sept outils ne pose `ANCHORED_TO` : `add_ref` pose `CITE`, et rien n'écrit
`ANCHORED_TO` ni `REPLACES`. Donc `review_anchored` part à chaque écriture de
sujet et trouve toujours zéro ligne concernée. J'ai cru que la pièce existait déjà
(`templates/tools/link_snapshot.mmd`, un `RelationBatchNode` générique) et que
six lignes de manifeste suffisaient. **Essayé, et non** :

```
anchor(links: [{from: {claim, reach}, to: {name}}])
→ "snapshot links cannot modify relations of managed entities"
```

Ce nœud refuse si l'un des deux bouts a un cycle de vie, une révision ou des
dates serveur — `Memory` et `Subject` ont les trois. Il n'existe donc, à cette
date, **aucun nœud qui relie deux entités gérées par leur identité**. Le ticket
`docs/tickets/2026-10-11-aucun-outil-n-ancre-une-memoire-a-un-sujet.md` porte
les trois chemins possibles et leur coût ; le moins risqué est un nœud de lien
générique, donc une signature nouvelle — ce que l'ordre A excluait.

C'est le même défaut que celui que ce dépôt écrit en boucle, pris une couche
plus haut : la déclaration est vraie, le moteur est vrai, et le chemin qui les
relie est fermé par en haut.

**2. `recall` exige un `options`.** `recall(query: "…")` est refusé :
« `options` is a required property ». Il faut `recall(query: "…", options: {})`.
Un modèle qui lit le schéma le verra, mais c'est une marche pour rien sur le
verbe le plus employé du gabarit.

**3. Les champs d'une mémoire sont des énumérations fermées**, et c'est voulu :
`kind` ∈ {fact, preference, decision, pointer}, `origin` ∈ {said, inferred},
`reach` ∈ {person, project, team}. J'ai écrit `kind: "mesure"` au premier essai
et le refus m'a corrigé — ce qui est exactement ce qu'on veut, mais note que
« mesure » n'est pas une sorte de mémoire dans ce gabarit.

**4. Le service d'embarquement est un tunnel vers luciepc** (127.0.0.1:7980 →
bge-m3, 1024). Tunnel fermé, le serveur refuse de démarrer en le disant. Si tu
veux que la mémoire vive sans luciepc, il faut soit un service local, soit
changer le gabarit pour granite-278m.

**5. `LD_LIBRARY_PATH` pointe sur un dossier de bâti que les sessions
rebâtissent.** `~/git_workspaces/rag3db/build/lecteurs-csv/src` est l'arbre de
travail de la session « arbre principal », pas une installation : elle y rebâtit
`librag3db.so` et `libvector` quand le moteur bouge. Le serveur chargera donc la
lib du moment, et le moteur vérifie le bâti de l'extension face à sa lib — ta
base indexée reste valide. Mais si tu veux une mémoire qui ne dépende pas d'un
arbre de travail, il faut copier les deux bibliothèques à côté du binaire, comme
je l'ai fait pour lui.

**6. Une base, un écrivain.** Sans `--demon`, le serveur possède sa base : deux
sessions Claude Code ouvertes en même temps, et la seconde est refusée. Pour
plusieurs sessions, c'est `--demon 127.0.0.1:7979` — mais attention, 7979 est
déjà pris ici par le tunnel d'embarquement, il faudra un autre port.

## La question que l'ordre A posait, et qui t'attend

« Est-ce que Claude range spontanément ? » Je ne peux pas y répondre : je suis
la session qui a écrit l'outil, donc mon usage ne prouve rien. Il faut une
session qui ne sait rien de tout ceci, à qui l'outil est simplement disponible.

Ce que je peux dire, c'est ce qui l'aidera ou la gênera :

- `recall` rend du Markdown lisible avec les champs en clair : une session qui
  cherche trouve quelque chose d'utilisable sans post-traitement ;
- `put_memory` demande six champs dont trois énumérés : c'est assez pour que
  ranger soit un geste réfléchi, et assez peu pour qu'il ne coûte pas une
  minute ;
- `create_ref_type` qui refuse en donnant l'appel exact est le meilleur morceau
  du gabarit. Un refus qui nomme le remède se corrige du premier coup ;
- il n'y a **rien** pour relier une mémoire à un sujet, et c'est le geste qui
  rendrait la mémoire navigable. C'est ce que je ferais en premier.

## Les décisions prises à ta place, toutes renversables

1. La base vit dans `~/.local/share/rag3weaver/` et non `~/.cache/` : une
   mémoire longue n'est pas un cache, et `~/.cache` se vide.
2. Le manifeste vit dans `~/.config/rag3weaver/memoire-longue/`, comme la
   recette du gabarit `code` le prescrit.
3. `--keys dev`, comme la recette jouée du 10 octobre ; aucun gabarit ne porte
   encore d'`exposure`, donc rien n'est écarté.
4. **Le binaire est copié dans `~/.local/bin/rag3weaver-backend`**, en release
   (md5 `8220eb46a2ae`, bâti du commit de ce lot), et pas laissé dans le
   `target` : les `target` s'effacent après chaque lot, et une recette qui
   pointe vers un chemin qu'on va effacer est une recette morte. Le rebâtir se
   fait par `cargo build --release --bin rag3weaver-backend` avec les features
   de `run_e2e.sh`, puis une copie.
5. Je n'ai pas ajouté l'outil d'ancrage : l'orchestration a dit oui sur ma
   parole qu'il tenait en six lignes, et j'avais tort — le nœud existant le
   refuse. L'essai a été joué sur ta copie puis retiré ; ton manifeste a bien
   ses sept outils, pas huit. Le ticket dit ce qu'il faudrait.
