# Les fiches mémoire classées par sortie structurée

*11 octobre 2026, fin d'après-midi. Session « mémoire longue ». Page de conception,
pour Lucie : aucun code avant ton mot. Lue avec les deux visions du 4 octobre
(`visions/2026-10-04-00h09-…` et `visions/2026-10-04-14h58-…`), la page 07 du
10 octobre et la page du lien générique.*

La question : que faut-il pour qu'une fiche soit **classée** — son genre, son
sujet, ce qu'elle devient quand ce qu'elle concerne change — par une décision
fermée d'un modèle, plutôt que par le texte libre que rend un modèle ? La réponse
tient en une phrase : **un nœud `decide` générique, dont les options viennent
toujours d'une déclaration, et rien de propre à la mémoire.**

## 1. Une fiche aujourd'hui, et ce qu'elle n'a pas

Le gabarit `memory` (`templates/backends/memory/`) déclare quatre entités :

| entité | ce qui la classe déjà | qui choisit |
|---|---|---|
| `Memory` | `kind` ∈ {fact, preference, decision, pointer} ; `origin` ∈ {said, inferred} ; `reach` ∈ {person, project, team} ; cycle de vie `state` (current, to_review, superseded, set_aside) | **le modèle client** (Claude), au `put_memory` : le schéma exige les trois champs, l'énumération est fermée et le validateur refuse une valeur hors liste |
| `Subject` | son nom ; son état (current, set_aside) | le client, au `put_subject` |
| `Ref` | `genre`, contrôlé par la liste des `RefType` déclarés | le client, à l'`add_ref` |
| `RefType` | la liste des genres, qui grandit par `create_ref_type` | le client |

Et trois relations : `ANCHORED_TO` (fiche → sujet), `REPLACES` (fiche → fiche),
`CITE` (fiche → référence), posées par les outils `anchor`, `supersede_link` et
`add_ref`. La réaction `review_anchored` fait passer une fiche « à revoir » quand
son sujet change.

Ce qu'elle n'a pas :

- **Personne ne classe à part le client.** Tout ce qui est classé l'est par le
  modèle qui travaille, au moment où il écrit, et seulement s'il pense à écrire.
  Un rangement sans qu'on le demande est l'objet de l'archiviste (vision des
  mémoires, §9) ; aucune pièce ne le porte encore.
- **Rien ne choisit un sujet.** `anchor` relie une fiche à un sujet que le client
  nomme. Rien ne propose « cette fiche va sous tel sujet existant, ou sous aucun ».
- **Une fiche à revoir le reste.** `to_review` se pose seul ; en sortir (confirmer,
  remplacer, mettre de côté) attend que quelqu'un le décide.
- **Une classification ne se voit pas comme telle.** `kind` ne dit pas qui l'a
  choisi. `origin` le dit pour le contenu (said / inferred), pas pour le
  classement.

## 2. Où une classification s'insère, et ce qu'elle produit

Trois moments, et dans les trois **le jeu d'étiquettes vient d'une déclaration,
jamais du modèle** :

| moment | la question | les étiquettes | d'où elles viennent |
|---|---|---|---|
| **à l'écriture** | quel genre, quelle portée ? | les valeurs du champ | l'`enum` du schéma de l'entité (`Memory.kind`, `Memory.reach`) |
| **au lien** | sous quel sujet ? cette fiche en double-t-elle une autre ? | les candidats d'une recherche bornée, **plus « aucun »** | les lignes que rend `recall_subjects` ou `recall`, une liste fermée le temps d'un appel |
| **à la relecture** | une fiche à revoir : que devient-elle ? | les transitions permises depuis son état | le cycle de vie déclaré (`confirm`, `supersede_reviewed`, `set_aside_reviewed`) |

Ce que chaque décision produit est **une étiquette de la liste, ou un refus nommé**.
Jamais un texte libre, jamais une valeur nouvelle. Créer quelque chose de neuf (un
sujet, un genre de référence) reste un **outil**, avec ses refus et sa recherche
des proches (vision des mémoires, §9.4, « des outils plutôt qu'une sortie
structurée ») : `decide` choisit parmi ce qui existe, l'outil crée.

« Aucun » est toujours une option quand la liste vient d'une recherche : sans lui,
une décision fermée range une fiche sous le moins mauvais sujet, et le classeur se
remplit de faux rangements.

## 3. Avec le nœud `decide`, sans rien de propre à la mémoire

