# Everything declarative — rapport de session du 25 octobre 2026

Session « everything declarative » (I), orchestrée par c7. Tout le travail se fait dans des
worktrees à soi. Les passes et les pushes tournent à luciepc, sous `systemd-run --user`, et un
push ne part qu'après une passe verte, suivi de trois contrôles. Ce qu'on sait du code est
dans `02-knowledge-dump.md`.

## Sur master aujourd'hui

| Hash | Ce qu'il pose |
|---|---|
| `a454d6a91` | 1 Tio, C4 (l'arrêt de tout run depuis l'IDE), K1 (le chat de l'atelier, service), K2 (sa vue dans l'IDE), K3 (le témoin Gemini) — un passage de cinq commits |
| `82c4f7ff9` | le magasin : un plancher que git ne peut juger n'est jamais tenu (`verifier` rend 3) |
| `067bb8814` | le témoin K2 : le second chargement attend « flux : ouvert » |
| `527a0356c` | R1 : le budget de raisonnement se déclare par modèle (`models.llm.reasoning`), et un run compte à part les jetons de raisonnement et le cache ; avec `7179b330e` (la page relit l'état du chat à chaque ouverture du flux) et `0ddb28945` (`verifier courant`) |
| `2e205927a` | le lien de la proposition d'un tour vient de la réponse du tour (`930cccc96`), et trois témoins d'annulation sur barrière |
| `8ef9791dd` | le relais du flux de l'IDE a ses propres fils (ticket fermé : la vue vivante gelée par un graphe long) |
| `cbb41d312` | la vue outline : le rendu d'un Plan en json (`a60ecd618`), la fiche d'un fichier montre le plan d'outline |
| `ca53f8e3f` | le code des pages de l'IDE et de l'atelier en fichiers servis (`ide/page.js`, `atelier/page.js`) |

## Le témoin du chat dans la page : quatre défauts, un seul symptôme

`le_chat_se_mene_et_s_arrete_dans_la_page` a rougi quatre fois sous la charge de luciepc, avec
toujours le même message : « … jamais vraie en 20 s ». Il y avait quatre causes distinctes, et
trois étaient aussi des défauts du produit.

1. **Le second chargement n'attendait pas le flux** (G, 1 sur 1 813). Un tour envoyé avant
   l'ouverture de l'EventSource publiait son `RunStarted` à une fenêtre qui n'existait pas encore.
   Le témoin attend maintenant « flux : ouvert » (`067bb8814`) ; c'est une barrière, puisque
   l'abonné est inscrit sous le verrou avant que l'en-tête parte. Côté produit, la page relit
   l'état du chat à chaque `onopen` (`7179b330e`).
2. **Le lien de la proposition arrivait après la fin du tour** (codeparsers). Le flux relaie
   chaque sujet par sa propre tâche, et `declarations` n'est pas ordonné par rapport à `agent`.
   Le lien vient désormais aussi de la réponse du tour, et les événements d'outil vont à la
   bulle de leur run, même finie (`930cccc96`).
3. **Le relais attendait derrière les graphes** (session recherche, sur le tour lent). Le relais
   tournait sur le runtime tokio du crate, qui n'a que deux fils pour tout le processus, et
   les nœuds qui bloquent les y tiennent. Le rouge n'apparaissait donc que dans la lib entière,
   jamais au témoin seul, même sous la charge d'un autre processus. Une sonde le prouve : les
   deux fils tenus 25 s pendant le tour lent donnent 3 rouges sur 3 avant le correctif, 3 verts
   sur 3 après. Chaque relais a maintenant son propre fil, en `recv_blocking` (`8ef9791dd`).
   En service, un graphe long gelait de même la vue vivante.
4. Le témoin attend désormais l'hôte avant la page (le tour en vol, puis arrêté, par
   `GET /ide/chat`, avec une borne de sûreté de 120 s). La page a ensuite 20 s pour le montrer,
   et si elle ne le fait pas, le message le dit. c7 l'a retenu comme ce qu'on peut faire de
   mieux sur du rendu.

