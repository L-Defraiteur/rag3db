# 14 — Tout est écoutable, tout est filtrable

26 août 2026, 7h30. Demande de Lucie : *« est-ce qu'on peut choisir
d'écouter un DAG en général — j'écoute tout de ce graphe, tout des graphes
avec tel tag, tout des graphes en général sur ces nœuds ; sur cet événement
je fais tel truc ; j'écoute ce nœud, quand tel input lui entre, quand tel
output lui sort. Vraiment 100 % tout est* listenable*. Et filtrable aussi à
chaque fois au max. »* Avec la garde qu'elle pose elle-même : *« faire
attention à ne pas écouter quand rien d'écoutable »*.

Suite du [07](../vision_roadmap_08_2026/07-evenements-runs-et-boucles.md)
(le bus, les runs, le réacteur) et du [13](13-la-session-comme-graphe.md)
(la session comme graphe — c'est elle, la première cliente).

## 1. Ce qu'on a, et pourquoi ça ne suffit pas

Trois mécanismes, nés séparément :

| | Ce qu'il porte | Qui l'entend | Filtrable ? |
|---|---|---|---|
| **le bus** (doc 07) | `NodeRun { run, node, node_type, ms, error }`, runs, appels, messages | n'importe qui, par sujet | par **sujet** seulement |
| **`DataflowEvent`** | `NodeStarted { inputs }`, `NodeCompleted { outputs, metrics }`, `NodeLog`, `NodeFailed` — avec les **instantanés de ports** | l'appelant du runtime, en local | pas du tout |
| **les taps** (`TapSpec`) | la **valeur** qui passe sur une arête précise | l'appelant du runtime | par arête, déclarée d'avance |

Le bus traverse les boucles mais ne dit presque rien d'un nœud ; les deux
autres disent tout mais ne sortent pas du runtime. Et un sujet n'est pas un
filtre : `dataflow` donne *tous* les nœuds de *tous* les graphes.

**Un seul mécanisme, donc** — le bus — avec des événements plus riches, un
sélecteur pour s'abonner, et un prédicat pour filtrer. Les taps deviennent
un cas particulier (un sélecteur sur une arête, au niveau `full`).

## 2. Le principe

> **Un événement est un fait structuré. On s'abonne par un sélecteur, on
> filtre par un prédicat, et on ne produit que ce que quelqu'un écoute.**

Trois pièces, dans cet ordre : le **sélecteur** dit *quoi* (structure), le
**prédicat** dit *lequel* (valeurs de champs), le **registre d'intérêt** dit
*si ça vaut la peine de le fabriquer*.

## 3. Le sélecteur

Cinq dimensions, chacune facultative — absente vaut `*` :

```
%% on: run=$parent                    -- tout du graphe qui m'a lancé
%% on: tag=search                     -- tout des graphes tagués `search`
%% on: kind=NodeFailed                -- tous les échecs de nœud, partout
%% on: node_type=BM25SearchNode       -- ce type de nœud, dans n'importe quel graphe
%% on: node=bm25, port=results, dir=out   -- ce que ce nœud sort sur ce port
%% on: node_type=LlmNode, dir=in      -- tout ce qui entre dans un nœud LLM
```

| Dimension | Valeurs |
|---|---|
| `run` | un identifiant, `$parent`, `$self`, ou `*` |
| `tag` | un tag déclaré par la fiche (`%% tags: search, code`) |
| `kind` | `RunStarted`, `RunFinished`, `NodeStarted`, `NodePort`, `NodeFinished`, `NodeFailed`, `NodeLog`, `LlmCall`, `ToolCall*`, `Message` |
| `node` / `node_type` | par nom d'instance ou par type |
| `port` / `dir` | un nom de port, `in` ou `out` |

Plusieurs `%% on:` s'additionnent (union). Un événement qui satisfait deux
sélecteurs d'une même montre **ne déclenche qu'une fois**.

**Les tags** : une fiche déclare `%% tags: search, code`, le runtime les
publie dans `RunStarted`, et l'abonné tient une petite table `run → tags`
pour filtrer les événements suivants. Le tag ne voyage pas sur chaque
événement — ce serait payer un champ à chaque nœud pour une information qui
ne change pas d'un run.

## 4. Le prédicat, et son coût

« Filtrable au max » a un prix qui n'est pas uniforme. Trois niveaux, et le
sélecteur dit lequel il veut :

| Niveau | Ce qu'on peut filtrer | Coût |
|---|---|---|
| **1. structure** (défaut) | tout champ de l'événement : `ms > 100`, `error != null`, `node_type = …`, `count = 0`, `bytes > 10_000`, `port_type = Results` | nul — l'événement existe déjà |
| **2. résumé** | ce qu'un instantané de port donne sans sérialiser la valeur entière : nombre d'enregistrements, type, taille | quasi nul |
| **3. contenu** | la valeur elle-même (`level=full`) : « quand l'entrée contient X » | **la sérialisation d'un port** — à demander explicitement |

La règle qu'on se donne : **le bus filtre sur des faits, pas sur des
valeurs métier.** Le niveau 3 existe (c'est ce que font les taps
aujourd'hui) mais il se demande, et un graphe qui veut décider sur le
*contenu* le fait avec ses propres nœuds (`GateNode`, `BranchNode`) —
sinon on réinvente un langage de requête dans un bus d'événements, et on
paie la sérialisation partout pour trois abonnés.

## 5. Ne pas produire ce que personne n'écoute

C'est la garde que Lucie pose, et c'est la partie qui a du contenu
d'ingénierie. Aujourd'hui, `execute_inner` construit
`input_snapshots: Vec<PortSnapshot>` **à chaque nœud, toujours**, même sans
un seul abonné.

Le bus tient un **registre d'intérêt** : un petit jeu de drapeaux recalculé
à chaque abonnement/désabonnement — « quelqu'un veut-il des événements de
port ? au niveau résumé ou contenu ? sur quels types de nœuds ? ». Le
producteur demande avant de fabriquer :

```rust
if bus.interest().ports(node_type) >= Level::Summary {
    // seulement alors, l'instantané
}
```

Trois propriétés à tenir : la question doit être **plus rapide que la
fabrication** (un `AtomicU8` par catégorie, pas une recherche) ; le registre
doit être **conservateur** (en cas de doute, on produit) ; et il doit être
**visible** — `bus.interest()` se lit, pour qu'un « pourquoi je ne reçois
rien » ait une réponse.

## 6. Trois règles payées d'avance

- **Un run ne s'écoute pas lui-même.** Un sélecteur exclut par défaut les
  événements produits par le run qui écoute. On a payé cette leçon cette
  nuit : le graphe de trace écrivait dans le catalogue, dont les graphes
  publiaient sur `dataflow`, qu'il écoutait — dix événements fantômes par
  drain. Avec des jokers, l'erreur devient facile ; l'exclusion est donc le
  défaut, et `include_self` est explicite.
- **Un sélecteur qui ne peut jamais correspondre est une erreur de fiche**,
  refusée à l'abonnement avec la liste des valeurs possibles — exactement
  comme `bad_choice` pour les cibles et les relations. Un `node_type` qui
  n'existe pas, un `port` que ce type de nœud n'a pas : on le sait, on le
  dit.
- **Une montre qui n'a jamais rien reçu se signale.** Après N minutes sans
  correspondance, un `WatchIdle { selector }` sur le bus. Un abonnement
  silencieux est indistinguable d'un abonnement cassé, et c'est ce qui fait
  perdre une heure.

## 7. Ce que ça donne pour la session

Le graphe de session du [13](13-la-session-comme-graphe.md) devient un
abonné comme un autre :

```
%% tool: session
%% tags: session
%% on: run=$parent, kind=NodeFailed        -- un outil a échoué : je le sais
%% on: run=$parent, node_type=LlmNode, dir=out
%% on: inbox                                -- ce qu'on me dit
%% policy: batch 50
```

Et le graphe de trace se resserre : au lieu d'écouter `agent` et `dataflow`
en entier, il écoute `kind=RunStarted|RunFinished|ToolCall*|LlmCall` et
laisse les nœuds tranquilles — ou l'inverse, selon ce qu'on veut garder.
C'est une ligne de fiche, plus une décision de code.

## 8. Ce qu'on ne fait pas

- **Pas de jointures ni d'agrégats** dans le sélecteur (« quand ce nœud a
  échoué *trois fois* ») : c'est un état, donc un nœud, donc un graphe.
- **Pas de rejeu.** Un abonné voit ce qui suit son abonnement ; l'histoire
  se relit dans `Trace`, qui est faite pour ça.
- **Pas de sélecteur sur le contenu par défaut** — voir §4.

## 9. L'ordre, et comment on saura

1. **`NodePort { run, node, node_type, port, dir, port_type, count, bytes }`**
   sur le bus, gardé par le registre d'intérêt. Test : sans abonné, aucun
   instantané n'est construit (un compteur le prouve) ; avec, on les reçoit.
2. **Le sélecteur et son prédicat de niveau 1**, `%% on:` étendu, plus la
   validation à l'abonnement. Test : `node_type=Inexistant` est refusé avec
   la liste ; `ms > 100` ne laisse passer que les lents.
3. **`%% tags:`** et la table `run → tags` de l'abonné. Test : deux graphes,
   un seul tagué, une montre par tag.
4. **L'auto-exclusion et `WatchIdle`.** Test : un graphe qui s'écoute
   lui-même ne boucle pas ; une montre qui ne reçoit rien le dit.
5. **Fondre les taps** dans le niveau 3, et retirer le mécanisme séparé.
6. **`DataflowEvent`** : ce qui reste utile (l'interface locale riche) garde
   son canal ; ce qui doublonne disparaît.

Les étapes 1 et 2 suffisent à rendre la session du doc 13 écrivable ; le
reste est du confort et du ménage.
