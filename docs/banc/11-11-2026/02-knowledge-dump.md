# Le banc : knowledge dump (11 novembre 2026)

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

Ce document dit tout ce qu'il faut pour reprendre les bancs sans l'historique de nos échanges.
Chaque mot de jargon est défini à son premier usage. Le rapport de session du même dossier
(`01-rapport-de-session.md`) dit l'état ; ce document dit comment faire.

## 1. Les postes, les cartes et les règles d’usage

La description et les lanceurs ci-dessous sont ceux du 11 novembre. La notice
du 12 novembre en tête prévaut sur toute autorisation historique de card0.
Le filtre ROCm ne garantit pas à lui seul l’absence de contexte sur seat1.

- **Le portable** : la session tourne ici. Il porte un llama-server personnel de Lucie (port 8080),
  qui tient environ 77 Gio de mémoire GPU partagée : rien de lourd ici, on joue à luciepc.
- **luciepc** : l'autre poste, `ssh -T -o BatchMode=yes lucied@luciepc`. Son shell de connexion
  est fish : une commande bash passe par `bash -c '…'` ou `bash -s <<'EOF'`. Il porte deux cartes
  AMD Radeon AI PRO R9700 de 32 Go :
  - **card2**, bus PCI 04:00.0, `GPU[0]` pour `rocm-smi` : les quatre services d'embarquement
    (granite-278m sur le port 7878, bge-m3 sur 7879, granite-107m sur 7880, et un 7882). **Jamais
    de cinquième charge sur card2.** Un second modèle posé à côté y a provoqué des resets de
    l'anneau graphique (le 25 octobre et le 9 novembre). Une passe du banc y a ralenti les services
    à 12-19 s par texte le 8 novembre.
  - **card0**, bus 07:00.0, `GPU[1]` pour `rocm-smi` : la carte de la session graphique de Lucie
    (seat1, ses jeux). Elle n'est utilisable qu'**au mot de Lucie**, en lisant la VRAM avant et en
    lui laissant au moins la moitié de la VRAM libre.
- **Épingler card0** : poser `ROCR_VISIBLE_DEVICES=1` **seul**. Poser aussi
  `HIP_VISIBLE_DEVICES=1` la rend invisible : HIP renumérote les cartes visibles à partir de 0,
  et torch retombe sur le CPU. Le vérifier avant chaque passe :
  `ROCR_VISIBLE_DEVICES=1 python -c "import torch; print(torch.cuda.get_device_properties(0).pci_bus_id)"`
  doit rendre 7.
- **L'ordre Vulkan n'est pas l'ordre ROCm** : en Vulkan, `gpu:0` est card2. Ne pas lancer de
  modèle par Vulkan ou burn sans réglage explicite.
- **Même en CPU**, torch ROCm ouvre les deux cartes et y tient 44 à 68 Kio de contexte. Avec
  `ROCR_VISIBLE_DEVICES=-1` et `HIP_VISIBLE_DEVICES=-1`, c'est 44 Kio, sans aucun calcul.
- **`poste`** (`~/.cache/rag3weaver-build/poste`, aux deux postes) : le verrou de charge partagé
  entre sessions.
  - `poste lourd <commande>` prend une des N places lourdes. Il y en a 2 ou 3 à luciepc selon
    l'orchestration, 1 ici. Le délai se met dans la commande : `poste lourd timeout N …`, sinon
    il s'écoule en file.
  - `poste mesure <commande>` prend toute la machine. Seulement dans une fenêtre donnée par
    l'orchestration.
  - `poste push banc -- <script>` : le verrou du push sur master, pour le dernier pas seulement.
  - `poste --qui` dit qui tient quoi.
- **Lancer à distance** : un `nohup … &` dans un `timeout N ssh …` meurt quand le timeout coupe la
  session ssh (tailscale). Lancer par `setsid nohup … & disown` dans un appel court, puis attendre
  dans un autre appel.
