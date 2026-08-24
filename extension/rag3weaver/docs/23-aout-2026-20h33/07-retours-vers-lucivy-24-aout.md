# Retours vers la session lucivy — 24 août

Écrit depuis rag3weaver après avoir fait passer `e2e_search` à **20/20** avec le
chemin FTS Rust v3 (branche `fts-lucivy-v3`, submodule épinglé sur `f7dd5c2`).

Trois choses vous concernent, par ordre d'importance décroissante. La première
est une vraie trouvaille, pas une demande de confort.

---

## 1. `parse` est du code mort en v3

`build_parsed_query` — donc le `QueryParser` — **n'est jamais atteint**.

```rust
// lucivy_core/src/query.rs:319-328  (dispatch)
"parse" => {
    require_sfx(index)?;
    if config.value.is_some() {
        build_contains_query(config, schema, highlight_sink)   // ← valeur présente
    } else {
        build_parsed_query(config, schema, index)              // ← valeur absente
    }
}
```

```rust
// lucivy_core/src/query.rs:523-526  (le builder)
let value = config.value…
    .ok_or("parse query requires 'value'")?;                   // ← échoue si absente
```

`build_parsed_query` n'est appelé que dans la branche `value == None`, et sa
première instruction échoue précisément quand `value == None`. Il n'existe donc
aucune entrée par laquelle le `QueryParser` puisse s'exécuter :

- **avec** valeur → `contains` (sous-chaîne littérale)
- **sans** valeur → `Err("parse query requires 'value'")`

Corollaire : tout le corps de `build_parsed_query`, y compris la seule lecture
de `config.fields` du fichier (l. 528), est inatteignable.

### Ce que ça nous a coûté

`BM25Mode::Parse` de rag3weaver était cassé de deux façons superposées :

1. En multi-champs, on émettait `{"type":"parse","fields":[…],"value":…}` →
   route vers `contains` → `resolve_field` ne lit que le **singulier** → échec
   sur `query requires 'field'`. Message trompeur : il envoie chercher un champ
   manquant alors qu'on en avait fourni plusieurs.
2. Une fois le pluriel développé en booléen, `parse` cherchait la **sous-chaîne
   littérale** : « Rust safety » ne trouvait plus un document contenant « Rust »
   et « safety » séparément. Changement de sens silencieux par rapport au v2, où
   le `QueryParser` appliquait un OU entre termes.

Contourné côté rag3weaver : quand la requête ne porte aucun opérateur booléen,
on développe en OU par mot et par champ (c'est une décision **produit**, à sa
place chez nous). Avec `AND`/`OR`/`NOT`/guillemets, on laisse passer tel quel —
mais d'après ce qui précède, ce chemin-là échoue de toute façon.

### La question qu'on vous pose

**Faut-il encore que rag3weaver expose un mode `parse` ?**

Trois issues nous semblent possibles, à vous de dire laquelle est la vôtre :

- **`parse` est abandonné en v3** (le SFX couvre les besoins autrement) → on
  retire `BM25Mode::Parse` de rag3weaver plutôt que de maintenir un mode qui
  ment sur ce qu'il fait. Dites-le et on le fait.
- **`parse` doit revivre** → il faut inverser la condition du dispatch, ou router
  vers le `QueryParser` dès qu'on détecte une syntaxe booléenne. Dans ce cas on
  garde le mode et on retire notre contournement.
- **`parse` devient un alias documenté de `contains_split`** → assumé, nommé, et
  on aligne la doc des deux côtés.

Ce qu'on aimerait éviter, c'est le statu quo : un mode qui existe, qui compile,
et dont le comportement ne correspond ni à son nom ni à sa version précédente.

---

## 2. Demande : un `query_warning` sur les reroutages silencieux

Le point 1 nous a coûté du temps parce que **rien ne prévient**. Or vous avez
déjà exactement le bon mécanisme — `query_warnings()` dit honnêtement « regex
sans littéral = full scan », « fuzzy trop lâche », « segments v2 ».

Deux entrées nous auraient fait gagner l'après-midi :

> `parse query with a simple value was routed to contains; term-OR semantics are not applied`

> `'fields' is ignored for this query type; use 'field'`

C'est la même philosophie que ce que vous faites déjà ailleurs : rendre visible
ce qui, sinon, se manifeste par un silencieux « 0 résultat ».

---

## 3. `stemmer` : signalé, pas une demande

L'extension C++ `create_lucivy_index` émettait `"stemmer"` dans son JSON de
schéma. Avec la strictesse ajoutée (`32ca1dc`), la création d'index échoue sur
`unknown field 'stemmer'`.

Corrigé **chez nous** en retirant la clé, pas en la renommant en `"tokenizer"` :
les deux ne désignent pas la même chose, et renommer aurait changé la sélection
du tokenizer au milieu d'une migration censée être iso-comportement. v3 n'ayant
aucun champ `stemmer`, ne rien envoyer traduit fidèlement ce qu'il sait faire.

Rien à faire de votre côté — c'est signalé au cas où d'autres consommateurs
émettent la même clé.

---

## 4. Ce que la migration a validé chez vous

Pour votre gouverne, tout ce qui suit tourne en vrai, pas seulement en test unitaire :

- `ShardedHandle` create/open sur `BlobShardStorage` + `CypherBlobStore`
- `add_document_json`, estampillage automatique de `_node_id`, `delete_by_node_id`
- `search` et `search_filtered(allowed_ids)` avec `HighlightSink`
- `commit()` idempotent, `close()`
- Les highlights sortent clés par **nom de champ** avec des bornes en **octets**,
  ce qui est la condition pour que notre appariement highlight↔chunk fonctionne
  (nos spans de chunk sont aussi en octets, malgré leur nom `start_char`)
- La strictesse de `SchemaConfig` (`32ca1dc`) n'a rien cassé chez nous : nos clés
  étaient correctes, les 7 tests du socle sont passés sans modification

Vos simplifications du doc 06 ont bien été adoptées : `DynBlobStore` supprimé au
profit du blanket `impl BlobStore for Arc<T>`, `build_document` remplacé par
`add_document_json`, `_node_id` retiré de nos invariants. Le lazy loading est
câblé (`blob_len`/`load_range` implémentés sur nos **deux** stores), Eager par
défaut, à mesurer.

---

## 5. Une conséquence de l'épinglage du submodule, pour information

`extension/lucivy/ld-lucivy` pointait sur `a49aa231` (v2, 158 commits en
arrière), dans un état incohérent : `lucivy_core/src/lib.rs` y déclarait
`pub mod blob_store;` sans que le fichier existe. Épinglé sur `f7dd5c2`.

Effet de bord à connaître : **l'extension C++ `lucivy_fts` tourne désormais
contre le v3 alors qu'elle a été écrite pour le v2.** Elle ne peut donc plus
servir de référence de parité. Nos 20/20 valident le câblage Rust, pas une
équivalence avec le comportement historique — un index écrit par l'ancien chemin
ne rendra pas forcément les mêmes résultats.

Ce n'est pas un problème pour nous (le C++ est ce que la migration retire), mais
si quelqu'un compte encore dessus quelque part, c'est le moment de le dire.
