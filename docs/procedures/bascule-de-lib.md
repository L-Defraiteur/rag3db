# Basculer la lib du moteur

Le déroulé d'une bascule de `librag3db.so`, des deux postes à la fois (ce poste et luciepc), tel
qu'il a tourné deux fois le 25 octobre 2026 : vers 04063ee06 (formats 41 et 42, requêtes
préparées) à 17 h 45, puis vers 3724accba (V2-1b, V2-1c) à 21 h 05. Les scripts sont à côté, dans
`scripts/`, copiés tels qu'ils ont tourné puis tenus à jour (7 novembre : la borne dans le lourd,
`e2e_lectures` au verdict, le contrôle des deux sens) ; leurs chemins sont ceux des deux postes.

## Pourquoi un déroulé

- **Une base touchée par une lib neuve peut refuser toutes les libs d'avant.** Depuis le format 41,
  une lib qui a passé un point de reprise sur une base y pose sa version ; une lib plus ancienne la
  refuse (« different version »). Ramener `courant` en arrière ne rouvre plus rien : le seul retour
  est l'instantané pris avant.
- **Les binaires sont liés à une lib, pas à `courant`.** Les hôtes suivent leur lib par
  `RAG3DB_LIBRARY_DIR` (RUNPATH) ; une ABI qui change (une vtable, un membre de classe) exige de les
  rebâtir contre la lib neuve.
- **Des passes d'autres sessions lisent `courant` à chaque appel cargo.** Le poser pendant leur passe
  mélange deux libs dans une même passe.

## Les étapes

1. **Bâtir hors de `courant`** (`scripts/batir-lib.sh <commit>`, à luciepc, sous `poste lourd`,
   `-j 8`). Worktree à soi, détaché ; jamais l'arbre de Lucie. La publication est toujours
   `--sans-courant`. La borne de durée se met **dans** le lourd (`poste lourd timeout 1h cmake …`),
   jamais devant : depuis le 7 novembre les lourds font la file, et un `timeout` posé devant
   compterait l'attente.
2. **Lire le nom que le magasin a donné.** Le magasin nomme une lib par **son dernier commit
   moteur** (`src`, `extension/vector`), pas par la tête demandée : bâtie sur 66994dd43, la lib du
   21 h s'appelait `rag3db-build-3724accba…`. Le script l'écrit dans `lib58/id` ; toute attente,
   toute annonce lit ce fichier.
3. **Dire l'ABI, exactement.** Le script compare à la lib `courant` les symboles exportés (par nom)
   et la taille des vtables communes. Ce qu'il ne voit pas : un membre de classe changé (le
   `shared_ptr` de `PreparedStatement` du 25 octobre) ; c'est le cœur qui le déclare
   (`ABI_CHANGEE=1`). Annoncer : symboles retirés, ajoutés, vtables changées, et ce qui doit se
   rebâtir.
