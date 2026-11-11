# Le classeur local dans le rappel — page de conception

*9 novembre 2026, session mémoire (96), à la demande de l'orchestration. Rien ne se code avant le
mot de Lucie. Sources : la page 03 du banc du 8 novembre (`docs/08-novembre-2026-banc/03-le-juge-local.md`),
le rappel des crochets de Claude Code (`src/claude_code_hook.rs`, lots 3 ter et 3 ter b,
`docs/07-novembre-2026-hooks-claude-code/08`), le décideur llama-server (`src/decider.rs`).*

## 1. Ce que le banc a conclu

Question de Lucie, mot pour mot : « ce bout de mémoire est-il pertinent avec le contexte de
l'agent ? ». Sur 88 positifs face à 4 négatifs chacun :

| juge | bonne fiche première | AUC dans le cas | temps pour 5 fiches |
|---|---|---|---|
| bge-m3 dense seul | 0,41 | — | (le rappel) |
| **Qwen3-4B-Instruct-2507**, préfixe partagé, fin de conversation 2 k | **0,76** | **0,92** | **≈ 1,0 s** |
| Qwen3-4B, forme courte | 0,73 | 0,89 | ≈ 0,45 s |
| Qwen2.5-1.5B, préfixe partagé | 0,65 | 0,84 | ≈ 0,42 s |
| Gemini 3.8 Flash, 5 en liste | 0,81 à 0,84 | — | 4,4 s p50, 11 à 23 s p95 |

C'est **un rangeur, pas un seuil** : l'échelle des scores bouge avec le contexte, on garde les
1 ou 2 premières. Les mesures sont en Python (torch ROCm, bf16) sur card0 de luciepc, carte
partagée avec le jeu de Lucie.

## 2. Ce qui existe aujourd'hui

- **Le rappel synchrone** (`UserPromptSubmit`) : `recall_fiches`, 3 candidates, filtre
  `state ∈ {current, to_review}`, sous `recall_timeout_ms` = 1 000 ms (13 à 70 ms mesurés en
  service). Ce qui est rendu tout de suite passe par la redite et le déjà-vu de la session.
