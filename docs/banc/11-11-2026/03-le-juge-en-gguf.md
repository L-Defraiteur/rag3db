# Le juge local en GGUF par llama-server (11 novembre 2026)

> **Résultat final — 12 novembre 2026, après la seconde passe.** Card2 a servi
> pour les 1 320 paires : médiane 540 ms pour cinq fiches, 787 ms pour dix.
> Le contexte seat1 est resté à 40 KiB de VRAM et 2 080 KiB de GTT ; les poids,
> cache et calcul sont placés sur card2. 7882 restauré, verrou rendu ; mesure
> clôturée. [Résultats, méthode et limites](../12-11-2026/01-le-juge-sur-card2.md).
> Les notices et commandes précédentes ci-dessous sont historiques.

> **Précision ultérieure de Lucie — 12 novembre 2026.** Le but est de garder
> le GPU seat1 disponible pour jouer : les poids, le cache et le calcul du banc
> doivent rester sur card2. Un contexte d’initialisation minuscule et stable
> sur card0 peut être toléré ; sa seule ouverture ne doit plus faire échouer
> le banc. Les notices exigeant zéro accès ci-dessous décrivent le contrat
> précédent. ROCm initialise aussi des ressources des GPU masqués ; le
> [ticket k10](../../tickets/closed/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md)
> donne les sources et les contrôles d’attribution restant à qualifier.
> Zéro **requête du banc** a été joué : le serveur avait effectué son warmup
> interne, dont l’exécution n’a pas été attribuée par compteur PCI.
> Aucun nouveau chargement ni retry sans nouvelle fenêtre coordonnée.

> **Consigne actuelle — 12 novembre 2026.** Card0/seat1 (PCI 07:00.0)
> est interdite aux sessions. Seule card2 (PCI 04:00.0) peut être utilisée,
> après coordination de charge et lecture de la VRAM, avec
> `ROCR_VISIBLE_DEVICES=0` explicite et identité PCI vérifiée. Les anciennes
> commandes card0 ci-dessous sont historiques et ne doivent pas être rejouées.
> Le passage du 12 novembre a été arrêté avant toute inférence : ROCm ouvrait
> aussi un contexte de 40 KiB sur card0 malgré le filtre. Aucun nouveau
> chargement avant exclusion stricte de seat1 et nouvelle fenêtre coordonnée.
> Suivi : [ticket k10](../../tickets/closed/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).

La demande de l'orchestration, pour la page de la session mémoire (96) sur le classeur local
(`docs/09-novembre-2026-memoire-longue/03-le-classeur-local-dans-le-rappel.md`) : rejouer le
banc du juge local dans la forme de production. Les chiffres de la page
`extension/rag3weaver/docs/08-novembre-2026-banc/03-le-juge-local.md` étaient en torch bf16.
Ici, c'est **llama-server** (le serveur HTTP de llama.cpp) avec des poids au format **GGUF** (le
format de llama.cpp), quantifiés en **Q8_0** (8 bits) et **Q4_K_M** (environ 4,5 bits).

**En une phrase : Q8_0 garde le juge tel quel (76 % de bonnes premières à 2 000 jetons, la même
première fiche que bf16 dans 97 % des cas), et le cache de préfixe de llama-server divise le coût
par trois. Q4_K_M ne perd rien à 2 000 jetons, mais perd sur la forme courte.**

## 1. Ce qui a tourné

- **llama.cpp** : le clone et le bâti de Lucie à luciepc, `~/git_workspaces/llama.cpp`, version
  b9590 (commit d2462f8), bâti HIP pour gfx1201 dans `build-hip/bin/`. On n'y a fait que lire et
  exécuter : aucun changement de commit, aucun nouveau bâti, `git status` resté propre.
- **La conversion** :
  - `convert_hf_to_gguf.py` du même clone, depuis les safetensors officiels de
    Qwen3-4B-Instruct-2507 (Apache 2.0) déjà à luciepc, vers un GGUF bf16 (8,05 Go) ;
  - puis `llama-quantize` vers Q8_0 (4,28 Go) et Q4_K_M (2,50 Go) ;
  - le paquet `gguf` vient du clone (`pip install --no-deps -e …/gguf-py`), dans le venv du banc ;
    rien d'autre n'a été téléchargé.
