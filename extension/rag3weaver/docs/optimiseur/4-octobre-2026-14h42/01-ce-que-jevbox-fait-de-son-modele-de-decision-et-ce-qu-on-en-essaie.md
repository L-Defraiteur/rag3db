# Ce que jevbox fait de son modèle de décision, et ce qu'on en essaie

4 octobre 2026. Lecture du dépôt `extend-hq/jevbox` (créé le 29 septembre
2026, gabarit à forker, **sans licence à la racine : on lit, on ne copie
pas**). Fichiers lus : `docs/retrieval.md`, `docs/organization.md`,
`docs/search-quality.md`, les trois comptes rendus de mesure, `server/jev.ts`,
`server/beam-search.ts`.

Suite des documents sur les modèles de décision
(`../3-octobre-2026-20h55/`). Rien n'est codé ; ce document dit ce qu'on
essaie et dans quel ordre.

## 1. Ce que c'est

Une bibliothèque de documents où toute la recherche passe par le modèle de
décision : ni vecteurs, ni index de mots. Le modèle est le service hébergé de
TypeSafe (`jev-latest`), pas un modèle local.

Le modèle n'y fait que deux gestes, envoyés par lots (un état partagé,
plusieurs questions nommées par requête) :

- **choisir** : un menu de choix décrits, plus un choix « aucun » → une
  distribution de probabilités ;
- **noter** : une grille à quatre niveaux → une note de 0 à 3.

La grille d'utilité d'un passage :

| Note | Niveau |
|---|---|
| 0 | sans rapport avec la question |
| 1 | même sujet, mais n'aide pas à répondre |
| 2 | répond en partie, ou donne des faits d'appui |
| 3 | contient les faits précis demandés, toutes contraintes comprises |

## 2. Ce que notre `Decider` sait aujourd'hui

`extension/rag3weaver/src/decider.rs` : une seule forme,
`decide(prompt, options) → probabilités`, lue sur les log-probabilités du
jeton suivant. Pas de choix « aucun » intégré, pas de note, pas de lot.

Les deux manques se construisent sur la forme existante, sans changer le
modèle :

- « aucun » est une option de plus, que l'appelant ajoute ;
- la note est l'espérance d'un choix entre les niveaux de la grille
  (Σ pᵢ · i) : un `score(prompt, grille)` au-dessus de `decide`.

Le lot (plusieurs questions par requête) n'existe pas côté `llama-server` ;
on envoie les questions en parallèle, c'est une affaire de débit, pas de
forme.

## 3. Les idées, et ce qu'on en fait

| # | Idée chez eux | Leurs réglages | Pour nous | Suite |
|---|---|---|---|---|
| 1 | Un choix ne vaut que s'il est net ; sinon on reste au parent | ≥ 0,65 et 0,2 d'avance sur le second pour ranger ; ≥ 0,8 pour déplacer ce qui est déjà rangé | la règle de Lucie — confirmer quand un proche existe, jamais refuser — avec des chiffres | **essai 1**, banc de la mémoire |
| 2 | « Aucun » diffère la branche au lieu de l'exclure | reprise des branches différées quand les preuves sont faibles | avec l'essai 1 : une option « aucun » dans le menu des sujets | **essai 1** |
| 3 | Créer une branche en deux temps : un modèle génératif propose nom et description, le modèle de décision valide contre l'existant | proposition bornée à 600 jetons, un ou deux niveaux | le garde-fou de `create_ref_type`, d'un nouveau sujet, d'un nouveau gabarit | **essai 2**, banc de la mémoire |
| 4 | La note d'utilité comme relecteur, avec règle d'arrêt | < 1,5 écarté ; ≥ 2,75 on s'arrête ; à trois passages partiels, une question de plus : couvrent-ils ensemble toute la question ? | la relecture des résultats de recherche | **essai 3**, avec le chantier relecture |
| 5 | Descente d'arbre en faisceau | 4 routes, moyenne géométrique des probabilités, 96 expansions au plus | la mémoire (mémoires → sujets → fiches), la porte de l'archiviste ; pas le code | piste, pas d'essai maintenant |
| 6 | Des extraits choisis selon la question dans les descriptions du menu, sans résumé à l'indexation | 3 extraits, 1 200 caractères | à garder en tête pour 5 | piste |

## 4. Les essais

**Essai 1 — seuils et « aucun » sur le choix du sujet.** Sur le banc de la
mémoire, comparer la règle actuelle à : menu des sujets proches + « aucun » ;
rangement direct si p ≥ 0,65 et avance ≥ 0,2 ; sinon demande de précision
(jamais un refus). Mesurer doublons, mauvais rangements, et le nombre de
demandes de précision — une règle qui demande à chaque fois n'a rien gagné.
Les seuils sont des réglages déclarés ; 0,65 / 0,2 sont leur point de départ,
pas un résultat pour notre modèle.

**Essai 2 — proposer puis valider.** Pour un genre de référence ou un sujet
nouveau : le modèle génératif propose (nom, description) ; le `Decider`
choisit entre l'existant, la proposition et « aucun ». La proposition n'est
créée que si elle l'emporte nettement. Mesurer : genres en double créés,
bonnes créations refusées.

**Essai 3 — la note d'utilité.** `score` au-dessus de `decide`, la grille du
§1, sur les cas du banc de recherche où la relecture était attendue. Mesurer
le gain et le coût en appels.

**La règle de preuve vaut ici** (`../3-octobre-2026-20h55/` : rien ne tenait
hors du jeu d'origine) : chaque essai se rejoue sur un jeu qui n'est pas de
notre main avant de conclure.

## 5. Les réserves

- **Coût.** Chez eux, environ six appels par recherche, une seconde de
  médiane, 4 à 5 s au 95ᵉ centile, avec un service hébergé. Avec notre modèle
  sur l'autre poste, à mesurer avant de mettre la décision dans un chemin
  chaud.
- **Portée de leurs mesures.** Chaque question est posée dans son document
  (DocBench), pas dans toute la bibliothèque ; ils le disent. La couverture
  des preuves y est jugée par un modèle, avec relecture des cas changés.
- **Leur modèle n'est pas le nôtre.** Les seuils tiennent à la calibration de
  `jev-latest` ; les nôtres sont à trouver.

À retenir aussi de leur méthode : mesurer la couverture des preuves à part de
la justesse de la réponse, et retirer ce qui n'a pas prouvé de gain (ils ont
défait quatre changements sur cinq le 3 octobre).
