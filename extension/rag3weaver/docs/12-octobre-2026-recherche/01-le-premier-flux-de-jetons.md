# Le premier vrai flux : la sortie du modèle en jetons

*12 octobre 2026, session recherche, à la demande de l'orchestration (point 3 de
ses décisions de la nuit ; page `04-les-buts.md` §4, moyen terme). La forme avant
le code, à relire par l'orchestration et par la session du proto, qui tient le
côté nœud (`LlmNode`, `Decide`).*

## 1. Ce qui existe

- **Le graphe sait déjà porter des jetons.** `ctx.token_sink(port)` et sa
  variante `token_sink_stop_when_unread` (`EmitSink`, `node.rs:338-346` et
  `586-608`) émettent chaque fragment sur un port de flux, et un témoin le
  prouve (`concurrent/tests.rs`). Aucun nœud de production ne s'en sert.
- **`LlmNode` remplit un `StringSink`** et ne publie le texte qu'à la fin, sur
  ses deux sorties de valeur `text` et `llm` (`llm_nodes.rs:111-135`). Son
  en-tête annonce déjà « l'étape 3 », un puits qui émet, sans toucher au trait
  ni aux ports existants.
- **Le client OpenAI pousse chaque jeton dès son arrivée** (SSE lu ligne à
  ligne, `openai_llm.rs:1415-1430`). Un `Flow::Stop` du puits ferme la socket
  et rend `Finish::cancelled()`.
- **`Decide` classe des textes entiers**, élément par élément (`json_of` exige
  un JSON). Un flux de jetons n'est pas son entrée.
- **La boucle d'agent** diffuse ses jetons à son appelant par un puits
  (`TeeSink`, puis `ChatSink` qui publie des événements `token`), hors du
  graphe. `model_out` porte la réponse entière.
- **Un modèle local** pour éprouver le protocole : `llama-server` et
  `~/models/qwen3-1.7b/Qwen3-1.7B-Q8_0.gguf`, joints par
  `RAG3WEAVER_LOCAL_LLM`. Rien n'écoute en ce moment ; le témoin lance son
  serveur. C'est pour le protocole seulement : le produit se juge avec Gemini.

## 2. La forme

### 2.1 `LlmNode` émet ses jetons (côté proto)

- Une sortie neuve **`tokens`** : `PortType::Text`, optionnelle,
  `.stream(PortKind::Finite)`. `text` et `llm` restent des valeurs posées à la
  fin, inchangées : qui lit le tout continue de lire le tout.
- **Un seul puits pour deux usages** : chaque fragment part sur `tokens` (par
  `EmitSink`) **et** s'accumule pour `text`. Le texte final est exactement la
  concaténation des jetons émis, et un témoin le vérifie.
- **Quand s'arrêter** : par défaut, seulement sur l'annulation du run, puisque
  `text` peut être lu ailleurs. Un réglage `stop_when_unread` (faux par
  défaut) fait rendre `Flow::Stop` quand plus personne ne lit `tokens` : un
  lecteur qui a vu ce qu'il cherchait arrête alors la génération, et le
  modèle cesse de coûter. `text` porte ce qui a été produit, et `llm.finish`
  vaut `Cancelled`.
- **Les jetons de raisonnement** (`on_reasoning`) : une sortie `reasoning`
  de même forme, plus tard, si un consommateur la demande. Hors de ce lot.

### 2.2 Le choix de l'exécuteur ne voit que les flux branchés (chez moi)

