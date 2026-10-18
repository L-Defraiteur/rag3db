# La forme JSON d'un Plan, sur l'exemple de `cited_by` — brouillon

*18 octobre 2026, mémoire longue, à fixer avec l'arbre principal avant tout code (suite de
la page 02). Rien n'est codé. Les remarques de l'arbre principal sont reprises : le
`PlanNode` lit par `read_for_tool` / `ReadCatalog::run_read_plan` (un instantané par
appel avec L6, pour que le total et les lignes concordent) ; chaque ajout se rend dans
les deux dialectes ou se refuse en se nommant ; `optional` se résout **au liage**, avant
`validate()`, si bien qu'un Plan validé n'a plus d'option.*

## Le graphe de l'outil

```text
%% tool: cited_by
%% param: value string! -- un chemin, un dossier ou un symbole, dans sa forme normale (<dépôt>:<chemin>[#<parent>::<nom>])
%% param: under bool = true -- aussi ce qui est dessous (le dossier, le fichier et ses symboles)
%% param: genres json = [] -- les genres de Ref gardés ; vide : tous
%% param: states json = [] / kinds json = [] / since int = 0 / subject string = ""
%% param: q string = "" / limit int = 20 / offset int = 0
%% param: format string = "markdown" -- markdown | json
params["RhaiNode(script=…)"]    -- les paramètres d'outil → les paramètres du Plan (value_dir, value_sym, et l'absent pour le vide)
plan["PlanNode(plan=@plans/cited_by.json)"]
render["RenderResultsNode(format=$format)"]
params --> plan --> render
```

Le `RhaiNode` (JSON vers JSON, sans lecture) fait ce qu'un graphe ne sait pas faire :
`value_dir = value + "/"`, `value_sym = value + "#"`, et **ne pose pas** un paramètre
vide ou à zéro (`genres: []`, `since: 0`, `subject: ""`, `under: false` → pas de
`value_dir` ni de `value_sym`). Le liage retire alors les `optional` dont un paramètre
manque. Le `q` passe par `KBQuerySourceNode` → `BM25SearchNode` → `IntersectResultsNode`
borné aux `results` du Plan.

## Le Plan (`templates/backends/memory/plans/cited_by.json`)

```json
{
  "let": {
    "ref_qui_repond": {"all_of": [
      {"optional": {"in": {"field": "genre", "param": "genres"}}},
      {"any_of": [
        {"equals":      {"field": "valeur", "param": "value"}},
        {"optional": {"starts_with": {"field": "valeur", "param": "value_dir"}}},
        {"optional": {"starts_with": {"field": "valeur", "param": "value_sym"}}}
      ]}
    ]},
    "fiche_gardee": {"all_of": [
      {"through": {"relation": "CITE", "direction": "outgoing", "end": "Ref",
                   "filter": {"let": "ref_qui_repond"}}},
      {"optional": {"in":       {"field": "state",      "param": "states"}}},
      {"optional": {"in":       {"field": "kind",       "param": "kinds"}}},
      {"optional": {"at_least": {"field": "updated_at", "param": "since"}}},
      {"optional": {"through": {"relation": "ANCHORED_TO", "direction": "outgoing", "end": "Subject",
                                "filter": {"equals": {"field": "name", "param": "subject"}}}}}
    ]}
  },
  "forms": {
    "fiches": {"select": {
      "table": "Memory",
      "filter": {"let": "fiche_gardee"},
      "returns": [
        "_uuid", "claim", "kind", "state", "origin", "reach", "created_at", "updated_at", "reviewed_at",
        {"collect": {"as": "refs", "relation": "CITE", "direction": "outgoing", "end": "Ref",
                     "fields": ["genre", "valeur", "etat"], "filter": {"let": "ref_qui_repond"}}},
        {"collect": {"as": "subjects", "relation": "ANCHORED_TO", "direction": "outgoing", "end": "Subject",
                     "fields": ["name"]}}
      ],
      "order_by": [{"coalesce": ["updated_at", "created_at"], "desc": true}, {"field": "_uuid"}],
      "limit":  {"param": "limit"},
      "offset": {"param": "offset"}
    }},
    "total": {"count": {"matching": {"table": "Memory", "filter": {"let": "fiche_gardee"}}}}
  },
  "outputs": {"rows": "fiches", "total": "total", "results": {"form": "fiches", "uuid": "_uuid", "entity": "Memory"}}
}
```

## Les choix à fixer ensemble

1. **Les noms** : les variantes en `snake_case`, étiquetées par une clé (`{"select": …}`,
   `{"in": …}`), comme `FilterCondition` ; `direction` en `outgoing` | `incoming`.
2. **`let`** : des prédicats nommés, référencés par `{"let": "nom"}` — la même condition
   sert le `through`, le `collect` (« les Refs qui ont répondu ») et le total, et ne
   s'écrit qu'une fois. Résolus au liage, comme `optional` ; un `let` inconnu ou cyclique
   refuse le Plan.
3. **`returns`** : un nom de champ nu vaut `Column::Node(nom)` ; `collect` est un objet.
   `_uuid` est permis (c'est l'identité rendue à l'explorateur).
4. **`limit` / `offset`** liés à un paramètre (`{"param": …}`) ou littéraux ; un `limit`
   lié a un plafond déclaré par le nœud (200 ?), au-delà duquel le liage refuse.
5. **`order_by`** : une liste d'`{field}` ou d'`{coalesce: [...]}`, chacune avec `desc`
   (faux par défaut) ; un dernier `_uuid` rend l'ordre total (la pagination ne saute ni ne
   répète une fiche entre deux pages).
6. **`outputs`** : le nœud sait à quel port va quoi ; `results` dit quelle colonne est
   l'uuid et quelle entité, pour `RenderResultsNode` et `IntersectResultsNode`.
7. **Où vit le Plan** : un fichier du gabarit (`plans/`), cité par `@plans/…` dans la
   config, plutôt qu'inline dans le `.mmd` (un JSON long dans une valeur mermaid ne porte
   pas ses retours à la ligne — retour de la tentative 1 des fiches).
8. **Les paramètres** : liés par nom depuis un port `params` (l'objet du `RhaiNode`) ou la
   config ; un paramètre que le Plan nomme et que rien ne donne est **absent** — retiré
   s'il est sous `optional`, refus sinon, en nommant le paramètre.
