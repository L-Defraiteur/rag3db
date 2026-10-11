# `code-tree` : l'arbre du cœur indexé, et qui suit ses éditions

*11 octobre 2026, soir, session codeparsers. Page courte avant le code, à la demande de
l'orchestration, après la priorité dite par Lucie : « ce qu'il faut, c'est que le cœur C++ ait son
worktree, peu importe lequel, indexé, et qui se met à jour quand il fait des edits ».
Relecture : l'arbre principal.*

## La commande

Elle vit dans le binaire `rag3weaver-backend`, à côté de `mcp`, `serve` et `daemon` (aiguillage
de `run()`, `src/bin/rag3weaver-backend.rs:506`) :

```
rag3weaver-backend code-tree add <arbre> [--from code-lourd] [--as code-coeur]
rag3weaver-backend code-tree remove <arbre | code-coeur>
```

- **`add`** :
  - copie le gabarit `code` sous `~/.config/rag3weaver/code-<nom>/` (ou `code-coeur` avec
    `--as`), pose la racine, la base, l'extension vecteur et le service d'embarquement du poste ;
    lecture seule, sans `edit_file` ni `run_command` (la recette de la page 04, devenue commande) ;
  - fait naître la base d'un **instantané reflink** de celle de `--from` (base, `.fts/`,
    `.checkpoints/`, `.extensions`), fermée ou cohérente ;
  - l'ouvre par l'hôte (`POST /open`, qui existe) ;
  - lance la **réconciliation** (plus bas).
- **`remove`** : ferme le backend par une route de l'hôte (à poser : `POST /close`, voir plus
  bas), puis met la base et le manifeste de côté.
- **`--as code-coeur`** : un manifeste stable, `code-coeur`, dont la racine est « l'arbre
  courant du cœur ». Son serveur MCP n'est jamais recréé : `add --as code-coeur <autre arbre>`
  ferme le backend, repointe la racine, rouvre et réconcilie.

## Ce que fait la réconciliation, et ce qui manque aujourd'hui

**Aujourd'hui, rien ne réconcilie un arbre avec sa base** (relevé du 11 octobre) :
- **la montre** (`src/watch.rs:120-136`, W1) photographie l'arbre en mémoire à l'ouverture,
  sans rien comparer à la base. Une édition faite backend fermé n'est jamais vue ;
- **`sync_source`** (`src/code_sync.rs:481`, ce que joue l'outil `index`) relit et ré-analyse
  **tous** les fichiers. Les lignes identiques ne sont ni réécrites ni ré-embarquées
  (`Catalog::split_unchanged`), mais l'analyse de codeparsers se refait en entier, et un fichier
  disparu ne part qu'en fin de passe, par la mise de côté.

**La réconciliation voulue**, à l'ouverture d'un backend (et donc après `add`) :
1. lister l'arbre et hacher chaque fichier (blake3, comme la montre ; ~60 Mo pour rag3db, de
   l'ordre de la seconde) ;
2. lire en base `File.content_hash` (`src/code.rs:212`, déjà rempli) pour chaque chemin ;
3. réindexer **les seuls fichiers dont le hachage diffère** (`code_sync::reingest_file`, déjà
   celui de la montre), ingérer les neufs, retirer les disparus (`code_sync::remove_file`) ;
4. donner à la montre cette photographie comme point de départ.

## L'écart avec « réindexer tout »

| | premier index complet | `add` par reflink + réconciliation |
|---|---|---|
| analyse codeparsers | 7 300 fichiers | les fichiers où la branche diffère de `--from` |
| vecteurs | 135 000 morceaux (~7 min sur luciepc) | ceux des scopes changés |
| ordre de grandeur | 7 à 8 min, 12 Go résidents au pic | quelques secondes de copie, puis le delta |

## Ce qui bloque, à trancher avec l'arbre principal

**Les identités sont des chemins absolus.** Avec une source `worktree:`, `File.path` et
`Scope.file_path` sont absolus (`src/code.rs:902-910`), et l'identité d'un `File` se calcule sur
`(source, path)` (`code.rs:223`), celle d'un Scope sur `{source}#{name}#…` (`code.rs:1010`). Une
base née de `code-lourd` (`/home/lucied/git_workspaces/rag3db-lourd/…`) pour un autre worktree
(`…/rag3db-pk/…`) verrait **toutes** ses identités changer : la réconciliation réingérerait tout
et retirerait tout l'ancien. Le delta ne vient qu'avec **des identités relatives à la racine**.
Trois formes :
- **(a)** l'identité sur `repo_path` (la coordonnée relative au dépôt, déjà portée par les
  Scope, `code.rs:206`), la racine dans la méta de l'arbre indexé — une migration des identités,
  bases existantes comprises ;
- **(b)** au `add`, réécrire dans la copie les chemins et les clés de l'ancienne racine vers la
  nouvelle (une passe sur toute la base, sans réanalyse) ;
- **(c)** ne cloner que vers un arbre de même racine (le même worktree qui change de branche) :
  sans valeur pour le cœur, qui change de worktree à chaque lot.

Ma recommandation : **(a)**, la forme générale (un arbre déplacé ou cloné garde sa base). La
réconciliation (1-4) vient d'abord, elle sert aussi l'arbre qui change de branche sur place et
le backend rouvert après un arrêt.

**Les routes de l'hôte** : `POST /open` existe ; **pas de fermeture d'un seul backend** (seulement
`shutdown()` de tout l'hôte, `src/daemon_host.rs:285`) — c'est le ticket « l'hôte ne relit pas un
manifeste modifié, et aucune route ne ferme un backend ». `remove` et le repointage de
`code-coeur` l'attendent.

## Les témoins prévus

- `add` sur un worktree de la bascule : `usages` voit une fonction de la branche en moins d'une
  minute ;
- `remove` : backend fermé, base disparue ;
- `add` puis `git checkout` d'une autre branche : la montre réindexe le delta ;
- un backend fermé, un fichier édité, le backend rouvert : la réconciliation le voit.

## L'ordre

1. La réconciliation à l'ouverture (codeparsers, `code_sync`, `watch`), avec son témoin du
   backend rouvert.
2. `POST /close` (l'arbre principal).
3. Les identités relatives (a), avec la migration des bases existantes.
4. `code-tree add|remove`, `--from`, `--as code-coeur`.

D'ici là, la recette d'une ligne de la page 04 et un `index` complet (7 à 8 min) tiennent lieu
de `add`.
