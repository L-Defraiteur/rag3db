# Reprise à 200k et observation des canaux de l’agent

Date : 20-09-2026. Suite au [premier comparatif](18-comparatif-local-bonsai-qwen-magic.md), Lucie demande de relancer les deux modèles à 200 000 tokens, d’observer leurs commentaires et la réflexion exposée par le provider, et de rester strictement dans sa collection.

## Contrat expérimental

Nouvelles configurations `chat-bonsai-200k.json` et `chat-qwen-200k.json`, mêmes outils et tâche, sessions distinctes, fenêtre déclarée 200 000, 32 générations maximum, sortie par génération 8 192 tokens, présentation outil 8 192 octets. Le budget de temps du runner est de 1 800 secondes. La consigne demande désormais de courts commentaires publics au fil de l’avancement. Bonsai garde son budget serveur de réflexion de 2 048 tokens. Ces différences rendent cette série distincte de la série 16k ; elles sont identiques côté application pour les deux candidats.

Les outils `search_cards` et `select_cards` sont liés à `OwnedCard` par le manifeste. Le backend refuse la substitution d’un paramètre lié. La recherche de graphes arbitraires est désactivée et la liste des outils accessibles exclut le catalogue de crafts. Les 5 380 entrées du snapshot de collection ont toutes `owned > 0`. Le modèle peut encore inventer une carte dans sa réponse ou dépasser une quantité : le périmètre de recherche ne remplace pas une validation de deck.

## Observation générique

`TokenSink::on_reasoning` reçoit le texte de réflexion que le provider choisit d’exposer. Valeur par défaut : ignorer et continuer. Le lecteur SSE compatible traite `reasoning_content` et l’alias `reasoning` séparément de `content`. Le décorateur de la boucle d’agent relaie le canal sans l’ajouter à la réponse publique. Le chat émet un événement JSONL `reasoning`; le runner le conserve comme les autres événements. L’annulation fonctionne aussi pendant ce canal.

Ce changement ne demande pas de réflexion à un modèle qui ne l’expose pas et ne reconstitue pas les essais antérieurs. Le contenu n’est pas implicitement réinjecté comme message public ni persisté dans `Turn` : il est conservé dans la trace de l’essai. `render_chat_trace.py` produit une transcription chronologique des commentaires, réflexions repliables, appels, résultats et mesures. Le TUI/web n’affiche pas encore ce nouveau canal explicitement.

## Validation et points d’architecture

Le test SSE vérifie la séparation des canaux et l’arrêt avant émission publique. Le test chat vérifie le passage du canal à travers l’agent. Les 67 tests du lecteur compatible passent. Un E2E navigateur a révélé une course dans le test au rechargement : il attendait l’historique mais supposait la liste des exports déjà chargée. L’attente porte maintenant explicitement sur les deux éléments asynchrones.

Le branchement modèle / outils / rendu reste générique. Les frictions observées portent sur la découverte et le typage des filtres, la validation des productions métier et la gestion cumulée du contexte. Une opération déclarative de validation de draft, exposée au même titre qu’une recherche, serait plus fiable que demander au modèle seul de compter et vérifier les possessions. Cela n’est pas implémenté par cette expérience.

## Résultats

Essais terminés, aucun deck conforme. Les serveurs ont été utilisés successivement puis arrêtés. Le service d’embeddings existant est resté actif. Les rapports, journaux serveur et transcriptions sous `experiments/mtga/data/model-benchmark-2026-09-20/` conservent les preuves.

| Mesure | Bonsai | Qwen |
|---|---:|---:|
| Fenêtre demandée / allouée (alignement runtime) | 200 000 / 200 192 | 200 000 / 200 192 |
| Cache K/V | Q8 GPU, 6 647 Mio | Q8 CPU, 9 970,5 Mio |
| Poids | GPU secondaire | GPU secondaire |
| Pic VRAM totale de la carte | 20,06 Gio | 29,94 Gio |
| Durée du parcours | 654,42 s | 108,66 s |
| Générations | 12 | 22 |
| Tokens générés | 24 372 | 3 014 |
| Plus grand prompt réellement envoyé | 23 058 tokens | 11 236 tokens |
| Appels outils / erreurs | 36 / 2 | 21 / 0 |

Les caches sont alloués pour 200k, mais aucun essai ne remplit cette fenêtre. Le cumul `tokens` du rapport compte plusieurs prompts successifs : il n’est pas la taille du contexte d’un appel. Qwen utilise `--no-kv-offload`, avec attention/cache en RAM hôte pour faire tenir ses poids Q6 et préserver les autres services sur le GPU. Il n’y a donc pas de comparaison pure de vitesse GPU entre ces deux réglages.

### Bonsai

La capture des canaux fonctionne : commentaires publics et réflexion séparée sont présents dans `bonsai-200k-run/events.jsonl` et `transcript.md`. Il explore notamment le cimetière et les Fungi, puis sélectionne des cartes par nom exact. Deux appels de recherche contiennent ensuite `field` et `field2` au même niveau d’un filtre qui attend une seule variante. La boucle s’arrête sur `RepeatedError`. Aucun fichier de deck. Ce n’est ni un dépassement de contexte ni une panne GPU. La politique d’arrêt après deux erreurs identiques mérite d’être évaluée séparément ; elle n’a pas été modifiée pendant cet essai.

### Qwen

Il enregistre `Explore and Seek Deck` dans `qwen-200k-state/artifacts/`. Les quantités des impressions choisies respectent cette fois la collection, mais `deck-audit.json` constate **57 cartes dont 16 terrains**, contrairement à sa déclaration 60/24. Les sorts demandent vert/bleu/rouge, tandis que les seuls terrains de base retenus sont des plaines. Les Landscapes ne fournissent pas directement ces couleurs et ne peuvent pas chercher des terrains de base absents du deck.

Son export texte ne correspond pas au JSON : il omet la plaine DFT 279, soit 56 cartes exportées. Son en-tête `Deck: Explore and Seek Deck` diffère aussi du gabarit demandé. Les fichiers originaux restent inchangés comme preuves, pas comme propositions validées à importer.

### Vérifications finales

67 tests du lecteur compatible, 5 tests chat et 4 E2E TUI/web passent. Le test navigateur attend désormais explicitement le chargement asynchrone des exports après rechargement. Le renderer de trace compile en Python et `git diff --check` passe. Le canal reasoning est journalisé par le runner ; son affichage interactif et sa persistance dans les sessions ordinaires restent à traiter si demandés.

Ces résultats justifient d’expérimenter un contrat `submit_result` avec validation structurée et métier, puis des hooks d’export sur données acceptées. Voir le [design 20](20-contrat-resultat-et-harness-declaratif.md). Aucun harness de correction n’a été ajouté rétroactivement à l’un des candidats.
