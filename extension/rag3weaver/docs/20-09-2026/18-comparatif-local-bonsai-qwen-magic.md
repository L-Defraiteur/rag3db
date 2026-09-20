# Comparatif local Bonsai / Qwen sur la R9700 secondaire

Date : 20-09-2026. Expérience demandée par Lucie : faire construire un deck Magic aux deux modèles, sur sa seconde carte, avec un branchement configurable et réutilisable. Essais terminés : allocation et outils opérationnels, aucun deck validé sur ce premier protocole. Ne pas confondre démarrage du modèle et réussite du parcours.

## Matériel et isolation

Deux AMD Radeon AI PRO R9700 de 32 Go. La secondaire est au bus PCI `07:00.0`, `card0`, sélectionnée par `HIP_VISIBLE_DEVICES=1` dans l'installation actuelle. La principale est `04:00.0`, `card2`. Le service d'embeddings BGE-M3 existant reste actif ; la mémoire GPU mesurée est donc la mémoire totale de la carte, pas uniquement celle du LLM. L'activité de bureau/jeu sur la principale n'est pas arrêtée.

Le fork PrismML publie un binaire Linux ROCm 7.2 : `prism-b10709-9a9394a`. Son énumération des GPU et une première réponse correcte fonctionnent sur cette machine. Aucune compilation AMD supplémentaire nécessaire à ce stade. Le llama.cpp existant de Lucie reste inchangé.

Bonsai 2 PQ2_0 : 7 206 168 928 octets, SHA-256 `3907dc1658db1f78a9826bf8d5bcb8dc65db0d466388937af57f2294fae62ec1`. Le fichier de 7,21 Go est choisi pour le support ROCm déclaré par leurs scripts ; ce n'est pas le packing PTQ1_0 de 5,95 Go. Le manifeste de téléchargement conserve URLs, révision du modèle, version du binaire et hashes.

Comparateur déjà installé : `Qwen3-Coder-30B-A3B-Instruct-abliterated.i1-Q6_K`, via le llama.cpp HIP local au commit `d2462f8`. **Ce n'est pas Qwen3.8-27B FP16.** Le résultat compare deux solutions disponibles sur cette machine ; il n'isole ni l'effet de la quantification, ni celui de l'architecture, ni celui du runtime.

## Branchement commun

Pas d'adapter Bonsai ou de boucle d'agent Magic dans le code. Les deux passent par `OpenAiLlm`, `Agent`, le même hôte JSONL et les outils issus du manifeste du backend. Les fichiers `chat-bonsai.json` et `chat-qwen.json` diffèrent uniquement par l'adresse/modèle et le dossier de session. Ce sont aussi les fichiers utilisables directement avec le TUI et le chat web.

Le runner [`benchmark_chat.py`](../../scripts/benchmark_chat.py) prend une configuration, un fichier de consigne et un dossier de résultats. Il enregistre les événements et les mesures ; il ne choisit aucune carte, ne traduit pas les requêtes et ne corrige pas la réponse du modèle. Les mesures par génération sont émises par la bibliothèque `chat` pour tout provider, pas seulement par le runner. Température et top-p sont désormais configurables dans `ChatConfig`.

Les graphes, le rendu compact des cartes et la consigne métier sont des fichiers de l'expérience. Les recherches exécutent le vrai moteur `search_cards` / `select_cards` dans `OwnedCard` : lucivy et vecteurs BGE-M3 existants. Aucun corpus présélectionné par l'assistant ni liste de deck fournie aux modèles.

Le backend utilise une copie reflink de la base et de ses checkpoints ; ses outils d'ingestion/écriture ne sont pas exposés. La persistance de ses capacités imbriquées a un défaut connu : celles-ci sont exclues des champs retournés et du rendu. Les textes principaux, IDs et quantités des sorties doivent être audités contre le snapshot, sans présenter cette restriction comme une réparation de la base.

## Exercice fixé avant les générations

Même prompt, mêmes outils, même fenêtre de 16 384 tokens, même température 1,0 et top-p 0,95. Maximum 18 générations, 4 096 tokens de sortie par génération ; interruption coopérative demandée après 900 secondes. Bonsai dispose d'un budget serveur de réflexion de 2 048 tokens. Le Qwen Coder installé est un modèle instruct ; il ne faut pas assimiler ses tokens à un mode thinking identique.

Le modèle doit découvrir une synergie originale dans la collection, comparer quelques pistes, sélectionner les impressions possédées et produire `deck.json` et `deck.arena.txt`, 60 cartes dont 24 terrains, aucun craft. Quantités et noms doivent provenir des outils ; les terrains de base possédés sont disponibles en quantité libre. Ce gabarit fixe un exercice comparable, pas une prescription universelle de taille de deck.

Le snapshot ne contient pas de registre de légalité à jour. La validité des quantités et de l'export peut être contrôlée ; une légalité compétitive actuelle et un taux de victoire ne peuvent pas être certifiés par ce test.

## Fichiers et commandes