- **Deux ajustements de la conversion**, sans rien changer à llama.cpp :
  - le `torchvision` du système, qui ne va pas avec torch 2.15, est masqué ;
  - le convertisseur importe `sentencepiece` avant de constater que Qwen n'a pas de
    `tokenizer.model`. Un module vide passe l'import, le fichier manque, et le vocabulaire BPE de
    Qwen est pris comme prévu.
- **La carte** : card0 de luciepc seule (`ROCR_VISIBLE_DEVICES=1`).
  - card0 est montée de 10,8 à 17,5 Go avec Q8_0 et de 10,9 à 15,9 Go avec Q4_K_M ;
  - card2, celle des services, n'a pas bougé (25,2 Go) ;
  - la charge de luciepc était de 11 à 17.
- **Le serveur** : `llama-server -ngl 99 -c 16384 -np 1`.
- **Le client** (`q_llama.py`) :
  - il applique le gabarit de conversation de Qwen3-4B par son tokeniseur, puis envoie le texte
    à `/completion`, avec un jeton prédit et `n_probs: 50` ;
  - le score vaut sigmoïde(log P(oui, Oui) − log P(non, Non)) sur les probabilités du premier
    jeton ;
  - même jeu (440 paires), même question que la page 03.
- **Le cache de préfixe** : les cinq fiches d'un cas se suivent. Avec `cache_prompt: true`,
  llama-server garde le contexte commun : la première fiche du cas traite tout le contexte (environ
  2 000 jetons), les suivantes leurs seuls jetons propres (environ 150).

## 2. Les résultats

| poids | forme | premier (test) | AUC dans le cas | p50 par paire | p50 par cas de 5 |
|---|---|---|---|---|---|
| bf16 torch (page 03) | début 2 k, préfixe partagé | 0,76 (0,74) | 0,92 | — | 1 017 ms |
| **Q8_0, avec cache** | début 2 k | **0,76** (0,76) | **0,92** | 83 ms | **1 076 ms** |
| Q8_0, sans cache | début 2 k | 0,76 (0,76) | 0,92 | 651 ms | 3 288 ms |
| Q8_0, avec cache | forme courte | 0,77 (0,71) | 0,89 | 86 ms | 445 ms |
| Q4_K_M, avec cache | début 2 k | 0,76 (0,76) | 0,91 | 90 ms | 1 162 ms |
| Q4_K_M, sans cache | début 2 k | 0,76 (0,76) | 0,92 | 752 ms | 3 790 ms |
| Q4_K_M, avec cache | forme courte | 0,72 (0,62) | 0,88 | 89 ms | 471 ms |

Les colonnes « premier » et « AUC dans le cas » suivent les définitions de la page 03 : la bonne
fiche est-elle première de ses 5 (hasard 0,20), et la part des paires bien ordonnées dans un cas
(hasard 0,50).

**Les écarts avec bf16 torch**, paire par paire, sur les mêmes 440 paires :

| poids | forme | même première fiche que bf16 | paires qui passent de l'autre côté de 0,5 | écart de score > 0,1 |
|---|---|---|---|---|
| Q8_0 | début 2 k | 97 % | 4,3 % | 9,3 % |
| Q8_0 | forme courte | 94 % | 5,2 % | 12,3 % |
| Q4_K_M | début 2 k | 92 % | 12,0 % | 17,0 % |
| Q4_K_M | forme courte | 91 % | 15,5 % | 20,0 % |

L'écart médian est nul : les scores sont presque toujours proches de 0 ou de 1. Les écarts tiennent
à quelques paires qui basculent. Q8_0 garde le classement ; Q4_K_M en bouge un sur douze sans
changer la mesure à 2 000 jetons, mais il le paie sur la forme courte.

## 3. Ce que ça change à la recommandation

La chaîne de la page 03, § 4, tient telle quelle dans la forme de production :

