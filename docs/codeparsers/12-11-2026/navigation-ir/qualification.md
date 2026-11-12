# Faits qualifiés pour la première navigation des symboles

Ticket commun, porté par Archi pour les façades et codeparsers pour ce soutien :
[exposer la navigation](../../../rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md).
Aucune nouvelle API ou exhaustivité livrée par cette qualification.

## Preuve et portée

Base a783013f1, producteur a730a9ba89ef233423bb7063f85181de26d98ce0.
Sept fichiers/six langages, 50 scopes dans la sélection finale. Exemple existant
voir_json construit une fois (6,57 s, dev), puis sept parse_file sous poste lourd,
j8, borne600 s après acquisition ; seul Rust enrichi est ensuite reparsé sous
poste lourd/borne60 s, sans reconstruction ni répétition des six autres fichiers.
Aucun GPU, service, index de Lucie, ancien dépôt ou pointeur parent modifié.
Target privé unique 569 Mo au relevé. Les parseurs rendent ast_valid=true et
ast_issues=[] dans les sept cas ; cela ne signifie pas extraction exhaustive.

Sources et recette publiées avant les exécutions : caedd236a/7a0e98b49,
complément Rust 3a10a493b, lecteur de preuves fa9d3a711. Artefacts à luciepc :

- `~/.cache/codeparsers-notes/navigation-ir-7a0e98b49/` : source.txt,
  fixtures.sha256, build.log, sept JSON bruts, fin.txt ;
- `~/.cache/codeparsers-notes/navigation-ir-3a10a493b/` : source.txt,
  entrees.sha256 (fixture Rust et binaire), members.rs.json, qualification.json.

Le lecteur publié `qualifier.py` sélectionne les six premiers JSON inchangés et
le complément Rust ; il valide les 50 bornes contre les fichiers UTF-8, puis
conserve les champs bruts et les SHA-256 des entrées. qualification.json :
`db7e572c8e35c65b45062406ce89e14a97b608846da0aeadff131e478182b37b`.
Ce sont des preuves de ScopeFileAnalysis. La projection vers ScopeRecord et les
relations est **relue dans le code**, sans création de base ni nouvelle exécution
de analyze_in_project dans ce lot. Ne pas présenter ce relevé comme un e2e de
navigation ni comme une vérification de fraîcheur de l'index courant.

## Coordonnées : ce que range_containment peut affirmer

`codeparsers/src/scope_extraction/types.rs::ScopeInfo` et
`parallel/parser_worker.rs::finalize` : début à la première colonne de la ligne
de début, fin à la fin de la dernière ligne, octet LF exclu. Ce sont des offsets
UTF-8 dans File, intervalle de lecture `[start_byte,end_byte)`, pas les bornes
exactes des nœuds AST ni des colonnes UTF-16. CR avant LF n'est pas retiré par
finalize ; ce lot ne contient pas de fixture CRLF et ne prétend pas la qualifier.
code.rs copie ces bornes dans ScopeRecord, puis ses propriétés persistables.

Dans members.rs, `mod compact { pub struct Item {} }` produit deux scopes,
compact Namespace et Item Class, **même plage [429,463)** et même ligne22.
Item.parent vaut compact. Une inclusion stricte ne retrouve pas cette arête ;
une inclusion non stricte fabrique un cycle/ordre arbitraire si l'égalité n'est
pas traitée. Garder égalité et parentage non déterminable explicites. Ne pas
remplacer cela par un choix UUID ou un nom supposé unique.

Les noms de parent ne qualifient pas l'identité : trois scopes Rust Class Box
(struct, impl inhérent et impl de trait) ont des plages différentes. Les deux
méthodes C++ size ont toutes deux parent=Box malgré Billing/Other ; les lignes,
signatures et contexte restent nécessaires. Les méthodes Go sont hors de la
plage de leur type : leur appartenance déclarée n'est pas une inclusion.

## Couverture réellement observée

Chaque ligne décrit les fixtures publiées, pas une garantie globale du langage.

