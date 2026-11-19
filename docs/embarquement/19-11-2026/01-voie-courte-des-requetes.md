# Embarquement : une voie courte pour les requêtes (conception)

Demande de c7, le 19 novembre, d'après le relevé de tty au ticket
`arbre-principal/tickets/2026-10-11-un-appel-d-outil-attend-derriere-un-index-sans-le-dire.md` (« les appels d'outil
passent avant les lots d'index […] au service d'embarquement »). **Rien n'est codé avant le mot de c7.**

## Ce qui se passe aujourd'hui

Dans le service (`EmbedDaemon::par_lots`, `src/daemon/embeddings.rs`), chaque requête `/embed` est découpée en lots ;
**chaque lot prend le verrou `passe`**, calcule, puis le rend. Une requête de recherche (un texte) attend donc derrière
le lot d'index en cours, et :
- **le verrou ne sert pas les demandes dans l'ordre d'arrivée** (`std::sync::Mutex`) : le fil d'un index, qui
  reprend le verrou dès qu'il l'a rendu, peut passer devant la requête plusieurs fois de suite ;
- **sur une carte partagée** (`gate`, rafales et trous), le trou s'attend *sous* le verrou, avant la rafale : la
  requête attend aussi les pauses de l'index ;
- côté client, l'attente est bornée par le délai de l'appel (60 s par défaut, `RAG3WEAVER_EMBED_TIMEOUT_SECS`), puis
  la recherche se replie sans le dense.

## La proposition

**Une priorité pour les requêtes courtes, au verrou de passe du service.**
1. Une requête de **4 textes au plus** est « courte » ; un seuil fixe, dit dans l'identité (`short_request_texts`).
   Une requête de recherche en embarque un, le surlignage quelques-uns.
2. **Un compteur de requêtes courtes en attente** (`AtomicUsize`) et une `Condvar` à côté du verrou :
   - une requête courte s'inscrit, prend le verrou, calcule, se désinscrit, et réveille les autres ;
   - **un lot long, avant de prendre le verrou, attend qu'aucune requête courte ne soit inscrite**.
   Une requête courte attend donc au plus **le lot long en cours**, jamais les suivants.
3. **Une requête courte ne prend pas le trou de la rafale** : elle ne passe pas par `gate.wait()`. Elle compte
   quand même dans la durée de rafale qui suit (`gate.done`), pour que l'écran garde sa part.
4. **Rien ne change pour les lots longs** entre eux : même ordre, même découpe.

Ce qui reste borné : une requête courte attend au plus un lot long : au 7878, 128 morceaux ; au 7879, 32. La durée
d'un tel lot n'est pas mesurée ici ; la mesure ci-dessous la donnera.

**Écartées** :
- découper le lot en cours pour laisser passer : le calcul d'un lot sur la carte ne s'interrompt pas ;
- un second fil de calcul pour les requêtes : deux calculs à la fois sur la même carte doublent la mémoire de
  travail, sur une carte déjà partagée.

## La mesure, avant et après

Un script client, `scripts/embed_latency.py` : un fil envoie sans arrêt des lots de 128 textes de 1 000 à 1 400
caractères, comme un index ; un autre envoie une requête d'un texte toutes les 500 ms et note sa latence. Il rend la
médiane, le 95e centile et le maximum des requêtes, pendant 60 s. Il se joue :
- **jamais sur le 7878 ou le 7879** : sur un service d'essai lancé à part, sur `igpu:0`, à un port libre, avec le
  binaire de master (avant) puis celui de la branche (après), le même modèle (granite-278m) ;
- le résultat (médiane, p95, max, avant et après) va dans cette page, avec les binaires.

## Le témoin, rouge d'abord

`a_short_request_passes_before_the_next_long_batch` (`src/daemon/embeddings.rs`, tests). Un embarqueur factice met
200 ms par appel. Une requête longue de 20 lots court dans un fil. Une requête d'un texte part pendant le deuxième
lot. Elle doit répondre en moins de 2 lots (400 ms) ; aujourd'hui, elle peut attendre jusqu'à la fin de la requête
longue (4 s). Le témoin juge sur 5 essais, et le pire compte.

## Taille

Une demi-journée : le code (1 h), le témoin, puis la mesure sur `igpu:0` (1 h 30).

## Fait (19 novembre 2026, au mot de c7)

- **Le code** (branche `short-lane`, déposée au pousseur sous `emb-short-lane`) : `SHORT_REQUEST_TEXTS` (4),
  `take_pass(short)` au verrou de passe (un compteur et une `Condvar`), pas de trou de rafale pour une requête
  courte ; l'identité dit `short_request_texts`.
- **Le témoin** `a_short_request_passes_before_the_next_long_batch` : rouge à **710 ms**, puis vert sous 400 ms. Il joue
  la voie des rafales : sans carte partagée, le fil d'index souffle hors du verrou entre deux lots, et le même témoin
  passait déjà.

## La mesure (19 novembre, 14 h 57 → 14 h 59)

`scripts/embed_latency.py` (branche `short-lane`), sous `poste mesure` : un service d'essai granite-278m sur `gpu:0`
(card2), port 7990, régime `plein`, avec l'accord de G. **Pas sur `igpu:0`** : granite y panique à sa chauffe, même avec
le binaire de master (ticket `embarquement/tickets/2026-11-19-granite-278m-panique-a-sa-chauffe-sur-l-igpu.md`). Un fil
envoie des lots de 128 textes de 1 000 à 1 400 caractères ; un autre envoie une requête d'un texte toutes les 500 ms,
pendant 60 s.

| Binaire | Médiane | p95 | Max | Lots d'index servis |
|---|---|---|---|---|
| Avant, master `91bd3ebd959ebedd` | 36,9 ms | 116,3 ms | 204,7 ms | 108 |
| Après, `short-lane` `d52b3d7aa761112d` | 23,8 ms | 38,4 ms | 120,2 ms | 109 |

- **Le p95 est divisé par 3, le max presque par 2**, sans perte de débit pour l'index (108 lots contre 109).
- En régime `plein`, sans rafales, l'attente d'avant n'était pas de minutes : le fil d'index rendait déjà le verrou
  entre deux lots. Le gain le plus grand est sur une carte partagée (le témoin : 710 ms, puis sous 400 ms).
- **card2 et les services de G sont restés intacts** : 13 829 Mio libres avant et après, 7878 et 7879 actifs,
  `NRestarts=0`. Les journaux sont à luciepc, dans `~/.cache/rag3weaver-build/short-lane-mesure/`.
