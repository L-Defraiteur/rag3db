# Backends déclaratifs et persistance — 19-09-2026

## Décision

Le backend vit dans rag3weaver. Magic sera un template de domaine. Le framework ne connaît ni les decks, ni les états `draft` et `published`.

Un manifeste JSON assemble les schémas JSON Schema des entités, leur identité et leurs rôles d'indexation, leurs politiques d'écriture, les relations et les outils exposés. Les graphes Mermaid décrivent les opérations. Le MCP annonce les outils du manifeste, avec leurs schémas d'entrée dérivés ; aucune liste métier séparée dans l'adaptateur.

Implémentation initiale : [backend.rs](../../src/backend.rs), [backend_nodes.rs](../../src/backend_nodes.rs), [hôte persistant](../../src/bin/rag3weaver-backend.rs), [adaptateur MCP](../../scripts/serve_backend_mcp.py).

## Politiques génériques disponibles

| Déclaration | Comportement |
| --- | --- |
| `config.hashsafe` | UUID stable dérivé des champs d'identité, chaînes non vides exigées par ce premier hôte |
| `writes.created_at` | Champ entier rempli à la création, conservé à la modification |
| `writes.updated_at` | Champ entier rempli à chaque écriture acceptée |
| `writes.revision` | Révision entière incrémentée ; remplacement conditionné par `expected_revision` |
| `writes.immutable` | Insertion puis relecture/rejeu identique ; modification refusée |
| `writes.transition_dates` | Horodatage lors d'un passage vers une valeur déclarée ; dernière date conservée ensuite |
| `config.lifecycle` | Machine à états existante du catalogue, configurée par entité |

Les dates sont des **millisecondes depuis l'époque Unix, UTC**, produites par le serveur. L'interface peut les afficher en ISO 8601 ou dans le fuseau de l'utilisateur. L'horloge système n'est pas monotone : la révision, et non la date, tranche les conflits. Les champs serveur sont retirés du schéma d'entrée et refusés s'ils sont fournis par l'appelant. Une date de transition vaut `null` avant le premier passage concerné.

`input_payloads` permet à chaque outil de déclarer une vue `identity` ou `editable` d'une entité. Types, tableaux, enums et contraintes proviennent du schéma de cette entité. La validation JSON Schema est effectuée côté hôte, même hors MCP ; le mapping de stockage ne tient pas lieu de validateur complet.

## Exemple indépendant de Magic

Le [template notebook](../../templates/backends/notebook/backend.json) expose cinq outils : `put_note`, `get_note`, `save_snapshot`, `get_snapshot`, `search_notes`.

Une note possède une révision, des dates et un état `working`/`reviewed`. Un snapshot est immuable. La recherche assemble BM25 lucivy et dense BGE-M3 local, avec filtres structurés. Cela vérifie que ces fonctions ne dépendent pas des notions de deck ou de publication.

L'hôte garde une base rag3db sur disque et son répertoire de checkpoints ; il sérialise les appels. Le MCP Python utilise le SDK officiel, lance cet hôte et partage le service d'embeddings déjà actif. Ce premier transport est stdio, pas un nouveau endpoint MCP dans `rag3daemon`.

## Problèmes révélés et corrections

1. **Date nullable interprétée comme texte à l'insertion native.** Le paramètre `null` ne portait pas son type scalaire. L'annotation au point d'écriture inclut maintenant les valeurs nulles, en plus des listes et structures.
2. **Recherche dense filtrée : `function LIST does not exist`.** Le backend de recherche avait sa propre conversion des paramètres, qui utilisait le format de débogage Rust pour les tableaux. Il partage désormais la conversion Cypher de la recherche existante.
3. **Substitution de paramètres fragile.** La conversion fait un seul passage, respecte les chaînes déjà présentes et ne réinterprète pas le contenu des valeurs insérées comme des paramètres. L'échappement de la requête imbriquée protège aussi les antislashs.
4. **Entrée MCP trop vague (`record` objet quelconque).** Les vues de payload produisent maintenant les propriétés typées, champs requis, tableaux et enums. Les références locales sont résolues pour rester valides dans le schéma englobant de l'outil.

## Validation réelle

[Test reproductible](../../scripts/test_backend_persistence.py) :

```bash
experiments/mtga/.venv/bin/python extension/rag3weaver/scripts/test_backend_persistence.py
```

Le test crée une base temporaire sur disque, puis :

1. Premier processus : crée une note et un snapshot immuable ; vérifie dates et révision.
2. Deuxième processus : rouvre la même base, relit exactement les données puis recherche en BM25, dense et hybride **sans aucune réingestion préalable**. Vérifie UUID, résultat et absence de réponse partielle. Modifie la note avec révision attendue ; rejette révision périmée, état invalide, date fournie et tentative de remplacement d'un binding. Vérifie qu'une erreur de validation n'a pas modifié l'enregistrement. Refuse l'écrasement du snapshot et accepte son rejeu identique.
3. Troisième processus : vrai client MCP et adaptateur SDK ; vérifie `tools/list`, schémas typés, `tools/call`, contenu structuré et refus d'un outil non exposé.

Service utilisé : BGE-M3 local réel, 1 024 dimensions, daemon `127.0.0.1:7878`. Aucun embedder factice. Ce test vérifie l'usage du service local ; il ne mesure pas à lui seul l'affectation physique du GPU.

Vérifications complémentaires sur ce jalon : **975 tests unitaires passants** hors sandbox (dont contrats MCP dérivés et substitution des paramètres). L'exécution en sandbox avait 969 succès et six échecs d'environnement : cinq ouvertures de sockets interdites et une détection de dépôt dans `/tmp`. La même suite passe entièrement hors sandbox. Les deux tests d’intégration `structured_payloads` passent aussi après la correction du convertisseur partagé (stockage et filtres imbriqués, puis DAG lucivy + dense réel). Les deux scripts Python passent la compilation syntaxique ; `git diff --check` ne signale pas d'erreur.

## Périmètre restant

Ce socle **ne constitue pas encore un workflow complet de publication de deck**. Le template Magic devra déclarer par exemple un deck mutable et des versions immuables, ainsi que les pointeurs vers la version de travail et la dernière version publiée. La promotion de plusieurs enregistrements en une opération atomique n'est pas encore implémentée. Il faut la traiter avant de promettre une publication sûre en cas de panne.

Le modèle souhaité reste : `draft` = dernière version non publiée ; `published` = dernière version publiée, si elle existe. Ces noms, leurs transitions et `published_at` appartiennent au template. La publication locale ne signifie pas import dans Arena.

Le premier hôte expose lecture et écriture unitaire et recherche ; il ne fournit pas encore de commande générique de resynchronisation HTTP, d'historique automatique, de suppression ni de migration versionnée de manifeste. Une erreur d'indexation peut survenir après une écriture : elle est renvoyée explicitement et nécessite inspection, sans prétendre à un rollback global des index. Les appels d'un hôte sont sérialisés ; ce n'est pas une garantie de transactions entre plusieurs hôtes.

Le test couvre fermeture propre et redémarrage, pas une coupure brutale pendant une écriture. Le MCP n'est pas encore enregistré dans la configuration Codex. La collection Magic complète n'a pas encore été ingérée dans ce nouveau backend.
