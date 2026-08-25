# 05 — `read` et `grep` sur une source de fichiers, annotés par le graphe

25 août 2026, 22h. Les deux outils qu'un agent de code utilise le plus,
écrits après relecture de l'ancienne version (ragforge, TypeScript sur neo4j
puis kuzu) — pour ne pas la refaire.

## 1. Ce que l'ancienne version a appris à notre place

Un agent a lu `community-docs` en entier. Ce qu'il en ressort, et ce qu'on
en a fait :

| leçon | d'où elle vient | ce qu'on a fait |
|---|---|---|
| **Un seul `read`, un seul `grep`, un seul propriétaire** | trois `read_file` arbitrés par des commentaires ; l'agent a fini **sans** outil `read_file` (un filtre « gardé pour compatibilité » a survécu à la migration qu'il compensait) | `code_tools.rs` : une fonction publique par op, les nœuds et les fiches d'outil l'appellent |
| **Grep sur la source ; le graphe *annote*, il ne cherche pas** | deux grep-en-base ratés : `LIMIT 5000` **avant** le filtre de chemin, N+1 sur du *pretty-print* découpé au `\|` | regex sur chaque fichier de la source, puis `(fichier, ligne)` → **scope le plus étroit** qui la contient |
| **Le contenu d'un `File` n'est jamais chunké ni résumé** | le `_content` des classes remplacé par un résumé « Members: » → tous les numéros de ligne faux, patchés par un `useParentLineRange` jamais validé | `File` n'a que son chemin comme contenu ; les offsets viennent de la source, pas de l'index |
| **Formats qui marchaient** | `00042\| ` à largeur fixe (et son inverse pour `edit`), pied de page *« Use offset=N to continue. Total: M lines »*, tableau `\| File \| Line \| Scope \| Match \|`, markdown ≈ 90 % de tokens en moins que le JSON | gardés tels quels ; markdown par défaut, `format=json` en option |
| **Plafonds côté serveur, dépassements dits** | `truncated: bool`, `slice(0, 10)` silencieux | `total_found` **et** `returned` ; `files_skipped` ; contexte plafonné à 5 ; 2 000 lignes, 2 000 caractères par ligne, 200 par ligne de résultat |
| **Pas de glob maison** | trois *matchers* différents, tous faux | préfixe de chemin et extension, point |
| **`special_ops` non typé** | `{grep: true, read: true}` sur une KB qui n'avait pas les champs requis — config qui aurait paniqué si elle avait été appelée, et ne l'a jamais été | pas de `special_ops` : `read`/`grep` sont des outils sur une source, le graphe annote ce qu'il connaît. Le champ `special_ops` de `config.rs` est à retirer |

Et ce qu'elle n'avait pas : **la péremption** — l'ancien `read` lisait le disque
et interrogeait la base sans jamais vérifier qu'ils parlaient du même
fichier ; les `startLine`/`endLine` décalés étaient rendus au modèle sans un
mot.

## 2. `FileSource` : les chemins sont virtuels

Question de Lucie : *« et chemins virtuels quand on ingère un repo distant,
on va savoir gérer ça ? »*. Réponse : `read` et `grep` ne touchent **jamais**
un chemin de disque. Ils parlent à une source :

```rust
pub trait FileSource: Send + Sync {
    fn cursor(&self) -> String;                          // worktree:<racine> | snapshot:<étiquette> | demain git:<sha>
    fn list(&self) -> Result<Vec<String>, String>;       // chemins relatifs, triés
    fn read(&self, path: &str) -> Result<Option<String>, String>;
}
```

Deux implémentations : **`WorkingTree`** (le disque, sous une racine,
`..` et chemins absolus refusés) et **`Snapshot`** (des contenus déjà
récupérés — un dépôt distant après téléchargement, une fixture). `GitRef`
s'emboîtera sans toucher aux nœuds. Service `file_source`
(`Arc<dyn FileSource>`).

`analyze_source(&source)` ingère ce qu'une source contient de parsable, avec
**`File.cursor` = l'identité de la source** et `absolute_path` vide pour une
source virtuelle. Les chemins sont les mêmes partout : dans la base, dans les
outils, dans les réponses au modèle.

## 3. Ce que `read` et `grep` rendent

**`read(path, offset, limit)`** : la fenêtre en lignes `00042| …`, les scopes
qui l'intersectent (`function \`take_results\` (592-598)`), et le verdict de
péremption — `stale: Some(false)` si le hash du contenu lu égale
`File.content_hash`, `Some(true)` sinon (**INDEX STALE** en tête du
markdown), `None` si la source connaît le fichier mais pas le catalogue.
Pied de page : `(File has more lines. Use offset=41 to continue. Total: 1360 lines)`.

**`grep(pattern, path_prefix, extension, max_results, context_lines)`** :

```
**Pattern:** `pub fn merge_port_values` | **Files:** 1 searched | **Matches:** 1

| File | Line | Scope | Match |
|------|------|-------|-------|
| port.rs | 109 | function `merge_port_values` (109-145) | `pub fn merge_port_values(a: PortValue, b: PortValue) -> Result<PortValue, String> {` |
```

Un appel dans le corps d'une méthode est rapproché de la **méthode**, pas de
l'`impl` (scope le plus étroit). Une regex invalide est une erreur, pas un
résultat vide. Un fichier modifié depuis l'indexation porte `⚠stale`.

Nœuds `ReadFileNode` / `GrepNode` (33 types avec la feature `code`), fiches
`templates/tools/read.mmd` et `grep.mmd`, enregistrées dans
`builtin_graph_tools` : le modèle voit **`grep`, `read`, `search`,
`search_expand`**. Une chaîne JSON nue sur le port de résultat est rendue au
modèle **sans guillemets** (`render_port_value`) — le markdown arrive tel quel.

`Catalog::find_by_field` (nouveau, dialectes rag3db et Postgres) pour les
scopes d'un fichier ; `Catalog::get` rend le nœud entier sous `"n"`, ce que
la lecture de colonnes sait désormais.

## 4. Mesuré

`e2e_code` 4/4 : un instantané de deux de nos fichiers ingéré comme un dépôt
distant (`cursor = snapshot:remote-demo`, pas de chemin absolu) ; `read`
frais ; `grep "fn take_results"` → `function take_results` ;
`grep take_results\(ctx, "signals"\)` → **`execute`**, pas l'`impl` ; un
`NOTES.md` connu de la source mais pas du catalogue → ni scope ni verdict ;
`services.rs` modifié dans l'instantané → `stale = Some(true)` dans `read`
**et** dans `grep`. Les deux outils appelés par le registre de graphes-outils
rendent du markdown nu ; un fichier inconnu rend une erreur lisible par le
modèle. Unitaires : 726 avec la feature, 720 sans.

## 5. Ce qui reste

- **`edit`** — l'inverse de `00042| ` (retirer les préfixes d'un texte
  recopié) était le détail d'ergonomie le plus payant de l'ancienne version ;
  à faire avec l'écriture.
- **`GitRef`** comme troisième source, avec l'histoire ([02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md) §5).
- **La ré-ingestion d'un fichier périmé** déclenchée par `read` — aujourd'hui
  on le dit, on ne le fait pas.
- Retirer `special_ops` de `config.rs`.
- `list` (les fichiers d'une source, avec leur état d'indexation) — petit,
  utile au modèle pour s'orienter avant de grep.
