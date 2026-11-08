# Un petit modèle de décision : le juge local (8 novembre 2026, soir)

Suite des pages 01 et 02, même jeu : 88 positifs construits à provenance vérifiée, chacun face à 4
négatifs (2 durs, 2 au hasard), 440 paires, test sur 3 sessions. Dans chaque cas, on regarde si la
bonne fiche est la première de ses 5 (hasard 0,20), et l'AUC dans le cas (hasard 0,50). La
question est celle de Lucie, mot pour mot : « ce bout de mémoire est-il pertinent avec le contexte
de l'agent ? ».

**En une phrase : Qwen3-4B-Instruct-2507, jugé fiche par fiche par les logits oui/non, est un juge
local presque aussi bon que Gemini** : il classe la bonne fiche première dans 76 à 80 % des cas,
contre 81 à 84 % pour Gemini. C'est un rangeur des candidates du rappel, pas un filtre à seuil.

## 1. Les modèles, sur card0

Chaque modèle juge une paire à la fois, par les logits oui/non en une passe, sur card0 de luciepc
(07:00.0, avec l'accord de Lucie).

| modèle (licence) | forme | premier (test) | AUC dans le cas (test) | p50 par paire | VRAM |
|---|---|---|---|---|---|
| Gemini 3.8 Flash (référence) | 5 en liste, 2 k à 16 k | 0,81 à 0,84 | — | 4,4 à 4,8 s par cas | — |
| **Qwen3-4B-Instruct-2507** (Apache 2.0) | début, 2 k | **0,76** (0,74) | **0,92** (0,90) | 504 ms | 9,1 Go |
| Qwen3-4B | fin de conversation, 2 k | 0,76 (0,74) | 0,93 | 517 ms | |
| Qwen3-4B | fin de conversation, 6 k | 0,78 (0,79) | 0,93 | 1 005 ms | |
| Qwen3-4B | début, 6 k | 0,80 (0,85) | 0,92 | 1 805 ms | |
| Qwen3-4B | forme courte | 0,73 (0,65) | 0,89 | 90 ms | |
| Qwen2.5-1.5B-Instruct (Apache 2.0) | début, 2 k | 0,65 (0,68) | 0,84 (0,86) | 207 ms | 4,3 Go |
| Qwen2.5-1.5B | fin de conversation, 2 k | 0,64 (0,65) | 0,82 | 212 ms | |
| Qwen2.5-1.5B | fin de conversation, 6 k | 0,66 (0,65) | 0,83 | 395 ms | |
| Qwen2.5-1.5B | début, 6 k | 0,62 (0,62) | 0,85 | 693 ms | |
| Qwen2.5-1.5B | forme courte | 0,53 (0,59) | 0,76 | 41 ms | |
| Qwen3-1.7B (Apache 2.0, réflexion coupée) | début, 2 k | 0,50 (0,65) | 0,75 | 255 ms | 4,6 Go |
| Qwen3-1.7B | forme courte | 0,42 (0,47) | 0,70 | 52 ms | |
| Qwen2.5-0.5B, SmolLM2-360M, Laya | toutes les formes | 0,02 à 0,23 | 0,46 à 0,57 | 245 ms à 1,3 s | |

- **Les formes du contexte** :
  - le **début** est le prompt et les tours d'avant, à rebours, les sorties d'outils réduites à
    leur première ligne (page 02) ;
  - la **fin de conversation** est ce que le rappel a sous la main en production : les deux
    derniers tours avant le prompt, avec leurs sorties d'outils (1 200 caractères chacune), puis
    le prompt. Elle est bornée à 2 k et 6 k jetons, et coupée par le début si elle déborde
    (p50 7 000 et 12 000 caractères) ;
  - la **forme courte** est la dernière phrase de l'agent et le dernier message.
- **Ce que disent les bornes** : à 2 k, la fin de conversation vaut le début. Au-delà de 2 k,
  Qwen3-4B gagne 2 à 4 points (0,78 à 0,80), et Qwen2.5-1.5B rien. Le prix : 2 à 3,5 fois le
  temps.
- **Laya à 16 k** : il ne lit que 8 192 jetons ; il voit le début du contexte et pas le prompt
  (premier 0,02).
- **Les licences** sont vérifiées sur la carte de chaque modèle et dans son fichier LICENSE avant
  usage. Qwen2.5-3B-Instruct n'est pas mesuré : il est sous Qwen Research License, non
  commerciale.

## 2. Plusieurs fiches d'un coup

| forme | modèle | contexte | premier | AUC | p50 par cas de 5 |
|---|---|---|---|---|---|
| par paire (5 passes entières) | Qwen3-4B | début 2 k | 0,76 | 0,92 | 2 520 ms (5 × 504) |
| **préfixe partagé** (cache KV) | Qwen3-4B | début 2 k | 0,76 (0,74) | 0,92 | **1 017 ms** |
| préfixe partagé | Qwen3-4B | fin 2 k | 0,75 (0,76) | 0,93 | 1 033 ms |
| liste A à E, ordre mélangé | Qwen3-4B | début 2 k | 0,38 | 0,68 | 645 ms |
| liste A à E, ordre mélangé | Qwen3-4B | fin 2 k | 0,45 | 0,71 | 657 ms |
| par paire | Qwen2.5-1.5B | début 2 k | 0,65 | 0,84 | 1 035 ms (5 × 207) |
| **préfixe partagé** | Qwen2.5-1.5B | début 2 k | 0,65 (0,65) | 0,84 | **424 ms** |
| liste A à E, ordre mélangé | Qwen2.5-1.5B | début 2 k | 0,16 | 0,46 | 247 ms |

- **Le préfixe partagé.** Le contexte est encodé une fois (environ 2 060 jetons), puis chaque fiche
  passe derrière (environ 153 jetons avec la question). Le texte est celui de la forme par paire,
  au caractère près. Les scores sont les mêmes : écart médian 0,0003 (1,5B) et 0,0000 (4B), au plus
  0,06 et 0,25 sur une paire, à la coupure de jetons à la jonction. Le classement est inchangé, pour
  2,4 à 2,5 fois moins de temps. Coût de la VRAM : le cache copié pour chaque fiche porte
  Qwen3-4B à environ 14 Go sur card0, plus que la moitié de la VRAM libre fixée par Lucie
  (12,4 Go ce soir) ; à garder en tête pour la forme de production (un seul cache, pas cinq
  copies).
- **La liste A à E** est plus rapide mais moins bonne : 0,38 à 0,45 pour Qwen3-4B, le hasard pour
  Qwen2.5-1.5B. **Un artefact écarté** : la première passe donnait 0,99 à Qwen2.5-1.5B, parce que
  le positif était toujours la lettre A dans mes cas groupés. Rejouée avec l'ordre mélangé (graine
  fixe, positif de A à E 24 à 40 fois chacun), elle tombe au hasard. Seule la liste mélangée est
  dans la table.