Avec une sortie `tokens` déclarée `Finite`, **tout graphe qui contient un
`LlmNode` passerait par l'exécuteur concurrent**, même sans lire `tokens`.
C'est un changement d'exécuteur pour tous les graphes LLM d'aujourd'hui,
sans raison. D'où : `has_streams` ne compte que les sorties de flux **dont une
arête part**. Un flux non branché **est jeté** (objection du proto, retenue) :
`ctx.emit` ne range rien, il compte seulement l'élément, pour qu'une sortie de
flux requise qui a émis reste « posée ». Rien ne finit dans la sortie du run,
sous aucun des deux exécuteurs : la fusion de fin de nœud disparaît (sous les
niveaux, un port de flux n'est jamais branché, puisqu'un flux branché fait
passer au concurrent), et un sous-graphe dont la sortie libre en flux n'est pas
branchée chez le parent ne rend plus ses éléments, seulement leur compte.
L'accumulation du puits double pour `text` n'est pas touchée. La parité des
graphes de valeurs est donc vraie par construction, et un témoin l'éprouve
(`flux_tests.rs`, `an_unconnected_stream_changes_neither_the_executor_nor_the_output`) :
le même graphe, avec et sans une sortie `tokens` non branchée, reste sous les
niveaux et rend la même sortie entière, port à port ; branché, il passe au
concurrent.

C'est « les ports portent des flux », la phrase de Lucie, qui prend forme : un
port déclare un flux, l'arête décide s'il coule, et rien ne change pour qui ne
le lit pas.

### 2.3 Deux consommateurs (côté proto, nœuds)

- **Afficher au fil de l'eau** : `TokenEvents` (une entrée `tokens` lue
  élément par élément) publie chaque fragment sur le bus de la session, sur
  un sujet à part, **`tokens`** (le bus des événements de nœuds n'en est pas
  noyé), au format que `ChatSink` emploie déjà (`{"event":"token","text":…}`,
  avec le run et le nœud). Une interface ou un agent appelant voit les jetons d'un graphe
  comme ceux d'une conversation.
- **Classer au fil de l'eau** : `Spot`, avec des motifs à étiquettes (texte
  ou expression régulière), garde une fenêtre glissante du texte reçu, émet une
  étiquette sur sa sortie (un flux) dès qu'un motif apparaît, et peut **lâcher
  son entrée** (`stop_on`). Avec `stop_when_unread` sur le `LlmNode`, le
  modèle s'arrête alors. C'est le classement en cours d'arrivée qui ne coûte
  rien.
- **`Decide` reste sur le texte entier** (`LlmNode.text → Decide`) : un appel
  au modèle par préfixe coûterait un appel par jeton.

### 2.4 Ce qui ne change pas

La boucle d'agent garde son puits ; `model_out` reste entier. Le jour où
`model_out` passera en jetons (le lot du proto), l'`EmitSink` deviendra le
puits intérieur du `TeeSink`, sans doublon.

## 3. Le partage

| chez le proto | chez moi |
|---|---|
| `LlmNode.tokens`, le puits double, `stop_when_unread` | `has_streams` sur les flux branchés, et son témoin de parité |
| `TokenEvents`, `Spot` | le témoin avec le modèle local (lancer `llama-server`, l'attendre, l'arrêter) |
| leurs schémas et le mermaid | la mesure (§5) et la page |

## 4. Les témoins, rouges d'abord

1. **Les jetons coulent pendant la génération** : un `CallbackLlm` qui ne rend
   son dernier fragment qu'après que l'aval a lu le premier (une poignée de
   main, comme le témoin 1 de la page 05). Il est rouge tant que `LlmNode`
   publie à la fin.
2. **Le texte est la concaténation des jetons** émis, au caractère près.
3. **La parité** : `LlmNode → Decide` sur `text` seul reste sous les niveaux
   (`has_streams` faux) et rend la même sortie entière qu'avant. Fait
   (`flux_tests.rs`, §2.2), rouge d'abord sur `has_streams`.
4. **L'arrêt anticipé** : `Spot` lâche son entrée au motif ; avec
   `stop_when_unread`, le `CallbackLlm` compte moins de fragments produits
   que sa réponse n'en a. Sans le réglage, il les produit tous, et `text` est
   complet.
5. **Le vrai modèle** (protocole) : `qwen3-1.7b` sous `llama-server`, une
   réponse de plusieurs centaines de jetons. Le premier jeton arrive au
   consommateur bien avant la fin de la génération, et `text` est égal à la
   concaténation.