## Les témoins d'annulation

`a_value_subgraph_dies_with_its_levels_parent` (rouge chez la mémoire) annulait 50 ms après le
départ du parent. Sous charge, l'annulation tombait avant que le sous-graphe lance son niveau,
que le moteur sautait alors, ce qui est juste. Une sonde ralentit la fabrique à 200 ms : master
rouge 10 sur 10, correctif vert 10 sur 10. Ce témoin et ses deux voisins de même forme annulent
désormais sur une barrière (le nœud est entré), avec une borne de sûreté qui dit pourquoi elle
tombe.

## La vue outline

Conçue avec l'arbre principal (le rendu) et codeparsers (l'outil), qui ont relu tous deux.
`RenderResultsNode` gagne `json = results | rows` et `provenance`. En json=rows, il rend
`{rows, total, provenance}`, toujours avec ces trois clés, et pose l'objet sur `text` pour que la
ligne de fraîcheur de l'index ne le préfixe pas. Les refus sont nommés, à la construction, à
l'exécution et au chargement (`verifier_le_cablage`, qui ne juge que si le format peut valoir
json). `outline` prend `format=json`, son markdown restant identique octet pour octet à celui
d'avant. La vue ne lit que ce json : les scopes sont imbriqués par lignes, chacun mène à sa
ligne et à son symbole, et la provenance s'affiche sous le plan.

## Ce qui a mal tourné, et ce que j'en garde

- `relais-ide-1` ne compilait pas : `sujet` était déplacé dans le fil puis réutilisé. Un essai
  rouge pour rien.
- Une liste de chemins dans `$F` sous zsh ne s'est pas découpée, et `ide-js-2` a été poussée en
  pointant sur la tête d'outline. Elle est sans dégât et laissée telle quelle. Les listes de
  chemins passent maintenant par des scripts bash.
- Un raw string `r#"…"#` contenait `"#L4"`. Il faut `r##"…"##`.
- Le bloc `outils` de l'IDE ne nommait pas `outline`. Une passe partielle ne jouait pas
  `ide::tests`, et la lib entière l'a vu.
- Le target `target-I-apres` a passé 22 Go, au-delà du plafond de 20, et e2e a refusé de
  partir. Je l'ai effacé, puis rebâti.
- Le README des tickets a été en conflit au cherry-pick, parce que master avait fermé d'autres
  tickets entre-temps. Ma ligne a été réintégrée et le compte recalculé.

## Réglages des passes (luciepc)

- `I-court-reprise.sh`, dont toutes mes passes tirent leurs variables :
  - features : le jeu le plus large, `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm` (c7, après le rouge de cited_by) ;
  - `RAG3WEAVER_PG=postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test` (le conteneur de luciepc) ;
  - la lib épinglée par identifiant, `rag3db-build-04063ee06…`, jamais par `courant` ;
  - les anciennes versions sont dans `.avant-openai` et `.avant-04063ee06`.
- Chaque passe de push dit la lib liée (« lib du magasin », « moteur lié »).

## Comment reprendre

- Rien n'est en attente de push. Les branches dépassées restent telles quelles : `relais-ide-1/2`,
  `outline-json-1..3`, `ide-js-1/2`, `pile-relais-outline-js-1`, `raisonnement-1/2`,
  `annulation-barriere-1/2`, `k2-flux-ouvert`.
- Worktrees de luciepc gardés entre les lots : `rag3db-I-apres` (push), `rag3db-I-mesure`
  (essais), avec leurs targets. Ils sont à effacer en fin de session.
- La pièce 3 de la session recherche (la page de recherche : scores, signaux, avertissements,
  case « reclasser ») touchera `ide/page.js` ; je la relis quand elle me la soumet.
- Le prochain lot des visions est à proposer à c7 pour la nuit.
- Reste à faire, d'avant : le lot qui porte à 900 s la patience par défaut du témoin K3 (les
  tours Gemini mesurés : 241, 291, 204, 42 s, et un au-delà de 300).
