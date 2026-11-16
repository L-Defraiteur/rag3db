# Préparer le renouvellement des jetons Vertex

Ticket : [un modèle Vertex déclaré garde son jeton d'une heure](../../banc/tickets/2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md).
Préparation après audit de master927941a67, branche `cx-ed-vertex-token-refresh-r1`.
Aucun Cargo, faux serveur lancé, service, cloud ou lecture d'identifiants. La
fenêtre G/Arbre reste prioritaire. Le lot d'ordre stable doit atterrir avant.

## Contrat et frontière

`connect_llm` crée TokenSource, appelle token une seule fois et construit
OpenAiLlm::vertex avec un Bearer fixe. TokenSource conserve déjà son cache,
son renouvellement et invalidate ; le client doit conserver cette capacité,
plutôt que recopier une fois son résultat. Une interface de source de jeton
peut unifier la source réelle et la source factice sans nouveau client HTTP.
La clé d'API fixe reste fixe ; aucun code d'activation ou embedder modifié.

Avant chaque appel authentifié Vertex, obtenir le jeton par la source. Si le
serveur refuse401 AVANT le flux : invalider puis retenter UNE fois avec le
nouveau jeton ; deuxième401 en erreur nommée. Ce renouvellement n'est pas le
retry générique des429/5xx ; il ne doit pas devenir une boucle. Après200 et
ouverture du SSE, ne jamais rejouer, même si le flux casse. Ne pas publier
de jeton dans erreurs, Debug ou journaux.

## Rouge à préparer avant correctif

Faux HTTP loopback privé et borné, source factice au jeton1 puis jeton2 après
invalidation. Premier appel jeton1 accepté ; second appel jeton1 refusé401 ;
retentative jeton2 attendue et acceptée. Attendre trois requêtes, une seule
invalidation et aucun fragment dupliqué. Le code actuel rend une erreur au
second appel. Aucun sleep d'une heure nécessaire.

Le premier témoin de transport peut reproduire la connexion actuelle en
prenant une fois le jeton factice pour le VRAI constructeur vertex existant.
Cette reproduction n'est pas encore une preuve du raccord de déclaration
`connect_llm`. Avant correctif, jouer ce rouge sous le créneau autorisé ; le
correctif raccordera ensuite le même scénario à la source retenue par le
client de production, sans modifier serveur, assertions ou scénario pour le
faire passer. Un témoin complémentaire devra couvrir le raccord déclaré.

Compléter les juges : deuxième401 persistant et borne exacte ; renouvellement
entre appels sans401 ; clé API non invalidée/non réessayée sur401 ; SSE déjà
ouvert non rejoué ; erreur d'obtention du jeton nommée sans valeur secrète.

## Recette future

Après FINI G et script épinglé neuf seulement : sources et sous-module figés,
lib59ac annoncée et résolue en id exact, ancien mode retiré selon annonce.
Rouge exact fauxHTTP sur le code avant fix, sous vrai poste/32G/j8/timeout
convenu ; journal conservé et commit du témoin publié. Puis correctif et même
juge vert, autres témoins de frontière ; passe courante proposée
`SUITE_DU_TEMOIN=e2e_demon_hote SUITES=-`, avec F8/lib entière/NAV de l'outil.
Pas de F300 ni accès cloud pour prouver ce correctif HTTP ; la mesure F300
revient au banc après qualification, avec sa propre décision.

Tous ces juges et recettes restent NON JOUÉS à la préparation. Aucun À TOI
ou délai historique repris, aucune modification de master ni pose d'hôte.

## Témoin de transport écrit, non joué

`src/openai_llm/vertex_refresh_tests.rs`, raccordé sous `cfg(all(test,
feature = "daemon"))` (le faux HTTP utilise la dépendance tiny_http déjà
présente). Deux fonctions :

- `vertex_rejected_token_is_refreshed_once_before_opening_a_stream` : premier
  appel accepté puis401 ; attend réussite, trois requêtes et une invalidation.
  Rouge attendu sur le client figé actuel, mais NON OBSERVÉ à cette étape.
- `fixture_token_source_changes_its_cached_token_only_after_invalidation` :
  contrôle de la source factice, qui ne remplace pas le juge de transport.

Le helper `current_declared_vertex` reproduit explicitement la frontière
actuelle : il prend un jeton factice puis appelle le VRAI constructeur vertex.
Il ne prétend ni retenir déjà cette source dans le produit ni passer par
`connect_llm`. Ce raccord et son témoin complémentaire restent à écrire après
observation du rouge. HTTP sur loopback aléatoire, client borné3s, serveur
borné8s et joint au Drop, pas de sleep d'expiration, pas de service/cloud ou
identifiant réel. Les refus passent par la racine d'état temporaire déjà
imposée par `builtin_templates::state_root` sous cfg(test).

Aucun comportement de production modifié, aucun verdict de compilation ou
d'exécution ; seul `git diff --check` a été contrôlé. Le correctif de
renouvellement reste NON ÉCRIT avant le rouge exécuté.
