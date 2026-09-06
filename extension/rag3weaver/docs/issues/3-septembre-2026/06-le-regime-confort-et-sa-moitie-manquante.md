# Issue 06 — le régime `confort` et sa moitié manquante

> **Close le 6 septembre 2026.** Les trois points sont faits, et vérifiés un par
> un avant d'écrire cette ligne :
>
> | | où c'est fait | ce qui le tient |
> |---|---|---|
> | 1 · la fabrique unique de `Llm`, et les cinq copies remplacées | `regime::modele_agentique` / `modele_agentique_nomme` (`regime.rs:196`, `:203`) — appelée par les cinq suites, et par elles seules | `une_intention_explicite_reprend_la_main`, `sous_confort_une_variable_qui_traine_ne_reprend_pas_la_carte`, `sous_plein_la_variable_decide_comme_avant` |
> | 2 · les trois rôles sur la carte libre sous `confort` | `burn_device.rs:138-152` — `carte_locale()` en dernier recours, `None` sous `plein` | `la_carte_ne_distingue_plus_les_roles` |
> | 3 · un test des **quatre** promesses | — | `confort_tient_ses_quatre_promesses` (`regime.rs:386`) |
>
> La question de précédence que ce document laissait ouverte — « à confirmer
> avec Lucie, pas à trancher seul » — a été tranchée dans le sens proposé :
> `RAG3WEAVER_LLM=local` dit une intention **pour cette passe** et gagne ;
> `RAG3WEAVER_LOCAL_LLM`, qui traîne dans un profil, ne reprend pas la carte
> sous `confort`. C'est écrit en tête de `regime.rs` et tenu par trois tests.
>
> **Rouvert et refermé le 6 septembre 2026, 15h30.** Les promesses 1 et 2
> — rapport cyclique 60 %, rafales de 2 048 — étaient une lecture de travers
> des mots de Lucie : elle demandait *la deuxième carte*, pas une carte
> bridée. Le bridage n'a de sens que si l'embarqueur partage la carte du
> compositeur. Sur une carte libre il ne protège personne et a coûté quatorze
> minutes pour trente fichiers (suite cloud, deux chunks par appel GPU, une
> pause après chacun). Depuis : `confort` bride **seulement quand il n'y a
> qu'une carte** (`Regime::carte_partagee`, `duty_si`, `budget_caracteres_si`),
> tenu par `le_confort_ne_bride_que_la_carte_partagee`.
>
> Ce qui reste vrai et vaut d'être gardé : la section « ce qui marche déjà »
> ci-dessous documentait trois promesses sur quatre, et **rien ne disait que la
> quatrième manquait**. C'est ce silence-là qui a rendu l'issue nécessaire.

`src/regime.rs` cite « l'issue 06 » depuis le 30 août. **Elle n'avait jamais
été écrite** — une référence vers un document qui n'existe pas, ce qui est la
version documentaire du défaut qu'on passe la semaine à débusquer. La voici.

## Ce que Lucie demande, dans ses mots

> une env var générale qui décide mode confort pour jamais m'emmerder quand je
> veux pas être emmerdée (gemini + embedding et autres modèles locaux dans
> deuxième gpu)

Le besoin est **ergonomique avant d'être technique** : un seul mot à poser pour
que le poste reste utilisable, sans se rappeler quelles variables existent.

## Ce qui marche déjà

`RAG3WEAVER_REGIME=confort` existe et tient trois promesses sur quatre :

| | fait ? | où |
|---|---|---|
| carte de l'embarqueur = la moins chargée | **oui** | `burn_device.rs:143` consomme `Regime::carte_embedder()` |
| rapport cyclique 60 % | **oui** | `embedder.rs:734` |
| rafale 2 048 caractères | **oui** | `embedder.rs:804` |
| agentique vers un fournisseur distant | **non** | rien |

La précédence est déjà posée et il faut la garder : **le code l'emporte sur la
variable, qui l'emporte sur le régime, qui l'emporte sur le défaut.** Un régime
*fournit ce que personne n'a dit* ; il ne force jamais.

