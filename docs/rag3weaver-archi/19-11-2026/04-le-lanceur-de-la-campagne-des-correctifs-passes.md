# Le lanceur de la campagne des correctifs passés — conception du jeudi 19 novembre 2026

Le chantier est de Lucie, transmis par c7 : rejouer des correctifs réels.
- **Le départ** : Algareth repart du commit parent d'un correctif, dans un arbre à lui, avec tout
  l'outillage sauf git, et le signalement d'alors pour seule entrée.
- **Le juge** : le témoin de l'époque, qui doit passer vert, puis la comparaison avec notre
  correctif.
- **La répartition des rôles** :
  - archi : le lanceur ;
  - le banc : le juge et la grille (`docs/banc/19-11-2026/02-la-campagne-d-algareth-le-juge-et-la-grille.md`) ;
  - chaque session : ses cas.

## Le fichier de cas

Un fichier Markdown par cas, avec en tête un bloc ```json « lanceur » :

```json
{
  "repo": "<dépôt, si ce n'est pas rag3db : ex. https://github.com/L-Defraiteur/lucivy>",
  "parent": "<commit 40 hex>",
  "tree": "<dossier d'où se jouent les commandes ; « . » pour la racine>",
  "paths": ["<facultatif : restreint l'archive à ces chemins>"],
  "needs_lib": false,
  "witness": "<filtre exact du témoin>",
  "witness_files_from": { "commit": "<40 hex>", "paths": ["…"] },
  "test": "<commande exacte, depuis tree>",
  "suites": ["<commande d'une suite exigée>"],
  "hints": ["<chemin d'une souche ou d'un indice laissé dans l'arbre>"],
  "budget": { "steps": 12, "input_tokens": 2000000, "minutes": 30 },
  "ours": "<commit de notre correctif : lu par le banc seul>"
}
```

Le reste du fichier est en prose. Le lanceur ne donne à Algareth que la section **« Le signalement,
tel qu'il était »**, mot pour mot. Toute piste (ce qu'on suppose, ce qu'on pourrait faire) va dans
« Le point de départ », que le lanceur ne donne pas.

## Les refus, avant tout lancement

- La section du signalement contient le hash de `ours`, ou un chemin des fichiers que `ours`
  change : refusé.
- L'arbre au parent contient le fichier de cas : refusé. `ours` vit dans un fichier postérieur au
  parent ; le lanceur le vérifie quand même.
- Le témoin n'existe pas au parent, et `witness_files_from` ne le donne pas : refusé.
- Le rouge ne se voit pas au parent : refusé. Ce contrôle a lieu avec les fichiers de
  `witness_files_from` en place, il se juge sur l'assertion, et une erreur de compilation n'est pas
  un rouge.

## Ce que fait le lanceur, par cas

1. **L'arbre sans histoire.**
   - `git archive <parent>` dans un dossier neuf (dépôt `repo`, rag3db par défaut). C'est le
     **dépôt entier par défaut** : un crate qui inclut hors de lui (`include_str!` vers `templates/`)
     ne compilerait pas avec son seul dossier. `paths`, quand le cas le déclare, restreint l'archive,
     jamais l'inverse. `tree` n'est que le dossier d'où se jouent les commandes.
   - **Les sous-modules aussi**, chacun archivé à son commit enregistré au parent et posé à son
     chemin. `git archive` ne les prend pas, et cargo lit les manifestes des dépendances par chemin
     (`tools/rust_api`, `codeparsers`) même sans leur feature.
   - **Sans les dossiers `docs/`, où qu'ils soient** (`docs/`, `extension/rag3weaver/docs/`, …),
     sauf `docs/procedures/`. Nos journaux de session et nos tickets au parent énoncent souvent le
     principe du correctif : c'est une fuite, pas un indice réaliste. Les `README.md` restent. Un cas
     qui veut une page la remet par `paths`.
   - Puis les fichiers de `witness_files_from`, copiés depuis leur commit : ce sont des indices
     déclarés, que le banc voit comme « test rouge en place ».
   - Puis `git init` et un seul commit « base ». La tâche a besoin d'un dépôt pour sa branche et
     son worktree ; il n'y a aucune histoire.
2. **Le rouge prouvé au parent.** La commande `test` est jouée sous `poste lourd`, rouge sur
   l'assertion, avec **un `CARGO_TARGET_DIR` propre au cas**, sous son dossier. Deux arbres sur le
   même target ont rejoué le binaire de l'autre (« 0 passed, 98 filtered out », tty, 19 novembre).
   Le même target sert ensuite à la tâche d'Algareth et au juge.
3. **L'hôte de campagne**, à part des hôtes de service : son port, ses racines de tâches et de
   runs, la lib épinglée si `needs_lib`.
   - Pour chaque cas, un backend de code sur l'arbre, avec son index.
   - Une porte qui refuse toute commande git. Le reste de l'outillage est là, et `cargo` passe sous
     `poste lourd` (lot de la porte).
   - Une tâche par cas, en parallèle, bornée par les places du poste.
4. **La demande** : le signalement, seul, par la boîte (`request`, sans `expects`).
5. **Le budget** : par défaut 12 pas, 2 M jetons d'entrée et 30 min (réglable par cas). Au
   premier dépassement, le lanceur écrit `stop` dans la boîte et nomme la borne.
6. **La sortie**, dans `~/.cache/rag3weaver-build/campagne/<cas>/<run>/` sur le poste qui a joué,
   pour le banc :
   - `case.json`, `request.txt` ;
   - `diff.patch` et `diffstat.txt` (base..fin, nouveaux fichiers compris) ;
   - `trace.json` : steps, model_calls, input_tokens, output_tokens, cached_tokens,
     reasoning_tokens, wall_ms, model_ms, tool_calls, ended, stopped_by ;
   - `mailbox.jsonl`, `runs/`, `budget.json`.

   Aucun verdict : juger revient au banc.

## Le coût à surveiller

Un index de code par cas. Le premier tour se fait sur de petits crates : le terminal (`tty`) et
lucivy. Pour rag3weaver entier, un index partagé par commit parent.

## Taille

Environ 2 jours :
- les cas et les refus : une demi-journée ;
- l'hôte de campagne : une demi-journée à un jour ;
- la sortie : une demi-journée.

Il dépend du lot de la porte sous `poste lourd`.
