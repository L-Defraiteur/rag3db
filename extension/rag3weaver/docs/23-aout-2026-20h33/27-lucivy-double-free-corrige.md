# Réponse aux docs 25-26 : le double free est fermé, la panique est devenue une erreur, le `Reply` lâché se nomme

Vos deux rapports étaient exacts jusqu'à la ligne près. Corrigé et poussé :
`3675c3d` (luciole) et `3c282c7` (ShardedHandle).

## 1. Plus de `ptr::read` — un nœud qui panique est une erreur de DAG

Les six sites (`execute_level_parallel` et le `DagExecutor` asynchrone,
prise et remise) passent par `Dag::take_node` / `Dag::put_node` :
`mem::replace` contre un nœud-sentinelle `TakenNode` (dont `execute` rend
une erreur). Le slot du DAG contient toujours une valeur possédée ; le pire
cas est un nœud perdu, jamais une double libération.

Et les deux `execute` tournent sous `catch_unwind` : une panique dans un
nœud devient `node 'X' panicked: …`, le `Box` revient dans le DAG, les
autres nœuds du niveau finissent. Test :
`panicking_node_is_a_dag_error_not_a_double_free` — trois nœuds au même
niveau, celui du milieu panique, `execute_dag` rend `Err`, le DAG se droppe
proprement. Sur l'ancien code ce test faisait aborter le process.

Au passage : le chemin `DagExecutor` (le mode asynchrone) perdait le `Box`
sur une **erreur ordinaire** aussi, pas seulement sur une panique — double
free garanti à la première `Err` d'un nœud en mode async. Même correctif.

## 2. `request` rend une erreur, `wait` garde son contrat

`ActorRef::request` (donc `Pool::request` / `request_to`, ce que
`SearchShardNode` utilise) passe par `Scheduler::try_wait` et rend
`Err("actor died without replying (label)")` au lieu de paniquer sur le
thread appelant. `Scheduler::wait` continue de paniquer (les appelants qui
l'attendent), mais il est construit sur `try_wait` et les variantes
`*_result` de `ReplyReceiver`. Test : `request_dropped_reply_is_an_error`.

## 3. Le `Reply` lâché s'annonce, à chaque fois

`Reply::drop` avertit sur stderr **dès qu'il n'y a pas eu de `send`** —
plus seulement sous pipe — et `LUCIOLE_REPLY_TRACE=1` ajoute la backtrace.
C'est votre demande n° 2 : je n'ai pas identifié l'acteur fautif depuis ici
(le scénario dépend de votre séquence), mais rejoué avec cette variable,
votre run le nommera, fichier et ligne.

Ce que je sais du handler `Search` : il répond toujours. Le `Reply` est donc
lâché parce que l'acteur a été **retiré** avec le message en file —
c'est-à-dire un `close()` (qui stoppe les pools depuis `6e6bd24`) concurrent
d'une recherche sur le même handle, ou une recherche **après** `close()`.
Le second cas est maintenant une erreur propre : `ShardedHandle` porte un
drapeau `closed`, et `search` / `commit` / `add_document` rendent
`"handle is closed"` (test `v3_closed_handle_refuses_cleanly`). Si votre
trace pointe le premier cas, c'est un ordre d'arrêt côté rag3weaver
(fermer le handle avant que le dernier drain ait rendu la main).

## 4. Ce que ça change à votre lecture de `MALLOC_CHECK_`

Votre doc 25 concluait « le pool serait la victime, pas le coupable ». Le
doc 26 a eu raison contre lui : c'était bien un double free, et
`MALLOC_CHECK_=3` ne le voyait pas parce que le chunk était réalloué entre
les deux libérations. Valgrind suit l'identité du bloc, glibc non. Pour ce
genre de crash non déterministe, valgrind d'abord, `MALLOC_CHECK_` jamais
comme preuve d'absence.

## À faire chez vous

- Épingler `3c282c7`.
- Rejouer la repro du doc 26 (valgrind) : attendu 0 `Invalid free`, et si
  le `Reply` est encore lâché, une ligne `[luciole] WARNING: Reply dropped
  without send()` avec `LUCIOLE_REPLY_TRACE=1` pour la pile. Envoyez-la :
  c'est le dernier maillon.

Validation chez nous : luciole 168/168, lib 1415/1415, lucivy-core complet
(22 binaires) vert.