## Ce qu'il reste à faire

### 1. Le `Llm` sous `confort` — le vrai manque

`regime.rs` explique pourquoi ce n'est pas branché, et l'argument est bon :

> le choix du `Llm` se fait chez l'appelant, et déclarer ici une intention que
> personne ne lit produirait exactement le genre de mécanisme
> construit-et-jamais-appelé qu'on passe la semaine à débusquer.

Donc **ne pas ajouter une méthode que rien n'appelle**. Le geste est :

1. Une fabrique unique dans le crate — elle lit l'environnement et le régime, et
   rend un `OpenAiLlm` (ou `None` avec une raison nommée, jamais un silence).
2. **Remplacer les cinq copies existantes** par elle. Elles sont déjà écrites et
   déjà divergentes :
   `tests/e2e_cloud_code_agent.rs:131`, `tests/e2e_avis_du_modele.rs:40`,
   `tests/e2e_cloud_schema_probe.rs:16`, `tests/e2e_lecture_mermaid.rs:39`,
   `tests/e2e_conversation_a_plusieurs.rs:54`.
   C'est ce remplacement qui fait la différence entre une fabrique et une
   intention décorative.

**Attention à la précédence, elle est à l'envers aujourd'hui.** Le motif
dupliqué fait gagner `RAG3WEAVER_LOCAL_LLM` en premier, *toujours* :

```rust
if let Ok(base) = std::env::var("RAG3WEAVER_LOCAL_LLM") { return local; }
vertex()
```

Sous `confort`, le local est précisément ce qu'on ne veut pas. La règle du
module reste valable — une variable explicite gagne — mais il faut distinguer
« la variable est posée pour cette passe » de « elle traîne dans le profil ».
Proposition : sous `confort`, Vertex d'abord ; `RAG3WEAVER_LOCAL_LLM` ne
reprend la main que si l'appelant le demande dans le code, ou par une variable
dédiée (`RAG3WEAVER_LLM=local`) qui dit une intention pour *cette* passe.
**C'est un choix à confirmer avec Lucie, pas à trancher seul.**

Les identifiants Vertex sont dans `.vault` (`vertex-sa.json`, projet
`lr-hub-472010`) et `src/gcp_auth.rs` sait déjà en tirer un jeton sans réacteur.

### 2. Les autres modèles locaux sur la carte libre

Aujourd'hui, seul l'embarqueur suit le régime. `burn_device.rs:142` exclut le
reranker et l'OCR, avec une raison écrite :

> un reranker ou un OCR prennent la carte le temps d'un appel, l'embarqueur la
> garde.

Cette raison parle d'**efficacité**. `confort` parle de **ne pas être
dérangée**. Sous `confort`, le second l'emporte : les trois rôles devraient
aller sur la carte la moins chargée. À faire en gardant l'exclusion sous
`plein`.

### 3. Ce qu'il faut éprouver

Un test qui pose `RAG3WEAVER_REGIME=confort` et vérifie que **les quatre**
promesses sont tenues — carte, rapport cyclique, rafale, fournisseur — plutôt
que trois sur quatre en silence. Aujourd'hui rien ne dit qu'une promesse manque.

## Ce qu'il ne faut pas faire

- **Pas de méthode sur `Regime` sans appelant.** Le module l'a refusée une fois
  pour cette raison exacte ; l'ajouter sans brancher les cinq sites serait lui
  donner tort.
- **Pas de repli silencieux.** Si `confort` veut Vertex et que le jeton manque,
  ça doit se dire — pas retomber sur le local en silence, ce qui reprendrait la
  carte que le régime voulait libérer.

## Frontière

Ce travail vit dans `extension/rag3weaver/src/{regime,burn_device,llm,openai_llm}.rs`
et dans `tests/`. La session *Rag3weaver architecture backend et FTS* travaille
au même moment dans `src/search.rs`, `src/catalog.rs` et `tests/` sur un banc de
mesure du seuil de confiance — **se le dire avant de toucher `src/search.rs` ou
`src/catalog.rs`**, le reste est disjoint.
