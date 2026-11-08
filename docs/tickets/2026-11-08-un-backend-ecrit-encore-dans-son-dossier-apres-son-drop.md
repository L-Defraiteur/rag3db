# Un backend écrit encore dans son dossier pendant qu'on l'efface, après son Drop

- **État** : ouvert (8 novembre 2026, session recherche), cause à départager.
- **Famille** : produit
- **Gravité** : confort pour les tests (des TempDir à moitié effacés dans /tmp, qui est de la
  mémoire vive) ; plus grave s'il dit qu'un backend fermé garde sa base ouverte par un fil.
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

## Ce qui reste

Départager ce qui écrit après le Drop du backend : la base encore tenue par un fil (le moteur
recrée son `.readers` à l'ouverture d'une lecture), l'écriture différée d'un run du journal (une
grâce de 250 ms), le plein texte qui finit son lot. Puis faire attendre ses fils à l'arrêt d'un
backend. **C'est le même défaut** que celui relevé par hotfixs en relisant la branche de
l'équipier `agent/tests-bases-dans-tmp` (« un `drop(base)` efface la base sous un démon encore
vivant », docs/08-novembre-2026-orchestration/01-essais-de-l-equipier.md) : un seul correctif pour
les deux, chez hotfixs, qui complète cette branche. Les trois suites sortent alors de la liste des
résidus, dans le commit du correctif, prouvé par plusieurs passes propres.
