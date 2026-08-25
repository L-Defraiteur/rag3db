# Doc 50 — Le chemin local : formats d'appel d'outils, schémas, et ce qu'on écrit

Repérage du 25 août, tout vérifié avec preuve de licence, une partie **exécutée**
et non lue. Complète le [doc 47](47-llm-tts-stt-sur-burn-reperage-et-etape-1.md)
(le chemin cloud est livré) et prépare le chemin **local**, où c'est nous le
serveur : on rend le chat template et on parse la sortie.

## 1. Pourquoi ce document existe

Sur le cloud, le fournisseur garantit la forme des appels d'outils. **En local,
personne.** Et beaucoup de petits modèles n'ont **aucun bloc `tools`** dans leur
template : SmolLM2-360M, Phi-4 (14B), Gemma 2/3/3n, EuroMoE, Baguettotron,
Ouro-1.4B, Motif-2.6B, ERNIE-4.5-0.3B.

**Fait marquant : `llama.cpp` a supprimé son handler « Generic »**, celui qui
donnait des outils à un template qui n'en a pas. C'est exactement notre cas, et
la référence principale ne le couvre plus. Son `chat.cpp` est aussi passé de
~25 formats par famille à un **autoparseur différentiel** (5 400 lignes : il rend
le template avec des sondes, diffe les sorties, en déduit les délimiteurs,
compile un parseur PEG) plus 14 familles à la main. **On ne refait pas ça.**

## 2. Les trois pièges mesurés (pas lus)

Rendus réellement, avec deux moteurs indépendants :

1. **Granite 3.3, SmolLM3 et Phi-4-mini perdent silencieusement
   `message.tool_calls`.** Le tour d'assistant se rend **vide**. L'historique
   n'est **pas rejouable** par la clé standard : l'appel doit être réécrit dans
   `content`. Aucune erreur, aucun avertissement.
2. **Granite 3.3 : fournir son propre message système supprime l'instruction
   d'appel d'outil.** Le template ne la concatène que dans la branche où il
   fabrique lui-même le système. Les outils restent listés, mais le modèle
   n'est plus jamais dit quoi en faire.
3. **Mistral v0.3 refuse nos identifiants.** Son template lève
   `raise_exception("Tool call IDs should be alphanumeric strings with length 9!")`.
   Notre `ToolCall::local_id` produit `call_local_<16 hex>` → **rejeté**. Et
   Command R7B **réécrit** les identifiants en compteurs `"0"`, `"1"`. Il faut
   une dérivation d'identifiant **par famille** (~20 lignes).

## 3. Les formats, par famille

Ce qu'il faut savoir pour parser. Les lignes ✅ ont été **rendues** ; les autres
viennent du relevé des templates officiels.

