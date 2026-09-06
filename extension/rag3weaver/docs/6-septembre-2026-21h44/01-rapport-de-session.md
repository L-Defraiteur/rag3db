# Rapport de session — 6 septembre 2026, après-midi et soirée

**Session principale, sur Fable.** Vingt-sept commits depuis `cacdd88cd`
(le bilan de la passe fondations, 14h), en parallèle d'une session « moteur
burn » (rag3db-e7) dans le même arbre. Quatre chantiers, dans l'ordre où ils
sont arrivés, et chacun a changé la question suivante.

## 1. La couche utilisateur du catalogue de gabarits (14h43)

Question de Lucie : *« les templates non "builtin" devraient pas enrichir une
extension "utilisateur" du catalogue ? »*. Doc :
[`../6-septembre-2026-14h43/01`](../6-septembre-2026-14h43/01-la-couche-utilisateur-du-catalogue-de-gabarits.md).

- La fiche porte son **origine** (`builtin` / `project`) et `derived_from`
  (l'empreinte du gabarit fourni qu'elle masque). Le masquage est une
  **identité** : même `(family, name)`, même `_uuid`, origine changée.
- `Catalog::sync_templates(project)` : la seule chose qui alimente la couche
  — avant, seuls les tests indexaient des fiches, et `adopt` disait
  « réindexez » à personne.
- `agent::mount_agent_services` / `_on` : **le seul montage d'agent**. La
  chaîne catalogue + source `worktree:` + palette n'existait qu'en tests ; il
  monte aussi la porte des commandes (un agent Gemini a rendu sa mission sans
  rien écrire parce que `run` n'avait pas de porte).
- La suite cloud, jouée hors du lot depuis des jours, tombait sur la garde
  du 5 septembre (HashEmbedder contre BGE-M3). Réparée : le même modèle des
  deux côtés.

## 2. Quatorze minutes pour trente fichiers (15h–16h)

Lucie : *« ça devrait pas mettre des années pour indexer si peu ? »*. Issues
[`01`](../issues/6-septembre-2026/01-quatorze-minutes-pour-trente-fichiers.md)
et [`02`](../issues/6-septembre-2026/02-le-chemin-burn-wgpu-a-optimiser.md).
Mesuré au lieu d'expliqué, et la première explication était fausse :

| passage | embarquement de `src/dataflow` |
|---|---|
| suite cloud, deux tests en parallèle | 862 s |
| démon **de la veille** (ancien bridage) remplacé | 354 s |
| release au lieu de debug | 337 s (rien) |
| lots triés par longueur | 149 s |

Trois causes réparées : un démon qui survit aux reconstructions (l'identité
porte l'empreinte de l'exécutable, `POST /quitter`, le harnais remplace) ;
le rembourrage au plus long du lot ; et le régime `confort` qui bridait
aussi sur la carte libre — puis qui choisissait **la carte de Lucie** dès
qu'un modèle occupait l'autre (« la moins chargée » se retourne). Son
heuristique : *la carte au moins d'écrans actifs*. Deux règles de mémoire
en route : `pidof`, jamais `pgrep -f` ; et **identifiants en anglais**.

La session moteur a pris le modèle lui-même (forks burn pre.3, Flex32,
autotune, fusion, attention fusionnée) : BGE-M3 5 300 → 24 000 jetons/s.

## 3. Le chemin d'indexation (19h57–21h)

Cahier des charges de la session moteur
([issue 04](../issues/6-septembre-2026/04-rapport-pour-la-session-principale.md)),
cible de Lucie : **un dépôt de 5 000 fichiers en 30 minutes**, prêt pour des
agents de code, sans troquer la généricité des schémas. Doc :
[`../6-septembre-2026-19h57/01`](../6-septembre-2026-19h57/01-la-decoupe-des-scopes-et-la-vue-par-parent.md).

| étape | `src/dataflow` (30 fichiers) | cœur C++ (1 642 fichiers) |
|---|---|---|
| référence sur le moteur réglé | 73 s | — |
| texte propre d'un scope + découpe par lignes + `signature` plus embarquée | 47 s | — |
| pipeline (modèle et base se recouvrent), deux producteurs | 36 s | — |
| granite-107m, lots par `budget_conseille`, borne de surface d'attention | 11,7 s | 368 s |
| index HNSW en masse (doc 18) | — | 270 s |
| liens par `COPY` | — | **51 s** |

**Cinq mille fichiers : moins de trois minutes**, dix fois sous la cible.
Ce que la grande mesure a appris que la petite cachait : **la base l'emporte
sur le modèle** — écriture des vecteurs ligne à ligne (120 s), insertion
d'arêtes (233 s, et CREATE ne fait pas mieux que MERGE). Le chargement en
masse du moteur règle les deux.

Aussi : la **vue par parent** (`EntityConfig.group_by`, générique), la
**garde du modèle** (`EmbeddingModelMismatch` : un index construit avec
granite refuse BGE-M3 en le nommant), la mesure qui choisit modèle et racine.

## 4. Ce qui reste, et à qui

| reste | à | taille |
|---|---|---|
| les nœuds par `COPY` en première ingestion, les vecteurs écrits avec la ligne de chunk, les vérifications sautées sur table vide — les 28 s hors modèle du cœur C++ vers 5 s | moi, **à ton feu vert** | une demi-journée, même patron |
| l'entrée « indexer ce dépôt » : source, signaux cochés, chargement en masse, étapes publiées ; puis l'incrémental au niveau du dépôt (empreintes de fichiers, branches) ; puis le tick de fond avec un hôte | moi, ordre proposé | trois demi-journées |
| le banc de qualité granite-107m / 278m / BGE-M3 sur des requêtes de code en français | session moteur | — |
| `Catalog::search` (le monolithe) : migrer les 105 appels de tests sur le lanceur | moi, décidé le matin | une demi-journée |
| le renommage anglais du crate (206 noms) | moi, décidé le matin | une demi-journée mécanique |

## 5. Une faute à ne pas refaire

`git add -A` dans un arbre partagé : le commit `a7bb1b308` porte huit fichiers
en cours de l'autre session. Règle en mémoire : chemins explicites, et un
état non compilable de son fichier se corrige ou se commite tout de suite en
prévenant l'autre.
