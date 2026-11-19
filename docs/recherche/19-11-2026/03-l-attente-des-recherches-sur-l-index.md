# L'attente des recherches sur l'index : texte et vecteurs (point 7 de la vision du 15 novembre)

Session recherche, 19 novembre 2026, sur le lot d'arbre `arbre-estimation-hote` (`ViewState.text_eta_s`,
`vectors_eta_s`, `Catalog::vectors_due_for_view`). Une page courte : ce qui existe, ce qui s'ajoute, ce qui reste
ouvert.

## Ce qui existe sur master

Un outil qui déclare `freshness` attend l'index de sa couche avant son graphe (`wait_for_a_lagging_index`). Sous
`search.index_wait_s` (6 s par défaut), il attend en synchrone et répond complet. Au-dessus, il répond tout de suite :
`index: {ready: false, eta_s, pending_files}` et une ligne « > » en tête. L'estimation est celle du texte seul : les
fichiers notés en retard dans la couche, par le débit observé (`Lag::estimate_of`).

Neuf outils du backend de code déclarent `freshness` : `search_code`, `read_file`, `grep_files`, `navigate_symbols`,
`outline`, `list_files`, `usages`, `impact` et `callees`.

## Ce qui s'ajoute (ce lot)

1. **Les vecteurs dus, pour une recherche qui lit le signal vector.** Les signaux se lisent dans `options.signals`, à
   défaut dans ceux de l'entité cible (`Scope`). Le mode `scan` ne lit aucun signal.
   - **bm25 seul** : le texte seul, comme aujourd'hui.
   - **vector ou hybride** : le texte et les vecteurs, chacun avec sa propre estimation (`text_eta_s`,
     `vectors_eta_s`). L'attente est la plus longue des deux. Sous le seuil, l'appel attend les deux ; au-dessus, il
     répond tout de suite, et le champ `index` dit les deux estimations.
   - **Le service d'embarquement absent** (`Embedder::is_absent`) : pas d'attente des vecteurs. Leur cause est dite
     (`vectors: "no embedding service"`), et le texte s'attend comme en bm25.
   - **Débit inconnu** (`rate_from: "unknown"`) : l'estimation des vecteurs est `null`, et l'attente est bornée par le
     seuil, comme pour le texte sans débit observé.
2. **Les outils sans vecteurs n'attendent jamais les vecteurs** : usages, outline, read_file, grep_files, list_files,
   navigate_symbols, impact et callees. Leur attente du texte reste celle d'aujourd'hui.
3. **Sans le verrou du catalogue** : `vectors_due_for_view` est une méthode du catalogue. L'attente la lit par un essai
   de verrou (`try_lock`), comme le fait déjà la ligne « vecteurs prêts » (`silent_unless_vectors_ready`). Sous un
   écrivain qui tient le catalogue, l'estimation des vecteurs est inconnue, et elle est dite.

**Témoin rouge d'abord** : une vue qui doit N morceaux, avec un débit déclaré par le banc (`RAG3WEAVER_EMBEDDING_RATES`).
- Une recherche hybride sous le seuil attend les vecteurs. La même au-dessus répond tout de suite, avec
  `vectors_eta_s`.
- En bm25 seul, la même vue n'attend rien.
- Avec l'embarqueur absent, l'appel n'attend pas et dit la cause.

## Ce qui reste ouvert (questions à c7, qui ne bloquent pas)

- **(a) « n'attendent jamais »** (Lucie, 18 novembre) : je le lis « n'attendent jamais les vecteurs ». L'attente du
  texte par usages et outline reste celle d'aujourd'hui, et des tests l'exigent. Si Lucie voulait « jamais rien », ces
  outils perdraient la déclaration `freshness` pour l'attente, mais garderaient la ligne de fraîcheur.
- **(b) La notification unique « index prêt »** : aucun canal n'existe. Le client mince ne sait notifier que
  `tools/list_changed`, par un rappel sans paramètre (`McpClient::set_notifier`), et le trait `Hote` de `mcp.rs` n'a
  aucune notification. Il faudrait un message neuf (`notifications/message`, niveau info, « l'index de la vue X est
  prêt »), posé par l'hôte quand la couche rattrape après une réponse « pas prêt », une fois par réponse. C'est un lot à
  part, sur l'hôte et le client mince : je le propose après celui-ci.