## 3. Le coût au préchargement

Mesuré sur card0, carte partagée avec le jeu de Lucie, luciepc à une charge de 10 à 25 :

| chaîne | k = 5 | k = 10 |
|---|---|---|
| rappel `recall_fiches` par l'hôte | ≈ 190 ms | ≈ 190 ms |
| + Qwen3-4B, préfixe partagé, fin de conversation 2 k | ≈ 1,0 s (mesuré) | ≈ 1,6 s (estimé : contexte ≈ 0,4 s + 0,12 s par fiche) |
| + Qwen2.5-1.5B, préfixe partagé, 2 k | ≈ 0,42 s (mesuré) | ≈ 0,66 s (estimé : ≈ 0,19 s + 0,047 s par fiche) |
| + Qwen3-4B, forme courte, par paire | ≈ 0,45 s (5 × 90 ms) | ≈ 0,9 s |
| Gemini 3.8 Flash, 5 en liste | 4,4 s p50, 11 à 23 s p95 | — |

Les valeurs de k = 10 sont extrapolées des mesures de k = 5, pas mesurées.

## 4. Recommandation

> **Rappel `recall_fiches` (k = 5) → Qwen3-4B-Instruct-2507, préfixe partagé, fin de conversation
> bornée à 2 k jetons, logits oui/non → garder les 1 à 2 premières → le modèle de l'agent.**
> Environ 1,2 s au préchargement sur une carte libre (au lieu de 4 à 23 s par Gemini), 76 % de
> bonnes premières contre 81 à 84 % pour Gemini.

