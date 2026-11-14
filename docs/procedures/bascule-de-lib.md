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
4. **Le verdict, sous la lib explicite** (`scripts/verdict-e2e-code.sh`) : `e2e_code`,
   `e2e_lectures` et `e2e_demon_hote` verts, sous `RAG3DB_LIB=<id>`, les sources du crate prises sur master, sans
   `RAG3WEAVER_MOTEUR_ANCIEN` ; la ligne « moteur lié » de chaque passe doit nommer la lib. Seules
   des bases temporaires de test s'ouvrent. Dans `e2e_lectures`, trois témoins se lisent par leur
   nom : `la_branche_dense_filtree_par_langage_repond_dans_l_instantane_de_l_appel` et
   `une_recherche_dense_bornee_a_un_chemin_tient_sous_l_instantane` (la branche dense filtrée
   sous l'instantané de lecture, cassée une fois par L6), et
   `la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel`, qui sous une lib d'avant
   le format 43 se rend en disant « non jugé » : sous la lib d'une bascule 43, ce « non jugé » est
   un rouge — le témoin doit se juger (`VERDICT_FORMAT=43` le fait compter ainsi par le script).
   `e2e_demon_hote` est au verdict depuis le 14 novembre 2026 (c7) : la réouverture d'une base
   après un point de reprise échoué (`une_base_a_rouvrir_ne_lache_que_son_backend`) y rougissait
   depuis le 13 novembre, sous e93710f36 puis 0c41f9fe6, et les deux suites d'avant ne la voyaient
   pas. Un rouge de cette suite se départage lib contre Rust par deux jeux croisés (sources
   d'avant sous la lib neuve, sources neuves sous la lib d'avant) avant d'accuser la lib ; s'il
   vient de la lib, la bascule attend le cœur.
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
   bâti contre la lib neuve du magasin (son `rag3db.hpp`) depuis **`origin/master`** (jamais un
   arbre de travail, qui peut avoir du retard ; le script dit le commit de l'outil), lit chaque
   table de relations par ses deux rangements, nœud par nœud, et désigne les relations qui
   divergent : d'un seul côté, vues deux fois, aux bouts différents, à une propriété différente,
   avec dix exemples nommés. Il ouvre une **seconde** copie reflink de chaque base
   (`…/instantanes/<étiquette>-controle/`), jamais l'instantané, qui reste le retour. Compter
   **environ 15 minutes par base de code** (la version ancrée, 339cbb5dc ; quelques secondes pour
   une mémoire). Un écart arrête la bascule (`courant` ne bouge pas) et s'écrit en ticket avec ses
   tables et leurs relations : la base se réparera ou se réindexera avant toute lib neuve.
   **La nuance du 14 novembre 2026** (c7, à la bascule vers 373b5477b, sur les écarts de code-lourd et de code-principal, ticket docs/optimiseur/tickets/2026-11-14-code-lourd-et-code-principal-divergent-entre-leurs-deux-rangements.md) : un écart
   antérieur à la lib neuve (lu sur l'instantané pris sous l'ancienne), petit et ticketé, de la famille que la lib
   neuve corrige, ne bloque pas la bascule. La base se répare dans la fenêtre suivante, sur instantané. Revenir en
   arrière à un seul poste laisserait les deux postes sur deux libs, ce qui est pire. Un écart d'une autre famille,
   ou plus grand, arrête toujours la bascule.
   **Un « aucun écart » est plus faible qu'il n'y paraît** (14 novembre, relevé d'hotfixs) : l'outil tire ses bouts
   d'une seule lecture non ancrée, jouée dans un seul sens, et ce que cette lecture perd n'est relu par aucun sens.
   Ticket fermé par a34fe130a (bouts ancrés par tranches, quatre comptes par table) : `docs/tickets/closed/2026-11-14-check-rel-directions-tire-ses-bouts-d-une-seule-lecture-non-ancree.md`.
   **L'outil tourne sous `poste lourd`** (14 novembre, relevé d'ED) : il prend 250 à 370 % de CPU pendant 15 à
   30 minutes par base de code. Lancé hors du verrou, il a faussé une mesure d'ED à luciepc et échappé au compte
   des lourds. Le script l'appelle par `~/.cache/rag3weaver-build/poste lourd`, la borne dans le lourd. Aucun
   écart ne prouve pas qu'une table est saine : le contrôle juge la cohérence des deux rangements, pas la
   complétude (14 novembre, hotfixs : une relation vraie perdue par un relevé non ancré lu dans un seul sens,
   « égales » au contrôle). Un relevé qui sert à reconstruire se fait par tranches d'offsets ancrées sur chaque
   étiquette, le sens prouvé par EXPLAIN, avec un compte par table (lues dans chaque sens, union, perdues
   d'un sens) ; `usages` et `impact` par le binaire, avant et après, sont le témoin final. Un contrôle qui ne trouve aucune base à lire est
   rouge (code 7), jamais « aucun écart » : le script compte les bases contrôlées (hotfixs,
   7 novembre). Référence connue : l'instantané
   `bascule-04063ee06` de `code-rag3db-depot` (25 octobre, avant le rebâti de la base de code)
   rend sur LOCKS 206 relations d'un seul côté, 179 vues deux fois, 247 aux bouts différents, et
   sur QUERIES 286, 286 et 306 — les demi-arêtes de ce jour-là ; celui de `memoire-longue`, aucun
   écart (outil 339cbb5dc, lib 04063ee06, 14 minutes, le 7 novembre).
9. **`courant` aux deux postes ensemble** (`scripts/courant-bascule.sh <id> <md5>`) : refuse si une
   passe qui lit `courant` tourne encore (l'unité `ide-fusion` à luciepc le 25 octobre) ou si la md5
   diffère d'un poste à l'autre ; pose ici, puis là-bas, et relit les deux `readlink`.
10. **Les tunnels d'embarquement avant le réveil** : l'hôte de ce poste embarque par
   `127.0.0.1:7979-7980`, des tunnels vers luciepc (7981, vers luciepc:7880 granite-107m, est arrêté depuis le 12 novembre au mot de Lucie : un script qui le cite encore attend dans le vide). Un hôte réveillé sans eux pose 0 vecteur et se
   replie en silence (le ticket du service injoignable à l'ouverture, revécu à 15 h 44 le
   25 octobre). Les reposer avant que G réveille.
11. **Le réveil, par G**, et la vérification par `daemon_status` : le `build` nomme la lib neuve, et
    chaque backend rouvre sa base sans refus (`refused: null`). La première ouverture de chaque base
    sous la lib neuve est le vrai essai de son format.

## Poser un binaire seul, sans changer de lib

Pour donner aux hôtes un binaire neuf de master (une fermeture qui tient, un correctif de l'hôte), sans toucher à
`courant` :

1. **Le bâti et l'essai** (`docs/optimiseur/scripts/preparer-binaire.sh <commit>`, `LIB=<id>` pour une lib qui n'est pas
   `courant`) : bâti à luciepc, essai sur 7991 et check-manifests aux deux postes. Il fait checkout dans
   `rag3db-controle-G` : jamais pendant un verdict ou un rejeu dans ce worktree.
2. **Le verdict des trois suites sur les sources du binaire** (`e2e_code`, `e2e_lectures`, `e2e_demon_hote`), sous la
   lib qu'il liera, nommée.
   **Un moteur plus neuf que la lib** (c7, 14 novembre 2026) : dès qu'un lot C++ atterrit, les sources du verdict portent
   un moteur plus neuf que la lib posée, et `run_e2e.sh` refuse la lib (« librag3db.so est plus vieux que le dernier
   commit de ses sources »). Le verdict se joue alors sous `RAG3WEAVER_MOTEUR_ANCIEN=1`, **dit** dans son état et dans
   la page de la pose. C'est juste : l'hôte posé tourne sous sa lib par son env et son filet, quel que soit le C++ de
   ses sources.
3. **Le nom** : un binaire se nomme par son commit **et** la lib qu'il lie, « binaire fac67a208 sur 373b5477b », jamais
   par son commit seul.
4. **L'annonce aux sessions**, puis la pose (`bascule-pose.sh` et `bascule-reveil.sh`, avec la même lib) quand aucune
   passe ne lit l'hôte, puis les témoins : `/sante` nomme la lib, chaque backend rouvre sans refus.

## Revenir

**Où sont les copies d'avant la pose** (14 novembre 2026) : le binaire et `rag3db-check-lib` d'avant restent dans
`~/.local/bin/` sous `.<étiquette>` ; l'env et le filet (`check-lib.conf`) d'avant vont dans
`~/.cache/rag3weaver-build/sauvegardes-de-pose/`, jamais dans le dossier d'unités systemd, où soixante copies
s'étaient accumulées. Le ménage de fin de session ramasse ce dossier.

Hôtes arrêtés, `courant` ramené à la lib d'avant, puis `RETOUR.sh` de l'instantané, à chaque poste :
il met l'état d'après de côté (`apres-<étiquette>-<base>`) et recopie l'instantané. Jamais en
ramenant `courant` seul sur une base que la lib neuve a touchée.

## La leçon du 25 octobre au soir

La lib bâtie sur 66994dd43 était prête et verte à 20 h 05 ; elle n'a été vue qu'à 21 h 01, sur une
fenêtre qui fermait à 21 h 25. La surveillance cherchait « 66994dd4 », le nom attendu, alors que le
magasin l'avait nommée 3724accba ; l'orchestration cherchait le même faux nom. **Une surveillance
cherche le nom que le bâti a rendu (`id`) et l'état « fini », pas le hash qu'on a en tête.**