Racine des données : `experiments/mtga/data/model-benchmark-2026-09-20/`, déjà ignorée par Git. Elle contient le runtime isolé, les poids, `downloads.json`, les deux configurations d'application, `task.txt`, le manifeste de lecture, les templates et les sous-dossiers de mesures/sessions/exports.

Depuis la racine de rag3db, après lancement du serveur de modèle correspondant :

```bash
python3 extension/rag3weaver/scripts/benchmark_chat.py \
  experiments/mtga/data/model-benchmark-2026-09-20/chat-bonsai.json \
  experiments/mtga/data/model-benchmark-2026-09-20/task.txt \
  experiments/mtga/data/model-benchmark-2026-09-20/bonsai-run \
  --memory-path /sys/class/drm/card0/device/mem_info_vram_used \
  --memory-path /sys/class/drm/card2/device/mem_info_vram_used
```

Pour Qwen, remplacer uniquement `chat-bonsai.json` par `chat-qwen.json` et `bonsai-run` par `qwen-run`. Pour utiliser une configuration de façon interactive :

```bash
python3 extension/rag3weaver/scripts/chat_app.py \
  experiments/mtga/data/model-benchmark-2026-09-20/chat-bonsai.json
```

Ne pas ouvrir simultanément deux hôtes sur la copie de base. Les modèles sont testés successivement, en libérant le précédent avant de charger le suivant.

## Mesures à interpréter

Le temps par génération inclut le traitement du prompt et la génération. Son quotient tokens/temps **n'est pas** un débit de décodage pur. Les journaux llama.cpp donnent séparément prompt evaluation et token generation. La première réponse non vide, les appels réels aux outils, leurs erreurs, le temps total du parcours et le résultat de validation du deck comptent autant que les tokens/s.

Les caches OS et modèle ne sont pas vidés artificiellement. Le service d'embeddings est partagé et peut être chaud au second essai. Une seule construction de deck par modèle reste exploratoire : elle ne démontre pas un classement général de qualité.

Sources d'installation : [version ROCm épinglée](https://github.com/PrismML-Eng/llama.cpp/releases/tag/prism-b10709-9a9394a), [modèle Bonsai](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf), [sélection officielle des backends](https://github.com/PrismML-Eng/Bonsai-demo/blob/main/scripts/common.sh).

## Incident du premier essai et correction commune

Premier essai Bonsai : trois générations, six recherches hybrides, puis une sélection de toutes les faces principales non-jetons. Le provider refuse le prompt suivant : 390 779 tokens pour 16 384 disponibles. Aucun deck final n'est produit. Cet échec est conservé dans `bonsai-run/` ; ce n'est pas un échec d'allocation GPU.

À la demande de Lucie, la correction est un plafond de **présentation à l'agent**, pas une limite du moteur : `ToolOutputLimit` est un décorateur de `ToolBox` dans la bibliothèque `agent`, utilisable hors MCP et hors chat. Il conserve un préfixe lisible et affiche `... N more lines (output truncated)`, sans transformer ce cas en erreur métier. La quantité omise est comptée, UTF-8 est respecté et l'identité de l'appel est conservée. La recherche ou un export vers fichier peuvent continuer à traiter la totalité des résultats. Le runner ne filtre aucune carte.

Les deux configurations reçoivent le même plafond de 8 192 octets par résultat et la même consigne : pas de sélection exhaustive de collection ; sélectionner des IDs/noms ciblés et relire une carte si son texte est coupé. L'essai Bonsai suivant est distinct, dans `bonsai-bounded-run/`, avec une nouvelle conversation ; les résultats du premier essai ne sont pas effacés.


## Cache : allocation réelle jusqu’à 262k

Le réglage initial de 16k était un choix d’exercice, **pas une limite matérielle**. Les métadonnées GGUF locales donnent pour Bonsai 16 couches d’attention complète, 4 têtes K/V de dimension 256 : 64 Kio de cache FP16 par token. Qwen Coder a 48 couches, 4 têtes K/V de dimension 128 : 96 Kio par token. À 262 144 tokens, cela représente respectivement 16 Gio et 24 Gio, hors poids, états récurrents et buffers. La quantification des poids ne quantifie pas automatiquement le cache.

Mesures avec `probe_server_memory.py`, GPU secondaire uniquement, un slot, Flash Attention, K/V FP16, batch 512 et microbatch 256 :

| Contexte Bonsai | VRAM totale après courte génération | Réponse |
|---|---:|---|
| 16 384 | 13,60 Gio | 42 |
| 65 536 | 16,61 Gio | 42 |
| 131 072 | 20,65 Gio | 42 |
| 262 144 | 28,72 Gio | 42 |

La carte expose 31,86 Gio. Les services préexistants restent actifs et représentent environ 5,68 Gio après arrêt de chaque serveur. Ces essais valident le chargement et une courte génération avec le cache alloué : **ni un prompt de 262k effectivement rempli, ni sa vitesse de traitement, ni la qualité à longue distance**. Config et rapports : `memory-probe.json`, `cache-probe/report.json`, journaux par taille.

