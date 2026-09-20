# Résultats soumis et validations composables

Ce backend illustre une soumission `{result: {name, value}}`, deux contraintes
indépendantes, puis une écriture immutable avec identité calculée et date serveur.
L'auteur du template écrit les scripts Rhai. L'agent reçoit uniquement les outils
déclarés et leurs schémas ; aucun outil d'écriture de script n'est requis.

Le [manifest](backend.json) attache un `harness` à un outil ordinaire :

```json
{"harness": {
  "input_schema": "schemas/input.json",
  "before": [{"graph": "graphs/validate.mmd"}],
  "after": [],
  "on_accept": []
}}
```

Le JSON Schema est vérifié avant les hooks. `before` bloque l'exécution de
l'outil en cas de refus ; `after` valide son résultat, sans annuler ses effets.
`on_accept` transforme les données acceptées et a un statut de livraison séparé.
Ces graphes sont actuellement limités aux calculs Rhai et aux nœuds de validation ;
ils n'exportent pas encore de PDF/Word et n'exécutent pas de commandes système.

Un hook déclare un seul paramètre `context`, de forme :

```json
{"tool":"submit_result","arguments":{"result":{"name":"Essai","value":-3}},"result":null,"data":{}}
```

Les fichiers JSON de faits sont injectés par `data` dans la déclaration du hook,
par exemple `"data":{"policy":"policy.json"}`. Ils sont lus au chargement du
backend, pas choisis par le modèle. `result` contient la sortie de l'outil dans
les hooks `after` et `on_accept`.

## Une règle par nœud

Voir [le graphe](graphs/validate.mmd) et les scripts [positive](hooks/positive.rhai)
et [name](hooks/name.rhai). Un `ValidationRuleNode` définit `script_id` (ou `script`),
`code`, `message`, `path` et éventuellement `severity=warning`. Son script retourne :

```text
#{valid: input.arguments.result.value > 0,
  params: #{minimum: 0, actual: input.arguments.result.value}}
```

Le message `Value must be greater than {minimum}; received {actual}.` est rendu
par substitution de valeurs scalaires, sans évaluation de code. Les paramètres
restent aussi présents dans le diagnostic structuré. Pour une règle appliquée à
plusieurs éléments, retourner `#{checks:[#{valid:..., params:..., path:"/items/0"}, ...]}`.
Chaque règle garde la même responsabilité ; plusieurs violations peuvent en sortir.

Un `ValidationMergeNode` combine ses ports `left` et `right` vers `report`.
Ces fusions se composent sans limite fixe du nombre de règles. Elles conservent
toutes les erreurs et tous les avertissements dans l'ordre des ports. Un échec
Rhai ou un résultat mal formé devient `hook_failed` et n'empêche pas les autres
règles indépendantes de produire leurs diagnostics. Une panne d'un nœud commun
en amont bloque le hook : aucune acceptation n'est inventée.

## Soumission et boucle d'agent

Dans la configuration de chat : `"completion_tool":"submit_result"`.
L'outil doit faire partie de `allowed_tools` si cette liste est configurée.
Le modèle voit le schéma et reçoit l'instruction de corriger puis resoumettre.
L'agent suit le reçu avant troncature d'affichage et expose `task_accepted` dans
le résultat du run : absent/null si aucun contrat n'est configuré, true seulement
après `validation.accepted=true` et `delivery.ok=true`, false sinon.
Ce statut est remis à zéro à chaque demande. Une affirmation de réussite ou
`save_artifact` ne remplace pas la soumission.

La persistance est une composition explicite de `RhaiNode` (préparer l'identité
et le payload) puis `EntityRecordNode`. Une proposition identique est rejouable
sans réécriture. `get_result` relit le résultat par clé, y compris après redémarrage.
La clé identifie le contenu soumis ; elle ne certifie pas une version du contrat
ou des faits. Leur versionnement/audit durable reste à ajouter.

## Exécution et vérification

Depuis la racine du dépôt :

```bash
cargo build --manifest-path extension/rag3weaver/Cargo.toml --features daemon,rag3db-native,openai-llm --bin rag3weaver-backend --bin rag3weaver-chat
python3 extension/rag3weaver/scripts/test_backend_harness.py
```

Le test ouvre une vraie base temporaire et n'appelle ni LLM ni embeddings.
Le manifest d'exemple utilise par défaut le daemon d'embeddings local ; adapter
ses paramètres de connexion et le chemin de l'extension vector à l'installation.

## Exemple métier facultatif

[Le graphe de deck](examples/deck/validate.mmd) sépare neuf contraintes : taille,
terrains, carte connue, face jouable, quantité possédée, maximum par nom, sources
de mana, distribution de mana et coût hybride. Un nœud partagé prépare les
comptages ; chaque script de règle décide indépendamment de sa validité.
Le [fichier de politique](examples/deck/policy.json) déclare 60/24 **pour cet exercice**.
Ce ne sont pas des règles universelles de Magic.

Pour l'utiliser, enregistrer `deck_facts: examples/deck/prepare.rhai` et chaque
script sous son nom de fichier sans extension ; attacher `validate.mmd` avec
`data.cards` pointant sur un snapshot privé et `data.policy` sur `policy.json`.
Utiliser `examples/deck/input.json` comme schéma d'entrée. Le snapshot par ID
contient `name, owned, is_land, basic, primary, token, required, hybrid, sources,
fetch, copy_limit`. Les listes de couleurs utilisent W/U/B/R/G/C.

La mana est un contrôle simplifié des sources potentielles des terrains, pas
une simulation des parties : restrictions, coûts alternatifs, cartes modales,
ramp et réanimation demandent une analyse complémentaire. Les quantités sont
par impression du snapshot ; les alias Arena demandent encore une normalisation.

Rhai s'exécute sans fonctions hôtes d'accès fichiers/réseau/processus, sans modules
ni `eval`, avec budgets de code, opérations, profondeur, collections, JSON et durée.
Cela borne le calcul, sans constituer un sandbox OS ni un plafond mémoire global.
Les permissions des autres outils/agents restent une responsabilité distincte.
