# Le socle d'agent de code : les primitives de fichier, un seul jeu déclaré

- **État** : ouvert (15 novembre 2026). Sa place est à donner par c7 ; il vient après la garde de
  l'équipier (B) d'archi
- **Famille** : produit
- **Gravité** : planification. Une tâche sur un backend qui ne déclare aucun outil d'écriture (la forme
  du manifeste de Lucie) n'a pas d'outils d'agent de code
- **Atteignable en service** : oui, toute tâche de l'atelier sur un tel backend
- **Touche rag3weaver** : oui (`src/code_tools.rs`, les gabarits `templates/tools/atelier_*.mmd`)
- **Pour** : everything declarative (ED), les primitives ; archi, leur branchement dans le pas
- **Signalé par** : archi, page 07 (f0e29b3dc) et la précision de c7 (« un seul mécanisme, le socle et
  les outils de l'atelier sont les mêmes »), 15 novembre 2026

## La découpe (archi, confirmée)

1. **ED, les primitives et leur réindexation**, en un jeu d'outils livré et déclaré une fois :
   - `read_file` {path, offset, length} : 200 lignes par défaut, numérotées, « lignes a à b sur N »,
     l'empreinte ;
   - `replace_text` : `old` unique, sinon un refus qui en dit le compte ;
   - `replace_lines` et `insert_lines` : ils exigent l'empreinte de la dernière lecture et refusent une
     empreinte périmée ;
   - `write` : un fichier neuf seulement, refus d'écraser ;
   - `glob`, `list`, `grep`.
   Chaque écriture rend le diff, réindexe par l'écrivain et écrit son run.
2. **archi, le pas** : toute tâche reçoit le socle, plus les outils du backend, et l'homonyme du backend
   gagne. La porte des écritures s'applique aux primitives du socle (o/n dans la boîte et dans tty).
3. **Ensemble, un témoin de bout en bout** sur un manifeste sans outil d'écriture : une lecture avec
   empreinte, une édition par `replace_text`, le diff au run, l'écriture qui attend le o sous la porte,
   et une `replace_lines` sur une empreinte périmée refusée sans rien écrire.

## Ce qui existe déjà

`code_tools.rs` a déjà `read_file`, `grep_files`, `list_files`, `edit_file` (Replace avec un `old` unique,
et Write, qui écrase) et `edit_file_with_hash`. Les gabarits `atelier_read_file`, `atelier_replace_text`,
`atelier_replace_lines`, `atelier_insert_lines`, `atelier_grep_files` et `atelier_list_files` les
déclarent. Il manque :
- `glob` ;
- le refus d'écraser de `write` ;
- l'exigence de l'empreinte sur `replace_lines` et `insert_lines`, à vérifier ;
- un nom de socle unique, les `atelier_*` gardés en alias pendant une période dite.

## Où en est le lot (15 novembre 2026, 17 h)

- Branche `socle-agent-de-code`, tête 8ef56dd5e, partie de master e0b3725ee.
  - `write` {path, text, force} et `glob` {pattern, limit} : leurs témoins ont été vus rouges
    (« unexposed tool … »), puis verts.
  - La passe épinglée est verte sur a4697560e.
- **Le run d'une écriture** (réserve de c7) : un appel passé par l'hôte (`DaemonHost::call`, le chemin
  d'un client, de l'IDE ou du chat) garde son run au journal, avec le rendu entier, donc le diff
  (`a_workshop_write_leaves_a_run_with_its_diff`). Ce n'est pas neuf, c'est vérifié. Un appel direct
  au backend, celui des témoins, ne garde que les événements du run.
- **L'empreinte** est tenue et exigée par la couche du chat, pas affichée. Le refus d'une empreinte
  périmée dit « le fichier a changé depuis ta lecture (lignes a-b de <chemin> lues au tour t) :
  relis-le avant d'écrire ; rien n'est écrit », et c7 juge que cela suffit.
- **La limite de `glob`** : il ne parcourt que la racine de l'atelier, pas un worktree attaché hors de
  la racine.
- Sa place en file : après codeparsers, avant archi (B) (c7).

## Ce qu'il faut pour le fermer

Les primitives au socle avec leurs témoins, rouges d'abord ; le branchement d'archi ; le témoin de bout
en bout vert ; et les nouveautés.
