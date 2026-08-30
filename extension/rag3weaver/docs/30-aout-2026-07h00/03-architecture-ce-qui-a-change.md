# Architecture — ce qui a changé depuis le 29 août

Complément à `docs/29-aout-2026-12h24/02-architecture.md`. Ne redit pas ce qui
n'a pas bougé : la recherche hybride, le dataflow, le catalogue, les embedders
burn, le client cloud sont tels que décrits là-bas.

## 1. Une couche nouvelle : les processus

C'est l'ajout structurel de la session. Le moteur avait des modèles, un
catalogue et une boucle ; il n'avait **aucune notion de processus**.

```
serveur.rs          lancer, sonder, retrouver, arrêter
  └── daemon/
       ├── mod.rs        Service, servir(), sonde, client, --exposer
       ├── embeddings.rs un modèle chargé une fois, servi à plusieurs
       └── db.rs         rag3daemon : la base derrière une adresse
```

**Le motif partagé** : un processus qui tient une ressource rare (une carte,
un verrou de fichier) et la sert. Le client se fait passer pour la chose
locale — `DaemonEmbedder` est un `Embedder`, `DaemonConnection` est un
`DbConnection` — donc rien en amont ne sait que la ressource est ailleurs.

**Trois états, pas deux.** `Etat::{Repond, Absent, Occupe}` — le troisième dit
« quelqu'un répond, ce n'est pas lui ». Sans lui, on tuerait le serveur d'un
autre ou on parlerait au mauvais.

**`/sante` est répondue par la plomberie**, pas par le démon : c'est la route
dont dépend la sonde, et elle doit répondre même quand la ressource est
occupée.

## 2. Une couche nouvelle : la porte des commandes

```
commande.rs          Commande, Faits, Verdict, Sentinelle, Garde, Autorisee
  └── s'appuie sur codeparsers::shell (réduction d'une ligne en argv)
  └── consommée par dataflow::run_nodes (run, run_bg, wait)
```

**Deux propriétés structurelles, pas disciplinaires :**

1. **On exécute par argv, jamais par un shell.** Sans quoi toute liste blanche
   est décorative.
2. **`executer` ne prend qu'une `Autorisee`**, dont le champ est privé et que
   seul `Garde::autoriser` produit. Il n'existe donc **aucun chemin** qui
   exécute ce qui a été refusé — pas même par erreur de programmation.

**Le verdict à quatre morceaux** — décision, portée, fondement, faits — parce
qu'une décision stockée sans ses raisons ne se rejoue pas. Même argument que
`meta.warnings`.

## 3. Une couche nouvelle : le régime

```
regime.rs    Regime::{Confort, Plein} — carte, rythme, rafale
```

Sous les variables d'environnement, au-dessus des défauts. **Le mode par défaut
de la porte des commandes est `auto`**, et non `standard` : un garde qui
demande toujours est un garde que personne n'active.

## 4. Ce qui a bougé dans l'existant

| module | changement |
|---|---|
| `embedder.rs` | accueille `budget_batches`, `embed_char_budget`, `souffler`, `gpu_duty`, hissés de `record_nodes` — ils ne servaient qu'à l'ingestion. Et `Embedder::distant()`, qui empêche les rythmes de se multiplier |
| `search.rs` | `BM25Mode::Auto`, résolu dans `build_bm25_query` — le seul point où un mode devient une requête lucivy |
| `agent.rs` | `tool_defs()` **relu à chaque tour** : les listes closes suivent le catalogue qui bouge |
| `render_nodes.rs` | `rendre<T: Serialize>` — le mécanisme de gabarit cesse d'être réservé à `search` ; `ResultsView` gagne `warnings` |
| `generic_search_nodes.rs` | `BM25SearchNode` gagne un port `meta` : ses avertissements avaient été collectés et jamais rendus |
| `mermaid.rs` | les deux orthographes d'arête, et les nœuds déclarés dans la ligne d'arête |
| `burn_device.rs` | `for_role` consulte le régime, et **dit d'où vient le choix** |
| `template.rs` | deux racines (projet d'abord, bibliothèque ensuite), `ecrire_entity`, `preparer_entity` |
| `rag3db_connection.rs` | `read_only()` — la seule forme de partage que le moteur offre nativement |
| `codeparsers` | `shell.rs` ; docstrings pour C, C++, C#, Go ; `extract_block_doc` |

## 5. La surface d'outils

**Cinq → onze.** `adopt`, `edit`, `grep`, `list`, `place`, `read`, `run`,
`run_bg`, `schema`, `search`, `wait`.

Trois choses à savoir :

- **`run_bg` est le premier usager de l'asynchrone.** Le mécanisme était
  complet et câblé depuis le 26 août — accusé immédiat, résultat par la boîte,
  `PauseKind::WaitingForRun`, interblocages détectés — et **pas une fiche ne le
  déclarait**.
- **`schema` est le premier à passer par `rendre`.** Douze affichages restent
  faits à la main.
- **Le nom vient de l'attachement, pas du gabarit** (`attach`) — un même
  gabarit peut être offert sous deux noms.

## 6. Le point d'architecture non résolu

**Un agent ne peut pas encore attacher un outil qu'il a écrit.**
`GraphToolBox` tient `&'a GraphToolRegistry` — immuable — alors que `attach`
demande `&mut self`.

La forme proposée, non implémentée : une **seconde couche** mutable
(`Arc<RwLock<GraphToolRegistry>>`) à côté du registre statique, fusionnée par
`tool_defs()`, **les fournis l'emportant** — un agent ne doit pas pouvoir
masquer `run` ou `edit`.

Deux lifetimes, deux couches : ce qu'on a donné à l'agent et ce qu'il a écrit
ne sont pas la même chose, et les mélanger empêcherait de jeter le second sans
toucher au premier.

## 7. Les invariants que la session a posés

Ils valent plus que les modules, parce qu'ils se transmettent :

1. **On n'exécute que ce qu'on a su réduire.** Ce qui n'entre pas dans la
   forme attendue est refusé **avec son nom**, jamais permis par défaut.
2. **Celui qui touche la carte souffle.** Un intermédiaire qui espace déjà ses
   rafales ne doit pas être espacé une seconde fois.
3. **Un intermédiaire ne blanchit pas ce qu'il transporte.** `is_mock()`
   traverse le démon ; sinon le garde-fou du catalogue tombait en silence.
4. **Ce qu'on montre dit ce qu'il ne montre pas.** Un aperçu tronqué nomme le
   journal ; un résultat vide nomme l'avertissement ; un refus nomme ses
   voisins.
5. **Un diagnostic qui nomme la mauvaise source est pire qu'absent.** Quatre
   corrigés dans la session, dont un que j'avais créé.
6. **Mesurer plutôt que supposer.** Les deux processus sur une base, les scores
   BM25, la lecture Mermaid : chaque affirmation de cette session a un test qui
   la porte.