| Fixture | Scopes et membres présents | Limite effective pour la façade |
|---|---|---|
| TypeScript, 9 scopes | Policy, Box, méthodes constructeur/size/size/retry ; propriétés limit:number et untyped sans type dans members | Namespace Billing absent ; fonction nested dans retry et child sur même ligne absentes ; constructor/get/set étiquetés Method, pas distingués par ces genres. |
| JavaScript, 6 scopes | Box, bloc de champs, constructeur, deux size et retry ; count et noType Property sans type | Champs présents en amont mais aucun type pour field_types ; nested absent ; pas de catégorie Getter/Setter/Constructor dans ces sorties, tous Method. |
| Python, 8 scopes | Base, Box, Inner, leaf, __init__, size, retry et nested ; typed et annotated Property:int | untyped et self.label absents de members ; les deux membres typés ont line=4 (ligne de classe), pas leurs déclarations5/12. Ne pas utiliser cette position comme une occurrence exacte. |
| Rust, 11 scopes | billing, trois Box, Policy, new/size/retry/retry ; champs count/label ; compact et Item | Appartenance des méthodes à l'impl, pas au scope struct par plage ; nested absent ; compact et Item ont des plages égales. |
| C++ en-tête, 5 scopes | Billing/Other et leurs Box ; count Property:int ; constructeur/prototypes Method et helper Function dans members | Inner et sa déclaration leaf ne deviennent pas des scopes dans cette fixture ; prototypes pas de second ScopeRef/UUID à inventer. |
| C++ définition, 6 scopes | size, leaf, constructeur Box, helper et second size ; parents Box/Inner/Billing | Définitions hors classe : pas de contenant lexical type dans ce fichier ; parent Inner ne garantit pas que le type ait été extrait dans l'en-tête ; deux Box homonymes. |
| Go, 5 scopes | Box Class avec Count:int et label:string ; Policy et Retry déclaré ; Size/Retry Method parent Box | Méthodes en dehors de la plage de Box ; lambda nested absente de cette sortie ; range_containment ne produit pas une liste exhaustive de membres déclarés. |

Les absences font partie du travail du ticket commun de couverture/navigation,
aucun correctif de parseur n'est introduit ici. Une liste partielle ne devient
pas exhaustive parce que ast_valid est vrai. Les méthodes Python imbriquées
peuvent être vues par plages ; leur genre Method ne suffit pas à les annoncer
comme méthodes de la classe extérieure.

## Projection existante relue : pertes et identités

- code.rs::field_types_of : seulement members de genre Property avec type non
  vide ; JSON `[{name,type}]`. Ligne, signature, accès et identité ne sont pas
  conservés. Une propriété non typée TS/JS disparaît à cet étage ; certaines
  affectations Python étaient déjà absentes du producteur.
- code.rs::declarations_of : seulement les genres Method/Function ; JSON
  `[{name,line,signature,kind}]`. Les variantes Getter/Setter/Constructor/Value
  seraient exclues si émises, mais les fixtures JS/TS émettent déjà ces formes
  comme Method : elles sont donc conservées **sans distinction de rôle**.
  Une déclaration n'a pas son UUID propre du seul fait de figurer dans ce JSON.
- HAS_PARENT/PARENT_OF : noms puis résolution par scope/même fichier/contenant
  étroit, avec repli sur le premier homonyme du fichier si aucun ne contient.
  code.rs filtre ensuite les parents aux genres conteneurs ; parent hors fichier
  prend rendez-vous par nom. Ces relations ne certifient pas une parenté lexicale.
- code.rs::fold_lambdas, appelé avant le retour d'analyse, retire les scopes
  lambda ayant un contenant nommé et redirige leurs usages. Une lambda brute
  ou l'existence du genre lambda ne prouve pas un nœud navigable dans l'index.
- Le Plan outline actuel ne sélectionne que UUID/nom/genre/lignes/parent_name/
  signature/fichier/langage, pas bytes/declarations/field_types ni File.hash.
  Une façade fondée sur ces autres données doit les lire explicitement et
  qualifier la fraîcheur séparément, sans réindexation implicite.

## Contrat proposé à Archi

`range_containment` désigne une relation dérivée sur scopes effectivement indexés,
qualifiés dans le même fichier/source/vue ; elle ne se renomme ni héritage ni
appartenance déclarée. Les plages égales, plusieurs parents minimaux possibles,
scopes manquants et index non qualifié restent signalés. Le plus petit intervalle
strict n'est pas une preuve générale du parent lexical AST, puisqu'il est arrondi
aux lignes. Les genres Block/Module des zones ne sont pas des propriétés inventées.

Pour members, distinguer scopes enfants indexés, déclarations disponibles et
métadonnées de champs typés. Rendre source/provenance et couverture partielle,
respecter les identités disponibles, ne pas fabriquer d'UUID, ligne exacte ou type
pour les données absentes. Garder les membres hérités hors de cette première
lecture de contenance et ne pas fusionner par simple nom les trois Box Rust.
La façade et les témoins d'accès réel appartiennent à Archi ; cette qualification
lui fournit les fixtures, limites et faits bruts, sans changement runtime partagé.
