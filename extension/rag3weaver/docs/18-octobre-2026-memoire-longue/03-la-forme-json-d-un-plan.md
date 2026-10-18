# La forme JSON d'un Plan, sur l'exemple de `cited_by`

*18 octobre 2026, mémoire longue, **fixée avec l'arbre principal** (suite de la page 02 ;
les neuf points du bas sont les siens, acceptés). Rien n'est codé : l'IR après son L4, le
`PlanNode` ensuite. Les remarques de l'arbre principal sont reprises : le
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
`value_dir = value + "/"` et `value_sym = value + "#"` (rien si `under` est faux), et ne
pose pas `since` quand il vaut 0 (un entier n'est jamais « vide », point 8). Les listes et
chaînes vides, il les laisse : le liage retire lui-même les `optional` dont un paramètre
est absent ou vide. Le `q` passe par `KBQuerySourceNode` → `BM25SearchNode` → `IntersectResultsNode`
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
      "limit":  {"param": "limit", "max": 200},
      "offset": {"param": "offset"}
    }},
    "total": {"count": {"matching": {"table": "Memory", "filter": {"let": "fiche_gardee"}}}}
  },
  "outputs": {"rows": "fiches", "total": "total", "results": {"form": "fiches", "uuid": "_uuid", "entity": "Memory"}}
}
```

## Les choix, fixés avec l'arbre principal

1. **Les noms** : `snake_case`, étiquetés par une clé, comme `FilterCondition` ;
   `direction` en `outgoing` | `incoming`. **Pas de couche de transport** : les enums de
   `rag3weaver_ir` gagnent eux-mêmes `#[serde(rename_all = "snake_case")]`, le JSON est
   l'IR un pour un. **`Predicate::Compiled` ne se désérialise jamais** (`#[serde(skip)]`) :
   sinon un Plan déclaré porterait du Cypher brut, la règle de Lucie à l'envers.
2. **`let`** : des prédicats nommés, pour les prédicats seulement (pas les formes),
   référencés par `{"let": "nom"}` ; résolus au liage ; un nom inconnu ou cyclique refuse
   le Plan en nommant la chaîne.
3. **`returns`** : une chaîne nue vaut `Column::Node(nom)` ; `{"edge": "champ"}` pour une
   propriété de relation (`Column::Edge`) ; `collect` est un objet ; `_uuid` permis.
4. **`limit`** : le plafond s'écrit **dans le Plan**, `{"param": "limit", "max": 200}`, et
   se lit avec la forme ; le liage refuse au-delà en le nommant ; un `limit` lié sans `max`
   est refusé par `validate()`. `offset` lié ou littéral.
5. **`order_by`** : une liste d'`{field}` ou d'`{coalesce: [...]}`, chacune avec `desc`
   (faux par défaut). **`validate()` exige** qu'un Plan qui a `limit` ou `offset` finisse
   son `order_by` par `_uuid` (ou une clé unique déclarée) : une pagination qui saute ou
   répète une ligne est refusée, pas permise.
6. **`outputs`** : le nœud sait à quel port va quoi ; `results` dit quelle colonne est
   l'uuid et quelle entité, pour `RenderResultsNode` et `IntersectResultsNode`.
7. **`plans/`** : un fichier du gabarit, cité par `@plans/…`, chemin relatif au manifeste.
   Conséquence pour l'hôte : l'empreinte du manifeste (`manifest_print`, qui fait relire un
   backend modifié) couvre aussi les fichiers de plans cités — sinon un Plan modifié ne se
   relit pas (posé avec la pièce générique).
8. **Absent et vide** :
   - sous `optional`, un paramètre **absent**, une **liste vide** ou une **chaîne vide**
     retirent le prédicat ;
   - hors `optional`, une liste vide donnée à `in` est **refusée** en nommant le paramètre
     (`in []` rendrait zéro ligne sans un mot) ;
   - un entier n'est jamais « vide » : `since: 0` reste un filtre ;
   - un `all_of` ou un `any_of` vidé par ces retraits **disparaît**, il ne vaut pas faux ;
   - un paramètre nommé que rien ne donne, hors `optional`, est refusé en le nommant.
9. **La validation contre le schéma**, au liage, contre la vue de lecture : `through` et
   `collect` vérifient que la relation existe, que `end` est son bout dans la `direction`
   dite, et que les champs nommés existent sur les tables visées ; un refus nomme la
   relation ou le champ.

## Le rendu (arbre principal)

- `starts_with` : `STARTS WITH` dans le dialecte rag3db ; `left(col, length($p)) = $p` dans
  le dialecte PostgreSQL — jamais `LIKE`, où un `%` ou un `_` d'un chemin serait un joker.
- `through` : `EXISTS { MATCH … }` côté rag3db, `EXISTS (SELECT 1 …)` côté SQL.
- `collect` : `OPTIONAL MATCH` + `collect` côté rag3db, sous-requête corrélée (`json_agg`)
  côté SQL.
- Chaque ajout a un témoin de rendu dans les deux dialectes ; `e2e_postgres` le rejoue. Ce
  qu'un dialecte ne sait pas rendre est refusé en le nommant.
- Le `PlanNode` lit par `read_for_tool` / `ReadCatalog::run_read_plan`, un instantané par
  appel avec L6 : le total et les lignes d'un même Plan sont lus dans le même instantané.
