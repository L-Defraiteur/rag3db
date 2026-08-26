# Les paramètres de config sont des entrées du graphe — et ils héritent

26 août 2026, après minuit. Suite du point 1 du [08](08-objectifs-immediats-et-long-terme.md)
et de la discussion sur les schémas imbriqués.

## 1. Le problème qu'on a fermé

Un graphe a deux sortes d'entrées : les **ports** (typés, déclarés par les
nœuds, vérifiés à la construction) et les **paramètres de config** (`$var`
dans le gabarit). Les ports étaient de première classe ; les `$var` n'étaient
que de la substitution de texte. Une fiche `%% param:` redisait tout — type,
défaut, description, et depuis hier `choices` — à côté du nœud qui savait
déjà tout ça. `FetchRelatedNode` déclarait `direction` avec la description
« Outgoing or Incoming » ; `search_expand.mmd` le redisait en `%% param:`
puis en `%% choices:`. Trois fois la même information, aucune reliée aux
autres.

Le trou de l'imbrication n'était qu'un symptôme : `as_node_factory` copiait
`params` sans `choices` parce que `choices` n'était pas *dans* le paramètre.
Un outil composé qui aurait fait suivre `$target` à un `SearchTool` sans le
redéclarer aurait perdu l'`enum` en silence.

## 2. Ce qui a changé

**`Choices` et `json_schema` descendent dans `ConfigParam`**
(`node_registry.rs`), le vocabulaire des nœuds. Le savoir est déclaré là où
il naît, une fois :

| Nœud | Paramètre | Valeurs admises |
|---|---|---|
| `SearchSourceNode` | `target_name` | `@targets` — les cibles du catalogue |
| `FetchRelatedNode` | `relation` | `@relations` — les relations du schéma |
| `FetchRelatedNode` | `direction` | `Outgoing \| Incoming` |

**Un `$var` du gabarit est une entrée de config du graphe, câblée sur les
paramètres de nœuds qu'il alimente — et il en hérite** (`GraphTool::bind`) :

- un paramètre déclaré **sans type** (`%% param: direction -- Sens…`) prend
  le type, le défaut et le caractère requis du paramètre de nœud ;
- un paramètre sans `choices` ni sous-schéma hérite de ceux du nœud ;
- ce qui est déclaré dans la fiche **prime** ;
- un `$var` câblé sur deux nœuds qui ne sont pas d'accord (types, listes)
  est une erreur de fiche, avec les deux extrémités nommées — on tranche en
  déclarant.

**L'imbrication est gratuite.** `as_node_factory` publie des `ConfigParam`
complets, donc le nœud `SearchTool` porte `target: @targets`, et
`search_expand` en hérite exactement comme d'un nœud de base. Un seul
mécanisme pour tous les niveaux. `builtin_graph_tools` lie chaque fiche au
registre qu'elle voit : `search` contre les nœuds de base, puis `SearchTool`
est enregistré, puis `search_expand` contre ce registre-là.

**La vérification a trois étages**, du plus tôt au plus tard :
`NodeRegistry::create` refuse une valeur hors liste close pour *n'importe
quel* nœud (un `FetchRelatedNode(direction='Sideways')` ne retombe plus en
silence sur `Outgoing`) ; `GraphNodeFactory::create` fait de même pour un
nœud-gabarit ; la fiche (`validate_arguments_with`) vérifie en plus les
listes du catalogue, là où le catalogue est disponible.

**Les fiches s'allègent.** `search.mmd` n'a plus de `%% choices:` ;
`search_expand.mmd` non plus, et `direction` s'y écrit
`%% param: direction -- Sens de parcours depuis chaque résultat.` — c'est tout.

**Le point 2 pour le même prix.** `json_schema: Option<Value>` sur
`ConfigParam` ; `param_schema` rend le sous-schéma déclaré à la place de
l'objet libre. Aucun nœud n'en déclare encore — le vocabulaire est prêt pour
le jour où un outil prendra des filtres structurés.

## 3. Ce qu'on a vérifié

- Unitaires : 821 (`code,openai-llm`), 730 (défaut). Nouveaux : héritage à
  travers deux niveaux (`search_expand.target` ← `SearchTool` ←
  `SearchSourceNode`), refus à la création d'un nœud de base, conflit
  entre deux nœuds et explicite qui prime, sans-type non câblé, type
  déclaré en désaccord, grammaire de `%% param:` sans type.
- E2E : `e2e_code` 5, `e2e_graph_tool` 4, `e2e_agent_loop` 4,
  `e2e_generic_search` 12, `e2e_highlight_long_text` 8 — cette dernière ne
  compilait plus depuis `return_fields` (littéral `EntityConfig` en retard),
  réparée en passant.

## 4. Ce qu'on n'a pas fait

- Pas de ports pour les paramètres (`-->|config|`) : un paramètre est global
  au graphe et fixé avant l'exécution, un port transporte une valeur pendant.
  L'héritage par câblage donne la première classe sans changer la syntaxe.
- Un `$var` enfoui dans une chaîne (`"prefix_$var"`) n'est pas un câblage :
  il se substitue, mais n'hérite de rien.

## 5. Les quatre listes restantes, tranchées

Une liste close est exacte : montrée au modèle comme `enum`, refusée hors
liste à la création. Elle devient *le* contrat, donc chaque alias qu'un
parseur acceptait en plus devait entrer dans la liste ou mourir. Décision :
strict partout — la seule chose que les alias protégeaient (un humain qui
tape `Aggregated`) est ce que l'`enum` rend impossible.

| Paramètre | Liste | Ce qui a changé |
|---|---|---|
| `result_mode` | `aggregated \| detailed \| source_resolved` | les alias `Aggregated`/`Detailed`/`SourceResolved` du parseur, sans aucun appelant, supprimés |
| `mode` (BM25) | `contains \| contains_split \| regex \| parse \| symbol` | `symbol` manquait à la description |
| `strategy` | `rrf \| weighted` | annoté, rien d'autre |
| `format` | `markdown \| json` | l'alias `md` de `ToolFormat::parse` supprimé |
| `direction` | `Outgoing \| Incoming` | le `_ => Outgoing` du parseur, devenu inatteignable, remplacé par une erreur |

Tests : `enumerated_params_are_exact_lists_without_aliases`,
`tool_format_has_no_alias` ; `e2e_result_mode` (10) et les suites de la
section 3 repassées.
