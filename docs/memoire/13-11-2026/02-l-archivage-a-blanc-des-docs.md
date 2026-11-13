# L'archivage à blanc des docs de la semaine — ce que l'archiviste ferait dans la mémoire de Lucie

*13 novembre 2026, session mémoire (96), à la demande de l'orchestration. Une proposition à
approuver ou à tailler, rien d'écrit chez Lucie : tout s'est fait sur une copie de sa mémoire, à
luciepc. La liste complète est en annexe : `02-annexe-proposition.md`.*

**En une phrase : avec la création allumée, l'archiviste ferait presque une fiche neuve par section
(326 pour 432), ne rangerait que 10 sections sous les 19 fiches qui existent, et ses regroupements
entre documents sont tantôt justes, tantôt faits sur un titre générique (« 1. Le jeu »). Telle quelle,
la proposition n'est pas à approuver en bloc ; le § 5 dit ce qui la rendrait utile.**

## 1. Le cadre

- **La copie** : l'instantané de G pris hôte arrêté le 13 novembre à 15 h 54
  (`~/.local/share/rag3weaver/instantanes/avant-207701bb2/memoire-longue`) et le dossier des fiches
  `~/memoire-longue`, copiés à luciepc dans `~/.cache/rag3weaver-build/blanc-96/` (une copie
  d'origine intacte, `copie-origine/`, à côté). 19 fiches au départ.
- **Le gabarit** : le gabarit `memory` de master, son manifeste pointé sur la copie, `models.llm` =
  `gemini-3.8-flash` par Vertex. Sa mémoire réelle est une copie du 25 octobre sans l'outil `archive` :
  le gabarit d'aujourd'hui s'ouvre sur sa base (première mesure de M8, acquise).
- **Le binaire** : la tête de master (c89e3f769 : le recul du modèle borné et dit, un refus 4xx gardé,
  les corrections de M8), en profil de développement, sous la lib 207701bb2.
- **L'outil** : `archive`, un appel par document, `allow_create: true` (sur la copie seulement ; le
  défaut est éteint), 5 candidates par section parmi 8 résultats de recherche, l'instruction par défaut.
- **Les documents** : le premier tiers demandé, les pages des dossiers de sessions du 6 au 9 novembre
  (`extension/rag3weaver/docs/0[6-9]-novembre-2026-*/`) : rapports, knowledge dumps, pages de
  conception. 52 documents, 432 sections.

## 2. Ce qui a tourné

| | |
|---|---|
| documents | 52 |
| sections décidées | 432 (un appel à Gemini chacune) |
| décisions abouties | 432, aucune erreur |
| refus du fournisseur | aucun 400 ; un seul recul (429 ou 503) dit au journal ; aucune requête refusée gardée |
| durée | 110,5 min, 15,3 s par section (de 10,5 s au premier document à 30 s au plus long) |
| sections jugées « none » (rien écrit) | 24 |
| sections rangées sous une fiche existante | 10 |
| sections rangées sous une fiche créée | 398 |
| **fiches créées** (état `proposed`, `via: archiviste`) | **326** |
| dont à une seule section | 274 |
| dont rassemblant plusieurs documents | 52 |

Le refus « 400 INVALID_ARGUMENT » vu le 9 novembre avec le binaire installé chez Lucie ne s'est
pas reproduit sur la tête de master, fiches du juge comprises dans les candidates. Sa cause n'est pas
lue ; si elle revient, le client garde maintenant la requête.

## 3. Ce que la proposition vaut

**Les 10 rangements sous une fiche existante sont justes.** Six sections des pages de hooks-essai
vont sous la fiche de cette session (« hooks-essai ») ; quatre sections de la page des essais de
l'équipier (essais 1, 3, 4, 5) vont sous les bilans du juge de ces mêmes tâches.

**Les fiches créées portent le titre de leur section** (le nom d'une fiche proposée, par
conception). Gemini n'invente pas de nom ; mais un titre de section n'est pas un sujet : 95 des
326 noms font moins de 25 caractères, comme « 7. Pour rejouer », « Le témoin », « La forme »,
« Ce qui existe », « En file ».

**Les regroupements entre documents sont de deux sortes** (52 fiches) :
- *justes* : « Les hooks de Claude Code — la mémoire qui parle à la session » rassemble la
  conception et les lots 1 et 2 ; « L'équipier, agent de code fonctionnel » rassemble trois pages
  d'everything declarative et de tty ; « La liste de la semaine du cœur » trois pages qui la citent ;
- *faits sur un titre générique* : « 1. Le jeu » réunit deux pages du banc et ma page de la précision
  du rappel, qui parlent de jeux différents ; « 0. Le 8 novembre », « 2. Décisions prises ou reçues »,
  « Ce qui est sur master » réunissent des sections sans autre lien que leur titre.

**Une section ne voit jamais les fiches que créent les sections précédentes du même document** :
aucune fiche neuve ne réunit deux sections d'un même document. L'outil décide toutes les sections
d'un document, puis écrit ; un document de dix sections donne jusqu'à dix fiches. C'est la cause
des 274 fiches à une seule section, plus que le jugement de Gemini.

## 4. Ce que ça confirme

La tentative 5 du banc des fiches classées (9 novembre) l'avait annoncé : Gemini répond `create`
partout où il ne range pas. Ici, 75 % des sections deviennent une fiche. C'est pour cela que
`allow_create` est éteint par défaut ; sans la création, l'archiviste aurait écrit 10 apports
justes, 398 sections seraient restées dans leurs Sources (rien n'est perdu : le texte est en L0), et
24 jugées sans sujet.

## 5. Ce qui rendrait la proposition utile, à trancher par Lucie

1. **Archiver sans création, d'abord** : les 10 rangements justes et les 52 Sources (les documents
   entiers, retrouvables par `cited_by` et la recherche), rien de plus. C'est sûr, et c'est maigre.
2. **Une fiche par document, pas par section** : un document de session devient une fiche nommée par
   son titre (« Rapport de la session hooks-essai », « M8 — la montée des gabarits »), ses sections en
   apports. 52 fiches au lieu de 326, aux noms qui sont des sujets.
3. **Une création qui voit le document** : décider les sections dans l'ordre et écrire au fil, pour
   qu'une section puisse rejoindre la fiche créée par la précédente ; et une création qui exige un
   nom de sujet proposé par le modèle, pas le titre de la section.
4. **Les sujets choisis par Lucie** : elle approuve dans l'annexe les 52 regroupements et les fiches
   qu'elle veut garder, et l'archivage réel ne crée que celles-là.

Les points 2 et 3 sont du code (l'outil `archive`) ; le point 1 peut se faire dès sa montée de
gabarit (M8) ; le point 4 demande son temps à elle.

## 6. Ce qui reste sur disque

À luciepc, `~/.cache/rag3weaver-build/blanc-96/` : `copie/` (la mémoire après l'archivage à
blanc), `copie-origine/` (avant), `templates/`, `src/` (les 52 documents figés), `lancer.py`,
`resultats.jsonl` (une ligne par document), `apports.json` (les 408 apports lus dans la copie). Le
binaire est `~/.cache/rag3weaver-build/target-96-m8/debug/rag3weaver-backend`. Rien n'a touché la
mémoire de Lucie, son hôte ni ses fichiers.