- **Vérifier un arrêt** : `pgrep -u lucied -af <script>` vide, plus l'heure de départ des
  processus restants (`ps -o lstart= -p <pid>`). Le code de sortie du lanceur ne suffit pas.
  Ne tuer que ses propres pids.

## 2. Le juge local de fiches de mémoire

**La forme de la question.** Le message est la question de Lucie, mot pour mot, avec le gabarit
de conversation du modèle :

```
Contexte de l'agent :
<contexte>

Bout de mémoire :
« <nom> » : <résumé> — <corps>

Ce bout de mémoire est-il pertinent avec le contexte de l'agent ? Réponds par oui ou par non.
```

**Le score par les logits.** Une seule passe avant, sans génération. On lit les log-probabilités
du premier jeton de la réponse et on en tire
`sigmoïde(logsumexp(oui, Oui, " oui", " Oui") − logsumexp(non, Non, " non", " Non"))`. Pour
Qwen3-1.7B, qui réfléchit par défaut, le gabarit reçoit `enable_thinking=False`.

**Les formes du contexte** :
- **début** : le prompt et les tours précédents, à rebours, les sorties d'outils réduites à leur
  première ligne ;
- **fin de conversation** : ce que le rappel a en production, les deux derniers tours avant le
  prompt avec leurs sorties d'outils (1 200 caractères chacune), puis le prompt ;
- **forme courte** : la dernière phrase de l'agent et le dernier message.

Les bornes sont de 2 000, 6 000 et 16 000 jetons, estimées à 3,5 caractères par jeton ; on coupe
par le début.

**Le préfixe partagé** (cache KV) :
- On encode une fois le texte jusqu'à la place de la fiche, et on garde son `past_key_values`, la
  mémoire d'attention.
- Chaque fiche passe ensuite derrière une copie de ce cache.
- Les scores égalent ceux de la forme par paire : écart médian 0,0003.
- C'est 2,5 fois moins de temps. Mais les copies de cache portent Qwen3-4B à environ 14 Go :
  en production, un seul cache.

**La liste A à E** :
- Les 5 fiches tiennent dans une seule question, et le score de chacune est la probabilité de sa
  lettre au premier jeton.
- **Piège** : il faut mélanger l'ordre des fiches. Le premier essai avait le positif toujours en A
  et donnait un 0,99 trompeur.

**La mesure** :
- Dans chaque cas, la bonne fiche est-elle première de ses 5 ? Le hasard donne 0,20.
- L'AUC dans le cas : la part des paires (positive, négative) bien ordonnées ; le hasard donne
  0,50.
- Pour un filtre : le seuil qui garde 100 %, puis 95 %, des utiles, et la part des inutiles qu'il
  ôte.
- La règle de la maison : **un filtre ôte, il ne promeut pas**. Un juge local peut écarter, ou
  renvoyer à l'humain, jamais approuver à sa place.

## 3. Les jeux de données et leur construction

Tout est dans `~/.cache/rag3db-banc-notes/decision/` du portable ; les scripts sont en Python,
lancés par `python3 -I`.

1. **Le premier jeu** (`a_rappels.py`, `b_jeu.py`) :
   - 118 rappels réels de `~/.local/state/rag3weaver/hooks.log`, le journal des hooks de Claude
     Code. Leur prompt a été retrouvé dans la transcription
     `~/.claude/projects/<projet>/<session>.jsonl` par l'heure et la longueur. Les messages entre
     sessions y sont des entrées `queue-operation`.
   - Plus 48 prompts d'autres sessions, leur requête tirée par
     `rag3weaver-backend hook --rejouer <transcription> UserPromptSubmit --prompt <texte> --config <copie>`,
     avec une copie du réglage qui écrit son journal à part.
   - Les candidats viennent de `recall_fiches` par l'hôte (HTTP `POST /session`, `/open`, `/call`
     sur 127.0.0.1:7970, jeton dans `~/.local/share/rag3weaver/daemon.json`, en-tête
     `X-Rag3weaver-Session`).
   - Résultat : aucun vrai positif. La mémoire n'avait que 19 fiches, et les « utiles » de Gemini
     étaient la fiche d'identité de la session elle-même.
