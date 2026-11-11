# Exports par défaut : bilan de décision et contrat de provenance

## Décision étayée au 11 novembre

Le lot d96ed3264, avec codeparsers a730a9b, conserve les invariants couverts
par **8 témoins e2e et 286 tests codeparsers effectivement passés**. Le complément
sur 23 questions source établit **six nouvelles liaisons strictement justes et
attribuées**. Il ne permet ni un score global de précision, ni une affirmation
que les 17 arêtes justes retirées représentent 17 usages perdus.

Le lot change aussi le contrat de navigation : un import local peut conduire à
la définition ou valeur exportée plutôt qu'à la déclaration de son alias local.
Sur cinq usages appariés, cette destination reste fausse selon le prédicat LSP
strict conservé. Ce désaccord doit être décidé comme contrat produit ; il ne
doit pas être effacé du verdict par un changement opportuniste de métrique.

La passe complète d'intégration reste à jouer sur la base convenue après la
fenêtre recherche R5. Aucun tour de push master n'est acquis par ces résultats.

## Preuves figées

Seconde A/B : A=aece4b9f1/c660893 ; B=d96ed3264/a730a9b. Source commune
a72acfeef, lib native épinglée 29a7bc52a, modèle granite-278m dense768 privé
sur 7882. Sources, oracle et gabarits sont contrôlés par le lanceur suivi
`outils/cx-mesure-default-exports-v2.sh`. Les deux index sont terminés ; leurs
hôtes ont été arrêtés et le service rendu à l'optimiseur, sans arrêt du service.

Preuves sur luciepc, conservées :
`~/.cache/codeparsers-notes/require-default-v2-d96ed3264/`.
Le complément est dans `complement-23/`, avec `provenance.json` terminé,
`questions.jsonl`, `A.jsonl`, `B.jsonl`, RPC et positions intégrales.
Son manifeste suivi `outils/cas-lsp-23-d96ed3264.json` a le SHA-256
`0d02f7959f9407c22042914cda6cd6a9708acbf35ab37b2f6a2761118a28ebc4`.
L'oracle historique, inchangé, a le SHA-256
`17b5f8f4158df4250bf3fe075843f8608a5276157cc09adffb6444a2c8109a0d`.

Les index comptent respectivement 137848 et 137783 chunks, et 128430 et
128971 CONSUMES. Les durées 1097 et 1092 secondes ne démontrent aucun gain
de vitesse : charge partagée, sans campagne de performance.

Sur les 298 mentions du jeu historique : A donne 324 arêtes, dont 279 justes,
10 fausses, 17 muettes, 12 liaisons d'import et 6 déclarations ; B donne 307
arêtes, dont 282 justes, 2 fausses, 17 muettes et 6 déclarations.
Les 464 ajouts se répartissent en 36 justes, 4 réexportations, 1 faux et
423 muets ; les 82 retraits en 17 justes, 14 faux et 51 liaisons d'import.
Ces catégories et populations différentes interdisent un ratio global comparé.

Le lecteur des interrogations distingue 422 ajouts sans question envoyée
(415 liaisons et 7 imports), 41 avec réponse et un interrogé sans réponse.
Les 82 retraits ont été interrogés. Le complément interroge réellement les
23 positions source figées : toutes ont une réponse definition ; les 23 RPC
`textDocument/declaration` échouent avec -32601, méthode non gérée. Ces erreurs
sont conservées, sans être transformées en absence de réponse definition.

## Gains et limites attribués

Les six nouveaux liens strictement justes sont FS à
`tools/wasm/src_js/index.js:12` vers la zone fs.js:172–174, et Rag3dbWasm dans
les déclarations de sync/connection.js:7, sync/database.js:7, sync/index.js:7,
sync/prepared_statement.js:8 et sync/query_result.js:6 vers rag3dbInstance,
sync/rag3db.js:42. A ne portait pas de CONSUMES sur ces déclarations.

Les cinq usages wasm/index.js:73–77 conduisent dans B aux quatre classes
exportées et à la zone FS. Le LSP strict choisit leurs alias locaux : les cinq
liens B sont donc strictement faux, les liens locaux A strictement justes.
C'est la différence de navigation décrite ci-dessus, pas une preuve indépendante
que ces nouvelles destinations sont le comportement produit souhaité.

Huit absences apparentes sont expliquées par l'agrégation des positions :
nodejs/index.js:9–12 conserve les quatre dépendances dans le scope fichier,
mais leurs lignes représentatives sont 3–6 ; sync/index.js:66–69 les conserve
avec les lignes représentatives 8–11. Les plages de scopes et les cibles ont
été relues. Aucun rappel perdu n'est établi pour ces huit usages.

