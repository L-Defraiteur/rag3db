# Les modèles de décision : trois pistes, sur trois jeux à la fois

**3 octobre 2026, session « optimiseur ».** Dernier tour de l'exploration,
après [le document 04](04-hors-du-jeu-qui-l-a-vu-naitre.md). Rien n'est réglé
sur un jeu : les mêmes gabarits partout. La synthèse des cinq documents est
dans [00-ou-en-est-la-question.md](00-ou-en-est-la-question.md).

**Ranger un texte parmi des sujets existants marche, et le cosinus le fait
aussi bien que JevK5. Décider qu'il faut créer un sujet ne marche par aucune
des trois pistes, sur aucun des trois jeux.**

## Les trois jeux

| jeu | d'où il vient | sujets | textes à ranger | textes à créer |
|---|---|---|---|---|
| le nôtre | écrit par moi (document 03) | 10 thèmes de travail | 16 | 8 |
| le journal | `docs/journal-des-chantiers.md`, bâti par script (document 04) | 5 sections, qui sont des états d'avancement | 32 | 15 |
| le public | MASSIVE, des demandes à un assistant vocal rangées par des annotateurs sous 18 scénarios ; reprise `mteb/amazon_massive_scenario` (carte : Apache-2.0 ; jeu d'origine CC-BY-4.0) | 10 scénarios tirés au sort | 30, moitié français, moitié anglais | 12, de 3 scénarios retirés |

Le tirage du jeu public est dans `banc/tirer_public.py` (graine 2026) : les
scénarios sont mélangés, les dix premiers sont les sujets (email, calendar,
alarm, audio, recommendation, datetime, play, iot, transport, general), les
trois suivants sont retirés (lists, qa, music). Ce n'est pas notre domaine ;
c'est le seul des trois dont les sujets sont des thèmes et dont les étiquettes
ne sont pas de ma main.

Chaque sujet est montré de deux façons : son **nom seul**, ou son **texte
complet** (nom, description quand il y en a une, et deux textes rangés dessous,
annoncés « For example, among many other texts », la formule que le document
04 a trouvée la moins nuisible). JevK5 reçoit les quatre premiers sujets du
cosinus, sans que le bon y soit ajouté s'il manque.

## Ranger parmi les existants : qui met le bon sujet en premier

| | le nôtre (16) | le journal (32) | le public (30) |
|---|---|---|---|
| cosinus granite, noms seuls | 8 | 7 | 15 |
| cosinus granite, **textes complets** | **14** | **19** | **22** |
| reranker bge, noms seuls | 5 | 12 | 19 |
| reranker bge, textes complets | 12 | 16 | 22 |
| JevK5, noms seuls | 10 | 13 | 21 |
| JevK5, textes complets | 12 | 15 | 20 |

- **La piste (c) tient sur les trois jeux pour les références** : donner le
  texte complet du sujet plutôt que son nom fait passer le cosinus de 8 à 14,
  de 7 à 19, de 15 à 22. Pour JevK5, le gain est faible ou nul.
- **Avec les textes complets, le cosinus range aussi bien ou mieux que
  JevK5** sur les trois jeux, sans modèle de plus et sans carte.

## Piste (a) : « aucun de ces sujets » à la place du sujet neuf nommé

JevK5, quatre sujets existants plus l'option « none of these topics ».

| | rejoint le bon | crée à tort | range ailleurs | textes neufs reconnus |
|---|---|---|---|---|
| le nôtre, noms | 10 sur 16 | 2 | 4 | 3 sur 8 |
| le nôtre, complets | 12 sur 16 | 0 | 4 | 3 sur 8 |
| le journal, noms | 13 sur 32 | 3 | 16 | 4 sur 15 |
| le journal, complets | 16 sur 32 | 1 | 15 | 2 sur 15 |
| le public, noms | 21 sur 30 | 1 | 8 | **0 sur 12** |
| le public, complets | 20 sur 30 | 1 | 9 | 3 sur 12 |

**Le biais s'inverse.** Quand le sujet neuf portait un nom fait des mots du
texte, le modèle créait à tort ; quand l'option est anonyme, il ne crée
presque plus jamais : au mieux un texte neuf sur trois est reconnu, et sur le
jeu public aucun. Retirer le nom ne règle pas le point faible, il le déplace.

## Piste (b) : pas d'option « créer », la création lue dans la faiblesse du meilleur score

On range toujours ; un texte est dit neuf si le meilleur score est faible.
AUC : 1,00 si tous les textes à ranger ont un meilleur score plus haut que
tous les textes neufs, 0,50 au hasard.

| AUC du meilleur score | le nôtre | le journal | le public |
|---|---|---|---|
| cosinus, noms | 0,57 | 0,55 | 0,73 |
| cosinus, complets | 0,66 | 0,66 | 0,80 |
| reranker, noms | 0,53 | 0,60 | 0,68 |
| reranker, complets | 0,65 | 0,74 | 0,72 |
| JevK5, noms | 0,80 | 0,72 | 0,68 |
| JevK5, complets | 0,79 | 0,67 | 0,79 |

La courbe, pour le meilleur de chaque jeu, avec les textes complets : si
l'on place le seuil de façon à attraper telle part des textes neufs, que
deviennent les textes à ranger ?

| | textes neufs attrapés | bien rangés | rangés ailleurs | créés à tort |
|---|---|---|---|---|
| le nôtre, JevK5 (16 à ranger) | 8 sur 8 | 9 | 0 | 7 |
| | 6 sur 8 | 9 | 3 | 4 |
| | 4 sur 8 | 10 | 3 | 3 |
| le journal, reranker (32) | 15 sur 15 | 7 | 3 | 22 |
| | 12 sur 15 | 9 | 9 | 14 |
| | 8 sur 15 | 12 | 10 | 10 |
| le public, cosinus (30) | 12 sur 12 | 10 | 1 | 19 |
| | 9 sur 12 | 17 | 2 | 11 |
| | 6 sur 12 | 22 | 6 | 2 |

**Aucun point de la courbe n'est bon.** Pour attraper tous les textes neufs,
il faut créer à tort pour la moitié des textes qui avaient leur sujet ; pour
n'en créer presque aucun à tort, il faut laisser passer la moitié des neufs.
Aucun des trois modèles ne fait mieux que les autres avec constance : le
meilleur change d'un jeu à l'autre.

## Piste (c) sur les mémoires : le fait et son pourquoi, plutôt que le titre

Les 48 paires du banc de la mémoire longue. « Titre » : la mémoire existante
est son énoncé court. « Complet » : l'énoncé et son pourquoi.

| | JevK5, AUC de P(même) | cosinus, AUC | reranker, AUC | redites dites « même » ou « complète » par JevK5 |
|---|---|---|---|---|
| titre | 0,80 | **0,99** | 0,91 | 3 sur 6 |
| complet | 0,92 | 0,83 | 0,90 | 5 sur 6 |

- **Avec le pourquoi, JevK5 remonte** (0,80 → 0,92) et reconnaît cinq
  redites sur six comme une redite ou un complément.
- **Sur les titres seuls, le cosinus sépare presque sans faute** (0,99) :
  deux énoncés courts qui disent la même chose sont proches, et le reste est
  loin. Avec le pourquoi, il baisse (0,83) : le pourquoi ajoute des mots que
  la redite n'a pas.
- Même au meilleur, trois redites sur six seulement passent au-dessus de
  toute autre paire : un seuil sans fausse fusion laisse la moitié des
  doublons.
- Les deux contradictions sont peu nombreuses pour en dire plus : ce jeu ne
  dit pas si le cosinus les prend pour des redites, ce que mes vingt paires
  montraient (0,85 contre 0,83).

## Ce que ce tour établit

1. **Ranger parmi les sujets existants est un problème d'ordre, et le
   cosinus sur le texte complet des sujets y suffit**, sur les trois jeux.
2. **« Faut-il créer un sujet ? » n'a de réponse fiable par aucune voie
   essayée** : nommé, le neuf attire ; anonyme, il n'est jamais choisi ; lu
   dans le score, il ne se sépare pas.
3. **Pour les mémoires, ce qu'on montre au modèle compte autant que le
   modèle** : le pourquoi aide JevK5 et gêne le cosinus.
