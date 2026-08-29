# 12 — Architecture, au soir du 27

Reprend le [doc 02](02-architecture.md) et le met à jour. Ce qui n'a pas bougé
y est rappelé court ; ce qui a changé aujourd'hui est développé.

## 1. Les trois couches, et la règle qui les tient

```
   agent        boucle, outils, session, postures, compteur
   dataflow     nœuds, ports, services, runtime, trace
   catalogue    entités, relations, index, drain
```

**Rien ne saute une couche.** Un agent ne parle pas au catalogue : il appelle un
outil, qui est un graphe, qui utilise des nœuds, qui touchent le catalogue.
C'est ce qui permet qu'un outil soit une donnée versionnée plutôt que du code.

## 2. Les axes, rangés par vitesse de changement

Le test qui les sépare : **deux choses qui changent à des rythmes différents ne
sont pas la même chose.** Un axe s'est ajouté aujourd'hui.

| Axe | Change | Porté par |
|---|---|---|
| organisation | jamais | `Scope { org }` |
| cellule (projet) | rarement | `Scope { project }` |
| origine | à chaque dépôt | `Origin`, `Coordinates` |
| **identité d'agent** | **survit aux sessions** | **`Participant`** |
| domaine de travail | par run | `WorkDomain`, `Run.domain` |
| lentille de chemin | par affichage | `PathLens` |

**L'identité d'agent est l'ajout du 27.** Un `Participant` était créé avec
l'**adresse** d'un run — donc « Ada » était quelqu'un de différent à chaque
réveil. `identity_of` résout une adresse vers le nom stable, et
`Participant -PERFORMED-> Run` referme le chemin *ce qui a été dit → le run →
celui qui l'a mené*.

Et le domaine est **sur le run**, pas sur l'identité : un agent peut travailler
ailleurs demain, donc son rôle se lit dans ce qu'il a fait.

> **Le fil est épisodique, le participant persiste.** Une conversation reste
> nommée par les adresses qui l'ont ouverte ; ses participants sont les mêmes
> d'un fil à l'autre.

## 3. Les fichiers, par question

| Question | Fichier |
|---|---|
| Comment on cherche | `search.rs`, `search_strategy.rs`, `fusion.rs` |
| Ce qui est déclaré, et ce que ça coûte | `config.rs` — `EntityConfig`, `SchemaCost`, `Lifecycle` |
| Ce qu'une ligne a le droit de devenir | `config.rs` — `Lifecycle`, appliqué au drain |
| Comment on écrit, et par quels lots | `catalog.rs`, `dataflow/record_nodes.rs` — `budget_batches` |
| Le code comme graphe | `code.rs`, `code_tools.rs` |
| D'où vient un fichier | `origin.rs` |
| Ce qu'un agent regarde | `work_domain.rs` |
| La boucle d'agent | `agent.rs` — `ToolBox`, `PauseKind`, asynchrone |
| Postures et blocages | `postures.rs` |
| Ce qu'on garde d'un tour à l'autre | `session.rs` — `Absorb`, renvois, `recall` |
| Ce qui a été consommé | `meter.rs` — `Unit`, `Consumption` |
| Qui a parlé, quand, à qui | `dataflow/trace_nodes.rs` |
| Quelle carte fait quoi | `burn_device.rs` — `BurnRole`, `or_role` |

## 4. Le piège des index, toujours vrai

- **plein texte** → sur la table **parente** ;
- **vecteur** et **sparse** → sur la table de **chunks**.

D'où `chunked: Some(false)` pour une entité dont le contenu *est* son titre, et
le refus de `validate` si on demande un vecteur sans rien à embarquer — les deux
formes de contenu comptant : le pipeline simple **et** la participation à une KB.

`SchemaCost` rend ce piège lisible en un chiffre : `Symbol` en `HYBRID` coûte
3 275 embeddings et **6 550** documents plein texte, parce que les chunks
s'ajoutent au parent.

## 5. Ce que le langage de déclaration sait dire

`EntityConfig` décrivait une **forme** et des **signaux**. Il décrit maintenant
aussi un **comportement** :

- `hashsafe` — l'identité ;
- `signals`, `chunking`, `chunked` — la recherche ;
- `return_fields` — ce qu'un résultat rend ;
- **`lifecycle`** — l'état et ses transitions, vérifiées sans rien exécuter
  (état inatteignable, champ non-`String`, transitions homonymes) et appliquées
  au drain, seul endroit où l'ancien état est connu.

La frontière à retenir : **elle sépare les invariants exprimables de ceux qui ne
le sont pas.** On l'élargit en agrandissant le langage, pas en rendant l'agent
plus malin.

## 6. La session, et l'invite comme projection

`Agent::run` reste le chemin simple. Deux choses la réécrivent, au même endroit :

- **`absorb`** réduit ce qui n'a plus à être envoyé en entier. Chaque forme est
  dérivée du contenu **mémorisé**, jamais de ce qui est dans l'historique —
  sinon chaque passage tronquerait la troncature.
- **`refresh_waiting_block`** injecte un bloc **dérivé** des postures : vide
  quand il n'y a rien, et **en fin d'historique** pour ne pas casser le cache de
  préfixe du fournisseur.

> **L'invite système n'est pas un document, c'est une vue recalculée sur ce que
> le graphe sait au moment d'assembler.**

C'est le modèle à étendre : le domaine, le répertoire courant, qui d'autre est
dans le fil — tous des blocs dérivés, aucun stocké.

## 7. Le compteur

`(ressource, unité, quantité)` — la même forme pour un LLM distant, un LLM
local, un TTS, un STT. **Des faits, jamais un prix** : les tarifs changent, la
tarification est une table qui résout des slugs au moment de lire.

`Consumed` est distinct de `LlmCall` : le premier dit *ce qui a été consommé*,
le second *comment la boucle s'est passée*.

## 8. Les invariants, chacun payé

1. Rien ne saute une couche.
2. L'historique rendu par `Agent::run` est **toujours bien formé** : chaque
   appel annoncé a son résultat.
3. Une forme réduite dérive du contenu entier, jamais d'elle-même.
4. Un résultat garde le tour où il est arrivé.
5. Le plein texte est sur le parent, le vecteur sur les chunks.
6. Une erreur de configuration vaut mieux qu'une entité silencieusement
   introuvable.
7. Un lot d'embedding se borne par le **texte**, pas par le nombre — l'optimum
   mesuré est ~2 048 jetons par passe, et **au-delà le débit redescend**.
8. Le choix d'une carte se **dit**, il ne se devine pas.
9. Une adresse est une incarnation ; un participant est quelqu'un.
10. Un fil à plus de deux se **dit** dans l'enveloppe — la paire ne peut plus le
    porter.
11. Ce qui est dérivé ne peut pas être périmé ; ce qui est stocké survit à ses
    raisons.

## 9. Les dettes nommées

- **`Consistency` déclaré, jamais honoré**, et `flush_insertions` écrit sans
  indexer. La plus vieille.
- **`Trace` sans `hashsafe`** : deux événements identiques fusionnent en
  silence.
- **`FlushConfig::embed_batch_size`** — doublon mort de `gpu_batch_size`.
- **Le rôle n'est pas appliqué** : `WorkDomain` ne filtre pas ce que les outils
  rendent, et une expérience l'a démontré ([doc 11](11-rapport-de-session.md)).
- **Le rendu des résultats déverse des champs** au lieu de rendre une forme —
  la spécification existe depuis le projet précédent
  ([doc 11 §5.2](11-rapport-de-session.md)).
