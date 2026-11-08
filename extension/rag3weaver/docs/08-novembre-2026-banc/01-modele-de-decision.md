# Un petit modèle de décision : la première mesure (8 novembre 2026)

La demande de Lucie, 8 novembre, 10 h 50 : « il faudrait qu'une session essaie de trouver vraiment
un modèle de décision, un bon petit modèle ; ça coûterait beaucoup moins cher en temps pour
plusieurs trucs, genre doit-on charger ce contexte ou est-il sans rapport pour le modèle, au
préchargement de mémoire ». Elle est confiée au banc par l'orchestration, en mesure d'abord, sans
aucun code produit.

**En une phrase : ce jeu ne permet de classer aucun modèle.** Sur les 1 328 paires (prompt, fiche)
tirées de l'usage réel, la relecture à la main n'en trouve aucune où la fiche aide vraiment. Les
« utiles » de Gemini sont, à une près, la fiche d'identité de la session elle-même, que le hook
écarte déjà. Il faut un jeu qui contienne de vrais positifs avant de comparer des modèles. Trois
constats servent quand même tout de suite (§ 5).

## 1. Le jeu

- **Les rappels réels** : 122 rappels de `~/.local/state/rag3weaver/hooks.log`. Ils viennent tous
  de la session d'essai des hooks (494dbbc6, worktree hooks-essai, 7 et 8 novembre). Chaque
  prompt a été retrouvé dans la transcription de la session par l'heure et la longueur ; dans
  104 cas, la longueur concorde exactement. 4 cas sont écartés, faute que les mots de la requête
  se retrouvent dans le prompt. Restent 118 rappels.
- **D'autres sessions** : 48 prompts tirés au hasard (graine fixe) dans 40 transcriptions de
  rag3db. Leur requête est tirée par le rejeu du hook (`hook --rejouer`, journal à part : rien
  n'est écrit dans celui de Lucie).
- **Les candidats** : `recall_fiches` par l'hôte, en lecture seule, avec le filtre du hook (état
  `current` ou `to_review`). La limite est de 8 au lieu des 3 du hook, et le rang est gardé pour
  rejouer la vue du hook (rang < 3).
- **Le total** : 166 cas, 1 328 paires, **19 fiches distinctes**. La mémoire de Lucie n'a
  aujourd'hui que ces fiches courantes : la fiche d'identité « hooks-essai », des bilans de tâches
  de l'équipier (dont le nom est la consigne entière) et des fiches de défauts d'outils
  (« outil : report », « outil : run_command »…).

Le juge différé du rappel (`hook --rappel-differe`, Gemini Flash) existe dans le code
(`claude_code_hook.rs`, `juger`), mais il est éteint chez Lucie : `hooks.log` n'a aucun verdict.
Les étiquettes ont donc été produites hors ligne, avec sa consigne mot pour mot.

## 2. Les étiquettes

**Gemini 3.8 Flash** (Vertex, global) : la consigne et le schéma strict de `juger`, température 0,
un appel par cas, l'un après l'autre (1,5 s entre deux, une attente de 60 s sur un 429).

| | |
|---|---|
| appels | 202, pour 166 cas (36 relances après une réponse coupée, § 5.1) |
| jetons | 466 067 d'entrée, 10 207 de sortie, dont 48 173 de réflexion |
| latence par cas (8 fiches) | p50 4,0 s, p95 11,7 s |
| quota | au moins deux 429 sur 202 appels (le journal de la première passe est perdu) |

Gemini garde 67 paires sur 1 328 (5 %) : **66 fois la fiche « hooks-essai », une fois
« régime GPU »**. Sa raison est presque toujours la même : « la fiche identifie la session
destinataire du message ». Les messages sont adressés « → hooks-essai », et la consigne ne dit
pas au juge quelle session il sert.

