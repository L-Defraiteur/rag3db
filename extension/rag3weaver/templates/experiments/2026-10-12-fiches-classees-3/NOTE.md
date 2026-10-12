# Tentative 3 des fiches classées — 12 octobre 2026, 4 h 47

*Même assemblage que la tentative 2, voie du LLM seule ; un seul changement, côté nœud :
`Decide` prend `max_tokens` (défaut 2 048, df1184d32) et dit « tronquée au plafond » une
réponse coupée. Le graphe (`decide_subject_llm.mmd`) est celui de la tentative 2, le
plafond laissé au défaut.*

## Ce qui a tourné

| | Decider (Qwen3-1.7B local, tentative 2) | **LLM (Gemini 3.5 Flash, Vertex)** |
|---|---|---|
| rappel des candidats | 29 / 31 | 29 / 31 |
| fiches justes | 25 / 39 | **32 / 39** |
| abstentions justes (fiches sans sujet) | 8 / 8 | **8 / 8** |
| ancrages justes / **faux** / manqués | 18 / **3** / 11 | **25 / 1 / 4** |
| décisions, temps | 117, 62 ms l'une | 117 en 626 s, **5,35 s** l'une |
| mémoire GPU | +2 859 Mio (llama-server) | aucune ici (embarquement par les services de luciepc) |

Aucune réponse tronquée, aucune étiquette hors liste : le plafond de 2 048 suffit à la
réflexion de Gemini 3.5 Flash sur ces questions. `resultats/llm-gemini.json` garde
chaque décision (`proba_verdict` vaut `null` : la sortie structurée ne rend pas de
probabilité).

## Les sept fiches qui ne sont pas justes

- **Deux viennent du rappel**, pas du juge : le bon sujet n'était pas parmi les trois
  candidats (« une suite nouvelle se monte par le script de génération… » → *tests de
  bout en bout* ; « bge-m3 rend des vecteurs de 1024 dimensions… » → *modèles
  d'embarquement*). Gemini s'est abstenu, ce qui est la bonne réponse sur ces candidats.
- **Quatre manqués**, le bon sujet présent et refusé : « commiter par chemins listés »
  et « pas de mention d'assistant dans les commits » sous *historique git* ; « les
  adresses du service d'embarquement s'écrivent complètes » sous *modèles
  d'embarquement* ; « une estimation en jours d'agent… » sous *préférences de
  communication*. Ce sont des fiches de **règle** dont le sujet est le domaine où la
  règle s'applique ; l'instruction (« seulement si elle parle de ce que le sujet
  décrit ; partager un mot ne suffit pas ») pousse Gemini à les refuser.
- **Un faux ancrage, à côté d'un juste** : « éditer un fichier que la suite indexe
  change ses résultats attendus » reçoit *oui* pour *tests de bout en bout* (juste) et
  pour *recherche plein texte*. Le oui / non par candidat permet deux sujets ; le banc
  compte le second comme faux.

**Gemini se trompe vers l'abstention**, le Decider vers le rangement : 1 faux ancrage
contre 3, 4 manqués contre 11. Pour un classeur, c'est le bon côté : un manqué laisse
une fiche sans sujet, un faux la range mal.

## Le retour

1. **Le coût** : 117 appels à ~5,4 s pour 39 fiches. La décision par fiche (« lequel des
   candidats, ou aucun », une sortie structurée à enum) diviserait les appels par trois
   et trancherait le double oui ; c'est la tentative 4.
2. **L'instruction** : dire qu'une règle se range sous le domaine où elle s'applique
   devrait reprendre une part des quatre manqués ; à mesurer dans la même tentative, en
   gardant les faux ancrages au compteur.
3. **Le rappel** (2 / 31 absents des candidats) se règle côté `recall_subjects`, pas
   côté juge.
