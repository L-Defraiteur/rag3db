# Explorer les symboles et leur hiérarchie sans lire tout le code

12 novembre 2026. Demande de Lucie, à partir d'un exemple de recherche, références
et réponse sourcée : permettre à l'agent de parcourir les arbres de code par
petites vues. **Conception proposée ; aucune nouvelle API livrée par cette note.**
[Ticket de capacité](../../rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md).

## Le parcours souhaité

L'agent trouve un fichier ou un symbole, en demande la structure, descend vers
les membres intéressants, puis lit seulement le corps nécessaire. Il peut
remonter vers un contenant ou parcourir un sous-arbre sans relire la classe.
Les références d'appel et leurs degrés de résolution restent un autre axe.

```text
list_symbols("billing/charge/retry.py", depth=2)
retry.py
└─ RetryPolicy                         class
   └─ schedule_retry(…)                method

list_members(RetryPolicy)
get_parents(RetryPolicy.schedule_retry, depth=1)
get_children(RetryPolicy, depth=1)
```

Ces formes sont des propositions d'interface. Le symbole donné par son nom doit
d'abord être qualifié par le contexte ; aucun homonyme n'est choisi au hasard.

## Ce que nous avons déjà

Relecture du socle intégré au 12 novembre :

- `outline(path, source, limit, format)` fournit les scopes d'un fichier depuis
  l'index, sans ouvrir son corps. Son Plan déclaratif trie lignes, étendue et UUID,
  rend nom, genre, signature, parent et positions, avec total et provenance.
  Le relais `atelier_outline.mmd` l'expose au contexte explicite du dépôt attaché.
- Le rendu de recherche qualifie déjà le nom par `parent_name` : par exemple
  `PortValue::take` ou `Session.run`. Il sait regrouper plusieurs résultats d'un
  même contenant. Un titre qualifié ne constitue pas à lui seul un arbre navigable.
- Le graphe porte `HAS_PARENT` et `PARENT_OF`. Leur qualification doit être relue
  avant de les présenter comme une hiérarchie purement lexicale : le code traite
  aussi des parents de conteneurs résolus, notamment hors du fichier courant.
- `ScopeRef` distingue les scopes indexés, qui possèdent éventuellement un UUID,
  des scopes analysés à la volée sans identité de graphe. Une analyse locale ne
  doit donc pas fabriquer des voisins indexés.
- `field_types` ne conserve actuellement que les propriétés avec un type connu.
  Les déclarations extraites des membres portent aussi méthodes/fonctions.
  Cela ne prouve pas une liste exhaustive de tous les membres de tout langage.

Points source : `templates/tools/{outline,atelier_outline}.mmd`,
`templates/backends/code/plans/outline.json`, `src/code_tools.rs::ScopeRef`,
`src/code.rs::{field_types_of,declarations_of}` et
`src/dataflow/render_nodes.rs::build_view`.

## Surfaces proposées, même modèle de navigation

| Interface | Sens proposé |
|---|---|
| `list_symbols(file, depth=1, …)` | Vue des symboles propres au fichier, en arbre ou JSON. Une racine de fichier est une racine de présentation, pas un symbole inventé. `depth=2` permet par exemple fichier → classe → méthodes. |
| `list_members(symbol, kinds=all, …)` | Membres directs déclarés d'une classe/type/contenant : méthodes, champs, propriétés, types imbriqués quand ils sont réellement disponibles. `kinds=method` couvre l'idée initiale `list_methods` sans dupliquer le mécanisme. |
| `get_parents(symbol, depth=1, …)` | Contenants successifs, bornés et qualifiés ; pas les classes de base implicites. |
| `get_children(symbol, depth=1, …)` | Sous-arbre des enfants de la relation de contenance choisie ; pas les fonctions appelées ni les sous-types par défaut. |
| `grep(…, symbol_hierarchy_depth=0)` | Résultats textuels habituels ; une option positive ajoute le symbole contenant le match et sa chaîne bornée de parents lorsque l'index peut la qualifier. |

Le premier geste doit être d'enrichir ou composer `outline` et les mécanismes
relationnels existants. Il faut décider si `list_symbols` devient un alias de
présentation ou le nom public choisi, plutôt que proposer à l'agent deux outils
qui se recouvrent sans différence claire. Plans, lectures de vue et parcours de
graphe communs portent les garanties ; le client IDE et le client terminal ne
reconstruisent pas chacun leur logique métier.

### Identité et sens du parent

