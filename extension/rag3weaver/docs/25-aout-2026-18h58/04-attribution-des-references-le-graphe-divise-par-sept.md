# 04 — L'attribution des références : le graphe divisé par sept

25 août 2026, 21h. Suite du [03](03-codeparsers-integre-et-deux-bugs-de-fond.md)
§2 : *« 66 771 relations pour 1 402 scopes, 47 par scope — dette nommée »*.
Avant de concevoir la résolution contre la base ([02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md) §4),
des chiffres.

## 1. D'où venait le bruit

Histogramme sur `src/dataflow/` (test ignoré `analyze_own_dataflow_dir`) :

| | avant |
|---|---|
| relations | 66 771 — CONSUMES 31 168, CONSUMED_BY 31 168, DEFINED_IN 1 783, PARENT_OF / HAS_PARENT 1 272 |
| cibles CONSUMES distinctes | 413 |
| top cibles | `namespace:port` 1 489, `enum:PortType` 1 347, `namespace:node` 1 315, `namespace:graph` 1 224, `class:PortDef` 1 076… |
| CONSUMES par type de source | **lambda 10 648**, method 11 028, class 5 628, function 1 518, namespace 1 188 |
| doublons par imbrication | 5 391 |

Trois causes, toutes dans la façon d'**attribuer** une référence, pas dans
la façon de la résoudre :

1. **Les références de niveau fichier étaient attribuées à chaque scope du
   fichier.** `resolve_unknown_references` ajoute, pour tout scope non-module,
   les références du scope de fichier (`use super::port::…`) — d'où
   `namespace:port` « consommé » 1 489 fois, par tout ce qui vit dans un
   fichier qui l'importe.
2. **Une classe portait les références de ses méthodes** — chaque référence
   comptée deux fois.
3. **Les fermetures sont des scopes** : 244 « Closure » anonymes, porteurs
   de 10 648 CONSUMES. Personne ne cherche `Closure@checkpoint.rs:95`.

Le résolveur, lui, choisit **un** candidat par identifiant (même fichier
d'abord, puis type-valeur, puis premier). Pas d'explosion en N candidats.

## 2. Ce qui a été fait

- `codeparsers` : deux options de résolveur, `include_file_level_refs` et
  `include_child_refs` (défaut `true`, historique, les 65 tests inchangés) ;
  `ParseProjectOptions.resolver_options` pour les passer.
- rag3weaver `analyze` : les deux à `false` — **une référence n'est portée
  que par le scope le plus interne qui la contient** — puis `fold_lambdas`
  (une fermeture disparaît des entités, ses relations vont au scope nommé le
  plus étroit qui l'englobe, transitivement) et `dedupe_relations`.

| | avant | après |
|---|---|---|
| scopes | 1 402 | **1 158** |
| relations | 66 771 | **9 645** |
| CONSUMES | 31 168 | **3 551** |
| cibles distinctes | 413 | **416** — rien de perdu |
| par scope | 47 | **8,3** |
| résolution | 194 ms | 54 ms |
| top cibles | `namespace:port` 1 489 | `method:name` 312, `enum:PortType` 211, `class:PortDef` 202, `NodeContext` 115 |

`e2e_code` inchangé : le fichier par son nom, `take_results` par sa
signature, `take_results CONSUMED_BY execute`, ré-ingestion idempotente.

## 3. Ce qui reste, nommé

- **`method:name` en tête (312)** : tout appel `.name()` est résolu vers
  *une* méthode `name` — celle que l'heuristique préfère — sans savoir sur
  quel type. C'est l'ambiguïté des méthodes sans inférence de type, bornée
  (un candidat), fausse une fois sur N. À traiter avec un type quand on en
  aura un (le résolveur d'imports de `codeparsers`, ou le type du receveur),
  pas avec plus d'heuristiques.
- **748 doublons par imbrication restent** : une méthode et son `impl`
  peuvent encore citer la même cible quand l'`impl` la référence lui-même
  (signature, `where`). Légitime en partie.
- **La résolution contre la base** ([02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md) §4)
  n'est pas faite : le mapping global reste construit en mémoire à chaque
  analyse. Ce document règle la **précision** ; l'**incrémentalité** est la
  prochaine étape, et elle est plus simple maintenant que le graphe est sept
  fois plus petit.
