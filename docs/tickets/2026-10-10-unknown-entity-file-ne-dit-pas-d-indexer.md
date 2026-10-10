# « unknown entity: File » ne dit pas ce qui manque — un manifeste sans `workspace.index`

- **État** : ouvert ; correctif en cours par l'arbre principal (branche `unknown-entity`)
- **Gravité** : réponse fausse d'interprétation — un modèle lira un défaut d'outil là où il y a une **déclaration manquante**
- **Atteignable en service** : seulement avec un manifeste qui **ne déclare pas** `workspace.index: "code"`
- **Touche rag3weaver** : oui — toute la surface de code, et le message est le seul indice

> **Corrigé dans sa prémisse le 10 octobre au soir, par l'arbre principal (rag3db-73),
> et il avait raison.** La première version de ce ticket disait « systématiquement sur
> toute base neuve » et « le premier mur de tout usage MCP ». **C'est faux.** Le schéma
> du code (`File`, `Scope`, `Symbol`) est déclaré à l'ouverture dès que le manifeste
> porte `workspace.index: "code"` (`backend.rs:1190-1198`) ; une base **vide** mais
> déclarée ne produit pas cette erreur — `entity_uuid` se calcule, `get` rend `None`, et
> `read_file` passe. Je ne l'ai observée qu'en ayant **moi-même retiré** la déclaration de
> mon manifeste de test, et j'ai pris la condition que j'avais créée pour l'état par
> défaut du produit. Le gabarit livré, lui, la déclare.

## Ce que c'est

Avec un manifeste de code qui **ne déclare pas** `workspace.index: "code"`, le
premier appel d'un outil de code répond :

```
ReadFileNode: unknown entity: File
```

C'est exact et inutilisable. Le remède n'y est pas, et **ce n'est pas celui que
j'avais écrit** : « appelle `index` d'abord » ne sert à rien, puisque `index` n'a
aucun schéma à remplir. Ce qui manque est une **déclaration du manifeste**. Un
modèle qui lit ce message conclut que l'outil est cassé ; au mieux il réessaie,
au pire il abandonne la piste et répond sans avoir cherché.

**Pourquoi ça compte quand même**, prémisse corrigée : c'est un message qui
désigne un symptôme interne (`File`) au lieu de la cause déclarative, et qui
mène son lecteur à une mauvaise action — j'ai moi-même écrit le mauvais remède
dans la première version de ce ticket **à cause de ce message**. Le dépôt soigne
ses refus partout ailleurs : celui du chemin hors workspace dit la règle **et**
la valeur fautive (« path must be relative to the source and without '..' »).

## La recette minimale

```sh
# un manifeste du gabarit code dont on a RETIRÉ workspace.index
rag3weaver-backend mcp --manifest <copie sans workspace.index>
# puis, par le protocole :
{"jsonrpc":"2.0","id":1,"method":"tools/call",
 "params":{"name":"read_file","arguments":{"path":"un/fichier.rs"}}}
```

Mesuré le 10 octobre 2026 avec `read_file` et `list_files`. **Toute** la surface
de code passe par l'entité `File`, donc tous ces outils sont concernés, pas
seulement ceux qui cherchent. Et le contre-cas est mesuré aussi, par le même
témoin : la déclaration remise, une base vide laisse passer `read_file`.

## La formulation attendue

Celle de l'arbre principal, qui remplace la mienne et qui est juste :

> cette base n'a pas le schéma du code (l'entité `File` n'existe pas) : le
> manifeste du backend doit déclarer `workspace.index: "code"`, puis l'outil
> `index` remplit l'index (`estimate` dit ce que ça coûtera).

Elle a ce que la mienne n'avait pas : **la vraie cause** (une déclaration
absente, pas un index vide), **dans le bon ordre** (déclarer, puis indexer), et
le verbe qui permet de décider avant d'indexer. Ma formulation envoyait le
lecteur appeler `index` sur un backend qui n'a pas de schéma à remplir — donc
vers un second échec.

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