Les anciens relevés Qwen retrouvés dans `docs/26-aout-2026-20h29/03-commandes.md` documentent 131 072 tokens avec cache Q8 et 32,02 Go de VRAM. Ils ne permettent pas de reconstituer le réglage à environ 250k évoqué par Lucie. À précision identique, le cache Bonsai est plus petit que celui de ce Qwen, et non plus lourd.

## Résultat Bonsai avec sorties bornées, fenêtre 16k

`bonsai-bounded-run/report.json` : 316,64 secondes, 12 générations, 17 appels d’outils dont 4 erreurs. La dernière génération atteint `MaxTokens` à 14 616 tokens de prompt + 1 768 générés ; aucune réponse finale ni fichier de deck. Le transport renvoie `ok: true` car la boucle termine normalement : **cela ne signifie pas que la tâche deck est réussie**.

Deux erreurs concernent une comparaison scalaire avec une liste de couleurs (`Blue/Green is not in STRING[] range`), deux des filtres sans champ `value`. Le modèle poursuit des recherches, parfois répétitives. Ce résultat mélange difficulté d’usage des filtres et plafond de contexte ; il ne démontre pas que Bonsai serait incapable de construire un deck avec une fenêtre plus grande. Le clipping évite le retour massif de 390k tokens, mais ne remplace pas une gestion du budget cumulé de conversation.

Validation du changement de présentation : les cinq tests ciblés de chat passent, ainsi que les quatre E2E TUI/web (outils, exports, historique, redémarrage, authentification et annulation).


### Mesure complémentaire : Bonsai 262k avec cache Q8

`cache-probe-q8/report.json` : démarrage réussi et réponse « 42 », 22,19 Gio de VRAM totale après génération, contre 28,72 Gio en FP16. Même contexte, même GPU, mêmes services présents. Le cache quantifié a ses métadonnées et ses buffers : on rapporte la mesure totale plutôt que de simplement diviser toute la mémoire par deux. Il reste environ 9,67 Gio sur les 31,86 Gio exposés. Aucun test de qualité à contexte rempli ou de régression due à Q8 n’est inclus.

Un seul serveur LLM était chargé à la fois. Chaque serveur de mesure était arrêté avant le suivant, avec retour de la carte à environ 5,68 Gio. Qwen a été lancé après les mesures FP16, puis arrêté avant la mesure Q8.

## Résultat Qwen et audit du deck

`qwen-run/report.json` : 47,88 secondes, 18 générations, 17 appels d’outils dont 6 erreurs, 3 173 tokens générés. Un `deck.json` est enregistré ; aucun `deck.arena.txt`. L’audit séparé `qwen-run/deck-audit.json` conserve le résultat brut sans le corriger.

Le JSON contient **76 cartes dont 16 terrains**, alors que la réponse affirme 60/24. Quatorze lignes dépassent la quantité possédée de l’impression choisie ; les terrains de base sont explicitement exemptés de cette limite. Tous les IDs existent dans le snapshot, mais cela ne suffit donc pas à rendre le deck valide. Le plan annonce noir/rouge, alors que les cartes demandent les cinq couleurs et que les seuls terrains de base retenus sont des plaines. Il affirme également à tort que tous les Eldrazi produisent des Spawn et que Devoid réduit les coûts. **Ne pas importer cette proposition comme un deck validé.**

### Comparaison exploratoire, sans gagnant qualité

| Mesure | Bonsai, sorties bornées | Qwen |
|---|---:|---:|
| Durée du parcours | 316,64 s | 47,88 s |
| Générations | 12 | 18 |
| Tokens générés | 13 275 | 3 173 |
| Appels / erreurs outils | 17 / 4 | 17 / 6 |
| Deck conforme | non, pas de fichier | non, JSON invalide pour la consigne |
| Pic VRAM totale pendant parcours 16k | 13,70 Gio | 30,79 Gio |

Les dernières générations longues du serveur donnent environ 44–46 tokens/s pour Bonsai, 85–90 pour Qwen. Il ne s’agit ni d’une moyenne contrôlée multi-runs, ni d’un débit à 262k remplis. Bonsai utilise ici un mode thinking ; Qwen Coder est un MoE instruct. Les 6,6 fois d’écart de durée comprennent aussi des quantités de tokens et des stratégies différentes.

Les essais montrent deux besoins distincts pour le produit : exposer des filtres plus faciles à découvrir et vérifier automatiquement un draft avant de l’annoncer utilisable. Le clipping du résultat outil est indépendant de ces validations ; il ne suffit pas à gérer l’accumulation de tous les tours. Une reprise avec contexte plus grand pour Bonsai demeure une expérience ultérieure, pas une réussite déduite de l’allocation du cache.

## Vérifications finales

1 100 tests Rust passent hors sandbox. Le dernier test `origin` échoue car `/tmp` contient déjà un dépôt Git ; relancé isolément avec `TMPDIR=/var/tmp`, il passe. Les cinq tests ciblés chat et les quatre E2E TUI/web passent. `git diff --check` ne relève pas d’erreur. Les serveurs LLM lancés pour cette expérience sont arrêtés ; les services et installations préexistants sont conservés.
