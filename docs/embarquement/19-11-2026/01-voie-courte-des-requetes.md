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