2. **Le jeu à vrais positifs** :
   - `g_construire.py` : pour 130 prompts réels (graine 20261108, 16 au plus par session), Gemini
     lit les tours suivants (32 000 caractères au plus) et écrit la fiche qui aurait aidé, avec sa
     provenance (le tour, un extrait mot pour mot vérifié). La consigne interdit de recopier les
     mots du prompt dans le nom et le résumé. Résultat : 88 positifs vérifiés.
   - `i_paires.py` : les 2 négatifs durs par le dense bge-m3 (`POST /embed` sur 127.0.0.1:7980,
     tunnel vers le 7879 de luciepc), 2 au hasard, les contextes aux bornes, le découpage 70/30
     par session.
   - `m_fin.py` : la fin de conversation et les cas groupés (`cas2.jsonl`, `cas2-melange.jsonl`).
3. **Les références** :
   - `c_gemini.py` et `j_reference.py` : Gemini 3.8 Flash par Vertex. Compte de service dans
     `~/git_workspaces/rag3db/.vault/vertex-sa.json`, projet `lr-hub-472010`, jeton JWT signé avec
     `cryptography`. URL : `https://aiplatform.googleapis.com/v1/projects/lr-hub-472010/locations/global/endpoints/openapi/chat/completions`.
   - Les appels se font un à la fois, 1,5 s entre deux. Sur un 429 (quota), attendre une minute,
     puis le double. Plafond de réponse à 8 000 jetons : la réflexion du modèle se compte dedans.
   - `d_bge.py` et `k_bge2.py` : le relecteur, `POST /rerank` sur 7980. Il refuse une requête de
     plus de 512 jetons, par choix écrit dans `burn_reranker.rs`.
4. **L'analyse** : `e_analyse.py` (premier jeu), `l_analyse.py` (jeu à vrais positifs) et les
   calculs en ligne des pages 02 et 03.

## 4. Les scripts des modèles à luciepc

Dans `~/.cache/rag3db-banc-notes/` :

- **`h_petits.py <modèle> <entrée.jsonl> <sortie.jsonl>`** : une paire à la fois.
  - Les modèles : `qwen`, `qwen15`, `qwen3-4b`, `qwen3-17`, `smollm`, `laya`.
  - `DEVICE=cpu` ou `cuda`, `FILS` pour le nombre de fils CPU.
  - La reprise se fait par le fichier de sortie.
  - Le torchvision du système ne va pas avec torch 2.15 : il est masqué dans le script.
- **`h_multi.py <modèle> prefixe|liste <cas.jsonl> <sortie.jsonl>`** : plusieurs fiches d'un coup.
- **Les lanceurs** (`gpu-card0.sh`, `gpu-qwen3.sh`, `gpu-multi.sh`, `gpu-liste.sh`, `cpu-2000.sh`,
  `cpu-court.sh`) :
  - ils relèvent la charge, la VRAM de card0 et de card2 et `rocm-smi` avant et après chaque
    modèle ;
  - ils exigent la moitié de la VRAM libre de card0 ;
  - leur chien de garde arrête le modèle si une paire ne rend rien en 30 s, et la note
    « dépassée ».
  - On les lance sous `poste lourd`, par `setsid`.
- **Le venv** : `decision-venv/bin/python`, avec torch du système.
- **Les poids** : sous `hf/`, téléchargés par `decision-venv/bin/hf download <dépôt> --local-dir …`.

## 5. Les licences lues