- **Si 1,2 s est trop** : Qwen3-4B en forme courte (0,45 s, 73 %), ou Qwen2.5-1.5B en préfixe
  partagé (0,42 s, 65 %).
- **En rangeur, jamais en seuil.** L'échelle des scores varie avec le contexte : un seuil qui
  garde 100 % des utiles n'ôte que 4 à 7 % des non utiles (Qwen2.5-1.5B).
- **Le dense bge-m3 seul ne suffit pas** (41 % de premières, page 02), ni le relecteur bge (36 à
  38 %), ni les modèles de moins d'un milliard de paramètres, ni Laya en zéro-coup.

## 5. Les limites

- **Petit jeu** : 88 positifs. La graine est fixe, de quoi l'étendre aux sessions de luciepc et à
  LR_CodeRag.
- **Positifs construits par Gemini** : la référence et les positifs viennent du même modèle,
  d'où une avance possible de Gemini.
- **Une carte partagée** : card0 portait aussi le jeu de Lucie. Les temps sont ceux d'une carte
  partagée.
- **Laya spécialisé, une piste** : il n'est pas lancé, faute de positifs en nombre pour la mise au
  point (54).

## 6. Les notes d'outil du soir

- **ROCm** : `ROCR_VISIBLE_DEVICES=1` seul rend card0 seule visible. Poser aussi
  `HIP_VISIBLE_DEVICES=1` la cache (HIP renumérote à partir de 0), et le modèle retombe sur le
  CPU. Même en CPU, torch ROCm tient 44 à 68 Kio de contexte sur chaque carte.
- **ssh tailscale** : un `nohup … &` lancé dans un `timeout N ssh` qui expire meurt avec la
  session. `setsid nohup … & disown`, dans un appel court, survit.
- **Un blocage sur card2** : une paire de Laya y a tenu la carte 56 minutes, et les services
  d'embarquement voisins ont ralenti (page 02, § 3). Depuis, un chien de garde note « dépassée »
  toute paire qui ne rend rien en 30 s, et arrête le modèle ; aucune ne l'a été sur card0.

## 7. La suite : un décideur d'actions

La piste de Lucie, relayée par l'orchestration le 8 novembre au soir : la même forme pour décider
d'une action. Le modèle reçoit le contexte de l'agent (borné) et l'action proposée (une commande,
une écriture, un chemin), et répond par les logits oui/non à « cette action est-elle permise ? ».
Qwen3-4B est le candidat.

- **Le jeu** : everything declarative journalisera chaque approbation et chaque refus donnés par
  l'utilisateur dans la porte de l'atelier, avec le contexte, l'action et la décision. On mesurera
  à quelques centaines de décisions, comme ici : positif premier et AUC. On mesurera surtout le
  **taux de faux « permis »**, l'erreur qui coûte.
- **La règle à poser avant de le brancher**, celle de « un filtre ôte, il ne promeut pas » : un
  décideur local peut demander plus (renvoyer à l'utilisateur ce qui lui paraît douteux), jamais
  approuver à la place de l'utilisateur ce que la porte lui aurait soumis. Son seuil se règle par
  le taux de faux permis, pas par l'accord moyen.
