# Un backend écrit encore dans son dossier pendant qu'on l'efface, après son Drop

- **État** : ouvert (8 novembre 2026, session recherche), cause à départager.
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