**La relecture à la main** : 60 paires tirées en trois strates (20 que Gemini garde, 20 qu'il ôte
alors que le score du rappel est haut, 20 qu'il ôte au hasard ; graine fixe). La feuille est
prête pour Lucie (`~/.cache/rag3db-banc-notes/decision/feuille.md`, la clé à part). Relue par le
banc, pas par Lucie : 21 paires en entier, les 39 autres par la fiche et le début du message.
**Aucune des 60 n'est utile.** La fiche d'identité n'apprend rien à la session qui la porte ; et
le hook l'écarte déjà (« exclu : « hooks-essai » (fiche propre) »). Accord de Gemini avec la
main : 0,67, soit 40 sur 40 pour les non et 0 sur 20 pour les oui.

## 3. Les candidats

| candidat | accord avec Gemini (AUC, toutes paires) | vue du hook (rang < 3) | latence | à télécharger |
|---|---|---|---|---|
| relecteur bge-reranker-v2-m3 (service bge-m3, luciepc) | 0,53 | 0,53 | p50 183 ms, p95 191 ms par cas de 8 paires, tunnel compris | rien |
| score fusionné du rappel (ce que le hook a déjà, seuil 0,50) | 0,92 | 0,89 | 0 (calculé par le rappel) | rien |
| laya-multilingual (ModernBERT 322 M, Apache 2.0) | non mesuré | | | 1,3 Go |
| Qwen2.5-0.5B-Instruct (Apache 2.0), par les logits | non mesuré | | | 1,0 Go |
| SmolLM2-360M-Instruct (Apache 2.0), par les logits | non mesuré | | | 0,7 Go |
| Gemini 3.8 Flash | (référence) | | p50 4,0 s par cas | |

- **Les trois non mesurés** : l'installation par pip (`transformers`, `laya`) dans un venv à
  luciepc a été refusée par la permission de la session (« code venu de l'extérieur »).
  L'orchestration l'a remise à Lucie. Rien n'a été créé.
- **Les deux passes du relecteur** sont identiques au bit près.
- **Le bon score du score fusionné est trompeur.** Contre Gemini, il dit seulement que le plein
  texte retrouve le mot « hooks-essai » dans un message adressé à hooks-essai. Contre la main, il
  n'y a aucun positif à classer.
- **Laya, d'après sa carte** : son zéro-coup sur des décisions typées est de 0,342, sous la classe
  majoritaire (0,461). La carte le présente comme une base à spécialiser.

## 4. Ce que la mesure ne dit pas

Avec zéro vrai positif, aucune précision ni aucun rappel n'a de sens, et aucun seuil ne se fixe
selon la règle « un filtre ôte, il ne promeut pas ». Le jeu ne vient que de deux jours d'une
session d'essai et de 19 fiches. Le préchargement de mémoire de Lucie dans une vraie tâche, avec
une mémoire riche, n'y est pas représenté.

## 5. Ce qui sert tout de suite

1. **Le juge du rappel coupe sa réponse** sous son plafond de 400 jetons : la réflexion de
   Gemini 3.8 Flash les prend. Sur 146 appels dont la raison de fin est gardée, 37 finissent sur
   `length` (25 %), avec 2 à 33 jetons de réponse. Relancés à 4 000, tous finissent sur `stop`.
   Ticket ouvert pour archi : `docs/tickets/2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md`
   (d596c829c). Le nœud `Decide` a déjà le remède : un plafond de 2 048 et une troncature
   nommée.
2. **Le juge ne sait pas qui il sert.** Sa consigne ne nomme pas la session, si bien qu'il garde
   la fiche d'identité de la session pour tout message qui lui est adressé. Le hook l'écarte en
   amont (la fiche propre), donc rien ne fuit aujourd'hui. Mais un juge, ou un petit modèle, posé
   sans cette exclusion ferait la même faute. Si le juge est rallumé, sa consigne doit dire
   l'identité de l'agent, ou ne jamais recevoir sa fiche propre.
3. **Le relecteur bge-m3 ne juge pas la pertinence d'un résumé de fiche pour un message**
   (AUC 0,53, le hasard). De plus, il refuse une requête de plus de 512 jetons (20 prompts sur
   166 ; ça passe une fois coupés à 300 caractères). C'est un choix écrit dans le code
   (`burn_reranker.rs`, troncature `OnlySecond` : la requête doit passer entière). Seul point
   discutable : le service le rend en 500, au lieu d'un refus typé.

## 6. Recommandation

- **N'adopter aucun modèle aujourd'hui** : il n'y a rien à filtrer, et le filtre actuel (le seuil
  0,50, la fiche propre, le dense seul non jugé) ne laisse rien passer de faux sur ce jeu.
- **Refaire la mesure sur un jeu à vrais positifs** :
  - (a) les prompts de Lucie elle-même, dans des tâches où une fiche sert vraiment, avec une
    cinquantaine de positifs relus par elle ;
  - ou (b) les décisions fermées de l'archiviste et du juge des bilans (5 candidates, aucune,
    créer), où Gemini est payé aujourd'hui, et où Laya et le décideur par logits ont leur forme
    naturelle.
- Il faudra alors la permission d'installer `transformers` et `laya` et de télécharger les trois
  modèles (3,1 Go, Apache 2.0), puis jouer en CPU, puis sur card0 de luciepc (libre de tout
  service ; index vérifié par `rocm-smi --showbus`, VRAM des deux cartes lue avant et après, arrêt
  au premier reset gfx).

## Fichiers

Les scripts, le jeu étiqueté, la feuille et sa clé sont dans
`~/.cache/rag3db-banc-notes/decision/` du portable : `a_rappels.py`, `b_jeu.py`, `c_gemini.py`,
`d_bge.py`, `e_analyse.py`, `f_feuille.py`, `paires.json`, `gemini.json`, `bge.json`,
`feuille.md`, `main.json`. Le jeu porte des prompts de sessions et n'est pas mis en annexe.
