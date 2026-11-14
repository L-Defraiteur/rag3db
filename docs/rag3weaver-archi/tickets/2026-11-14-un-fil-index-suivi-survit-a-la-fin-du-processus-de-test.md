# Un fil index-suivi survit à la fin du processus de test

- **État** : ouvert (14 novembre 2026, relevé par archi, lu par hotfixs)
- **Famille** : produit
- **Gravité** : plantage à la sortie (SIGSEGV du processus de test, une fois que tous ses tests ont
  passé). Un passage vert peut finir en rouge par sa seule fin.
- **Atteignable en service** : à établir. L'hôte qui s'arrête pendant le suivi d'un premier index
  pourrait le connaître de même.
- **Touche rag3weaver** : oui (`src/daemon_host/worktrees.rs`, `suivre_le_premier_index`)
- **Pour** : archi

## Ce qu'on voit

Au portable, le 14 novembre à 18 h 22 m 39 s, le core 2513521 (`coredumpctl info 2513521`) :
- binaire de tests de la lib rag3weaver
  `~/.cache/rag3weaver-build/delais-c/debug/deps/rag3weaver-861fc0b9e09f91ec`, lib 0c41f9fe6 ;
- filtres `agent:: daemon_host::chat daemon_host::atelier_fichiers::` ;
- la mort tombe **à la sortie du processus**, après « 89 passed; 0 failed » ;
- signal SEGV, `si_code` **SEGV_ACCERR** ;
- le fil qui meurt est le TID 2519233, **nommé `index-suivi:56b`** ;
- la seule trame est `#0 0x000055c1d29e9860 n/a (n/a + 0x0)` : une adresse d'exécution qui n'appartient
  à aucun module connu, dans un core **tronqué** à 3,5 Mo ;
- rejoué seul, deux fois : vert.

Ce n'est pas la famille du dictionnaire
(`docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`),
malgré le même `SEGV_ACCERR`. Celui-là meurt dans le COMMIT qui écrit un point de reprise ; celui-ci
dans un fil de rag3weaver, à la fin du processus.

## Le fil

Un seul endroit lance un fil `index-suivi:<id>` : `suivre_le_premier_index`
(`src/daemon_host/worktrees.rs`, vers la ligne 194). Il lit le journal du premier index d'un dépôt et
publie son avancement, puis `RepoIndexed`.
- Le fil est **détaché** : `let _ = std::thread::Builder::new().name(nom).spawn(…)`. Personne ne le
  joint.
- Il tient un **`Arc<Hosted>` fort**, donc le backend et sa base. L'hôte n'est tenu que par un `Weak`.
- Il boucle sur le journal jusqu'à ses bornes de temps (un journal qui ne paraît pas, ou qui ne finit
  pas), en relisant par moments les comptes de l'index (`comptes_de_l_index`, qui lit le backend).
- Il sort de lui-même quand l'hôte est parti (`hote.upgrade()` vide), mais seulement au moment où il
  publie.

**Lecture probable, non prouvée** : à la fin d'un test qui a lancé un premier index, le processus
sort pendant que ce fil boucle encore. Le processus défait alors ses statiques et ses bibliothèques,
tandis que le fil, par son `Arc<Hosted>`, touche encore le backend ou le moteur. Il accède à une page
dont les droits ont été retirés, d'où le `SEGV_ACCERR` sur une adresse hors de tout module.

## Ce qui le départage

- **Les lanceurs** : dans `agent::`, `daemon_host::chat` et `daemon_host::atelier_fichiers::`, quels
  tests ouvrent un dépôt et déclenchent un premier index (donc `suivre_le_premier_index`), et laissent
  le processus finir avant la dernière ligne du journal.
- **Comment ils l'arrêtent** : aujourd'hui, rien ne joint ce fil ni ne lui demande de s'arrêter. Il ne
  meurt que de ses bornes de temps, ou de l'hôte parti au moment de publier.
- **Un core complet** : celui-ci est tronqué. Un core entier donnerait la pile du fil.

## Ce qu'il faut pour le fermer

1. Un témoin rouge : un test qui lance un premier index puis laisse le processus finir avant la fin du
   journal, joué en boucle jusqu'à la mort à la sortie.
2. Le correctif : le fil garde son `JoinHandle`, et l'hôte ou le test le joint ou l'arrête avant de se
   défaire (un drapeau d'arrêt lu entre deux lectures du journal). Le fil peut aussi ne tenir qu'un
   `Weak<Hosted>`.
