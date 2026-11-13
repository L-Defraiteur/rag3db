# Un chemin de l'atelier sort de sa racine par un `..` qui suit un dossier absent, sans accord

- **État** : corrigé (13 novembre 2026, branche atelier-chemin-hors-racine, archi) : `canonique`
  résout chaque composant avant le test de racine. Témoin
  `daemon_host::atelier_fichiers::tests::a_dot_dot_after_a_missing_folder_does_not_leave_the_workshop_root`,
  rouge puis vert.
- **Famille** : produit
- **Gravité** : perte — une écriture hors de la racine de l'atelier, sans accord, remplace un fichier entier (et dans la racine, écrase un fichier en le disant créé)
- **Atteignable en service** : oui, par `insert_lines`, `replace_lines` ou `replace_text` de l'atelier avec un chemin relatif `<dossier absent>/../…` ; dans le chat, l'empreinte de lecture limite le cas à un fichier dont la ligne visée a été lue (un accord en lecture suffit) ; hors du chat, rien ne l'arrête
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : `canonique` 128-136, `atelier_resoudre` 145-146, `atelier_ecrire` 469-504)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

`canonique` rend la forme canonique d'un chemin qui peut ne pas exister : si `canonicalize` échoue,
elle remonte au parent et rejoint le nom. Sur un parent qui finit par `..`, `file_name()` vaut
`None`, et la branche `_ => p.to_path_buf()` rend le chemin tel quel, `..` compris. Le test de
racine d'`atelier_resoudre` (`chemin.starts_with(&racine)`, 146) compare composant par composant
sans résoudre `..` : il passe.

## Le cas

Racine de l'atelier `R`, dossier `nouveau` absent :
- `nouveau/../../.bashrc` se résout en `R/nouveau/../../.bashrc`, jugé dans la racine. La lecture
  d'`avant` échoue en NotFound (à cause de `nouveau`), `insert_lines at=1` part d'un texte vide,
  `ecrire_atomique` crée `R/nouveau/` puis renomme le temporaire sur `~/.bashrc` : l'ancien
  contenu est perdu, sans accord.
- `nouveau/../notes.txt` remplace `R/notes.txt` existant par le seul texte inséré ; le diff le dit
  « création ».

## Le témoin à écrire

Un atelier posé sur une racine temporaire, un fichier frère hors de la racine et un fichier dans la
racine : l'outil d'écriture sur ces deux chemins doit refuser (hors racine sans accord) ou viser le
bon fichier, et aucun des deux fichiers ne doit changer. Rouge sur le code d'aujourd'hui.

## Ce qui le fermerait

Normaliser `..` lexicalement avant le test de racine (ou refuser tout chemin qui en contient), et
le témoin vert ; vérifier aussi le chemin côté worktree (`code_tools.rs`, `full_path_within`).
