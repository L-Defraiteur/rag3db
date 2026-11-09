# Un backend écrit encore dans son dossier pendant qu'on l'efface, après son Drop

- **État** : corrigé (9 novembre 2026, lot arret-du-backend de la session recherche) : un backend qu'on laisse tomber attend ses fils et ferme son catalogue, comme `shutdown()`.
- **Famille** : produit
- **Gravité** : **plantage** — un processus de test meurt en SIGSEGV à sa sortie (96, 8 novembre
  18 h 49) ; et des TempDir à moitié effacés dans /tmp, qui est de la mémoire vive. Passe devant le
  Surveillant dans les lots de recherche (c7) : il rougit les passes des autres.
- **Atteignable en service** : à départager (un hôte qui ferme un backend puis efface ou copie son
  dossier).
- **Touche rag3weaver** : oui, l'arrêt d'un `Backend` et ses fils de fond.

## Ce que c'est

Le témoin de suite de run_e2e.sh (lot residus-de-suite) voit des `.tmp*` de tempfile laissés par
des tests qui ouvrent un backend dans un `TempDir` et le laissent tomber à la fin. Le dossier n'est
pas vide : il garde `memoire.rag3db.readers` (0 o), des `templates/backends/memory` vides, des
tronçons `memoire.rag3db.fts/Lucivy_Apport/shard_*`, des runs `runs/<backend>/*.json.zst`,
`memoire.checkpoints`. Un `remove_dir_all` qui bute sur un fichier recréé pendant l'effacement
s'arrête à mi-chemin, et `TempDir` avale l'erreur.

## La mesure

À luciepc, le 8 novembre 2026, trois passes par suite sous run_e2e.sh : e2e_cited_by 3/3,
e2e_juge 3/3, e2e_lectures 0/3 (2/3 en comptant la passe entière de master du matin et celle du
lot). Les suites passent ; seul le dossier reste.

## Le plantage (8 novembre 2026, 18 h 49, tour de 96)

Master 43dd520b0, lib cbc06c3e1 avec MOTEUR_ANCIEN : `e2e_cited_by` passe ses trois tests, puis le
processus meurt en **SIGSEGV à la sortie** (code 101), juste après la fermeture tardive d'un plein
texte encore partagé, dont le dossier temporaire est déjà effacé :

```
[rag3weaver] drop: fermeture FTS incomplète — Apport (encore partagé): … save_metas : No such file
```

Dossier gardé chez 96 : `~/.cache/rag3weaver-build/e2e-gardes/e2e_cited_by-20261108-184931-3253843`
(à luciepc ; il n'y reste que `checkpoints/`).

La même fermeture tardive revient dans d'autres suites, sans plantage mais à chaque fermeture d'un
backend de code : quatre tables à la fois, chaque fois « encore partagé ». Exemples
(`passe-gi.log` de 96 l. 3508-3519, `journaux-residus/e2e-entier.log` l. 465-468 et
`e2e_lectures.log` l. 450-453 du lot residus-de-suite, à luciepc) :

```
[rag3weaver] drop: fermeture FTS incomplète — Scope (encore partagé): close shard_0: commit shard_0: System error.'execute DAG: node 'save' failed: save_metas: An IO error occurred: 'No such file or directory (os error 2)''
[rag3weaver] drop: fermeture FTS incomplète — File (encore partagé): … (idem)
[rag3weaver] drop: fermeture FTS incomplète — Symbol (encore partagé): … (idem)
[rag3weaver] drop: fermeture FTS incomplète — Library (encore partagé): … (idem)
```

Lecture : au Drop du catalogue, une poignée de plein texte est encore tenue par quelqu'un d'autre
(un fil de fond, une vue de lecture, un rattrapage) ; sa fermeture se fait plus tard, quand le
dernier détenteur la lâche, et le dossier n'est plus là. Selon le moment, cela laisse un TempDir à
moitié effacé, une erreur dans le journal, ou un SIGSEGV à la sortie du processus.

## Le point de départ du correctif (c7)

Le mécanisme d'arrêt d'everything declarative, `stop_indexings_of` (84b0a3f4e : un rattrapage de
fond s'arrête au paquet suivant sur `/backend/close`), est à étendre plutôt qu'à doubler. Le Drop
d'un backend doit attendre tous ses fils, puis fermer ses poignées (plein texte, base) en dernier
détenteur, avant que son dossier ne puisse partir.


Départager ce qui écrit après le Drop du backend : la base encore tenue par un fil (le moteur
recrée son `.readers` à l'ouverture d'une lecture), l'écriture différée d'un run du journal (une
grâce de 250 ms), le plein texte qui finit son lot. Puis faire attendre ses fils à l'arrêt d'un
backend. **Une cause à part** (hotfixs, 8 novembre 2026) : ses bases-fichiers de la branche de
l'équipier `agent/tests-bases-dans-tmp` sont une autre cause ; le seul cas qui y ressemblait (un
`drop(base)` sous un démon encore vivant, dans `un_lecteur_choisit_son_chemin_et_le_dit`) y est
déjà retiré. Ce ticket porte celle-ci ; son correctif est à part. Les trois suites sortent de la
liste des résidus dans le commit qui la corrige, prouvé par plusieurs passes propres.

## Le correctif (9 novembre 2026, lot arret-du-backend)

- **Un seul registre d'arrêt** pour les fils de fond d'un catalogue (`catalog_threads`) : les
  indexations de fond et le rebâti du plein texte s'y inscrivent, chacun avec son drapeau d'arrêt
  lu entre deux paquets ; `stop_and_join` les arrête et les attend. Le registre tient le catalogue
  par un `Weak` (l'ancien `INDEXINGS` gardait l'`Arc` des indexations finies). `indexings_running`
  et `embed_watch_of` ne comptent que les index (accordé avec everything declarative).
- **Le rebâti du plein texte** s'arrête au lot suivant et reprend à la prochaine ouverture.
- **`impl Drop for Backend`** fait `shutdown()`, une fois : réactions attendues, fils arrêtés et
  attendus, catalogue fermé. **`Drop for Catalog`** vide sa vue de lecture avant de fermer ses
  poignées du plein texte.
- Témoins (daemon_host.rs) : `un_backend_qu_on_laisse_tomber_ne_laisse_ni_catalogue_tenu_ni_dossier`
  (un backend de code indexé puis cherché) et `une_memoire_qu_on_laisse_tomber_avec_ses_reactions_ne_laisse_rien`
  (le cas d'e2e_cited_by) : rouges sur master (« 1 détenteur tient encore le catalogue après le
  Drop »), verts sur le correctif.
- e2e_cited_by et e2e_lectures sortent de la liste des résidus
  (`tests/e2e/suites_qui_laissent_des_residus.txt`), prouvé par deux e2e entiers propres.
  e2e_juge y reste : son résidu (des journaux de runs) a une autre cause, le journal des runs qui
  écrit après l'arrêt de son hôte (`2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`).
- Hors de ce lot : `ShardedHandle::close` de lucivy sort sur erreur avant d'arrêter ses pools (c7
  l'a dit à la session lucivy).