Une référence doit désigner une déclaration/occurrence dans sa source et sa vue,
pas seulement la chaîne `RetryPolicy`. Utiliser l'identité indexée disponible et
le contexte explicite existant ; un nom qualifié peut résoudre vers plusieurs
candidats, qui doivent alors être rendus avec leurs chemins et signatures.

Conserver trois notions distinctes : contenance syntaxique dans le fichier,
appartenance déclarée à un type et héritage/implémentation. Elles coïncident dans
des cas simples, mais une méthode C++ définie hors classe en est un contre-exemple.
Un `HAS_PARENT` résolu ou un `parent_name` d'affichage ne doit pas être renommé
preuve lexicale sans qualification. Les membres hérités éventuels sont une
extension explicitement demandée et marquée, avec provenance et ambiguïtés.

### Profondeur, volume et résultat incomplet

Pour les parcours d'arbre, la profondeur compte les arêtes depuis la racine
fournie : 0 donne cette racine, 1 ses voisins directs, 2 un niveau de plus.
Les valeurs par défaut des nouvelles fonctions restent proposées ; elles devront
être arrêtées en revue. Limite de nœuds et budget de sortie bornent également
le résultat. Un arbre tronqué doit l'annoncer, avec total lorsqu'il est connu.

Le grep garde **`symbol_hierarchy_depth=0` par défaut**, comme proposé par Lucie :
aucune nouvelle lecture d'index pour cet enrichissement. Pour une valeur positive,
le rattachement trouve d'abord le scope contenant le match, puis remonte au plus
ce nombre d'arêtes. Le résultat explicite le scope de départ et les ancêtres ;
il ne surcharge pas le texte de leurs corps. Cette convention doit être visible
dans le schéma de l'outil pour éviter de compter différemment dans chaque client.

Les sorties structurées portent au minimum identité/référence, nom, genre,
signature, chemin et positions disponibles, lien de parent avec sa qualification,
provenance et état de couverture/troncature. La vue compacte affiche ces mêmes
faits en arbre ; elle ne constitue pas une deuxième représentation d'autorité.

### Couverture des membres et greps enrichis

`list_members` doit combiner seulement les faits réellement extraits : scopes
enfants et déclarations de membres existantes, avec une identité et une position
quand elles sont établies. Un champ sans type ne doit pas disparaître d'une liste
annoncée exhaustive parce que `field_types` ne le conserve pas. Une déclaration
seule, une macro ou une propriété non extraite sont des limites nommées ; ce n'est
pas la preuve que le membre n'existe pas.

Pour le grep, convertir correctement les positions textuelles vers les positions
des scopes, y compris UTF-8, imbrications et matches multilignes. L'index doit
correspondre au fichier effectivement cherché : après déplacement de lignes dans
un worktree, une vieille plage ne permet pas d'affirmer le bon contenant.
Sans index, ou si l'état ne peut pas être qualifié, le grep conserve ses matches
et indique que la hiérarchie demandée est indisponible/périmée. Il ne réindexe pas
implicitement et n'invente pas un parent. Une analyse du fichier à la volée reste
une autre provenance, avec son contrat explicite.

La vue, les cellules, les masques et la source doivent rester les mêmes entre
recherche et navigation. Une référence à une branche ne remplace pas la provenance
d'un fichier modifié dans un worktree. Ce besoin compose les contrats de vues du
projet ; il ne livre pas à lui seul l'activation de la pyramide de couches.

## Inspiration du parcours montré

L'exemple fourni par Lucie rend aussi intéressants les rôles de références et
leur résolution affichée, la distinction recherche sémantique/grep exact, les
citations par dépôt/révision et les questions de suivi. Dans `refs`,
`in ChargeService.capture` situe le site d'appel ; dans une recherche de symboles,
le contenant situe la définition trouvée. Le parent d'une conversation de suivi
est encore une autre identité : les noms d'API ne doivent pas les confondre.

## Suite avant implémentation

Qualifier les relations actuelles sur quelques langages et les cas hors classe,
définir la référence de symbole et les sorties communes, puis proposer une
première façade déclarative sur les données disponibles. Les témoins doivent
couvrir homonymes, surcharge, membre sans type, déclaration sans corps, langage
à couverture partielle, index absent/périmé, worktree modifié, profondeur 0/1/2,
cycles/limites et troncature. Le vrai agent doit pouvoir appeler ces outils et
parcourir un arbre sans recevoir les corps complets. Aucun modèle d'inférence ni
nouvel index n'est nécessaire pour la seule navigation des faits existants.
