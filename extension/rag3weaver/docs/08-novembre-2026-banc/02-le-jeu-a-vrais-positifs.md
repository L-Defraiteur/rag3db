# Un petit modèle de décision : le jeu à vrais positifs (8 novembre 2026, après-midi)

Suite de la page 01. Le premier jeu n'avait aucun vrai positif. Lucie, 12 h 05 : « il faut
fabriquer de fausses fiches, ou en construire de vraies ». Puis, 12 h 20 : « on lui donne tout le
chat et les outils, et on lui dit : ce bout de mémoire est-il pertinent avec le contexte de
l'agent ». La question ci-dessous est la sienne, mot pour mot.

> **Suite** : les modèles plus gros (Qwen2.5-1.5B, Qwen3-4B), la fin de conversation, le préfixe partagé, la liste et la recommandation finale sont dans la page 03 (`03-le-juge-local.md`).

**En une phrase : aucun petit modèle ne sait le faire en zéro-coup.** Qwen2.5-0.5B, SmolLM2-360M et
Laya sont au hasard ; leur score suit le contexte et pas la fiche. Gemini 3.8 Flash retrouve la
bonne fiche dans 81 à 84 % des cas. Le meilleur candidat local est le dense bge-m3 déjà en service
(41 % contre 20 % pour le hasard), mais il ne vaut rien comme filtre.

## 1. Le jeu

- **Les positifs, construits** :
  - 130 prompts réels de 12 sessions rag3db (graine fixe, au plus 16 par session, les annonces
    écartées).
  - Pour chacun, Gemini lit les tours suivants de la même session, jusqu'à 32 000 caractères, et
    écrit la fiche qui aurait aidé dès le premier tour : nom, résumé, corps, provenance. Ou il
    répond « aucune ».
  - La consigne interdit d'emprunter les mots du prompt dans le nom et le résumé.
  - La provenance est vérifiée ici : l'extrait cité doit se trouver mot pour mot dans le tour
    désigné.
  - Résultat : 95 fiches utiles, dont **88 à provenance vérifiée, les positifs**.
- **Les négatifs** : pour chaque positif, les 2 fiches construites d'autres sessions les plus
  proches par le dense bge-m3 (même sujet, autre travail), et 2 au hasard d'autres sessions. Soit
  **440 paires**, 5 candidates par cas.
- **Le contexte de l'agent** : le prompt et les tours d'avant, à rebours, les sorties d'outils
  réduites à leur première ligne. Il est borné à 2 000, 6 000 et 16 000 jetons (estimés à
  3,5 caractères par jeton ; p50 de 6 800, 20 900 et 36 200 caractères).
- **La forme courte**, pour ce qui ne lit que 512 jetons : la dernière phrase de l'agent et le
  dernier message reçu (1 000 caractères au plus).
- **Le découpage** : par session, 70/30, graine fixe. La part de test compte 3 sessions et
  170 paires, dont 34 positifs. La part de mise au point (54 positifs) n'a servi à rien : aucune
  spécialisation n'a été lancée.
- **La limite** : 88 positifs, c'est un petit jeu. La graine permet de l'étendre à d'autres sessions
  (celles de luciepc, LR_CodeRag).

## 2. Les mesures

Dans chaque cas, le positif est-il le premier de ses 5 candidates ? Le hasard donne 0,20 ; l'AUC
dans le cas, 0,50. La vérité est le positif construit, et Gemini n'est qu'un candidat de
référence.

| candidat | positif premier | AUC dans le cas | temps | où |
|---|---|---|---|---|
| Gemini 3.8 Flash, 5 candidates en liste | positif retrouvé 0,81 à 0,84, précision 0,90 à 0,91 | — | p50 4,4 à 4,8 s par cas | Vertex |
| dense bge-m3, cosinus (forme courte contre fiche) | 0,41 | 0,70 | p50 66 ms par cas, 154 ms pour embarquer le contexte | service en place |
| relecteur bge-reranker-v2-m3 (forme courte) | 0,38 | 0,62 | p50 66 ms par cas (5 paires) | service en place |
| relecteur, dernier message seul | 0,36 | 0,64 | p50 57 ms par cas | service en place |
| Qwen2.5-0.5B, logits oui/non, borne 2 000 | 0,23 | 0,57 | p50 3,3 s par paire | CPU, luciepc chargé |
| Qwen2.5-0.5B, forme courte | 0,20 | 0,57 | p50 0,54 s par paire | CPU |
| SmolLM2-360M, borne 2 000 | 0,17 | 0,49 | p50 3,9 s par paire | CPU |
| SmolLM2-360M, forme courte | 0,22 | 0,51 | p50 0,54 s par paire | CPU |
| Laya (laya-multilingual, noul), borne 2 000 | 0,20 | 0,51 | p50 1,4 s par paire | CPU |
| Laya, forme courte | 0,17 | 0,48 | p50 0,27 s par paire | CPU |

- **Gemini selon la borne** : il retrouve le positif dans 81 % des cas à 2 000 jetons, 84 % à
  6 000 et 81 % à 16 000, avec une précision de 0,90 à 0,91. Il ne garde que 7 ou 8 des
  176 négatifs durs. Sur la part de test : 85, 91 et 82 %.
- **Les petits modèles lisent le contexte, pas la fiche.** Leurs moyennes sont presque égales pour
  les positifs, les durs et le hasard : 0,194, 0,190 et 0,185 pour Qwen ; 0,937, 0,939 et 0,938
  pour SmolLM2 ; 0,976, 0,975 et 0,968 pour Laya. Le score varie d'un cas à l'autre, pas d'une
  fiche à l'autre dans un même cas. Aucun seuil n'a de sens pour eux : garder 95 % des utiles
  n'ôte que 1 à 6 % des non utiles.