| modèle | licence | vérifiée |
|---|---|---|
| Qwen2.5-0.5B-Instruct | Apache 2.0 | carte du modèle |
| Qwen2.5-1.5B-Instruct | Apache 2.0 | carte et fichier LICENSE |
| Qwen3-4B-Instruct-2507 | Apache 2.0 | carte et fichier LICENSE |
| Qwen3-1.7B | Apache 2.0 | carte et fichier LICENSE |
| SmolLM2-360M-Instruct | Apache 2.0 | carte |
| laya-multilingual | Apache 2.0 | carte |
| Qwen2.5-3B-Instruct | **Qwen Research License, non commerciale** | **refusé** sans le mot de Lucie |
| GGUF d'unsloth pour Qwen3-4B-2507 | Apache 2.0 | carte ; écarté au profit d'une conversion maison |

Règle : la licence et la taille sont dites à l'orchestration avant chaque téléchargement.

## 6. Les pièges appris

- **Une commande refusée par la permission a pu s'exécuter en partie** : vérifier ses effets
  avant d'affirmer que rien n'a changé. Un refus de permission ne se contourne pas, ni soi-même ni
  par une autre session : on le remonte à Lucie.
- **Un script copié par `scp` doit être rendu exécutable** (`chmod +x`), sinon `poste lourd`
  refuse en silence. Une passe GPU a été perdue ainsi le 8 novembre.
- **Un modèle peut se bloquer sur une paire** en tenant la carte à 100 % (Laya, 56 minutes, le
  8 novembre). D'où le chien de garde.
- **Laya ne lit que 8 192 jetons** : à 16 000, il ne voit que le début du contexte.
- **Les binaires de paquets CachyOS du portable** sont compilés en x86-64-v4, avec AVX-512 :
  ils plantent à luciepc, qui n'a qu'avx2.
- **Un témoin de temps** se juge sur une machine calme ; sous la charge d'un comparateur complet,
  il bascule. Ces témoins vont dans `probabilistic.txt` ou `long.txt`, pas dans `known_red.txt`.
- **`known_red.txt`** est la liste des rouges connus des témoins C++ de concurrence
  (`test/transaction/concurrence/`). Une ligne s'écrit `NomDuTest etiquette-a,etiquette-b`, avec
  des virgules sans espace. Le comparateur `compare_known_red.cmake` refuse toute différence.
- **Le push** : sur un worktree à soi, avec un commit par chemins listés, identité
  `luciedefraiteur@gmail.com`, sans ligne d'attribution. On passe par
  `~/.cache/rag3weaver-build/poste push banc -- ~/.cache/rag3db-banc-notes/push-lot.sh <worktree> aucune`.
  Puis trois contrôles : `git status` propre, aucun rebase en cours, `FETCH_HEAD` égal à `HEAD`.
  Jamais de push en force, jamais de suppression de branche distante (c'est la décision de
  Lucie).
- **L'arbre de Lucie**, `~/git_workspaces/rag3db`, ne reçoit ni commit, ni changement de branche,
  ni fichier.


## Reprise du 12 novembre : règles de charge actuelles

Card2 porte les services 7878, 7879, 7880 et le privé 7882. Ne pas ajouter
un cinquième processus GPU aux quatre services : pour le banc Q8_0, libérer
7882 avec son porteur et l’optimiseur, prévoir son retour avant arrêt, relire
la VRAM réellement disponible et les consommateurs globaux. Un service libre
côté banc ne prouve pas l’absence de clients tiers. Aucun arrêt d’un processus
d’autrui sans remise explicite.

Pour le passage k10 du 12 novembre, le budget convenu était 9 Gio libres minimum,
hausse maximale de 8 Gio et marge d’au moins 1 Gio, `poste mesure` exclusif,
20 minutes après acquisition ; premier défaut : arrêt et diagnostic, aucun retry.
Ce budget n’est pas une autorisation permanente ni une preuve d’isolation.
Le service a été restauré et le verrou rendu ; le ticket garde la suite.

Les résultats Q8_0 à cinq fiches du 11 novembre restent des résultats historiques
obtenus sur card0 avec l’autorisation d’alors ; ils n’ont pas été rejoués sur card2.
Aucun résultat k10 disponible. Lire le ticket avant toute nouvelle activation.
