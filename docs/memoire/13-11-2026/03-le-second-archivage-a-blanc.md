# Le second archivage à blanc — l'archiviste qui voit son document et nomme ses sujets

*13 novembre 2026, session mémoire (96). Le même essai que la page 02 du même dossier, refait avec
l'archiviste corrigé (master cd51ab5cd), pour que Lucie compare les deux listes et tranche la
politique par défaut. Rien d'écrit chez elle : une copie de sa mémoire, à luciepc. La liste complète
est en annexe, `03-annexe-second-essai.md`.*

**En une phrase : 113 fiches au lieu de 326, presque toutes nommées par un vrai sujet, et des
documents rangés en quelques fiches au lieu d'une par section. Il reste un défaut net : d'un
document à l'autre, l'archiviste ne retrouve pas toujours la fiche qu'il a lui-même créée, et le
même sujet existe sous deux à quatre noms voisins.**

## 1. Ce qui a changé entre les deux essais

Deux corrections, poussées avant ce second essai (ticket chacune, dans `docs/tickets/closed/`) :

- **Une section voit les fiches proposées plus haut dans son document** : elles sont ajoutées à ses
  candidates, et elle peut les rejoindre au lieu d'en créer une autre.
- **Une fiche créée porte le nom du sujet que le modèle donne**, pas le titre de sa section.

Le reste est identique : la même copie d'origine (l'instantané du 13 novembre à 15 h 54), les mêmes
52 documents, `allow_create: true` sur la copie seulement, `gemini-3.8-flash` par Vertex. La lib est
835d5298f (posée entre les deux essais) au lieu de 207701bb2.

## 2. Les deux listes, côte à côte

| | premier essai | **second essai** |
|---|---|---|
| sections décidées | 432, sans erreur | 432, sans erreur |
| durée | 110,5 min, 15,3 s par section | **75,8 min, 10,5 s** |
| reculs du modèle | 1 | 0 |
| sections jugées « none » | 24 | 16 |
| sous une fiche existante | 10 | 9 |
| **fiches créées** | **326** | **113** |
| dont à une seule section | 274 | 45 |
| dont réunissant plusieurs sections d'un même document | 0 | 31 |
| dont réunissant plusieurs documents | 52 | 37 |
| noms de moins de 25 caractères | 95 | 33 |

## 3. Ce que vaut la seconde liste

**Les noms sont des sujets.** « Bascule au format 43 », « Montée des gabarits sur le gabarit
vivant », « Injection automatique de l'index de code », « Vues par worktree », « Surlignage en
creux », « Thème daemon pour le terminal et l'IDE ». Les noms courts qui restent sont des sujets
courts (« L'atelier », « Jauge de contexte », « Orchestration »), plus des titres de section.

**Les documents se rangent en peu de fiches.** Le plan du week-end (7 sections) donne 3 fiches au lieu
de 6 ; « Journal du 8 novembre 2026 » réunit les 30 sections du rapport d'orchestration du 8 ; la
page de M8 se range presque entière sous « Montée des gabarits sur le gabarit vivant ».

**Les ancrages aux fiches existantes restent justes** : huit sections des pages de hooks-essai sous
« hooks-essai », une section des essais de l'équipier sous son bilan du juge.

**Le défaut qui reste : les doublons d'un document à l'autre.** Le même sujet existe sous plusieurs
noms voisins, créés par des documents différents :

- « Hooks Claude Code » (12 sections), « Hooks Claude Code de rag3weaver » (11), « Hooks de Claude
  Code » (11), à côté de « Rappel différé et juge du rappel » et « Règles de sélection du rappel par
  les hooks » ;
- « Nettoyage des worktrees et du disque », « … et espace disque », « … et caches », « Nettoyage des
  branches distantes » ;
- « Classeur local du rappel » (11) et « Classeur de rappel » (1) ; « Juge local pour le rappel de
  mémoire » et « Juge de pertinence de la mémoire » ;
- « Session hooks-essai » et « Bilan de la session hooks-essai », à côté de la fiche existante
  « hooks-essai ».

Chaque document est validé avant le suivant : la fiche créée par l'un est donc en base, et
cherchable, quand l'autre décide. Mais la recherche des candidates part du texte de la section, et la
fiche neuve n'a pour description que la première phrase de la section qui l'a créée : elle ne remonte
pas toujours parmi les cinq candidates, et le modèle crée un nom voisin.

**Les fiches très larges** : « Journal du 7 novembre 2026 » (20 sections) et « Journal du 8 novembre
2026 » (30) sont des rapports de session rangés tels quels. C'est juste comme provenance, moins comme
sujet : le contenu de ces rapports touche une dizaine de sujets.

## 4. Ce que Lucie tranche

La politique par défaut de l'archiviste, sur sa vraie mémoire, une fois M8 posé :

1. **Sans création** (le défaut d'aujourd'hui) : 9 à 10 rangements justes et les 52 Sources, rien de
   plus. Sûr, maigre.
2. **Une fiche par document** : 52 fiches nommées par les titres des documents ; ce que le second
   essai fait déjà pour les rapports de session, sans le faire pour les autres.
3. **La création qui voit le document et nomme un sujet** (ce second essai) : 113 fiches, utiles, avec
   des doublons à fusionner. Deux corrections les réduiraient : chercher les candidates aussi par le
   nom de sujet que le modèle propose avant de créer, et rejoindre une fiche dont le nom est voisin ;
   ou une passe de fusion des fiches `proposed` voisines avant de les accepter.
4. **Ses choix dans l'annexe** : elle garde, fusionne ou retire les fiches de la seconde liste, et
   l'archivage réel n'écrit que celles-là.

Les fiches créées restent `proposed` dans tous les cas : rien ne devient `current` sans elle.

## 5. Ce qui reste sur disque

À luciepc : `~/.cache/rag3weaver-build/blanc-96-2/` (ce second essai : `copie/`, `copie-origine/`,
`templates/`, `src/`, `lancer.py`, `resultats.jsonl`, `apports.json`) et `blanc-96/` (le premier,
intact). Le binaire de la tête est `~/.cache/rag3weaver-build/target-96-m8/debug/rag3weaver-backend`.
Rien n'a touché la mémoire de Lucie, son hôte ni ses fichiers.