Le nœud `decide` n'est pas encore sur master : la session du proto le pose (lot 2
de `docs/11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md`,
§ « `decide` » : « un appel structuré au modèle — le texte du flux complété, N
étiquettes — qui rend une étiquette, et sa probabilité quand le fournisseur la
donne »). Deux voies existent au crate, aucune n'a encore de nœud client :

| voie | ce qui existe | ce qu'elle rend | où elle tourne |
|---|---|---|---|
| **sortie structurée** | `GenOptions::with_response_format` et `ResponseFormat::JsonSchema { strict }` (`src/llm.rs`), posés par le client compatible OpenAI (`src/openai_llm.rs`) | l'étiquette, **sans probabilité** (le client ne demande pas de `logprobs`) | Vertex (Gemini), llama-server |
| **le Decider à un jeton** | `Decider::decide_at` (`src/decider.rs`), servi par `LlamaServerDecider` | **une probabilité par option**, lue sur les log-probabilités du jeton suivant | llama-server seul |

Ce que la mémoire demande au nœud, et qui vaut pour tout gabarit :

1. **Ses options viennent d'une source déclarée**, l'une des trois du §2 :
   l'`enum` d'un champ d'une entité, les transitions permises d'un cycle de vie
   depuis l'état de la ligne, ou une liste rendue sur un port par le nœud
   d'avant (les candidats d'une recherche), avec « aucun » ajouté. Ce sont des
   notions d'`EntityConfig` et du dataflow, pas de la mémoire.
2. **Son texte est un gabarit rendu depuis la ligne** (les champs de la fiche, ceux
   des candidats), comme les autres prompts déclarés.
3. **Il rend `{label, probability?, via}`**, et refuse une étiquette hors liste en
   le disant (la vérification après coup du proto). La voie choisie se lit au
   manifeste, sur ce que le modèle déclaré sait faire.
