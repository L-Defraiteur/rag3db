# Un client MCP, plusieurs hôtes

*rag3weaver archi, 21 novembre 2026, à la demande de c7 après la question de Lucie à G (12 h 45) : « faut que les 2
hôtes soient accessible par le même mcp je pense non? ». Une page de conception courte, avant tout code. Ce qui est dit
« aujourd'hui » a été lu dans le code de master par le MCP, sans être joué.*

## Aujourd'hui

Un client `rag3weaver-backend mcp` parle à **un seul** hôte. Par défaut, c'est celui du poste (son `daemon.json`) ;
sinon, c'est l'hôte désigné par `--daemon <adresse>` avec `--daemon-json-command`, la source du jeton
(`DaemonSource::Command`, ici `ssh luciepc cat …/daemon.json`). Claude Code voit donc deux serveurs MCP distincts,
`code-rag3db` pour la tablette et `code-rag3db-luciepc` pour luciepc par le tunnel 7971 de G, avec les mêmes outils
en double.

## 1. Le routage

Le client connaît une **liste d'hôtes déclarée**, dans un fichier de l'utilisatrice
(`~/.config/rag3weaver/hosts.json`) :

```json
{"hosts": [{"name": "tablette", "local": true},
           {"name": "luciepc", "address": "127.0.0.1:7971", "token_from": "…"}]}
```

Un appel va à l'hôte de **sa cible**, résolue dans cet ordre :

1. **La vue nommée par son poste** : `view: "luciepc:archi/21-11-2026-x"`. C'est la forme d'adresse de l'invariant
   de Lucie (`<poste>:<nom>`), qui ne laisse aucune ambiguïté.
2. **La vue ou le worktree sans poste** (`view: {worktree: "/home/…"}`) : l'hôte qui suit ce chemin. Un chemin vit sur
   une machine : le client demande à chaque hôte joignable ses vues (`views`), et un seul les a.
3. **Le dépôt** : l'hôte qui l'indexe. Si les deux l'indexent (le cas de rag3db : master à luciepc, l'arbre de Lucie
   ici), c'est l'hôte par défaut, et la réponse le dit.

**Un outil sans cible** va à l'hôte par défaut : celui du poste, comme aujourd'hui. La réponse dit toujours quel hôte
a répondu (`host: "tablette"`). Rien ne change pour un appel d'aujourd'hui.

**Ce que ça coûte** : petit. Une liste d'hôtes, une résolution de la cible, un champ `host` dans chaque réponse. Les
outils ne changent pas, ni les hôtes.

## 2. La recherche des deux côtés

Chercher sur les deux hôtes, puis fusionner. C'est utile quand un même dépôt est indexé deux fois, à deux états
différents (master à luciepc, l'arbre de Lucie ici).

**Ce que ça coûte** : nettement plus.
- **Les scores ne se comparent pas** d'un index à l'autre : le BM25 dépend des fréquences de chaque base, et les
  vecteurs ne se comparent que sous le même embarqueur et le même modèle. Il faut fusionner par **rang**
  (Reciprocal Rank Fusion), jamais par score brut.
- **Le dédoublonnage** : un même morceau vu des deux côtés se reconnaît par (dépôt, chemin, empreinte du contenu). Une
  différence de contenu n'est pas un doublon : c'est une **divergence** entre les deux états, à montrer comme telle (« ici
  tel contenu, là-bas tel autre »), pas à masquer.
- **La latence** est celle de l'hôte le plus lent, et un hôte injoignable rend une réponse partielle, dite comme telle.
- Les témoins doivent couvrir deux faux hôtes, un doublon, une divergence et un hôte tombé.

**La recommandation de c7 tient** : router d'abord par la cible (1), et garder la recherche des deux côtés pour plus
tard, sur un besoin concret. Le jour venu, la recherche des deux côtés est une option de l'appel (`hosts: "all"`), pas
le défaut.

## 3. La santé

- Chaque hôte est sondé à part, avec son délai. Un hôte injoignable ne casse pas l'autre : ses cibles rendent une
  erreur qui le nomme (« luciepc injoignable : … ») ; le reste répond.
- La liste des outils est celle de l'hôte par défaut : les outils sont les mêmes d'un hôte à l'autre, puisqu'ils
  viennent du même gabarit.
- Une réponse dit toujours son hôte.

## 4. Ce que voit `daemon_status`

Un bloc par hôte de la liste : son nom, son adresse, son état (en marche, injoignable et pourquoi), son bâti et sa lib,
ses dépôts et ses vues, et le moment du dernier contact. L'hôte par défaut est marqué.

## 5. Ce qui ne bouge pas

- **Un hôte reste la seule porte d'écriture de ses bases.** Une écriture (index, édition, tâche) va à l'hôte qui
  possède la cible, et à lui seul. Jamais d'écriture d'un hôte dans la base d'un autre.
- **Le relais entre postes** se fait d'hôte à hôte par HTTP avec un jeton, jamais par ssh : c'est l'invariant de Lucie.
  La lecture du jeton distant par `ssh … cat daemon.json` (`DaemonSource::Command`) est un pis-aller d'aujourd'hui.
  La cible, c'est un jeton d'accès entre hôtes, donné une fois. C'est aussi ce qui permettra au terminal de lire
  son `user_token` distant sans ssh.
- **Le titre d'humain ne voyage pas.** Le `user_token` d'un poste ne vaut que pour l'hôte de ce poste ; un humain sur
  la tablette n'est pas humain pour luciepc sans son propre titre.

## Lots, si la page est retenue

1. Le routage par la cible, avec le champ `host` dans chaque réponse et `daemon_status` par hôte. Petit.
2. Le jeton entre hôtes, qui remplace la lecture du `daemon.json` distant par ssh.
3. Plus tard, et seulement sur un besoin : la recherche des deux côtés, fusionnée par rang.

## À trancher par Lucie

- Une seule entrée MCP qui voit les deux hôtes (cette page) ? Ou deux entrées, comme aujourd'hui, avec seulement le
  champ `host` et `daemon_status` améliorés ?
- L'hôte par défaut d'un client : toujours celui du poste, ou déclaré dans `hosts.json` ?
