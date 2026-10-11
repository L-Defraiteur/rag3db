# Un index se fige sur un appel d'embarquement sans délai : ni l'index ni l'arrêt de l'hôte ne reprennent

- **État** : ouvert (11 octobre 2026) — quatre fois sur l'hôte de luciepc, `code-lourd` (session codeparsers), autour de 18 à 20 % des vecteurs ; la montre hors de cause ; une attente interne à l'hôte, pas réseau — la pile reste à prendre (voir la dernière section)
- **Gravité** : blocage (l'index ne finit jamais, l'hôte ne s'arrête plus proprement ; la première fois, la mémoire est montée jusqu'à 30 Go)
- **Atteignable en service** : oui, tout index par un service d'embarquement (`RAG3WEAVER_EMBED_SERVICE`)
- **Touche rag3weaver** : oui (`src/daemon/mod.rs`, `src/daemon/embeddings.rs`, l'indexation)

## Ce que c'est

L'index de `code-lourd` (`~/git_workspaces/rag3db-lourd`, ~136 000 morceaux, granite-278m sur le
service 7878 de luciepc, hôte au binaire a3c1ca98f) se fige **deux fois au même endroit** des
vecteurs :

| essai | lancé | figé | à | mémoire de l'hôte |
|---|---|---|---|---|
| 1 | 18 h 45 | dès 18 h 51 | 25 208 sur 136 635 (18 %) | +0,5 Go/min, jusqu'à 30,7 Go (un checkout de l'arbre et un rebâti de la lib sont venus dessous) |
| 2 | 19 h 50 | dès ~19 h 52 | 24 747 sur 136 996 (18 %) | stable, 10,3 Go (ni checkout ni rebâti) |

Pendant le figement, le service 7878 répond à `/sante` en 0,4 ms et reste au repos (1,7 % de CPU),
avec une connexion de l'hôte établie : l'hôte attend. `systemctl --user restart` ne l'arrête pas
(premier essai : plus de 30 minutes en « deactivating », jusqu'au `kill -9` de Lucie).

`code-rag3db` sur ce poste-ci (135 592 morceaux, service 7979, granite-278m) a fini en 12 min 27 s
à 15 h 34. Ce qui diffère : l'arbre (`rag3db-lourd` à b4655da39 contre l'arbre principal de 15 h 22)
et le service (7878 de luciepc contre 7979, même exécutable `rag3weaver-embeddings`, même modèle).

## La piste

**Aucun délai côté client** : `daemon::agent()` (`src/daemon/mod.rs:348`) construit l'agent ureq
sans délai (`config_builder().http_status_as_error(false)` seul), et `DaemonEmbedder::embed`
(`src/daemon/embeddings.rs:622`) s'en sert pour `POST /embed`. Une requête qui ne revient pas —
un morceau qui fait tomber le tokenizer, un texte très long, un service qui perd la requête —
tient le fil d'indexation sans fin, et l'arrêt (`stop_indexings_of`, au prochain paquet) ne peut
pas le couper.

## Ce qui le prouverait, et ce qui manque pour le prouver

- **Le morceau en cours au figement** : ni le journal de l'index ni celui de l'hôte ne le disent —
  un manque (« le morceau en cours » dans `index_state`).
- Les piles du fil : refusées (`/proc/<pid>/task/*/stack`, `gdb` sans droit d'attache).
- Un `curl` du même texte sur 7878, une fois le morceau connu.

## Ce qu'il faut pour le fermer

Un délai sur chaque appel d'embarquement (lecture et connexion) ; au-delà, un refus nommé du
morceau (« embarquement sans réponse en N s : <fichier>:<rang> »), le morceau mis en reprise, et
l'index qui continue. Le morceau en cours écrit dans l'état de l'index. Témoin : un faux service
qui ne répond jamais à un morceau donné ; l'index finit, le morceau est nommé, et l'hôte s'arrête
en quelques secondes.

## Un fait qui affaiblit la piste (11 octobre, 20 h 30)

Deux arrêts suivants de l'hôte de luciepc, chacun pendant un index figé, ont été **propres en
moins de 30 s** (« 1 indexation(s) arrêtée(s) ; relancer index reprend là ») : celui de G à
20 h 11 (index de `code-lourd` figé à 18 %), et celui de 20 h 27 (index de `code-principal` figé à
99 %, 119 506 sur 120 317, le total bougeant sous la montre). Le fil d'indexation lisait donc son
drapeau d'arrêt : il n'était pas bloqué dans un appel d'embarquement sans fin. Restent ouvertes :
une boucle de rattrapage qui ne converge pas (la montre réinvalide des morceaux pendant que l'index
les rattrape ; vu à 99 % sur `code-principal`, où des fichiers d'essai étaient déposés puis retirés),
et, pour le figement à 18 % sans changement de l'arbre, une cause à trouver avec l'état de l'index
(le morceau en cours, toujours absent). Le délai sur les appels d'embarquement reste à poser.

## Quatre occurrences, la montre hors de cause (11 octobre, 22 h 30)

Deux essais de plus sur `code-lourd`, le second **sans la montre** (`watches` et `reactions`
retirés des manifestes de `code-lourd` et de `code-principal`, l'hôte redémarré à 21 h 53) :

| essai | lancé | figé | à | montre |
|---|---|---|---|---|
| 3 | soir, binaire neuf | — | 27 812 sur 138 579 (20 %) | oui |
| 4 | 22 h 02 | dès ~22 h 04 | 28 369 sur 138 579 (20 %) | **non** |

L'hypothèse de la montre (ses réconciliations et ses réactions en concurrence avec l'index)
tombe : sans elle, l'index se fige au même point.

Le quatrième essai précise le figement : il **n'écrit aucun vecteur neuf**. Les 28 369 sont ceux
des essais précédents, puisque l'index reprend là où il s'était arrêté (« 110 146 chunks à
embarquer », soit 138 579 − 28 433) ; le compte est atteint dès la reprise et ne bouge plus. Chaque
essai a ajouté quelques centaines de vecteurs puis s'est arrêté autour de 18 à 20 %.

Pendant le figement (lu sans s'attacher, `ptrace_scope` vaut 1) :

- tous les fils de l'hôte dorment sur un futex ; un seul (`demon-7`) prend encore un peu de CPU,
  ~4 % ;
- le service 7878 est au repos ; les connexions de l'hôte vers lui sont établies, files vides ;
- aucune panique ni erreur dans `daemon.log` ;
- `/sante` et `/etat` répondent ; un appel MCP sur `code-lourd` attend derrière l'index.

Tous les fils sur un futex et le service au repos : **une attente interne à l'hôte** (verrou,
variable de condition ou canal), pas une attente réseau. Le petit CPU de `demon-7` laisse ouverte
une boucle d'attente qui ne converge pas, cohérente avec les arrêts propres en moins de 30 s
relevés plus haut. La piste du délai manquant sur les appels d'embarquement n'explique pas ce
figement ; le délai reste à poser pour lui-même.

L'hôte est gardé figé pour Lucie (pid 132823). La pile (`sudo gdb -p <pid> -batch -ex 'thread
apply all bt'`) dira où l'attente se tient ; le morceau en cours manque toujours à l'état de
l'index.