Reader à sourceReader:12 a un lien B strictement juste vers la zone
collect.cjs:1–4, voisin d'une arête processName qui ne peut être jugée avec
la question Reader. Reader:27 et CSSgrammar.js:12 restent sans attribution
suffisante. Une jointure sur la seule ligne ne démontre pas leur disparition.
Les fichiers `source-scope-candidates-A.jsonl` et `source-scope-candidates-B.jsonl`
conservent les candidates par plage de scope, sans nouveau LSP ni sélection
fondée sur la réponse. Les noms/plages ne remplacent pas une clé de scope.
Le faux platform vers un homonyme demeure un défaut connu, ticket séparé,
hors du changement de politique des imports externes.

## Où existe la position de mention, où elle disparaît

Références ci-dessous : producteur codeparsers a730a9b et consommateur d96ed3264.

| Étage | Information disponible | Limite |
|---|---|---|
| `scope_extraction/types.rs`, IdentifierReference | identifiant source, ligne, colonne optionnelle, contexte, origine d'import | La colonne des extracteurs AST/regex est un décalage en octets ; certains chemins n'en fournissent pas. |
| UsageSite et `relationship_resolution/relationship_resolver.rs`, site_of | nature d'usage et ligne | Identifiant et colonne sont perdus. merge_by_target regroupe les sites par source/cible/type. |
| `extension/rag3weaver/src/code.rs`, CodeRelation | clés de scopes et tous les UsageSite | Le scope propriétaire n'est pas l'identité d'une mention. fold_lambdas peut remonter le propriétaire tout en gardant les sites. |
| CodeAnalysis.pending_sites / pending_defaults | nom local, clé propriétaire et sites ; correspondance avec la cible par défaut | Exploitables avant émission des relations ; ils ne sont pas une provenance persistée des CONSUMES. |
| code.rs:540, usage_properties | usage principal, ensemble de natures, minimum des lignes | `line=min(sites.line)` ; les autres occurrences disparaissent. `usages` contient des natures, pas des positions. |
| `extension/rag3weaver/src/records.rs`, RelationRecord ; catalogue et matérialisation | propriétés génériques puis usage copié vers CONSUMES/CONSUMED_BY | Aucun mécanisme ne reconstitue les données supprimées en amont. |
| Sonde historique | nom de cible, ligne représentative, propriétaire | Chercher le nom de cible sur cette ligne peut ne produire aucune question, ou interroger une autre occurrence. |

Le champ RelationshipMetadata.symbol n'est pas une solution générale : il est
renseigné pour USES_LIBRARY, avec le nom importé, pas pour chaque alias et
occurrence source. La présence de `Serialize` sur CodeAnalysis ne prouve pas
non plus sa récupération dans les checkpoints :
`dataflow/checkpoint.rs:59`, port_value_to_checkpoint, ne traite pas ce type ;
son repli stocke Any sans données. Aucun instantané complet n'est présumé sauvé.

## Ce qui est récupérable sans nouveau schéma

La sonde peut lire les clés et bornes des scopes existants (`Scope.key`,
file_path, start_line/end_line, start_byte/end_byte). Cela améliore l'attribution
au propriétaire, mais ne restitue ni toutes les mentions ni leur token.
Le graphe seul ne permet pas d'inverser min, regroupement et renommage d'alias.

Une réanalyse du snapshot immuable avec les versions et options exactes peut
exporter les IdentifierReference, puis les sites et correspondances de
CodeAnalysis avant usage_properties. Un artefact diagnostique peut les joindre
aux clés de scopes existantes sans nouveau schéma de base. Il faut valider le
token dans la source et convertir les colonnes en octets vers UTF-16 avant une
question LSP ; les positions ambiguës ou absentes doivent rester explicites.
Cette récupération n'a pas encore été exécutée : disponibilité du producteur
et conservation effective d'une preuve sont deux affirmations distinctes.

Prochaine étape bornée proposée : capturer cette provenance sur les mêmes
23 questions et réutiliser les réponses LSP déjà conservées, sans nouvelle
campagne ni sélection de cible au moyen de ces réponses. Décider ensuite le
contrat alias local/définition exportée et les cas Reader/CSS réellement
attribuables. Une provenance persistée de chaque occurrence serait un contrat
séparé de sérialisation et de schéma, à cadrer avant toute implémentation.

Aucun import_binding, règle portant un nom de cas, schéma nouveau ou changement
de l'oracle général n'a été introduit pour ce bilan.