> rappel `recall_fiches` (k = 5) → Qwen3-4B-Instruct-2507 **en GGUF Q8_0 par llama-server, avec
> `cache_prompt`**, contexte de fin de conversation borné à 2 000 jetons, logits oui/non → garder
> les 1 à 2 premières.

- **Le coût** : environ 1,1 s par cas de 5, plus environ 0,2 s de rappel, soit 1,3 s au
  préchargement. La première fiche coûte le contexte, environ 0,65 s ; chaque fiche suivante
  environ 0,09 s.
- **Sans `cache_prompt`**, le cas coûte trois fois plus (3,3 s).
- **Q4_K_M** ne fait gagner ni temps ni mémoire utile ici : 1 Go de VRAM de moins, et aucun
  gain de temps sur card0. Il perd 5 points sur la forme courte. Q8_0 est le choix.
- **Pour k = 10**, on estime 1,5 s (0,65 s + 10 × 0,09 s), sans l'avoir mesuré.

## 4. Les fichiers

- **À luciepc** :
  - les poids GGUF dans `~/.cache/rag3db-banc-notes/hf/qwen3-4b-{bf16,Q8_0,Q4_K_M}.gguf` ;
  - les scripts `gguf-bati.sh`, `gguf-jeu.sh` et `q_llama.py`, les journaux `llama-*.log` et les
    résultats `gguf-*.jsonl`, dans `~/.cache/rag3db-banc-notes/`.
- **Sur le portable** : une copie des résultats dans `~/.cache/rag3db-banc-notes/decision/gguf/`.


## 5. Passage du 12 novembre : aucune mesure nouvelle

Suivi et preuves : [ticket k10](../../tickets/closed/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md),
porteur banc. Les 440 paires k5 et 880 paires k10 ont été construites et contrôlées.
Après GO coordonné, acquisition de `poste mesure` à 17:54:59 Paris et libération
explicite de 7882 par l’optimiseur, card2 avait 11,685 Gio libres.

Le serveur Q8_0, `ROCR_VISIBLE_DEVICES=0`, a chargé poids/cache sur card2
(PCI 04:00.0, 6 783 872 KiB de VRAM dans ses fdinfo). Sa garde a aussi trouvé
un contexte sur card0/seat1 (PCI 07:00.0, 40 KiB de VRAM et 2 080 KiB de GTT),
et l’a arrêté à 17:55:50 avant toute requête d’inférence. Une ouverture de contexte
n’établit pas un calcul du modèle sur card0. Elle empêche cependant d’affirmer
que le périphérique seat1 est exclu par le filtre ROCm seul.

Zéro paire mesurée, aucun retry : les chiffres Q8_0 du 11 novembre n’ont pas été
rejoués et l’estimation k10 n’est toujours pas une mesure. Le pic card2 pendant
ce chargement était de 28 731 637 760 octets, soit environ 6,58 Gio au-dessus
de la baseline. Le serveur a terminé, 8099 a été libéré et la VRAM rendue.

7882 a été restauré par l’optimiseur à 17:56:22.790, puis revérifié par le banc
(santé granite-278m/768/Flex32 réelle, PID, fdinfo card2). Les trois autres services
sont restés inchangés. Verrou mesure rendu à 17:57:00, poste relu libre.
Le ticket reste ouvert pour établir une exclusion stricte de seat1 avant tout
nouveau passage coordonné ; aucun nouveau chargement n’est autorisé par ce bilan.


## 6. Mesure achevée le 12 novembre sur card2

La [page de résultats](../12-11-2026/01-le-juge-sur-card2.md) remplace désormais
l’estimation k10 : 786,6 ms médians et 883,4 ms au p95 par cas de dix, contre
539,9/635,8 ms pour cinq dans la même passe. Première utile : 58/88 et 67/88 ;
les deux premières contiennent l’utile dans 78/88 et 84/88 cas. Zéro requête
tronquée, aucun doublon, pas de retry. Les scores communs sont identiques.
Flags/cache différents de l’historique, première paire sans warmup et ordre
k5 puis k10 documentés ; corpus construit, pas mesure du rappel réel.
Retour 7882 et verrou rendu attestés. Cette mesure ne branche aucun juge en
production ; elle fournit les données à la session mémoire.