4. **Ce qui suit est un `route`** sur l'étiquette : poser le champ, appliquer la
   transition, poser le lien par le verbe générique (`link.mmd`), ou ne rien faire.
   Une transition ou un lien passent par les mêmes nœuds que les outils, avec les
   mêmes politiques (`from_states`, le refus d'un bout absent).

Le gabarit `memory` n'aurait donc à écrire que des graphes : par exemple
`put_memory → recall_subjects → decide(candidats + aucun) → route → link`. C'est
la même règle que pour le lien générique : une organisation nouvelle se décrit
dans le schéma et les graphes, jamais en dur.

**Plusieurs sujets pour une fiche** : le Decider ne rend qu'une option. Deux formes
possibles, à trancher (question 3) : une décision oui / non par candidat, ou une
sortie structurée à tableau (voie OpenAI seule).

## 4. Ce que ça permet

- **Le classeur de l'archiviste** (vision des mémoires, §9.2 et §9.3) : « classer le
  L1 » sous des sujets existants, c'est le moment *au lien* ; la porte à deux temps
  (« faut-il chercher ? », « faut-il injecter ? ») est deux `decide` à options
  oui / non. Le même nœud, sans pièce de plus.
- **Des fiches qui pointent vers tout** : le genre d'une référence dite en langue
  (« le design d'hier ») se choisit parmi les `RefType` déclarés ; ce qu'une règle
  rhai reconnaît seule n'appelle pas de modèle (§9.4, « l'extraction est
  déterministe d'abord »).
- **Le contexte comme une vue** (seconde vision, §5) : une zone « ce qui a été
  appris » se remplit de fiches classées par sujet et par état. Une fiche « à
  revoir » qui a sa décision de relecture ne pèse plus dans la vue comme une
  inconnue.
- **Une relecture qui avance seule** : une fiche à revoir reçoit une proposition de
  transition. Pour la mémoire de la personne, cela reste une proposition par
  défaut (question 2).

## 5. Comment on la mesure

- **Un banc de fiches étiquetées.** Partir de ta base `memoire-longue` (ses fiches,
  ses sujets) et de fiches écrites pour le banc. Pour chaque moment du §2, la bonne
  étiquette, dont « aucun » quand aucun sujet ne va. On mesure l'exactitude par
  étiquette, la confusion, et surtout **les faux rangements** (une étiquette donnée
  quand « aucun » était juste).
- **Gemini (Vertex) juge et sert le produit.** C'est le modèle de référence pour
  étiqueter et celui contre qui le produit se mesure, comme pour l'agent de code.
- **Les modèles locaux éprouvent le protocole, pas la qualité.** Avec llama-server
  et le Decider, on vérifie que les options se forment, que `route` suit et qu'un
  hors-liste se refuse. Aucune promesse de tout-local.
- **pplx-decider est un candidat à évaluer**, par la voie du Decider, sur le même
  banc. La décision du 10 octobre tient : un score ordonne bien et juge mal, et un
  seuil ne se transporte pas d'un jeu à l'autre. Une probabilité ne sert donc qu'à
  **s'abstenir** (rendre « aucun » sous un seuil), et ce seuil se mesure sur deux
  corpus au moins avant d'être cru.

## 6. Écarts et questions pour Lucie

Écarts avec ce qui est écrit ailleurs :

- **Les modèles de décision reviennent, comme option mesurée.** La page 05 du
  10 octobre les avait mis de côté ; ici ils ne sont qu'une voie du nœud, jamais le
  défaut, et jugés par le banc.
- **La création reste un outil.** Rien ici ne crée de sujet ni de genre par sortie
  structurée : c'est la forme retenue le 4 octobre (§9.4), gardée.

Questions :

1. **Qui classe à l'écriture ?** Le client, comme aujourd'hui, avec `decide` en
   second avis qui signale un désaccord ; ou `decide` seul, et le client n'écrit
   que le texte ? Je propose le premier : le client sait des choses que la fiche ne
   dit pas.
2. **Une classification écrase-t-elle ?** Je propose que non : elle remplit un champ
   absent ou propose. Il faut alors marquer qui a classé (un champ `classified_by`,
   ou `origin` étendu au classement), donc toucher au schéma du gabarit. Ce qui a
   été dit par la personne n'est jamais reclassé.
3. **Plusieurs sujets** : une décision oui / non par candidat (les deux voies) ou un
   tableau (sortie structurée seule) ?
4. **Le premier usage** : le choix du sujet au `put_memory` (l'ancre qu'on ne pense
   pas à poser), ou la relecture des fiches à revoir ? Je propose le sujet : c'est
   là que le classeur se fait ou se défait.
5. **pplx-decider** : un 27B. À servir sur luciepc par llama-server, s'il existe en
   GGUF ; sa licence est à lire avant tout usage hors banc.

L'ordre, si tu dis oui : le nœud `decide` arrive par le proto (rien à faire ici) ;
puis le banc du §5, avant le moindre graphe ; puis le premier usage de la
question 4.

## 7. Une piste pour plus tard : un Decider qui rend des probabilités

*Note du 18 octobre 2026, de Lucie par l'orchestration : « petit luxe, pas bloquant ».
La méthode ne change pas : chercher les sujets voisins (`recall_subjects`), puis les
proposer à Gemini en sortie structurée — une décision par fiche depuis la tentative 4.*

**pplx-decider-v1.1-27b** (Perplexity,
<https://huggingface.co/perplexity-ai/pplx-decider-v1.1-27b>) rend **une probabilité
par réponse d'une liste fermée**. Ce qu'il apporterait ici :

- un **seuil réglable pour « aucun »**, au lieu de la seule abstention que le modèle
  choisit ;
- une **mesure de confiance par fiche**, à garder avec le rangement (une fiche rangée
  de justesse se relit en premier).

Ce qu'on en sait, d'après la note (non vérifié ici, à relire avant tout usage hors
banc) : licence Apache 2.0, ce qui répond à la question 5 du §6 si elle se confirme ;
0,04 $ par million de jetons en entrée par l'API, la sortie gratuite ; il tient en
local sur luciepc en 4 bits. Il s'emboîte dans la voie `decider` de `Decide`, que le
banc joue déjà (`RAG3WEAVER_BANC_DECIDE`, tentatives 1 et 2), et `labels_from` lui
donne la liste des candidats par fiche (4c3e8179a).

**La réserve à mesurer d'abord** : la tentative 2 a montré un Decider sûr de lui là où
il se trompe (Qwen3-1.7B, 0,92 à 1,0 sur l'étiquette choisie dans les manqués et les
faux ancrages) — « un score ordonne bien et juge mal ». Un seuil ne vaut que si la
probabilité est calibrée : la première mesure serait la courbe des fiches justes
selon la probabilité, sur le jeu v1, avant de régler quoi que ce soit.

**Quand y revenir** : si le classement des fiches devient le goulot — en coût (les
appels à Gemini) ou en qualité (des faux rangements qu'aucune instruction ne
reprend).