- **Le rappel différé et son juge** (3 ter b) : un fils détaché (`hook --rappel-differe`) refait
  le rappel si le budget a coupé, puis **Gemini** juge les candidates (`juger`, sortie
  structurée, il ôte, jamais n'ajoute), au plus `recall_judge_max_per_hour` = 20 appels par
  heure. Le résultat est rangé et rendu au prochain outil ou au prochain tour. Ce juge ne voit
  **que le prompt** nettoyé, pas les tours d'avant.
- **La transcription** de la session est lisible par le crochet
  (`claude_code_transcript::tours_du_prompt`) : les tours, les appels d'outils, leurs résultats.
- **Le décideur llama-server** (`LlamaServerDecider`, la voie `RAG3WEAVER_BANC_DECIDE` du banc des
  fiches classées) : `/tokenize`, puis `/completion` d'un jeton avec `n_probs`, les
  probabilités des options normalisées. Il pose **`cache_prompt: false`** : chaque appel réencode
  tout, sans le préfixe partagé que le banc a mesuré.

## 3. Où il se branche

**Dans le fils du rappel différé, à la place du juge Gemini quand un classeur est déclaré**, pas
dans le rappel synchrone :

- le synchrone a une seconde pour tout, rappel compris (≈ 190 ms) ; le classeur à 1,0 s ne tient
  pas dedans, et la forme courte (0,45 s) n'en laisse aucune marge sur une carte partagée ;
- le fils n'a pas de budget serré : 1,2 s y est quinze fois plus court que le juge Gemini
  d'aujourd'hui, et le résultat arrive au prochain outil, souvent dans la seconde ;
- **une option du synchrone, mesurée d'abord** : `recall_classifier.sync = true` essaie le
  classeur en forme courte dans la seconde du tour, et au premier dépassement retombe sur le
  différé pour ce tour. Elle n'est allumée qu'après la mesure du § 7.

Le classeur **range** les candidates du rappel (k = 5 au lieu de 3, comme au banc) et garde les
`keep` premières (1 ou 2). Il n'en ajoute jamais, comme le juge d'aujourd'hui.

## 4. La voie

- **Déclarée dans `hooks.json`**, puisque c'est la commande du crochet qui l'appelle :

      "recall_classifier": {
        "address": "http://127.0.0.1:7982",
        "model": "qwen3-4b-instruct-2507",
        "keep": 2,
        "candidates": 5,
        "context_tokens": 2000,
        "timeout_ms": 3000,
        "sync": false
      }

  Absente : rien ne change (§ 6).
- **Le client est `LlamaServerDecider`**, avec deux ajouts : `cache_prompt: true` et un même
  préfixe pour toutes les fiches d'un rappel (le contexte d'abord, puis la fiche, puis la
  question), pour que llama-server réutilise le cache de son slot d'une fiche à l'autre ; et un
  délai borné par `timeout_ms` (aujourd'hui 60 s en dur).
- **Le service tourne à luciepc**, jamais sur le portable (Lucie : pas de service lourd sur le
  portable). Le portable l'atteint par un tunnel ssh à côté de ceux des services d'embarquement
  (7979 à 7981), sur un port neuf.
- **Le modèle** : Qwen3-4B-Instruct-2507 en GGUF (Apache 2.0, licence relue sur la carte et le
  fichier LICENSE avant la pose).

## 5. Le contexte donné au juge

La **fin de conversation** du banc, celle que Lucie tient : les **deux derniers tours avant le
prompt, avec les résultats de leurs outils** (1 200 caractères chacun), puis le prompt, le tout
borné à 2 000 jetons et coupé par le début s'il déborde. Elle se lit dans la transcription au
moment du fils. Puis, pour chaque candidate : son nom, son `about`, et la question de Lucie.
Réponse : P(oui) contre P(non) au jeton suivant.

Rien de ce contexte ne quitte le poste de luciepc, au lieu de partir chez Vertex comme le prompt
du juge d'aujourd'hui.

## 6. Sans llama-server

**Le rappel d'aujourd'hui, rien de moins.** `recall_classifier` absent, service injoignable ou
délai dépassé : le fils prend le juge Gemini s'il est disponible, sinon la politique « sans juge »
d'aujourd'hui (rien rendu pour des candidates venues du tour, la première au plein texte pour un
rappel refait). Une ligne au journal des crochets dit lequel a jugé et pourquoi. Jamais un code
d'erreur, jamais un bloc qui attend.

## 7. La mesure qui dira si ça vaut le coût

Sur les crochets réels de Lucie, en rejeu (`hook --rejouer`, qui n'écrit rien) de ses sessions
des derniers jours, trois juges sur les mêmes tours : aucun, Gemini, le classeur.

- **La précision des fiches glissées** : pour chaque fiche rendue, est-elle utile au tour ? La
  référence : le jugement de Lucie sur un échantillon de 50 tours, et à défaut celui de Gemini sur
  la fin de conversation (avec la réserve du banc : Gemini a construit les positifs).
- **Le temps par tour** : p50 et p95 du fils (le rappel, le contexte, le classeur), et du tour
  synchrone si l'option `sync` est essayée.
- **La VRAM de luciepc** : la carte du service, avant et après la pose, sous le jeu de Lucie.
  Le banc a vu 9,1 Go en bf16 et 14 Go avec le cache copié pour chaque fiche ; un seul slot et un
  GGUF Q8 devraient tenir autour de 5 Go, à mesurer.
- **L'écart du GGUF** : le banc était en torch bf16. Avant la mesure réelle, le jeu du banc
  (88 cas) rejoué par llama-server : on ne passe que si la bonne première reste à 0,74 ou plus.
- **Le seuil d'adoption**, proposé : précision des fiches glissées au moins égale au juge Gemini
  moins 5 points, p95 du fils sous 3 s, et la VRAM dans ce que Lucie accorde.

### L'écart du GGUF, mesuré (session banc, 11 novembre 2026)

Le préalable ci-dessus est levé : `docs/banc/11-11-2026/03-le-juge-en-gguf.md` (master
96a98a452). Qwen3-4B-Instruct-2507 converti en GGUF par le llama.cpp de Lucie, servi par
llama-server sur card0 de luciepc, même jeu de 440 paires, oui/non par les logits du premier jeton.

| forme | bonne première | AUC | par cas de 5 |
|---|---|---|---|
| bf16 torch (8 novembre), préfixe partagé, 2 k | 0,76 | 0,92 | 1 017 ms |
| **Q8_0, `cache_prompt`, 2 k** | **0,76** | **0,92** | **1 076 ms** |
| Q8_0, forme courte | 0,77 | — | 445 ms |
| Q8_0 sans `cache_prompt` | — | — | 3 288 ms |
| Q4_K_M, 2 k | 0,76 | — | pas plus rapide sur card0 |

La même première fiche que bf16 dans 97 % des cas. **Q8_0 avec `cache_prompt` est le choix**,
les cinq fiches d'un rappel à la suite sur le même slot, le contexte d'abord, la fiche à la fin :
sans le cache, trois fois plus lent. Le client de référence est `~/.cache/rag3db-banc-notes/q_llama.py`
à luciepc (gabarit appliqué côté client, `/completion` avec `n_predict` 1 et `n_probs` 50, score =
sigmoïde(log P(oui, Oui) − log P(non, Non))). `LlamaServerDecider` doit donc prendre
`cache_prompt: true` (§ 4) et ce score.

La page 04 (la précision du rappel d'aujourd'hui) reste la condition de fond : le classeur ne
vaut sa seconde que lorsque la mémoire de Lucie a du contenu à ranger.

## 8. Questions pour Lucie

1. **La carte** : un llama-server de Qwen3-4B à luciepc, sur quelle carte et avec quel plafond de
   VRAM (≈ 5 Go en Q8 attendus) ; ou rien.
2. **Remplacer ou ajouter** : le classeur remplace-t-il le juge Gemini du rappel différé quand il
   répond, ou les deux jugent-ils (Gemini en second avis, payant) ?
3. **Garder 1 ou 2** fiches par tour.
4. **Le synchrone** : essayer la forme courte dans la seconde du tour, ou s'en tenir au différé.
5. **L'échantillon de référence** : jugeras-tu 50 tours toi-même, ou Gemini sert-il de référence ?

## 9. L'ordre, après son mot

1. Le jeu du banc rejoué par llama-server (GGUF Q8, préfixe partagé), contre les chiffres bf16.
2. `LlamaServerDecider` : `cache_prompt` et le délai ; `recall_classifier` dans `hooks.json` ; le
   classeur dans le fils, avec sa ligne de journal ; témoins sur un faux llama-server.
3. La mesure du § 7 en rejeu, puis le rapport à Lucie.
4. Seulement ensuite, l'allumage chez elle, et `sync` si la mesure le permet.
