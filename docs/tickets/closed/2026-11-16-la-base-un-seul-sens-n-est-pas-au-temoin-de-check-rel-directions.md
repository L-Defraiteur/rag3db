# La base « un seul sens » du cœur n'est pas au témoin de `check_rel_directions`

- **État** : fait (18 novembre 2026, branche check-rel-one-direction) : test_check_rel_directions.sh --one-direction joue l'outil sur la table R de la base et exige les comptes exacts lus sous la lib 3d706f709 : avant 10, arrière 12, union 12, perdues d'un seul sens 2 (1:111 et 1:60, au seul sens arrière) ; ATTENDU.txt disait 11 et 1, faute d'avoir relu 1:60 ancré après la réouverture ; rouge sur le témoin de master, 5 vert à luciepc
- **Famille** : moteur (l'outil de contrôle des deux sens : `tools/check_rel_directions/`)
- **Gravité** : témoin incomplet. L'outil n'est prouvé que sur LOCKS de code-principal, sans compte exact attendu.
- **Atteignable en service** : non
- **Touche rag3weaver** : non
- **Pour** : la session qui reprend l'outillage de G (Codex)
- **Signalé par** : G, 15 novembre 2026 (le cœur a fabriqué la base à 1 h 18)

## Ce qu'on sait

- À luciepc : `~/.cache/optimiseur-mesure/un-seul-sens-copie/` contient `base.kz`, `base.kz.readers` et `ATTENDU.txt`,
  3,3 Mo. Table de nœuds `N(id INT64)`, 4 200 nœuds. Table de relations `R (FROM N TO N, line INT64)`, 12 relations,
  dont une (line 1001, `N {id: 8} -> N {id: 2101}`) lisible seulement au sens arrière.
- Comptes attendus (`ATTENDU.txt`) : avant 11, arrière 12, union 12, perdues d'un sens 1, vues deux fois 0, bouts
  différents 0.
- `tools/check_rel_directions/test_check_rel_directions.sh <source .cpp> <dossier de la lib> <copie de la base>` ne
  juge aujourd'hui que LOCKS, et seulement « au moins une relation d'un seul côté ».

## Ce qu'il faudrait

1. Un second mode du témoin, `test_check_rel_directions.sh --one-direction <source> <lib> <copie de la base>`, qui
   joue l'outil sur la table `R` et exige les comptes exacts ci-dessus.
2. La base ne va pas dans le dépôt (un binaire de 3 Mo, d'un format moteur qui change). Le témoin la lit par son chemin,
   sur une copie reflink jetable (`cp --reflink=always`), jamais sur l'original. Une base absente est un ROUGE « non
   joué ».
3. Prouver le témoin à luciepc, sous `~/.cache/rag3weaver-build/poste lourd`, avec la lib `courant` du magasin.
   L'ouverture d'une base écrite par f41c0ba5b sous une lib plus neuve est aussi un essai de format : un refus se dit
   dans le ticket, il ne se contourne pas.
