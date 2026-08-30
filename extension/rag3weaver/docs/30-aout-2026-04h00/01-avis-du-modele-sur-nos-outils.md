# L'avis d'un modèle sur nos outils

> **Lu et vérifié après coup** — voir
> [02-ce-que-l-avis-a-donne.md](02-ce-que-l-avis-a-donne.md). Une critique était
> juste et bloquante, elle est corrigée. Une autre portait sur un montage
> d'expérience, pas sur le moteur. Le reste est ouvert.

**Produit par `tests/e2e_avis_du_modele.rs`**, modèle `google/gemini-3.5-flash`, 7 outils publiés, 5 cibles énumérées, réponse en 21.5s.

Les fiches ci-dessous ne sont pas recopiées : elles sortent de `tools::graph_tool_defs_openai`, la liste que reçoit un agent en production. La réponse est verbatim.

---

## Les outils, tels que le moteur les publie

### `adopt`

Enregistre une entité que vous venez de construire comme gabarit du projet, réutilisable ensuite avec place. Ce qui a marché une fois se repose sans le réécrire.

Paramètres : `as`, `category`, `description`, `entity`, `note`

### `edit`

Modifie un fichier du projet puis met l'index à jour. Soit remplacer un passage unique (old → new, copié exactement depuis read, préfixes de lignes tolérés), soit écrire le fichier entier (content).

Paramètres : `content`, `new`, `old`, `path`

### `grep`

Cherche une expression régulière dans les fichiers du projet ; chaque ligne trouvée est rapprochée du scope (fonction, classe…) qui la contient.

Paramètres : `context_lines`, `extension`, `max_results`, `path_prefix`, `pattern`, `relation`, `relation_limit`

### `list`

Liste les fichiers du projet sous un préfixe de chemin, avec leur nombre de lignes et leur état d'indexation (indexé, périmé, non indexé). Pour s'orienter avant grep ou read.

Paramètres : `limit`, `path_prefix`

### `place`

Pose un gabarit du catalogue dans le schéma vivant : une entité toute faite (user, product, conversation…) avec ses champs et ses signaux de recherche, plutôt qu'une configuration à écrire à la main. Cherchez d'abord le gabarit avec search(target='Template').

Paramètres : `as`, `patterns`, `template`

### `read`

Lit un fichier du projet par tranche de lignes numérotées ; dit les scopes de la fenêtre et si l'index est périmé.

Paramètres : `limit`, `offset`, `path`

### `search`

Cherche dans une entité ou une base de connaissances, par mots-clés **et** par sens : une requête en langue naturelle ('comment un nœud signale son échec') trouve ce qui en parle sans en reprendre les mots. Donnez en plus 'relation' pour suivre le graphe depuis chaque résultat et obtenir ses voisins en arbre — pour du code : CONSUMES (ce que ce scope utilise), CONSUMED_BY (qui l'utilise), PARENT_OF / HAS_PARENT (méthodes d'une classe et inversement), IMPLEMENTS, INHERITS_FROM, USES_LIBRARY.

Paramètres : `direction`, `expand_limit`, `limit`, `query`, `relation`, `rerank`, `target`

---

## La question posée

> Voici la liste exacte des outils dont tu disposerais, au format que tu reçois d'habitude. Ce moteur indexe du code et de la connaissance dans une base de graphe, et sert des agents qui construisent des applications.
> 
> Trois questions, dans l'ordre, en français :
> 
> 1. **Qu'est-ce que ces outils rendent possible ?** Sois concret : quelles tâches tu mènerais à bien avec ça, et lesquelles tu abandonnerais.
> 2. **Qu'est-ce qui te gêne dans ce qui existe ?** Une description ambiguë, un paramètre dont tu ne saurais pas quoi faire, deux outils que tu confondrais, un défaut qui te ferait perdre des tours.
> 3. **Que te manque-t-il ?** Nomme les outils absents par ordre d'importance, et pour chacun dis ce que tu ne peux pas faire sans lui.
> 
> Ne sois pas poli : les réponses complaisantes ne servent à rien. Si quelque chose est mal conçu, dis-le et dis pourquoi.

---

## La réponse, verbatim

Voici une évaluation brute et sans concession de votre boîte à outils.

---

### 1. Qu'est-ce que ces outils rendent possible ?

#### Ce que je peux faire :
*   **Cartographier et naviguer dans du code inconnu :** Grâce au couplage entre la recherche textuelle (`grep`/`search`) et les relations de graphe (`CONSUMES`, `CONSUMED_BY`, etc.), je peux rapidement comprendre l'impact d'une modification, trouver qui appelle quoi, et identifier les dépendances d'un module sans avoir à lire tout le projet.
*   **Appliquer des modifications locales ciblées :** L'outil `edit` avec le mode `old`/`new` est propre pour des patchs chirurgicaux (remplacer une signature de fonction, corriger un bug dans un bloc précis).
*   **Générer du boilerplate via des templates :** Si le catalogue de templates est bien fourni, je peux instancier des structures de données standard (`place`) et sauvegarder des patterns récurrents du projet (`adopt`).

