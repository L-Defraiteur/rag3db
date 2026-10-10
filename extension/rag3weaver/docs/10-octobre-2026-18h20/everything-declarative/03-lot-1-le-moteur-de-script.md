# Lot 1 — le moteur de script générique, rhai et TypeScript

10 octobre 2026, session « everything declarative » (chantier I). Ce que le
lot pose, comment s'en servir, ce qu'il ne fait pas. La page de conception est
`../orchestration/03-le-proto-tout-declaratif.md`, §3.

## 1. Ce que c'est

`src/script/` : une seule interface, que chaque langage remplit.

```rust
pub trait ScriptEngine {
    fn prepare(&self, source: &str, limits: &ScriptLimits)
        -> Result<Arc<dyn PreparedScript>, ScriptError>;
}
pub trait PreparedScript {
    fn call(&self, input: &Value, limits: &ScriptLimits) -> Result<Value, ScriptError>;
}
script::prepare("typescript", source, &limits)?.call(&input, &limits)?
```

- **Préparer** : lire, vérifier les bornes, effacer les types, compiler (rhai)
  ou évaluer une première fois dans un moteur jetable (TypeScript,
  JavaScript : la syntaxe, le code de premier niveau et la présence de `run`
  sont vérifiés à la préparation, pas au premier appel).
- **Appeler** : une valeur JSON entre, une valeur JSON sort ; autant d'appels
  qu'on veut sur le même script préparé.
- Les langages, par le nom qu'une déclaration leur donnera : `typescript`
  (le langage du produit), `javascript` (du TypeScript sans types), `rhai`
  (l'historique). Un nom inconnu est refusé avec la liste des connus.

## 2. Écrire un script

TypeScript ou JavaScript : le script définit `function run(input)` et rend la
sortie.

```ts
interface Input { a: number; b: number; tags: string[] }
function run(input: Input): { total: number } {
  return { total: input.a + input.b };
}
```

rhai : inchangé, une expression qui lit la constante `input`.

**Les types ne sont jamais vérifiés.** Ils sont effacés au chargement
(`swc_ts_fast_strip`, le retrait de types de Node, en mode `StripOnly`) ; ce
sont les **ports du nœud** qui vérifient entrées et sorties à la frontière
(lot 2). Le TypeScript accepté est le TypeScript « effaçable » (l'option
`erasableSyntaxOnly` de TypeScript 5.8) : `enum`, `namespace` porteur de
code, propriétés de paramètre de constructeur et `<T>expr` sont refusés en
les nommant, avec leur ligne. Les annotations deviennent des blancs : lignes
et colonnes du JavaScript exécuté sont celles du source écrit, et une erreur
levée à l'exécution porte la ligne écrite, sans carte de source.

## 3. Ce qu'un script peut faire

Rien qu'on ne lui donne. Le moteur JavaScript est QuickJS (`rquickjs` 0.14,
qui embarque quickjs-ng 0.16.2) **sans `quickjs-libc`** : aucun module de
fichiers ni de système n'existe dans le binaire. `require`, `fetch`,
`process`, `std`, `os`, `Deno`, `setTimeout`… et `import` (statique ou
dynamique) sont refusés en les nommant (`ScriptErrorKind::Forbidden(nom)`).
rhai garde ses deux fonctions (`json_string`, `content_hash`). Rien
d'asynchrone : un `run` qui rend une promesse est refusé.

Les fonctions de l'hôte (requêter par le dialecte, journaliser) viendront avec
le nœud scripté (lot 2), par ce qu'on lui donne.

## 4. Les bornes