- **Le dense bge-m3 comme filtre qui ôte seulement**, la règle « un filtre ôte, il ne promeut
  pas » :
  - en forme courte, le seuil qui garde 100 % des utiles (cosinus 0,327) n'ôte que 3 % des non
    utiles (7 % sur le test), et le seuil qui en garde 95 % (0,366) en ôte 14 % ;
  - à la borne de 2 000, le seuil de 100 % ôte 6 % des non utiles (24 % sur le test), mais
    l'embarquement du contexte prend alors 20 s au p50, sur une carte chargée ;
  - à 6 000, le service refuse la requête (500, corps vide).
- **Mémoire, en CPU** : pic de 3,0 à 4,8 Gio de RSS par modèle, chargement en 3 à 10 s (192 s une
  fois, pour Qwen sous charge).
- **La charge** : luciepc était à 21 à 47 pendant les passes CPU (8 fils), d'où les latences
  notées chargées. Lucie jouait sur card0 par moments.
- **Le contexte ROCr** : la passe CPU masquait les GPU (`ROCR_VISIBLE_DEVICES` et
  `HIP_VISIBLE_DEVICES` à -1). Il reste 44 Kio de contexte ROCr sur chaque carte, sans aucun calcul
  (sans le réglage : 68 Kio).

## 3. La passe GPU, interrompue

- **Ce qui a tourné** : sur card2 (04:00.0, celle des services), Laya seul, 253 paires sur 440 à
  6 000 jetons et autant à 16 000 (p50 197 et 275 ms par paire, p95 10,6 s). Puis il s'est bloqué
  à 19 h 23 sur une paire (borne 6 000, 21 000 caractères) : la carte restait à 100 % et rien ne
  s'écrivait plus.
- **L'effet sur les services** : dans le même temps, le service granite-278m de la même carte est
  passé à 12 puis 19 s par texte, et les passes des autres sessions ont rougi sur l'indexation.
  Passe arrêtée à 20 h 20.
- **Après l'arrêt** : card2 restait à 94 % et le 7878 à 9 s. Les passes concurrentes de la file
  saturaient la carte elles aussi (l'orchestration a abaissé le plafond des lourds à 3).
- **Aucun reset gfx au journal du noyau.**
- **La reprise**, autorisée par Lucie : sur card0, la moitié de la VRAM laissée à son jeu, et une
  limite de 30 s par paire (au-delà, la paire est notée « dépassée » au lieu de tenir la carte).
  Viendront Qwen 0,5B, SmolLM2 et Laya à 6 000 et 16 000 jetons, puis Qwen2.5-1.5B-Instruct
  (Apache 2.0, 2,9 Go, déjà téléchargé).

## 4. Le coût

| passe | appels | jetons d'entrée | sortie | dont réflexion |
|---|---|---|---|---|
| construction des fiches | 130 | 1 699 422 | 20 421 | 297 087 |
| référence, 88 cas × 3 bornes | 264 | 1 815 089 | 19 700 | 99 420 |
| total de cette page | 394 | 3 514 511 | 40 121 | 396 507 |

- **Les refus de quota** : tous les appels en série, 1,5 s entre deux. 4 refus 429, passés au
  second essai ; l'attente est maintenant d'une minute, puis le double.
- **Le plafond de jetons** : à 4 000, la construction coupait encore (3 837 jetons de réflexion),
  d'où un plafond de 8 000. La référence n'a eu aucune coupure.
- **La consigne de la référence** : celle du juge du rappel (la corrigée d'archi, le (g), n'était
  pas sur master), lue sur le contexte de l'agent, avec la question de Lucie et un plafond de
  8 000.
- **Les téléchargements** : `transformers` et `laya` dans un venv à luciepc ; laya-multilingual
  (647 Mo), Qwen2.5-0.5B (954 Mo) et SmolLM2-360M (694 Mo, les formats ONNX inutiles effacés :
  4,7 Go avant) ; puis Qwen2.5-1.5B (2,9 Go). Tous sous Apache 2.0.
- **Le torchvision du système** ne va pas avec torch 2.15 et transformers l'importait. Il est masqué
  dans le script seulement.

## 5. Ce que je recommande pour « ce bout de mémoire est-il pertinent avec le contexte de l'agent »

1. **Ni Qwen2.5-0.5B, ni SmolLM2-360M, ni Laya en zéro-coup** : au hasard, sur toutes les formes
   de contexte mesurées.
2. **Pas de filtre local par le dense bge-m3** : pour garder toutes les fiches utiles, il n'ôte
   presque rien (3 %). Il peut ranger les candidates (41 % de premiers), pas les écarter.
3. **Le juge reste Gemini aujourd'hui**, avec la consigne qui dit la session servie et un plafond
   qui laisse la place à la réflexion (ticket d596c829c).
4. **À mesurer ensuite** :
   - (a) Qwen2.5-1.5B par logits, sur card0, puis 3B s'il tient ;
   - (b) Laya spécialisé, quand le jeu aura des positifs en nombre (d'autres sessions,
     LR_CodeRag) : 54 positifs de mise au point ne suffisent pas.

## Fichiers

- **Sur le portable**, dans `~/.cache/rag3db-banc-notes/decision/` :
  - les scripts : `g_construire.py`, `i_paires.py`, `j_reference.py`, `k_bge2.py`,
    `l_analyse.py` ;
  - les données : `paires2.json`, `construites.json`, `reference.json`, `bge2.json`, `dense.json`,
    `petits-*.jsonl`.
- **À luciepc**, dans `~/.cache/rag3db-banc-notes/` : `h_petits.py`, les poids et le venv.
- **Le jeu reste hors du dépôt** : il porte des prompts de sessions.