## 5. La mesure

- **Le surcoût par jeton du passage par le graphe** : `CallbackLlm` qui émet
  100 000 fragments, avec le puits direct contre le passage par `tokens` vers
  un consommateur `PerElement`, en nanosecondes par jeton. Un jeton de modèle
  arrive toutes les dizaines de millisecondes : le canal ne doit pas peser.
- **Le premier jeton chez le consommateur** contre le premier jeton reçu du
  serveur, sur le modèle local (l'écart est le coût du graphe), et contre la
  fin de la génération (le gain d'afficher au fil de l'eau).
- La capacité des canaux (64) contre le débit d'un modèle : un consommateur
  lent fait refluer jusqu'au puits, donc jusqu'à la socket. C'est voulu ; la
  mesure le dira.

## 6. Les questions ouvertes

- Le volume sur le bus : un événement par jeton, ou des paquets de quelques
  millisecondes ? C'est à mesurer avec une interface réelle.
- `reasoning` en flux à part : seulement si un consommateur le demande.

## 7. Les résultats (12 octobre, matin)

`LlmNode.tokens` est sur master depuis `7627fa92d`, côté proto. Ses témoins 1,
2 et 4 (la poignée de main, la concaténation, l'arrêt anticipé avec et sans
le réglage) sont rouges d'abord. Le témoin 3, la parité, est chez moi
(`38a95fa9b`, §2.2). Le témoin 5 et la mesure sont dans
`src/dataflow/jetons_tests.rs`, en deux tests `#[ignore]` joués à la main :

```
scripts/cargo_magasin.sh test --lib --features rag3db-native,code,daemon,openai-llm \
  jetons_tests -- --ignored --nocapture --test-threads=1
```

**Le vrai modèle** (témoin 5,
`local_model_tokens_reach_the_consumer_before_the_end`). Le témoin lance
`llama-server` avec qwen3-1.7b Q8, sur le processeur (`-ngl 0`, six fils,
`--reasoning-budget 0`), et l'arrête par son propre pid, même sur un échec.
Le serveur est prêt en 1,8 s. Sur une réponse de 397 jetons :

| | depuis le départ du run |
|---|---|
| premier jeton reçu du serveur | 209,5 ms |
| premier jeton lu par le consommateur | 210,0 ms (écart **0,53 ms**) |
| dernier jeton lu | 14,14 s |
| fin du run | 14,14 s |

Le consommateur voit donc le premier jeton **67 fois plus tôt** que la fin de
la génération, et le graphe n'y ajoute qu'une demi-milliseconde. `text` est
exactement la concaténation des 397 jetons lus. Les bornes du témoin sont
larges : un premier jeton avant le quart de la génération, un écart sous
50 ms.

**Le surcoût par jeton** (`measure_token_overhead`) : 100 000 fragments d'un
`CallbackLlm`, en profil debug (un majorant) :

| chemin | ns par jeton |
|---|---|
| puits nu, sans graphe | 18 |
| `LlmNode`, `tokens` non branché (jeté, exécuteur par niveaux) | 312 |
| `LlmNode` → lecteur `PerElement` (exécuteur concurrent, canal de 64) | 1 135 |
| **le canal seul** | **822** |

Un jeton de qwen3-1.7b sur le processeur arrive toutes les 35,6 ms. Le canal
en coûte 0,8 µs, soit 0,002 % : il ne pèse pas. La borne du test est
20 µs par jeton. Le refoulement d'un consommateur lent vers la socket
(§5, troisième point) n'est pas encore mesuré : il attend `Spot` et
`TokenEvents`, qui sont les premiers vrais consommateurs.

Ces deux tests ne sont pas rouges d'abord. Ce sont une mesure et un témoin
du protocole sur un vrai serveur. Ce qui serait rouge si `LlmNode` publiait
seulement à la fin, la poignée de main, est le témoin 1 du proto.