| Famille | Balises | Forme | ∥ | Fin de tour | Résultat réinjecté |
|---|---|---|---|---|---|
| **Llama 3.1/3.3** | *aucune* (JSON nu) ; `<\|python_tag\|>` pour les intégrés | `{"name":…,"parameters":…}` — **`parameters`**, jamais `arguments` | **non** | `<\|eot_id\|>`, `<\|eom_id\|>` si `builtin_tools` | rôle **`ipython`** |
| **Llama 4** | `[` → `]` | **pythonic** : `[get_weather(location="…")]` | oui | **`<\|eot\|>`** (≠ `eot_id`) | `ipython` |
| **Qwen2.5 / Qwen3** ✅ | `<tool_call>` → `</tool_call>` | `{"name":…,"arguments":…}` | oui | `<\|im_end\|>` | **rôle `user`** + `<tool_response>`, tours groupés |
| **Qwen3-Coder / Seed-OSS** ✅ | `<tool_call><function=N>` | **XML, zéro JSON** : `<parameter=k>v</parameter>` | oui | `<\|im_end\|>` | `user` |
| **Hermes 2/3/4** ✅ | `<tool_call>` → `</tool_call>` | `{"name":…,"arguments":…}` | oui | `<\|im_end\|>` | **rôle `tool`** (l'exception) |
| **Mistral v0.3 / Nemo** ✅ | `[TOOL_CALLS] [` (v0.3, **avec** espace) / `[TOOL_CALLS][` (Nemo) | tableau JSON | oui | `</s>` | `[TOOL_RESULTS]…[/TOOL_RESULTS]` |
| **Mistral v11 / v13** | `[TOOL_CALLS]` | **pas de JSON englobant** : `N[CALL_ID]id[ARGS]{…}`, v13 sans `CALL_ID` | oui | `</s>` | idem |
| **GLM-4.5 / 4.6** ✅ | `<tool_call>N` → `</tool_call>` | XML par clé : `<arg_key>`/`<arg_value>` | oui | `<\|assistant\|>` | **`<\|observation\|>`** |
| **DeepSeek-R1 / V3.1** | `<｜tool▁calls▁begin｜>…` (U+FF5C, U+2581) | R1 avec fence ```` ```json ````, V3.1 sans | oui | `<｜end▁of▁sentence｜>` | `<｜tool▁outputs▁begin｜>` |
| **Granite 3.3** ✅ | `<\|tool_call\|>`, pas de fermeture | **liste JSON** | oui | `<\|end_of_text\|>` | rôle libre |
| **Granite 4.0** | `<tool_call>` → `</tool_call>` — **rupture totale vs 3.3** | style Qwen | oui | `<\|end_of_text\|>` | `tool` → `user` |
| **Command-R / R+** | `Action:` + fence json | `[{"tool_name":…,"parameters":…}]` — **`tool_name`** | oui | `<\|END_OF_TURN_TOKEN\|>` | template séparé |
| **Command R7B** | `<\|START_ACTION\|>[` → `]<\|END_ACTION\|>` | `{"tool_call_id":"0",…}` | oui | idem | `<\|START_TOOL_RESULT\|>` |
| **gpt-oss (Harmony)** | `<\|start\|>assistant to=functions.N<\|channel\|>commentary json<\|message\|>` → **`<\|call\|>`** | arguments bruts | **non** — un seul appel par message | `<\|end\|>` / `<\|return\|>` | `<\|start\|>functions.N to=assistant` |
| **Phi-4-mini** ✅ | ` functools[` → `]` | liste JSON | oui | `<\|end\|>` | `<\|tool_response\|>` (jamais émis par le Jinja) |
| **SmolLM3** ✅ | `<tool_call>` → `</tool_call>` | `{"name":…,"arguments":…}` | — | `<\|im_end\|>` | **rôle `user` brut**, sans balise |
| **Gemma 2/3/3n, Phi-4 14B, SmolLM2-360M** ✅ | — | **aucun support d'outils** | — | — | — |

Pièges de parsing relevés : `<\|constrain\|>` de gpt-oss **n'est pas** dans le
Jinja livré ; DeepSeek utilise U+FF5C et U+2581, donc un `grep '<|tool'` ASCII ne
matche jamais ; **GLM-4.5 insère les chaînes brutes** dans `<arg_value>`, donc
une valeur contenant `</arg_value>` casse tout ; le template Mistral v0.3 produit
du **JSON invalide** dans `[TOOL_RESULTS]` (pas de `|tojson`) ; SmolLM3 émet
`"additionalProperties": False` — le `False` de Python, pas `false`.

## 4. Le rendu des templates : `hf-chat-template`

Mesuré sur **9 templates réels** : `minijinja` 2.24 + `pycompat` en rend **6 sur
9** (échoue sur GLM-4.5 — `tojson(ensure_ascii=False)`, Mistral v0.3 —
`raise_exception`, SmolLM3 — `{% generation %}`). **`hf-chat-template` 1.0.0**
(MIT + Apache, `LICENSE-MIT` et `LICENSE-APACHE` lus) en rend **9 sur 9** :
`tojson` compatible Python, `raise_exception`, `strftime_now`, `{% generation %}`
neutralisé, et il active `serde_json/preserve_order` **parce que l'ordre des clés
est signifiant à travers `| tojson`**.

**16 crates** sans features, 20 avec `pycompat`+`strftime`, **compile en
wasm32**. Réserve honnête : **604 téléchargements, un mainteneur** — c'est un
pari sur une personne. Le repli est minijinja nu plus quatre polyfills (~80
lignes), identifiés.

## 5. `schemars` : le harnais de schéma

`schemars` 1.2.2, **MIT prouvée**, **16 crates**, wasm OK, draft 2020-12.
**N'activer aucune feature optionnelle** — `url2` seul fait passer l'arbre à 53
crates (toute la famille ICU/idna). Utiliser `contract = Deserialize` : avec
`Serialize`, les `Option` finiraient dans `required`.

**Chez `llguidance` : ça passe tel quel** — vérifié par exécution,
`warnings=[]`. Les `format` numériques que schemars émet (`double`, `uint32`)
sont **ignorés en silence**, ce qui le sauve. Trois pièges : `uniqueItems`
(émis pour `HashSet`) → erreur ; `oneOf` sur branches non prouvablement
disjointes → erreur ; `format` de chaîne hors des dix implémentées → erreur.
Règle contre-intuitive : un mot-clé **connu du draft mais non implémenté** fait
échouer, un mot-clé **inconnu** est ignoré. Filet : `"x-guidance": {"lenient": true}`.

**Chez OpenAI en mode strict : neuf transformations**, ~45 lignes d'`impl
Transform`. `oneOf`→`anyOf` ; `additionalProperties: false` sur **chaque** objet ;
`required` = **toutes** les clés ; purger `default`, `uniqueItems`, `examples`,
`allOf`, `not`, `if/then/else`, `min/maxLength` ; filtrer `format` sur liste
blanche (aucun format numérique n'est accepté) ; réduire un `$ref` à clé sœur ;
retirer `$schema` ; rejeter une racine non-objet.

**Et une règle qui n'est pas une réparation mais un refus** : `HashMap<String,V>`
produit `additionalProperties: {sous-schéma}` ; appliquer naïvement la règle
`additionalProperties: false` donne **un objet qui ne peut être que vide** — le
modèle renverra `{}` et les données seront perdues, **en silence**. Erreur
explicite au build, remplacer par `Vec<{key,value}>`. Idem les tuples.

Bonne nouvelle : **`$ref`, `$defs` et la récursion `{"$ref":"#"}` sont
explicitement supportés par OpenAI en strict.** Pas besoin d'inliner.

**Aucune alternative** : `utoipa` préfixe ses `$ref` en dur et n'a pas
d'équivalent du trait `Transform` ; `okapi` est deux majeures en retard ;
`apistos` fait 159 crates et ne compile pas en wasm.

⚠️ **`llguidance` déclare `serde_json/preserve_order`** : par unification cargo,
**tout le workspace** voit `serde_json::Map` devenir un `IndexMap`. Ici c'est
plutôt bienvenu, mais c'est un changement silencieux à connaître.

## 6. La récursion est le point de rupture de la portabilité

| | schémas récursifs |
|---|---|
| OpenAI (strict) | ✅ y compris `{"$ref":"#"}` |
| Gemini natif | ✅ |
| llguidance | ✅ (Earley, natif) |
| **Vertex, couche compatible** | ❌ « fully recursive schemas are not supported » |
| **Anthropic** | ❌ catégoriquement |

> **Un schéma destiné à être portable doit être non récursif.**

Corollaire : **Anthropic a bien des sorties structurées** (GA, champ
`output_config.format`) — la prémisse inverse, que j'avais retenue, est fausse.

## 7. Ce qu'on écrit nous-mêmes — chiffré

Ce qui se **prend** : `hf-chat-template` (rendu), `schemars` (dérivation),
`llguidance` (contrainte). **34 crates transitives au total, aucun tokio, aucun
reqwest, aucun `getrandom`, wasm OK.**

Ce qui se **recopie comme donnée** : la table des formats de `tool-parser`
(Apache-2.0) et la taxonomie de `chat-auto-parser.h` de llama.cpp (MIT) —
`tool_format {JSON_NATIVE, TAG_WITH_JSON, TAG_WITH_TAGGED}`,
`call_id_position`, `content_mode`… La meilleure classification existante.

| À écrire | Lignes |
|---|---|
| Rendu + **double** détection de capacité (variables non déclarées **et** rendu-sonde à sentinelle, seul moyen d'attraper Granite/SmolLM3/Phi-4-mini) | 60-90 |
| Injection des outils pour un template qui n'en a pas (ce que llama.cpp a supprimé, ce que TGI garde) | 80-120 |
| Parsing par famille : registre **ordonné** + 6 à 8 parseurs | 250-400 |
| Streaming : décider de mettre en tampon — **on a déjà la moitié** (`holdback`/`first_stop`) | 80-120 |
| Contrainte grammaticale (`TopLevelGrammar::from_json_schema`) | 40-80 |
| Transform schemars → strict (le **vérificateur** existe déjà) | ~45 |
| **Réparer nos propres schémas d'outils** (§8) | ~30 |
| Identifiants par famille | ~20 |
| Refus des marqueurs réservés dans une description d'outil | ~40 |
| Boucle d'agent + re-ask | 160-180 |
| **Total** | **≈ 800-1 100** |

À comparer : **564 lignes** chez TGI pour le même périmètre sans le parsing par
famille, **3 908** pour `chat.cpp`, **5 400** pour l'autoparseur qu'on ne refait
pas.

**Le patron d'architecture vient de `mistral.rs`** (MIT) : un trait
`ToolFormatParser { could_be_tool_call, parse, tool_call_grammar }` et un
**registre dont l'ordre compte** — Gemma4 avant Qwen, parce que `<|tool_call>`
contient `<tool_call>`. Inutilisable comme dépendance (candle + tokio + pyo3,
~100 crates), excellent comme modèle.

## 8. Une dette à nous, révélée par le mode strict

`params_object_schema` (`tools.rs`) **ne passe pas le mode strict** : il omet
`required` quand il est vide, n'y met **pas** les paramètres optionnels (strict
les exige, typés `["T","null"]`), émet `default` (refusé), et
`ConfigParamType::Json` produit `{"type":"object"}` nu — refusé, et *justement*
le cas où une grammaire ne borne plus rien. **15 de nos 28 nœuds échouent.** Sans
effet sur les appels d'outils ; empêche de réutiliser un `ToolDef.parameters`
comme schéma de sortie structurée.

## 9. Le réessai change de nature (il ne disparaît pas)

Cinq bibliothèques étudiées (`instructor`, `instructors`, `rstructor`,
`pydantic-ai`, `instructor-ai`). **Aucune ne distingue l'échec syntaxique du
sémantique**, et **aucune ne détecte une erreur qui se répète à l'identique** —
leur seul frein est un compteur.

- **La moitié syntaxique est morte** sous décodage contraint. Preuve la plus
  propre : **`tysm`** (MIT, 40 000 téléchargements) fait schemars + strict et n'a
  **aucune ligne de réessai**. Personne ne s'en plaint.
- **La moitié sémantique reste vivante** : une grammaire garantit qu'une date a
  le bon format, pas qu'elle est antérieure à aujourd'hui ; qu'un total est un
  nombre, pas qu'il égale la somme des lignes. Aucun décodage ne dit une
  contrainte inter-champs ou référentielle.
- **Trois résidus syntaxiques** : les endpoints « compatibles » qui acceptent
  `response_format` et l'ignorent, les troncatures (`Finish::MaxTokens` — sortie
  valide *jusqu'ici* mais incomplète), et les refus.

**Et le prompt de réessai de ces cinq bibliothèques serait contre-productif chez
nous** : « ensure your response is valid JSON matching the schema exactly », dit
à un modèle dont le JSON est parfait par construction, l'oriente vers la forme
alors que le problème est le fond.

**Ce qu'on prend** : le squelette de `rstructor` — `assistant(sortie fautive)`
puis `user(diagnostic)`, **historique conservé**, budget partagé, dispositions
typées (40-60 lignes) — et son `decode.rs` (~105 lignes, `serde_path_to_error`),
**la seule idée de tout l'écosystème** qui produise un message qu'un modèle peut
exploiter : `$.orders[2].total: invalid type: string, expected f64`. Plus les
transformations schemars→dialecte de `instructors` (~200 lignes MIT).
**Ce que personne n'a** : deux gabarits distincts syntaxique/sémantique
(~15 lignes) et l'abandon sur erreur répétée (3 lignes).

**Et la moitié « outil » est déjà chez nous** : `GraphToolRegistry::call` rend
**toujours** un `Turn::tool_result`, succès ou échec, avec des messages écrits
pour être lus par un agent. Une panique de nœud elle-même est rattrapée.

## 10. Verdicts

| | |
|---|---|
| `hf-chat-template` | **prendre** — 9/9 templates, 16 crates, wasm ; repli identifié |
| `schemars` 1.2.2 | **prendre**, sans aucune feature optionnelle |
| `llguidance` 1.8 | **prendre** (déjà décidé) — note : copyright **Microsoft**, pas guidance-ai |
| `serde_path_to_error` | **prendre** — la seule dépendance nouvelle du re-ask |
| llama.cpp, TGI, mistral.rs, `tool-parser` | **spécifications et données**, pas dépendances |
| `instructor-ai` | **ignorer** — mort depuis 25 mois, un bug qui écrase l'erreur à chaque tour |
| `instructors`, `rstructor` | **ignorer comme dépendances** (115 et 95 crates, tokio, wasm cassé), **reprendre deux fichiers** |
| `genai`, `swiftide`, `async-openai`, `kalosm` | rien — tool calling natif HTTP seulement, zéro parsing par famille |

## 11. Ce qui n'a pas pu être vérifié

Aucun appel réseau vers un fournisseur : tout le comportement d'OpenAI, Google
et Anthropic vient de leur documentation. Le sens exact de « fully recursive » de
Vertex n'est écrit nulle part. Le statut de `minLength`/`maxLength` en strict est
ambigu. Les limites chiffrées d'Anthropic relayées ailleurs ne figurent pas sur
la page lue. Le transform strict n'a jamais été soumis à l'API. Le comportement
réel des endpoints « compatibles » (llama.cpp server, Ollama, vLLM, Groq) face à
`strict` n'a pas été testé — c'est pourtant lui qui décide si le résidu
syntaxique du réessai est marginal ou fréquent chez nous. Plusieurs templates
(Llama, Gemma, Cohere) sont passés par des miroirs, byte-identiques entre eux
mais **pas contre l'original gated**.
