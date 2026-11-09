# Tentative 5 des fiches classées — 9 novembre 2026, jouée à luciepc de 9 h 58 à 10 h 35

*La question que pose l'outil `archive` (pas 3 de M4, ff7a18fe2) : que fait le modèle d'une
fiche dont aucun candidat ne porte le sujet ? Même jeu que la tentative 4
(`tests/data/banc-fiches-classees/v1.json`, 39 fiches dont 8 sans sujet, 10 sujets, trois
candidats par `recall_subjects`), même forme par fiche, même instruction, plus l'étiquette
`create`. Lib 9bae60786, `gemini-3.8-flash` sur Vertex. Les décisions brutes sont dans
`resultats/llm-gemini.json`.*

## Ce qui change

- **Les étiquettes fixes** sont celles d'`archive` : `none` et `create` (`tentative.json` les
  nomme ; la tentative 4 disait `aucun` et n'avait pas de création).
- **L'instruction** garde celle de la tentative 4 et ajoute la phrase de création d'`archive` :
  « si le sujet de la fiche est net et qu'aucun candidat ne le porte, réponds create ».
- **Le compte** reste celui de la tentative 4, plus trois nombres : les créations sur une fiche
  sans sujet, sur une fiche dont le bon sujet manque aux candidats, et les **créations
  fautives**, quand le bon sujet était candidat. Une création ne compte pas comme abstention ;
  sur une fiche sans sujet, elle ne range rien à tort et la fiche reste comptée juste.

## La tentative 4 rejouée d'abord

Le banc changé, sur le dossier de la tentative 4, dans la même passe :

| | tentative 4 (18 oct.) | **tentative 4 rejouée (9 nov.)** |
|---|---|---|
| rappel des candidats | 29 / 31 | **29 / 31** |
| fiches justes | 36 / 39 | **35 / 39** |
| abstentions justes | 8 / 8 | **8 / 8** |
| ancrages justes / faux / manqués | 28 / 0 / 1 | **27 / 0 / 2** |
| temps par décision | 9,6 s | **30,0 s** |

Le compte est le même ; l'écart d'une fiche s'explique ligne à ligne. La recherche ne rend plus
les mêmes trois candidats pour 30 fiches sur 39 (le premier est presque toujours le même, les
suivants ont changé avec le moteur), et le rappel garde son total en échangeant deux fiches :
« bge-m3 rend des vecteurs de 1024 dimensions » a maintenant son sujet parmi les candidats, et
Gemini l'y range ; « jamais plus de huit tâches de compilation » l'a perdu, et Gemini répond
`aucun`, la bonne réponse sur ces candidats. Gemini lui-même change une seule décision : « les
adresses du service d'embarquement s'écrivent complètes » n'est plus rangée sous *modèles
d'embarquement* (un manqué de plus). Les décisions prennent trois fois plus de temps qu'en
octobre : c'est Vertex, les 39 recherches gardent leurs 3 s.

## Ce qui a tourné

| | tentative 4 rejouée | **tentative 5** |
|---|---|---|
| fiches justes | 35 / 39 | **35 / 39** |
| abstentions justes | 8 / 8 | **0 / 8** |
| ancrages justes / faux / manqués | 27 / 0 / 2 | **27 / 0 / 2** |
| créations sur une fiche sans sujet | — | **8 / 8** |
| créations hors du rappel | — | **2 / 2** |
| **créations fautives** | — | **2** |
| temps par décision | 30,0 s | 25,5 s |

Aucune réponse tronquée, aucune étiquette hors liste.

**Gemini crée partout où il ne range pas.** Les huit fiches sans sujet reçoivent toutes `create`,
« la personne préfère le thé au café l'après-midi » comprise, comme le connecteur Slack ou le
rechargement du gestionnaire de fenêtres. Les deux fiches dont le bon sujet manquait aux
candidats aussi. Et deux fois il crée alors que le bon sujet était candidat :
- « les commentaires, commits et pages sont en français, les identifiants en anglais », que la
  tentative 4 rejouée rangeait sous *documentation* ;
- « une estimation en jours d'agent se rend comme un ordre de grandeur », sous *préférences de
  communication* : le manqué de toutes les tentatives, devenu une création.

**Les ancrages ne bougent pas** : 27 justes, aucun faux. L'étiquette nouvelle ne prend rien aux
rangements justes, sauf ces deux-là ; elle prend toutes les abstentions. « Les adresses du
service d'embarquement », manquée à la tentative 4 rejouée, est rangée juste ici : sur une
fiche à la limite, la décision de Gemini varie d'une passe à l'autre.

## Ce que la mesure tranche

La règle de la page 06 de la mémoire longue (§6) : si le modèle crée là où il devrait
s'abstenir, `archive` démarre avec la création éteinte, et le dit. Il crée sur toutes les
fiches sans sujet et sur deux fiches dont la fiche existait : **`allow_create` est éteint par
défaut** (`archive.mmd`, même commit). Une section sans candidate juste devient `none`, son
texte reste dans la Source (L0), et `allow_create: true` reste possible à l'appel.

## Le retour

1. **Une création se demande autrement** : pas « create si le sujet est net », que Gemini lit
   comme « create sinon », mais un second temps, sur les seules sections `none` d'un document :
   « ce texte mérite-t-il une fiche à lui ? », avec des exemples d'une fiche qui n'en mérite pas
   (une préférence passagère). À mesurer sur les huit fiches sans sujet, en gardant les créations
   fautives au compteur.
2. **Le jeu ne dit pas lesquelles des huit méritent une fiche.** Une tentative de création a
   besoin d'un or pour elles : un champ `create` dans le jeu, posé par Lucie.
3. **Le rappel** reste le premier facteur d'erreur (deux fiches sur 31 sans leur sujet parmi
   les candidats) et bouge avec le moteur : à régler côté `recall_subjects`.

## Comment la rejouer

À luciepc, la lib des passes épinglée (`exporter_la_lib`), avec le service d'embarquement et
les identifiants Vertex. Le banc est une suite « à services » : `run_e2e.sh` ne la joue que
nommée par `--test`, et la refuse sans sa voie.

    cd extension/rag3weaver
    RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_LLM=vertex \
    RAG3WEAVER_BANC_TENTATIVE=templates/experiments/2026-11-09-fiches-classees-5 \
    RAG3WEAVER_BANC_SORTIE=templates/experiments/2026-11-09-fiches-classees-5/resultats/llm-gemini.json \
    ./run_e2e.sh --test e2e_banc_fiches_classees

39 décisions, environ seize minutes ce matin.
