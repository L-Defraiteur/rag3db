# « unknown entity: File » ne dit pas d'indexer — le premier mur de tout usage MCP

- **État** : ouvert
- **Gravité** : réponse fausse d'interprétation — un modèle lira un défaut d'outil là où il y a un ordre à suivre
- **Atteignable en service** : oui, **systématiquement** : toute base neuve est dans cet état
- **Touche rag3weaver** : oui, et c'est la première chose qu'un utilisateur rencontre

## Ce que c'est

Sur une base **neuve** servie par le gabarit `code`, le premier appel d'un outil
de code répond :

```
ReadFileNode: unknown entity: File
```

C'est exact et inutilisable. Le remède n'y est pas, et il est simple : **la base
n'est pas encore indexée, il faut appeler l'outil `index` d'abord.** Un modèle
qui lit ce message conclut que l'outil est cassé ; au mieux il réessaie, au pire
il abandonne la piste et répond sans avoir cherché.

**Pourquoi ça compte plus qu'un message maladroit** : c'est le **premier** mur de
tout usage MCP. Une base neuve est l'état de départ de chaque nouvelle
installation — donc tout le monde le rencontre, une fois, avant d'avoir rien
compris au produit. Et le dépôt soigne ses refus partout ailleurs : celui du
chemin hors workspace, par comparaison, dit la règle **et** la valeur fautive
(« path must be relative to the source and without '..': ../../../etc/passwd »).

## La recette minimale

```sh
# un manifeste du gabarit code sur une base qui n'existe pas encore
rag3weaver-backend mcp --manifest <copie de templates/backends/code/backend.json>
# puis, par le protocole :
{"jsonrpc":"2.0","id":1,"method":"tools/call",
 "params":{"name":"read_file","arguments":{"path":"un/fichier.rs"}}}
```

Mesuré le 10 octobre 2026 avec `read_file` et `list_files`. **Toute** la surface
de code passe par l'entité `File`, donc tous ces outils sont concernés, pas
seulement ceux qui cherchent.

## La formulation attendue

> `read_file` : cette base n'est pas encore indexée (l'entité `File` n'existe
> pas). Appelle l'outil `index` d'abord ; `estimate` dit ce que ça coûtera.

Trois choses qu'elle a et que l'actuelle n'a pas : **ce qui manque** en clair
plutôt qu'en nom interne, **le verbe à appeler**, et **le verbe qui permet de
décider avant** — puisque l'indexation d'un gros dépôt n'est pas gratuite et que
`index` refuse de lui-même au-delà d'un seuil sans confirmation.

## Le chemin du code

- `extension/rag3weaver/src/code_tools.rs:425-430` — `entity_uuid(FILE, …)` puis
  `catalog.get(FILE, &uuid)`, dont l'erreur remonte telle quelle par
  `.map_err(|e| e.to_string())` ;
- l'erreur vient de `CatalogError::UnknownEntity` (`src/catalog.rs:114`), levée
  par le catalogue qui ne sait pas, et ne peut pas savoir, qu'il parle à un
  outil de code dont le remède est `index` ;
- les appelants : `ReadFileNode` (`src/dataflow/code_nodes.rs:219`) et les
  autres nœuds de la surface de code, qui préfixent par leur nom
  (`ReadFileNode: …`) sans enrichir le fond.

**Où le corriger, et une mise en garde** : pas dans `CatalogError`. Le catalogue
a raison de dire « unknown entity » — c'est vrai, et c'est son vocabulaire. Le
remède appartient à la **surface de code**, qui sait que `File` vient de
`register_code_schema` et que le verbe est `index`. Donc dans `code_tools.rs`,
en reconnaissant l'erreur du catalogue sur `FILE` et en la remplaçant par la
formulation ci-dessus. Enrichir `CatalogError` ferait parler le catalogue d'un
outil qu'il ne connaît pas.

## Le témoin

Aucun aujourd'hui, et l'absence est assumée : `extension/rag3weaver/tests/e2e_mcp_stdio.rs`
**documente** l'état dans son en-tête mais ne l'affirme pas, parce qu'il a
besoin d'une base indexée pour le reste de son travail. Le témoin à écrire est
simple et n'a pas besoin de MCP : ouvrir un backend de code sur une base neuve,
appeler `read_file`, et exiger que le message **nomme `index`**. À écrire avec
le correctif, dans le même commit.

## Ce qu'il faut pour le fermer

1. le message ci-dessus, ou mieux, rendu par la surface de code ;
2. son témoin, qui vérifie la présence du mot `index` et non le texte entier —
   un test qui diffe un message complet se casse à chaque reformulation ;
3. retirer la ligne de secours du README MCP
   (`extension/rag3weaver/templates/mcp/README.md`, section « si ça ne marche
   pas »), qui documente ce défaut en attendant.

## Et une question plus large, qu'il faudra trancher un jour

Faut-il qu'un outil de code **propose** d'indexer, voire indexe tout seul au
premier appel ? Non à mon avis : indexer un gros dépôt coûte des minutes et de
la carte, et `index` refuse déjà de lui-même au-delà d'un seuil sans
confirmation — précisément pour que ce soit une décision. Un bon message suffit,
et c'est ce que ce ticket demande. Mais la question appartient à Lucie, pas à ce
ticket.
