# Lot 2 — le nœud entièrement scripté, et les noms possédés

11 octobre 2026, session « everything declarative » (chantier I). Ce que le
lot pose, la forme d'une déclaration, ce qu'il ne fait pas encore. Le moteur
de script sur lequel il repose est la page `03-lot-1-le-moteur-de-script.md`.

## 1. Les noms possédés, d'abord

Le moteur de graphes nommait ses ports, ses paramètres et ses types en
`&'static str`. Un nom **déclaré** (un sous-graphe, une fiche d'outil lue
d'un fichier, demain un nœud scripté) devait donc être fui par `Box::leak`
pour vivre aussi longtemps que le programme — ce qui interdit le
rechargement à chaud (lot 3) sans fuite à chaque rechargement.

- `PortDef.name`, `ConfigParam.{name, description}` et
  `NodeSchema.{node_type, description}` sont des `Cow<'static, str>` : un nom
  écrit dans le code reste un `&'static` emprunté (`"value".into()`, aucune
  allocation, même pour `Node::inputs()` rappelé à chaque exécution), un nom
  déclaré est possédé.
- `NodeFactory::node_type()` rend `&str` (emprunté à la fabrique) ; le
  registre est indexé par `String`.
- `Node::node_type()` reste `&'static str` : c'est le trait que la session
  recherche change pour l'exécution asynchrone ; il passera en dernier, après
  sa fusion, relu par elle.

**Ce que la mesure a trouvé en route** : la fuite n'attendait pas le
rechargement. `GraphNode::from_definition` fuyait le nom de chaque port libre
**à chaque création de nœud** — donc à chaque instanciation d'un graphe qui
contient un sous-graphe, en service — et `graph_tool.rs` fuyait le nom et la
description de chaque paramètre d'un outil lu d'un fichier. Le témoin
`tests/owned_names.rs` (un allocateur qui compte les octets vivants, dans sa
propre cible de test) mesurait +193 000 octets pour mille sous-graphes
rebâtis et +72 000 pour mille nœuds créés ; il est stable après.

Le changement touche ~550 littéraux dans 28 fichiers : il est fait par trois
scripts rejouables (`owned_names.py` pour les littéraux, `owned_names_types.py`
pour les types, `owned_names_fixes.py` pour les usages), pour se rejouer sur
la tête de master plutôt que de résoudre des conflits à la main.

## 2. Déclarer un nœud

Deux fichiers côte à côte dans le dossier `nodes/` d'un backend :
`price_with_tax.node.json` et `price_with_tax.ts` (décidé le 10 oct. : la
déclaration est une donnée, vérifiable sans exécuter ; le dossier est
découvert, il se copie tel quel comme un paquet).

```json
{
  "name": "PriceWithTax",
  "description": "adds the tax to a product's price",
  "script": "price_with_tax.ts",
  "inputs":  { "product": { "schema": { "type": "object", "required": ["name", "price"] } } },
  "outputs": { "priced":  { "schema": { "type": "object", "required": ["with_tax"] } } },
  "config":  { "rate": { "schema": { "type": "number" }, "default": 0.25, "description": "the tax rate" } }
}
```

```ts
function run({ inputs, config }: { inputs: { product: { price: number } }; config: { rate: number } }) {
  return { priced: { ...inputs.product, with_tax: inputs.product.price * (1 + config.rate) } };
}
```

- `script` est relatif au fichier de déclaration, jamais absolu ni hors de
  son dossier ; `language` se déduit de l'extension (`.ts`, `.js`, `.rhai`)
  s'il n'est pas écrit ; une clé inconnue est refusée en la nommant.
- Chaque port porte un **JSON Schema** ; `required` vaut vrai par défaut pour
  un port, faux pour un paramètre.
- En rhai, l'expression lit `input.inputs` et `input.config` et rend
  `#{priced: …}`.

## 3. Ce que le système en fait

- **À la déclaration** (`ScriptedNodeFactory::new`, `discover`) : le nom est
  un identifiant, au moins un port de sortie, aucun port ni paramètre en
  double, chaque schéma se compile, chaque défaut respecte son schéma, et le
  script se **prépare** — une erreur de syntaxe est refusée ici, avec sa
  ligne. Deux déclarations du même type dans un dossier sont refusées. Toute
  erreur nomme le fichier, le nœud et la chose à corriger.
- **À la création d'un nœud** : la configuration est vérifiée (paramètre
  inconnu, manquant, hors schéma), les défauts posés.
