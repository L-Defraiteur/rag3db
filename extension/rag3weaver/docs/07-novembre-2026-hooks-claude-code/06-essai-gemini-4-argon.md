# Essai de `gemini-4-argon` contre `gemini-3.8-flash` sur `e2e_equipier`

7 novembre 2026, rag3weaver archi, à la demande de c7. Pour que Lucie compare le 8 novembre avec
sa console de facturation Vertex : ses crédits startup couvrent-ils Argon ?

## Ce qui a été joué

Le même témoin `e2e_equipier`, deux fois de suite, sur le même binaire : master 120695b37,
features du jeu e2e plus `openai-llm,anthropic-llm`. L'équipier écrit le README d'un petit dépôt
Rust de trois fichiers, puis le juge note son pas.

- Projet `lr-hub-472010`, région `global`. Les identifiants passent par leur chemin dans `.vault`.
- **Seule `VERTEX_MODEL` change** (`google/gemini-3.8-flash`, puis `google/gemini-4-argon`). Le
  manifeste du témoin n'a pas de `models.llm` : **l'équipier et le juge prennent tous deux ce
  modèle**. Aucun manifeste de Lucie n'a changé.
- La copie du témoin gardait son dossier pour relire le journal des runs. Elle était dans un
  worktree jetable, n'a pas été poussée et a été effacée.

Un premier passage, à 16 h 27, n'a rien appelé : `run_e2e.sh` bâtissait sans `openai-llm` et le
chat était éteint. **Rien n'a donc été facturé avant 16 h 32.**

## Les heures

| Modèle | Témoin lancé | Équipier | Juge |
|---|---|---|---|
| 3.8 Flash | 16:32:06 | 16:32 à 16:39 environ, 430 s | fini avant 16:41, note 10 |
| 4 Argon | 16:40:57 | 16:41 environ, 34 s | 16:41 à 16:46 environ, 429 à chaque essai |

Les événements d'appel du journal n'ont pas d'horodatage propre. Les heures se déduisent du
lancement du témoin et des durées en millisecondes des appels.

## Les jetons de l'équipier, lus dans le journal du run

Somme des événements `LlmCall` et `Consumed` du run `agent-…` de chaque tâche.

| | 3.8 Flash | 4 Argon |
|---|---|---|
| Appels au modèle | 9 | 4 |
| Jetons d'entrée (prompt) | 53 645 | 29 214 |
| dont lus en cache | 3 037 | 14 944 |
| Jetons de sortie (completion) | 409 | 550 |
| dont raisonnement | 704 au compteur | 0 au compteur |
| Durée cumulée des appels | 429,4 s | 33,3 s |
| Appel le plus long | 252,9 s | 10,3 s |

Sur Flash, la réponse « completion » ne compte pas les jetons de raisonnement, que le compteur
range à part : 704 jetons de raisonnement pour 99 jetons de sortie au compteur. Sur Argon, le
compteur n'en voit aucun. Trois appels Flash ont duré 89 s, 253 s et 50 s, sans reprise au
journal : la lenteur est côté Vertex.

## Le juge

- **Sur Flash** : le pas est jugé, note **10**. Le juge n'écrit pas ses jetons au journal des
  runs. Il a fait au moins un appel, sans plus de précision possible.
- **Sur Argon** : **aucun bilan**. Le juge a reçu `HTTP 429 RESOURCE_EXHAUSTED` (« Resource
  exhausted. Please try again later ») au premier appel et aux 3 reprises, pendant 297,9 s.
  L'équipier venait de faire 4 appels Argon sans erreur, en 34 s. Le témoin rougit sur son
  attente de 10 minutes. Il n'a pas été rejoué : rien de plus avant la lecture de la facturation.

Pour la facturation : 4 appels Argon facturables de l'équipier, puis 4 requêtes du juge refusées
en 429. Un 429 ne devrait pas être facturé ; la console le dira.

## Ce que la réponse a de différent

Les deux README sont exacts et courts. Les deux rapports disent « fait sans plan », en un pas.

- **Argon lit tout d'un coup** : un appel à 4 outils, contre un fichier par appel sur Flash. D'où
  4 appels au lieu de 9, et plus de cache.
- **Argon donne les signatures** (`entiers(ligne: &str) -> Vec<i64>`, `milliers(n: i64) ->
  String`) et une section sur la fonction `somme`.
- **L'exemple d'Argon couvre les deux modules** : `somme("1000 2000 ignore 500")` vaut 3500, puis
  `format::milliers` rend `"3\u{202f}500"`. Les deux assertions sont justes. Flash n'illustre
  que `somme`.
- Une imprécision chez Argon : sa phrase d'ouverture dit que la bibliothèque « les formate »,
  comme si `somme` formatait. C'est `format::milliers` qui le fait.

## Ce qui reste à Lucie

Lire la facturation du 7 novembre entre 16 h 32 et 16 h 47 : 9 appels Flash plus ceux du juge
Flash, 4 appels Argon et 4 refus 429. Si Argon est couvert, le quota du juge est à regarder
avant tout usage : le 429 est arrivé dès le cinquième appel en une minute.