#### Ce que je dois abandonner (impossible en l'état) :
*   **Garantir que le code fonctionne :** Je n'ai aucun moyen d'exécuter le code, de lancer les tests unitaires, ou même de compiler. Je vais fatalement introduire des erreurs de syntaxe ou des régressions. **Je code à l'aveugle.**
*   **Faire du refactoring d'envergure :** Sans outil de renommage global ou de déplacement de fichiers, et sans compilateur pour me signaler les erreurs, déplacer une classe ou modifier une interface partagée est une mission suicide.
*   **Gérer le cycle de vie des fichiers :** Je ne peux pas supprimer de fichier, ni en renommer.

---

### 2. Qu'est-ce qui te gêne dans ce qui existe ?

#### Le bug de conception majeur : L'incohérence de `search.target`
La description de `place` dit : *"pose une entité toute faite (user, product, conversation…)"*. La description de `search` dit : *"Cherche dans une entité... ex. 'Product'"*.
**Mais le paramètre `target` de `search` est un `enum` strict : `["File", "Library", "Scope", "Symbol", "Template"]`.**
Si j'instancie un template "Product" ou "User", je ne peux pas chercher dedans car ils ne font pas partie de l'enum. C'est bloquant. Soit l'enum est trop rigide, soit la documentation ment.

#### L'ambiguïté de `adopt` et de la notion d'« entité »
`adopt` demande un paramètre `entity` (le nom de l'entité enregistrée à adopter).
*   Qu'est-ce qu'une "entité" pour le moteur ? Un fichier ? Une classe ? Un symbole ?
*   Comment ai-je "construit" cette entité au préalable si mes seuls outils d'écriture sont `edit` (qui écrit du texte brut dans des fichiers) ? Le moteur extrait-il magiquement les entités du code ? Si oui, comment savoir si l'extraction a réussi avant d'appeler `adopt` ?

#### Le piège de `edit` (Exclusivité implicite)
L'outil `edit` propose `content` (écriture complète) OU `old`/`new` (remplacement).
*   Rien dans le schéma JSON n'empêche de passer les trois paramètres en même temps. Que se passe-t-il si je le fais ? Crash ? Priorité silencieuse ?
*   Le remplacement `old`/`new` exige que `old` soit présent "une seule fois". Si je me trompe et qu'il est présent deux fois, l'outil échoue-t-il proprement ou applique-t-il le changement au premier ? Sans "dry-run", c'est dangereux.

#### Le nœud de cerveau de `search.direction` + `relation`
Si je cherche avec la relation `CONSUMED_BY` (qui signifie "A est consommé par B", donc une flèche de A vers B) et que je règle `direction` sur `Incoming`, est-ce que j'inverse le sens de la relation (ce qui revient à faire un `CONSUMES`) ?
Mélanger des relations orientées par nature avec un paramètre de direction générique est une source d'erreurs systématique pour un agent LLM.

#### Les limites arbitraires et le "Pied de page" de `read`
`read` a un plafond de 2000 lignes et dit *"le pied de page dit comment continuer"*.
Un agent de code n'a pas à parser un "pied de page" textuel pour savoir comment paginer. Donnez-moi simplement un booléen `has_more` et le prochain `offset` dans la réponse structurée (JSON), pas du texte à décoder.

---

### 3. Que te manque-t-il ? (Par ordre d'importance)

#### 1. Un outil d'exécution de commandes (`execute_command` / `run_terminal`)
*   **Pourquoi c'est vital :** Sans ça, je ne peux pas lancer `cargo check`, `npm test`, `pytest`, ou un linter. Je ne peux pas valider mon code. Un agent qui ne peut pas tester ses modifications est un agent qui produit du code cassé.
*   **Sans lui :** Je refuse de faire des modifications complexes. Je me limite à de la lecture ou à des corrections de typos.

#### 2. Un outil de suppression et déplacement de fichiers (`delete_file`, `move_file`)
*   **Pourquoi c'est vital :** `edit` permet de créer des fichiers (via `content`), mais je ne peux pas nettoyer le projet, supprimer un fichier obsolète, ou réorganiser l'arborescence.
*   **Sans lui :** Impossible de mener à bien des tâches de refactoring ou de nettoyage de dette technique.

#### 3. Un outil d'inspection du schéma et des templates (`get_schema` / `list_templates`)
*   **Pourquoi c'est vital :** `place` me demande d'utiliser des templates, et `search` me demande de chercher dans des cibles. Mais je n'ai aucun outil pour lister les templates disponibles dans le catalogue, ni pour voir la structure actuelle du graphe de connaissances. Je dois deviner leurs noms.
*   **Sans lui :** Je vais tenter d'utiliser des templates inexistants ou mal orthographiés, ce qui va générer des erreurs en boucle.

#### 4. Un outil de comparaison (`git_diff` ou `preview_edit`)
*   **Pourquoi c'est vital :** Avant de valider une modification avec `edit`, je dois pouvoir vérifier l'état global des modifications pour m'assurer que je n'ai pas cassé la cohérence du projet.
*   **Sans lui :** Si je fais une erreur dans un enchaînement d'appels à `edit`, je perds le fil de ce qui a été modifié et je ne peux pas revenir en arrière proprement.