- **À l'exécution** : chaque entrée vient de son **port** et est validée
  contre son schéma **avant** l'appel ; le script reçoit
  `{ inputs, config }` ; la sortie doit être un objet dont chaque clé est un
  port de sortie, chacune validée **après** l'appel. C'est là que « les ports
  vérifient » devient vrai : TypeScript n'est jamais vérifié.
- Le script est préparé une fois, à la déclaration ; tous les nœuds de ce
  type, dans tous les graphes, le partagent. Le nœud est exécuté et
  enregistré nœud par nœud comme un nœud fourni (rapport d'exécution, ports).
- **Une seule forme pour un nombre** : à la sortie, un flottant entier
  (dans ±2⁵³) devient un entier — rhai rend `125.0` là où JavaScript rend
  `125` ; la sortie d'un nœud ne dépend pas de son langage (tranché par la
  session, réversible).

## 4. Les témoins

`src/dataflow/scripted_node/tests.rs` : rouges sur l'exécution avec un
bouchon (5), puis 11 sur 11.

- un nœud déclaré, posé dans un graphe derrière un nœud fourni, s'exécute et
  figure au rapport d'exécution ;
- son schéma est celui de sa déclaration ; ses entrées viennent des ports ;
- le même nœud sert deux graphes (avec deux configurations) ;
- le même nœud en rhai et en TypeScript rend la même sortie ;
- déclaration fautive (sans sortie, nom, port en double, schéma invalide,
  défaut hors schéma, script en erreur avec sa ligne, langage inconnu) :
  refusée en la nommant ;
- entrée ou sortie hors schéma, sortie inconnue ou manquante : refusées à la
  frontière, avec le port et le chemin ;
- configuration hors schéma ou inconnue : refusée à la création ;
- un dossier de déclarations se lit et ses nœuds s'exécutent ; un dossier
  absent ne déclare rien ; chemin hors du dossier, clé inconnue, langage
  introuvable, type déclaré deux fois : refusés en nommant le fichier.

## 5. Ce qui n'est pas dans ce lot

- **Le branchement dans un backend** (le dossier `nodes/` découvert par
  `PreparedBackend::load`, les nœuds scriptés admis dans les listes blanches
  des outils et des réactions de `backend_code.rs`) : au lot 4, avec le
  backend jouet qui l'exercera de bout en bout. Un nœud scripté est pur (ni
  fichiers, ni réseau, ni commande) : il entrera dans toutes les politiques.
- **Ce que le script peut appeler** (requêter par le dialecte, journaliser) :
  rien encore ; à donner quand un nœud du jouet en aura besoin.
- ~~`Node::node_type()` possédé~~ **fait le 11 octobre** (trouvé par la
  session recherche en relisant le nœud) : `Node::node_type()` rendait
  `ScriptedNode` pour tout type scripté, et `to_definition` — donc un point
  de reprise ou un aller-retour mermaid — écrivait un type que le registre
  ne retrouvait pas ; même trou pour un sous-graphe de `GraphNodeFactory`
  (« GraphNode »). Le trait emprunte désormais le nom à l'instance ; un
  nœud scripté rend son nom déclaré, un sous-graphe de fabrique son type
  déclaré et la configuration reçue. **Reste, préexistant** : un sous-graphe
  monté à la main (`GraphNode::from_definition` sans fabrique) garde le type
  « GraphNode », qui n'est pas un type enregistré — l'aller-retour vaut pour
  les types *déclarés* ; un montage à la main ne se restaure pas par le
  registre.