4. **Le verdict, sous la lib explicite** (`scripts/verdict-e2e-code.sh`) : `e2e_code` et
   `e2e_lectures` verts, sous `RAG3DB_LIB=<id>`, les sources du crate prises sur master, sans
   `RAG3WEAVER_MOTEUR_ANCIEN` ; la ligne « moteur lié » de chaque passe doit nommer la lib. Seules
   des bases temporaires de test s'ouvrent. Dans `e2e_lectures`, trois témoins se lisent par leur
   nom : `la_branche_dense_filtree_par_langage_repond_dans_l_instantane_de_l_appel` et
   `une_recherche_dense_bornee_a_un_chemin_tient_sous_l_instantane` (la branche dense filtrée
   sous l'instantané de lecture, cassée une fois par L6), et
   `la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel`, qui sous une lib d'avant
   le format 43 se rend en disant « non jugé » : sous la lib d'une bascule 43, ce « non jugé » est
   un rouge — le témoin doit se juger (`VERDICT_FORMAT=43` le fait compter ainsi par le script).
5. **Copier la lib à l'autre poste**, md5 vérifiée, sans poser `courant`.
6. **Les hôtes, par G** : rebâtis contre la lib explicite, essayés — dont l'essai de l'IDE sur la
   lib neuve, avant le « posé » —, puis arrêtés avec leurs sockets (une socket éveille un hôte au
   premier appel MCP).
7. **Les instantanés** (`scripts/instantanes-bascule.sh <étiquette>`), au « posé » de G : les quatre
   bases partagées (ici `memoire-longue` et `code-rag3db-depot`, à luciepc `code-principal` et
   `code-lourd`). Le script refuse sans rien copier si un processus tient un fichier d'une base,
   copie par `cp --reflink=always` (les deux `/home` sont en btrfs : immédiat, sans place), vérifie
   les tailles octet pour octet et écrit `RETOUR.sh` à côté. Un `.wal` absent des copies dit que les
   hôtes se sont arrêtés proprement.
8. **Le contrôle des deux sens, sur les instantanés** (`scripts/controle-des-deux-sens.sh
   <étiquette> <id>`, à chaque poste ; demande d'hotfixs, 7 novembre 2026) : `tools/check_rel_directions`,
   bâti contre la lib neuve du magasin (son `rag3db.hpp`), lit chaque table de relations par ses
   deux rangements et compare, propriété par propriété. Il ouvre une **seconde** copie reflink de
   chaque base (`…/instantanes/<étiquette>-controle/`), jamais l'instantané, qui reste le retour.
   Un écart arrête la bascule (`courant` ne bouge pas) et s'écrit en ticket avec ses tables et
   leurs comptes : la base se réparera ou se réindexera avant toute lib neuve. Aucun écart ne
   prouve pas qu'une table est saine. Un contrôle qui ne trouve aucune base à lire est rouge
   (code 7), jamais « aucun écart » : le script compte les bases contrôlées (hotfixs, 7 novembre). Référence connue : l'instantané `bascule-04063ee06` de
   `code-rag3db-depot` (25 octobre, avant le rebâti de la base de code) rend LOCKS 413 contre 505
   et QUERIES 21 804 contre 22 089 — les demi-arêtes de ce jour-là ; celui de `memoire-longue`,
   aucun écart (essayé le 7 novembre).
9. **`courant` aux deux postes ensemble** (`scripts/courant-bascule.sh <id> <md5>`) : refuse si une
   passe qui lit `courant` tourne encore (l'unité `ide-fusion` à luciepc le 25 octobre) ou si la md5
   diffère d'un poste à l'autre ; pose ici, puis là-bas, et relit les deux `readlink`.
10. **Les tunnels d'embarquement avant le réveil** : l'hôte de ce poste embarque par
   `127.0.0.1:7979-7981`, des tunnels vers luciepc. Un hôte réveillé sans eux pose 0 vecteur et se
   replie en silence (le ticket du service injoignable à l'ouverture, revécu à 15 h 44 le
   25 octobre). Les reposer avant que G réveille.
11. **Le réveil, par G**, et la vérification par `daemon_status` : le `build` nomme la lib neuve, et
    chaque backend rouvre sa base sans refus (`refused: null`). La première ouverture de chaque base
    sous la lib neuve est le vrai essai de son format.

## Revenir

Hôtes arrêtés, `courant` ramené à la lib d'avant, puis `RETOUR.sh` de l'instantané, à chaque poste :
il met l'état d'après de côté (`apres-<étiquette>-<base>`) et recopie l'instantané. Jamais en
ramenant `courant` seul sur une base que la lib neuve a touchée.

## La leçon du 25 octobre au soir

La lib bâtie sur 66994dd43 était prête et verte à 20 h 05 ; elle n'a été vue qu'à 21 h 01, sur une
fenêtre qui fermait à 21 h 25. La surveillance cherchait « 66994dd4 », le nom attendu, alors que le
magasin l'avait nommée 3724accba ; l'orchestration cherchait le même faux nom. **Une surveillance
cherche le nom que le bâti a rendu (`id`) et l'état « fini », pas le hash qu'on a en tête.**