`ScriptLimits` (l'ancien `RhaiLimits`, qui en est un alias) : temps
(`timeout_ms`, 1 s), taille du source (64 Kio), de l'entrée et de la sortie
(16 Mio de JSON), et selon le langage :

| Borne | rhai | TypeScript / JavaScript |
|---|---|---|
| échéance | `on_progress` | gestionnaire d'interruption de QuickJS : exception que le script ne peut pas attraper |
| opérations | `operations` | — (le temps borne) |
| mémoire | tailles de chaînes et de collections | `memory_bytes` (64 Mio), plafond du tas de QuickJS ; un `Runtime` neuf par appel |
| pile | 32 niveaux d'appel | 256 Kio |

Un dépassement se dit par son genre : `Timeout`, `Memory`, `Limit`.

## 5. Ce qui n'a pas changé

`harness::evaluate` appelle le branchement rhai ; ses trois appelants
(`RhaiNode`, `ValidationRuleNode`, `AddRefNode`) et leurs messages d'erreur
sont identiques, mot pour mot (témoin `rhai_messages_are_unchanged`). La
seule différence : le script rhai est compilé une fois à la préparation, puis
évalué — même moteur, mêmes réglages.

## 6. Les témoins (`src/script/tests.rs`)

Rouges avec des bouchons, puis verts : 18 sur 18 (`script::`, `harness::`).

- le même script en rhai, JavaScript et TypeScript rend la même sortie ;
- TypeScript annoté (interface sur plusieurs lignes, `import type`,
  générique, `as`) = le même en JavaScript ;
- une boucle infinie est arrêtée par l'échéance (les trois langages, en
  moins de 3 s pour 200 ms), aussi au premier niveau, dès la préparation ;
- une bombe mémoire est arrêtée par le plafond ;
- `require`, `fetch`, `std`, `os`, `process`, `read_file` (rhai), `import`
  statique et dynamique : refusés en les nommant ;
- `enum`, `namespace`, propriété de paramètre : refusés en les nommant, à leur
  ligne ;
- la ligne d'une erreur d'exécution et d'une erreur de syntaxe est la ligne
  écrite, après effacement des types ;
- sans `run`, `run` asynchrone, sortie `undefined`, sortie trop grande,
  source trop grand, langage inconnu : refusés.

## 7. Plateformes

`rquickjs-sys` compile le C de quickjs-ng par `cc` (pas de libclang : les
liaisons sont livrées, dont `x86_64-pc-windows-msvc` et
`aarch64-apple-darwin`, et son `build.rs` a une branche MSVC). swc est du Rust
pur. Ce sont les runners du paquet npm qui le prouveront : `rag3db-90` est
prévenue à la fusion.

## 7 bis. Le poids (mesuré le 11 octobre)

Le paquet npm a grossi de 79 à 89 Mo strippé sous Linux, de 81 à 93 Mo sous
macOS (`rag3db-90`). Mesuré sur un binaire témoin en release strippé, chaque
variante exerçant vraiment le code :

| Variante | Taille | Écart |
|---|---|---|
| base (serde_json) | 0,39 Mo | — |
| + QuickJS (`rquickjs`) | 1,74 Mo | +1,35 Mo |
| + `swc_ts_fast_strip` | 7,55 Mo | +7,2 Mo |
| + `swc_ts_fast_strip`, LTO complète | 5,19 Mo | +4,85 Mo |
| le parseur swc seul | 2,51 Mo | +2,1 Mo |

**C'est swc qui pèse, pas QuickJS** : `swc_ts_fast_strip` tire ses
transformations TypeScript et React (son mode `Transform`), non
optionnelles. Décidé (orchestration, 11 oct.) : garder tel quel pour le
proto ; la LTO complète au profil de publication est proposée à la session
du paquet (elle réduirait tout le binaire) ; un effaceur à nous sur le seul
parseur swc (~+2,1 Mo) seulement si Lucie demande le poids.

## 8. Pour la suite

- Lot 2 : en tête, les noms possédés (`Arc<str>`) dans `PortDef`,
  `NodeSchema`, `ConfigParam` et `node_type()`, relus par la session
  recherche ; puis le nœud déclaré en script, ses ports et sa configuration,
  et ses entrées par les ports.
- Lot 3 : le rechargement se branchera sur le montage `Reactor::watch_bound`
  de la session mémoire (les `reactions` du manifeste).
