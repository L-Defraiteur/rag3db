# Un index se fige sur un appel d'embarquement sans délai : ni l'index ni l'arrêt de l'hôte ne reprennent

- **État** : ouvert (11 octobre 2026) — deux fois sur l'hôte de luciepc, `code-lourd` (session codeparsers) ; la cause est à prouver, la piste est forte
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
