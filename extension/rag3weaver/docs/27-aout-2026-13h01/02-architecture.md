# 02 — L'architecture, en une lecture

27 août 2026. Ce qu'il faut avoir en tête avant de toucher au code.

## 1. Trois couches, et rien ne les saute

```
   fiches (.mmd)          ce que le modèle voit et appelle
        ↓                 %% tool: %% param: %% choices: %% on: %% async:
   dataflow (graphes)     nœuds, ports, services, runs
        ↓
   catalogue              entités, relations, index, recherche
```

**La règle** : un outil est une fiche, une fiche est un graphe, un graphe
s'exécute sur des nœuds. Un outil qui appellerait le catalogue directement
court-circuiterait la trace, les runs et la politique — donc personne ne le
fait.

## 2. Les objets qu'on ne confond plus

Rangés par **vitesse de changement** — c'est le test qui a révélé qu'on en
avait confondu quatre sous le mot « racine » :

| Axe | Question | Change |
|---|---|---|
| **Org** | à qui ça appartient ? | jamais |
| **Cellule** (`Scope`) | dans quel index ? | à la création d'un projet |
| **Origine** | quel est ton nom ? | jamais — c'est le point |
| **Domaine** (`WorkDomain`) | qu'est-ce que je regarde ? | à chaque tâche |
| **Lentille** (`PathLens`) | comment je te l'écris ? | à chaque tour |

> Deux choses qui changent à des rythmes différents ne sont pas la même
> chose.

## 3. Les fichiers, par question

| Question | Fichier |
|---|---|
| Comment un outil est déclaré | `dataflow/graph_tool.rs` |
| Comment un graphe s'exécute | `dataflow/runtime.rs` (trois phases, parallèle par niveau) |
| Ce qui circule entre nœuds | `dataflow/port.rs` (`PortValue`, `QueryPayload`) |
| Les services partagés | `dataflow/services.rs` (`ServiceRegistry::layered`) |
| Recherche par signal | `dataflow/generic_search_nodes.rs` |
| Recherche complète | `catalog.rs::search`, `search.rs` |
| Rendu pour le modèle | `dataflow/render_nodes.rs` (`PathLens`, compact) |
| Trace, runs, messages, conversations | `dataflow/trace_nodes.rs` |
| Bus, sujets, boîtes aux lettres | `events.rs` |
| Réaction à des événements | `dataflow/reactor.rs` (`%% on:`, `%% policy:`) |
| Boucle d'agent | `agent.rs` (`ToolBox`, `PauseKind`, asynchrone) |
| Postures et blocages | `postures.rs` |
| Ce qu'on garde d'un tour à l'autre | `session.rs` (`Absorb`, renvois, `recall`) |
| Identité d'un fichier | `origin.rs` |
| Vision d'un agent | `work_domain.rs` |
| Code → graphe | `code.rs`, `code_tools.rs`, `codeparsers/` |
| Multi-locataire | `scope.rs` |
| Modèles burn | `burn_*.rs`, `burn_device.rs` |

## 4. Le catalogue, et son piège

**Où vivent les index** — c'est ce qui surprend tout le monde :

| Signal | Table |
|---|---|
| **plein texte** | la table **parente** |
| **vecteur** | la table de **chunks** |
| **sparse** | la table de **chunks** |

D'où : une entité sans chunk (`chunked: Some(false)`, comme `Symbol`) reste
cherchable en BM25 mais invisible au vecteur — et la configuration **refuse**
la combinaison plutôt que de la laisser arriver en silence.

Et le corollaire qui a coûté une nuit : un filtre vectoriel porte sur les
champs du **parent**, mais le HNSW indexe les **chunks**. La condition se
compile donc sur un alias joint.

## 5. Le pré-filtre, par signal

| Signal | Mécanisme | Exact ? |
|---|---|---|
| **BM25** | `allowed_ids` — descend jusqu'aux résolveurs, `doc_freq` sur le sous-ensemble | oui |
| **sparse** | `search_filtered` — pas de statistique de corpus, donc un filtre ne peut que retirer des lignes | oui |
| **vecteur** | graphe projeté + semi-masque HNSW | oui **depuis le 27 août** |

Les ids passés à lucivy doivent être **triés et dédupliqués** : c'est leur
contrat d'API, et ça vaut 6 ms → 0,22 ms sur un gros ensemble.

## 6. Runs, événements, conversations

- Tout ce qui s'exécute est un **run**, avec un parent et une portée.
- Le **bus** a des sujets (`catalog`, `search`, `agent`, `dataflow`,
  `messages`, `run.<id>`, `run.<id>.inbox`) et des curseurs.
- Une **boîte aux lettres** est un sujet ; un agent la lit **entre deux
  tours**, jamais au milieu.
- Un **outil asynchrone** est un run enfant qui poste dans cette boîte.
- Une **conversation** est un fil de participants ; elle ne se ferme pas.
- Une **posture** dit qui s'est tu, envers qui, et pourquoi. Un cycle dans le
  graphe « qui attend qui » est un blocage, et il est annoncé.

## 7. Les invariants

Chacun payé par un incident.

1. **Une erreur d'outil est un résultat**, jamais une exception : c'est ce
   qui laisse le modèle se rattraper.
2. **Un `tool_call` reçoit toujours une réponse dans son tour.** D'où
   l'accusé des outils asynchrones.
3. **Jamais de rétrécissement silencieux.** Si un filtre ne s'applique pas,
   on le dit.
4. **Jamais d'absence invisible.** Un blocage, un domaine, une restriction se
   voient dans le rendu.
5. **L'identité ne dépend pas du contenu.** Ni pour un scope, ni pour un
   fichier.
6. **On stocke l'absolu, on affiche un point de vue.** Chemins et dates.
7. **Le moteur lit des `enum`, l'humain lit du texte.** Fiches, genres de
   pause, coordonnées.
8. **Ce qu'on ne sait pas, on le dit** — la nature d'un participant inconnu
   est « inconnue », pas « humain ».
